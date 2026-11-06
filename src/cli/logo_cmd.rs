use crate::cli::args::LogoCommands;
use crate::cli::output;
use crate::error::XFetchError;
use crate::logos;

pub fn run(action: LogoCommands, json: bool) -> Result<(), XFetchError> {
    match action {
        LogoCommands::List { query } => list(query.as_deref(), json),
        LogoCommands::Preview { name } => preview(&name, json),
    }
}

fn list(query: Option<&str>, json: bool) -> Result<(), XFetchError> {
    let index = logos::fetch_index().map_err(XFetchError::Network)?;
    let query = query.map(|query| query.to_ascii_lowercase());
    let entries: Vec<&logos::LogoEntry> = index
        .logos
        .iter()
        .filter(|entry| {
            let Some(query) = &query else {
                return true;
            };
            entry.id.to_ascii_lowercase().contains(query)
                || entry
                    .aliases
                    .iter()
                    .any(|alias| alias.to_ascii_lowercase().contains(query))
                || entry.file.to_ascii_lowercase().contains(query)
        })
        .collect();

    if json {
        let entries: Vec<serde_json::Value> = entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "id": entry.id,
                    "file": entry.file,
                    "aliases": entry.aliases,
                })
            })
            .collect();
        return output::print_json(&serde_json::json!({
            "schema_version": output::JSON_SCHEMA_VERSION,
            "logos": entries,
        }));
    }

    println!("{:<24} {:<40} ALIASES", "ID", "FILE");
    for entry in entries {
        println!(
            "{:<24} {:<40} {}",
            entry.id,
            entry.file,
            entry.aliases.join(", ")
        );
    }
    Ok(())
}

fn preview(name: &str, json: bool) -> Result<(), XFetchError> {
    let index = logos::fetch_index().map_err(XFetchError::Network)?;
    let entry = logos::find_entry(&index, name).ok_or_else(|| {
        XFetchError::NotFound(format!("logo '{}' was not found in the catalog", name))
    })?;
    let art = logos::fetch_art(entry).map_err(XFetchError::Network)?;

    if json {
        return output::print_json(&serde_json::json!({
            "schema_version": output::JSON_SCHEMA_VERSION,
            "id": entry.id,
            "file": entry.file,
            "aliases": entry.aliases,
            "art": art,
        }));
    }

    print!("{art}");
    if !art.ends_with('\n') {
        println!();
    }
    Ok(())
}
