//! Self-update support (`xfetch update`).
//!
//! The command never guesses: it detects how this binary was installed and
//! only touches the file when doing so is safe.
//!
//! - `--check` reports whether a newer release exists without installing.
//! - Installs done with `install-prebuilt.sh` (typically `~/.local/bin`) are
//!   updated in place: the release asset is downloaded, verified against the
//!   published `SHA256SUMS`, extracted to a temporary file and moved over the
//!   current executable with an atomic rename. A single `xfetch.bak` keeps the
//!   previous binary for rollback (overwritten on every update).
//! - `cargo install` binaries are updated through `cargo install --force`.
//! - Package-manager installs and local builds are never replaced; the
//!   command prints the right command instead.
//!
//! Windows prebuilt updates are intentionally not implemented yet: the `zip`
//! asset published by the release workflow is not reliable, so the command
//! only reports and suggests `cargo install` or a manual update there.

mod github;
#[cfg(unix)]
mod install;
mod provenance;

use crate::update::github::{Release, fetch_latest};
use crate::update::provenance::InstallSource;
use semver::Version;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;

/// GitHub API endpoint used to discover the latest release. Overridable for
/// mirrors and tests with `XFETCH_UPDATE_API`.
const DEFAULT_UPDATE_API: &str = "https://api.github.com/repos/xfetch-cli/xfetch/releases/latest";

/// Options collected from the CLI.
#[derive(Debug, Default)]
pub struct UpdateOptions {
    /// Only report; never install.
    pub check: bool,
    /// Force the in-place prebuilt update (Unix only).
    pub prebuilt: bool,
    /// Directory that holds the xfetch binary (with `--prebuilt`).
    #[cfg_attr(not(unix), allow(dead_code))]
    pub bin_dir: Option<PathBuf>,
    /// Answer yes to confirmation prompts.
    pub yes: bool,
}

/// What the command decided (and did).
#[derive(Debug, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// The installed version is current.
    UpToDate { current: Version, latest: Version },
    /// A newer release exists; `command` is the recommended way to install it.
    UpdateAvailable {
        current: Version,
        latest: Version,
        method: &'static str,
        command: String,
    },
    /// The prebuilt binary was replaced.
    Updated {
        from: Version,
        to: Version,
        path: PathBuf,
    },
    /// Nothing was changed; the message explains what to do.
    Manual { message: String },
}

/// Checks for updates and, when allowed, installs the new release.
pub fn run(options: UpdateOptions) -> Result<UpdateOutcome, String> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|err| format!("Invalid built-in version: {}", err))?;
    let client = github::Client::new()?;
    let latest = fetch_latest(&client, &api_base())?;

    if latest.version <= current {
        return Ok(UpdateOutcome::UpToDate {
            current,
            latest: latest.version.clone(),
        });
    }

    let executable = std::env::current_exe()
        .map_err(|err| format!("Cannot locate the running xfetch binary: {}", err))?;
    let source = provenance::detect(&executable);

    if options.check {
        let command = recommended_command(source, &latest);
        return Ok(UpdateOutcome::UpdateAvailable {
            current,
            latest: latest.version.clone(),
            method: source.label(),
            command,
        });
    }

    if options.prebuilt {
        return install_prebuilt(&client, &latest, current, &executable, &options);
    }

    match source {
        InstallSource::Prebuilt => {
            install_prebuilt(&client, &latest, current, &executable, &options)
        }
        InstallSource::Cargo => run_cargo_update(&latest, current, &options),
        InstallSource::PackageManager => Ok(UpdateOutcome::Manual {
            message: format!(
                "xfetch {} is available, but this binary is managed by a package manager.\n\
                 Update it with your package manager (pacman, apt, brew, winget, ...).",
                latest.version
            ),
        }),
        InstallSource::Unknown => Ok(UpdateOutcome::Manual {
            message: format!(
                "xfetch {} is available, but this binary is not a recognized prebuilt install.\n\
                 Update it your way, or run:\n  xfetch update --prebuilt --bin-dir <directory>",
                latest.version
            ),
        }),
    }
}

/// The API base, honoring the mirror override.
fn api_base() -> String {
    std::env::var("XFETCH_UPDATE_API").unwrap_or_else(|_| DEFAULT_UPDATE_API.to_string())
}

