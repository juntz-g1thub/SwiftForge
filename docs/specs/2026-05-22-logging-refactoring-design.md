# 日志静态库设计文档

> 文档版本: 2.0
> 生成日期: 2026-05-31
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
| 无统一接口 | 缺少独立的日志模块作为日志系统的入口 |

### 1.2 目标

- **独立静态库**：`swiftforge-log` 作为独立的 Rust staticlib crate
- **仅文件输出**：不打印到控制台，减少 I/O 开销
- **多级别支持**：TRACE、DEBUG、INFO、WARN、ERROR
- **全局宏简化**：通过 `log::info!()` 等宏简化调用
- **单一日志文件**：所有模块日志写入同一文件，便于追踪
- **可复用**：其他 Rust 项目可独立依赖此库

---

## 二、库结构

### 2.1 目录结构

```
libs/
└── swiftforge-log/                 # 独立静态库
    ├── Cargo.toml                  # 库配置
    ├── src/
    │   ├── lib.rs                  # 库入口、模块导出
    │   ├── level.rs                # LogLevel 枚举
    │   ├── writer.rs               # FileWriter 实现
    │   ├── formater.rs             # 日志格式化
    │   └── macro.rs                # 公共宏定义
    └── README.md                   # 库文档
```

### 2.2 Cargo.toml 配置

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
```

### 2.3 workspace 集成

```toml
# Cargo.toml (workspace)
[workspace]
members = ["rust-agent-platform", "libs/swiftforge-log"]
resolver = "2"

[workspace.dependencies]
# workspace 级依赖

# rust-agent-platform/Cargo.toml
[dependencies]
swiftforge-log = { path = "../libs/swiftforge-log" }
```

---

## 三、核心类型设计

### 3.1 LogLevel 枚举

```rust
// src/level.rs

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

### 3.2 FileWriter

```rust
// src/writer.rs

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::level::LogLevel;

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

### 3.3 Formatter

```rust
// src/formater.rs

use chrono::Local;
use super::level::LogLevel;

/// 日志格式化器
pub struct Formatter {
    include_module: bool,
}

