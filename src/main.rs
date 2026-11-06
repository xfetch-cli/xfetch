mod addons;
mod atomic_fs;
mod cache;
mod cli;
mod config;
mod effects;
mod error;
mod extensions;
mod hash;
mod info;
mod install_support;
mod logos;
mod plugins;
mod runtime;
mod subprocess;
mod themes;
mod ui;
#[cfg(feature = "update")]
mod update;
mod wasm;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    error::exit_on_error(cli::run(cli));
}
