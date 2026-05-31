# swiftforge-log 静态库实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建 `swiftforge-log` 静态库，替换当前 `tracing` 日志系统，移除 TUI Debug Panel

**Architecture:** 创建独立静态库 `libs/swiftforge-log/`，作为 workspace 成员。替换 `tracing` 为自定义文件日志，提供 `log!()`, `info!()` 等宏。TUI Debug Panel 代码全部移除。

**Tech Stack:** Rust 2021, chrono (时间格式化)

---

## 文件结构

```
SwiftForge/
├── Cargo.toml (workspace)              # 添加 swiftforge-log 到 members
├── libs/
│   └── swiftforge-log/                 # 新建静态库
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   ├── level.rs
│       │   ├── writer.rs
│       │   ├── formater.rs
│       │   └── macro.rs
│       └── README.md
└── swiftforge/                         # 主应用
    ├── Cargo.toml                      # 添加 swiftforge-log 依赖
    └── src/
        ├── main.rs                     # 修改：移除 tracing，使用 swiftforge-log
        ├── lib.rs
        └── ... (重构的模块)

```

---

## Task 1: 创建 swiftforge-log 库骨架

**Files:**
- Create: `libs/swiftforge-log/Cargo.toml`
- Create: `libs/swiftforge-log/src/lib.rs`

- [ ] **Step 1: 创建 libs/swiftforge-log/ 目录**

Run: `mkdir -p libs/swiftforge-log/src`

- [ ] **Step 2: 创建 Cargo.toml**

```toml
# libs/swiftforge-log/Cargo.toml
[package]
name = "swiftforge-log"
version = "0.1.0"
edition = "2021"
description = "A lightweight file-based logging library for Rust applications"
license = "MIT OR Apache-2.0"

[lib]
name = "swiftforge_log"
crate-type = ["staticlib"]          # 静态链接库

[dependencies]
chrono = "0.4"
```

- [ ] **Step 3: 创建 lib.rs 骨架**

```rust
// libs/swiftforge-log/src/lib.rs

mod level;
mod writer;
mod formater;
mod macro_def;

pub use level::LogLevel;
pub use writer::FileWriter;
pub use formater::Formatter;
```

- [ ] **Step 4: 添加到 workspace Cargo.toml**

Modify: `.worktrees/feat-tui-refactor/Cargo.toml`

```toml
[workspace]
members = [
    "libs/swiftforge-types",
    "libs/swiftforge-task",
    "libs/swiftforge-tools",
    "libs/swiftforge-mcp",
    "libs/swiftforge-hooks",
    "libs/swiftforge-skill",
    "libs/swiftforge-provider-core",
    "libs/swiftforge-providers",
    "libs/swiftforge-log",           # 新增
    "swiftforge",
]
```

Run: `cargo build --workspace -p swiftforge-log` 验证编译通过

- [ ] **Step 5: Commit**

```bash
git add libs/swiftforge-log/
git commit -m "feat(log): add swiftforge-log static library skeleton"
```

---

## Task 2: 实现 LogLevel 枚举

**Files:**
- Create: `libs/swiftforge-log/src/level.rs`

- [ ] **Step 1: 创建 level.rs**

```rust
// libs/swiftforge-log/src/level.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    TRACE = 0,
    DEBUG = 1,
    INFO  = 2,
    WARN  = 3,
    ERROR = 4,
}

impl LogLevel {
    /// 从 &str 解析级别
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Some(Self::TRACE),
            "DEBUG" => Some(Self::DEBUG),
            "INFO"  => Some(Self::INFO),
            "WARN"  | "WARNING" => Some(Self::WARN),
            "ERROR" => Some(Self::ERROR),
            _ => None,
        }
    }

    /// 级别对应的字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TRACE => "TRACE",
            Self::DEBUG => "DEBUG",
            Self::INFO  => "INFO",
            Self::WARN  => "WARN",
            Self::ERROR => "ERROR",
        }
    }

    /// 默认级别（INFO）
    pub fn default() -> Self {
        Self::INFO
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::INFO
    }
}
```

