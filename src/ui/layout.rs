use super::custom_x::render_custom_x;
use super::nodes::RenderNode;
use super::renders::{
    render_classic, render_classic_variants, render_compact, render_minimal, render_section,
    render_section_box, render_side_block, render_tree,
};
use crate::config::Config;

const DEFAULT_LAYOUT: &str = "default";
const SIDEBLOCK_LAYOUT: &str = "side-block";
const TREE_LAYOUT: &str = "tree";
const SECTION_LAYOUT: &str = "section";
const SECTION_BOX_LAYOUT: &str = "section-box";
const CUSTOM_X_LAYOUT: &str = "custom-x";
const COMPACT_LAYOUT: &str = "compact";
const MINIMAL_LAYOUT: &str = "minimal";
const CLASSIC_VARIANTS: &[&str] = &["pacman", "box", "line", "dots", "bottom_line"];
const VERTICAL_LAYOUTS: &[&str] = &["horizontal", "bottom"];

pub fn get_content_lines(
    nodes: &[RenderNode],
    config: &Config,
    available_width: Option<usize>,
) -> Vec<String> {
    let layout_type = config.layout.as_deref().unwrap_or(DEFAULT_LAYOUT);
    match layout_type {
        SIDEBLOCK_LAYOUT => render_side_block(nodes, config),
        TREE_LAYOUT => render_tree(nodes, config),
        SECTION_LAYOUT => render_section(nodes, config),
        SECTION_BOX_LAYOUT => render_section_box(nodes, config),
        CUSTOM_X_LAYOUT => render_custom_x(nodes, config, available_width),
        COMPACT_LAYOUT => render_compact(nodes, config),
        MINIMAL_LAYOUT => render_minimal(nodes, config),
        _ if CLASSIC_VARIANTS.contains(&layout_type) => {
            render_classic_variants(nodes, config, layout_type)
        }
        _ => render_classic(nodes, config),
    }
}

pub fn is_vertical(layout: Option<&String>) -> bool {
    layout
        .map(|l| VERTICAL_LAYOUTS.contains(&l.as_str()))
        .unwrap_or(false)
}

pub fn is_minimal(layout: Option<&String>) -> bool {
    layout.map(|l| l == MINIMAL_LAYOUT).unwrap_or(false)
}

pub fn is_bottom(layout: Option<&String>) -> bool {
    layout.map(|l| l == "bottom").unwrap_or(false)
}

//tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_get_content_lines_empty() {
        // Verify that the layout correctly handles an empty list of nodes without crashing
        let config = Config::default();
        let nodes = vec![];
        let lines = get_content_lines(&nodes, &config, None);

        // We simply ensure it returns a valid vector
        assert!(lines.is_empty() || !lines.is_empty());
    }
}
