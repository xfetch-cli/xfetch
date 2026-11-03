//! Helpers for direct wasm installs and post-install addon identity.

use crate::addons::model::AddonKind;
use crate::install_support::validate_guest_name;
use crate::wasm::detect::is_wasm_file;
use crate::wasm::manifest::Manifest;
use std::fs;
use std::path::Path;

/// Derives the installed name from a local path, URL or remote shorthand.
///
/// The installer uses the same conventions, so this keeps the post-install
/// guidance and `info` commands aligned with the actual installed files.
pub fn infer_name(
    kind: AddonKind,
    source: &str,
    name_override: Option<&str>,
) -> Result<String, String> {
    if let Some(name) = name_override {
        validate_guest_name(name)?;
        return Ok(name.to_string());
    }

    if is_url(source) {
        let raw = crate::wasm::install::name_from_path(source)
            .ok_or_else(|| format!("Cannot derive a name from URL '{}'", source))?;
        let name = strip_prefix(kind, &raw);
        validate_guest_name(&name)?;
        return Ok(name);
    }

    let path = Path::new(source);
    if path.is_dir() {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("Cannot derive a name from '{}'", source))?;
        validate_guest_name(name)?;
        return Ok(name.to_string());
    }

    if path.is_file() {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("Cannot derive a name from '{}'", source))?;
        let stem = file_name.strip_suffix(".wasm").unwrap_or(file_name);
        let name = strip_prefix(kind, stem);
        validate_guest_name(&name)?;
        return Ok(name);
    }

    // Remote repository paths and non-existent path-like sources install
    // under the final path component. Apply the same suffix/prefix rules used
    // for existing files so a dry-run path and the installed name agree.
    let last = source.rsplit(['/', '\\']).next().unwrap_or(source);
    let stem = last.strip_suffix(".wasm").unwrap_or(last);
    let name = strip_prefix(kind, stem);
    validate_guest_name(&name)?;
    Ok(name)
}

/// Installs a direct `.wasm` file or URL. This is the path used when the user
/// passes `--name` or `--manifest`, which the directory/repository installers
/// do not support.
pub fn install_direct(
    kind: AddonKind,
    source: &str,
    sha256: Option<&str>,
    name_override: Option<&str>,
    manifest_path: Option<&str>,
) -> Result<String, String> {
    let name = infer_name(kind, source, name_override)?;
    let directory = kind.directory();
    let manifest = manifest_path.map(parse_manifest).transpose()?;

    if is_url(source) {
        crate::wasm::install::download_and_install(
            source,
            manifest,
            sha256,
            &directory,
            kind.prefix(),
            &name,
            kind.label(),
        )?;
        return Ok(name);
    }

    let path = Path::new(source);
    if !path.is_file() || !is_wasm_file(path) {
        return Err(format!(
            "'{}' is not a WebAssembly file; --name/--manifest apply only to direct .wasm \
             files or URLs",
            source
        ));
    }

    crate::wasm::install::install_file(
        path,
        sha256,
        &directory,
        kind.prefix(),
        &name,
        kind.label(),
    )?;

    if let Some(manifest) = manifest {
        write_sidecar(kind, &name, &manifest, manifest_path)?;
    }

    Ok(name)
}

/// Parses a user-supplied sidecar manifest before it is copied into the
/// installation directory.
pub fn parse_manifest(path: &str) -> Result<Manifest, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("Failed to read manifest '{}': {}", path, err))?;
    serde_json::from_slice(&bytes).map_err(|err| format!("Invalid manifest '{}': {}", path, err))
}

fn write_sidecar(
    kind: AddonKind,
    name: &str,
    manifest: &Manifest,
    source: Option<&str>,
) -> Result<(), String> {
    let directory = kind.directory();
    fs::create_dir_all(&directory)
        .map_err(|err| format!("Failed to create {} directory: {}", kind.label(), err))?;
    let destination = directory.join(kind.manifest_name(name));
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|err| format!("Failed to serialize manifest: {}", err))?;
    crate::atomic_fs::write_atomic(&destination, &bytes).map_err(|err| {
        format!(
            "Failed to write manifest '{}': {}",
            destination.display(),
            err
        )
    })?;
    if let Some(source) = source {
        println!("Installed manifest '{}'", source);
    }
    Ok(())
}

fn strip_prefix(kind: AddonKind, value: &str) -> String {
    let stripped = value.strip_prefix(kind.prefix()).unwrap_or(value);
    stripped
        .strip_suffix(".wasm")
        .unwrap_or(stripped)
        .to_string()
}

fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_plugin_name_from_wasm_file() {
        assert_eq!(
            infer_name(AddonKind::Plugin, "/tmp/xfetch-plugin-weather.wasm", None).expect("name"),
            "weather"
        );
    }

    #[test]
    fn infers_effect_name_from_directory() {
        assert_eq!(
            infer_name(AddonKind::Effect, "/tmp/effects/wasm-matrix", None).expect("name"),
            "wasm-matrix"
        );
    }

    #[test]
    fn infers_name_from_url() {
        assert_eq!(
            infer_name(
                AddonKind::Extension,
                "https://example.com/xfetch-extension-night-mode.wasm",
                None
            )
            .expect("name"),
            "night-mode"
        );
    }

    #[test]
    fn accepts_remote_shorthand() {
        assert_eq!(
            infer_name(AddonKind::Plugin, "weather", None).expect("name"),
            "weather"
        );
        assert_eq!(
            infer_name(AddonKind::Plugin, "plugins/weather", None).expect("name"),
            "weather"
        );
    }

    #[test]
    fn override_wins() {
        assert_eq!(
            infer_name(AddonKind::Plugin, "/tmp/plugin.wasm", Some("custom")).expect("name"),
            "custom"
        );
    }
}
