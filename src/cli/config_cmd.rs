use crate::cli::args::{Cli, ConfigCommands};
use crate::cli::output::print_json;
use crate::config::{
    Config, ConfigOverrides, default_config_path, load_config_with_overrides,
    parse_jsonc_file_strict, parse_jsonc_str_strict, resolve_theme_path,
};
use crate::error::XFetchError;
use crate::runtime;
use serde_json::Value;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(
    action: ConfigCommands,
    cli: &Cli,
    overrides: &ConfigOverrides,
    config_path: Option<String>,
) -> Result<(), XFetchError> {
    let path = config_path
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(default_config_path);

    match action {
        ConfigCommands::Validate => validate(&path, cli, config_path.is_some()),
        ConfigCommands::Print { raw } => {
            print_config(&path, raw, cli, overrides, config_path.clone())
        }
        ConfigCommands::Get { key } => get_value(&path, &key, cli, overrides, config_path),
        ConfigCommands::Set { key, value } => set_value(&path, &key, &value, cli),
        ConfigCommands::Unset { key } => unset_value(&path, &key, cli),
        ConfigCommands::Reset { yes } => reset_config(&path, yes, cli),
        ConfigCommands::Edit => edit_config(&path, cli),
        ConfigCommands::Backups => crate::cli::backup_cmd::list_backups(cli.json),
        ConfigCommands::Prune { keep, dry_run } => {
            crate::cli::backup_cmd::prune(keep, dry_run, cli.json)
        }
        ConfigCommands::Backup {
            output,
            with_addons,
            compress,
            encrypt,
            passphrase_file,
        } => crate::cli::backup_cmd::backup(
            config_path,
            output,
            with_addons,
            compress,
            encrypt,
            passphrase_file,
            cli.json,
        ),
        ConfigCommands::Restore {
            from,
            dry_run,
            root,
            passphrase_file,
        } => crate::cli::backup_cmd::restore(
            config_path,
            from,
            dry_run,
            root,
            passphrase_file,
            cli.json,
        ),
    }
}

fn validate(path: &Path, cli: &Cli, explicit: bool) -> Result<(), XFetchError> {
    if !path.exists() {
        if explicit {
            return Err(XFetchError::Config(format!(
                "config file '{}' does not exist",
                path.display()
            )));
        }
        if cli.json {
            return print_json(&serde_json::json!({
                "schema_version": 1,
                "valid": true,
                "path": path.display().to_string(),
                "exists": false,
            }));
        }
        runtime::info(format!(
            "Config not found at {}; defaults will be used.",
            path.display()
        ));
        return Ok(());
    }

    let value = parse_jsonc_file_strict(path).map_err(XFetchError::Config)?;
    let config: Config = serde_json::from_value(value)
        .map_err(|err| XFetchError::Config(format!("invalid config schema: {err}")))?;

    if let Some(theme) = &config.theme
        && resolve_theme_path(theme).is_none()
    {
        return Err(XFetchError::Config(format!(
            "theme '{}' referenced by the config was not found",
            theme
        )));
    }

    if cli.json {
        return print_json(&serde_json::json!({
            "schema_version": 1,
            "valid": true,
            "path": path.display().to_string(),
            "exists": true,
        }));
    }
    println!("Config OK: {}", path.display());
    Ok(())
}

fn print_config(
    path: &Path,
    raw: bool,
    cli: &Cli,
    overrides: &ConfigOverrides,
    config_path: Option<String>,
) -> Result<(), XFetchError> {
    if raw {
        let content = fs::read_to_string(path).map_err(|err| {
            XFetchError::Config(format!("cannot read '{}': {}", path.display(), err))
        })?;
        print!("{content}");
        if !content.ends_with('\n') {
            println!();
        }
        return Ok(());
    }

    let config = load_config_with_overrides(config_path, overrides);
    let value = serde_json::to_value(&config)
        .map_err(|err| XFetchError::Config(format!("failed to serialize config: {err}")))?;
    if cli.json {
        return print_json(&value);
    }
    let text = serde_json::to_string_pretty(&value)
        .map_err(|err| XFetchError::Config(format!("failed to serialize config: {err}")))?;
    println!("{text}");
    Ok(())
}

fn get_value(
    path: &Path,
    key: &str,
    cli: &Cli,
    overrides: &ConfigOverrides,
    config_path: Option<String>,
) -> Result<(), XFetchError> {
    let config = load_config_with_overrides(config_path, overrides);
    let value = serde_json::to_value(&config)
        .map_err(|err| XFetchError::Config(format!("failed to serialize config: {err}")))?;
    let found = lookup(&value, key).ok_or_else(|| {
        XFetchError::NotFound(format!(
            "config key '{}' was not found in {}",
            key,
            path.display()
        ))
    })?;

    if cli.json {
        return print_json(found);
    }
    match found {
        Value::String(text) => println!("{text}"),
        other => {
            let text = serde_json::to_string_pretty(other).unwrap_or_default();
            println!("{text}");
        }
    }
    Ok(())
}

