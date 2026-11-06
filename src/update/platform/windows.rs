//! Windows self-update policy.

use crate::update::github::Release;
use crate::update::{UpdateOptions, UpdateOutcome};
use semver::Version;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

/// Release target for the Windows build.
///
/// The release workflow publishes the MSVC archive. A GNU-built local binary
/// can still consume it when the user explicitly asks for a prebuilt update.
pub(crate) fn release_target() -> Option<&'static str> {
    if cfg!(target_arch = "x86_64") {
        Some("x86_64-pc-windows-msvc")
    } else {
        None
    }
}

/// Executable name inside the archive and on disk.
pub(crate) const fn binary_name() -> &'static str {
    "xfetch.exe"
}

/// Windows reports a locked `cargo install` as actionable instructions instead
/// of a hard failure: the user usually only has to close another xfetch process.
pub(crate) fn cargo_failure(command: &str) -> Result<UpdateOutcome, String> {
    Ok(UpdateOutcome::Manual {
        message: format!(
            "cargo install could not replace the running binary.\n\
             Close any other xfetch processes and run:\n  {}",
            command
        ),
    })
}

/// A missing `cargo` is actionable on Windows rather than a hard failure.
pub(crate) fn cargo_spawn_failure(command: &str, err: &str) -> Result<UpdateOutcome, String> {
    Ok(UpdateOutcome::Manual {
        message: format!(
            "Failed to run cargo: {}\nClose any other xfetch processes and run:\n  {}",
            err, command
        ),
    })
}

/// Delegates a winget install to the package manager.
pub(crate) fn run_winget_update(
    release: &Release,
    current: Version,
    executable: &Path,
    options: &UpdateOptions,
) -> Result<UpdateOutcome, String> {
    let command = format!(
        "winget upgrade --id {} --exact --silent --accept-package-agreements --accept-source-agreements",
        super::super::WINGET_PACKAGE_ID
    );

    if !options.yes {
        if !std::io::stdin().is_terminal() {
            return Ok(UpdateOutcome::Manual {
                message: format!(
                    "xfetch {} is available; this is a winget install.\n  {}",
                    release.version, command
                ),
            });
        }
        if !super::super::confirm(&release.version, options)? {
            return Ok(UpdateOutcome::Manual {
                message: "Update cancelled.".to_string(),
            });
        }
    }

    let status = Command::new("winget")
        .args([
            "upgrade",
            "--id",
            super::super::WINGET_PACKAGE_ID,
            "--exact",
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ])
        .status();

    match status {
        Ok(status) if status.success() => {
            if super::super::installed_version(executable)
                .is_some_and(|version| version >= release.version)
            {
                Ok(UpdateOutcome::Updated {
                    from: current,
                    to: release.version.clone(),
                    path: executable.to_path_buf(),
                })
            } else {
                Ok(UpdateOutcome::Manual {
                    message: format!(
                        "winget reported success, but xfetch is still {}.\n\
                         The winget manifest may not include {} yet. Try again later:\n  {}",
                        current, release.version, command
                    ),
                })
            }
        }
        Ok(_) => Ok(UpdateOutcome::Manual {
            message: format!(
                "winget could not upgrade xfetch. Close other xfetch processes and run:\n  {}",
                command
            ),
        }),
        Err(err) => Ok(UpdateOutcome::Manual {
            message: format!(
                "winget is not available ({}). Update it manually:\n  {}",
                err, command
            ),
        }),
    }
}
