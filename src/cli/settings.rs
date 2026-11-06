use crate::cli::args::{Cli, ColorChoice};
use crate::runtime::{self, RuntimeFlags};
use std::io::IsTerminal;

pub fn init(cli: &Cli) {
    let no_color = cli.no_color
        || cli.color == ColorChoice::Never
        || (cli.color == ColorChoice::Auto && !std::io::stdout().is_terminal());

    let log_level = match cli.log_level {
        Some(level) => match level {
            crate::cli::args::LogLevel::Off => runtime::LogLevel::Off,
            crate::cli::args::LogLevel::Error => runtime::LogLevel::Error,
            crate::cli::args::LogLevel::Warn => runtime::LogLevel::Warn,
            crate::cli::args::LogLevel::Info => runtime::LogLevel::Info,
            crate::cli::args::LogLevel::Debug => runtime::LogLevel::Debug,
            crate::cli::args::LogLevel::Trace => runtime::LogLevel::Trace,
        },
        None if cli.quiet => runtime::LogLevel::Warn,
        None if cli.verbose => runtime::LogLevel::Debug,
        None => runtime::LogLevel::Info,
    };

    runtime::init(RuntimeFlags {
        log_level,
        log_json: cli.log_json,
        offline: cli.offline,
        no_color,
    });
}
