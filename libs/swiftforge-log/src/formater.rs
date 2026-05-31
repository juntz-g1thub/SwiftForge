use crate::level::LogLevel;
use chrono::Local;

pub struct Formatter {
    _private: (),
}

impl Formatter {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn format(&self, level: LogLevel, module: Option<&str>, msg: &str) -> String {
        let timestamp = Local::now().format("%H:%M:%S%.3f");
        let module_str = module
            .map(|m| format!(" [{}] ", m))
            .unwrap_or_else(|| " ".to_string());

        format!("[{}] [{}]{}{}", timestamp, level.as_str(), module_str, msg)
    }

    pub fn format_multiline(
        &self,
        level: LogLevel,
        module: Option<&str>,
        msg: &str,
    ) -> Vec<String> {
        let prefix = format!(
            "[{}] [{}]{}",
            Local::now().format("%H:%M:%S%.3f"),
            level.as_str(),
            module
                .map(|m| format!(" [{}] ", m))
                .unwrap_or_else(|| " ".to_string())
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
