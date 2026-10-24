//! Installation helpers for wasm artifacts.
//!
//! Shared by the plugin, effect and extension installers. A wasm source
//! directory is recognized by any of:
//!
//! - a source manifest (`xfetch-<label>.json`, `<label>.json` or
//!   `<name>.json`) that declares a wasm runtime, `artifact`, `artifact_url`
//!   or `build`;
//! - a prebuilt artifact following the conventional locations
//!   (`artifact` field, `dist/<name>.wasm`, `<name>.wasm`,
//!   `xfetch-<label>-<name>.wasm`, or the cargo wasm targets).
//!
//! Installation copies the artifact as `<prefix><name>.wasm` and the manifest
//! as `<prefix><name>.json` (the sidecar convention the runtime resolves), so
//! installed wasm guests look exactly like native ones to the loaders.
//!
//! Remote sources can ship prebuilt artifacts through `artifact_url`, which
//! lets users install wasm guests without any language toolchain installed.

use crate::hash::{normalize_sha256, verify_sha256};
use crate::wasm::detect::is_wasm_file;
use crate::wasm::manifest::Manifest;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Upper bound for a downloaded artifact (release wasm files are small; this
/// only guards against runaway responses).
const DOWNLOAD_LIMIT: u64 = 64 * 1024 * 1024;

/// What the installer should do with a source directory.
#[derive(Debug)]
pub enum SourceAction {
    /// A wasm artifact is ready to install.
    Install(InstallPlan),
    /// Run a build command, then plan again.
    BuildThenInstall { command: String },
    /// Fetch a prebuilt artifact from a URL.
    Download {
        url: String,
        manifest: Option<Manifest>,
    },
    /// Not a wasm source: the caller falls back to the native cargo flow.
    NotWasm,
}

/// A resolved artifact plus its optional manifest.
#[derive(Debug)]
pub struct InstallPlan {
    pub artifact: PathBuf,
    pub manifest_path: Option<PathBuf>,
    pub manifest: Option<Manifest>,
}

/// Inspects a source directory and decides how to install it.
///
/// `label` is `plugin`, `effect` or `extension`; `name` is the guest name.
pub fn plan(dir: &Path, label: &str, name: &str) -> Result<SourceAction, String> {
    let manifest_path = find_source_manifest(dir, label);
    let manifest = match &manifest_path {
        Some(path) => Some(parse_manifest(path)?),
        None => None,
    };

    if let Some(artifact) = find_artifact(dir, label, name, manifest.as_ref()) {
        return Ok(SourceAction::Install(InstallPlan {
            artifact,
            manifest_path,
            manifest,
        }));
    }

    if let Some(manifest) = &manifest
        && let Some(url) = manifest.artifact_url.clone()
    {
        return Ok(SourceAction::Download {
            url,
            manifest: Some(manifest.clone()),
        });
    }

    if let Some(command) = manifest.as_ref().and_then(|m| m.build.clone()) {
        return Ok(SourceAction::BuildThenInstall { command });
    }

    // No manifest and no artifact: only treat the directory as a wasm source
    // when an artifact actually exists, so plain native crates are untouched.
    if manifest.is_some() && declares_wasm(manifest.as_ref()) {
        return Err(format!(
            "Wasm source '{}' declares a wasm runtime but no artifact was found; \
             add an artifact, an artifact_url or a build command",
            dir.display()
        ));
    }

    Ok(SourceAction::NotWasm)
}

/// Runs the manifest build command inside the source directory.
///
/// Build commands are not sandboxed: they run on the host with the invoking
/// user's privileges, so the exact command is printed before execution.
pub fn run_build(dir: &Path, command: &str) -> Result<(), String> {
    println!("Running wasm build command in '{}':", dir.display());
    println!("  {}", command);
    println!(
        "WARNING: manifest build commands run on the host, unsandboxed, with your user privileges."
    );

    #[cfg(unix)]
    let mut builder = {
        let mut builder = Command::new("sh");
        builder.arg("-c").arg(command);
        builder
    };
    #[cfg(windows)]
    let mut builder = {
        let mut builder = Command::new("cmd");
        builder.arg("/C").arg(command);
        builder
    };

    let status = builder
        .current_dir(dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| format!("Failed to run build command '{}': {}", command, err))?;

    if !status.success() {
        return Err(format!("Build command failed: {}", command));
    }

    Ok(())
}

/// Resolves the digest to enforce: an explicit `--sha256` value wins over the
/// manifest field. Both are normalized before use, so malformed digests fail
/// before anything is downloaded or written.
fn expected_sha256(
    flag: Option<&str>,
    manifest: Option<&Manifest>,
) -> Result<Option<String>, String> {
    let value = match flag {
        Some(value) => Some(value),
        None => manifest.and_then(|manifest| manifest.sha256.as_deref()),
    };
    value.map(normalize_sha256).transpose()
}

/// Copies an installed plan into the destination directory.
///
/// When a digest is supplied by `--sha256` or the plan's manifest, the artifact
/// bytes are verified before the destination directory is created or any file
/// is replaced.
pub fn install(
    mut plan: InstallPlan,
    sha256: Option<&str>,
    dest_dir: &Path,
    prefix: &str,
    name: &str,
    label: &str,
) -> Result<(), String> {
    let expected = expected_sha256(sha256, plan.manifest.as_ref())?;

    // Persist a CLI-supplied pin in the installed sidecar when one is written.
    if let (Some(manifest), Some(hash)) = (plan.manifest.as_mut(), expected.as_ref()) {
        manifest.sha256 = Some(hash.clone());
    }

    // With a digest, read the artifact exactly once and keep those bytes: the
    // verified buffer is what gets written, so swapping the path after the
    // check cannot bypass verification.
    let verified_bytes = match expected.as_deref() {
        Some(expected) => {
            let bytes = fs::read(&plan.artifact).map_err(|err| {
                format!(
                    "Failed to read artifact '{}' for SHA-256 verification: {}",
                    plan.artifact.display(),
                    err
                )
            })?;
            verify_sha256(&bytes, expected).map_err(|err| {
                format!(
                    "Refusing to install wasm {} '{}' from '{}': {}",
                    label,
                    name,
                    plan.artifact.display(),
                    err
                )
            })?;
            if bytes
                .get(..8)
                .and_then(crate::wasm::detect::kind_from_header)
                .is_none()
            {
                return Err(format!(
                    "Artifact '{}' is not a WebAssembly binary",
                    plan.artifact.display()
                ));
            }
            println!("Verified SHA-256 {}", expected);
            Some(bytes)
        }
        None => {
            if !is_wasm_file(&plan.artifact) {
                return Err(format!(
                    "Artifact '{}' is not a WebAssembly binary",
                    plan.artifact.display()
                ));
            }
            None
        }
    };

    fs::create_dir_all(dest_dir)
        .map_err(|err| format!("Failed to create {} directory: {}", label, err))?;

    let wasm_dest = dest_dir.join(format!("{}{}.wasm", prefix, name));
    if let Some(bytes) = &verified_bytes {
        crate::atomic_fs::write_atomic(&wasm_dest, bytes).map_err(|err| {
            format!(
                "Failed to write verified wasm artifact '{}': {}",
                plan.artifact.display(),
                err
            )
        })?;
    } else {
        crate::atomic_fs::copy_atomic(&plan.artifact, &wasm_dest).map_err(|err| {
            format!(
                "Failed to copy wasm artifact '{}': {}",
                plan.artifact.display(),
                err
            )
        })?;
    }

    if let (Some(manifest_path), Some(manifest)) = (&plan.manifest_path, &plan.manifest) {
        let sidecar = dest_dir.join(format!("{}{}.json", prefix, name));
        crate::atomic_fs::write_atomic(
            &sidecar,
            &serde_json::to_vec_pretty(manifest)
                .map_err(|err| format!("Failed to serialize manifest: {}", err))?,
        )
        .map_err(|err| format!("Failed to write manifest '{}': {}", sidecar.display(), err))?;
        println!("Installed manifest '{}'", manifest_path.display());
    }

    println!(
        "Installed wasm {} '{}' to {}",
        label,
        name,
        wasm_dest.display()
    );
    Ok(())
}

/// Downloads a prebuilt artifact from `url` and installs it, optionally with
/// the manifest shipped by the source repository.
///
/// `sha256` is the `--sha256` CLI value and takes precedence over the manifest
/// `sha256` field. When neither supplies a digest the artifact is installed as
/// before, after printing an explicit unverified-integrity warning.
pub fn download_and_install(
    url: &str,
    mut manifest: Option<Manifest>,
    sha256: Option<&str>,
    dest_dir: &Path,
    prefix: &str,
    name: &str,
    label: &str,
) -> Result<(), String> {
    let expected = expected_sha256(sha256, manifest.as_ref())?;

    // Record the enforced digest in the installed sidecar so the provenance
    // survives the install even when the source manifest lacked it.
    if let (Some(manifest), Some(hash)) = (manifest.as_mut(), expected.as_ref()) {
        manifest.sha256 = Some(hash.clone());
    }

    println!("Downloading wasm {} '{}' from {}...", label, name, url);

    let response = ureq::get(url)
        .timeout(Duration::from_secs(120))
        .call()
        .map_err(|err| format!("Failed to download '{}': {}", url, err))?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(DOWNLOAD_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| format!("Failed to read '{}': {}", url, err))?;

    if bytes.len() as u64 > DOWNLOAD_LIMIT {
        return Err(format!(
            "Downloaded artifact exceeds the {} MiB limit",
            DOWNLOAD_LIMIT / (1024 * 1024)
        ));
    }

    if let Some(expected) = expected.as_deref() {
        verify_sha256(&bytes, expected).map_err(|err| {
            format!(
                "Refusing to install wasm {} '{}' from '{}': {}",
                label, name, url, err
            )
        })?;
        println!("Verified SHA-256 {}", expected);
    } else {
        eprintln!(
            "WARNING: no SHA-256 checksum supplied for '{}'; the downloaded wasm {} is UNVERIFIED.",
            url, label
        );
        eprintln!(
            "         Pass --sha256 <hex> or add a \"sha256\" field to the manifest to pin it."
        );
    }

    fs::create_dir_all(dest_dir)
        .map_err(|err| format!("Failed to create {} directory: {}", label, err))?;

    let wasm_dest = dest_dir.join(format!("{}{}.wasm", prefix, name));
    crate::atomic_fs::write_atomic(&wasm_dest, &bytes)
        .map_err(|err| format!("Failed to write '{}': {}", wasm_dest.display(), err))?;

    if !is_wasm_file(&wasm_dest) {
        let _ = fs::remove_file(&wasm_dest);
        return Err(format!(
            "Downloaded file from '{}' is not a WebAssembly binary",
            url
        ));
    }

    if let Some(manifest) = manifest {
        let sidecar = dest_dir.join(format!("{}{}.json", prefix, name));
        crate::atomic_fs::write_atomic(
            &sidecar,
            &serde_json::to_vec_pretty(&manifest)
                .map_err(|err| format!("Failed to serialize manifest: {}", err))?,
        )
        .map_err(|err| format!("Failed to write manifest '{}': {}", sidecar.display(), err))?;
    }

    println!(
        "Installed wasm {} '{}' to {}",
        label,
        name,
        wasm_dest.display()
    );
    Ok(())
}

/// Installs a single `.wasm` file chosen by the user.
///
/// `sha256` is enforced against the local file bytes before any write.
pub fn install_file(
    artifact: &Path,
    sha256: Option<&str>,
    dest_dir: &Path,
    prefix: &str,
    name: &str,
    label: &str,
) -> Result<(), String> {
    let manifest_path = crate::wasm::manifest::sidecar_path(artifact).filter(|path| path.is_file());
    let manifest = match &manifest_path {
        Some(path) => Some(parse_manifest(path)?),
        None => None,
    };

    install(
        InstallPlan {
            artifact: artifact.to_path_buf(),
            manifest_path,
            manifest,
        },
        sha256,
        dest_dir,
        prefix,
        name,
        label,
    )
}

/// Derives a guest name from a URL or file path (`foo.wasm` -> `foo`).
pub fn name_from_path(path: &str) -> Option<String> {
    let trimmed = path.split(['?', '#']).next().unwrap_or(path);
    let file = trimmed.rsplit('/').next()?;
    let stem = file.strip_suffix(".wasm").unwrap_or(file);
    if stem.is_empty() {
        None
    } else {
        Some(stem.to_string())
    }
}

/// Source manifest names accepted in a repository directory.
///
/// Only the explicit label names are recognized so an unrelated `<name>.json`
/// file in a native plugin directory can never be mistaken for a manifest.
fn find_source_manifest(dir: &Path, label: &str) -> Option<PathBuf> {
    let candidates = [format!("xfetch-{}.json", label), format!("{}.json", label)];
    candidates
        .iter()
        .map(|candidate| dir.join(candidate))
        .find(|path| path.is_file())
}

/// Parses a source manifest with a helpful error message.
fn parse_manifest(path: &Path) -> Result<Manifest, String> {
    let bytes = fs::read(path)
        .map_err(|err| format!("Failed to read manifest '{}': {}", path.display(), err))?;
    let mut manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|err| format!("Invalid manifest '{}': {}", path.display(), err))?;
    if manifest.manifest_version == 0 {
        manifest.manifest_version = crate::wasm::manifest::MANIFEST_VERSION;
    }
    Ok(manifest)
}

