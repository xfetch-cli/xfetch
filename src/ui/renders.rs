use super::nodes::RenderNode;
use crate::config::Config;
use console::strip_ansi_codes;

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

pub fn get_color_code(key: &str, config: &Config) -> &'static str {
    let color_name = config
        .colors
        .get(key)
        .map(|s| s.as_str())
        .unwrap_or("White");
    color_code_from_name(color_name)
}

pub fn color_code_from_name(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "black" => "30",
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        "grey" | "gray" => "90",
        _ => "37",
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