impl Formatter {
    pub fn new() -> Self {
        Self {
            include_module: true,
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
        let prefix = format!("[{}] [{}]{}",
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

---

## 四、库入口设计

### 4.1 lib.rs

```rust
// src/lib.rs

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
///
/// # Arguments
/// * `path` - 日志文件路径
/// * `level` - 日志级别阈值
///
/// # Example
/// ```rust
/// use swiftforge_log::{init_log, LogLevel};
///
/// fn main() {
///     init_log("/tmp/app.log", LogLevel::DEBUG).unwrap();
///     // 之后可以使用 log!(), info!() 等宏
/// }
/// ```
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

### 4.2 宏定义

```rust
// src/macro_def.rs

#[macro_export]
macro_rules! log {
    ($level:expr, $module:expr, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        $crate::global_writer().write($level, &format!("[{}] {}", $module, msg));
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

---

## 五、日志格式

### 5.1 单行格式

```
[HH:MM:SS.mmm] [LEVEL] [MODULE] message
```

示例：
```
[14:28:49.111] [INFO]  [agent]   Agent started with model: deepseek-chat
[14:28:49.234] [DEBUG] [provider] chat_with_tools_streaming called
[14:28:50.567] [ERROR] [tool]    Tool 'bash' execution failed: timeout
```

### 5.2 多行消息

对于多行消息（如 JSON 请求体），每行前面加上相同的标签：

```
[14:28:49.111] [DEBUG] [provider] REQUEST:
[14:28:49.111] [DEBUG] [provider] {
[14:28:49.111] [DEBUG] [provider]   "model": "deepseek-chat",
[14:28:49.111] [DEBUG] [provider]   "messages": [...]
[14:28:49.111] [DEBUG] [provider] }
```

---

## 六、使用示例

### 6.1 库依赖

```toml
# rust-agent-platform/Cargo.toml
[dependencies]
swiftforge-log = { path = "../libs/swiftforge-log" }
```

### 6.2 应用初始化

```rust
// main.rs
use swiftforge_log::{init_log, LogLevel, info};

fn main() -> anyhow::Result<()> {
    // 初始化日志系统
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".fastcode");
    let log_path = log_dir.join("ragent.log");

    init_log(log_path, LogLevel::DEBUG)?;

    info!("[main]", "Application started");
    // ...
}
```

### 6.3 模块级日志

```rust
// deepseek.rs
use swiftforge_log::{debug, info, warn, error};

pub struct DeepSeekProvider { /* ... */ }

impl DeepSeekProvider {
    pub async fn stream_chat_with_tools(&self, ...) -> Result<()> {
        debug!("[provider:deepseek]", "stream_chat_with_tools called with {} tools", tools.len());

        // ... 业务逻辑 ...

        info!("[provider:deepseek]", "Stream completed, {} tool_calls", tool_count);
        Ok(())
    }
}
```

### 6.4 结构化日志

```rust
// 支持格式化参数
debug!("[agent]", "Iteration {} complete, {} tools called", i + 1, tool_count);
info!("[provider:deepseek]", "Request sent: model={}, stream={}", model, stream);
error!("[mcp]", "Tool '{}' failed: {:?}", tool_name, error);
```

---

## 七、依赖关系

### 7.1 库自身依赖

```
swiftforge-log
└── chrono (时间格式化)
```

### 7.2 依赖它的项目

```
rust-agent-platform
├── swiftforge-log (日志)
├── ratatui (TUI)
├── tokio (异步)
├── reqwest (HTTP)
└── ... (其他依赖)
```

---

## 八、TUI Debug Panel 移除计划

### 8.1 移除内容映射

| 当前代码 | 移除方式 | 替代方案 |
|---------|---------|---------|
| `tui/app_context.rs debug_log_path` | 删除 | 日志文件固定路径 `~/.fastcode/ragent.log` |
| `tui/app_context.rs debug_messages` | 删除 | 使用 swiftforge-log |
| `tui/app_context.rs debug_tx` | 删除 | 使用 swiftforge-log |
| `tui/app_controller.rs show_debug` | 删除 | 日志级别控制 |
| `tui/app_controller.rs debug_rx` | 删除 | 使用 swiftforge-log |
| `tui/views/chat_view.rs render_debug_panel()` | 删除 | 使用 swiftforge-log |
| `tui/views/debug_view.rs` | 删除整个文件 | 使用 swiftforge-log |
| `tui/state/action.rs ToggleDebug` | 删除 | 使用 swiftforge-log |
| `tui/state/action.rs ScrollDebugUp/Down` | 删除 | 不再需要 |
| `tui/state/view_state.rs DebugViewState` | 删除 | 使用 swiftforge-log |

**注意**: `main.rs --debug` 参数**保留**，用于启用 TRACE 级别日志（而非控制 Debug Panel）。

### 8.2 移除步骤

1. **Phase 1**: 创建 `swiftforge-log` 库 ✅ 已完成
2. **Phase 2**: 替换现有日志调用 ✅ 已完成
3. **Phase 3**: 移除 debug panel 代码 ✅ 已完成

---

## 九、重构任务清单

| 任务 | 描述 | 优先级 | 状态 |
|------|------|--------|------|
| 1 | 创建 `libs/swiftforge-log/` 目录结构 | 高 | ✅ 完成 |
| 2 | 实现 `LogLevel` 枚举 | 高 | ✅ 完成 |
| 3 | 实现 `FileWriter` 单例 | 高 | ✅ 完成 |
| 4 | 实现 `Formatter` 格式化器 | 高 | ✅ 完成 |
| 5 | 实现宏定义 | 高 | ✅ 完成 |
| 6 | 添加到 workspace members | 高 | ✅ 完成 |
| 7 | 在 `swiftforge` 中依赖 | 高 | ✅ 完成 |
| 8 | 更新 `main.rs` 初始化 | 高 | ✅ 完成 |
| 9 | 重构 deepseek.rs | 中 | ✅ 完成 |
| 10 | 重构 agent.rs | 中 | ✅ 完成 |
| 11 | 重构 app_controller.rs | 中 | ✅ 完成 |
| 12 | 移除 TUI Debug Panel | 中 | ✅ 完成 |
| 13 | 保留 `--debug` 参数（TRACE 日志开关）| 中 | ✅ 完成 |

---

## 十、验证清单

- [x] `cargo build --workspace` 编译通过
- [x] 日志文件正常生成在 `~/.fastcode/ragent.log`
- [x] 各模块日志正确写入
- [x] 日志级别过滤生效
- [x] TUI Debug Panel 完全移除
- [x] `--debug` 参数保留（TRACE 级别日志开关）

---

## 十一、扩展设计（可选）

### 11.1 日志轮转

后续可扩展支持日志轮转：

```rust
pub struct FileWriter {
    file: Arc<Mutex<File>>,
    level: LogLevel,
    path: PathBuf,
    max_size: u64,        // 单文件最大字节数
    max_files: usize,     // 保留文件数量
}
```

### 11.2 异步写入

对于高吞吐场景，可考虑异步写入：

```rust
// 使用 channel + worker 线程
static WRITER_CHANNEL: OnceLock<Sender<LogEntry>> = OnceLock::new();

pub fn init_async_log(path: PathBuf, level: LogLevel, buffer_size: usize) {
    let (tx, rx) = channel(buffer_size);
    // 启动 worker 线程处理写入
    // ...
}
```

---

*文档状态: 初稿 - 待审批*