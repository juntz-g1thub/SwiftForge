# 方案C: TUI 完全重构详细设计

> 文档版本: 1.0
> 生成日期: 2026-05-19
> 状态: 待确认

---

## 一、设计目标

1. **组件化**: 每个UI组件独立管理自己的状态
2. **可测试**: View逻辑与渲染分离，便于单元测试
3. **可扩展**: 新增视图/功能不影响现有代码
4. **类型安全**: 状态转换使用enum，避免无效状态

---

## 二、目标架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         AppController                               │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │  current_view: ViewState                                       │ │
│  │  context: Arc<AppContext>                                      │ │
│  │  pending_actions: Vec<Action>                                   │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                              │                                       │
│            ┌─────────────────┼─────────────────┐                     │
│            ▼                 ▼                 ▼                     │
│     ┌────────────┐    ┌────────────┐    ┌────────────┐              │
│     │ ChatView   │    │ ConfigView │    │ DebugView  │              │
│     │ (Stateful) │    │ (Stateful) │    │ (Stateful) │              │
│     └────────────┘    └────────────┘    └────────────┘              │
│            │                 │                 │                     │
│            └─────────────────┴─────────────────┘                     │
│                              │                                       │
│                              ▼                                       │
│                    ┌─────────────────┐                               │
│                    │   AppContext    │                               │
│                    │  ┌───────────┐  │                               │
│                    │  │   Agent   │  │                               │
│                    │  │   Config  │  │                               │
│                    │  │ToolRegistry│  │                               │
│                    │  └───────────┘  │                               │
│                    └─────────────────┘                               │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 三、核心类型定义

### 3.1 AppContext (共享状态)

```rust
/// 全局共享上下文
pub struct AppContext {
    pub agent: Agent,
    pub config: ConfigManager,
    pub tool_registry: Arc<ToolRegistry>,
    pub debug_log_path: Option<PathBuf>,
}

/// UI相关的共享状态（通过channel传递）
pub struct UIState {
    pub streaming_text: Option<String>,
    pub debug_messages: Vec<String>,
    pub pending_response: Option<Receiver<Result<String>>>,
}
```

### 3.2 ViewState (路由状态)

```rust
/// 应用级状态机
pub enum ViewState {
    Chat(ChatViewState),
    Config(ConfigViewState),
    Debug(DebugViewState),
}

/// 聊天视图状态
pub struct ChatViewState {
    pub messages: Vec<(String, String)>,  // (role, content)
    pub input: String,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
    pub content_height: usize,
    pub is_streaming: bool,
    pub pending_receiver: Option<mpsc::Receiver<Result<String>>>,
}

/// 配置视图状态
pub enum ConfigViewState {
    SelectProvider,                                  // 选择Provider
    EditProvider(ProviderEditState),                  // 编辑Provider详情
    FetchModels { error: Option<String> },            // 正在获取模型列表
    SelectModel(Vec<String>),                        // 选择模型
}

/// 调试视图状态
pub struct DebugViewState {
    pub messages: Vec<String>,
    pub scroll_offset: usize,
    pub content_height: usize,
}
```

### 3.3 Action (事件驱动)

```rust
/// View返回的动作，由Controller执行
pub enum Action {
    // 聊天相关
    SendMessage(String),
    CancelStreaming,
    AppendMessage(String, String),  // role, content

    // 配置相关
    SwitchProvider(String),
    SaveApiKey(String),
    SaveModel(String),
    SaveBaseUrl(String),
    FetchModels,

    // 导航
    SwitchView(ViewState),

    // 滚动
    ScrollUp,
    ScrollDown,
    ScrollDebugUp,
    ScrollDebugDown,

    // 退出
    Quit,
}
```

### 3.4 View Trait

```rust
/// View接口 - 所有视图必须实现
pub trait View {
    /// 渲染视图到指定区域
    fn render(&mut self, f: &mut Frame, area: Rect, ui_state: &UIState);

    /// 处理按键事件，返回Action
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action>;

    /// 进入视图时调用
    fn on_enter(&mut self) {}

    /// 离开视图时调用
    fn on_exit(&mut self) {}
}
```

---

## 四、View 实现详细设计

### 4.1 ChatView