- [ ] **Step 2: 更新 lib.rs 导出**

Modify: `libs/swiftforge-log/src/lib.rs`

```rust
mod level;
mod writer;
mod formater;
mod macro_def;

pub use level::LogLevel;    // 添加这行
pub use writer::FileWriter;
pub use formater::Formatter;
```

- [ ] **Step 3: 验证编译**

Run: `cargo build -p swiftforge-log`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add libs/swiftforge-log/src/level.rs libs/swiftforge-log/src/lib.rs
git commit -m "feat(log): implement LogLevel enum"
```

---

## Task 3: 实现 FileWriter

**Files:**
- Create: `libs/swiftforge-log/src/writer.rs`

- [ ] **Step 1: 创建 writer.rs**

```rust
// libs/swiftforge-log/src/writer.rs

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::level::LogLevel;

/// 文件写入器 - 线程安全单例
pub struct FileWriter {
    file: Arc<Mutex<File>>,
    level: LogLevel,
    path: PathBuf,
}

impl FileWriter {
    /// 创建新的 FileWriter
    pub fn new(path: PathBuf, level: LogLevel) -> std::io::Result<Self> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 打开或创建文件（append 模式）
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            level,
            path,
        })
    }

    /// 写入日志
    pub fn write(&self, level: LogLevel, msg: &str) {
        if level < self.level {
            return; // 低于阈值，跳过
        }

        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
        let line = format!("[{}] [{}] {}\n", timestamp, level.as_str(), msg);

        if let Ok(mut guard) = self.file.lock() {
            let _ = guard.write_all(line.as_bytes());
            let _ = guard.flush(); // 立即刷写，保证持久化
        }
    }

    /// 检查级别是否启用
    pub fn is_enabled(&self, level: LogLevel) -> bool {
        level >= self.level
    }

    /// 获取日志文件路径
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// 获取当前级别
    pub fn level(&self) -> LogLevel {
        self.level
    }
}

impl Clone for FileWriter {
    fn clone(&self) -> Self {
        Self {
            file: Arc::clone(&self.file),
            level: self.level,
            path: self.path.clone(),
        }
    }
}
```

- [ ] **Step 2: 更新 lib.rs 导出**

Modify: `libs/swiftforge-log/src/lib.rs`

```rust
mod level;
mod writer;
mod formater;
mod macro_def;

pub use level::LogLevel;
pub use writer::FileWriter;   // 添加这行
pub use formater::Formatter;
```

- [ ] **Step 3: 验证编译**

Run: `cargo build -p swiftforge-log`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add libs/swiftforge-log/src/writer.rs libs/swiftforge-log/src/lib.rs
git commit -m "feat(log): implement FileWriter"
```

---

## Task 4: 实现 Formatter

**Files:**
- Create: `libs/swiftforge-log/src/formater.rs`

- [ ] **Step 1: 创建 formater.rs**

```rust
// libs/swiftforge-log/src/formater.rs

use chrono::Local;
use crate::level::LogLevel;

/// 日志格式化器
pub struct Formatter {
    _private: (),  // 禁止直接构造
}

impl Formatter {
    /// 创建格式化器
    pub fn new() -> Self {
        Self {
            _private: (),
        }
    }

    /// 格式化为单行字符串
    pub fn format(&self, level: LogLevel, module: Option<&str>, msg: &str) -> String {
        let timestamp = Local::now().format("%H:%M:%S%.3f");
        let module_str = module
            .map(|m| format!(" [{}] ", m))
            .unwrap_or_else(|| " ".to_string());

        format!("[{}] [{}]{}{}", timestamp, level.as_str(), module_str, msg)
    }

    /// 格式化多行消息（每行加前缀）
    pub fn format_multiline(&self, level: LogLevel, module: Option<&str>, msg: &str) -> Vec<String> {
        let prefix = format!(
            "[{}] [{}]{}",
            Local::now().format("%H:%M:%S%.3f"),
            level.as_str(),
            module.map(|m| format!(" [{}] ", m)).unwrap_or_else(|| " ".to_string())
        );

        msg.lines()
            .map(|line| format!("{}{}", prefix, line))
            .collect()
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: 更新 lib.rs 导出**

Modify: `libs/swiftforge-log/src/lib.rs`

```rust
mod level;
mod writer;
mod formater;
mod macro_def;

