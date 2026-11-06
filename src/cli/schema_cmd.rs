use crate::cli::args::SchemaKind;
use crate::cli::output;
use crate::error::XFetchError;
use serde_json::{Value, json};

pub fn run(kind: SchemaKind) -> Result<(), XFetchError> {
    let value = match kind {
        SchemaKind::Fetch => fetch_schema(),
        SchemaKind::Backup => backup_schema(),
        SchemaKind::Addon => addon_schema(),
        SchemaKind::Update => update_schema(),
        SchemaKind::Doctor => doctor_schema(),
        SchemaKind::Config => config_schema(),
        SchemaKind::All => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "xfetch CLI outputs",
            "type": "object",
            "properties": {
                "fetch": fetch_schema(),
                "backup": backup_schema(),
                "addon": addon_schema(),
                "update": update_schema(),
                "doctor": doctor_schema(),
                "config": config_schema(),
            },
        }),
    };
    output::print_json(&value)
}

fn base(title: &str) -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("https://xfetch-cli.github.io/web/schemas/{}.json", title.to_ascii_lowercase()),
        "title": format!("xfetch {}", title),
        "type": "object",
        "properties": {
            "schema_version": { "type": "integer", "const": 1 }
        },
        "required": ["schema_version"]
    })
}

fn fetch_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://xfetch-cli.github.io/web/schemas/fetch.json",
        "title": "xfetch fetch output",
        "type": "object",
        "properties": {
            "schema_version": { "type": "integer", "const": 1 },
            "name": { "type": "string", "const": "xfetch" },
            "version": { "type": "string" },
            "os": { "type": "string" },
            "kernel": { "type": "string" },
            "hostname": { "type": "string" },
            "user": { "type": "string" },
            "shell": { "type": "string" },
            "terminal": { "type": "string" },
            "desktop": { "type": "string" },
            "cpu": { "type": "string" },
            "gpu": { "type": "array", "items": { "type": "string" } },
            "memory": { "type": "string" },
            "swap": { "type": "string" },
            "disks": { "type": "array", "items": { "type": "string" } },
            "battery": { "type": "string" },
            "uptime": { "type": "string" },
            "packages": { "type": "string" },
            "packages_breakdown": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "manager": { "type": "string" },
                        "count": { "type": "string" }
                    },
                    "required": ["manager", "count"]
                }
            },
            "datetime": { "type": "string" },
            "local_ip": { "type": "string" },
            "local_ip_v6": { "type": "string" },
            "public_ip": { "type": "string" },
            "network_interfaces": { "type": "string" },
            "plugins": {
                "type": "object",
                "additionalProperties": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            },
            "benchmark": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": [
            "schema_version", "name", "version", "os", "kernel", "hostname",
            "user", "shell", "terminal", "desktop", "cpu", "gpu", "memory",
            "swap", "disks", "battery", "uptime", "packages",
            "packages_breakdown", "datetime", "local_ip", "local_ip_v6",
            "public_ip", "network_interfaces", "plugins"
        ]
    })
}

fn backup_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://xfetch-cli.github.io/web/schemas/backup.json",
        "title": "xfetch backup",
        "type": "object",
        "properties": {
            "schema_version": { "type": "integer", "const": 1 },
            "created_unix": { "type": "integer" },
            "created_unix_ms": { "type": "integer" },
            "created_utc": { "type": "string" },
            "xfetch_version": { "type": "string" },
            "config_path": { "type": "string" },
            "theme": { "type": ["string", "null"] },
            "theme_path": { "type": ["string", "null"] },
            "config_providers": { "type": "array", "items": { "type": "string" } },
            "files": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "role": { "type": "string" },
                        "path": { "type": "string" },
                        "name": { "type": ["string", "null"] },
                        "encoding": { "type": ["string", "null"], "enum": ["base64", null] },
                        "content": { "type": "string" }
                    },
                    "required": ["role", "path", "content"]
                }
            },
            "missing": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "role": { "type": "string" },
                        "path": { "type": "string" },
                        "reason": { "type": "string" }
                    },
                    "required": ["role", "path", "reason"]
                }
            },
            "warnings": { "type": "array", "items": { "type": "string" } },
            "resolved_config": { "type": "object" }
        },
        "required": [
            "schema_version", "created_unix", "created_utc", "xfetch_version",
            "config_path", "config_providers", "files", "missing", "warnings",
            "resolved_config"
        ]
    })
}

fn addon_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://xfetch-cli.github.io/web/schemas/addon.json",
        "title": "xfetch addon output",
        "type": "object",
        "properties": {
            "schema_version": { "type": "integer", "const": 1 },
            "kind": { "type": "string", "enum": ["plugin", "effect", "extension"] },
            "name": { "type": "string" },
            "path": { "type": "string" },
            "runtime": { "type": "string", "enum": ["native", "wasm"] },
            "manifest_kind": { "type": "string" },
            "version": { "type": ["string", "null"] },
            "description": { "type": ["string", "null"] },
            "enabled": { "type": "boolean" },
            "capabilities": { "type": "array", "items": { "type": "string" } },
            "limits": { "type": ["string", "null"] },
            "config": { "type": "array", "items": { "type": "string" } },
            "manifest_source": { "type": ["string", "null"] },
            "installed_source": { "type": ["string", "null"] },
            "addons": { "type": "array", "items": { "type": "object" } },
            "checks": { "type": "array", "items": { "type": "object" } }
        },
        "required": ["schema_version"]
    })
}

fn update_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://xfetch-cli.github.io/web/schemas/update.json",
        "title": "xfetch update check output",
        "type": "object",
        "properties": {
            "schema_version": { "type": "integer", "const": 1 },
            "status": {
                "type": "string",
                "enum": ["up-to-date", "update-available", "updated", "manual", "rolled-back"]
            },
            "current": { "type": "string" },
            "latest": { "type": "string" },
            "method": { "type": "string" },
            "command": { "type": "string" },
            "path": { "type": "string" },
            "message": { "type": "string" },
            "update_available": { "type": "boolean" }
        },
        "required": ["schema_version", "status"]
    })
}

fn doctor_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://xfetch-cli.github.io/web/schemas/doctor.json",
        "title": "xfetch doctor output",
        "type": "object",
        "properties": {
            "schema_version": { "type": "integer", "const": 1 },
            "version": { "type": "string" },
            "platform": {
                "type": "object",
                "properties": {
                    "os": { "type": "string" },
                    "arch": { "type": "string" },
                    "family": { "type": "string" },
                    "details": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "value": { "type": "string" }
                            },
                            "required": ["name", "value"]
                        }
                    }
                },
                "required": ["os", "arch", "family", "details"]
            },
            "checks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "category": { "type": "string" },
                        "name": { "type": "string" },
                        "status": { "type": "string", "enum": ["ok", "warn", "fail"] },
                        "detail": { "type": "string" }
                    },
                    "required": ["category", "name", "status", "detail"]
                }
            },
            "fixes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string" },
                        "detail": { "type": "string" }
                    },
                    "required": ["action", "detail"]
                }
            }
        },
        "required": ["schema_version", "version", "platform", "checks", "fixes"]
    })
}

fn config_schema() -> Value {
    let _ = base("config");
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://xfetch-cli.github.io/web/schemas/config.json",
        "title": "xfetch resolved config",
        "type": "object",
        "additionalProperties": true
    })
}
