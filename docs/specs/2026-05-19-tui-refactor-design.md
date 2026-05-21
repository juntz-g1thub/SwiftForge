# TUI 重构 - 详细实现方案

> 文档版本: 1.0
> 生成日期: 2026-05-19
> 分支: feature/tui-refactor
> Worktree: `.worktrees/tui-refactor/`
> 状态: 待确认后实现

---

## 一、目标

将 `rust-agent-platform/src/tui/app.rs` (1330行) 重构为模块化的 TUI 架构。

---

## 二、最终文件结构

```
src/tui/
├── mod.rs                      # 导出所有公开类型
├── app.rs                      # AppController (重构目标: ~200行)
│
├── views/                      # View 接口和实现
│   ├── mod.rs
│   ├── view.rs                 # View trait 定义 (~50行)
│   ├── chat_view.rs            # ChatView (~250行)
│   ├── config_view.rs          # ConfigView (~250行)
│   └── debug_view.rs           # DebugView (~100行)
│
├── state/                      # 状态类型定义
│   ├── mod.rs
│   ├── app_context.rs          # AppContext, UIState (~60行)
│   ├── view_state.rs           # ViewState, ChatViewState, ConfigViewState, DebugViewState (~150行)
│   └── action.rs               # Action enum (~100行)
│
└── components/                 # 复用组件
    ├── mod.rs
    ├── message_list.rs         # 消息列表渲染
    ├── input_area.rs           # 输入框组件
    ├── scroll_bar.rs           # 滚动条组件
    └── status_bar.rs          # 状态栏组件
```

---

## 三、实现顺序

### 阶段1: 基础类型定义

#### 3.1.1 state/action.rs

```rust
use crossterm::event::KeyEvent;

/// 用户可执行的动作枚举
#[derive(Debug, Clone)]
pub enum Action {
    // === 聊天相关 ===
    /// 发送消息 (消息内容)
    SendMessage(String),
    /// 取消当前流式传输
    CancelStreaming,
    /// 追加消息 (role, content)
    AppendMessage(String, String),

    // === 导航相关 ===
    /// 切换视图 (目标视图状态)
    SwitchView(ViewStateKind),
    /// 返回上个视图
    GoBack,

    // === 滚动相关 ===
    ScrollUp,
    ScrollDown,
    ScrollDebugUp,
    ScrollDebugDown,
    /// 重置滚动位置到最新
    ResetScroll,

    // === 输入编辑相关 ===
    InputChar(char),
    InputBackspace,
    InputDelete,
    InputHome,
    InputEnd,
    InputLeft,
    InputRight,
    /// 清空输入框
    ClearInput,

    // === 配置相关 ===
    /// 选择Provider (provider_id)
    SelectProvider(String),
    SaveApiKey(String),
    SaveModel(String),
    SaveBaseUrl(String),
    /// 请求获取模型列表
    FetchModels,
    /// 选择模型 (model_name)
    SelectModel(String),

    // === 系统相关 ===
    /// 切换Debug面板显示
    ToggleDebug,
    /// 退出应用
    Quit,
}

/// 视图类型标识
#[derive(Debug, Clone, PartialEq)]
pub enum ViewStateKind {
    Chat,
    Config,
    Debug,
}
```

#### 3.1.2 state/view_state.rs

```rust
use super::action::ViewStateKind;

/// 视图状态 - 用于SwitchView Action
#[derive(Debug, Clone)]
pub enum ViewState {
    Chat(ChatViewState),
    Config(ConfigViewState),
    Debug(DebugViewState),
}

impl ViewState {
    pub fn kind(&self) -> ViewStateKind {
        match self {
            ViewState::Chat(_) => ViewStateKind::Chat,
            ViewState::Config(_) => ViewStateKind::Config,
            ViewState::Debug(_) => ViewStateKind::Debug,
        }
    }
}

/// 聊天视图状态
#[derive(Debug, Clone)]
pub struct ChatViewState {
    /// 聊天消息列表 (role, content)
    pub messages: Vec<(String, String)>,
    /// 用户输入
    pub input: String,
    /// 光标位置 (字符索引)
    pub cursor_pos: usize,
    /// 是否正在流式传输
    pub is_streaming: bool,
    /// 当前滚动偏移
    pub scroll_offset: usize,
    /// 内容总高度
    pub content_height: usize,
    /// 流式传输中的文本片段
    pub streaming_text: Option<String>,
    /// 当前Provider名称
    pub current_provider: String,
    /// 当前Model名称
    pub current_model: String,
}

impl ChatViewState {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            is_streaming: false,
            scroll_offset: 0,
            content_height: 0,
            streaming_text: None,
            current_provider: provider.to_string(),
            current_model: model.to_string(),
        }
    }
}

/// 配置视图状态
#[derive(Debug, Clone)]
pub enum ConfigViewState {
    /// 选择Provider
    SelectProvider,
    /// 编辑Provider (阶段)
    Editing(ProviderEditStage),
    /// 正在获取模型列表
    FetchingModels { error: Option<String> },
    /// 选择模型
    SelectModel(Vec<String>),
}

/// Provider编辑阶段
#[derive(Debug, Clone)]
pub enum ProviderEditStage {
    SelectProvider,       // 选择要编辑的Provider
    ApiKey,               // 编辑API Key
    Model,                // 编辑Model
    BaseUrl,              // 编辑Base URL
    CustomName,           // 编辑Custom Provider名称
    CustomUrl,            // 编辑Custom Base URL
}

impl ConfigViewState {
    pub fn new() -> Self {
        ConfigViewState::SelectProvider
    }
}

/// Debug视图状态
#[derive(Debug, Clone)]
pub struct DebugViewState {
    /// Debug日志消息
    pub messages: Vec<String>,
    /// 滚动偏移
    pub scroll_offset: usize,
    /// 内容高度
    pub content_height: usize,
}

impl DebugViewState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            content_height: 0,
        }
    }
}
```