pub use level::LogLevel;
pub use writer::FileWriter;
pub use formater::Formatter;  // 添加这行
```

- [ ] **Step 3: 验证编译**

Run: `cargo build -p swiftforge-log`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add libs/swiftforge-log/src/formater.rs libs/swiftforge-log/src/lib.rs
git commit -m "feat(log): implement Formatter"
```

---

## Task 5: 实现宏定义

**Files:**
- Create: `libs/swiftforge-log/src/macro.rs`

- [ ] **Step 1: 创建 macro.rs**

```rust
// libs/swiftforge-log/src/macro.rs

#[macro_export]
macro_rules! log {
    ($level:expr, $module:expr, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        if let Some(w) = $crate::GLOBAL_WRITER.get() {
            w.write($level, &format!("[{}] {}", $module, msg));
        }
    }};
}

/// 记录 TRACE 级别日志
#[macro_export]
macro_rules! trace {
    ($module:expr, $($arg:tt)*) => {{
        $crate::log!($crate::LogLevel::TRACE, $module, $($arg)*);
    }};
}

/// 记录 DEBUG 级别日志
#[macro_export]
macro_rules! debug {
    ($module:expr, $($arg:tt)*) => {{
        $crate::log!($crate::LogLevel::DEBUG, $module, $($arg)*);
    }};
}

/// 记录 INFO 级别日志
#[macro_export]
macro_rules! info {
    ($module:expr, $($arg:tt)*) => {{
        $crate::log!($crate::LogLevel::INFO, $module, $($arg)*);
    }};
}

/// 记录 WARN 级别日志
#[macro_export]
macro_rules! warn {
    ($module:expr, $($arg:tt)*) => {{
        $crate::log!($crate::LogLevel::WARN, $module, $($arg)*);
    }};
}

/// 记录 ERROR 级别日志
#[macro_export]
macro_rules! error {
    ($module:expr, $($arg:tt)*) => {{
        $crate::log!($crate::LogLevel::ERROR, $module, $($arg)*);
    }};
}
```

- [ ] **Step 2: 更新 lib.rs 完整实现**

Modify: `libs/swiftforge-log/src/lib.rs`

```rust
// libs/swiftforge-log/src/lib.rs

mod level;
mod writer;
mod formater;
mod macro_def;

pub use level::LogLevel;
pub use writer::FileWriter;
pub use formater::Formatter;

// ============================================================================
// 全局日志系统
// ============================================================================

use std::sync::{Arc, OnceLock};

/// 全局 FileWriter 实例
static GLOBAL_WRITER: OnceLock<Arc<FileWriter>> = OnceLock::new();

/// 全局格式化器
static GLOBAL_FORMATTER: OnceLock<Formatter> = OnceLock::new();

/// 初始化日志系统（幂等调用）
pub fn init_log(path: std::path::PathBuf, level: LogLevel) -> std::io::Result<()> {
    let writer = FileWriter::new(path, level)?;
    let formatter = Formatter::new();

    GLOBAL_WRITER.set(Arc::new(writer)).ok();
    GLOBAL_FORMATTER.set(formatter).ok();

    Ok(())
}

/// 获取全局 FileWriter（必须先调用 init_log）
pub fn global_writer() -> &'static Arc<FileWriter> {
    GLOBAL_WRITER.get().expect("Log not initialized. Call init_log() first.")
}

/// 获取全局 Formatter
pub fn global_formatter() -> &'static Formatter {
    GLOBAL_FORMATTER.get().expect("Log not initialized. Call init_log() first.")
}

/// 检查日志级别是否启用
pub fn is_level_enabled(level: LogLevel) -> bool {
    GLOBAL_WRITER
        .get()
        .map(|w| w.is_enabled(level))
        .unwrap_or(false)
}

/// 获取日志文件路径
pub fn log_path() -> Option<std::path::PathBuf> {
    GLOBAL_WRITER.get().map(|w| w.path().clone())
}

// ============================================================================
// 简化接口（不需要宏的场景）
// ============================================================================

/// 记录 TRACE 级别日志
pub fn trace(module: &str, msg: &str) {
    if let Some(w) = GLOBAL_WRITER.get() {
        w.write(LogLevel::TRACE, &format!("[{}] {}", module, msg));
    }
}

/// 记录 DEBUG 级别日志
pub fn debug(module: &str, msg: &str) {
    if let Some(w) = GLOBAL_WRITER.get() {
        w.write(LogLevel::DEBUG, &format!("[{}] {}", module, msg));
    }
}

/// 记录 INFO 级别日志
pub fn info(module: &str, msg: &str) {
    if let Some(w) = GLOBAL_WRITER.get() {
        w.write(LogLevel::INFO, &format!("[{}] {}", module, msg));
    }
}

/// 记录 WARN 级别日志
pub fn warn(module: &str, msg: &str) {
    if let Some(w) = GLOBAL_WRITER.get() {
        w.write(LogLevel::WARN, &format!("[{}] {}", module, msg));
    }
}

/// 记录 ERROR 级别日志
pub fn error(module: &str, msg: &str) {
    if let Some(w) = GLOBAL_WRITER.get() {
        w.write(LogLevel::ERROR, &format!("[{}] {}", module, msg));
    }
}

// ============================================================================
// 公共宏定义
// ============================================================================

pub use macro_def::{log, trace, debug, info, warn, error};
```

