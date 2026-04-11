//! Core graph data structures for the CSS Property Knowledge Graph.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// A single CSS property node in the knowledge graph.
#[derive(Debug, Clone)]
pub struct PropertyNode {
    /// Property name in kebab-case, e.g. "font-size".
    pub name: String,
    /// Relative proto file path, e.g. "css/css_properties.proto".
    pub proto_file: String,
    /// Enum discriminant index from CssProperty enum.
    pub proto_enum_idx: u32,

    // --- Extracted from proto doc comments ---
    /// Value syntax from spec, e.g. "<absolute-size> | <relative-size> | <length>".
    pub value_syntax: String,
    /// Initial/default value, e.g. "medium".
    pub initial_value: String,
    /// Whether the property inherits.
    pub inherits: bool,
    /// Whether changes trigger layout recalculation.
    pub affects_layout: bool,
    /// Whether changes trigger repaint.
    pub affects_paint: bool,
    /// Whether the property can be animated.
    pub animatable: bool,
    /// How the computed value is determined.
    pub computed_value: String,

    // --- Relationships ---
    /// Other properties this one depends on.
    pub depends_on: Vec<String>,
    /// Longhand properties this shorthand expands to.
    pub longhands: Vec<String>,
    /// If this is a longhand, the shorthand it belongs to.
    pub shorthand_for: Option<String>,
}

impl PropertyNode {
    pub fn new(name: impl Into<String>, proto_file: impl Into<String>, proto_enum_idx: u32) -> Self {
        Self {
            name: name.into(),
            proto_file: proto_file.into(),
            proto_enum_idx,
            value_syntax: String::new(),
            initial_value: String::new(),
            inherits: false,
            affects_layout: false,
            affects_paint: false,
            animatable: false,
            computed_value: String::new(),
            depends_on: Vec::new(),
            longhands: Vec::new(),
            shorthand_for: None,
        }
    }
}

/// Summary statistics about the graph.
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub total: usize,
    pub inherited: usize,
    pub animatable: usize,
    pub layout_triggers: usize,
    pub paint_triggers: usize,
    pub shorthands: usize,
    pub longhands: usize,
}

/// The CSS Property Knowledge Graph.
///
/// Stores every CSS property as a [`PropertyNode`] and provides
/// fluent query APIs for exploration.
#[derive(Debug, Clone)]
pub struct PropertyGraph {
    /// All nodes, keyed by property name.
    pub(crate) nodes: BTreeMap<String, PropertyNode>,
}

impl PropertyGraph {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }

    /// Insert or replace a property node.
    pub fn insert(&mut self, node: PropertyNode) {
        let name = node.name.clone();
        self.nodes.insert(name, node);
    }

    /// Get a node by property name.
    pub fn get(&self, name: &str) -> Option<&PropertyNode> {
        self.nodes.get(name)
    }

    /// Iterate over all property nodes.
    pub fn iter(&self) -> impl Iterator<Item = &PropertyNode> {
        self.nodes.values()
    }

    /// Number of properties in the graph.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Return all property names.
    pub fn property_names(&self) -> Vec<&str> {
        self.nodes.keys().map(|s| s.as_str()).collect()
    }

    /// Find all properties that have `reverse_deps` pointing to `name`.
    pub fn reverse_deps(&self, name: &str) -> Vec<&str> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.depends_on.iter().any(|d| d == name))
            .map(|(k, _)| k.as_str())
            .collect()
    }

    /// Expand a shorthand property into its longhand properties.
    pub fn expand_shorthand(&self, shorthand: &str) -> Vec<&str> {
        if let Some(node) = self.nodes.get(shorthand) {
            node.longhands.iter().map(|s| s.as_str()).collect()
        } else {
            Vec::new()
        }
    }

    /// Compute summary statistics.
    pub fn stats(&self) -> GraphStats {
        let mut stats = GraphStats {
            total: 0,
            inherited: 0,
            animatable: 0,
            layout_triggers: 0,
            paint_triggers: 0,
            shorthands: 0,
            longhands: 0,
        };
        for node in self.nodes.values() {
            stats.total += 1;
            if node.inherits {
                stats.inherited += 1;
            }
            if node.animatable {
                stats.animatable += 1;
            }
            if node.affects_layout {
                stats.layout_triggers += 1;
            }
            if node.affects_paint {
                stats.paint_triggers += 1;
            }
            if !node.longhands.is_empty() {
                stats.shorthands += 1;
            }
            if node.shorthand_for.is_some() {
                stats.longhands += 1;
            }
        }
        stats
    }

    /// Begin a fluent query.
    pub fn query(&self) -> QueryBuilder<'_> {
        QueryBuilder::new(self)
    }
}