#### 3.1.3 state/app_context.rs

```rust
use std::sync::{Arc, Mutex, mpsc};
use std::path::PathBuf;

use crate::core::{Agent, ToolRegistry};
use super::action::ViewStateKind;

/// 全局共享上下文 (线程安全)
#[derive(Clone)]
pub struct AppContext {
    pub agent: Arc<Agent>,
    pub config: Arc<ConfigManager>,
    pub tool_registry: Arc<ToolRegistry>,
    pub debug_log_path: Option<PathBuf>,
}

impl AppContext {
    pub fn new(agent: Agent, config: ConfigManager, tool_registry: Arc<ToolRegistry>, show_debug: bool) -> Self {
        let debug_log_path = if show_debug {
            let log_dir = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".fastcode");
            std::fs::create_dir_all(&log_dir).ok();
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let log_path = log_dir.join(format!("ragent_{}.log", timestamp));
            std::fs::write(&log_path, "").ok();
            Some(log_path)
        } else {
            None
        };

        Self {
            agent: Arc::new(agent),
            config: Arc::new(config),
            tool_registry,
            debug_log_path,
        }
    }
}

/// UI相关状态 (线程安全，用于跨线程通信)
pub struct UIState {
    /// 流式传输中的文本
    pub streaming_text: Arc<Mutex<Option<String>>>,
    /// Debug日志消息
    pub debug_messages: Arc<Mutex<Vec<String>>>,
    /// Agent响应接收器
    pub response_receiver: Arc<Mutex<Option<mpsc::Receiver<Result<String>>>>>,
    /// 响应回调通道 (用于发送消息给Agent)
    pub agent_command_tx: Arc<Mutex<Option<mpsc::Sender<AgentCommand>>>>,
}

impl UIState {
    pub fn new() -> Self {
        Self {
            streaming_text: Arc::new(Mutex::new(None)),
            debug_messages: Arc::new(Mutex::new(Vec::new())),
            response_receiver: Arc::new(Mutex::new(None)),
            agent_command_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// 添加debug日志
    pub fn add_debug(&self, msg: String) {
        if let Ok(mut messages) = self.debug_messages.lock() {
            messages.push(msg);
            if messages.len() > 100 {
                messages.remove(0);
            }
        }
    }

    /// 追加流式文本
    pub fn append_streaming(&self, chunk: &str) {
        if let Ok(mut streaming) = self.streaming_text.lock() {
            if let Some(ref mut text) = *streaming {
                text.push_str(chunk);
            } else {
                *streaming = Some(chunk.to_string());
            }
        }
    }

    /// 清空流式文本
    pub fn clear_streaming(&self) {
        if let Ok(mut streaming) = self.streaming_text.lock() {
            *streaming = None;
        }
    }
}

impl Default for UIState {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent命令
pub enum AgentCommand {
    SendMessage(String),
    CancelCurrentRequest,
}
```

---

### 阶段2: View Trait 定义

#### 3.2.1 views/view.rs

```rust
use ratatui::{Frame, layout::Rect};
use crossterm::event::KeyEvent;

use crate::tui::state::{AppContext, UIState};
use crate::tui::state::action::Action;

/// View接口 - 所有视图必须实现
pub trait View {
    /// 渲染视图到指定区域
    fn render(&mut self, f: &mut Frame, area: Rect, ctx: &AppContext, ui_state: &UIState);

    /// 处理按键事件，返回Action
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action>;

    /// 进入视图时调用 (可选)
    fn on_enter(&mut self) {}

    /// 离开视图时调用 (可选)
    fn on_exit(&mut self) {}
}
```

---

### 阶段3: ChatView 实现

#### 3.3.1 views/chat_view.rs