```rust
pub struct ChatView {
    pub state: ChatViewState,
    pub scrollbar_state: ScrollbarState,
}

impl View for ChatView {
    fn render(&mut self, f: &mut Frame, area: Rect, ui_state: &UIState) {
        // 使用Layout分割区域
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),   // 消息区域
                Constraint::Length(3), // 输入框
                Constraint::Length(1), // 状态栏
            ])
            .split(area);

        // 渲染消息列表
        self.render_messages(f, chunks[0], ui_state);

        // 渲染输入框
        self.render_input(f, chunks[1]);

        // 渲染状态栏
        self.render_status(f, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.state.is_streaming {
            // 流式传输中只允许特定按键
            match key.code {
                KeyCode::Up => return Some(Action::ScrollUp),
                KeyCode::Down => return Some(Action::ScrollDown),
                KeyCode::Esc => return Some(Action::CancelStreaming),
                _ => return None,
            }
        }

        match key.code {
            KeyCode::Char(c) if key.modifiers == KeyModifiers::CONTROL => {
                match c {
                    'q' => Some(Action::Quit),
                    's' => Some(Action::SwitchView(ViewState::Config(ConfigViewState::SelectProvider))),
                    _ => None,
                }
            }
            KeyCode::Char(c) => {
                self.state.input.push(c);
                Some(Action::AppendMessage("user".to_string(), c.to_string()))
            }
            KeyCode::Backspace => {
                remove_char_before_cursor(&mut self.state.input, &mut self.state.cursor_pos);
                None
            }
            KeyCode::Enter => {
                if !self.state.input.trim().is_empty() {
                    let msg = self.state.input.clone();
                    self.state.input.clear();
                    self.state.cursor_pos = 0;
                    self.state.is_streaming = true;
                    Some(Action::SendMessage(msg))
                } else {
                    None
                }
            }
            KeyCode::Up => Some(Action::ScrollUp),
            KeyCode::Down => Some(Action::ScrollDown),
            // ... 其他按键
            _ => None,
        }
    }
}
```

### 4.2 ConfigView

```rust
pub struct ConfigView {
    pub state: ConfigViewState,
    pub input_buffer: String,
    pub cursor_pos: usize,
}

impl View for ConfigView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match &mut self.state {
            ConfigViewState::SelectProvider => {
                match key.code {
                    KeyCode::Char('1') => Some(Action::SwitchProvider("openai".to_string())),
                    KeyCode::Char('2') => Some(Action::SwitchProvider("anthropic".to_string())),
                    KeyCode::Char('3') => Some(Action::SwitchProvider("ollama".to_string())),
                    KeyCode::Char('4') => Some(Action::SwitchProvider("deepseek".to_string())),
                    KeyCode::Char('5') => Some(Action::SwitchProvider("minimax".to_string())),
                    KeyCode::Char('c') => Some(Action::SwitchView(ViewState::Config(ConfigViewState::EditProvider(ProviderEditState::CustomName))))),
                    KeyCode::Char('f') => Some(Action::FetchModels),
                    KeyCode::Esc => Some(Action::SwitchView(ViewState::Chat(ChatViewState::new()))),
                    _ => None,
                }
            }
            ConfigViewState::EditProvider(edit_state) => {
                match key.code {
                    KeyCode::Enter => {
                        // 保存并切换到下一步
                        match edit_state {
                            ProviderEditState::ApiKey => Some(Action::SaveApiKey(self.input_buffer.clone())),
                            ProviderEditState::Model => Some(Action::SaveModel(self.input_buffer.clone())),
                            ProviderEditState::BaseUrl => Some(Action::SaveBaseUrl(self.input_buffer.clone())),
                            _ => None,
                        }
                    }
                    KeyCode::Esc => {
                        self.input_buffer.clear();
                        self.state = ConfigViewState::SelectProvider;
                        None
                    }
                    KeyCode::Char(c) => {
                        self.input_buffer.push(c);
                        None
                    }
                    KeyCode::Backspace => {
                        remove_char_before_cursor(&mut self.input_buffer, &mut self.cursor_pos);
                        None
                    }
                    _ => None,
                }
            }
            ConfigViewState::FetchModels { .. } => {
                if let KeyCode::Esc = key.code {
                    Some(Action::SwitchView(ViewState::Config(ConfigViewState::SelectProvider)))
                } else {
                    None
                }
            }
            ConfigViewState::SelectModel(models) => {
                // 上下选择，回车确认
                None
            }
        }
    }
}
```

