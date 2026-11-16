use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "xfetch-cli-test-{}-{}-{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn command(dir: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xfetch"))
        .args(args)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("HOME", dir.join("home"))
        .env("TERM", "xterm-256color")
        .env_remove("KITTY_WINDOW_ID")
        .output()
        .expect("run xfetch")
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("valid JSON stdout")
}

#[test]
fn version_and_lists() {
    let dir = temp_dir("lists");
    let version = command(&dir, &["--version"]);
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains(env!("CARGO_PKG_VERSION")));

    let modules = command(&dir, &["--list-modules"]);
    assert!(modules.status.success());
    let text = String::from_utf8_lossy(&modules.stdout);
    assert!(text.contains("os"));
    assert!(text.contains("packages:<manager>"));

    let layouts = command(&dir, &["--list-layouts"]);
    assert!(layouts.status.success());
    assert!(String::from_utf8_lossy(&layouts.stdout).contains("right"));
}

#[test]
fn json_and_get_outputs() {
    let dir = temp_dir("get");
    let json = command(&dir, &["--json", "--get", "os,kernel"]);
    assert!(json.status.success());
    let value = stdout_json(&json);
    assert_eq!(value["schema_version"], 1);
    assert!(value["values"]["os"].is_string());
    assert!(value["values"]["kernel"].is_string());

    let plain = command(&dir, &["--get", "os"]);
    assert!(plain.status.success());
    let text = String::from_utf8_lossy(&plain.stdout);
    assert!(!text.trim().is_empty());
    assert!(!text.contains('='));

    let modules = command(&dir, &["modules", "--json"]);
    assert!(modules.status.success());
    let value = stdout_json(&modules);
    assert!(
        value
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["key"] == "os") })
    );
}

#[test]
fn template_and_format_outputs() {
    let dir = temp_dir("template");
    let format = command(&dir, &["--format", "os={{os}} missing={{nope}}"]);
    assert!(format.status.success());
    let text = String::from_utf8_lossy(&format.stdout);
    assert!(text.starts_with("os="));
    assert!(text.contains("missing="));

    let template_path = dir.join("template.txt");
    fs::write(&template_path, "kernel={{kernel}}\n").unwrap();
    let template = command(&dir, &["--template", template_path.to_str().unwrap()]);
    assert!(template.status.success());
    assert!(String::from_utf8_lossy(&template.stdout).starts_with("kernel="));
}

#[test]
fn config_roundtrip_preserves_comments() {
    let dir = temp_dir("config");
    let path = dir.join("config.jsonc");
    fs::write(
        &path,
        "{\n  // keep me\n  \"layout\": \"section\",\n  \"modules\": [\"os\"]\n}\n",
    )
    .unwrap();
    let config_arg = format!("--config={}", path.display());

    let validate = command(&dir, &[&config_arg, "config", "validate"]);
    assert!(validate.status.success());

    let get = command(&dir, &[&config_arg, "config", "get", "layout"]);
    assert!(get.status.success());
    assert_eq!(String::from_utf8_lossy(&get.stdout).trim(), "section");

    let set = command(&dir, &[&config_arg, "config", "set", "layout", "right"]);
    assert!(set.status.success());
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("// keep me"));
    assert!(content.contains("\"layout\": \"right\""));

    let raw = command(&dir, &[&config_arg, "config", "print", "--raw"]);
    assert!(raw.status.success());
    assert!(String::from_utf8_lossy(&raw.stdout).contains("// keep me"));
}

#[test]
fn config_errors_have_stable_exit_codes() {
    let dir = temp_dir("config-errors");
    let path = dir.join("bad.jsonc");
    fs::write(&path, "{ \"layout\": ").unwrap();
    let config_arg = format!("--config={}", path.display());

    let validate = command(&dir, &[&config_arg, "config", "validate"]);
    assert_eq!(validate.status.code(), Some(3));

    let get = command(&dir, &[&config_arg, "config", "get", "missing"]);
    assert_eq!(get.status.code(), Some(7));
}