```rust
use ratatui::{
    Frame, layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    text::{Line, Span, Text},
    style::{Color, Style},
};
use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};

use crate::tui::state::{
    AppContext, UIState,
    view_state::ChatViewState,
    action::{Action, ViewStateKind},
};
use crate::tui::views::View;

pub struct ChatView {
    pub state: ChatViewState,
    scrollbar_state: ScrollbarState,
}

impl ChatView {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            state: ChatViewState::new(provider, model),
            scrollbar_state: ScrollbarState::new(0),
        }
    }

    /// 计算字符的显示宽度 (用于CJK)
    fn display_width(c: char) -> usize {
        match c {
            c if c.is_ascii() => 1,
            c if c == '\t' => 4,
            c if c >= '\u{3000}' && c <= '\u{303F}' => 2,
            c if c >= '\u{3040}' && c <= '\u{309F}' => 2,
            c if c >= '\u{30A0}' && c <= '\u{30FF}' => 2,
            _ if c >= '\u{4E00}' && c <= '\u{9FFF}' => 2,
            _ => 1,
        }
    }

    /// 计算字符串显示宽度
    fn text_width(text: &str) -> usize {
        text.chars().map(Self::display_width).sum()
    }

    /// 获取光标字节索引
    fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
        let mut char_count = 0;
        for (byte_idx, _c) in s.char_indices() {
            if char_count >= char_idx {
                return byte_idx;
            }
            char_count += 1;
        }
        s.len()
    }

    /// 移除光标前的字符
    fn remove_char_before_cursor(input: &mut String, cursor_pos: &mut usize) {
        if *cursor_pos == 0 || input.is_empty() {
            return;
        }
        let target_char_pos = *cursor_pos - 1;
        let byte_idx = Self::char_to_byte_idx(input, target_char_pos);
        input.remove(byte_idx);
        *cursor_pos = target_char_pos;
    }

    /// 渲染消息列表
    fn render_messages(&mut self, f: &mut Frame, area: Rect, ui_state: &UIState) {
        let mut lines: Vec<Line> = Vec::new();

        // 已确认的消息
        for (role, content) in &self.state.messages {
            let role_style = match role.as_str() {
                "user" => Style::new().green().bold(),
                "assistant" => Style::new().cyan().bold(),
                "system" => Style::new().yellow().bold(),
                "error" => Style::new().red().bold(),
                _ => Style::new().white(),
            };
            let role_display = format!("[{} {}]", role, self.state.current_model);
            lines.push(Line::from(Span::styled(format!("{}: ", role_display), role_style)));
            lines.push(Line::from(Span::raw(content.clone())));
        }

        // 流式传输中的消息
        if let Ok(streaming) = ui_state.streaming_text.lock() {
            if let Some(ref text) = *streaming {
                lines.push(Line::from(Span::styled(
                    format!("[assistant {}]: ", self.state.current_model),
                    Style::new().cyan().bold()
                )));
                lines.push(Line::from(Span::raw(text.clone())));
                lines.push(Line::from(Span::styled("▌", Style::new().slow_blink())));
            }
        }

        self.state.content_height = lines.len();
        let visible_height = area.height as usize;

        // 计算滚动
        let max_scroll = if self.state.content_height > visible_height {
            self.state.content_height - visible_height
        } else {
            0
        };
        if self.state.scroll_offset > max_scroll {
            self.state.scroll_offset = max_scroll;
        }
        self.scrollbar_state = ScrollbarState::new(max_scroll).position(self.state.scroll_offset);

        // 可滚动内容
        let scrollable_lines: Vec<Line> = if self.state.scroll_offset >= lines.len() {
            lines.clone()
        } else {
            lines[self.state.scroll_offset..].to_vec()
        };

        let paragraph = Paragraph::new(Text::from(scrollable_lines))
            .block(Block::default().borders(Borders::ALL).title("Chat (Ctrl+S: Settings)"))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);

        let scrollbar_area = area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 });
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            scrollbar_area,
            &mut self.scrollbar_state,
        );
    }

    /// 渲染输入框
    fn render_input(&self, f: &mut Frame, area: Rect) {
        let display_input = if self.state.input.is_empty() {
            String::from("> ")
        } else {
            let byte_pos = Self::char_to_byte_idx(&self.state.input, self.state.cursor_pos.min(self.state.input.chars().count()));
            let before = &self.state.input[..byte_pos];
            let after = &self.state.input[byte_pos..];
            format!("> {}{}▌", before, after)
        };

        let input_para = Paragraph::new(display_input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input_para, area);

        // 设置光标位置
        let char_offset: u16 = self.state.input.chars().take(self.state.cursor_pos).map(Self::display_width).sum::<usize>() as u16;
        let cursor_x = area.x + 1 + 2 + char_offset;
        let cursor_y = area.y + 1;
        f.set_cursor(cursor_x, cursor_y);
    }

    /// 渲染状态栏
    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status_text = format!(
            "[{}] - Press Ctrl+Q to quit, Ctrl+S for settings",
            self.state.current_provider
        );
        let status_para = Paragraph::new(status_text.as_str());
        f.render_widget(status_para, area);
    }
}

impl View for ChatView {
    fn render(&mut self, f: &mut Frame, area: Rect, ctx: &AppContext, ui_state: &UIState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),   // 消息区域
                Constraint::Length(3), // 输入框
                Constraint::Length(1), // 状态栏
            ])
            .split(area);

        self.render_messages(f, chunks[0], ui_state);
        self.render_input(f, chunks[1]);
        self.render_status(f, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // 流式传输中 - 限制按键
        if self.state.is_streaming {
            match key.code {
                KeyCode::Up => {
                    if self.state.scroll_offset > 0 {
                        self.state.scroll_offset -= 1;
                    }
                    return Some(Action::ScrollUp);
                }
                KeyCode::Down => {
                    let max_scroll = self.state.content_height.saturating_sub(1);
                    if self.state.scroll_offset < max_scroll {
                        self.state.scroll_offset += 1;
                    }
                    return Some(Action::ScrollDown);
                }
                KeyCode::Esc => {
                    self.state.is_streaming = false;
                    return Some(Action::CancelStreaming);
                }
                _ => return None,
            }
        }

        // 普通模式
        match (key.code, key.modifiers) {
            (KeyCode::Char(c), KeyModifiers::CONTROL) => {
                match c {
                    'q' | 'Q' => Some(Action::Quit),
                    's' | 'S' => Some(Action::SwitchView(
                        crate::tui::state::view_state::ViewState::Config(
                            crate::tui::state::view_state::ConfigViewState::new()
                        )
                    )),
                    _ => None,
                }
            }
            (KeyCode::Char(c), _) => {
                self.state.input.push(c);
                self.state.cursor_pos += 1;
                None
            }
            (KeyCode::Backspace, _) => {
                Self::remove_char_before_cursor(&mut self.state.input, &mut self.state.cursor_pos);
                None
            }
            (KeyCode::Delete, _) => {
                let char_count = self.state.input.chars().count();
                if self.state.cursor_pos < char_count && !self.state.input.is_empty() {
                    let byte_idx = Self::char_to_byte_idx(&self.state.input, self.state.cursor_pos);
                    self.state.input.remove(byte_idx);
                }
                None
            }
            (KeyCode::Left, _) => {
                if self.state.cursor_pos > 0 {
                    self.state.cursor_pos -= 1;
                }
                None
            }
            (KeyCode::Right, _) => {
                if self.state.cursor_pos < self.state.input.chars().count() {
                    self.state.cursor_pos += 1;
                }
                None
            }
            (KeyCode::Home, _) => {
                self.state.cursor_pos = 0;
                None
            }
            (KeyCode::End, _) => {
                self.state.cursor_pos = self.state.input.chars().count();
                None
            }
            (KeyCode::Up, _) => {
                if self.state.scroll_offset > 0 {
                    self.state.scroll_offset -= 1;
                }
                Some(Action::ScrollUp)
            }
            (KeyCode::Down, _) => {
                let max_scroll = self.state.content_height.saturating_sub(1);
                if self.state.scroll_offset < max_scroll {
                    self.state.scroll_offset += 1;
                }
                Some(Action::ScrollDown)
            }
            (KeyCode::Enter, _) => {
                if !self.state.input.trim().is_empty() {
                    let msg = self.state.input.trim().to_string();
                    self.state.input.clear();
                    self.state.cursor_pos = 0;
                    self.state.is_streaming = true;
                    Some(Action::SendMessage(msg))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
```

