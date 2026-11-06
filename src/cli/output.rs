use crate::addons::model::{AddonEntry, AddonKind};
use crate::config::Config;
use crate::error::XFetchError;
use crate::info::Info;
use crate::runtime;
use crate::ui::nodes::{format_palette, get_module_value};
use console::strip_ansi_codes;
use serde::Serialize;
use std::collections::BTreeMap;

pub const JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
pub struct PackageCount {
    pub manager: String,
    pub count: String,
}

#[derive(Debug, Serialize)]
pub struct FetchJson {
    pub schema_version: u32,
    pub name: &'static str,
    pub version: &'static str,
    pub os: String,
    pub kernel: String,
    pub hostname: String,
    pub user: String,
    pub shell: String,
    pub terminal: String,
    pub desktop: String,
    pub cpu: String,
    pub gpu: Vec<String>,
    pub memory: String,
    pub swap: String,
    pub disks: Vec<String>,
    pub battery: String,
    pub uptime: String,
    pub packages: String,
    pub packages_breakdown: Vec<PackageCount>,
    pub datetime: String,
    pub local_ip: String,
    pub local_ip_v6: String,
    pub public_ip: String,
    pub network_interfaces: String,
    pub plugins: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub benchmark: Vec<String>,
}

impl FetchJson {
    pub fn from_info(info: &Info, benchmark: Vec<String>) -> Self {
        let packages_breakdown = info
            .packages_breakdown
            .iter()
            .map(|(manager, count)| PackageCount {
                manager: manager.clone(),
                count: count.clone(),
            })
            .collect();
        let plugins: BTreeMap<String, Vec<String>> = info
            .plugin_info
            .iter()
            .map(|(key, lines)| {
                (
                    key.clone(),
                    lines
                        .iter()
                        .map(|line| strip_ansi_codes(line).to_string())
                        .collect(),
                )
            })
            .collect();

        Self {
            schema_version: JSON_SCHEMA_VERSION,
            name: "xfetch",
            version: env!("CARGO_PKG_VERSION"),
            os: info.os.clone(),
            kernel: info.kernel.clone(),
            hostname: info.host_name.clone(),
            user: info.user.clone(),
            shell: info.shell.clone(),
            terminal: info.terminal.clone(),
            desktop: info.desktop.clone(),
            cpu: info.cpu.clone(),
            gpu: info.gpu.clone(),
            memory: info.memory.clone(),
            swap: info.swap.clone(),
            disks: info.disks.clone(),
            battery: info.battery.clone(),
            uptime: info.uptime.clone(),
            packages: info.packages.clone(),
            packages_breakdown,
            datetime: info.datetime.clone(),
            local_ip: info.local_ip.clone(),
            local_ip_v6: info.local_ip_v6.clone(),
            public_ip: info.public_ip.clone(),
            network_interfaces: info.network_interfaces.clone(),
            plugins,
            benchmark,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GetJson {
    pub schema_version: u32,
    pub values: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct AddonJson {
    pub kind: String,
    pub name: String,
    pub path: String,
    pub runtime: String,
    pub manifest_kind: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub enabled: bool,
    pub capabilities: Vec<String>,
    pub limits: Option<String>,
    pub config: Vec<String>,
    pub manifest_source: Option<String>,
    pub installed_source: Option<String>,
}

impl AddonJson {
    pub fn from_entry(entry: &AddonEntry) -> Self {
        Self {
            kind: entry.kind.label().to_string(),
            name: entry.name.clone(),
            path: entry.path.display().to_string(),
            runtime: entry.runtime.as_str().to_string(),
            manifest_kind: entry.manifest_kind().to_string(),
            version: entry.version().map(str::to_string),
            description: entry.description().map(str::to_string),
            enabled: entry.enabled,
            capabilities: entry.capability_lines(),
            limits: entry.limits_line(),
            config: entry.config_snippet(),
            manifest_source: entry.manifest_source.clone(),
            installed_source: entry.receipt.as_ref().map(|receipt| receipt.source.clone()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AddonListJson {
    pub schema_version: u32,
    pub kind: String,
    pub addons: Vec<AddonJson>,
}

#[derive(Debug, Serialize)]
pub struct AddonInfoJson {
    pub schema_version: u32,
    #[serde(flatten)]
    pub addon: AddonJson,
}

pub fn print_json<T: Serialize>(value: &T) -> Result<(), XFetchError> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| XFetchError::Fatal(format!("failed to serialize JSON: {}", err)))?;
    println!("{text}");
    Ok(())
}

pub fn value_for_key(info: &Info, config: &Config, key: &str) -> Option<String> {
    let value: Option<String> = if key == "palette" {
        Some(format_palette(config))
    } else if key == "name" {
        Some("xfetch".to_string())
    } else if key == "version" {
        Some(env!("CARGO_PKG_VERSION").to_string())
    } else if key == "header" {
        Some(format!("{}@{}", info.user, info.host_name))
    } else if key == "sep" {
        Some("---".to_string())
    } else if let Some(lines) = info.plugin_info.get(key) {
        Some(lines.join("\n"))
    } else {
        get_module_value(info, key)
    };

    value.map(|text: String| {
        if runtime::no_color() {
            strip_ansi_codes(&text).to_string()
        } else {
            text
        }
    })
}

pub fn collect_values(
    info: &Info,
    config: &Config,
    keys: &[String],
) -> BTreeMap<String, Option<String>> {
    keys.iter()
        .map(|key| (key.clone(), value_for_key(info, config, key)))
        .collect()
}

pub fn print_values(
    values: &BTreeMap<String, Option<String>>,
    keys: &[String],
    json: bool,
) -> Result<(), XFetchError> {
    if json {
        let values = values
            .iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    value
                        .as_ref()
                        .map(|text| strip_ansi_codes(text).to_string()),
                )
            })
            .collect();
        return print_json(&GetJson {
            schema_version: JSON_SCHEMA_VERSION,
            values,
        });
    }

    if keys.len() == 1 {
        if let Some(Some(value)) = values.iter().next().map(|(_, value)| value) {
            println!("{value}");
        }
        return Ok(());
    }

    for key in keys {
        let value = values
            .get(key)
            .and_then(|value| value.as_deref())
            .unwrap_or("");
        let single_line = value.replace('\n', "\\n");
        println!("{key}={single_line}");
    }
    Ok(())
}

pub fn print_addon_list_json(kind: AddonKind, entries: &[AddonEntry]) -> Result<(), XFetchError> {
    print_json(&AddonListJson {
        schema_version: JSON_SCHEMA_VERSION,
        kind: kind.label().to_string(),
        addons: entries.iter().map(AddonJson::from_entry).collect(),
    })
}

pub fn print_addon_info_json(entry: &AddonEntry) -> Result<(), XFetchError> {
    print_json(&AddonInfoJson {
        schema_version: JSON_SCHEMA_VERSION,
        addon: AddonJson::from_entry(entry),
    })
}
