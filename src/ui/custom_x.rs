use crate::config::Config;
use crate::ui::nodes::RenderNode;
use crate::ui::renders::format_line;
use console::strip_ansi_codes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CustomXWidth {
    Mode(String),
    Fixed(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomX {
    pub top: Option<String>,
    pub bottom: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub fill: Option<String>,
    pub padding: Option<usize>,
    pub width: Option<CustomXWidth>,
    pub full_margin: Option<usize>,
    pub group_title: Option<String>,
    pub divider: Option<String>,
    pub divider_between: Option<String>,
    pub module_top: Option<String>,
    pub module_bottom: Option<String>,
    pub header_lines: Vec<String>,
    pub footer_lines: Vec<String>,
}

impl Default for CustomX {
    fn default() -> Self {
        Self {
            top: Some("╭─ {title}{fill}╮".to_string()),
            bottom: Some("╰{fill}╯".to_string()),
            left: Some("│".to_string()),
            right: Some("│".to_string()),
            fill: Some("─".to_string()),
            padding: Some(1),
            width: None,
            full_margin: Some(2),
            group_title: Some("── {title} ──".to_string()),
            divider: Some(String::new()),
            divider_between: Some("groups".to_string()),
            module_top: Some(String::new()),
            module_bottom: Some(String::new()),
            header_lines: Vec::new(),
            footer_lines: Vec::new(),
        }
    }
}

enum Row {
    Line(String),
    Template { tpl: String, title: String },
}

fn visual_len(s: &str) -> usize {
    console::measure_text_width(&strip_ansi_codes(s))
}

fn render_template(tpl: &str, title: &str, fill: &str, width: usize) -> String {
    let fill = if fill.is_empty() { " " } else { fill };
    let t = if title.is_empty() {
        tpl.replace(" {title}", "").replace("{title}", "")
    } else {
        tpl.replace("{title}", title)
    };

    if t.contains("{fill}") {
        let base = t.replace("{fill}", "");
        let base_len = visual_len(&base);
        let need = width.saturating_sub(base_len);
        let fill_len = visual_len(fill).max(1);
        let full = need / fill_len;
        let rem = need % fill_len;
        let mut rep = fill.repeat(full);
        rep.push_str(&fill.chars().take(rem).collect::<String>());
        t.replace("{fill}", &rep)
    } else {
        let len = visual_len(&t);
        if len < width {
            let need = width - len;
            let fill_len = visual_len(fill).max(1);
            let full = need / fill_len;
            let rem = need % fill_len;
            let mut s = t;
            s.push_str(&fill.repeat(full));
            s.push_str(&fill.chars().take(rem).collect::<String>());
            s
        } else {
            t
        }
    }
}

struct GroupCtx<'a> {
    title_tpl: &'a str,
    divider_tpl: &'a str,
    between: &'a str,
    module_top_tpl: &'a str,
    module_bottom_tpl: &'a str,
    config: &'a Config,
}

fn append_group_rows(
    rows: &mut Vec<Row>,
    title: &str,
    children: &[RenderNode],
    ctx: &GroupCtx,
    last_was_divider: &mut bool,
) {
    if !rows.is_empty() && !*last_was_divider && !ctx.divider_tpl.is_empty() {
        rows.push(Row::Template {
            tpl: ctx.divider_tpl.to_string(),
            title: String::new(),
        });
        *last_was_divider = true;
    }
    if !ctx.title_tpl.is_empty() {
        rows.push(Row::Template {
            tpl: ctx.title_tpl.to_string(),
            title: title.to_string(),
        });
        *last_was_divider = false;
    }
    for child in children {
        match child {
            RenderNode::Group {
                title,
                children: inner,
            } => {
                append_group_rows(rows, title, inner, ctx, last_was_divider);
            }
            RenderNode::Line { key, value, icon } => {
                if !ctx.module_top_tpl.is_empty() {
                    rows.push(Row::Template {
                        tpl: ctx.module_top_tpl.to_string(),
                        title: String::new(),
                    });
                }
                rows.push(Row::Line(format_line(key, value, icon, ctx.config)));
                if !ctx.module_bottom_tpl.is_empty() {
                    rows.push(Row::Template {
                        tpl: ctx.module_bottom_tpl.to_string(),
                        title: String::new(),
                    });
                    *last_was_divider = true;
                } else {
                    *last_was_divider = false;
                }
                if ctx.between == "modules" && !ctx.divider_tpl.is_empty() {
                    rows.push(Row::Template {
                        tpl: ctx.divider_tpl.to_string(),
                        title: String::new(),
                    });
                    *last_was_divider = true;
                }
            }
        }
    }
}

/// Renders `"layout": "custom-x"`: every border line is a literal template the
/// user writes in `custom_x` config. `{fill}` repeats the `fill` character up
/// to the box width; `{title}` is replaced with the current group title.
/// Templates shorter than the box are extended with `fill`; longer ones define
/// the box width.
pub fn render_custom_x(
    nodes: &[RenderNode],
    config: &Config,
    available_width: Option<usize>,
) -> Vec<String> {
    let cx = config.custom_x.clone().unwrap_or_default();
    let left = cx.left.clone().unwrap_or_default();
    let right = cx.right.clone().unwrap_or_default();
    let fill = cx.fill.clone().unwrap_or_else(|| "─".to_string());
    let padding = cx.padding.unwrap_or(1);
    let title_tpl = cx.group_title.clone().unwrap_or_default();
    let divider_tpl = cx.divider.clone().unwrap_or_default();
    let module_top_tpl = cx.module_top.clone().unwrap_or_default();
    let module_bottom_tpl = cx.module_bottom.clone().unwrap_or_default();
    let between = cx
        .divider_between
        .clone()
        .unwrap_or_else(|| "groups".to_string());
    let top_tpl = cx.top.clone().unwrap_or_else(|| "╭─ {title}{fill}╮".to_string());
    let bottom_tpl = cx.bottom.clone().unwrap_or_else(|| "╰{fill}╯".to_string());

    let mut rows: Vec<Row> = Vec::new();
    let mut last_was_divider = false;
    let ctx = GroupCtx {
        title_tpl: &title_tpl,
        divider_tpl: &divider_tpl,
        between: &between,
        module_top_tpl: &module_top_tpl,
        module_bottom_tpl: &module_bottom_tpl,
        config,
    };

    for node in nodes {
        match node {
            RenderNode::Group { title, children } => {
                append_group_rows(
                    &mut rows,
                    title,
                    children,
                    &ctx,
                    &mut last_was_divider,
                );
            }
            RenderNode::Line { key, value, icon } => {
                if !module_top_tpl.is_empty() {
                    rows.push(Row::Template {
                        tpl: module_top_tpl.to_string(),
                        title: String::new(),
                    });
                }
                rows.push(Row::Line(format_line(key, value, icon, config)));
                if !module_bottom_tpl.is_empty() {
                    rows.push(Row::Template {
                        tpl: module_bottom_tpl.to_string(),
                        title: String::new(),
                    });
                    last_was_divider = true;
                } else {
                    last_was_divider = false;
                }
            }
        }
    }

    let top_title = nodes
        .iter()
        .find_map(|n| {
            if let RenderNode::Group { title, .. } = n {
                Some(title.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    let side_len = visual_len(&left) + visual_len(&right) + padding * 2;
    let mut width = 0usize;
    for l in &cx.header_lines {
        width = width.max(visual_len(&render_template(l, "", "", 0)));
    }
    for l in &cx.footer_lines {
        width = width.max(visual_len(&render_template(l, "", "", 0)));
    }
    for row in &rows {
        match row {
            Row::Line(s) => width = width.max(visual_len(s) + side_len),
            Row::Template { tpl, title } => {
                width = width.max(visual_len(&render_template(tpl, title, "", 0)))
            }
        }
    }
    width = width.max(visual_len(&render_template(&top_tpl, &top_title, "", 0)));
    width = width.max(visual_len(&render_template(&bottom_tpl, "", "", 0)));

    let stretch = match &cx.width {
        Some(CustomXWidth::Fixed(n)) => Some(*n),
        Some(CustomXWidth::Mode(m)) if m == "full" => available_width.map(|w| {
            w.saturating_sub(cx.full_margin.unwrap_or(2))
        }),
        _ => None,
    };
    if let Some(w) = stretch {
        width = width.max(w);
    }

    let mut lines = Vec::new();
    if !top_tpl.is_empty() {
        lines.push(render_template(&top_tpl, &top_title, &fill, width));
    }
    for l in &cx.header_lines {
        if !l.is_empty() {
            lines.push(render_template(l, "", &fill, width));
        }
    }
    let pad = " ".repeat(padding);
    for row in &rows {
        match row {
            Row::Line(s) => {
                let content = format!("{}{}{}{}", left, pad, s, pad);
                let extra = width.saturating_sub(visual_len(&content) + visual_len(&right));
                lines.push(format!("{}{}{}", content, " ".repeat(extra), right));
            }
            Row::Template { tpl, title } => {
                lines.push(render_template(tpl, title, &fill, width));
            }
        }
    }
    for l in &cx.footer_lines {
        if !l.is_empty() {
            lines.push(render_template(l, "", &fill, width));
        }
    }
    if !bottom_tpl.is_empty() {
        lines.push(render_template(&bottom_tpl, "", &fill, width));
    }

    lines
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    fn sample_nodes() -> Vec<RenderNode> {
        vec![
            RenderNode::Group {
                title: "Hardware".to_string(),
                children: vec![
                    RenderNode::Line {
                        key: "cpu".to_string(),
                        value: "Example CPU".to_string(),
                        icon: "\u{f2db}".to_string(),
                    },
                    RenderNode::Line {
                        key: "memory".to_string(),
                        value: "8 GiB".to_string(),
                        icon: "\u{e266}".to_string(),
                    },
                ],
            },
            RenderNode::Group {
                title: "Software".to_string(),
                children: vec![RenderNode::Line {
                    key: "os".to_string(),
                    value: "ExampleOS".to_string(),
                    icon: "\u{f17c}".to_string(),
                }],
            },
        ]
    }

    #[test]
    fn test_render_template_fill_reaches_width() {
        let s = render_template("╭─ {title}{fill}╮", "Hardware", "─", 30);
        assert_eq!(visual_len(&s), 30);
        assert!(s.starts_with("╭─ Hardware"));
        assert!(s.ends_with("╮"));
    }

    #[test]
    fn test_render_template_short_without_fill_extends() {
        let s = render_template("── {title} ──", "Hardware", "─", 25);
        assert_eq!(visual_len(&s), 25);
    }

    #[test]
    fn test_render_custom_x_structure() {
        let config = Config::default();
        let lines = render_custom_x(&sample_nodes(), &config, None);
        assert!(lines.len() >= 5);
        let joined = lines.join("\n");
        assert!(joined.contains("╭"));
        assert!(joined.contains("╰"));
        assert!(joined.contains("Hardware"));
        assert!(joined.contains("Software"));
        assert!(joined.contains("Example CPU"));
        assert_eq!(visual_len(&lines[0]), visual_len(&lines[lines.len() - 1]));
    }

    #[test]
    fn test_render_custom_x_literal_top() {
        let mut config = Config::default();
        config.custom_x = Some(CustomX {
            top: Some("-----mifetch{fill}|".to_string()),
            bottom: Some("|{fill}-----".to_string()),
            left: Some("|".to_string()),
            right: Some("|".to_string()),
            ..CustomX::default()
        });
        let lines = render_custom_x(&sample_nodes(), &config, None);
        assert!(lines[0].starts_with("-----mifetch"));
        assert!(lines[0].ends_with('|'));
        assert!(lines[lines.len() - 1].starts_with('|'));
        assert_eq!(visual_len(&lines[0]), visual_len(&lines[lines.len() - 1]));
    }

    #[test]
    fn test_render_custom_x_width_full_stretches() {
        let mut config = Config::default();
        config.custom_x = Some(CustomX {
            width: Some(CustomXWidth::Mode("full".to_string())),
            ..CustomX::default()
        });
        let lines = render_custom_x(&sample_nodes(), &config, Some(100));
        for line in &lines {
            assert_eq!(visual_len(line), 98);
        }
    }

    #[test]
    fn test_render_custom_x_width_full_no_margin() {
        let mut config = Config::default();
        config.custom_x = Some(CustomX {
            width: Some(CustomXWidth::Mode("full".to_string())),
            full_margin: Some(0),
            ..CustomX::default()
        });
        let lines = render_custom_x(&sample_nodes(), &config, Some(100));
        for line in &lines {
            assert_eq!(visual_len(line), 100);
        }
    }

    #[test]
    fn test_render_custom_x_width_fixed() {
        let mut config = Config::default();
        config.custom_x = Some(CustomX {
            width: Some(CustomXWidth::Fixed(60)),
            ..CustomX::default()
        });
        let lines = render_custom_x(&sample_nodes(), &config, None);
        for line in &lines {
            assert_eq!(visual_len(line), 60);
        }
    }

    #[test]
    fn test_render_custom_x_module_boxes() {
        let mut config = Config::default();
        config.custom_x = Some(CustomX {
            module_top: Some("╠{fill}╣".to_string()),
            module_bottom: Some("╠{fill}╣".to_string()),
            ..CustomX::default()
        });
        let lines = render_custom_x(&sample_nodes(), &config, None);
        let joined = lines.join("\n");
        assert!(joined.matches("╠").count() >= 6);
        assert!(joined.contains("Example CPU"));
        for line in &lines {
            assert_eq!(visual_len(line), visual_len(&lines[0]));
        }
    }
}