#[test]
fn doctor_json_is_machine_readable() {
    let dir = temp_dir("doctor");
    let output = command(&dir, &["doctor", "--json"]);
    assert!(output.status.success());
    let value = stdout_json(&output);
    assert_eq!(value["schema_version"], 1);
    assert!(
        value["checks"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(value["platform"]["os"].is_string());
}

#[test]
fn completions_and_man() {
    let dir = temp_dir("docs");
    let completions = command(&dir, &["completions", "bash"]);
    assert!(completions.status.success());
    assert!(String::from_utf8_lossy(&completions.stdout).contains("_xfetch"));

    let man = command(&dir, &["man"]);
    assert!(man.status.success());
    assert!(String::from_utf8_lossy(&man.stdout).contains(".TH xfetch"));
}

#[test]
fn addon_json_and_unified_command() {
    let dir = temp_dir("addons");
    let list = command(&dir, &["--json", "addon", "plugin", "list"]);
    assert!(list.status.success());
    let value = stdout_json(&list);
    assert_eq!(value["schema_version"], 1);
    assert!(value["addons"].is_array());

    let legacy = command(&dir, &["--json", "plugin", "list"]);
    assert!(legacy.status.success());
    assert!(String::from_utf8_lossy(&legacy.stdout).contains("\"addons\""));
}

#[test]
fn offline_blocks_network_and_exit_codes() {
    let dir = temp_dir("offline");
    let search = command(&dir, &["--offline", "plugin", "search", "logo"]);
    assert_eq!(search.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&search.stderr).contains("offline"));

    let update = command(&dir, &["--offline", "update", "--check"]);
    assert_eq!(update.status.code(), Some(6));
    assert!(String::from_utf8_lossy(&update.stderr).contains("offline"));
}

#[test]
fn color_flags_control_ansi_output() {
    let dir = temp_dir("color");
    let always = command(&dir, &["--color", "always", "--modules", "os,kernel"]);
    assert!(always.status.success());
    assert!(always.stdout.contains(&0x1b));

    let never = command(&dir, &["--color", "never", "--modules", "os,kernel"]);
    assert!(never.status.success());
    assert!(!never.stdout.contains(&0x1b));
}

#[test]
fn json_config_set_and_theme_list() {
    let dir = temp_dir("json-config");
    let path = dir.join("config.jsonc");
    fs::write(&path, "{\n}\n").unwrap();
    let config_arg = format!("--config={}", path.display());

    let set = command(
        &dir,
        &[&config_arg, "--json", "config", "set", "layout", "right"],
    );
    assert!(set.status.success());
    let value = stdout_json(&set);
    assert_eq!(value["key"], "layout");
    assert_eq!(value["value"], "right");

    let themes = command(&dir, &["--json", "theme", "list"]);
    assert!(themes.status.success());
    let value = stdout_json(&themes);
    assert_eq!(value["schema_version"], 1);
    assert!(value["themes"].is_array());
}

#[test]
fn get_version_and_conflicting_output_modes() {
    let dir = temp_dir("conflicts");
    let version = command(&dir, &["--get", "version"]);
    assert!(version.status.success());
    assert!(!String::from_utf8_lossy(&version.stdout).trim().is_empty());

    let json_format = command(&dir, &["--json", "--format", "{{os}}"]);
    assert_eq!(json_format.status.code(), Some(2));

    let get_format = command(&dir, &["--get", "os", "--format", "{{os}}"]);
    assert_eq!(get_format.status.code(), Some(2));
}

#[test]
fn quiet_suppresses_informational_output() {
    let dir = temp_dir("quiet");
    let path = dir.join("config.jsonc");
    fs::write(&path, "{\n  \"layout\": \"section\"\n}\n").unwrap();
    let config_arg = format!("--config={}", path.display());
    let output = command(
        &dir,
        &["--quiet", &config_arg, "config", "set", "layout", "right"],
    );
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn backup_and_restore_simple_config() {
    let dir = temp_dir("backup-simple");
    let path = dir.join("config.jsonc");
    fs::write(
        &path,
        "{\n  // keep me\n  \"layout\": \"section\",\n  \"modules\": [\"os\"]\n}\n",
    )
    .unwrap();
    let config_arg = format!("--config={}", path.display());
    let backup_dir = dir.join("backups");
    fs::create_dir_all(&backup_dir).unwrap();

    let backup = command(
        &dir,
        &[
            &config_arg,
            "config",
            "backup",
            "--output",
            backup_dir.to_str().unwrap(),
        ],
    );
    assert!(backup.status.success());
    let backup_file = fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .expect("backup file");

    fs::write(&path, "{\n  \"layout\": \"minimal\"\n}\n").unwrap();
    let restore = command(
        &dir,
        &[
            &config_arg,
            "config",
            "restore",
            "--from",
            backup_file.to_str().unwrap(),
        ],
    );
    assert!(restore.status.success());
    let restored = fs::read_to_string(&path).unwrap();
    assert!(restored.contains("// keep me"));
    assert!(restored.contains("\"layout\": \"section\""));
}

#[test]
fn backup_and_restore_config_roulette() {
    let dir = temp_dir("backup-roulette");
    let config_dir = dir.join("config").join("xfetch");
    fs::create_dir_all(config_dir.join("themes")).unwrap();
    fs::create_dir_all(config_dir.join("fetchs").join("a")).unwrap();
    fs::create_dir_all(config_dir.join("fetchs").join("b")).unwrap();

    let theme = config_dir.join("themes").join("nord.jsonc");
    fs::write(&theme, "{ \"layout\": \"compact\" }\n").unwrap();
    let route_a = config_dir.join("fetchs").join("a").join("config.jsonc");
    fs::write(&route_a, "{ \"theme\": \"nord\", \"modules\": [\"os\"] }\n").unwrap();
    let route_b = config_dir.join("fetchs").join("b").join("config.jsonc");
    fs::write(
        &route_b,
        "{ \"layout\": \"tree\", \"modules\": [\"kernel\"] }\n",
    )
    .unwrap();
    let routes = config_dir.join("routes.json");
    let routes_json = serde_json::json!([
        {"_name": "a", "path": route_a.display().to_string()},
        {"_name": "b", "path": route_b.display().to_string()},
    ]);
    fs::write(&routes, serde_json::to_string(&routes_json).unwrap()).unwrap();
    let path = config_dir.join("config.jsonc");
    let config_json = serde_json::json!({
        "config_providers": [{
            "extension": "config-roulette",
            "args": {
                "routes": routes.display().to_string(),
                "strategy": "daily"
            }
        }]
    });
    fs::write(&path, serde_json::to_string(&config_json).unwrap()).unwrap();
    let config_arg = format!("--config={}", path.display());
    let backup_file = dir.join("roulette.json");
    let backup = command(
        &dir,
        &[
            &config_arg,
            "config",
            "backup",
            "--output",
            backup_file.to_str().unwrap(),
        ],
    );
    assert!(backup.status.success());

    let bundle: Value = serde_json::from_slice(&fs::read(&backup_file).unwrap()).unwrap();
    let roles: Vec<&str> = bundle["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|file| file["role"].as_str())
        .collect();
    assert!(roles.contains(&"config"));
    assert!(roles.contains(&"routes"));
    assert_eq!(
        roles.iter().filter(|role| **role == "route-config").count(),
        2
    );
    assert!(roles.contains(&"theme"));

    for path in [&path, &routes, &route_a, &route_b, &theme] {
        fs::remove_file(path).unwrap();
    }
    let restore = command(
        &dir,
        &[
            &config_arg,
            "config",
            "restore",
            "--from",
            backup_file.to_str().unwrap(),
        ],
    );
    assert!(restore.status.success());
    assert!(path.is_file());
    assert!(routes.is_file());
    assert!(route_a.is_file());
    assert!(route_b.is_file());
    assert!(theme.is_file());
}

#[test]
fn backup_and_restore_global_flags_and_json() {
    let dir = temp_dir("backup-global");
    let config_dir = dir.join("config").join("xfetch");
    fs::create_dir_all(&config_dir).unwrap();
    let path = config_dir.join("config.jsonc");
    fs::write(&path, "{\n  \"layout\": \"section\"\n}\n").unwrap();

    let backup = command(&dir, &["--json", "--backup"]);
    assert!(backup.status.success());
    let value = stdout_json(&backup);
    assert_eq!(value["schema_version"], 1);
    assert!(
        value["path"]
            .as_str()
            .is_some_and(|path| path.ends_with(".json"))
    );

    fs::write(&path, "{\n  \"layout\": \"minimal\"\n}\n").unwrap();
    let restore = command(&dir, &["--json", "--restore"]);
    assert!(restore.status.success());
    let value = stdout_json(&restore);
    assert_eq!(value["dry_run"], false);
    assert!(
        value["restored"]
            .as_array()
            .is_some_and(|files| !files.is_empty())
    );
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("\"layout\": \"section\"")
    );
}

#[test]
fn backup_conflicts_and_errors() {
    let dir = temp_dir("backup-errors");
    let conflict = command(&dir, &["--backup", "--restore"]);
    assert_eq!(conflict.status.code(), Some(2));

    let with_subcommand = command(&dir, &["--backup", "modules"]);
    assert_eq!(with_subcommand.status.code(), Some(2));

    let missing = command(&dir, &["--backup"]);
    assert_eq!(missing.status.code(), Some(3));

    let restore = command(&dir, &["--restore"]);
    assert_eq!(restore.status.code(), Some(7));
}

#[test]
fn json_addon_rollback_and_remove() {
    let dir = temp_dir("addon-json");
    let plugins = dir.join("config").join("xfetch").join("plugins");
    fs::create_dir_all(&plugins).unwrap();
    let ext = if cfg!(windows) { ".exe" } else { "" };
    let artifact = plugins.join(format!("xfetch-plugin-test{ext}"));
    let backup = plugins.join(format!("xfetch-plugin-test{ext}.bak"));
    fs::write(&artifact, b"new").unwrap();
    fs::write(&backup, b"old").unwrap();

    let rollback = command(&dir, &["--json", "addon", "plugin", "rollback", "test"]);
    assert!(rollback.status.success());
    let value = stdout_json(&rollback);
    assert_eq!(value["status"], "rolled-back");
    assert_eq!(fs::read(&artifact).unwrap(), b"old");

    let remove = command(&dir, &["--json", "addon", "plugin", "remove", "test"]);
    assert!(remove.status.success());
    let value = stdout_json(&remove);
    assert_eq!(value["status"], "removed");
    assert!(!artifact.exists());
}

#[test]
fn config_unset_and_array_set() {
    let dir = temp_dir("config-edit");
    let path = dir.join("config.jsonc");
    fs::write(
        &path,
        "{\n  // keep\n  \"layout\": \"section\",\n  \"modules\": [\"os\", \"cpu\"],\n  \"colors\": { \"cpu\": \"red\", \"os\": \"cyan\" }\n}\n",
    )
    .unwrap();
    let config_arg = format!("--config={}", path.display());

    let set = command(&dir, &[&config_arg, "config", "set", "modules.1", "kernel"]);
    assert!(set.status.success());
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("// keep"));
    assert!(content.contains("\"kernel\""));

    let unset = command(&dir, &[&config_arg, "config", "unset", "colors.cpu"]);
    assert!(unset.status.success());
    let content = fs::read_to_string(&path).unwrap();
    assert!(!content.contains("\"cpu\": \"red\""));
    assert!(content.contains("\"os\": \"cyan\""));

    let unset_top = command(&dir, &[&config_arg, "config", "unset", "layout"]);
    assert!(unset_top.status.success());
    let content = fs::read_to_string(&path).unwrap();
    assert!(!content.contains("\"layout\""));
}

#[test]
fn config_reset_backups_and_prune() {
    let dir = temp_dir("config-reset");
    let config_dir = dir.join("config").join("xfetch");
    fs::create_dir_all(&config_dir).unwrap();
    let path = config_dir.join("config.jsonc");
    fs::write(&path, "{\n  \"layout\": \"section\"\n}\n").unwrap();

    let reset = command(&dir, &["config", "reset", "--yes"]);
    assert!(reset.status.success());
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("// xfetch configuration"));

    let backups = command(&dir, &["--json", "config", "backups"]);
    assert!(backups.status.success());
    let value = stdout_json(&backups);
    assert!(
        value["backups"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let backup = command(&dir, &["--json", "--backup"]);
    assert!(backup.status.success());
    let prune = command(&dir, &["--json", "config", "prune", "--keep", "0"]);
    assert!(prune.status.success());
    let value = stdout_json(&prune);
    assert!(
        value["deleted"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
}

#[test]
fn backup_compress_and_root_remap() {
    let dir = temp_dir("backup-compress");
    let original = dir.join("original").join("xfetch");
    fs::create_dir_all(original.join("themes")).unwrap();
    let config = original.join("config.jsonc");
    let theme = original.join("themes").join("nord.jsonc");
    let config_json = serde_json::json!({
        "theme": theme.display().to_string()
    });
    fs::write(&config, serde_json::to_string(&config_json).unwrap()).unwrap();
    fs::write(&theme, "{ \"layout\": \"compact\" }\n").unwrap();
    let config_arg = format!("--config={}", config.display());
    let backup_dir = dir.join("backups");
    fs::create_dir_all(&backup_dir).unwrap();
    let backup = command(
        &dir,
        &[
            &config_arg,
            "config",
            "backup",
            "--output",
            backup_dir.to_str().unwrap(),
            "--compress",
        ],
    );
    assert!(backup.status.success());
    let backup_file = fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.to_string_lossy().ends_with(".json.gz"))
        .expect("compressed backup");

    fs::remove_dir_all(&original).unwrap();
    let restored_root = dir.join("restored");
    let restore = command(
        &dir,
        &[
            "config",
            "restore",
            "--from",
            backup_file.to_str().unwrap(),
            "--root",
            restored_root.to_str().unwrap(),
        ],
    );
    assert!(restore.status.success());
    assert!(restored_root.join("config.jsonc").is_file());
    assert!(restored_root.join("themes").join("nord.jsonc").is_file());
}

#[test]
fn template_conditionals_run_from_the_cli() {
    let dir = temp_dir("template-blocks");
    let output = command(
        &dir,
        &[
            "--format",
            "{{#if kernel}}kernel-ok{{else}}no-kernel{{/if}} {{#unless nope}}unless-ok{{/unless}}",
        ],
    );
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("kernel-ok"));
    assert!(text.contains("unless-ok"));
}

#[test]
fn doctor_fix_generates_a_missing_config() {
    let dir = temp_dir("doctor-fix");
    let output = command(&dir, &["--json", "doctor", "--fix"]);
    assert!(output.status.success());
    let value = stdout_json(&output);
    assert!(
        value["fixes"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(
        dir.join("config")
            .join("xfetch")
            .join("config.jsonc")
            .is_file()
    );
}

#[test]
fn schema_and_update_rollback() {
    let dir = temp_dir("schema-rollback");
    let schema = command(&dir, &["--json", "schema", "fetch"]);
    assert!(schema.status.success());
    let value = stdout_json(&schema);
    assert_eq!(value["title"], "xfetch fetch output");

    let bin_dir = dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let binary = if cfg!(windows) {
        "xfetch.exe"
    } else {
        "xfetch"
    };
    fs::write(bin_dir.join(binary), b"new").unwrap();
    fs::write(bin_dir.join(format!("{binary}.bak")), b"old").unwrap();
    let rollback = command(
        &dir,
        &[
            "--json",
            "update",
            "--rollback",
            "--bin-dir",
            bin_dir.to_str().unwrap(),
        ],
    );
    assert!(rollback.status.success());
    let value = stdout_json(&rollback);
    assert_eq!(value["status"], "rolled-back");
    assert_eq!(fs::read(bin_dir.join(binary)).unwrap(), b"old");
}

#[cfg(unix)]
#[test]
fn self_uninstall_removes_a_copied_binary() {
    let dir = temp_dir("self-uninstall");
    let bin_dir = dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let binary = bin_dir.join("xfetch");
    fs::copy(env!("CARGO_BIN_EXE_xfetch"), &binary).unwrap();
    let output = Command::new(&binary)
        .args(["--json", "self-uninstall", "--purge", "--yes"])
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("HOME", dir.join("home"))
        .output()
        .unwrap();
    assert!(output.status.success());
    let value = serde_json::from_slice::<Value>(&output.stdout).unwrap();
    assert_eq!(value["status"], "uninstalled");
    assert!(!binary.exists());
}

#[test]
fn backup_encrypt_roundtrip() {
    let dir = temp_dir("backup-encrypt");
    let path = dir.join("config.jsonc");
    fs::write(&path, "{\n  \"layout\": \"section\"\n}\n").unwrap();
    let config_arg = format!("--config={}", path.display());
    let passphrase_file = dir.join("pass.txt");
    fs::write(&passphrase_file, "correct horse battery staple\n").unwrap();
    let backup_file = dir.join("backup.enc");

    let backup = command(
        &dir,
        &[
            &config_arg,
            "config",
            "backup",
            "--output",
            backup_file.to_str().unwrap(),
            "--encrypt",
            "--passphrase-file",
            passphrase_file.to_str().unwrap(),
        ],
    );
    assert!(backup.status.success());
    assert!(fs::read(&backup_file).unwrap().starts_with(b"XFBACKUP1"));

    fs::write(&path, "{\n  \"layout\": \"minimal\"\n}\n").unwrap();
    let restore = command(
        &dir,
        &[
            &config_arg,
            "config",
            "restore",
            "--from",
            backup_file.to_str().unwrap(),
            "--passphrase-file",
            passphrase_file.to_str().unwrap(),
        ],
    );
    assert!(restore.status.success());
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("\"layout\": \"section\"")
    );
}

#[cfg(unix)]
#[test]
fn config_edit_runs_an_editor_and_validates() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("config-edit-editor");
    let path = dir.join("config.jsonc");
    fs::write(&path, "{\n  \"layout\": \"section\"\n}\n").unwrap();
    let config_arg = format!("--config={}", path.display());
    let editor = dir.join("editor.sh");
    fs::write(
        &editor,
        "#!/bin/sh\ncat > \"$1\" <<'EOF'\n{ \"layout\": \"minimal\" }\nEOF\n",
    )
    .unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o755)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_xfetch"))
        .args([&config_arg, "config", "edit"])
        .env("VISUAL", &editor)
        .env("EDITOR", &editor)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("HOME", dir.join("home"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "config edit failed: status={:?}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("\"layout\": \"minimal\"")
    );
}

#[test]
fn diagnostics_paths_env_and_build_info() {
    let dir = temp_dir("diagnostics");
    let paths = command(&dir, &["--json", "paths"]);
    assert!(paths.status.success());
    let value = stdout_json(&paths);
    assert!(value["config"].is_string());
    assert!(value["config_dir"].is_string());
    assert!(value["backups"].is_string());

    let env = command(&dir, &["--json", "env"]);
    assert!(env.status.success());
    let value = stdout_json(&env);
    assert!(value["build"]["version"].is_string());
    assert!(value["platform"]["os"].is_string());
    assert!(value["settings"]["log_level"].is_string());

    let build = command(&dir, &["--json", "build-info"]);
    assert!(build.status.success());
    let value = stdout_json(&build);
    assert_eq!(value["features"]["encryption"], true);
    assert!(value["target"]["arch"].is_string());
}

#[test]
fn doctor_bench_and_image_flags() {
    let dir = temp_dir("doctor-bench");
    let bench = command(&dir, &["--json", "doctor", "--bench"]);
    assert!(bench.status.success());
    let value = stdout_json(&bench);
    assert!(value["benchmark"].as_array().is_some());

    let image = command(&dir, &["doctor", "--image"]);
    assert!(image.status.success());
}

#[test]
fn structured_logs_and_levels() {
    let dir = temp_dir("logs");
    let output = command(
        &dir,
        &[
            "--log-json",
            "--log-level",
            "debug",
            "--verbose",
            "--get",
            "os",
        ],
    );
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = stderr
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert!(!lines.is_empty(), "expected debug logs");
    for line in lines {
        let value: Value = serde_json::from_str(line).expect("JSON log line");
        assert_eq!(value["level"], "debug");
        assert_eq!(value["target"], "xfetch");
    }

    let silent = command(&dir, &["--log-level", "off", "--verbose", "--get", "os"]);
    assert!(silent.status.success());
    assert!(silent.stderr.is_empty());
}

#[test]
fn help_is_available_for_command_groups() {
    let dir = temp_dir("help-groups");
    for args in [
        vec!["help", "plugin", "install"],
        vec!["help", "theme", "install"],
        vec!["help", "config", "backup"],
        vec!["doctor", "--help"],
    ] {
        let output = command(&dir, &args);
        assert!(output.status.success(), "help failed for {:?}", args);
        assert!(!output.stdout.is_empty(), "empty help for {:?}", args);
    }
}

#[cfg(unix)]
#[test]
fn command_addon_list_run_and_remove() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("command-addon");
    let commands = dir.join("config").join("xfetch").join("commands");
    fs::create_dir_all(&commands).unwrap();
    let script = commands.join("xfetch-command-echo");
    fs::write(
        &script,
        "#!/bin/sh\nprintf 'repo: %s\\nauthor: test\\n' \"$1\"\n",
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let list = command(&dir, &["--json", "command", "list"]);
    assert!(list.status.success());
    let value = stdout_json(&list);
    assert_eq!(value["kind"], "command");
    assert!(
        value["addons"]
            .as_array()
            .is_some_and(|entries| entries.iter().any(|entry| entry["name"] == "echo"))
    );

    let run = command(&dir, &["run", "echo", "--", "owner/repo"]);
    assert!(run.status.success());
    let text = String::from_utf8_lossy(&run.stdout);
    assert!(text.contains("repo: owner/repo"));
    assert!(text.contains("author: test"));

    let json = command(&dir, &["--json", "run", "echo", "--", "owner/repo"]);
    assert!(json.status.success());
    let value = stdout_json(&json);
    assert_eq!(value["command"], "echo");
    assert!(
        value["lines"]
            .as_array()
            .is_some_and(|lines| lines.iter().any(|line| line == "repo: owner/repo"))
    );

    let remove = command(&dir, &["command", "remove", "echo"]);
    assert!(remove.status.success());
    assert!(!script.exists());
}

#[cfg(unix)]
#[test]
fn run_does_not_touch_or_require_the_loaded_config() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("command-config-independence");
    let commands = dir.join("config").join("xfetch").join("commands");
    fs::create_dir_all(&commands).unwrap();
    let script = commands.join("xfetch-command-echo");
    fs::write(&script, "#!/bin/sh\nprintf 'repo: %s\\n' \"$1\"\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    // Deliberately invalid config: `run` without --render must not parse it.
    let config = dir.join("config.jsonc");
    let sentinel = b"{\n  \"layout\": \"this is not valid json\"\n";
    fs::write(&config, sentinel).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_xfetch"))
        .args([
            "--config",
            config.to_str().unwrap(),
            "run",
            "echo",
            "--",
            "owner/repo",
        ])
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("HOME", dir.join("home"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("repo: owner/repo"));
    assert_eq!(
        fs::read(&config).unwrap(),
        sentinel,
        "config must stay untouched"
    );
}