---

### 阶段4: ConfigView 实现

#### 3.4.1 views/config_view.rs

```rust
use ratatui::{
    Frame, layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    text::{Line, Span, Text},
    style::Style,
};
use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::state::{
    AppContext, UIState,
    view_state::{ConfigViewState, ProviderEditStage},
    action::Action,
};
use crate::tui::views::View;

pub struct ConfigView {
    pub state: ConfigViewState,
    input_buffer: String,
    cursor_pos: usize,
}

impl ConfigView {
    pub fn new() -> Self {
        Self {
            state: ConfigViewState::new(),
            input_buffer: String::new(),
            cursor_pos: 0,
        }
    }

    fn render_provider_list(&self, f: &mut Frame, area: Rect, current_provider: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

        f.render_widget(
            Paragraph::new(format!("Current: {}", current_provider))
                .block(Block::default().borders(Borders::ALL).title("Provider")),
            chunks[0],
        );

        let providers = vec![
            ("openai", "OpenAI"),
            ("anthropic", "Anthropic"),
            ("ollama", "Ollama"),
            ("deepseek", "DeepSeek"),
            ("minimax", "MiniMax"),
            ("custom", "Custom"),
        ];

        let mut items = Vec::new();
        for (id, name) in providers {
            let marker = if id == current_provider { ">" } else { " " };
            items.push(format!("{} [{}]", marker, name));
        }

        let list = Paragraph::new(Text::from(items.join("\n")))
            .block(Block::default().borders(Borders::ALL).title("Select Provider"));

        f.render_widget(list, chunks[1]);

        f.render_widget(
            Paragraph::new("[F] Fetch Models | [ESC] Back to Chat"),
            chunks[2],
        );
    }

    fn render_input_field(&self, f: &mut Frame, area: Rect, title: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(area);

        let input_para = Paragraph::new(self.input_buffer.as_str())
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(input_para, chunks[0]);

        let hint_para = Paragraph::new("[ENTER] Save | [ESC] Cancel");
        f.render_widget(hint_para, chunks[1]);
    }

    fn render_fetching(&self, f: &mut Frame, area: Rect, error: Option<&str>) {
        let message = if let Some(err) = error {
            format!("Error fetching models: {}", err)
        } else {
            String::from("Fetching models...")
        };

        let para = Paragraph::new(message)
            .block(Block::default().borders(Borders::ALL).title("Fetch Models"));
        f.render_widget(para, area);

        let hint = Paragraph::new("[ESC] Cancel");
        f.render_widget(hint, area);
    }

    fn render_model_list(&self, f: &mut Frame, area: Rect, models: &[String]) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        f.render_widget(
            Paragraph::new("Select Model")
                .block(Block::default().borders(Borders::ALL).title("Models")),
            chunks[0],
        );

        let list = Paragraph::new(Text::from(models.join("\n")))
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(list, chunks[1]);

        f.render_widget(
            Paragraph::new("[ENTER] Select | [ESC] Cancel"),
            chunks[2],
        );
    }
}

impl View for ConfigView {
    fn render(&mut self, f: &mut Frame, area: Rect, ctx: &AppContext, _ui_state: &UIState) {
        match &self.state {
            ConfigViewState::SelectProvider => {
                let provider = ctx.config.get_provider();
                self.render_provider_list(f, area, provider);
            }
            ConfigViewState::Editing(stage) => {
                let title = match stage {
                    ProviderEditStage::ApiKey => "API Key",
                    ProviderEditStage::Model => "Model",
                    ProviderEditStage::BaseUrl => "Base URL",
                    ProviderEditStage::CustomName => "Custom Provider Name",
                    ProviderEditStage::CustomUrl => "Custom Base URL",
                    ProviderEditStage::SelectProvider => "Select Provider",
                };
                self.render_input_field(f, area, title);
            }
            ConfigViewState::FetchingModels { error } => {
                self.render_fetching(f, area, error.as_deref());
            }
            ConfigViewState::SelectModel(models) => {
                self.render_model_list(f, area, models);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match &self.state {
            ConfigViewState::SelectProvider => {
                match key.code {
                    KeyCode::Char('1') => Some(Action::SelectProvider("openai".to_string())),
                    KeyCode::Char('2') => Some(Action::SelectProvider("anthropic".to_string())),
                    KeyCode::Char('3') => Some(Action::SelectProvider("ollama".to_string())),
                    KeyCode::Char('4') => Some(Action::SelectProvider("deepseek".to_string())),
                    KeyCode::Char('5') => Some(Action::SelectProvider("minimax".to_string())),
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        self.state = ConfigViewState::Editing(ProviderEditStage::SelectProvider);
                        None
                    }
                    KeyCode::Char('f') | KeyCode::Char('F') => Some(Action::FetchModels),
                    KeyCode::Esc => Some(Action::SwitchView(
                        crate::tui::state::view_state::ViewState::Chat(
                            crate::tui::state::view_state::ChatViewState::new(
                                ctx.config.get_provider(),
                                ctx.config.get_model(ctx.config.get_provider())
                            )
                        )
                    )),
                    _ => None,
                }
            }
            ConfigViewState::Editing(stage) => {
                match (key.code, stage) {
                    (KeyCode::Enter, _) => {
                        let action = match stage {
                            ProviderEditStage::ApiKey => Action::SaveApiKey(self.input_buffer.clone()),
                            ProviderEditStage::Model => Action::SaveModel(self.input_buffer.clone()),
                            ProviderEditStage::BaseUrl => Action::SaveBaseUrl(self.input_buffer.clone()),
                            _ => return None,
                        };
                        self.input_buffer.clear();
                        self.cursor_pos = 0;
                        Some(action)
                    }
                    (KeyCode::Esc, _) => {
                        self.input_buffer.clear();
                        self.cursor_pos = 0;
                        self.state = ConfigViewState::SelectProvider;
                        None
                    }
                    (KeyCode::Char(c), _) => {
                        self.input_buffer.push(c);
                        self.cursor_pos += 1;
                        None
                    }
                    (KeyCode::Backspace, _) => {
                        if self.cursor_pos > 0 {
                            self.cursor_pos -= 1;
                            self.input_buffer.pop();
                        }
                        None
                    }
                    _ => None,
                }
            }
            ConfigViewState::FetchingModels { .. } => {
                if let KeyCode::Esc = key.code {
                    self.state = ConfigViewState::SelectProvider;
                    None
                } else {
                    None
                }
            }
            ConfigViewState::SelectModel(_) => {
                if let KeyCode::Esc = key.code {
                    self.state = ConfigViewState::SelectProvider;
                    None
                } else {
                    None
                }
            }
        }
    }
}
```

