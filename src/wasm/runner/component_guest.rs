//! Component-model runner for typed guests.
//!
//! Components implement one of the worlds in `wit/xfetch-runtime.wit`
//! (`plugin`, `effect`, `extension` or `command`). The world's `run` export
//! receives the JSON request string and returns the response string (JSON for
//! plugins/effects/extensions, plain text for commands); host
//! capabilities arrive as typed WIT imports instead of the JSON bridge used by
//! core modules. WASI preview 2 is linked as well, so componentized Python,
//! JavaScript and `wit-bindgen` guests get clocks, random and preopened
//! filesystems for free.

use super::{GuestKind, map_execution_error};
use crate::wasm::engine;
use crate::wasm::host::{self, HostCallError, HostContext, HostErrorKind};
use crate::wasm::manifest::{FsMode, Manifest};
use crate::wasm::policy::FsMount;
use serde_json::{Value, json};
use std::time::Instant;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::{FsPerms, ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// Cap for guest stderr forwarded to the terminal.
const STDERR_CAP: usize = 256 * 1024;

/// Generated bindings, one module per world. Each world imports the same host
/// interface but generates nominally distinct types, so the host side uses a
/// macro to implement all of them.
pub mod bindings {
    pub mod plugin {
        wasmtime::component::bindgen!({ path: "wit/xfetch-runtime.wit", world: "plugin" });
    }
    pub mod effect {
        wasmtime::component::bindgen!({ path: "wit/xfetch-runtime.wit", world: "effect" });
    }
    pub mod extension {
        wasmtime::component::bindgen!({ path: "wit/xfetch-runtime.wit", world: "extension" });
    }
    pub mod command {
        wasmtime::component::bindgen!({ path: "wit/xfetch-runtime.wit", world: "command" });
    }
}

/// Store data for component guests.
struct ComponentState {
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    host: HostContext,
}

impl WasiView for ComponentState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

/// Implements the generated `host` interface for one world.
///
/// `$convert` names the per-world error converter to avoid collisions between
/// the three expanded impl blocks.
macro_rules! impl_host_interface {
    ($world:ident, $convert:ident) => {
        impl bindings::$world::xfetch::runtime::host::Host for ComponentState {
            fn fetch(
                &mut self,
                req: bindings::$world::xfetch::runtime::types::HttpRequest,
            ) -> Result<
                bindings::$world::xfetch::runtime::types::HttpResponse,
                bindings::$world::xfetch::runtime::types::HostError,
            > {
                let mut headers = Vec::new();
                for (name, value) in &req.headers {
                    headers.push(json!([name, value]));
                }

                let args = json!({
                    "method": req.method,
                    "url": req.url,
                    "headers": headers,
                    "body_base64": req.body.as_ref().map(|body| host::to_base64(body)),
                    "timeout_ms": req.timeout_ms,
                });

                let value =
                    host::dispatch_value("http", &args, &self.host).map_err(Self::$convert)?;

                Ok(bindings::$world::xfetch::runtime::types::HttpResponse {
                    status: value["status"].as_u64().unwrap_or(0) as u16,
                    headers: headers_from_value(&value),
                    body: bytes_from_field(&value, "body_base64"),
                })
            }

            fn exec(
                &mut self,
                req: bindings::$world::xfetch::runtime::types::ExecRequest,
            ) -> Result<
                bindings::$world::xfetch::runtime::types::ExecResult,
                bindings::$world::xfetch::runtime::types::HostError,
            > {
                let mut env = serde_json::Map::new();
                for (name, value) in &req.env {
                    env.insert(name.clone(), Value::String(value.clone()));
                }

                let args = json!({
                    "program": req.program,
                    "args": req.args,
                    "stdin_base64": req.stdin.as_ref().map(|stdin| host::to_base64(stdin)),
                    "env": env,
                    "timeout_ms": req.timeout_ms,
                });

                let value =
                    host::dispatch_value("exec", &args, &self.host).map_err(Self::$convert)?;

                Ok(bindings::$world::xfetch::runtime::types::ExecResult {
                    code: value["code"].as_i64().unwrap_or(-1) as i32,
                    stdout: bytes_from_field(&value, "stdout_base64"),
                    stderr: bytes_from_field(&value, "stderr_base64"),
                })
            }

            fn log(&mut self, level: String, message: String) {
                host::log_message(&self.host.name, &level, &message);
            }

            fn protocol_version(&mut self) -> u32 {
                host::PROTOCOL_VERSION
            }
        }

        impl ComponentState {
            /// Maps a host-call failure into the world's generated error type.
            #[allow(dead_code)]
            fn $convert(err: HostCallError) -> bindings::$world::xfetch::runtime::types::HostError {
                use bindings::$world::xfetch::runtime::types::HostError as Generated;
                match err.kind {
                    HostErrorKind::Denied => Generated::Denied(err.message),
                    HostErrorKind::Failed => Generated::Failed(err.message),
                    HostErrorKind::Timeout => Generated::Timeout(err.message),
                    HostErrorKind::TooLarge => Generated::TooLarge(err.message),
                    HostErrorKind::Unsupported => Generated::Unsupported(err.message),
                }
            }
        }
    };
}

impl_host_interface!(plugin, to_host_error_plugin);
impl_host_interface!(effect, to_host_error_effect);
impl_host_interface!(extension, to_host_error_extension);
impl_host_interface!(command, to_host_error_command);

/// Runs a component and returns its JSON response bytes.
pub(super) fn run(
    bytes: &[u8],
    manifest: &Manifest,
    ctx: HostContext,
    request: &[u8],
    kind: GuestKind,
) -> Result<Vec<u8>, String> {
    let engine = engine::engine();
    let component = Component::new(engine, bytes)
        .map_err(|err| format!("Failed to compile component '{}': {}", ctx.name, err))?;

    let stdout = MemoryOutputPipe::new(manifest.limits.output_bytes().max(1));
    let stderr = MemoryOutputPipe::new(STDERR_CAP);
    let wasi = build_wasi(&ctx, request, stdout.clone(), stderr.clone())?;

    let timeout = ctx.deadline.saturating_duration_since(Instant::now());
    let deadline = ctx.deadline;
    let name = ctx.name.clone();

    let limits = StoreLimitsBuilder::new()
        .memory_size(manifest.limits.memory_bytes().max(1))
        .build();
    let mut store = Store::new(
        engine,
        ComponentState {
            wasi,
            table: ResourceTable::new(),
            limits,
            host: ctx,
        },
    );
    store.limiter(|state| &mut state.limits);
    store.set_epoch_deadline(engine::deadline_ticks(timeout));
    store.epoch_deadline_trap();

    let mut linker: Linker<ComponentState> = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
        .map_err(|err| format!("Failed to configure WASI for '{}': {}", name, err))?;

    let response = match kind {
        GuestKind::Plugin => {
            bindings::plugin::xfetch::runtime::host::add_to_linker::<
                ComponentState,
                HasSelf<ComponentState>,
            >(&mut linker, |state| state)
            .map_err(|err| format!("Failed to configure host for '{}': {}", name, err))?;
            let pre = linker
                .instantiate_pre(&component)
                .map_err(|err| format!("Failed to prepare '{}': {}", name, err))?;
            let guest = bindings::plugin::PluginPre::new(pre)
                .map_err(|err| format!("Invalid plugin world for '{}': {}", name, err))?
                .instantiate(&mut store)
                .map_err(|err| format!("Failed to instantiate '{}': {}", name, err))?;
            call_guest(
                &name,
                deadline,
                timeout,
                guest.call_run(&mut store, request_json(request)),
            )
        }
        GuestKind::Effect => {
            bindings::effect::xfetch::runtime::host::add_to_linker::<
                ComponentState,
                HasSelf<ComponentState>,
            >(&mut linker, |state| state)
            .map_err(|err| format!("Failed to configure host for '{}': {}", name, err))?;
            let pre = linker
                .instantiate_pre(&component)
                .map_err(|err| format!("Failed to prepare '{}': {}", name, err))?;
            let guest = bindings::effect::EffectPre::new(pre)
                .map_err(|err| format!("Invalid effect world for '{}': {}", name, err))?
                .instantiate(&mut store)
                .map_err(|err| format!("Failed to instantiate '{}': {}", name, err))?;
            call_guest(
                &name,
                deadline,
                timeout,
                guest.call_run(&mut store, request_json(request)),
            )
        }
        GuestKind::Extension => {
            bindings::extension::xfetch::runtime::host::add_to_linker::<
                ComponentState,
                HasSelf<ComponentState>,
            >(&mut linker, |state| state)
            .map_err(|err| format!("Failed to configure host for '{}': {}", name, err))?;
            let pre = linker
                .instantiate_pre(&component)
                .map_err(|err| format!("Failed to prepare '{}': {}", name, err))?;
            let guest = bindings::extension::ExtensionPre::new(pre)
                .map_err(|err| format!("Invalid extension world for '{}': {}", name, err))?
                .instantiate(&mut store)
                .map_err(|err| format!("Failed to instantiate '{}': {}", name, err))?;
            call_guest(
                &name,
                deadline,
                timeout,
                guest.call_run(&mut store, request_json(request)),
            )
        }
        GuestKind::Command => {
            bindings::command::xfetch::runtime::host::add_to_linker::<
                ComponentState,
                HasSelf<ComponentState>,
            >(&mut linker, |state| state)
            .map_err(|err| format!("Failed to configure host for '{}': {}", name, err))?;
            let pre = linker
                .instantiate_pre(&component)
                .map_err(|err| format!("Failed to prepare '{}': {}", name, err))?;
            let guest = bindings::command::CommandPre::new(pre)
                .map_err(|err| format!("Invalid command world for '{}': {}", name, err))?
                .instantiate(&mut store)
                .map_err(|err| format!("Failed to instantiate '{}': {}", name, err))?;
            call_guest(
                &name,
                deadline,
                timeout,
                guest.call_run(&mut store, request_json(request)),
            )
        }
    };

    forward_stderr(&stderr);
    response
}

/// Normalizes the three generated `call_run` results (same shape, distinct
/// types) into the output contract.
fn call_guest(
    name: &str,
    deadline: Instant,
    timeout: std::time::Duration,
    result: wasmtime::Result<Result<String, String>>,
) -> Result<Vec<u8>, String> {
    match result {
        Ok(Ok(body)) => {
            if body.is_empty() {
                Err(format!("Wasm guest '{}' produced no output", name))
            } else {
                Ok(body.into_bytes())
            }
        }
        Ok(Err(guest_error)) => Err(format!(
            "Wasm guest '{}' returned an error: {}",
            name, guest_error
        )),
        Err(trap) => match super::exit_code(&trap) {
            Some(0) => Err(format!(
                "Wasm guest '{}' exited before returning a response",
                name
            )),
            Some(code) => Err(format!("Wasm guest '{}' exited with code {}", name, code)),
            None => Err(map_execution_error(name, deadline, timeout, trap)),
        },
    }
}

/// Builds the preview2 context shared with the component.
fn build_wasi(
    ctx: &HostContext,
    request: &[u8],
    stdout: MemoryOutputPipe,
    stderr: MemoryOutputPipe,
) -> Result<WasiCtx, String> {
    let mut builder = WasiCtxBuilder::new();
    builder.stdin(MemoryInputPipe::new(request.to_vec()));
    builder.stdout(stdout);
    builder.stderr(stderr);
    // `argv` stays empty unless the manifest grants the `args` capability.
    if ctx.policy.args {
        builder.args(&[ctx.name.as_str()]);
    }

    for (name, value) in std::env::vars().filter(|(name, _)| ctx.policy.env_allowed(name)) {
        builder.env(name, value);
    }

    for mount in ctx.policy.fs_mounts() {
        builder
            .preopened_dir(&mount.host, &mount.guest, mount_perms(mount))
            .map_err(|err| {
                format!(
                    "Failed to mount '{}' for '{}': {}",
                    mount.host.display(),
                    ctx.name,
                    err
                )
            })?;
    }

    Ok(builder.build())
}

/// Maps a manifest mount mode to WASI permissions.
fn mount_perms(mount: &FsMount) -> FsPerms {
    match mount.mode {
        FsMode::Ro => FsPerms::ReadOnly,
        FsMode::Rw => FsPerms::ReadWrite,
    }
}

/// Converts a response `headers` array into WIT tuples.
fn headers_from_value(value: &Value) -> Vec<(String, String)> {
    value["headers"]
        .as_array()
        .map(|pairs| {
            pairs
                .iter()
                .filter_map(|pair| {
                    let pair = pair.as_array()?;
                    Some((
                        pair.first()?.as_str()?.to_string(),
                        pair.get(1)?.as_str()?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Decodes a base64 field from a host response; malformed data yields no bytes
/// (the host only emits valid base64).
fn bytes_from_field(value: &Value, field: &str) -> Vec<u8> {
    use base64::Engine as _;
    value[field]
        .as_str()
        .and_then(|encoded| {
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .ok()
        })
        .unwrap_or_default()
}

/// Borrows the request as UTF-8 for the typed `run` export.
fn request_json(request: &[u8]) -> &str {
    std::str::from_utf8(request).unwrap_or_default()
}

/// Forwards captured guest stderr to the terminal.
fn forward_stderr(stderr: &MemoryOutputPipe) {
    let bytes = stderr.contents();
    if !bytes.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&bytes));
    }
}
