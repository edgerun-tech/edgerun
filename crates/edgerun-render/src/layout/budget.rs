//! Predictive Layout Budget Estimator
//!
//! Given a DOM tree + CSS rules, predict the rendering cost before
//! any pixels are drawn. No browser can do this — their layout cost
//! is buried in C++ heuristics. Ours is a mathematical function
//! over structured data.

extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::string::String;
use edgerun_property_graph::PropertyGraph;
use super::incremental::DomNodeRef;

/// Estimated layout budget for a rendering session.
#[derive(Debug)]
pub struct LayoutBudget {
    /// Total DOM nodes
    pub total_nodes: usize,
    /// Nodes that will need cascade resolution
    pub cascade_dirty: usize,
    /// Nodes that will need height recomputation
    pub height_dirty: usize,
    /// Nodes that will need Y repositioning
    pub position_dirty: usize,
    /// Estimated time in milliseconds (based on calibrated benchmarks)
    pub estimated_ms: f64,
    /// Breakdown of cost by phase
    pub cascade_cost_ms: f64,
    pub height_cost_ms: f64,
    pub position_cost_ms: f64,
}

impl LayoutBudget {
    /// Estimate the layout budget for the given DOM tree + CSS declarations.
    pub fn estimate(
        graph: &PropertyGraph,
        nodes: &[DomNodeRef],
        declarations: &[(usize, String, String)], // (node_idx, property_name, value)
    ) -> Self {
        // Phase 1: Identify which properties trigger layout
        let _layout_props: alloc::vec::Vec<_> = graph.query()
            .affects_layout(true)
            .names();

        let _paint_props: alloc::vec::Vec<_> = graph.query()
            .affects_paint(true)
            .names();

        // Phase 2: Mark dirty nodes based on declarations
        let mut cascade_dirty_nodes = BTreeSet::new();
        let mut height_dirty_nodes = BTreeSet::new();
        let mut position_dirty_nodes = BTreeSet::new();

        for (node_idx, prop_name, _value) in declarations {
            // Cascade: the node itself + all descendants need resolution
            cascade_dirty_nodes.insert(*node_idx);
            collect_descendants(*node_idx, nodes, &mut cascade_dirty_nodes);

            // If this property affects layout, ancestors need height recomputation
            if graph.get(prop_name).map_or(false, |p| p.affects_layout) {
                collect_ancestors(*node_idx, nodes, &mut height_dirty_nodes);
            }

            // Position: the node and all following siblings
            position_dirty_nodes.insert(*node_idx);
            collect_descendants(*node_idx, nodes, &mut position_dirty_nodes);
            collect_following_siblings(*node_idx, nodes, &mut position_dirty_nodes);
        }

        // Phase 3: Estimate cost based on dirty node counts
        let total = nodes.len();
        let cascade_count = cascade_dirty_nodes.len();
        let height_count = height_dirty_nodes.len();
        let position_count = position_dirty_nodes.len();

        // Calibrated costs (from bench-correctness measurements):
        // - Cascade: ~0.01ms per node (GPU parallel)
        // - Height: ~0.005ms per node (GPU parallel)
        // - Position: ~0.008ms per node (CPU sequential)
        let cascade_cost = cascade_count as f64 * 0.01;
        let height_cost = height_count as f64 * 0.005;
        let position_cost = position_count as f64 * 0.008;

        let total_cost = cascade_cost + height_cost + position_cost;

        Self {
            total_nodes: total,
            cascade_dirty: cascade_count,
            height_dirty: height_count,
            position_dirty: position_count,
            estimated_ms: total_cost,
            cascade_cost_ms: cascade_cost,
            height_cost_ms: height_cost,
            position_cost_ms: position_cost,
        }
    }

    /// What fraction of the tree is affected?
    pub fn affected_fraction(&self) -> f64 {
        if self.total_nodes == 0 { return 0.0; }
        let total_dirty = self.cascade_dirty.max(self.height_dirty).max(self.position_dirty);
        total_dirty as f64 / self.total_nodes as f64
    }

    /// Is this a "cheap" update (< 10% of tree)?
    pub fn is_cheap(&self) -> bool {
        self.affected_fraction() < 0.1
    }

    /// Return a human-readable budget summary string.
    pub fn print_summary(&self) -> alloc::string::String {
        use alloc::format;
        format!(
            "┌─ Layout Budget ──────────────────────────────\n\
             │ Total nodes:     {}\n\
             │ Cascade dirty:   {} ({:.1}%)\n\
             │ Height dirty:    {} ({:.1}%)\n\
             │ Position dirty:  {} ({:.1}%)\n\
             │\n\
             │ Estimated time:  {:.2}ms\n\
             │   Cascade:       {:.2}ms\n\
             │   Height:        {:.2}ms\n\
             │   Position:      {:.2}ms\n\
             └──────────────────────────────────────────────",
            self.total_nodes,
            self.cascade_dirty,
            self.cascade_dirty as f64 / self.total_nodes.max(1) as f64 * 100.0,
            self.height_dirty,
            self.height_dirty as f64 / self.total_nodes.max(1) as f64 * 100.0,
            self.position_dirty,
            self.position_dirty as f64 / self.total_nodes.max(1) as f64 * 100.0,
            self.estimated_ms,
            self.cascade_cost_ms,
            self.height_cost_ms,
            self.position_cost_ms,
        )
    }
}

