//! Self-update support (`xfetch update`).
//!
//! The command never guesses: it detects how this binary was installed and
//! only touches the file when doing so is safe.
//!
//! - `--check` reports whether a newer release exists without installing.
//! - Installs done with the prebuilt installer are updated in place on Unix
//!   and Windows. The release asset is downloaded, verified against the
//!   published `SHA256SUMS`, extracted to a temporary file and moved over the
//!   current executable. A single backup (`xfetch.bak` / `xfetch.exe.bak`)
//!   keeps the previous binary for rollback and is overwritten on every update.
//! - `cargo install` binaries are updated through `cargo install --force`.
//!   Binaries built from a checkout (`cargo install --path`, `install.ps1`)
//!   are rebuilt from their source when the checkout still exists.
//! - winget installs are delegated to `winget upgrade`; xfetch never replaces
//!   a winget-managed file itself.
//! - Package-manager installs and unrecognized local builds are never
//!   replaced; the command prints the right command instead.

mod github;
mod install;
mod platform;
mod provenance;
mod receipt;

use crate::update::github::{Release, fetch_latest};
use crate::update::provenance::{DetectedInstall, InstallSource};
use semver::Version;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// GitHub API endpoint used to discover the latest release. Overridable for
/// mirrors and tests with `XFETCH_UPDATE_API`.
const DEFAULT_UPDATE_API: &str = "https://api.github.com/repos/xfetch-cli/xfetch/releases/latest";

/// Package identifier used by the winget manifest.
#[cfg(windows)]
pub const WINGET_PACKAGE_ID: &str = "Xfetch-cli.Xfetch";

/// Options collected from the CLI.
#[derive(Debug, Default)]
pub struct UpdateOptions {
    /// Only report; never install.
    pub check: bool,
    /// Force the in-place prebuilt update (Unix and Windows).
    pub prebuilt: bool,
    /// Directory that holds the xfetch binary (with `--prebuilt`).
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
    let detected = provenance::detect(&executable);

    if options.check {
        let command = recommended_command(&detected, &latest);
        return Ok(UpdateOutcome::UpdateAvailable {
            current,
            latest: latest.version.clone(),
            method: detected.source.label(),
            command,
        });
    }

    if options.prebuilt {
        if detected.source.is_package_managed() {
            return Ok(UpdateOutcome::Manual {
                message: format!(
                    "xfetch {} is available, but this binary is managed by {}.\n\
                     Refusing to replace it with --prebuilt. Use:\n  {}",
                    latest.version,
                    detected.source.label(),
                    recommended_command(&detected, &latest)
                ),
            });
        }
        return install_prebuilt(&client, &latest, current, &executable, &options);
    }

