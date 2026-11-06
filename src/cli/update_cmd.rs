use crate::cli::args::Cli;
use crate::error::XFetchError;

#[cfg(feature = "update")]
use crate::cli::output;
#[cfg(feature = "update")]
use crate::runtime;
#[cfg(feature = "update")]
use std::path::PathBuf;

#[cfg(feature = "update")]
pub fn run(
    check: bool,
    prebuilt: bool,
    bin_dir: Option<String>,
    yes: bool,
    rollback: bool,
    cli: &Cli,
) -> Result<(), XFetchError> {
    if rollback {
        if check || prebuilt {
            return Err(XFetchError::Usage(
                "--rollback cannot be combined with --check or --prebuilt".to_string(),
            ));
        }
        let path =
            crate::update::rollback(bin_dir.map(PathBuf::from)).map_err(XFetchError::Update)?;
        if cli.json {
            return output::print_json(&serde_json::json!({
                "schema_version": output::JSON_SCHEMA_VERSION,
                "status": "rolled-back",
                "path": path.display().to_string(),
            }));
        }
        runtime::info(format!("Rolled back xfetch binary: {}", path.display()));
        return Ok(());
    }

    if cli.json && !check {
        return Err(XFetchError::Usage(
            "--json for 'update' requires --check.".to_string(),
        ));
    }

    let options = crate::update::UpdateOptions {
        check,
        prebuilt,
        bin_dir: bin_dir.map(PathBuf::from),
        yes,
    };
    let outcome = crate::update::run(options).map_err(XFetchError::Update)?;

    if cli.json {
        use crate::update::UpdateOutcome;
        let value = match &outcome {
            UpdateOutcome::UpToDate { current, latest } => serde_json::json!({
                "schema_version": output::JSON_SCHEMA_VERSION,
                "status": "up-to-date",
                "current": current.to_string(),
                "latest": latest.to_string(),
                "update_available": false,
            }),
            UpdateOutcome::UpdateAvailable {
                current,
                latest,
                method,
                command,
            } => serde_json::json!({
                "schema_version": output::JSON_SCHEMA_VERSION,
                "status": "update-available",
                "current": current.to_string(),
                "latest": latest.to_string(),
                "method": method,
                "command": command,
                "update_available": true,
            }),
            UpdateOutcome::Updated { from, to, path } => serde_json::json!({
                "schema_version": output::JSON_SCHEMA_VERSION,
                "status": "updated",
                "current": from.to_string(),
                "latest": to.to_string(),
                "path": path.display().to_string(),
                "update_available": false,
            }),
            UpdateOutcome::Manual { message } => serde_json::json!({
                "schema_version": output::JSON_SCHEMA_VERSION,
                "status": "manual",
                "message": message,
                "update_available": false,
            }),
        };
        output::print_json(&value)?;
        if check && matches!(outcome, UpdateOutcome::UpdateAvailable { .. }) {
            return Err(XFetchError::Exit(crate::error::EXIT_GENERIC));
        }
        return Ok(());
    }

    use crate::update::UpdateOutcome;
    match outcome {
        UpdateOutcome::UpToDate { current, latest } => {
            runtime::info(format!(
                "xfetch {} is up to date (latest v{}).",
                current, latest
            ));
        }
        UpdateOutcome::UpdateAvailable {
            current,
            latest,
            method,
            command,
        } => {
            println!("xfetch {} -> v{} available ({}).", current, latest, method);
            println!("  {}", command);
            return Err(XFetchError::Exit(crate::error::EXIT_GENERIC));
        }
        UpdateOutcome::Updated { from, to, path } => {
            runtime::info(format!(
                "Updated xfetch {} -> {} ({})",
                from,
                to,
                path.display()
            ));
        }
        UpdateOutcome::Manual { message } => println!("{}", message),
    }
    Ok(())
}

#[cfg(not(feature = "update"))]
pub fn run(
    _check: bool,
    _prebuilt: bool,
    _bin_dir: Option<String>,
    _yes: bool,
    _rollback: bool,
    _cli: &Cli,
) -> Result<(), XFetchError> {
    Err(XFetchError::Update(
        "This xfetch binary was built without the 'update' feature.".to_string(),
    ))
}
