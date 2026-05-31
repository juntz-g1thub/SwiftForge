#[macro_export]
macro_rules! log {
    ($level:expr, $module:expr, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        if let Some(w) = $crate::global_writer().get() {
            w.write($level, &format!("[{}] {}", $module, msg));
        }
    }};
}

#[macro_export]
macro_rules! trace {
    ($module:expr, $($arg:tt)*) => {{
        $crate::log!($crate::LogLevel::TRACE, $module, $($arg)*);
    }};
}

#[macro_export]
macro_rules! debug {
    ($module:expr, $($arg:tt)*) => {{
        $crate::log!($crate::LogLevel::DEBUG, $module, $($arg)*);
    }};
}

#[macro_export]
macro_rules! info {
    ($module:expr, $($arg:tt)*) => {{
        $crate::log!($crate::LogLevel::INFO, $module, $($arg)*);
    }};
}

#[macro_export]
macro_rules! warn {
    ($module:expr, $($arg:tt)*) => {{
        $crate::log!($crate::LogLevel::WARN, $module, $($arg)*);
    }};
}

#[macro_export]
macro_rules! error {
    ($module:expr, $($arg:tt)*) => {{
        $crate::log!($crate::LogLevel::ERROR, $module, $($arg)*);
    }};
}