### 4.3 DebugView

```rust
pub struct DebugView {
    pub state: DebugViewState,
    pub scrollbar_state: ScrollbarState,
}

impl View for DebugView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Up => Some(Action::ScrollDebugUp),
            KeyCode::Down => Some(Action::ScrollDebugDown),
            KeyCode::Esc => Some(Action::SwitchView(ViewState::Chat(ChatViewState::new()))),
            _ => None,
        }
    }
}
```

---

## 五、AppController 设计

### 5.1 结构

```rust
pub struct AppController {
    current_view: Box<dyn View>,
    context: Arc<AppContext>,
    ui_state: UIState,
}

impl AppController {
    pub fn new(config: ConfigManager, agent: Agent, tool_registry: Arc<ToolRegistry>, show_debug: bool) -> Self {
        let context = Arc::new(AppContext {
            agent,
            config,
            tool_registry,
            debug_log_path: Self::init_debug_log(show_debug),
        });

        let ui_state = UIState {
            streaming_text: None,
            debug_messages: Vec::new(),
            pending_response: None,
        };

        Self {
            current_view: Box::new(ChatView::new()),
            context,
            ui_state,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        loop {
            terminal.draw(|f| self.current_view.render(f, f.size(), &self.ui_state))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if let Some(action) = self.current_view.handle_key(key) {
                        self.handle_action(action, &runtime)?;
                        if let ViewState::Chat(cs) = &self.ui_state {
                            if cs.should_quit {
                                break;
                            }
                        }
                    }
                }
            }

            // 处理pending的响应
            self.process_pending_response();
        }
        Ok(())
    }

    fn handle_action(&mut self, action: Action, runtime: &tokio::runtime::Runtime) -> Result<()> {
        match action {
            Action::SendMessage(msg) => {
                self.context.state.add_message("user", &msg);
                self.ui_state.is_streaming = true;

                let (tx, rx) = mpsc::channel();
                self.ui_state.pending_response = Some(rx);

                let context = Arc::clone(&self.context);
                std::thread::spawn(move || {
                    runtime.block_on(async {
                        // agent.run_agent_loop() 调用
                        // 结果通过 tx 发送
                    });
                });
            }
            Action::SwitchView(view) => {
                self.current_view.on_exit();
                self.current_view = Self::create_view(view);
                self.current_view.on_enter();
            }
            // ... 其他action处理
            _ => {}
        }
        Ok(())
    }
}
```

### 5.2 状态流转图

```
                    ┌─────────┐
                    │  Start  │
                    └────┬────┘
                         │
                         ▼
              ┌─────────────────────┐
              │  ChatView (default) │
              └──────────┬──────────┘
                         │
          ┌──────────────┼──────────────┐
          │ Ctrl+S      │ Ctrl+Q       │ User Input
          ▼             ▼              ▼
    ┌───────────┐  ┌─────────┐  ┌──────────────┐
    │ConfigView │  │  Quit   │  │ SendMessage  │
    └─────┬─────┘  └─────────┘  └──────┬───────┘
          │                           │
          │                           ▼
          │                ┌─────────────────┐
          │                │ Streaming...    │
          │                │ (is_streaming)  │
          │                └────────┬────────┘
          │                         │
          │         ┌──────────────┼──────────────┐
          │         │ Response     │ Esc          │ Error
          │         ▼              ▼              ▼
          │   ┌───────────┐  ┌─────────┐  ┌───────────┐
          │   │ Append to │  │ Cancel  │  │ Show Error│
          │   │ messages  │  └─────────┘  └───────────┘
          │   └───────────┘
          │
          └──────────────┐
                         │ Esc
                         ▼
                 ┌───────────────┐
                 │ Back to Chat  │
                 └───────────────┘
```

---

## 六、组件间通信

### 6.1 Channel 设计

```rust
// UI → Agent 的命令channel
type CommandChannel = mpsc::Sender<AgentCommand>;

enum AgentCommand {
    SendMessage(String),
    CancelCurrentRequest,
    // ... 其他命令
}

// Agent → UI 的响应channel (多个并发响应)
type ResponseChannel = mpsc::Sender<Result<String>>;

// Debug日志channel
type DebugChannel = mpsc::Sender<String>;
```

