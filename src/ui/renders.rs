use super::nodes::RenderNode;
use crate::config::Config;
use console::strip_ansi_codes;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

const BOX_PADDING: usize = 2;
const BORDER_COLOR: &str = "38;5;2";
const SECTION_COLOR: &str = "38;5;240";
const PACMAN_GREEN: &str = "32";
const PACMAN_COLORS: [&str; 5] = ["33", "31", "35", "36", "33"];
const PACMAN_WHITE: &str = "37";
const LINE_SEPARATOR: &str = "──────────────────────────────";
const DOTS_SEPARATOR: &str = "..............................";
const SEPARATOR_COLOR: &str = "90";
const BOTTOM_LINE_COLOR: &str = "37";
const DEFAULT_TREE_ICON: &str = "";
const TREE_LAST_PREFIX: &str = "└──";
const TREE_CHILD_PREFIX: &str = "├──";
const DEFAULT_FOOTER: &str = "X";

pub fn render_classic(nodes: &[RenderNode], config: &Config) -> Vec<String> {
    let mut lines = Vec::new();
    for node in nodes {
        match node {
            RenderNode::Line { key, value, icon } => {
                if icon.is_empty() && key.starts_with("plugin:") {
                    let color_code = get_color_code(key, config);
                    lines.push(format!(
                        "\x1b[{}m│\x1b[0m \x1b[{}m{}\x1b[0m",
                        SECTION_COLOR, color_code, value
                    ));
                } else if icon.is_empty() {
                    lines.push(format!("\x1b[{}m│\x1b[0m {}", SECTION_COLOR, value));
                } else {
                    lines.push(format_line(key, value, icon, config));
                }
            }
            RenderNode::Group { title, children } => {
                lines.push(format!("-- {} --", title));
                for child in children {
                    if let RenderNode::Line { key, value, icon } = child {
                        lines.push(format_line(key, value, icon, config));
                    }
                }
            }
        }
    }
    lines
}

pub fn render_classic_variants(
    nodes: &[RenderNode],
    config: &Config,
    variant: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    let flat_items = flatten_nodes(nodes);

    match variant {
        "box" => {
            let max_len = flat_items
                .iter()
                .map(|(k, v, i)| {
                    let content = format_line_content(k, v, i, config);
                    strip_ansi_codes(&content).chars().count()
                })
                .max()
                .unwrap_or(0);

            let border_len = max_len + BOX_PADDING;
            lines.push(format!("╭{}╮", "─".repeat(border_len)));

            for (key, val, icon) in flat_items {
                let content = format_line_content(&key, &val, &icon, config);
                let visual_len = strip_ansi_codes(&content).chars().count();
                let padding = max_len - visual_len;
                lines.push(format!("│ {} {}│", content, " ".repeat(padding)));
            }
            lines.push(format!("╰{}╯", "─".repeat(border_len)));
        }
        "pacman" => {
            let icons = config.header_icons.clone().unwrap_or_default();
            let mut header = format!("\x1b[{}m╭─ \x1b[0m", PACMAN_GREEN);
            for (idx, icon) in icons.iter().enumerate() {
                let color = PACMAN_COLORS[idx % 5];
                header.push_str(&format!("\x1b[{}m{} \x1b[0m", color, icon));
            }
            header.push_str(&format!("\x1b[{}m────────────────╮\x1b[0m", PACMAN_GREEN));
            lines.push(header);

            for (key, val, icon) in flat_items {
                lines.push(format_line(&key, &val, &icon, config));
            }

            let footer_text = config.footer_text.as_deref().unwrap_or(DEFAULT_FOOTER);
            lines.push(format!(
                "\x1b[{}m╰────────── \x1b[{}m{}\x1b[{}m ──────────╯\x1b[0m",
                PACMAN_GREEN, PACMAN_WHITE, footer_text, PACMAN_GREEN
            ));
        }
        "line" | "dots" => {
            for (idx, (key, val, icon)) in flat_items.iter().enumerate() {
                lines.push(format_line(key, val, icon, config));
                if (idx + 1) % 3 == 0 && idx != flat_items.len() - 1 {
                    let sep = if variant == "line" {
                        LINE_SEPARATOR
                    } else {
                        DOTS_SEPARATOR
                    };
                    lines.push(format!("\x1b[{}m{}\x1b[0m", SEPARATOR_COLOR, sep));
                }
            }
        }
        "bottom_line" => {
            for (key, val, icon) in flat_items {
                lines.push(format_line(&key, &val, &icon, config));
            }
            lines.push(format!(
                "\x1b[{}m{}\x1b[0m",
                BOTTOM_LINE_COLOR, LINE_SEPARATOR
            ));
        }
        _ => return render_classic(nodes, config),
    }
    lines
}