fn set_value(path: &Path, key: &str, value: &str, cli: &Cli) -> Result<(), XFetchError> {
    let content = if path.exists() {
        fs::read_to_string(path).map_err(|err| {
            XFetchError::Config(format!("cannot read '{}': {}", path.display(), err))
        })?
    } else {
        "{\n}\n".to_string()
    };

    let parsed_value =
        serde_json::from_str::<Value>(value).unwrap_or_else(|_| Value::String(value.to_string()));
    let new_json = serde_json::to_string(&parsed_value)
        .map_err(|err| XFetchError::Config(format!("failed to serialize the value: {err}")))?;
    let segments = crate::cli::jsonc_edit::parse_path(key);

    let updated = if let Some(updated) =
        crate::cli::jsonc_edit::replace_value_preserving(&content, &segments, &new_json)
    {
        updated
    } else if !key.contains('.')
        && let Value::String(text) = &parsed_value
        && let Some(updated) = set_top_level_string(&content, key, text)
    {
        updated
    } else {
        let mut root = parse_jsonc_str_strict(&content).map_err(XFetchError::Config)?;
        set_path(&mut root, key, parsed_value)?;
        serde_json::to_string_pretty(&root)
            .map_err(|err| XFetchError::Config(format!("failed to serialize config: {err}")))?
    };

    let updated_value = parse_jsonc_str_strict(&updated).map_err(XFetchError::Config)?;
    serde_json::from_value::<Config>(updated_value).map_err(|err| {
        XFetchError::Config(format!(
            "the new value would make the config invalid: {err}"
        ))
    })?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            XFetchError::Config(format!("cannot create '{}': {}", parent.display(), err))
        })?;
    }
    crate::atomic_fs::write_atomic(path, updated.as_bytes()).map_err(|err| {
        XFetchError::Config(format!("cannot write '{}': {}", path.display(), err))
    })?;

    if cli.json {
        return print_json(&serde_json::json!({
            "schema_version": 1,
            "path": path.display().to_string(),
            "key": key,
            "value": value,
        }));
    }
    runtime::info(format!("Set {} = {}", key, value));
    Ok(())
}

fn unset_value(path: &Path, key: &str, cli: &Cli) -> Result<(), XFetchError> {
    let content = fs::read_to_string(path)
        .map_err(|err| XFetchError::Config(format!("cannot read '{}': {}", path.display(), err)))?;
    let segments = crate::cli::jsonc_edit::parse_path(key);

    let line_removed = remove_top_level_key(&content, key);
    let updated = if let Some(updated) = line_removed
        && parse_jsonc_str_strict(&updated).is_ok()
    {
        updated
    } else {
        let mut root = parse_jsonc_str_strict(&content).map_err(XFetchError::Config)?;
        remove_path(&mut root, &segments)?;
        serde_json::to_string_pretty(&root)
            .map_err(|err| XFetchError::Config(format!("failed to serialize config: {err}")))?
    };

    let updated_value = parse_jsonc_str_strict(&updated).map_err(XFetchError::Config)?;
    serde_json::from_value::<Config>(updated_value).map_err(|err| {
        XFetchError::Config(format!(
            "removing the value would make the config invalid: {err}"
        ))
    })?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            XFetchError::Config(format!("cannot create '{}': {}", parent.display(), err))
        })?;
    }
    crate::atomic_fs::write_atomic(path, updated.as_bytes()).map_err(|err| {
        XFetchError::Config(format!("cannot write '{}': {}", path.display(), err))
    })?;

    if cli.json {
        return print_json(&serde_json::json!({
            "schema_version": 1,
            "path": path.display().to_string(),
            "key": key,
            "status": "unset",
        }));
    }
    runtime::info(format!("Unset {key}"));
    Ok(())
}

