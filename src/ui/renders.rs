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
                    lines.push(format!("\x1b[{}m│\x1b[0m \x1b[{}m{}\x1b[0m", SECTION_COLOR, color_code, value));
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
                            lines.push(format!(
                                "\x1b[{}m{}\x1b[0m \x1b[{}m{}\x1b[0m {}",
                                SECTION_COLOR, prefix, key_color, key, value
                            ));
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
                            lines.push(format!(
                                "\x1b[{}m│\x1b[0m \x1b[{}m{} {}:\x1b[0m {}",
                                SECTION_COLOR, key_color, icon, key, value
                            ));
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

pub fn render_minimal(nodes: &[RenderNode], _config: &Config) -> Vec<String> {
    let mut lines = Vec::new();
    for node in nodes {
        match node {
            RenderNode::Line { key, value, .. } => {
                lines.push(format!("{}: {}", key, value));
            }
            RenderNode::Group { title, children } => {
                lines.push(format!("-- {} --", title));
                for child in children {
                    if let RenderNode::Line { key, value, .. } = child {
                        lines.push(format!("{}: {}", key, value));
                    }
                }
            }
        }
    }
    lines
}

pub fn format_line(key: &str, value: &str, icon: &str, config: &Config) -> String {
    let color_code = get_color_code(key, config);
    if icon.is_empty() && key.starts_with("plugin:") {
        format!("\x1b[{}m{}\x1b[0m", color_code, value)
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
    match color_name.to_lowercase().as_str() {
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
    //  Test that tree render doesn't crash with empty nodes
    fn test_render_tree_empty() {
        let config = Config::default();
        let nodes: Vec<RenderNode> = vec![];
        let lines = render_tree(&nodes, &config);

        assert!(lines.is_empty() || !lines.is_empty());
    }
}
