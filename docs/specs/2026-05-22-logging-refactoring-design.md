# 日志重构设计文档

> 文档版本: 1.0
> 生成日期: 2026-05-22
> 分支: feature/tui-refactor
> Worktree: `.worktrees/feat-tui-refactor/`
> 状态: **初稿 - 待审批**

---

## 一、设计目标

### 1.1 问题描述

当前日志系统存在以下问题：

| 问题 | 说明 |
|------|------|
| 分散实现 | 各模块独立实现文件写入（deepseek.rs、agent.rs、app_controller.rs） |
| 格式不统一 | 时间戳格式、标签格式不一致 |
| 无日志级别 | 所有日志同等对待，无法按级别过滤 |
| 难以配置 | 日志路径和级别硬编码或通过环境变量传递 |
| 无统一接口 | 缺少 `log/` 模块作为日志系统的入口 |

### 1.2 目标

- **独立模块**：`src/log/` 作为统一的日志系统
- **仅文件输出**：不打印到控制台，减少 I/O 开销
- **多级别支持**：TRACE、DEBUG、INFO、WARN、ERROR
- **全局宏简化**：通过 `log::info!()` 等宏简化调用
- **单一日志文件**：所有模块日志写入同一文件，便于追踪

---

## 二、架构设计

### 2.1 目录结构

```
src/log/
├── mod.rs           # Log, FileWriter, 全局宏导出
├── level.rs         # LogLevel 枚举
└── writer.rs        # FileWriter 单例实现
```

### 2.2 核心类型

#### LogLevel 枚举

```rust
// log/level.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    TRACE = 0,
    DEBUG = 1,
    INFO = 2,
    WARN = 3,
    ERROR = 4,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Option<Self>;
    pub fn to_str(&self) -> &'static str;
}
```

#### FileWriter 单例

```rust
// log/writer.rs
pub struct FileWriter {
    file: Arc<Mutex<File>>,
    level: LogLevel,
}

impl FileWriter {
    pub fn new(path: PathBuf, level: LogLevel) -> Result<Self>;
    pub fn write(&self, level: LogLevel, msg: &str);
}

impl Clone for FileWriter {
    fn clone(&self) -> Self {
        Self {
            file: Arc::clone(&self.file),
            level: self.level,
        }
    }
}
```

#### Log 结构体

```rust
// log/mod.rs
pub struct Log {
    writer: Arc<FileWriter>,
    module: String,
}

impl Log {
    pub fn new(writer: Arc<FileWriter>, module: impl Into<String>) -> Self {
        Self {
            writer,
            module: module.into(),
        }
    }

    pub fn trace(&self, msg: &str) { self.log(LogLevel::TRACE, msg); }
    pub fn debug(&self, msg: &str) { self.log(LogLevel::DEBUG, msg); }
    pub fn info(&self, msg: &str) { self.log(LogLevel::INFO, msg); }
    pub fn warn(&self, msg: &str) { self.log(LogLevel::WARN, msg); }
    pub fn error(&self, msg: &str) { self.log(LogLevel::ERROR, msg); }

    fn log(&self, level: LogLevel, msg: &str) {
        if level >= self.writer.level {
            self.writer.write(level, &format!("[{}] {}", self.module, msg));
        }
    }
}
```

### 2.3 全局宏

```rust
// log/mod.rs
#[macro_export]
macro_rules! log {
    ($level:expr, $module:expr, $($arg:tt)*) => {{
        $crate::log::Log::new(
            $crate::log::GLOBAL_WRITER.clone(),
            $module
        ).log($level, &format!($($arg)*));
    }};
}

macro_rules! trace {
    ($module:expr, $($arg:tt)*) => { log!($crate::log::LogLevel::TRACE, $module, $($arg)*); };
}
macro_rules! debug {
    ($module:expr, $($arg:tt)*) => { log!($crate::log::LogLevel::DEBUG, $module, $($arg)*); };
}
macro_rules! info {
    ($module:expr, $($arg:tt)*) => { log!($crate::log::LogLevel::INFO, $module, $($arg)*); };
}
macro_rules! warn {
    ($module:expr, $($arg:tt)*) => { log!($crate::log::LogLevel::WARN, $module, $($arg)*); };
}
macro_rules! error {
    ($module:expr, $($arg:tt)*) => { log!($crate::log::LogLevel::ERROR, $module, $($arg)*); };
}
```

### 2.4 全局初始化

```rust
// log/mod.rs
static GLOBAL_WRITER: OnceLock<Arc<FileWriter>> = OnceLock::new();

pub fn init_log(path: PathBuf, level: LogLevel) -> Result<()> {
    let writer = FileWriter::new(path, level)?;
    GLOBAL_WRITER.set(Arc::new(writer)).ok();
    Ok(())
}

pub fn global_writer() -> &'static Arc<FileWriter> {
    GLOBAL_WRITER.get().expect("Log not initialized")
}
```

---

## 三、日志格式

### 3.1 日志行格式