### 6.2 共享状态

```rust
/// Thread-safe 的共享状态
pub struct SharedState {
    pub messages: Arc<Mutex<Vec<(String, String)>>>,
    pub is_streaming: Arc<Mutex<bool>>,
    pub debug_messages: Arc<Mutex<Vec<String>>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            is_streaming: Arc::new(Mutex::new(false)),
            debug_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
```

---

## 七、渲染流程

### 7.1 聊天视图布局

```
┌─────────────────────────────────────────┐
│ Chat (Ctrl+S: Settings)                 │
├─────────────────────────────────────────┤
│ [user]: Hello                            │
│ [assistant deepseek-chat]: Hi!           │
│                                         │
│ <thinking>                              │
│ Let me think about this...              │
│ </thinking>                             │
│                                         │
│ <content>                               │
│ Here's my response...                   │
│ </content>                              │
│                                         │
│ [assistant deepseek-chat]: ▌            │  ← 闪烁光标表示流式
│                                         │
├─────────────────────────────────────────┤
│ > ▌                                      │  ← 输入区域
├─────────────────────────────────────────┤
│ [deepseek] - Press Ctrl+Q to quit...   │  ← 状态栏
└─────────────────────────────────────────┘
```

### 7.2 配置视图布局

```
┌─────────────────────────────────────────┐
│ Provider                                │
├─────────────────────────────────────────┤
│ Current: deepseek                       │
├─────────────────────────────────────────┤
│   [ ] OpenAI                            │
│   [>] Anthropic                         │
│   [ ] Ollama                            │
│   [ ] DeepSeek                          │
│   [ ] MiniMax                           │
│   [ ] Custom                            │
├─────────────────────────────────────────┤
│ [C] Custom Provider (0 configured)      │
│ [F] Fetch Models | [ESC] Back to Chat   │
└─────────────────────────────────────────┘
```

---

## 八、与现有代码的对比

### 8.1 文件结构变化

**当前**:
```
src/tui/
├── app.rs          (1330行 - 全部在一起)
├── mod.rs
├── config.rs
├── components.rs   (未使用?)
└── input.rs        (未使用?)
```

**重构后**:
```
src/tui/
├── mod.rs
├── app.rs                    # AppController + main entry
├── views/
│   ├── mod.rs
│   ├── view.rs               # View trait
│   ├── chat_view.rs          # ChatView 实现 (~200行)
│   ├── config_view.rs        # ConfigView 实现 (~200行)
│   └── debug_view.rs         # DebugView 实现 (~100行)
├── state/
│   ├── mod.rs
│   ├── app_context.rs        # AppContext + UIState
│   ├── view_state.rs         # ViewState, ChatViewState等
│   └── action.rs             # Action enum
├── components/
│   ├── mod.rs
│   ├── message_list.rs       # 消息列表组件
│   ├── input_area.rs         # 输入框组件
│   ├── scrollbar.rs          # 滚动条组件
│   └── status_bar.rs         # 状态栏组件
└── utils/
    ├── mod.rs
    └── text_utils.rs         # 文本处理工具
```

### 8.2 代码行数分布