- [ ] **Step 3: 验证编译**

Run: `cargo build -p swiftforge-log`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add libs/swiftforge-log/src/macro.rs libs/swiftforge-log/src/lib.rs
git commit -m "feat(log): implement logging macros"
```

---

## Task 6: 在 swiftforge 中依赖 swiftforge-log

**Files:**
- Modify: `swiftforge/Cargo.toml`
- Modify: `swiftforge/src/main.rs`

- [ ] **Step 1: 添加依赖到 swiftforge/Cargo.toml**

Modify: `swiftforge/Cargo.toml`

```toml
[dependencies]
swiftforge-log = { path = "../libs/swiftforge-log" }
# ... 其他依赖保持不变 ...
```

Run: `cargo build -p swiftforge` 验证依赖解析通过

- [ ] **Step 2: Commit**

```bash
git add swiftforge/Cargo.toml
git commit -m "chore: add swiftforge-log dependency to swiftforge"
```

---

## Task 7: 更新 main.rs 使用 swiftforge-log

**Files:**
- Modify: `swiftforge/src/main.rs`

- [ ] **Step 1: 重写 main.rs**

Modify: `swiftforge/src/main.rs`

```rust
// swiftforge/src/main.rs

use clap::Parser;
use rust_agent_platform::tui::app_controller::AppController;
use swiftforge_log::{init_log, LogLevel, info};

#[derive(Parser, Debug)]
#[command(name = "ragent")]
#[command(about = "Rust Agent Platform - TUI")]
struct Args {
    #[arg(short, long, help = "Enable debug logging (TRACE level)")]
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // 初始化日志系统
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".fastcode");
    let log_path = log_dir.join("ragent.log");

    let level = if args.debug {
        LogLevel::TRACE
    } else {
        LogLevel::INFO
    };

    init_log(log_path, level)?;

    info!("[main]", "Application started (debug={})", args.debug);

    let mut app = AppController::new()?;
    app.run()?;

    Ok(())
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo build -p swiftforge`

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add swiftforge/src/main.rs
git commit -m "refactor(main): replace tracing with swiftforge-log"
```

---

## Task 8: 重构 AppController 移除 debug 参数

**Files:**
- Modify: `swiftforge/src/tui/app_controller.rs`
- Modify: `swiftforge/src/tui/state/app_context.rs`

- [ ] **Step 1: 查看当前 app_controller.rs**

读取文件了解当前 `debug` 参数使用情况