/// Resolves the artifact path using the manifest and conventional locations.
fn find_artifact(
    dir: &Path,
    label: &str,
    name: &str,
    manifest: Option<&Manifest>,
) -> Option<PathBuf> {
    if let Some(relative) = manifest.and_then(|m| m.artifact.as_deref()) {
        let candidate = dir.join(relative);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let candidates = [
        dir.join(format!("dist/{}.wasm", name)),
        dir.join(format!("{}.wasm", name)),
        dir.join(format!("xfetch-plugin-{}.wasm", name)),
        dir.join(format!("xfetch-effect-{}.wasm", name)),
        dir.join(format!("xfetch-extension-{}.wasm", name)),
        dir.join(format!("target/wasm32-wasip1/release/{}.wasm", name)),
        dir.join(format!(
            "target/wasm32-wasip1/release/xfetch-{}-{}.wasm",
            label, name
        )),
        dir.join(format!("target/wasm32-wasi/release/{}.wasm", name)),
    ];

    candidates.into_iter().find(|path| path.is_file())
}

/// True when a manifest asks for a wasm runtime.
fn declares_wasm(manifest: Option<&Manifest>) -> bool {
    let Some(manifest) = manifest else {
        return false;
    };
    if manifest.artifact.is_some() || manifest.artifact_url.is_some() || manifest.build.is_some() {
        return true;
    }
    matches!(
        manifest.runtime.as_deref(),
        Some("wasm") | Some("core") | Some("component")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    const MINIMAL_WASM: &[u8] = b"\0asm\x01\0\0\0";

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "xfetch-wasm-install-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn name_from_url_strips_extension_and_query() {
        assert_eq!(
            name_from_path("https://example.com/releases/foo.wasm?download=1"),
            Some("foo".to_string())
        );
        assert_eq!(
            name_from_path("/tmp/xfetch-plugin-bar.wasm"),
            Some("xfetch-plugin-bar".to_string())
        );
    }

    #[test]
    fn plan_detects_artifact_url() {
        let dir = temp_dir("url");
        fs::write(
            dir.join("xfetch-plugin.json"),
            r#"{ "artifact_url": "https://example.com/foo.wasm" }"#,
        )
        .expect("write manifest");

        match plan(&dir, "plugin", "foo").expect("plan") {
            SourceAction::Download { url, .. } => {
                assert_eq!(url, "https://example.com/foo.wasm");
            }
            other => panic!("expected download, got {:?}", other),
        }
    }

    #[test]
    fn plan_falls_back_to_native_without_hints() {
        let dir = temp_dir("native");
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"foo\"").expect("write cargo");
        assert!(matches!(
            plan(&dir, "plugin", "foo").expect("plan"),
            SourceAction::NotWasm
        ));
    }

    #[test]
    fn plan_requires_artifact_when_manifest_declares_wasm() {
        let dir = temp_dir("missing");
        fs::write(dir.join("xfetch-plugin.json"), r#"{ "runtime": "core" }"#)
            .expect("write manifest");
        assert!(plan(&dir, "plugin", "foo").is_err());
    }

    #[test]
    fn plan_ignores_unrelated_json_sidecar() {
        let dir = temp_dir("sidecar");
        fs::write(dir.join("foo.json"), r#"{ "unrelated": true }"#).expect("write json");
        // A JSON file named after the guest parses as a manifest but has no
        // wasm hints; the source is not wasm-only, so the native path wins.
        assert!(matches!(
            plan(&dir, "plugin", "foo").expect("plan"),
            SourceAction::NotWasm
        ));
    }

    #[test]
    fn local_artifact_with_matching_sha256_installs() {
        let dir = temp_dir("local-ok");
        let dest = dir.join("plugins");
        let artifact = dir.join("xfetch-plugin-sample.wasm");
        fs::write(&artifact, MINIMAL_WASM).expect("write wasm");
        let hash = crate::hash::sha256_hex(MINIMAL_WASM);

        install_file(
            &artifact,
            Some(&hash),
            &dest,
            "xfetch-plugin-",
            "sample",
            "plugin",
        )
        .expect("install verified artifact");

        assert_eq!(
            fs::read(dest.join("xfetch-plugin-sample.wasm")).expect("read installed"),
            MINIMAL_WASM
        );
    }

    #[test]
    fn local_artifact_with_wrong_sha256_is_not_installed() {
        let dir = temp_dir("local-bad");
        let dest = dir.join("plugins");
        let artifact = dir.join("xfetch-plugin-sample.wasm");
        fs::write(&artifact, MINIMAL_WASM).expect("write wasm");
        let wrong = "0".repeat(64);

        let err = install_file(
            &artifact,
            Some(&wrong),
            &dest,
            "xfetch-plugin-",
            "sample",
            "plugin",
        )
        .expect_err("wrong digest must be rejected");

        assert!(
            err.contains("SHA-256 mismatch"),
            "unexpected error: {}",
            err
        );
        assert!(!dest.join("xfetch-plugin-sample.wasm").exists());
    }

    #[test]
    fn checked_in_artifact_is_verified_instead_of_falling_back_to_url() {
        let dir = temp_dir("checked-in");
        let dest = dir.join("plugins");
        fs::write(dir.join("chosen.wasm"), MINIMAL_WASM).expect("write wasm");
        fs::write(
            dir.join("xfetch-plugin.json"),
            r#"{
                "artifact": "chosen.wasm",
                "artifact_url": "http://127.0.0.1:9/should-not-be-used.wasm"
            }"#,
        )
        .expect("write manifest");

        let plan = match plan(&dir, "plugin", "sample").expect("plan") {
            SourceAction::Install(plan) => plan,
            other => panic!("expected checked-in artifact, got {:?}", other),
        };
        // The digest belongs to some other artifact: fail instead of silently
        // downloading the artifact_url alternative.
        let err = install(
            plan,
            Some(&"a".repeat(64)),
            &dest,
            "xfetch-plugin-",
            "sample",
            "plugin",
        )
        .expect_err("wrong checked-in digest must be rejected");

        assert!(
            err.contains("SHA-256 mismatch"),
            "unexpected error: {}",
            err
        );
        assert!(!dest.join("xfetch-plugin-sample.wasm").exists());
    }

    fn serve_once(body: Vec<u8>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("test server address");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0u8; 2048];
            let _ = stream.read(&mut request);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/wasm\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write headers");
            stream.write_all(&body).expect("write body");
            stream.flush().expect("flush response");
        });
        (format!("http://{}/plugin.wasm", addr), handle)
    }

    #[test]
    fn download_with_matching_sha256_still_installs() {
        let body = MINIMAL_WASM.to_vec();
        let (url, server) = serve_once(body.clone());
        let dir = temp_dir("download-ok");
        let dest = dir.join("plugins");
        let hash = crate::hash::sha256_hex(&body);

        download_and_install(
            &url,
            None,
            Some(&hash),
            &dest,
            "xfetch-plugin-",
            "remote",
            "plugin",
        )
        .expect("download and verify");
        server.join().expect("test server");

        assert_eq!(
            fs::read(dest.join("xfetch-plugin-remote.wasm")).expect("read installed"),
            body
        );
    }
}