    match detected.source {
        InstallSource::Prebuilt => {
            install_prebuilt(&client, &latest, current, &executable, &options)
        }
        InstallSource::Cargo => run_cargo_update(&latest, current, &options, None),
        InstallSource::SourceBuild => match detected.source_path.as_deref() {
            Some(path) => run_cargo_update(&latest, current, &options, Some(path)),
            None => Ok(UpdateOutcome::Manual {
                message: format!(
                    "xfetch {} is available, but this is a source build and the checkout is unknown.\n\
                     Re-run the installer or rebuild with:\n  cargo install --path <directory> --force --locked",
                    latest.version
                ),
            }),
        },
        #[cfg(windows)]
        InstallSource::Winget => {
            platform::run_winget_update(&latest, current, &executable, &options)
        }
        #[cfg(unix)]
        InstallSource::PackageManager => Ok(UpdateOutcome::Manual {
            message: format!(
                "xfetch {} is available, but this binary is managed by a package manager.\n\
                 Update it with your package manager (pacman, apt, brew, ...).",
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
fn recommended_command(detected: &DetectedInstall, release: &Release) -> String {
    match detected.source {
        InstallSource::Cargo => "cargo install xfetch-cli --force --locked".to_string(),
        InstallSource::SourceBuild => detected.source_path.as_deref().map_or_else(
            || {
                "rebuild from the source checkout: cargo install --path <directory> --force --locked"
                    .to_string()
            },
            |path| {
                format!(
                    "cargo install --path \"{}\" --force --locked",
                    path.display()
                )
            },
        ),
        #[cfg(windows)]
        InstallSource::Winget => {
            format!("winget upgrade --id {} --exact", WINGET_PACKAGE_ID)
        }
        #[cfg(unix)]
        InstallSource::PackageManager => {
            "update through your package manager (pacman, apt, brew, ...)".to_string()
        }
        InstallSource::Prebuilt => format!("xfetch update  (installs {})", release.version),
        InstallSource::Unknown => {
            "run the prebuilt installer or: xfetch update --prebuilt --bin-dir <directory>"
                .to_string()
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

/// Updates a prebuilt install in place.
fn install_prebuilt(
    client: &github::Client,
    release: &Release,
    current: Version,
    executable: &Path,
    options: &UpdateOptions,
) -> Result<UpdateOutcome, String> {
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (client, release, current, executable, options);
        return Err(format!(
            "Prebuilt updates are not supported on this platform. \
             Install xfetch {} with: cargo install xfetch-cli --force",
            release.version
        ));
    }

    #[cfg(any(unix, windows))]
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
                .map(Path::to_path_buf)
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

/// Runs `cargo install` for registry installs, or rebuilds a source checkout.
fn run_cargo_update(
    release: &Release,
    current: Version,
    options: &UpdateOptions,
    source_path: Option<&Path>,
) -> Result<UpdateOutcome, String> {
    let command = source_path.map_or_else(
        || "cargo install xfetch-cli --force --locked".to_string(),
        |path| {
            format!(
                "cargo install --path \"{}\" --force --locked",
                path.display()
            )
        },
    );

    if !options.yes {
        if !std::io::stdin().is_terminal() {
            return Ok(UpdateOutcome::Manual {
                message: format!(
                    "xfetch {} is available; this is a {} install.\n  {}",
                    release.version,
                    if source_path.is_some() {
                        "source"
                    } else {
                        "cargo"
                    },
                    command
                ),
            });
        }
        if !confirm(&release.version, options)? {
            return Ok(UpdateOutcome::Manual {
                message: "Update cancelled.".to_string(),
            });
        }
    }

    let status = match source_path {
        Some(path) => Command::new("cargo")
            .arg("install")
            .arg("--path")
            .arg(path)
            .args(["--force", "--locked"])
            .status(),
        None => Command::new("cargo")
            .args(["install", "xfetch-cli", "--force", "--locked"])
            .status(),
    };

    match status {
        Ok(status) if status.success() => {
            let path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("xfetch"));
            if installed_version(&path).is_some_and(|version| version >= release.version) {
                Ok(UpdateOutcome::Updated {
                    from: current,
                    to: release.version.clone(),
                    path,
                })
            } else {
                Ok(UpdateOutcome::Manual {
                    message: format!(
                        "cargo install reported success, but xfetch is still {}.\n\
                         Close any other xfetch processes and run:\n  {}",
                        current, command
                    ),
                })
            }
        }
        Ok(_) => platform::cargo_failure(&command),
        Err(err) => platform::cargo_spawn_failure(&command, &err.to_string()),
    }
}

/// Reads the version printed by `xfetch --version`.
fn installed_version(executable: &Path) -> Option<Version> {
    let output = Command::new(executable).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    parse_version_output(&String::from_utf8_lossy(&output.stdout))
        .or_else(|| parse_version_output(&String::from_utf8_lossy(&output.stderr)))
}

fn parse_version_output(output: &str) -> Option<Version> {
    let token = output.split_whitespace().nth(1)?;
    Version::parse(token.trim_start_matches('v')).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(version: &str) -> Release {
        Release {
            tag: format!("v{}", version),
            version: Version::parse(version).expect("version"),
            assets: Vec::new(),
        }
    }

    fn detected(source: InstallSource, path: Option<&str>) -> DetectedInstall {
        DetectedInstall {
            source,
            source_path: path.map(PathBuf::from),
        }
    }

    #[test]
    fn recommended_command_matches_the_install_source() {
        let release = release("9.9.9");
        assert!(
            recommended_command(&detected(InstallSource::Cargo, None), &release)
                .contains("cargo install")
        );
        assert!(
            recommended_command(
                &detected(InstallSource::SourceBuild, Some("/src/xfetch")),
                &release
            )
            .contains("--path")
        );
        #[cfg(windows)]
        assert!(
            recommended_command(&detected(InstallSource::Winget, None), &release)
                .contains("winget upgrade")
        );
        #[cfg(unix)]
        assert!(
            recommended_command(&detected(InstallSource::PackageManager, None), &release)
                .contains("package manager")
        );
        assert!(
            recommended_command(&detected(InstallSource::Prebuilt, None), &release)
                .contains("xfetch update")
        );
    }

    #[test]
    fn parses_version_output() {
        assert_eq!(
            parse_version_output("xfetch 1.2.3\n"),
            Some(Version::parse("1.2.3").expect("version"))
        );
        assert_eq!(
            parse_version_output("xfetch v1.2.3\n"),
            Some(Version::parse("1.2.3").expect("version"))
        );
        assert_eq!(parse_version_output("not a version"), None);
    }

    #[test]
    fn package_managed_sources_refuse_forced_prebuilt() {
        for source in [InstallSource::Cargo, InstallSource::SourceBuild] {
            assert!(source.is_package_managed(), "{source:?}");
        }
        #[cfg(windows)]
        assert!(InstallSource::Winget.is_package_managed());
        #[cfg(unix)]
        assert!(!InstallSource::PackageManager.is_package_managed());
        assert!(!InstallSource::Prebuilt.is_package_managed());
        assert!(!InstallSource::Unknown.is_package_managed());
    }
}
