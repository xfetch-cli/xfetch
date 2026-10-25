#![cfg_attr(not(feature = "wasm"), allow(dead_code))]
//! WebAssembly artifact manifests.
//!
//! A manifest declares the artifact identity, the guest shape, the capability
//! allowlists and the runtime limits. It is resolved in this order:
//!
//! 1. Sidecar JSON next to the artifact: `<stem>.json`
//!    (e.g. `xfetch-plugin-weather.wasm` -> `xfetch-plugin-weather.json`).
//! 2. Custom section embedded in the binary under `xfetch:manifest`.
//! 3. An empty default: no capabilities, conservative limits. Pure
//!    stdin/stdout guests keep working with no manifest at all.
//!
//! Capabilities are deny-by-default. A malformed sidecar is a hard error so
//! authors notice mistakes instead of silently running without permissions.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Current manifest schema version.
pub const MANIFEST_VERSION: u32 = 1;
/// Name of the custom section that may embed the manifest.
pub const MANIFEST_SECTION: &str = "xfetch:manifest";

/// Where a manifest was loaded from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestSource {
    /// Sidecar JSON file.
    Sidecar(PathBuf),
    /// Custom section embedded in the wasm binary.
    Embedded,
    /// No manifest present; restrictive defaults apply.
    Default,
}

impl ManifestSource {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Sidecar(_) => "sidecar",
            Self::Embedded => "embedded",
            Self::Default => "default",
        }
    }
}

/// A loaded manifest and its provenance.
#[derive(Debug, Clone)]
pub struct LoadedManifest {
    pub manifest: Manifest,
    pub source: ManifestSource,
}

/// Capability and limit declaration for one wasm guest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Manifest {
    pub manifest_version: u32,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    /// Informational contract: `info_provider`, `logo_animation`, `effect` or
    /// `config_provider`.
    pub kind: Option<String>,
    /// Optional runtime hint (`core` or `component`); detection always reads
    /// the binary header, this only documents intent.
    pub runtime: Option<String>,
    /// Component export called by the host. Defaults to `run`.
    pub entry: String,
    pub capabilities: Capabilities,
    pub limits: Limits,
    /// Relative path to a prebuilt artifact inside the source directory.
    pub artifact: Option<String>,
    /// Command that builds the artifact when it is not checked in. Executed
    /// from the plugin directory by `xfetch <kind> install`.
    pub build: Option<String>,
    /// Absolute URL of a prebuilt artifact (for example a GitHub release
    /// download) used by remote installs that cannot build locally.
    pub artifact_url: Option<String>,
    /// Expected lowercase-hex SHA-256 of the downloaded artifact. When present,
    /// the installer verifies the bytes before writing them; absent keeps the
    /// legacy unverified behavior (with a warning).
    #[serde(default)]
    pub sha256: Option<String>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            manifest_version: MANIFEST_VERSION,
            name: None,
            version: None,
            description: None,
            kind: None,
            runtime: None,
            entry: "run".to_string(),
            capabilities: Capabilities::default(),
            limits: Limits::default(),
            artifact: None,
            build: None,
            artifact_url: None,
            sha256: None,
        }
    }
}

/// All capability allowlists. Empty means "nothing allowed".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Capabilities {
    pub http: HttpCapability,
    pub exec: ExecCapability,
    /// Filesystem preopens. Strings mount read-only at the expanded host
    /// path; object entries control the guest path and mode.
    pub fs: Vec<FsEntry>,
    /// Environment variable names forwarded into the guest. `["*"]` forwards
    /// everything (discouraged).
    pub env: Vec<String>,
    /// Whether `wasi:cli` argument access is granted. The host always passes
    /// the guest name as `argv[0]`.
    pub args: bool,
}

/// HTTP allowlist; patterns glob-match the full URL.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpCapability {
    pub allow: Vec<String>,
}

/// Process execution allowlist and environment forwarding for `exec`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecCapability {
    pub allow: Vec<String>,
    /// Environment names the guest may forward to the child process, on top
    /// of the parent environment inherited by `Command` defaults.
    pub env: Vec<String>,
}

/// One filesystem preopen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FsEntry {
    /// `"~/.config/xfetch"`: read-only mount at the expanded host path.
    Path(String),
    /// Explicit mount with a guest path and mode.
    Mount {
        host: String,
        guest: String,
        #[serde(default)]
        mode: FsMode,
    },
}

/// Access mode for a filesystem mount.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FsMode {
    /// Read-only.
    #[default]
    Ro,
    /// Read-write.
    Rw,
}

