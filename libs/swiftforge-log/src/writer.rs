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
        let file = OpenOptions::new().create(true).append(true).open(&path)?;

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