- [ ] **Step 2: 修改 app_controller.rs 签名**

```rust
// Before
pub fn new(show_debug: bool) -> Result<Self>

// After
pub fn new() -> Result<Self>
```

- [ ] **Step 3: 修改 app_context.rs 移除 debug 相关字段**

```rust
// 移除
pub debug_log_path: Option<PathBuf>,
pub debug_messages: Arc<Mutex<Vec<String>>>,
pub debug_tx: Arc<Mutex<Option<mpsc::Sender<String>>>>,

// 修改 AppContext::new 签名
pub fn new(agent: Arc<Agent>, config: Config, tool_registry: Arc<ToolRegistry>) -> Self
```

- [ ] **Step 4: 更新 main.rs 调用**

```rust
// Before
let mut app = AppController::new(args.debug)?;

// After
let mut app = AppController::new()?;
```

- [ ] **Step 5: 验证编译**

Run: `cargo build -p swiftforge`

Expected: PASS（可能有很多后续步骤的错误，逐步修复）

- [ ] **Step 6: Commit**

```bash
git add swiftforge/src/tui/app_controller.rs swiftforge/src/tui/state/app_context.rs swiftforge/src/main.rs
git commit -m "refactor(tui): remove debug parameter from AppController"
```

---

## Task 9: 移除 TUI Debug Panel 代码

**Files:**
- Delete: `swiftforge/src/tui/views/debug_view.rs`
- Modify: `swiftforge/src/tui/views/mod.rs`
- Modify: `swiftforge/src/tui/state/action.rs`
- Modify: `swiftforge/src/tui/state/view_state.rs`
- Modify: `swiftforge/src/tui/views/chat_view.rs`

- [ ] **Step 1: 删除 debug_view.rs**

Run: `rm swiftforge/src/tui/views/debug_view.rs`

- [ ] **Step 2: 更新 mod.rs 移除 DebugView**

Modify: `swiftforge/src/tui/views/mod.rs`

```rust
// 删除
mod debug_view;
pub use debug_view::DebugView;

// 保留
pub use chat_view::ChatView;
pub use config_view::ConfigView;
```

- [ ] **Step 3: 更新 action.rs 移除 debug 相关 Action**

Modify: `swiftforge/src/tui/state/action.rs`

```rust
// 移除
ToggleDebug,
ScrollDebugUp,
ScrollDebugDown,
```

- [ ] **Step 4: 更新 view_state.rs 移除 DebugViewState**

Modify: `swiftforge/src/tui/state/view_state.rs`

```rust
// 移除 DebugViewState 和相关代码
```

- [ ] **Step 5: 更新 chat_view.rs 移除 render_debug_panel()**

Modify: `swiftforge/src/tui/views/chat_view.rs`

- 移除 `debug_scrollbar_state` 字段
- 移除 `render_debug_panel()` 方法
- 移除 `show_debug` 检查和 `debug_height` 逻辑

- [ ] **Step 6: 验证编译**

Run: `cargo build -p swiftforge`

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor(tui): remove debug panel"
```

---

## Task 10: 重构 Agent 移除 debug 参数

**Files:**
- Modify: `swiftforge/src/core/agent.rs`

- [ ] **Step 1: 查看 agent.rs 中的 debug 参数**

```rust
pub async fn chat_with_tools(
    &self,
    messages: Vec<Message>,
    debug_log: Option<String>,
    debug_ui: Option<std::sync::mpsc::Sender<String>>,
) -> Result<ModelResponse>

pub async fn chat_with_tools_streaming<F>(
    &self,
    messages: Vec<Message>,
    debug_log: Option<String>,
    debug_ui: Option<std::sync::mpsc::Sender<String>>,
    mut on_chunk: F,
) -> Result<ModelResponse>

pub async fn run_agent_loop(
    &self,
    initial_message: &str,
    max_iterations: usize,
    debug_log: Option<String>,
    debug_ui: Option<std::sync::mpsc::Sender<String>>,
    stream_ui: Option<std::sync::mpsc::Sender<Result<String>>>,
) -> Result<String>
```

- [ ] **Step 2: 修改函数签名移除 debug 参数**

```rust
pub async fn chat_with_tools(
    &self,
    messages: Vec<Message>,
) -> Result<ModelResponse>

