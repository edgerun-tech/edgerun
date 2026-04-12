//! Incremental Layout Engine
//!
//! When one CSS property changes on one element, only recompute the affected
//! subtree — not the entire page.
//!
//! For a single font-size change on one element:
//!   - Full relayout: O(N) GPU dispatches
//!   - Incremental: O(affected_subtree) — typically 5-20 nodes

extern crate alloc;
use alloc::collections::BTreeSet;

#[derive(Clone, Copy, Debug)]
pub struct DomNodeRef {
    pub parent_idx: u32,
    pub first_child_idx: u32,
    pub next_sibling_idx: u32,
}

#[derive(Clone, Debug, Default)]
pub struct DirtySet {
    pub cascade_dirty: BTreeSet<u32>,
    pub height_dirty: BTreeSet<u32>,
    pub position_dirty: BTreeSet<u32>,
}

impl DirtySet {
    pub fn new() -> Self { Self::default() }

    pub fn mark_style_change(&mut self, node_idx: u32, nodes: &[DomNodeRef], affects_height: bool) {
        self.cascade_dirty.insert(node_idx);
        self.position_dirty.insert(node_idx);
        collect_descendants(node_idx, nodes, &mut self.cascade_dirty);
        collect_descendants(node_idx, nodes, &mut self.position_dirty);
        if affects_height {
            collect_ancestors(node_idx, nodes, &mut self.height_dirty);
        }
        collect_following_siblings(node_idx, nodes, &mut self.position_dirty);
    }

    pub fn total_affected(&self) -> usize {
        let mut all: BTreeSet<u32> = BTreeSet::new();
        all.extend(&self.cascade_dirty);
        all.extend(&self.height_dirty);
        all.extend(&self.position_dirty);
        all.len()
    }

    pub fn affected_fraction(&self, total: usize) -> f64 {
        if total == 0 { return 0.0; }
        self.total_affected() as f64 / total as f64
    }

    pub fn clear(&mut self) {
        self.cascade_dirty.clear();
        self.height_dirty.clear();
        self.position_dirty.clear();
    }
}

fn collect_descendants(root: u32, nodes: &[DomNodeRef], out: &mut BTreeSet<u32>) {
    if root >= nodes.len() as u32 { return; }
    let mut ci = nodes[root as usize].first_child_idx;
    while ci != u32::MAX && ci < nodes.len() as u32 {
        out.insert(ci);
        collect_descendants(ci, nodes, out);
        ci = nodes[ci as usize].next_sibling_idx;
    }
}

fn collect_ancestors(mut idx: u32, nodes: &[DomNodeRef], out: &mut BTreeSet<u32>) {
    while idx < nodes.len() as u32 {
        out.insert(idx);
        let parent = nodes[idx as usize].parent_idx;
        if parent == idx { break; }
        idx = parent;
    }
}

fn collect_following_siblings(node_idx: u32, nodes: &[DomNodeRef], out: &mut BTreeSet<u32>) {
    if node_idx >= nodes.len() as u32 { return; }
    let mut next = nodes[node_idx as usize].next_sibling_idx;
    while next != u32::MAX && next < nodes.len() as u32 {
        out.insert(next);
        collect_descendants(next, nodes, out);
        next = nodes[next as usize].next_sibling_idx;
    }
}

pub fn speedup_ratio(dirty: &DirtySet, total: usize) -> f64 {
    let affected = dirty.total_affected();
    if affected == 0 { return 0.0; }
    total as f64 / affected as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_nodes() -> Vec<DomNodeRef> {
        vec![
            DomNodeRef { parent_idx: u32::MAX, first_child_idx: 1, next_sibling_idx: u32::MAX },
            DomNodeRef { parent_idx: 0, first_child_idx: 2, next_sibling_idx: u32::MAX },
            DomNodeRef { parent_idx: 1, first_child_idx: u32::MAX, next_sibling_idx: 3 },
            DomNodeRef { parent_idx: 1, first_child_idx: 4, next_sibling_idx: u32::MAX },
            DomNodeRef { parent_idx: 3, first_child_idx: u32::MAX, next_sibling_idx: u32::MAX },
        ]
    }

    #[test]
    fn test_leaf_change() {
        let nodes = make_nodes();
        let mut dirty = DirtySet::new();
        dirty.mark_style_change(4, &nodes, true);
        assert!(dirty.cascade_dirty.contains(&4));
        assert!(dirty.height_dirty.contains(&0));
        assert!(dirty.height_dirty.contains(&1));
        assert!(dirty.height_dirty.contains(&3));
        assert!(dirty.height_dirty.contains(&4));
        assert!(dirty.position_dirty.contains(&4));
    }

    #[test]
    fn test_root_change() {
        let nodes = make_nodes();
        let mut dirty = DirtySet::new();
        dirty.mark_style_change(0, &nodes, true);
        assert!(dirty.cascade_dirty.contains(&0));
        assert!(dirty.cascade_dirty.contains(&1));
        assert!(dirty.cascade_dirty.contains(&2));
        assert!(dirty.cascade_dirty.contains(&3));
        assert!(dirty.cascade_dirty.contains(&4));
    }

    #[test]
    fn test_sibling_isolation() {
        let nodes = make_nodes();
        let mut dirty = DirtySet::new();
        dirty.mark_style_change(3, &nodes, true);
        assert!(!dirty.cascade_dirty.contains(&2)); // prior sibling not affected
        assert!(dirty.cascade_dirty.contains(&3));
        assert!(dirty.cascade_dirty.contains(&4)); // child of span
    }
}