| 模块 | 当前行数 | 重构后行数 | 变化 |
|------|----------|------------|------|
| app.rs | 1330 | ~150 (AppController) | -89% |
| chat_view.rs | - | ~250 | 新增 |
| config_view.rs | - | ~250 | 新增 |
| debug_view.rs | - | ~100 | 新增 |
| view.rs | - | ~50 | 新增 |
| state/*.rs | - | ~200 | 新增 |
| components/*.rs | - | ~200 | 新增 |
| **总计** | 1330 | ~1200 | -10% |

**减少的原因**: 重复代码消除，状态管理集中

---

## 九、迁移策略

### 阶段1: 创建基础框架
1. 创建 `views/` 目录和 `view.rs`
2. 定义 `ViewState`, `Action`, `AppContext` 等基础类型
3. 实现简单的 `View` trait

### 阶段2: 实现ChatView
1. 将 `app.rs` 的 Chat 相关逻辑迁移到 `chat_view.rs`
2. 测试渲染和按键处理

### 阶段3: 实现ConfigView
1. 将配置相关逻辑迁移到 `config_view.rs`
2. 测试配置流程

### 阶段4: 实现AppController
1. 创建 `app.rs` 中的 `AppController`
2. 连接各个View
3. 处理Agent通信

### 阶段5: 移除旧代码
1. 删除 `app.rs` 中的旧实现
2. 清理不再需要的字段

---

## 十、风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 重构期间功能退化 | 保留旧代码作为备份，逐步迁移 |
| 状态同步问题 | 使用 `Arc<Mutex>` 确保线程安全 |
| 性能下降 | 确保tokio runtime复用，不每次创建新runtime |
| 编译时间增加 | 合理拆分模块，避免循环依赖 |

---

## 十一、已确认的设计决策

> 更新日期: 2026-05-19

### 1. State管理 - 选项C (混合)

```rust
// AppContext - 共享的、线程安全的、数据类
struct AppContext {
    agent: Arc<Agent>,
    config: Arc<ConfigManager>,
    tool_registry: Arc<ToolRegistry>,
}

// UIState - UI相关状态，通过channel传递
struct UIState {
    streaming_text: Arc<Mutex<Option<String>>>,
    debug_messages: Arc<Mutex<Vec<String>>>,
}

// ViewState - 各View私有状态
enum ViewState {
    Chat(ChatViewState),
    Config(ConfigViewState),
    Debug(DebugViewState),
}
```

**结论**: 共享状态(Agent/Config)用Arc管理，UI状态各View私有，通过Action传递。

### 2. Action粒度 - 中等粒度

```rust
enum Action {
    // 聊天
    SendMessage(String),
    CancelStreaming,
    AppendMessage(String, String),
    ToggleDebug,

    // 导航
    SwitchView(ViewState),
    GoBack,

    // 滚动
    ScrollUp, ScrollDown,
    ScrollDebugUp, ScrollDebugDown,
    ResetScroll,

    // 输入编辑
    InputChar(char),
    InputBackspace, InputDelete,
    InputHome, InputEnd, InputLeft, InputRight,
    ClearInput,

    // 配置
    SelectProvider(String),
    SaveApiKey(String), SaveModel(String), SaveBaseUrl(String),
    FetchModels, SelectModel(String),

    // 系统
    Quit,
}
```

**结论**: 中等粒度，数据带在Action里，避免在Controller中额外推断。

### 3. Agent交互 - App持有Runtime

```rust
struct AppController {
    runtime: tokio::runtime::Runtime,
    agent_command_tx: mpsc::Sender<AgentCommand>,
    agent_response_rx: mpsc::Receiver<AgentResponse>,
}

impl AppController {
    fn spawn_agent_task(&self, msg: String) {
        let agent = self.agent.clone();
        self.runtime.spawn(async move {
            agent.run_agent_loop(msg).await;
        });
    }
}
```

**结论**: App创建时初始化单一Runtime，通过channel与Agent通信，复用Runtime减少开销。

---

## 十二、实现计划

### 阶段1: 基础框架
1. 创建 `src/tui/views/` 目录
2. 创建 `src/tui/state/` 目录
3. 创建 `src/tui/components/` 目录
4. 定义基础类型 (ViewState, Action, AppContext, UIState)
5. 实现 View trait

### 阶段2: ChatView实现
1. 实现 ChatViewState
2. 实现 ChatView::render()
3. 实现 ChatView::handle_key()
4. 实现 input handling 组件

### 阶段3: ConfigView实现
1. 实现 ConfigViewState
2. 实现 ConfigView::render()
3. 实现 ConfigView::handle_key()

### 阶段4: DebugView实现
1. 实现 DebugViewState
2. 实现 DebugView::render()
3. 实现 DebugView::handle_key()

### 阶段5: AppController实现
1. 实现 AppController
2. 连接 Agent (runtime + channel)
3. 实现 Action 处理分发
4. 实现主事件循环

### 阶段6: 集成测试
1. 编译验证
2. 功能测试
3. Debug panel 测试

---

## 十三、待确认事项

- [x] View trait 的设计是否合理？ → 已确认
- [x] Action enum 的粒度是否合适？ → 已确认 (中等粒度)
- [x] 状态流转是否满足所有场景？ → 已确认
- [x] Channel 设计是否满足并发需求？ → 已确认
- [x] 文件拆分方案是否可接受？ → 已确认

**所有事项已确认，可以开始实现。**