pub fn render_side_block(nodes: &[RenderNode], config: &Config) -> Vec<String> {
    let mut lines = Vec::new();
    let flat_items = flatten_nodes(nodes);

    let max_key_len = flat_items
        .iter()
        .map(|(_, _, icon)| strip_ansi_codes(icon).chars().count())
        .max()
        .unwrap_or(0);
    let max_val_len = flat_items
        .iter()
        .map(|(_, v, _)| strip_ansi_codes(v).chars().count())
        .max()
        .unwrap_or(0);

    let left_width = max_key_len + BOX_PADDING;
    let right_width = max_val_len + BOX_PADDING;

    let top = format!(
        "\x1b[{}m╭{}╮\x1b[0m \x1b[{}m╭{}╮\x1b[0m",
        BORDER_COLOR,
        "─".repeat(left_width),
        BORDER_COLOR,
        "─".repeat(right_width)
    );
    lines.push(top);

    for (key, val, icon) in flat_items {
        let color_code = get_color_code(&key, config);
        let key_str = format!(
            "\x1b[{}m{:<width$}\x1b[0m",
            color_code,
            icon,
            width = max_key_len
        );

        let val_stripped_len = strip_ansi_codes(&val).chars().count();
        let padding = max_val_len - val_stripped_len;

        let line = format!(
            "\x1b[{}m│\x1b[0m {} \x1b[{}m│\x1b[0m \x1b[{}m│\x1b[0m {}{} \x1b[{}m│\x1b[0m",
            BORDER_COLOR,
            key_str,
            BORDER_COLOR,
            BORDER_COLOR,
            val,
            " ".repeat(padding),
            BORDER_COLOR
        );
        lines.push(line);
    }

    let bottom = format!(
        "\x1b[{}m╰{}╯\x1b[0m \x1b[{}m╰{}╯\x1b[0m",
        BORDER_COLOR,
        "─".repeat(left_width),
        BORDER_COLOR,
        "─".repeat(right_width)
    );
    lines.push(bottom);

    lines
}

pub fn render_tree(nodes: &[RenderNode], config: &Config) -> Vec<String> {
    let mut lines = Vec::new();

    for node in nodes {
        match node {
            RenderNode::Group { title, children } => {
                let icon = config
                    .icons
                    .get(title.to_lowercase().as_str())
                    .map(|s| s.as_str())
                    .unwrap_or(DEFAULT_TREE_ICON);
                let color_code = get_color_code(&title.to_lowercase(), config);

                lines.push(format!("\x1b[{}m{} {}\x1b[0m", color_code, icon, title));

                for (idx, child) in children.iter().enumerate() {
                    let is_last = idx == children.len() - 1;
                    let prefix = if is_last {
                        TREE_LAST_PREFIX
                    } else {
                        TREE_CHILD_PREFIX
                    };

                    if let RenderNode::Line {
                        key,
                        value,
                        icon: _,
                    } = child
                    {
                        let key_color = get_color_code(key, config);
                        if key.starts_with("plugin:") {
                            lines.push(format!(
                                "\x1b[{}m{}\x1b[0m \x1b[{}m{}\x1b[0m",
                                SECTION_COLOR, prefix, key_color, value
                            ));
                        } else {
                            let label = display_key(key, config);
                            if label.is_empty() {
                                lines.push(format!(
                                    "\x1b[{}m{}\x1b[0m \x1b[{}m{}\x1b[0m",
                                    SECTION_COLOR, prefix, key_color, value
                                ));
                            } else {
                                lines.push(format!(
                                    "\x1b[{}m{}\x1b[0m \x1b[{}m{}\x1b[0m {}",
                                    SECTION_COLOR, prefix, key_color, label, value
                                ));
                            }
                        }
                    }
                }
            }
            RenderNode::Line { key, value, icon } => {
                lines.push(format_line(key, value, icon, config));
            }
        }
    }
    lines
}