/// Runtime limits. Every field has a safe default even without a manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Limits {
    /// Wall-clock deadline for the whole guest invocation, in milliseconds.
    pub timeout_ms: u64,
    /// Linear memory cap per memory, in mebibytes.
    pub memory_mb: u64,
    /// Cap for the JSON response written to stdout, in kibibytes.
    pub output_kb: u64,
    /// Cap for a single host-call response, in kibibytes.
    pub host_call_kb: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            memory_mb: 256,
            output_kb: 4 * 1024,
            host_call_kb: 4 * 1024,
        }
    }
}

impl Limits {
    /// Output cap in bytes.
    pub fn output_bytes(&self) -> usize {
        (self.output_kb as usize).saturating_mul(1024)
    }

    /// Host-call response cap in bytes.
    pub fn host_call_bytes(&self) -> usize {
        (self.host_call_kb as usize).saturating_mul(1024)
    }

    /// Linear memory cap in bytes.
    pub fn memory_bytes(&self) -> usize {
        (self.memory_mb as usize).saturating_mul(1024 * 1024)
    }
}

/// Loads the manifest for `artifact`, trying the sidecar then the embedded
/// custom section.
///
/// A malformed sidecar is reported as an error; a missing manifest yields the
/// restrictive default.
pub fn load(artifact: &Path) -> Result<LoadedManifest, String> {
    if let Some(sidecar) = sidecar_path(artifact)
        && sidecar.is_file()
    {
        let bytes = fs::read(&sidecar)
            .map_err(|err| format!("Failed to read manifest '{}': {}", sidecar.display(), err))?;
        let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|err| {
            format!(
                "Invalid manifest '{}': {} (see docs/WASM.md)",
                sidecar.display(),
                err
            )
        })?;
        return Ok(LoadedManifest {
            manifest: normalize(manifest),
            source: ManifestSource::Sidecar(sidecar),
        });
    }

    if let Ok(bytes) = fs::read(artifact)
        && let Some(section) = read_custom_section(&bytes, MANIFEST_SECTION)
        && let Ok(manifest) = serde_json::from_slice::<Manifest>(section)
    {
        return Ok(LoadedManifest {
            manifest: normalize(manifest),
            source: ManifestSource::Embedded,
        });
    }

    Ok(LoadedManifest {
        manifest: Manifest::default(),
        source: ManifestSource::Default,
    })
}

/// The sidecar path for an artifact: `name.wasm` -> `name.json`.
pub fn sidecar_path(artifact: &Path) -> Option<PathBuf> {
    let stem = artifact.file_stem()?;
    Some(artifact.with_file_name(format!("{}.json", stem.to_string_lossy())))
}

/// Human-readable guest name: manifest `name` or the artifact stem.
pub fn display_name(artifact: &Path) -> String {
    artifact
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| artifact.display().to_string())
}

/// Fills schema gaps: unknown/missing manifest versions are treated as the
/// current schema, and zeroed limits fall back to safe defaults.
fn normalize(mut manifest: Manifest) -> Manifest {
    if manifest.manifest_version == 0 {
        manifest.manifest_version = MANIFEST_VERSION;
    }
    if manifest.limits.timeout_ms == 0 {
        manifest.limits.timeout_ms = Limits::default().timeout_ms;
    }
    if manifest.limits.memory_mb == 0 {
        manifest.limits.memory_mb = Limits::default().memory_mb;
    }
    if manifest.limits.output_kb == 0 {
        manifest.limits.output_kb = Limits::default().output_kb;
    }
    if manifest.limits.host_call_kb == 0 {
        manifest.limits.host_call_kb = Limits::default().host_call_kb;
    }
    if manifest.entry.trim().is_empty() {
        manifest.entry = "run".to_string();
    }
    manifest
}

/// Reads a custom section payload from a core module or component binary.
///
/// The section framing is the same for both shapes: section id (0 = custom),
/// LEB128 size, then for custom sections a LEB128-length name. The parser is
/// defensive: any truncation or malformed length returns `None`.
pub fn read_custom_section<'a>(bytes: &'a [u8], name: &str) -> Option<&'a [u8]> {
    if bytes.len() < 8 || bytes[..4] != *b"\0asm" {
        return None;
    }

    let mut pos = 8usize;
    while pos < bytes.len() {
        let id = *bytes.get(pos)?;
        pos += 1;
        let size = read_var_u32(bytes, &mut pos)? as usize;
        let end = pos.checked_add(size)?;
        if end > bytes.len() {
            return None;
        }
        if id == 0 {
            let mut name_pos = pos;
            let name_len = read_var_u32(bytes, &mut name_pos)? as usize;
            let name_end = name_pos.checked_add(name_len)?;
            if name_end <= end && bytes.get(name_pos..name_end) == Some(name.as_bytes()) {
                return Some(&bytes[name_end..end]);
            }
        }
        pos = end;
    }

    None
}