/// Suggested command for `--check` output, tailored to the install source.
fn recommended_command(source: InstallSource, release: &Release) -> String {
    match source {
        InstallSource::Cargo => "cargo install xfetch-cli --force".to_string(),
        InstallSource::PackageManager => {
            "update through your package manager (pacman, apt, brew, winget, ...)".to_string()
        }
        InstallSource::Prebuilt => format!("xfetch update  (installs {})", release.version),
        InstallSource::Unknown => {
            "run install-prebuilt.sh or: xfetch update --prebuilt --bin-dir <directory>".to_string()
        }
    }
}

/// Asks for confirmation unless `--yes` was passed.
fn confirm(version: &Version, options: &UpdateOptions) -> Result<bool, String> {
    if options.yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        return Err(format!(
            "Refusing to install xfetch {} without confirmation in a non-interactive shell; \
             re-run with --yes",
            version
        ));
    }

    print!("Update to xfetch {}? [Y/n] ", version);
    std::io::stdout()
        .flush()
        .map_err(|err| format!("Failed to write prompt: {}", err))?;

    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .map_err(|err| format!("Failed to read confirmation: {}", err))?;
    let answer = answer.trim().to_ascii_lowercase();
    Ok(answer.is_empty() || answer == "y" || answer == "yes")
}

/// Updates a prebuilt install in place (Unix) or explains why it cannot.
fn install_prebuilt(
    client: &github::Client,
    release: &Release,
    current: Version,
    executable: &std::path::Path,
    options: &UpdateOptions,
) -> Result<UpdateOutcome, String> {
    #[cfg(not(unix))]
    {
        let _ = (client, release, current, executable, options);
        Err(format!(
            "Prebuilt updates are not supported on this platform yet. \
             Install xfetch {} with: cargo install xfetch-cli --force",
            release.version
        ))
    }

    #[cfg(unix)]
    {
        if !confirm(&release.version, options)? {
            return Ok(UpdateOutcome::Manual {
                message: "Update cancelled.".to_string(),
            });
        }

        let target_dir = match &options.bin_dir {
            Some(dir) => dir.clone(),
            None => executable
                .parent()
                .map(std::path::Path::to_path_buf)
                .ok_or_else(|| "Cannot determine the binary directory".to_string())?,
        };
        let target = install::update_prebuilt(client, release, &target_dir)
            .map_err(|err| format!("Update failed: {}", err))?;

        Ok(UpdateOutcome::Updated {
            from: current,
            to: release.version.clone(),
            path: target,
        })
    }
}

/// Runs `cargo install xfetch-cli --force --locked` for cargo installs.
fn run_cargo_update(
    release: &Release,
    current: Version,
    options: &UpdateOptions,
) -> Result<UpdateOutcome, String> {
    if !options.yes {
        if !std::io::stdin().is_terminal() {
            return Ok(UpdateOutcome::Manual {
                message: format!(
                    "xfetch {} is available; this is a cargo install.\n  cargo install xfetch-cli --force",
                    release.version
                ),
            });
        }
        if !confirm(&release.version, options)? {
            return Ok(UpdateOutcome::Manual {
                message: "Update cancelled.".to_string(),
            });
        }
    }

    let status = std::process::Command::new("cargo")
        .args(["install", "xfetch-cli", "--force", "--locked"])
        .status()
        .map_err(|err| format!("Failed to run cargo: {}", err))?;
    if !status.success() {
        return Err("cargo install failed".to_string());
    }

    let path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("xfetch"));
    Ok(UpdateOutcome::Updated {
        from: current,
        to: release.version.clone(),
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommended_command_matches_the_install_source() {
        let release = Release {
            tag: "v9.9.9".to_string(),
            version: Version::parse("9.9.9").expect("version"),
            assets: Vec::new(),
            html_url: String::new(),
        };
        assert!(recommended_command(InstallSource::Cargo, &release).contains("cargo install"));
        assert!(
            recommended_command(InstallSource::PackageManager, &release)
                .contains("package manager")
        );
        assert!(recommended_command(InstallSource::Prebuilt, &release).contains("xfetch update"));
    }
}