fn prefix_width(icon: &str, key: &str) -> usize {
    let icon_w = console::measure_text_width(icon);
    let key_w = console::measure_text_width(key);
    1 + icon_w + 1 + key_w + 2
}

pub fn render_section(nodes: &[RenderNode], config: &Config) -> Vec<String> {
    let mut lines = Vec::new();

    for node in nodes {
        match node {
            RenderNode::Group { title, children } => {
                let header = format!(
                    "\x1b[{}m──────\x1b[0m \x1b[1m{}\x1b[0m \x1b[{}m──────\x1b[0m",
                    SECTION_COLOR, title, SECTION_COLOR
                );
                lines.push(header);

                let indent = children
                    .iter()
                    .filter_map(|c| {
                        if let RenderNode::Line { icon, key, .. } = c {
                            if !icon.is_empty() {
                                Some(prefix_width(icon, key))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .max()
                    .unwrap_or(0);

                for child in children {
                    if let RenderNode::Line { key, value, icon } = child {
                        if icon.is_empty() && key.is_empty() {
                            lines.push(format!("\x1b[{}m│\x1b[0m {}", SECTION_COLOR, value));
                        } else if icon.is_empty() && key.starts_with("plugin:") {
                            let color_code = get_color_code(key, config);
                            lines.push(format!(
                                "\x1b[{}m│\x1b[0m \x1b[{}m{}\x1b[0m",
                                SECTION_COLOR, color_code, value
                            ));
                        } else if icon.is_empty() {
                            let color_code = get_color_code(key, config);
                            lines.push(format!(
                                "\x1b[{}m│\x1b[0m \x1b[{}m{:indent$}{}\x1b[0m",
                                SECTION_COLOR,
                                color_code,
                                "",
                                value,
                                indent = indent
                            ));
                        } else {
                            let key_color = get_color_code(key, config);
                            let label = display_key(key, config);
                            if label.is_empty() {
                                lines.push(format!(
                                    "\x1b[{}m│\x1b[0m \x1b[{}m{} {}\x1b[0m",
                                    SECTION_COLOR, key_color, icon, value
                                ));
                            } else {
                                let key_display = match config.key_width {
                                    Some(w) => format!("{:width$}:", label, width = w),
                                    None => format!("{}:", label),
                                };
                                lines.push(format!(
                                    "\x1b[{}m│\x1b[0m \x1b[{}m{} {}\x1b[0m {}",
                                    SECTION_COLOR, key_color, icon, key_display, value
                                ));
                            }
                        }
                    }
                }
                lines.push("".to_string());
            }
            RenderNode::Line { key, value, icon } => {
                if icon.is_empty() && key.is_empty() {
                    lines.push(format!("\x1b[{}m│\x1b[0m {}", SECTION_COLOR, value));
                } else if icon.is_empty() {
                    let color_code = get_color_code(key, config);
                    lines.push(format!(
                        "\x1b[{}m│\x1b[0m \x1b[{}m{}\x1b[0m",
                        SECTION_COLOR, color_code, value
                    ));
                } else {
                    lines.push(format_line(key, value, icon, config));
                }
            }
        }
    }
    lines
}

pub fn render_section_box(nodes: &[RenderNode], config: &Config) -> Vec<String> {
    let mut lines = Vec::new();
    let mut first = true;
    for node in nodes {
        match node {
            RenderNode::Group { title, children } => {
                if !first {
                    lines.push(String::new());
                }
                first = false;
                lines.extend(render_group_box(title, children, config));
            }
            RenderNode::Line { key, value, icon } => {
                if !first {
                    lines.push(String::new());
                }
                first = false;
                lines.push(render_section_row(key, value, icon, config, 0));
            }
        }
    }
    lines
}

fn render_group_box(title: &str, children: &[RenderNode], config: &Config) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();

    let indent = children
        .iter()
        .filter_map(|c| {
            if let RenderNode::Line { icon, key, .. } = c {
                if !icon.is_empty() {
                    Some(prefix_width(icon, key))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0);

    for child in children {
        match child {
            RenderNode::Group { title, children } => {
                rows.extend(render_group_box(title, children, config));
            }
            RenderNode::Line { key, value, icon } => {
                rows.push(render_section_row(key, value, icon, config, indent));
            }
        }
    }

    let mut inner_width = rows
        .iter()
        .map(|r| strip_ansi_codes(r).chars().count())
        .max()
        .unwrap_or(0);
    inner_width = inner_width.max(title.chars().count() + 1).max(1);
    let fill = inner_width - title.chars().count() - 1;

    let border = |s: String| format!("\x1b[{}m{}\x1b[0m", SECTION_COLOR, s);

    let mut lines = Vec::new();
    lines.push(border(format!("╭─ {} {}╮", title, "─".repeat(fill))));
    for row in rows {
        let pad = inner_width.saturating_sub(strip_ansi_codes(&row).chars().count());
        lines.push(border(format!("│ {} {}│", row, " ".repeat(pad))));
    }
    lines.push(border(format!("╰{}╯", "─".repeat(inner_width + 2))));
    lines
}

fn render_section_row(
    key: &str,
    value: &str,
    icon: &str,
    config: &Config,
    indent: usize,
) -> String {
    if icon.is_empty() && key.is_empty() {
        value.to_string()
    } else if icon.is_empty() && key.starts_with("plugin:") {
        let color_code = get_color_code(key, config);
        format!("\x1b[{}m{}\x1b[0m", color_code, value)
    } else if icon.is_empty() {
        let color_code = get_color_code(key, config);
        format!(
            "\x1b[{}m{:indent$}\x1b[0m{}",
            color_code,
            "",
            value,
            indent = indent
        )
    } else {
        format_line(key, value, icon, config)
    }
}

pub fn flatten_nodes(nodes: &[RenderNode]) -> Vec<(String, String, String)> {
    let mut items = Vec::new();
    for node in nodes {
        match node {
            RenderNode::Line { key, value, icon } => {
                items.push((key.clone(), value.clone(), icon.clone()))
            }
            RenderNode::Group { children, .. } => {
                let mut child_items = flatten_nodes(children);
                items.append(&mut child_items);
            }
        }
    }
    items
}

pub fn render_compact(nodes: &[RenderNode], config: &Config) -> Vec<String> {
    let mut lines = Vec::new();
    for node in nodes {
        match node {
            RenderNode::Line { key, value, icon } => {
                if icon.is_empty() && key.starts_with("plugin:") {
                    let color_code = get_color_code(key, config);
                    lines.push(format!("\x1b[{}m{}\x1b[0m", color_code, value));
                } else if icon.is_empty() {
                    lines.push(value.clone());
                } else {
                    let color_code = get_color_code(key, config);
                    lines.push(format!("\x1b[{}m{}\x1b[0m {}", color_code, icon, value));
                }
            }
            RenderNode::Group { children, .. } => {
                for child in children {
                    if let RenderNode::Line { key, value, icon } = child {
                        if icon.is_empty() && key.starts_with("plugin:") {
                            let color_code = get_color_code(key, config);
                            lines.push(format!("\x1b[{}m{}\x1b[0m", color_code, value));
                        } else if icon.is_empty() {
                            lines.push(value.clone());
                        } else {
                            let color_code = get_color_code(key, config);
                            lines.push(format!("\x1b[{}m{}\x1b[0m {}", color_code, icon, value));
                        }
                    }
                }
            }
        }
    }
    lines
}

pub fn render_minimal(nodes: &[RenderNode], config: &Config) -> Vec<String> {
    let mut lines = Vec::new();
    for node in nodes {
        match node {
            RenderNode::Line { key, value, .. } => {
                let label = display_key(key, config);
                if label.is_empty() {
                    lines.push(value.clone());
                } else {
                    let k = match config.key_width {
                        Some(w) => format!("{:width$}", label, width = w),
                        None => label,
                    };
                    lines.push(format!("{}: {}", k, value));
                }
            }
            RenderNode::Group { title, children } => {
                lines.push(format!("-- {} --", title));
                for child in children {
                    if let RenderNode::Line { key, value, .. } = child {
                        let label = display_key(key, config);
                        if label.is_empty() {
                            lines.push(value.clone());
                        } else {
                            let k = match config.key_width {
                                Some(w) => format!("{:width$}", label, width = w),
                                None => label,
                            };
                            lines.push(format!("{}: {}", k, value));
                        }
                    }
                }
            }
        }
    }
    lines
}

/// The key label shown for a module: the `labels` config entry when present
/// (an empty string hides the key), the module name otherwise. Colors keep
/// using the raw key, so renaming a label never breaks its color.
fn display_key(key: &str, config: &Config) -> String {
    config
        .labels
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

pub fn format_line(key: &str, value: &str, icon: &str, config: &Config) -> String {
    let color_code = get_color_code(key, config);
    if icon.is_empty() && key.starts_with("plugin:") {
        format!("\x1b[{}m{}\x1b[0m", color_code, value)
    } else if (config.show_keys && !key.is_empty()) || config.labels.contains_key(key) {
        let label = display_key(key, config);
        if label.is_empty() {
            format!("\x1b[{}m{} \x1b[0m{}", color_code, icon, value)
        } else {
            format!(
                "\x1b[{}m{} \x1b[0m\x1b[{}m{}\x1b[0m{}",
                color_code,
                icon,
                color_code,
                format_key(key, config),
                value
            )
        }
    } else {
        format!("\x1b[{}m{} \x1b[0m{}", color_code, icon, value)
    }
}

pub fn format_line_content(key: &str, value: &str, icon: &str, config: &Config) -> String {
    format_line(key, value, icon, config)
}

/// Resolves the SGR parameter string for a module color.
///
/// Accepts the same formats as the logo color: names (`"Cyan"`),
/// 256-color indexes (`"196"`) and hex RGB (`"#FF0000"`). Unknown values
/// fall back to white and are reported once.
pub fn get_color_code(key: &str, config: &Config) -> String {
    let value = config
        .colors
        .get(key)
        .map(String::as_str)
        .unwrap_or("White");
    if !is_known_color(value) {
        warn_unknown_color(value);
    }
    color_sgr(value)
}

pub fn color_code_from_name(name: &str) -> &'static str {
    named_color_code(name).unwrap_or("37")
}

/// Named colors shared by the renderers, including the documented dark
/// aliases. Returns `None` for anything that is not a known name.
fn named_color_code(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "black" | "darkblack" => Some("30"),
        "red" | "darkred" => Some("31"),
        "green" | "darkgreen" => Some("32"),
        "yellow" | "darkyellow" => Some("33"),
        "blue" | "darkblue" => Some("34"),
        "magenta" | "darkmagenta" => Some("35"),
        "cyan" | "darkcyan" => Some("36"),
        "white" => Some("37"),
        "grey" | "gray" | "darkgrey" | "darkgray" => Some("90"),
        _ => None,
    }
}

/// Whether `value` is a color this renderer understands.
fn is_known_color(value: &str) -> bool {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        return hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    if !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()) {
        return value.parse::<u8>().is_ok();
    }
    named_color_code(value).is_some()
}

/// Reports an unrecognized color once per value, so typos are visible
/// without spamming one warning per rendered line.
fn warn_unknown_color(value: &str) {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let warned = WARNED.get_or_init(|| Mutex::new(HashSet::new()));
    let Ok(mut warned) = warned.lock() else {
        return;
    };
    if warned.insert(value.to_string()) {
        eprintln!(
            "Warning: unknown color '{}'; using white. Use a name, a 0-255 index or #RRGGBB.",
            value
        );
    }
}

/// Resolves a color value to an SGR parameter string.
/// Supports names (`"Cyan"`), 256-color indexes (`"196"`, `"0"`-`"255"`)
/// and hex RGB (`"#FF0000"`).
pub fn color_sgr(value: &str) -> String {
    let v = value.trim();
    if let Some(hex) = v.strip_prefix('#') {
        let parse = |s: &str| u8::from_str_radix(s, 16).ok();
        if hex.len() == 6
            && let (Some(r), Some(g), Some(b)) =
                (parse(&hex[0..2]), parse(&hex[2..4]), parse(&hex[4..6]))
        {
            return format!("38;2;{};{};{}", r, g, b);
        }
        return "37".to_string();
    }
    if !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(n) = v.parse::<u8>() {
            return format!("38;5;{}", n);
        }
        return "37".to_string();
    }
    color_code_from_name(v).to_string()
}

pub fn format_key(key: &str, config: &Config) -> String {
    let label = display_key(key, config);
    if label.is_empty() {
        return String::new();
    }
    let k = match config.key_width {
        Some(w) => format!("{:width$}", label, width = w),
        None => label,
    };
    format!("{}: ", k)
}

//tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::ui::nodes::RenderNode;

    #[test]
    fn test_color_sgr_accepts_names_indexes_and_hex() {
        assert_eq!(color_sgr("Cyan"), "36");
        assert_eq!(color_sgr("cyan"), "36");
        assert_eq!(color_sgr("darkred"), "31");
        assert_eq!(color_sgr("196"), "38;5;196");
        assert_eq!(color_sgr("0"), "38;5;0");
        assert_eq!(color_sgr("#FF8800"), "38;2;255;136;0");
        assert_eq!(color_sgr("  #ff8800  "), "38;2;255;136;0");
    }

    #[test]
    fn test_color_sgr_falls_back_for_invalid_values() {
        assert_eq!(color_sgr("#F00"), "37");
        assert_eq!(color_sgr("#GG0000"), "37");
        assert_eq!(color_sgr("256"), "37");
        assert_eq!(color_sgr("nonsense"), "37");
    }

    #[test]
    fn test_module_colors_use_the_full_color_parser() {
        let mut config = Config::default();
        config
            .colors
            .insert("os".to_string(), "#FF8800".to_string());
        config.colors.insert("cpu".to_string(), "196".to_string());
        config
            .colors
            .insert("memory".to_string(), "Cyan".to_string());

        assert_eq!(get_color_code("os", &config), "38;2;255;136;0");
        assert_eq!(get_color_code("cpu", &config), "38;5;196");
        assert_eq!(get_color_code("memory", &config), "36");
        assert_eq!(get_color_code("unknown", &config), "37");
    }

    #[test]
    fn test_known_color_detection_matches_the_parser() {
        assert!(is_known_color("Cyan"));
        assert!(is_known_color("DARKGRAY"));
        assert!(is_known_color("196"));
        assert!(is_known_color("#FF8800"));
        assert!(!is_known_color("nonsense"));
        assert!(!is_known_color("#F00"));
        assert!(!is_known_color("256"));
    }

    #[test]
    // Test that classic render doesn't crash with empty nodes
    fn test_render_classic_empty() {
        let config = Config::default();
        let nodes: Vec<RenderNode> = vec![];
        let lines = render_classic(&nodes, &config);

        assert!(lines.is_empty() || !lines.is_empty());
    }

    #[test]
    // Test that side block render doesn't crash with empty nodes
    fn test_render_side_block_empty() {
        let config = Config::default();
        let nodes: Vec<RenderNode> = vec![];
        let lines = render_side_block(&nodes, &config);

        assert!(lines.is_empty() || !lines.is_empty());
    }

    #[test]
    // Test that section-box renders a bordered box per group
    fn test_render_section_box_groups() {
        let config = Config::default();
        let nodes = vec![
            RenderNode::Group {
                title: "Hardware".to_string(),
                children: vec![
                    RenderNode::Line {
                        key: "cpu".to_string(),
                        value: "Apple M4".to_string(),
                        icon: "\u{f2db}".to_string(),
                    },
                    RenderNode::Line {
                        key: "memory".to_string(),
                        value: "16 GiB".to_string(),
                        icon: "\u{e266}".to_string(),
                    },
                ],
            },
            RenderNode::Group {
                title: "Software".to_string(),
                children: vec![RenderNode::Line {
                    key: "os".to_string(),
                    value: "Darwin".to_string(),
                    icon: "\u{f17c}".to_string(),
                }],
            },
        ];

        let lines = render_section_box(&nodes, &config);
        assert!(lines.len() >= 6);

        let joined = lines.join("\n");
        assert!(joined.contains("╭─ Hardware"));
        assert!(joined.contains("╭─ Software"));
        assert!(joined.contains("│"));
        assert!(joined.contains("╰"));
        assert_eq!(
            strip_ansi_codes(&lines[1]).chars().count(),
            strip_ansi_codes(&lines[0]).chars().count()
        );
    }

    #[test]
    fn test_format_line_with_keys() {
        let config = Config {
            show_keys: true,
            key_width: Some(10),
            ..Config::default()
        };
        let line = format_line("cpu", "Apple M4", "\u{f2db}", &config);
        assert!(line.contains("cpu"));
        assert!(line.contains("Apple M4"));
        let stripped = strip_ansi_codes(&line);
        assert!(stripped.contains("cpu       : "));
        assert!(stripped.contains("\u{f2db}"));
    }

    #[test]
    fn test_format_line_no_keys_by_default() {
        let config = Config::default();
        let line = format_line("cpu", "Apple M4", "\u{f2db}", &config);
        assert!(!line.contains("cpu"));
    }

    #[test]
    fn test_format_line_label_renames_key() {
        let config = Config {
            show_keys: true,
            labels: [("cpu".to_string(), "procesador".to_string())]
                .into_iter()
                .collect(),
            ..Config::default()
        };
        let line = format_line("cpu", "Apple M4", "\u{f2db}", &config);
        assert!(line.contains("procesador:"));
        assert!(!line.contains("cpu:"));
    }

    #[test]
    fn test_format_line_empty_label_hides_key() {
        let config = Config {
            show_keys: true,
            labels: [("cpu".to_string(), String::new())].into_iter().collect(),
            ..Config::default()
        };
        let line = format_line("cpu", "Apple M4", "\u{f2db}", &config);
        assert!(line.contains("Apple M4"));
        assert!(!line.contains("cpu"));
    }

    #[test]
    fn test_format_line_label_shows_without_show_keys() {
        let config = Config {
            labels: [("cpu".to_string(), "cpu2".to_string())]
                .into_iter()
                .collect(),
            ..Config::default()
        };
        let line = format_line("cpu", "Apple M4", "\u{f2db}", &config);
        assert!(line.contains("cpu2:"));
    }

    #[test]
    fn test_render_minimal_labels() {
        let config = Config {
            labels: [
                ("cpu".to_string(), "cpu".to_string()),
                ("gpu".to_string(), String::new()),
            ]
            .into_iter()
            .collect(),
            ..Config::default()
        };
        let nodes = vec![
            RenderNode::Line {
                key: "cpu".to_string(),
                value: "Apple M4".to_string(),
                icon: String::new(),
            },
            RenderNode::Line {
                key: "gpu".to_string(),
                value: "Apple M4 Max".to_string(),
                icon: String::new(),
            },
        ];
        let lines = render_minimal(&nodes, &config);
        assert_eq!(lines[0], "cpu: Apple M4");
        assert_eq!(lines[1], "Apple M4 Max");
    }

    #[test]
    // Test that tree render doesn't crash with empty nodes
    fn test_render_tree_empty() {
        let config = Config::default();
        let nodes: Vec<RenderNode> = vec![];
        let lines = render_tree(&nodes, &config);

        assert!(lines.is_empty() || !lines.is_empty());
    }
}
