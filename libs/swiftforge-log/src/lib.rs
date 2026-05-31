mod formater;
mod level;
mod macro_def;
mod writer;

pub use formater::Formatter;
pub use level::LogLevel;
pub use writer::FileWriter;

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
pub fn global_writer() -> &'static OnceLock<Arc<FileWriter>> {
    &GLOBAL_WRITER
}

/// 获取全局 Formatter
pub fn global_formatter() -> &'static Formatter {
    GLOBAL_FORMATTER
        .get()
        .expect("Log not initialized. Call init_log() first.")
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
