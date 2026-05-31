#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    TRACE = 0,
    DEBUG = 1,
    INFO = 2,
    WARN = 3,
    ERROR = 4,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Some(Self::TRACE),
            "DEBUG" => Some(Self::DEBUG),
            "INFO" => Some(Self::INFO),
            "WARN" | "WARNING" => Some(Self::WARN),
            "ERROR" => Some(Self::ERROR),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TRACE => "TRACE",
            Self::DEBUG => "DEBUG",
            Self::INFO => "INFO",
            Self::WARN => "WARN",
            Self::ERROR => "ERROR",
        }
    }

    pub fn default() -> Self {
        Self::INFO
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::INFO
    }
}