---

### 阶段5: DebugView 实现

#### 3.5.1 views/debug_view.rs

```rust
use ratatui::{
    Frame, layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    text::Line,
};
use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::state::{AppContext, UIState};
use crate::tui::state::view_state::DebugViewState;
use crate::tui::state::action::Action;
use crate::tui::views::View;

pub struct DebugView {
    pub state: DebugViewState,
    scrollbar_state: ScrollbarState,
}

impl DebugView {
    pub fn new() -> Self {
        Self {
            state: DebugViewState::new(),
            scrollbar_state: ScrollbarState::new(0),
        }
    }
}

impl View for DebugView {
    fn render(&mut self, f: &mut Frame, area: Rect, _ctx: &AppContext, ui_state: &UIState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        // 获取debug消息
        let messages: Vec<String> = if let Ok(guard) = ui_state.debug_messages.lock() {
            guard.clone()
        } else {
            Vec::new()
        };

        self.state.content_height = messages.len();
        let visible_height = chunks[0].height as usize;

        let max_scroll = if self.state.content_height > visible_height {
            self.state.content_height - visible_height
        } else {
            0
        };
        if self.state.scroll_offset > max_scroll {
            self.state.scroll_offset = max_scroll;
        }
        self.scrollbar_state = ScrollbarState::new(max_scroll).position(self.state.scroll_offset);

        let visible_lines: Vec<Line> = if self.state.scroll_offset >= messages.len() {
            messages.iter().map(|m| Line::from(m.clone())).collect()
        } else {
            messages[self.state.scroll_offset..]
                .iter()
                .map(|m| Line::from(m.clone()))
                .collect()
        };

        let paragraph = Paragraph::new(visible_lines.into())
            .block(Block::default().borders(Borders::ALL).title("Debug Log (↑↓ scroll)"))
            .style(Style::default().fg(Color::Red));

        f.render_widget(paragraph, chunks[0]);

        let scrollbar_area = chunks[0].inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 });
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            scrollbar_area,
            &mut self.scrollbar_state,
        );

        let hint = Paragraph::new("[ESC] Back to Chat");
        f.render_widget(hint, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Up => {
                if self.state.scroll_offset > 0 {
                    self.state.scroll_offset -= 1;
                }
                Some(Action::ScrollDebugUp)
            }
            KeyCode::Down => {
                let max_scroll = self.state.content_height.saturating_sub(1);
                if self.state.scroll_offset < max_scroll {
                    self.state.scroll_offset += 1;
                }
                Some(Action::ScrollDebugDown)
            }
            KeyCode::Esc => Some(Action::SwitchView(
                crate::tui::state::view_state::ViewState::Chat(
                    crate::tui::state::view_state::ChatViewState::new("deepseek", "deepseek-chat")
                )
            )),
            _ => None,
        }
    }
}
```