fn remove_top_level_key(content: &str, key: &str) -> Option<String> {
    let double = format!("\"{}\"", key);
    let single = format!("'{}'", key);
    let bare = format!("{}:", key);
    let mut changed = false;
    let mut output = String::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        let is_key = trimmed.starts_with(&double)
            || trimmed.starts_with(&single)
            || trimmed.starts_with(&bare);
        if !changed && is_key {
            changed = true;
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    changed.then_some(output)
}

fn remove_path(
    root: &mut Value,
    segments: &[crate::cli::jsonc_edit::Segment],
) -> Result<(), XFetchError> {
    use crate::cli::jsonc_edit::Segment;
    if segments.is_empty() {
        return Err(XFetchError::Usage("empty config key".to_string()));
    }

    let mut current = root;
    for segment in &segments[..segments.len() - 1] {
        current = match (current, segment) {
            (Value::Object(map), Segment::Key(key)) => map.get_mut(key).ok_or_else(|| {
                XFetchError::NotFound(format!("config key '{}' was not found", key))
            })?,
            (Value::Array(items), Segment::Index(index)) => {
                items.get_mut(*index).ok_or_else(|| {
                    XFetchError::NotFound(format!("array index {index} was not found"))
                })?
            }
            _ => {
                return Err(XFetchError::NotFound(
                    "config path was not found".to_string(),
                ));
            }
        };
    }

    match (current, segments.last().expect("checked above")) {
        (Value::Object(map), Segment::Key(key)) => map
            .remove(key)
            .map(|_| ())
            .ok_or_else(|| XFetchError::NotFound(format!("config key '{}' was not found", key))),
        (Value::Array(items), Segment::Index(index)) => {
            if *index < items.len() {
                items.remove(*index);
                Ok(())
            } else {
                Err(XFetchError::NotFound(format!(
                    "array index {index} was not found"
                )))
            }
        }
        _ => Err(XFetchError::NotFound(
            "config path was not found".to_string(),
        )),
    }
}

fn reset_config(path: &Path, yes: bool, cli: &Cli) -> Result<(), XFetchError> {
    if path.exists() && !yes {
        if std::io::stdin().is_terminal() {
            eprint!(
                "Replace '{}' with the built-in defaults? [y/N]: ",
                path.display()
            );
            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .map_err(|err| XFetchError::Fatal(format!("cannot read confirmation: {err}")))?;
            if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                return Ok(());
            }
        } else {
            return Err(XFetchError::Usage(
                "refusing to reset the config without --yes in non-interactive mode".to_string(),
            ));
        }
    }

    let safety = crate::cli::backup_cmd::safety_backup(Some(path.display().to_string()))?;
    crate::config::write_default_config(path).map_err(XFetchError::Config)?;

    if cli.json {
        return print_json(&serde_json::json!({
            "schema_version": 1,
            "path": path.display().to_string(),
            "status": "reset",
            "safety_backup": safety.as_ref().map(|path| path.display().to_string()),
        }));
    }
    runtime::info(format!("Config reset to defaults: {}", path.display()));
    if let Some(safety) = safety {
        runtime::info(format!("Safety backup: {}", safety.display()));
    }
    Ok(())
}

fn edit_config(path: &Path, cli: &Cli) -> Result<(), XFetchError> {
    let safety = crate::cli::backup_cmd::safety_backup(Some(path.display().to_string()))?;
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| XFetchError::Usage("set $VISUAL or $EDITOR to edit the config".to_string()))?;
    let status = Command::new(&editor)
        .arg(path)
        .status()
        .map_err(|err| XFetchError::Fatal(format!("cannot run '{}': {err}", editor)))?;
    if !status.success() {
        return Err(XFetchError::Fatal(format!(
            "editor '{}' exited with status {}",
            editor, status
        )));
    }

    validate_config_file(path)?;
    if cli.json {
        return print_json(&serde_json::json!({
            "schema_version": 1,
            "path": path.display().to_string(),
            "status": "edited",
            "safety_backup": safety.as_ref().map(|path| path.display().to_string()),
        }));
    }
    runtime::info(format!("Config edited and validated: {}", path.display()));
    if let Some(safety) = safety {
        runtime::info(format!("Safety backup: {}", safety.display()));
    }
    Ok(())
}

fn validate_config_file(path: &Path) -> Result<(), XFetchError> {
    let content = fs::read_to_string(path)
        .map_err(|err| XFetchError::Config(format!("cannot read '{}': {}", path.display(), err)))?;
    let value = parse_jsonc_str_strict(&content).map_err(XFetchError::Config)?;
    serde_json::from_value::<Config>(value)
        .map(|_| ())
        .map_err(|err| XFetchError::Config(format!("invalid config schema: {err}")))
}

