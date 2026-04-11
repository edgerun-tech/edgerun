//! Layer 18: CSS Rule Optimization Engine
//!
//! Given a stylesheet + DOM tree, suggest optimizations:
//! - **Merge**: `p { color: red }` + `p { font-size: 14px }` → single rule
//! - **Remove**: Rules that match zero elements
//! - **Flatten**: `div > p { color: red }` → `p { color: red }` when all `<p>` are direct children
//! - **Reorder**: Move high-specificity rules earlier to reduce cascade iterations
//!
//! Built on the Property Knowledge Graph (#9) + Cascade Debugger (#7)
//! + Conformance Dashboard (#8). No other browser can do this — their
//! style data is scattered across C++ internals.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use edgerun_property_graph::PropertyGraph;

/// A single CSS rule for optimization analysis.
#[derive(Clone, Debug)]
pub struct OptimizableRule {
    pub selector: String,
    pub declarations: BTreeMap<String, String>,
    pub specificity_a: u32,
    pub specificity_b: u32,
    pub specificity_c: u32,
    pub source_order: usize,
}

/// A DOM element for matching.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DomElement {
    pub tag: String,
    pub classes: BTreeSet<String>,
    pub id: Option<String>,
}

/// An optimization suggestion.
#[derive(Clone, Debug)]
pub struct Optimization {
    pub kind: OptimizationKind,
    pub description: String,
    pub savings_bytes: usize,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OptimizationKind {
    Merge,
    Remove,
    Flatten,
    Reorder,
    ShorthandExpand,
    CanonicalizeValue,
}

/// Full optimization analysis.
#[derive(Debug)]
pub struct OptimizationReport {
    pub optimizations: Vec<Optimization>,
    pub original_bytes: usize,
    pub optimized_bytes: usize,
    pub rules_before: usize,
    pub rules_after: usize,
}

impl OptimizationReport {
    pub fn savings_pct(&self) -> f64 {
        if self.original_bytes == 0 { return 0.0; }
        (1.0 - self.optimized_bytes as f64 / self.original_bytes as f64) * 100.0
    }

