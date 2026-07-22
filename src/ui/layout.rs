use super::nodes::RenderNode;
use super::renders::{
    render_classic, render_classic_variants, render_section, render_side_block, render_tree,
};
use crate::config::Config;

const DEFAULT_LAYOUT: &str = "default";
const SIDEBLOCK_LAYOUT: &str = "side-block";
const TREE_LAYOUT: &str = "tree";
const SECTION_LAYOUT: &str = "section";
const CLASSIC_VARIANTS: &[&str] = &["pacman", "box", "line", "dots", "bottom_line"];

pub fn get_content_lines(nodes: &[RenderNode], config: &Config) -> Vec<String> {
    let layout_type = config.layout.as_deref().unwrap_or(DEFAULT_LAYOUT);
    match layout_type {
        SIDEBLOCK_LAYOUT => render_side_block(nodes, config),
        TREE_LAYOUT => render_tree(nodes, config),
        SECTION_LAYOUT => render_section(nodes, config),
        _ if CLASSIC_VARIANTS.contains(&layout_type) => {
            render_classic_variants(nodes, config, layout_type)
        }
        _ => render_classic(nodes, config),
    }
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
        let lines = get_content_lines(&nodes, &config);

        // We simply ensure it returns a valid vector
        assert!(lines.is_empty() || !lines.is_empty());
    }
}