---

### 阶段6: AppController 实现

#### 3.6.1 app.rs 重构

```rust
use anyhow::Result;
use crossterm::{event::{self, Event, KeyCode}, execute};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use std::io::Stdout;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tokio::runtime::Builder;

use crate::providers::{OpenAIProvider, AnthropicProvider, OllamaProvider, DeepSeekProvider, MiniMaxProvider, CustomProvider};
use crate::tools::{BashTool, ReadTool, WriteTool, EditTool, GrepTool};
use crate::core::{Agent, AgentConfig, AgentRole, ToolRegistry};

use crate::tui::state::{
    AppContext, UIState,
    view_state::{ViewState, ChatViewState},
    action::{Action, ViewStateKind},
};
use crate::tui::views::{View, chat_view::ChatView, config_view::ConfigView, debug_view::DebugView};

pub struct AppController {
    context: AppContext,
    ui_state: UIState,
    runtime: tokio::runtime::Runtime,
    current_view: Box<dyn View>,
}

impl AppController {
    /// 创建新的AppController
    pub fn new(show_debug: bool) -> Self {
        // 创建 runtime (单例)
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        // 创建 ToolRegistry
        let mut tool_registry = ToolRegistry::new();
        tool_registry.register(BashTool::new());
        tool_registry.register(ReadTool::new());
        tool_registry.register(WriteTool::new());
        tool_registry.register(EditTool::new());
        tool_registry.register(GrepTool::new());
        let tool_registry = Arc::new(tool_registry);

        // 创建 Agent
        let agent_config = AgentConfig {
            name: "tui-agent".to_string(),
            role: AgentRole::Executor,
            model: None,
            temperature: 0.7,
        };
        let agent = Agent::new(agent_config)
            .with_tool_registry(Arc::clone(&tool_registry));

        // 创建 ConfigManager
        let config = crate::tui::config::ConfigManager::new();

        // 创建 Context 和 UIState
        let context = AppContext::new(agent, config, tool_registry, show_debug);
        let ui_state = UIState::new();

        // 默认 ChatView
        let current_view: Box<dyn View> = Box::new(ChatView::new(
            context.config.get_provider(),
            context.config.get_model(context.config.get_provider()),
        ));

        Self {
            context,
            ui_state,
            runtime,
            current_view,
        }
    }

    /// 启动应用
    pub fn run(&mut self) -> Result<()> {
        let mut terminal = self.setup_terminal()?;

        self.log("Application started");

        loop {
            // 渲染
            terminal.draw(|f| {
                self.current_view.render(f, f.size(), &self.context, &self.ui_state);
            })?;

            // 处理事件
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if let Some(action) = self.current_view.handle_key(key) {
                        self.handle_action(action)?;

                        // 检查是否退出
                        if self.should_quit() {
                            return;
                        }
                    }
                }
            }

            // 处理Agent响应
            self.process_agent_response();
        }

        self.cleanup_terminal()?;
        Ok(())
    }

    /// 处理Action
    fn handle_action(&mut self, action: Action) -> Result<()> {
        self.log(&format!("Action: {:?}", action));

        match action {
            Action::SendMessage(msg) => {
                self.state.add_message("user", &msg);
                self.spawn_agent_task(msg);
            }
            Action::SwitchView(view_state) => {
                self.current_view.on_exit();
                self.current_view = match view_state {
                    ViewState::Chat(state) => {
                        let mut view = ChatView::new(
                            &state.current_provider,
                            &state.current_model,
                        );
                        view.state = state;
                        Box::new(view)
                    }
                    ViewState::Config(state) => {
                        let mut view = ConfigView::new();
                        view.state = state;
                        Box::new(view)
                    }
                    ViewState::Debug(state) => {
                        let mut view = DebugView::new();
                        view.state = state;
                        Box::new(view)
                    }
                };
                self.current_view.on_enter();
            }
            Action::GoBack => {
                // 根据当前视图决定返回到哪里
                self.current_view.on_exit();
                self.current_view = Box::new(ChatView::new(
                    self.context.config.get_provider(),
                    self.context.config.get_model(self.context.config.get_provider()),
                ));
                self.current_view.on_enter();
            }
            Action::CancelStreaming => {
                self.ui_state.clear_streaming();
                // 发送取消命令给Agent
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::SelectProvider(name) => {
                self.context.config.set_provider(&name);
                // 更新当前视图的provider信息
            }
            Action::SaveApiKey(key) => {
                let provider = self.context.config.get_provider();
                self.context.config.set_api_key(&provider, Some(key));
            }
            Action::SaveModel(model) => {
                let provider = self.context.config.get_provider();
                self.context.config.set_model(&provider, model);
            }
            Action::SaveBaseUrl(url) => {
                let provider = self.context.config.get_provider();
                self.context.config.set_base_url(&provider, Some(url));
            }
            Action::FetchModels => {
                self.spawn_fetch_models();
            }
            Action::ToggleDebug => {
                // 切换debug视图
            }
            // ... 其他action
            _ => {}
        }

        Ok(())
    }

    /// 生成Agent任务
    fn spawn_agent_task(&mut self, msg: String) {
        let context = self.context.clone();
        let ui_state = &self.ui_state;
        let runtime = &self.runtime;

        // 创建响应channel
        let (tx, rx) = mpsc::channel();
        *ui_state.response_receiver.lock().unwrap() = Some(rx);

        // 清空流式文本
        ui_state.clear_streaming();

        runtime.spawn(async move {
            let provider_name = context.config.get_provider();
            let api_key = context.config.get_api_key(&provider_name);
            let base_url = context.config.get_base_url(&provider_name);
            let model = Some(context.config.get_model(&provider_name));

            let mut agent = context.agent.clone();

            // 根据provider创建tool provider
            match provider_name.as_str() {
                "openai" => {
                    let p = OpenAIProvider::new(api_key.unwrap_or_default(), base_url);
                    agent = agent.with_tool_provider("openai", p);
                }
                "anthropic" => {
                    let p = AnthropicProvider::new(api_key.unwrap_or_default(), base_url);
                    agent = agent.with_tool_provider("anthropic", p);
                }
                "ollama" => {
                    let p = OllamaProvider::new(base_url, model);
                    agent = agent.with_tool_provider("ollama", p);
                }
                "deepseek" => {
                    let p = DeepSeekProvider::new(api_key.unwrap_or_default(), base_url, model);
                    agent = agent.with_tool_provider("deepseek", p);
                }
                "minimax" => {
                    let p = MiniMaxProvider::new(api_key.unwrap_or_default(), base_url, model);
                    agent = agent.with_tool_provider("minimax", p);
                }
                "custom" => {
                    let p = CustomProvider::new(
                        "custom".to_string(),
                        api_key.unwrap_or_default(),
                        base_url.unwrap_or_default(),
                        model.unwrap_or_default(),
                    );
                    agent = agent.with_tool_provider("custom", p);
                }
                _ => {}
            }

            // 追加debug日志
            ui_state.add_debug(format!("Starting request to {}", provider_name));

            // 调用agent loop
            match agent.run_agent_loop(
                &msg,
                5,
                context.debug_log_path.clone().map(|p| p.to_string_lossy().to_string()),
                None,
                Some(tx),
            ).await {
                Ok(response) => {
                    ui_state.add_debug(format!("Response length: {}", response.len()));
                }
                Err(e) => {
                    ui_state.add_debug(format!("Error: {}", e));
                }
            }
        });
    }

    /// 处理Agent响应
    fn process_agent_response(&self) {
        if let Ok(receiver) = self.ui_state.response_receiver.lock() {
            if let Some(ref rx) = *receiver {
                while let Ok(result) = rx.try_recv() {
                    match result {
                        Ok(chunk) => {
                            self.ui_state.append_streaming(&chunk);
                        }
                        Err(e) => {
                            // 处理错误
                            if let Ok(mut streaming) = self.ui_state.streaming_text.lock() {
                                let partial = streaming.take().unwrap_or_default();
                                self.state.add_message("error", &format!("{} (partial: {})", e, partial));
                            }
                        }
                    }
                }
            }
        }
    }

    fn setup_terminal(&self) -> Result<Terminal<CrosstermBackend<Stdout>>> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        Ok(Terminal::new(backend))
    }

    fn cleanup_terminal(&self) -> Result<()> {
        execute!(std::io::stdout(), LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    fn log(&self, msg: &str) {
        if let Some(ref path) = self.context.debug_log_path {
            let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
            let formatted = format!("[{}] {}", timestamp, msg);
            let _ = std::fs::OpenOptions::new()
                .append(true)
                .open(path)
                .and_then(|mut f| {
                    use std::io::Write;
                    writeln!(f, "{}", formatted)
                });
        }
        self.ui_state.add_debug(msg.to_string());
    }

    fn should_quit(&self) -> bool {
        // 从 ChatViewState 获取 quit 状态
        // 简化版本直接返回 false
        false
    }
}
```