impl Default for PropertyGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Fluent Query Builder
// ---------------------------------------------------------------------------

/// A fluent query builder for filtering property nodes.
///
/// # Example
/// ```ignore
/// let inherited = graph.query()
///     .inherits(true)
///     .execute();
/// ```
#[derive(Clone)]
pub struct QueryBuilder<'a> {
    graph: &'a PropertyGraph,
    filter_inherits: Option<bool>,
    filter_affects_layout: Option<bool>,
    filter_affects_paint: Option<bool>,
    filter_animatable: Option<bool>,
    filter_name_contains: Option<&'a str>,
    filter_proto_file: Option<&'a str>,
}

impl<'a> QueryBuilder<'a> {
    fn new(graph: &'a PropertyGraph) -> Self {
        Self {
            graph,
            filter_inherits: None,
            filter_affects_layout: None,
            filter_affects_paint: None,
            filter_animatable: None,
            filter_name_contains: None,
            filter_proto_file: None,
        }
    }

    /// Filter by inheritance.
    pub fn inherits(mut self, value: bool) -> Self {
        self.filter_inherits = Some(value);
        self
    }

    /// Filter by whether the property triggers layout.
    pub fn affects_layout(mut self, value: bool) -> Self {
        self.filter_affects_layout = Some(value);
        self
    }

    /// Filter by whether the property triggers paint.
    pub fn affects_paint(mut self, value: bool) -> Self {
        self.filter_affects_paint = Some(value);
        self
    }

    /// Filter by whether the property is animatable.
    pub fn animatable(mut self, value: bool) -> Self {
        self.filter_animatable = Some(value);
        self
    }

    /// Filter by property name substring.
    pub fn name_contains(mut self, substr: &'a str) -> Self {
        self.filter_name_contains = Some(substr);
        self
    }

    /// Filter by proto file path substring.
    pub fn proto_file(mut self, file: &'a str) -> Self {
        self.filter_proto_file = Some(file);
        self
    }

    /// Execute the query and return matching property names.
    pub fn execute(&self) -> Vec<&str> {
        self.graph
            .nodes
            .values()
            .filter(|node| {
                if let Some(v) = self.filter_inherits {
                    if node.inherits != v {
                        return false;
                    }
                }
                if let Some(v) = self.filter_affects_layout {
                    if node.affects_layout != v {
                        return false;
                    }
                }
                if let Some(v) = self.filter_affects_paint {
                    if node.affects_paint != v {
                        return false;
                    }
                }
                if let Some(v) = self.filter_animatable {
                    if node.animatable != v {
                        return false;
                    }
                }
                if let Some(substr) = self.filter_name_contains {
                    if !node.name.contains(substr) {
                        return false;
                    }
                }
                if let Some(file) = self.filter_proto_file {
                    if !node.proto_file.contains(file) {
                        return false;
                    }
                }
                true
            })
            .map(|node| node.name.as_str())
            .collect()
    }

    /// Execute the query and return owned property names.
    ///
    /// Use this when the builder is a temporary, as it avoids lifetime issues.
    pub fn execute_owned(self) -> Vec<String> {
        self.graph
            .nodes
            .values()
            .filter(|node| {
                if let Some(v) = self.filter_inherits {
                    if node.inherits != v {
                        return false;
                    }
                }
                if let Some(v) = self.filter_affects_layout {
                    if node.affects_layout != v {
                        return false;
                    }
                }
                if let Some(v) = self.filter_affects_paint {
                    if node.affects_paint != v {
                        return false;
                    }
                }
                if let Some(v) = self.filter_animatable {
                    if node.animatable != v {
                        return false;
                    }
                }
                if let Some(substr) = self.filter_name_contains {
                    if !node.name.contains(substr) {
                        return false;
                    }
                }
                if let Some(file) = self.filter_proto_file {
                    if !node.proto_file.contains(file) {
                        return false;
                    }
                }
                true
            })
            .map(|node| node.name.clone())
            .collect()
    }