```
[HH:MM:SS.mmm] [LEVEL] [MODULE] message
```

示例：
```
[14:28:49.111] [INFO] [agent] Agent started with model: deepseek-chat
[14:28:49.234] [DEBUG] [provider] chat_with_tools_streaming called
[14:28:50.567] [ERROR] [tool] Tool 'bash' execution failed: timeout
```

### 3.2 多行消息

对于多行消息（如 JSON 请求体），每行前面加上相同的标签：

```
[14:28:49.111] [DEBUG] [provider] REQUEST:
[14:28:49.111] [DEBUG] [provider] {
[14:28:49.111] [DEBUG] [provider]   "model": "deepseek-chat",
[14:28:49.111] [DEBUG] [provider]   "messages": [...]
[14:28:49.111] [DEBUG] [provider] }
```

---

## 四、使用示例

### 4.1 模块级日志

```rust
// deepseek.rs
use crate::log::{info, debug, error, LogLevel};

pub struct DeepSeekProvider {
    // ...
}

impl DeepSeekProvider {
    pub async fn stream_chat_with_tools(&self, ...) -> Result<()> {
        debug!("[provider:deepseek]", "stream_chat_with_tools called with {} tools", tools.len());
        // ...
        info!("[provider:deepseek]", "Stream completed, {} tool_calls", tool_count);
        Ok(())
    }
}
```

### 4.2 Agent 级日志

```rust
// agent.rs
use crate::log::{info, debug, error, LogLevel};

impl Agent {
    pub async fn run_agent_loop(&self, ...) -> Result<String> {
        info!("[agent:{}]", "Agent loop started", self.config.name);
        // ...
        debug!("[agent:{}]", "Iteration {} complete", i + 1);
        // ...
        Ok(result)
    }
}
```

### 4.3 TUI 级日志

```rust
// app_controller.rs
use crate::log::{info, debug, error, LogLevel};

impl AppController {
    pub fn new() -> Self {
        info!("[tui]", "Application started");
        // ...
    }

    fn handle_action(&mut self, action: Action) -> Result<()> {
        debug!("[tui]", "Action: {:?}", action);
        // ...
    }
}
```

---

## 五、初始化流程

### 5.1 初始化时机

在 `main.rs` 中，应用启动时立即初始化日志系统：

```rust
// main.rs
use rust_agent_platform::log::{init_log, LogLevel};

fn main() -> anyhow::Result<()> {
    // 创建日志目录
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".fastcode");
    std::fs::create_dir_all(&log_dir)?;

    // 初始化日志
    let log_path = log_dir.join("ragent.log");
    init_log(log_path, LogLevel::DEBUG)?;

    tracing::info!("Starting Rust Agent Platform...");
    // ...
}
```

### 5.2 移除现有调试代码

重构后移除：
- `deepseek.rs` 中的 `write_log()`、`log_request()` 等
- `agent.rs` 中的内联 `log` closure
- `app_controller.rs` 中的 `log()` 方法
- `main.rs` 中的 `--debug` 参数

---

## 六、TUI Debug Panel 移除

### 6.1 移除内容

| 文件 | 移除内容 |
|------|----------|
| `main.rs` | `--debug` 参数 |
| `tui/app_context.rs` | `debug_log_path: Option<PathBuf>` 字段 |
| `tui/app_controller.rs` | `show_debug` 参数、`log()` 方法 |
| `tui/views/chat_view.rs` | `show_debug` 检查、`render_debug_panel()` 调用 |
| `tui/views/debug_view.rs` | 整个 DebugView 文件 |

### 6.2 Action 移除

```rust
// 移除以下 Action
ToggleDebug,
ScrollDebugUp,
ScrollDebugDown,
```

### 6.3 ViewState 简化

```rust
pub enum ViewState {
    Chat(ChatViewState),
    Config(ConfigViewState),
    // DebugView 移除
}

pub enum ViewStateKind {
    Chat,
    Config,
    // Debug 移除
}
```

---

## 七、重构任务清单

| 任务 | 描述 | 优先级 |
|------|------|--------|
| 1. 创建 `src/log/` 模块 | 实现 Log、FileWriter、日志级别 | 高 |
| 2. 初始化全局日志 | 在 main.rs 中初始化 | 高 |
| 3. 重构 deepseek.rs | 移除自定义日志，使用统一宏 | 中 |
| 4. 重构 agent.rs | 移除内联日志 closure | 中 |
| 5. 重构 app_controller.rs | 移除 log() 方法和 debug 相关代码 | 中 |
| 6. 移除 TUI Debug Panel | 移除 DebugView、ToggleDebug Action | 中 |
| 7. 移除 main.rs --debug 参数 | 移除 debug flag | 中 |

---

## 八、验证清单

- [ ] `cargo build` 编译通过
- [ ] 日志文件正常生成
- [ ] 各模块日志正确写入
- [ ] 日志级别过滤生效
- [ ] TUI Debug Panel 完全移除
- [ ] `--debug` 参数移除

---

*文档状态: 待审批*