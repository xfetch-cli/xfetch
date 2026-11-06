use crate::cli::args::{Cli, WasmCommands};
use crate::error::XFetchError;

pub fn run(action: WasmCommands, cli: &Cli) -> Result<(), XFetchError> {
    match action {
        WasmCommands::Inspect { path } => {
            crate::wasm::inspect::inspect(std::path::Path::new(&path), cli.json)
                .map_err(XFetchError::Addon)
        }
        WasmCommands::Run {
            path,
            request,
            request_file,
            timeout,
            kind,
        } => {
            let request = match (request, request_file) {
                (Some(_), Some(_)) => {
                    Err("Use either --request or --request_file, not both".to_string())
                }
                (Some(request), None) => Ok(request.into_bytes()),
                (None, Some(file)) => std::fs::read(&file)
                    .map_err(|err| format!("Failed to read '{}': {}", file, err)),
                (None, None) => Ok(b"{}".to_vec()),
            }
            .map_err(XFetchError::Usage)?;

            let kind = match kind.as_str() {
                "effect" => crate::wasm::GuestKind::Effect,
                "extension" => crate::wasm::GuestKind::Extension,
                "command" => crate::wasm::GuestKind::Command,
                _ => crate::wasm::GuestKind::Plugin,
            };
            let timeout = timeout.map(std::time::Duration::from_secs);

            let response =
                crate::wasm::run_request(std::path::Path::new(&path), &request, timeout, kind)
                    .map_err(XFetchError::Addon)?;
            use std::io::Write;
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            let _ = stdout.write_all(&response);
            let _ = stdout.write_all(b"\n");
            Ok(())
        }
        WasmCommands::Wit => {
            crate::wasm::inspect::print_wit();
            Ok(())
        }
    }
}