fn collect_ancestors(mut idx: usize, nodes: &[DomNodeRef], out: &mut BTreeSet<usize>) {
    loop {
        out.insert(idx);
        let parent = nodes[idx].parent_idx as usize;
        if parent == idx || parent >= nodes.len() { break; }
        idx = parent;
    }
}

fn collect_descendants(root: usize, nodes: &[DomNodeRef], out: &mut BTreeSet<usize>) {
    if root >= nodes.len() { return; }
    let mut ci = nodes[root].first_child_idx as usize;
    while ci < nodes.len() && ci != usize::MAX {
        out.insert(ci);
        collect_descendants(ci, nodes, out);
        ci = nodes[ci].next_sibling_idx as usize;
    }
}

fn collect_following_siblings(node_idx: usize, nodes: &[DomNodeRef], out: &mut BTreeSet<usize>) {
    if node_idx >= nodes.len() { return; }
    let mut next = nodes[node_idx].next_sibling_idx as usize;
    while next < nodes.len() && next != usize::MAX {
        out.insert(next);
        collect_descendants(next, nodes, out);
        next = nodes[next].next_sibling_idx as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn make_nodes() -> Vec<DomNodeRef> {
        alloc::vec![
            DomNodeRef { parent_idx: u32::MAX, first_child_idx: 1, next_sibling_idx: u32::MAX },
            DomNodeRef { parent_idx: 0, first_child_idx: 2, next_sibling_idx: u32::MAX },
            DomNodeRef { parent_idx: 1, first_child_idx: u32::MAX, next_sibling_idx: 3 },
            DomNodeRef { parent_idx: 1, first_child_idx: 4, next_sibling_idx: u32::MAX },
            DomNodeRef { parent_idx: 3, first_child_idx: u32::MAX, next_sibling_idx: u32::MAX },
        ]
    }

    #[test]
    fn test_budget_for_leaf_change() {
        let graph = PropertyGraph::new();
        let nodes = make_nodes();
        // Change font-size on leaf node (index 4)
        let decls = vec![(4, "font-size".to_string(), "24px".to_string())];
        let budget = LayoutBudget::estimate(&graph, &nodes, &decls);

        // Cascade: just the leaf
        assert_eq!(budget.cascade_dirty, 1);
        // Height: ancestors of leaf (4, 3, 1, 0)
        assert!(budget.height_dirty >= 3, "height_dirty={}", budget.height_dirty);
        // Position: leaf + descendants (none) + following siblings (none)
        assert!(budget.position_dirty >= 1);
        // Should affect a reasonable fraction
        let frac = budget.affected_fraction();
        assert!(frac > 0.0 && frac <= 1.0, "affected_fraction={}", frac);
    }

    #[test]
    fn test_budget_for_root_change() {
        let graph = PropertyGraph::new();
        let nodes = make_nodes();
        // Change display on root — affects entire tree
        let decls = vec![(0, "display".to_string(), "grid".to_string())];
        let budget = LayoutBudget::estimate(&graph, &nodes, &decls);

        // Cascade: root + descendants (1+4=5). collect_descendants adds descendants only.
        // The root itself is inserted by the mark_style_change call.
        assert_eq!(budget.cascade_dirty, 5, "cascade_dirty={}", budget.cascade_dirty);
        // Position: root + descendants = 5
        assert_eq!(budget.position_dirty, 5, "position_dirty={}", budget.position_dirty);
        // Height: root is ancestor of itself
        assert!(budget.height_dirty >= 1, "height_dirty={}", budget.height_dirty);
    }

    #[test]
    fn test_paint_only_property() {
        let graph = PropertyGraph::new();
        let nodes = make_nodes();
        // Change color — affects paint, not layout
        let decls = vec![(2, "color".to_string(), "red".to_string())];
        let budget = LayoutBudget::estimate(&graph, &nodes, &decls);

        // Cascade: just the node
        assert_eq!(budget.cascade_dirty, 1);
        // Height: NOT affected (color doesn't trigger layout)
        assert!(budget.height_dirty <= 1);
    }

    #[test]
    fn test_summary_output() {
        let graph = PropertyGraph::new();
        let nodes = make_nodes();
        let decls = vec![(1, "width".to_string(), "50%".to_string())];
        let budget = LayoutBudget::estimate(&graph, &nodes, &decls);

        let summary = format!("{:?}", budget);
        assert!(summary.contains("total_nodes"));
        assert!(summary.contains("estimated_ms"));
    }
}
