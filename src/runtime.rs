//! Process-wide runtime flags shared by the CLI, addons and network probes.
//!
//! The flags are set once from the parsed CLI and are read-only afterwards.

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            LogLevel::Off => "off",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }

    pub const fn includes(self, level: LogLevel) -> bool {
        self as u8 >= level as u8
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeFlags {
    pub log_level: LogLevel,
    pub log_json: bool,
    pub offline: bool,
    pub no_color: bool,
}

static FLAGS: OnceLock<RuntimeFlags> = OnceLock::new();

pub fn init(flags: RuntimeFlags) {
    let _ = FLAGS.set(flags);
}

pub fn flags() -> &'static RuntimeFlags {
    FLAGS.get_or_init(RuntimeFlags::default)
}

pub fn log_level() -> LogLevel {
    flags().log_level
}

pub fn log_json() -> bool {
    flags().log_json
}

pub fn offline() -> bool {
    flags().offline
}

pub fn no_color() -> bool {
    flags().no_color
}

pub fn quiet() -> bool {
    !flags().log_level.includes(LogLevel::Info)
}

pub fn verbose() -> bool {
    flags().log_level.includes(LogLevel::Debug)
}

/// Emits a log line at `level`. Errors are always emitted; other levels are
/// filtered by `--log-level`. `--log-json` writes JSON lines to stderr.
pub fn log_at(level: LogLevel, message: impl AsRef<str>) {
    let flags = flags();
    if level == LogLevel::Off {
        return;
    }
    if level != LogLevel::Error && level > flags.log_level {
        return;
    }

    let message = message.as_ref();
    if flags.log_json {
        let line = serde_json::json!({
            "level": level.as_str(),
            "target": "xfetch",
            "message": message,
        });
        eprintln!("{line}");
        return;
    }

    if level == LogLevel::Info {
        println!("{message}");
    } else {
        eprintln!("{message}");
    }
}

pub fn trace(message: impl AsRef<str>) {
    log_at(LogLevel::Trace, message);
}

pub fn debug(message: impl AsRef<str>) {
    log_at(LogLevel::Debug, message);
}

pub fn info(message: impl AsRef<str>) {
    log_at(LogLevel::Info, message);
}

pub fn warn(message: impl AsRef<str>) {
    log_at(LogLevel::Warn, message);
}

pub fn error(message: impl AsRef<str>) {
    log_at(LogLevel::Error, message);
}
