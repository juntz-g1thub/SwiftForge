# swiftforge-log

A lightweight file-based logging library for Rust applications.

## Documentation

| Document | Description |
|----------|-------------|
| `docs/2026-05-22-logging-refactoring-design.md` | Logging library design document |

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

## Log Levels

| Level | Value | Description |
|-------|-------|-------------|
| TRACE | 0 | Most verbose - detailed trace |
| DEBUG | 1 | Debug information |
| INFO | 2 | General information |
| WARN | 3 | Warning messages |
| ERROR | 4 | Error messages |

## Macros

- `trace!(module, format, ...)` - Log at TRACE level
- `debug!(module, format, ...)` - Log at DEBUG level
- `info!(module, format, ...)` - Log at INFO level
- `warn!(module, format, ...)` - Log at WARN level
- `error!(module, format, ...)` - Log at ERROR level

## Initialization

Call `init_log()` once at application start:

```rust
use swiftforge_log::{init_log, LogLevel};

fn main() -> std::io::Result<()> {
    init_log("/path/to/app.log", LogLevel::DEBUG)?;
    // ... rest of application
    Ok(())
}
```

## License

MIT OR Apache-2.0