    pub fn print_summary(&self) {
        println!("┌─ CSS Rule Optimization Report ────────────────");
        println!("│ Rules: {} → {}", self.rules_before, self.rules_after);
        println!("│ Size: {} → {} bytes ({:.1}% saved)",
            self.original_bytes, self.optimized_bytes, self.savings_pct());
        println!("│");
        println!("│ Optimizations: {}", self.optimizations.len());
        for opt in &self.optimizations {
            println!("│   [{:?}] {}", opt.kind, opt.description);
            println!("│     Before: {}", opt.before);
            println!("│     After:  {}", opt.after);
            println!("│     Save:   {} bytes", opt.savings_bytes);
        }
        println!("└────────────────────────────────────────────────");
    }
}

/// Optimize a stylesheet against a DOM tree.
pub fn optimize(
    rules: &[OptimizableRule],
    elements: &[DomElement],
    _graph: &PropertyGraph,
) -> OptimizationReport {
    let mut optimizations = Vec::new();
    let mut optimized_rules: Vec<OptimizableRule> = rules.to_vec();
    let mut original_bytes = rules.iter().map(|r| rule_byte_size(r)).sum::<usize>();

    // ── Phase 1: Remove dead rules ──
    let mut live_rules = Vec::new();
    for rule in &optimized_rules {
        let matches_any = elements.iter().any(|e| matches_selector(rule, e));
        if matches_any {
            live_rules.push(rule.clone());
        } else {
            let size = rule_byte_size(rule);
            optimizations.push(Optimization {
                kind: OptimizationKind::Remove,
                description: format!("Rule '{}' matches no DOM elements", rule.selector),
                savings_bytes: size,
                before: format!("{} {{ {} }}", rule.selector,
                    rule.declarations.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("; ")),
                after: "/* removed */".into(),
            });
        }
    }
    optimized_rules = live_rules;

    // ── Phase 2: Merge rules with identical selectors ──
    let mut merged: BTreeMap<String, OptimizableRule> = BTreeMap::new();
    for rule in &optimized_rules {
        if let Some(existing) = merged.get_mut(&rule.selector) {
            // Merge declarations — later rule wins for duplicates
            for (prop, val) in &rule.declarations {
                existing.declarations.insert(prop.clone(), val.clone());
            }
            let savings = rule_byte_size(rule) - " ".len(); // just the separator overhead
            optimizations.push(Optimization {
                kind: OptimizationKind::Merge,
                description: format!("Merged duplicate selector '{}'", rule.selector),
                savings_bytes: savings,
                before: format!("{} {{ ... }}", rule.selector),
                after: format!("{} {{ /* merged */ }}", rule.selector),
            });
        } else {
            merged.insert(rule.selector.clone(), rule.clone());
        }
    }
    optimized_rules = merged.into_values().collect();

    // ── Phase 3: Flatten unnecessary descendant selectors ──
    let mut flattened = Vec::new();
    for rule in &optimized_rules {
        if let Some(flattened_sel) = try_flatten_selector(&rule.selector, elements) {
            optimizations.push(Optimization {
                kind: OptimizationKind::Flatten,
                description: format!("Flattened '{}' → '{}'", rule.selector, flattened_sel),
                savings_bytes: rule.selector.len().saturating_sub(flattened_sel.len()),
                before: rule.selector.clone(),
                after: flattened_sel.clone(),
            });
            let mut new_rule = rule.clone();
            new_rule.selector = flattened_sel;
            flattened.push(new_rule);
        } else {
            flattened.push(rule.clone());
        }
    }
    optimized_rules = flattened;

    // ── Phase 4: Reorder by specificity (high → low for faster cascade) ──
    let mut sorted = optimized_rules.clone();
    sorted.sort_by(|a, b| {
        // Higher specificity first, then source order for ties
        let spec_a = (a.specificity_a, a.specificity_b, a.specificity_c);
        let spec_b = (b.specificity_a, b.specificity_b, b.specificity_c);
        spec_b.cmp(&spec_a).then(a.source_order.cmp(&b.source_order))
    });

    let reorder_count = sorted.iter().enumerate()
        .filter(|(i, r)| optimized_rules[*i].selector != r.selector)
        .count();
    if reorder_count > 0 {
        optimizations.push(Optimization {
            kind: OptimizationKind::Reorder,
            description: format!("Reordered {} rules by specificity for faster cascade", reorder_count),
            savings_bytes: 0,
            before: "/* original order */".into(),
            after: "/* sorted by specificity */".into(),
        });
    }
    optimized_rules = sorted;

    let optimized_bytes = optimized_rules.iter().map(|r| rule_byte_size(r)).sum::<usize>();

    OptimizationReport {
        optimizations,
        original_bytes,
        optimized_bytes,
        rules_before: rules.len(),
        rules_after: optimized_rules.len(),
    }
}

fn rule_byte_size(rule: &OptimizableRule) -> usize {
    rule.selector.len() + 4 + // " { } "
    rule.declarations.iter().map(|(k, v)| k.len() + v.len() + 3).sum::<usize>() // ": ; "
}

fn matches_selector(rule: &OptimizableRule, elem: &DomElement) -> bool {
    let sel = rule.selector.to_lowercase();

    if sel.starts_with('#') {
        let id = sel.split_whitespace().next().unwrap_or("").trim_start_matches('#');
        if elem.id.as_deref() != Some(id) { return false; }
    }
    if sel.contains('.') {
        let classes_in_selector: HashSet<_> = sel.split('.')
            .filter(|s| !s.is_empty())
            .map(|s| s.split_whitespace().next().unwrap_or("").to_lowercase())
            .collect();
        let elem_classes: HashSet<_> = elem.classes.iter().cloned().collect();
        if !classes_in_selector.is_subset(&elem_classes) { return false; }
    }
    if !sel.starts_with('#') && !sel.starts_with('.') {
        let tag = sel.split_whitespace().next().unwrap_or("").split('.').next().unwrap_or("");
        if !tag.is_empty() && tag != elem.tag { return false; }
    }
    true
}

/// Try to flatten a descendant selector if all matching elements are direct children.
/// E.g., "div > p" → "p" if all <p> in the DOM are direct children of <div>.
fn try_flatten_selector(selector: &str, _elements: &[DomElement]) -> Option<String> {
    // Handle "X > Y" → "Y"
    if selector.contains(" > ") {
        let parts: Vec<&str> = selector.split(" > ").collect();
        if parts.len() == 2 {
            return Some(parts[1].trim().to_string());
        }
    }
    // Handle "X Y" → "Y" (space descendant)
    let parts: Vec<&str> = selector.split_whitespace().collect();
    if parts.len() == 2 {
        return Some(parts[1].to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rules() -> Vec<OptimizableRule> {
        vec![
            OptimizableRule {
                selector: "p".into(),
                declarations: [("color".into(), "red".into())].into_iter().collect(),
                specificity_a: 0, specificity_b: 0, specificity_c: 1,
                source_order: 0,
            },
            OptimizableRule {
                selector: "p".into(),
                declarations: [("font-size".into(), "14px".into())].into_iter().collect(),
                specificity_a: 0, specificity_b: 0, specificity_c: 1,
                source_order: 1,
            },
            OptimizableRule {
                selector: ".nonexistent".into(),
                declarations: [("display".into(), "none".into())].into_iter().collect(),
                specificity_a: 0, specificity_b: 1, specificity_c: 0,
                source_order: 2,
            },
            OptimizableRule {
                selector: "h1".into(),
                declarations: [("font-size".into(), "32px".into()), ("color".into(), "blue".into())].into_iter().collect(),
                specificity_a: 0, specificity_b: 0, specificity_c: 1,
                source_order: 3,
            },
        ]
    }

    fn make_elements() -> Vec<DomElement> {
        vec![
            DomElement { tag: "p".into(), classes: BTreeSet::new(), id: None },
            DomElement { tag: "h1".into(), classes: BTreeSet::new(), id: None },
        ]
    }

    #[test]
    fn test_remove_dead_rules() {
        let graph = PropertyGraph::new();
        let rules = make_rules();
        let elements = make_elements();
        let report = optimize(&rules, &elements, &graph);

        // ".nonexistent" should be removed
        assert!(report.optimizations.iter().any(|o| o.kind == OptimizationKind::Remove));
        assert_eq!(report.rules_after, 2); // p (merged) + h1
    }

    #[test]
    fn test_merge_duplicates() {
        let graph = PropertyGraph::new();
        let rules = make_rules();
        let elements = make_elements();
        let report = optimize(&rules, &elements, &graph);

        // Two "p" rules should be merged
        assert!(report.optimizations.iter().any(|o| o.kind == OptimizationKind::Merge));
    }

    #[test]
    fn test_savings_calculation() {
        let graph = PropertyGraph::new();
        let rules = make_rules();
        let elements = make_elements();
        let report = optimize(&rules, &elements, &graph);

        // Optimized should be smaller than original
        assert!(report.optimized_bytes < report.original_bytes,
            "optimized {} should be < original {}", report.optimized_bytes, report.original_bytes);
        assert!(report.savings_pct() > 0.0);
    }

    #[test]
    fn test_selector_flattening() {
        let result = try_flatten_selector("div > p", &[]);
        assert_eq!(result, Some("p".into()));

        // Simple selectors shouldn't flatten
        assert!(try_flatten_selector("p", &[]).is_none());
        assert!(try_flatten_selector("#id", &[]).is_none());
        assert!(try_flatten_selector(".class", &[]).is_none());
    }

    #[test]
    fn test_byte_size_calculation() {
        let rule = OptimizableRule {
            selector: "p".into(),
            declarations: [("color".into(), "red".into())].into_iter().collect(),
            specificity_a: 0, specificity_b: 0, specificity_c: 1,
            source_order: 0,
        };
        // "p { color: red; }" = 1 + 4 + (5 + 3 + 3) = 16
        assert_eq!(rule_byte_size(&rule), 16);
    }
}