pub async fn chat_with_tools_streaming<F>(
    &self,
    messages: Vec<Message>,
    mut on_chunk: F,
) -> Result<ModelResponse>

pub async fn run_agent_loop(
    &self,
    initial_message: &str,
    max_iterations: usize,
    stream_ui: Option<std::sync::mpsc::Sender<Result<String>>>,
) -> Result<String>
```

- [ ] **Step 3: 在 Agent 内部使用 swiftforge_log 替换 debug 日志**

```rust
use swiftforge_log::{info, debug, warn, error};

impl Agent {
    pub async fn run_agent_loop(&self, ...) -> Result<String> {
        info!("[agent:{}]", "Agent loop started", self.config.name);
        debug!("[agent:{}]", "Iteration {} complete", i + 1);
        // ...
    }
}
```

- [ ] **Step 4: 验证编译**

Run: `cargo build -p swiftforge`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add swiftforge/src/core/agent.rs
git commit -m "refactor(agent): remove debug parameters, use swiftforge-log"
```

---

## Task 11: 清理 tracing 依赖

**Files:**
- Modify: `swiftforge/Cargo.toml`
- Modify: `swiftforge/src/lib.rs`（如需要）

- [ ] **Step 1: 从 Cargo.toml 移除 tracing 相关依赖**

```toml
# 删除
tracing.workspace = true
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
```

- [ ] **Step 2: 在 lib.rs 中移除 tracing 模块**

```rust
// swiftforge/src/lib.rs
pub mod core;
pub mod tui;
pub mod platform;
// 移除 tracing 相关
```

- [ ] **Step 3: 验证编译**

Run: `cargo build -p swiftforge`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add swiftforge/Cargo.toml swiftforge/src/lib.rs
git commit -m "chore: remove tracing dependencies, use swiftforge-log only"
```

---

## Task 12: 创建 README.md

**Files:**
- Create: `libs/swiftforge-log/README.md`

- [ ] **Step 1: 创建 README.md**

```markdown
# swiftforge-log

A lightweight file-based logging library for Rust applications.

## Features

- File-only logging (no stdout)
- Multiple log levels: TRACE, DEBUG, INFO, WARN, ERROR
- Thread-safe global writer
- Macro-based API: `info!()`, `debug!()`, etc.
- Static linking support

## Usage

```rust
use swiftforge_log::{init_log, LogLevel, info};

fn main() -> std::io::Result<()> {
    init_log("/tmp/app.log", LogLevel::DEBUG)?;

    info!("[main]", "Application started");

    Ok(())
}
```

## Log Format

```
[HH:MM:SS.mmm] [LEVEL] [MODULE] message
```

Example:
```
[14:28:49.111] [INFO] [main] Application started
[14:28:50.234] [DEBUG] [agent] Iteration 1 complete
```

## License

MIT OR Apache-2.0
```

- [ ] **Step 2: Commit**

```bash
git add libs/swiftforge-log/README.md
git commit -m "docs(log): add README for swiftforge-log"
```

---

## Task 13: 最终验证

- [ ] **Step 1: 运行完整构建**

Run: `cargo build --workspace`

Expected: PASS

- [ ] **Step 2: 运行测试**

Run: `cargo test --workspace`

Expected: PASS

- [ ] **Step 3: 验证日志输出**

Run:
```bash
# 清理旧日志
rm -rf ~/.fastcode/ragent.log

# 运行应用
cargo run --bin ragent

# 检查日志文件
cat ~/.fastcode/ragent.log
```

Expected: 日志文件存在，包含启动日志

- [ ] **Step 4: 验证 debug 模式**

Run:
```bash
cargo run --bin ragent -- --debug
cat ~/.fastcode/ragent.log
```

Expected: TRACE 级别日志出现

---

## 执行选项

**Plan complete and saved to `docs/superpowers/plans/2026-05-31-swiftforge-log-implementation.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**