/// Decodes an unsigned LEB128 value, advancing `pos`.
fn read_var_u32(bytes: &[u8], pos: &mut usize) -> Option<u32> {
    let mut result = 0u32;
    let mut shift = 0u32;
    loop {
        let byte = *bytes.get(*pos)?;
        *pos += 1;
        result |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift >= 35 {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_defaults_are_conservative() {
        let manifest = Manifest::default();
        assert_eq!(manifest.entry, "run");
        assert!(manifest.capabilities.http.allow.is_empty());
        assert!(manifest.capabilities.exec.allow.is_empty());
        assert!(manifest.capabilities.fs.is_empty());
        assert!(!manifest.capabilities.args);
        assert_eq!(manifest.limits.timeout_ms, 30_000);
    }

    #[test]
    fn parses_full_manifest() {
        let json = r#"{
            "manifest_version": 1,
            "name": "weather-wasm",
            "kind": "info_provider",
            "runtime": "core",
            "capabilities": {
                "http": { "allow": ["https://wttr.in/*"] },
                "exec": { "allow": ["curl"], "env": ["PATH"] },
                "fs": [
                    "~/.config/xfetch",
                    { "host": "~/.cache/xfetch", "guest": "/data", "mode": "rw" }
                ],
                "env": ["HOME"]
            },
            "limits": { "timeout_ms": 5000, "memory_mb": 64 }
        }"#;

        let manifest: Manifest = serde_json::from_str(json).expect("parse manifest");

        assert_eq!(manifest.name.as_deref(), Some("weather-wasm"));
        assert_eq!(manifest.capabilities.http.allow.len(), 1);
        assert_eq!(manifest.capabilities.exec.allow, vec!["curl"]);
        match &manifest.capabilities.fs[1] {
            FsEntry::Mount { guest, mode, .. } => {
                assert_eq!(guest, "/data");
                assert_eq!(*mode, FsMode::Rw);
            }
            other => panic!("expected mount, got {:?}", other),
        }
        assert_eq!(manifest.limits.timeout_ms, 5000);
        assert_eq!(manifest.limits.memory_bytes(), 64 * 1024 * 1024);
    }

    #[test]
    fn parses_sha256_and_defaults_to_none() {
        let with_hash = r#"{
            "sha256": "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
        }"#;
        let manifest: Manifest = serde_json::from_str(with_hash).expect("parse sha256");
        assert_eq!(
            manifest.sha256.as_deref(),
            Some("BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD")
        );

        let without_hash: Manifest =
            serde_json::from_str(r#"{ "name": "legacy" }"#).expect("parse");
        assert!(without_hash.sha256.is_none());
        assert!(Manifest::default().sha256.is_none());
    }

    #[test]
    fn sha256_roundtrips_through_serialization() {
        let manifest = Manifest {
            sha256: Some("a".repeat(64)),
            ..Manifest::default()
        };
        let json = serde_json::to_string(&manifest).expect("serialize");
        let parsed: Manifest = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed.sha256, manifest.sha256);
    }

    #[test]
    fn sidecar_path_replaces_wasm_extension() {
        let path = Path::new("/plugins/xfetch-plugin-weather.wasm");
        assert_eq!(
            sidecar_path(path),
            Some(PathBuf::from("/plugins/xfetch-plugin-weather.json"))
        );
    }

    #[test]
    fn reads_embedded_custom_section() {
        let payload = br#"{"name":"embedded"}"#;
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        wasm.push(0);
        let mut section = Vec::new();
        section.push(MANIFEST_SECTION.len() as u8);
        section.extend_from_slice(MANIFEST_SECTION.as_bytes());
        section.extend_from_slice(payload);
        wasm.push(section.len() as u8);
        wasm.extend_from_slice(&section);

        let found = read_custom_section(&wasm, MANIFEST_SECTION).expect("section");
        assert_eq!(found, payload);
        assert!(read_custom_section(&wasm, "other").is_none());
    }

    #[test]
    fn malformed_section_is_ignored() {
        let wasm = b"\0asm\x01\0\0\0\x00\xff".to_vec();
        assert!(read_custom_section(&wasm, MANIFEST_SECTION).is_none());
    }

    #[test]
    fn normalize_restores_zeroed_limits() {
        let json = r#"{ "limits": { "timeout_ms": 0, "memory_mb": 0 } }"#;
        let manifest: Manifest = serde_json::from_str(json).expect("parse");
        let manifest = normalize(manifest);
        assert_eq!(manifest.limits.timeout_ms, 30_000);
        assert_eq!(manifest.limits.memory_mb, 256);
    }
}