---

## 四、模块依赖关系

```
app.rs (AppController)
    │
    ├── state/
    │   ├── action.rs (Action, ViewStateKind)
    │   ├── view_state.rs (ViewState, ChatViewState, ConfigViewState, DebugViewState)
    │   └── app_context.rs (AppContext, UIState)
    │
    ├── views/
    │   ├── view.rs (View trait)
    │   ├── chat_view.rs (ChatView)
    │   ├── config_view.rs (ConfigView)
    │   └── debug_view.rs (DebugView)
    │
    └── components/ (复用组件)
```

---

## 五、关键设计决策说明

### 1. 为什么 ViewState 作为参数传递？

`SwitchView(ViewState)` 包含完整的目标状态，这样：
- 新View可以直接使用传入的状态，无需额外初始化
- 返回时可以从旧View获取状态恢复

### 2. 为什么 UIState 用 Arc<Mutex>？

- `streaming_text` 和 `debug_messages` 需要跨线程读写
- `response_receiver` 需要在主线程和Agent线程间共享
- 使用 `Arc<Mutex>` 确保线程安全

### 3. 为什么 AppController 持有 runtime？

- 避免每次请求创建新Runtime
- Runtime生命周期清晰 (App退出时统一清理)
- 可以用 `JoinHandle` 追踪任务

---

## 六、待实现补充

### 1. components/ 模块

需要补充以下组件的具体实现：
- `message_list.rs` - 消息列表的渲染逻辑提取
- `input_area.rs` - 输入框的渲染和状态管理
- `scroll_bar.rs` - 滚动条组件封装
- `status_bar.rs` - 状态栏组件

### 2. Debug视图切换

需要在 `app.rs` 中添加 `ToggleDebug` action 的处理逻辑，切换到 DebugView。

### 3. 配置保存后返回

配置完成后需要返回 ChatView。

### 4. 模型列表获取

`FetchModels` action 需要在 AppController 中实现获取模型列表的逻辑。

---

## 七、验证清单

实现完成后需要验证：

- [ ] `cargo build` 编译通过
- [ ] ChatView 正常显示消息
- [ ] 输入框可以正常输入文字
- [ ] Enter 键发送消息
- [ ] Ctrl+S 切换到配置页面
- [ ] Ctrl+Q 退出应用
- [ ] 流式传输正常显示
- [ ] Debug面板正常切换

---

*文档状态: 待确认后实现*
*工作目录: `.worktrees/tui-refactor/`*