fn lookup<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in key.split('.') {
        current = match current {
            Value::Object(map) => map.get(part)?,
            Value::Array(items) => items.get(part.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

fn set_path(root: &mut Value, key: &str, value: Value) -> Result<(), XFetchError> {
    let segments = crate::cli::jsonc_edit::parse_path(key);
    if segments.is_empty() {
        return Err(XFetchError::Usage("empty config key".to_string()));
    }
    set_path_segments(root, &segments, value)
}

fn set_path_segments(
    current: &mut Value,
    segments: &[crate::cli::jsonc_edit::Segment],
    value: Value,
) -> Result<(), XFetchError> {
    use crate::cli::jsonc_edit::Segment;
    if segments.is_empty() {
        *current = value;
        return Ok(());
    }

    match &segments[0] {
        Segment::Key(name) => {
            if !current.is_object() {
                *current = Value::Object(serde_json::Map::new());
            }
            let object = current.as_object_mut().expect("object just ensured");
            let entry = object.entry(name.clone()).or_insert(Value::Null);
            set_path_segments(entry, &segments[1..], value)
        }
        Segment::Index(index) => {
            if !current.is_array() {
                *current = Value::Array(Vec::new());
            }
            let items = current.as_array_mut().expect("array just ensured");
            if *index > items.len() {
                return Err(XFetchError::Usage(format!(
                    "array index {index} is out of range (length {})",
                    items.len()
                )));
            }
            if *index == items.len() {
                items.push(Value::Null);
            }
            set_path_segments(&mut items[*index], &segments[1..], value)
        }
    }
}

/// Replaces a top-level quoted string without touching comments or the rest of
/// the document. Returns `None` when the key cannot be edited in place.
fn set_top_level_string(content: &str, key: &str, value: &str) -> Option<String> {
    let mut found = false;
    let mut result = String::new();
    let double = format!("\"{}\"", key);
    let single = format!("'{}'", key);
    let bare = format!("{}:", key);

    for line in content.lines() {
        let trimmed = line.trim_start();
        let is_key = trimmed.starts_with(&double)
            || trimmed.starts_with(&single)
            || trimmed.starts_with(&bare);
        if !found && is_key {
            if let Some(colon) = line.find(':') {
                let head = &line[colon + 1..];
                let trimmed_head = head.trim_start();
                let skip = head.len() - trimmed_head.len();
                if let Some(quote) = trimmed_head.chars().next()
                    && (quote == '"' || quote == '\'')
                {
                    let open = colon + 1 + skip;
                    if let Some(relative) = line[open + 1..].find(quote) {
                        let close = open + 1 + relative;
                        result.push_str(&line[..open]);
                        result.push('"');
                        result.push_str(value);
                        result.push('"');
                        result.push_str(&line[close + 1..]);
                        found = true;
                        result.push('\n');
                        continue;
                    }
                }
            }
            if !found {
                let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
                result.push_str(&indent);
                result.push('"');
                result.push_str(key);
                result.push_str("\": \"");
                result.push_str(value);
                result.push('"');
                found = true;
                result.push('\n');
                continue;
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    if found {
        return Some(result);
    }

    let index = content.find('{')?;
    let rest = &content[index + 1..];
    let rest_trimmed = rest.trim_start();
    let needs_comma = !(rest_trimmed.is_empty() || rest_trimmed.starts_with('}'));
    let mut output = String::with_capacity(content.len() + key.len() + value.len() + 8);
    output.push_str(&content[..=index]);
    output.push_str("\n    \"");
    output.push_str(key);
    output.push_str("\": \"");
    output.push_str(value);
    output.push('"');
    if needs_comma {
        output.push(',');
    }
    output.push_str(rest);
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_supports_dotted_paths_and_indices() {
        let value = serde_json::json!({"colors": {"cpu": "red"}, "modules": ["os", "cpu"]});
        assert_eq!(
            lookup(&value, "colors.cpu"),
            Some(&Value::String("red".to_string()))
        );
        assert_eq!(
            lookup(&value, "modules.1"),
            Some(&Value::String("cpu".to_string()))
        );
        assert_eq!(lookup(&value, "modules.9"), None);
        assert_eq!(lookup(&value, "missing"), None);
    }

    #[test]
    fn set_top_level_string_preserves_comments() {
        let content = "{\n  // keep me\n  \"layout\": \"section\",\n  \"theme\": \"nord\"\n}\n";
        let updated = set_top_level_string(content, "layout", "right").unwrap();
        assert!(updated.contains("// keep me"));
        assert!(updated.contains("\"layout\": \"right\""));
        assert!(updated.contains("\"theme\": \"nord\""));
    }

    #[test]
    fn set_top_level_string_inserts_a_missing_key() {
        let content = "{\n  \"theme\": \"nord\"\n}\n";
        let updated = set_top_level_string(content, "layout", "right").unwrap();
        assert!(updated.contains("\"layout\": \"right\""));
        assert!(updated.contains("\"theme\": \"nord\""));
    }

    #[test]
    fn set_top_level_string_handles_an_empty_object() {
        let updated = set_top_level_string("{\n}\n", "layout", "right").unwrap();
        let parsed = crate::config::parse_jsonc_str_strict(&updated).expect("valid jsonc");
        assert_eq!(parsed["layout"], "right");
    }
}
