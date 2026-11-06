use std::process::Command;

fn run(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_xfetch"))
        .args(args)
        .env("COLUMNS", "100")
        .env("NO_COLOR", "1")
        .output()
        .expect("run xfetch");
    assert!(
        output.status.success(),
        "command failed: {:?}\n{}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf-8 output")
        .replace("xfetch.exe", "xfetch")
}

fn snapshot(name: &str, actual: &str) {
    let expected = match name {
        "help_main" => include_str!("snapshots/help_main.txt"),
        "help_addon" => include_str!("snapshots/help_addon.txt"),
        "help_plugin" => include_str!("snapshots/help_plugin.txt"),
        "help_theme" => include_str!("snapshots/help_theme.txt"),
        "help_logo" => include_str!("snapshots/help_logo.txt"),
        "help_wasm" => include_str!("snapshots/help_wasm.txt"),
        "help_update" => include_str!("snapshots/help_update.txt"),
        "help_doctor" => include_str!("snapshots/help_doctor.txt"),
        "help_modules" => include_str!("snapshots/help_modules.txt"),
        "help_config" => include_str!("snapshots/help_config.txt"),
        "help_config_backup" => include_str!("snapshots/help_config_backup.txt"),
        "help_config_restore" => include_str!("snapshots/help_config_restore.txt"),
        "help_completions" => include_str!("snapshots/help_completions.txt"),
        "help_man" => include_str!("snapshots/help_man.txt"),
        "help_schema" => include_str!("snapshots/help_schema.txt"),
        "help_self_uninstall" => include_str!("snapshots/help_self_uninstall.txt"),
        "help_paths" => include_str!("snapshots/help_paths.txt"),
        "help_env" => include_str!("snapshots/help_env.txt"),
        "help_build_info" => include_str!("snapshots/help_build_info.txt"),
        "help_plugin_install" => include_str!("snapshots/help_plugin_install.txt"),
        "help_theme_install" => include_str!("snapshots/help_theme_install.txt"),
        "help_command" => include_str!("snapshots/help_command.txt"),
        "help_run" => include_str!("snapshots/help_run.txt"),
        "help_command_install" => include_str!("snapshots/help_command_install.txt"),
        "list_layouts" => include_str!("snapshots/list_layouts.txt"),
        "modules_json" => include_str!("snapshots/modules_json.txt"),
        "schema_fetch" => include_str!("snapshots/schema_fetch.txt"),
        _ => panic!("unknown snapshot {name}"),
    };
    assert_eq!(actual, expected, "snapshot '{name}' changed");
}

#[test]
fn help_outputs_match_snapshots() {
    snapshot("help_main", &run(&["--help"]));
    snapshot("help_addon", &run(&["addon", "--help"]));
    snapshot("help_plugin", &run(&["plugin", "--help"]));
    snapshot("help_theme", &run(&["theme", "--help"]));
    snapshot("help_logo", &run(&["logo", "--help"]));
    snapshot("help_wasm", &run(&["wasm", "--help"]));
    snapshot("help_update", &run(&["update", "--help"]));
    snapshot("help_doctor", &run(&["doctor", "--help"]));
    snapshot("help_modules", &run(&["modules", "--help"]));
    snapshot("help_config", &run(&["config", "--help"]));
    snapshot("help_config_backup", &run(&["config", "backup", "--help"]));
    snapshot(
        "help_config_restore",
        &run(&["config", "restore", "--help"]),
    );
    snapshot("help_completions", &run(&["completions", "--help"]));
    snapshot("help_man", &run(&["man", "--help"]));
    snapshot("help_schema", &run(&["schema", "--help"]));
    snapshot("help_self_uninstall", &run(&["self-uninstall", "--help"]));
    snapshot("help_paths", &run(&["paths", "--help"]));
    snapshot("help_env", &run(&["env", "--help"]));
    snapshot("help_build_info", &run(&["build-info", "--help"]));
    snapshot(
        "help_plugin_install",
        &run(&["plugin", "install", "--help"]),
    );
    snapshot("help_theme_install", &run(&["theme", "install", "--help"]));
    snapshot("help_command", &run(&["command", "--help"]));
    snapshot("help_run", &run(&["run", "--help"]));
    snapshot(
        "help_command_install",
        &run(&["command", "install", "--help"]),
    );
}

#[test]
fn deterministic_outputs_match_snapshots() {
    snapshot("list_layouts", &run(&["--list-layouts"]));
    snapshot("modules_json", &run(&["--json", "modules"]));
    snapshot("schema_fetch", &run(&["--json", "schema", "fetch"]));
}

#[test]
fn help_output_is_ascii_and_has_no_emojis() {
    let commands: &[&[&str]] = &[
        &["--help"],
        &["plugin", "--help"],
        &["plugin", "install", "--help"],
        &["theme", "--help"],
        &["theme", "install", "--help"],
        &["logo", "--help"],
        &["addon", "--help"],
        &["wasm", "--help"],
        &["update", "--help"],
        &["doctor", "--help"],
        &["modules", "--help"],
        &["config", "--help"],
        &["config", "backup", "--help"],
        &["config", "restore", "--help"],
        &["completions", "--help"],
        &["man", "--help"],
        &["schema", "--help"],
        &["paths", "--help"],
        &["env", "--help"],
        &["build-info", "--help"],
        &["self-uninstall", "--help"],
        &["command", "--help"],
        &["command", "install", "--help"],
        &["run", "--help"],
    ];
    for args in commands {
        let output = run(args);
        assert!(
            output.is_ascii(),
            "help output for {:?} contains non-ASCII characters",
            args
        );
    }
}