    /// Execute and return the full matching nodes.
    pub fn execute_nodes(&self) -> Vec<&PropertyNode> {
        self.graph
            .nodes
            .values()
            .filter(|node| {
                if let Some(v) = self.filter_inherits {
                    if node.inherits != v {
                        return false;
                    }
                }
                if let Some(v) = self.filter_affects_layout {
                    if node.affects_layout != v {
                        return false;
                    }
                }
                if let Some(v) = self.filter_affects_paint {
                    if node.affects_paint != v {
                        return false;
                    }
                }
                if let Some(v) = self.filter_animatable {
                    if node.animatable != v {
                        return false;
                    }
                }
                if let Some(substr) = self.filter_name_contains {
                    if !node.name.contains(substr) {
                        return false;
                    }
                }
                if let Some(file) = self.filter_proto_file {
                    if !node.proto_file.contains(file) {
                        return false;
                    }
                }
                true
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_graph() -> PropertyGraph {
        let mut g = PropertyGraph::new();
        let mut fs = PropertyNode::new("font-size", "css/css_properties.proto", 60);
        fs.inherits = true;
        fs.animatable = true;
        fs.value_syntax = "<absolute-size> | <relative-size> | <length-percentage [0,∞]> | math".into();
        fs.initial_value = "medium".into();
        g.insert(fs);

        let mut lh = PropertyNode::new("line-height", "css/css_properties.proto", 155);
        lh.inherits = true;
        lh.animatable = true;
        lh.depends_on.push("font-size".into());
        g.insert(lh);

        let mut bg = PropertyNode::new("background", "css/css_properties.proto", 18);
        bg.longhands = vec![
            "background-color".into(),
            "background-image".into(),
            "background-repeat".into(),
            "background-attachment".into(),
            "background-position".into(),
            "background-clip".into(),
            "background-origin".into(),
            "background-size".into(),
        ];
        bg.affects_paint = true;
        g.insert(bg);

        let mut bc = PropertyNode::new("background-color", "css/css_properties.proto", 10);
        bc.affects_paint = true;
        bc.animatable = true;
        bc.shorthand_for = Some("background".into());
        g.insert(bc);

        let mut mt = PropertyNode::new("margin-top", "css/css_properties.proto", 36);
        mt.affects_layout = true;
        g.insert(mt);

        g
    }

    #[test]
    fn test_stats() {
        let g = test_graph();
        let s = g.stats();
        assert_eq!(s.total, 5);
        assert_eq!(s.inherited, 2);      // font-size, line-height
        assert_eq!(s.animatable, 3);     // font-size, line-height, background-color
        assert_eq!(s.layout_triggers, 1); // margin-top
        assert_eq!(s.paint_triggers, 2); // background, background-color
        assert_eq!(s.shorthands, 1);     // background
        assert_eq!(s.longhands, 1);      // background-color
    }

    #[test]
    fn test_query_inherits() {
        let g = test_graph();
        let inherited = g.query().inherits(true).execute_owned();
        assert_eq!(inherited.len(), 2);
        assert!(inherited.contains(&"font-size".to_string()));
        assert!(inherited.contains(&"line-height".to_string()));
    }

    #[test]
    fn test_query_affects_layout() {
        let g = test_graph();
        let layout = g.query().affects_layout(true).execute_owned();
        assert_eq!(layout.len(), 1);
        assert!(layout.contains(&"margin-top".to_string()));
    }

    #[test]
    fn test_query_animatable() {
        let g = test_graph();
        let anim = g.query().animatable(true).execute_owned();
        assert_eq!(anim.len(), 3);
    }

    #[test]
    fn test_expand_shorthand() {
        let g = test_graph();
        let longhands = g.expand_shorthand("background");
        assert_eq!(longhands.len(), 8);
        assert!(longhands.contains(&"background-color"));
    }

    #[test]
    fn test_reverse_deps() {
        let g = test_graph();
        let deps = g.reverse_deps("font-size");
        assert_eq!(deps.len(), 1);
        assert!(deps.contains(&"line-height"));
    }

    #[test]
    fn test_query_combined_filters() {
        let g = test_graph();
        // animatable AND inherits
        let result = g.query().animatable(true).inherits(true).execute_owned();
        assert_eq!(result.len(), 2); // font-size, line-height
    }

    #[test]
    fn test_query_name_contains() {
        let g = test_graph();
        let bg = g.query().name_contains("background").execute_owned();
        assert_eq!(bg.len(), 2); // background, background-color
    }
}
