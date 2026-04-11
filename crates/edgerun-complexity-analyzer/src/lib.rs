//! Layer 16: CSS Complexity Analyzer
//!
//! Scores any stylesheet on a "complexity index" — how hard it will be
//! to maintain, debug, and render. Built on the conformance test data
//! (#1), cascade debugger (#7), and property knowledge graph (#9).
//! No other browser can do this — their style data is scattered across
//! 47 C++ files and 12 IDL files. Ours is one query over proto definitions.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use edgerun_property_graph::PropertyGraph;

/// Full complexity analysis of a stylesheet.
#[derive(Debug)]
pub struct ComplexityReport {
    pub overall_score: f64,           // 0-10, higher = more complex
    pub specificity_score: f64,       // 0-10
    pub cascade_depth_score: f64,     // 0-10
    pub dead_rules_pct: f64,          // percentage
    pub duplicate_count: usize,
    pub layout_trigger_pct: f64,      // percentage of properties that cause reflow
    pub inheritance_pct: f64,         // percentage of properties that are inherited
    pub issues: Vec<ComplexityIssue>,
}

#[derive(Debug)]
pub struct ComplexityIssue {
    pub category: &'static str,
    pub severity: &'static str,
    pub message: String,
    pub suggestion: String,
}

/// A parsed CSS rule for analysis.
#[derive(Clone, Debug)]
pub struct CssRule {
    pub selector: String,
    pub specificity_a: u32, // IDs
    pub specificity_b: u32, // classes
    pub specificity_c: u32, // elements
    pub declarations: BTreeMap<String, String>,
}

/// A DOM element for matching against rules.
#[derive(Clone, Debug)]
pub struct DomElement {
    pub tag: String,
    pub classes: BTreeSet<String>,
    pub id: Option<String>,
}

/// Analyze a stylesheet against a DOM tree.
pub fn analyze(
    rules: &[CssRule],
    elements: &[DomElement],
    graph: &PropertyGraph,
) -> ComplexityReport {
    let mut issues = Vec::new();

    // ── 1. Specificity Analysis ──
    let max_a = rules.iter().map(|r| r.specificity_a).max().unwrap_or(0);
    let max_b = rules.iter().map(|r| r.specificity_b).max().unwrap_or(0);
    let max_c = rules.iter().map(|r| r.specificity_c).max().unwrap_or(0);
    let max_specificity = max_a * 100 + max_b * 10 + max_c;

    let specificity_score = if max_specificity > 30 { 10.0 }
        else if max_specificity > 20 { 8.0 }
        else if max_specificity > 10 { 5.0 }
        else { 2.0 };

    if max_a > 0 {
        issues.push(ComplexityIssue {
            category: "specificity",
            severity: "warning",
            message: format!("ID selectors used (specificity A={}). Avoid IDs for easier overrides.", max_a),
            suggestion: "Use classes instead of IDs for styling".into(),
        });
    }
    if max_b > 2 {
        issues.push(ComplexityIssue {
            category: "specificity",
            severity: "warning",
            message: format!("Triple-class selectors found (max B={}).", max_b),
            suggestion: "Simplify selectors to single class or element+class".into(),
        });
    }

    // ── 2. Cascade Depth Analysis ──
    let mut rules_per_element: Vec<usize> = Vec::new();
    for elem in elements {
        let matching = rules.iter().filter(|r| matches_selector(r, elem)).count();
        rules_per_element.push(matching);
    }
    let avg_rules = if elements.is_empty() { 0.0 } else {
        rules_per_element.iter().sum::<usize>() as f64 / elements.len() as f64
    };

    let cascade_depth_score = if avg_rules > 8.0 { 10.0 }
        else if avg_rules > 5.0 { 7.0 }
        else if avg_rules > 3.0 { 4.0 }
        else { 2.0 };

    // ── 3. Dead Rules (match no elements) ──
    let mut dead_count = 0;
    for rule in rules {
        let matches_any = elements.iter().any(|e| matches_selector(rule, e));
        if !matches_any {
            dead_count += 1;
        }
    }
    let dead_pct = if rules.is_empty() { 0.0 } else {
        dead_count as f64 / rules.len() as f64 * 100.0
    };
    if dead_pct > 20.0 {
        issues.push(ComplexityIssue {
            category: "dead-rules",
            severity: "warning",
            message: format!("{:.0}% of rules match no elements in the DOM ({} dead rules)",
                dead_pct, dead_count),
            suggestion: "Remove unused rules or update the DOM to match".into(),
        });
    }

    // ── 4. Duplicate Detection ──
    let mut all_declarations: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for rule in rules {
        for (prop, val) in &rule.declarations {
            let key = (rule.selector.clone(), prop.clone());
            all_declarations.entry(key).or_default().push(val.clone());
        }
    }
    let mut duplicates = 0;
    for ((_selector, _prop), values) in &all_declarations {
        if values.len() > 1 {
            // Check if they're actually different values
            let unique: HashSet<_> = values.iter().collect();
            if unique.len() < values.len() {
                duplicates += 1;
            }
        }
    }
    if duplicates > 5 {
        issues.push(ComplexityIssue {
            category: "duplicates",
            severity: "info",
            message: format!("{} properties declared multiple times on same selector", duplicates),
            suggestion: "Merge duplicate declarations into single rule".into(),
        });
    }

    // ── 5. Layout Triggers ──
    let mut layout_count = 0;
    let mut total_props = 0;
    let mut inherited_count = 0;
    for rule in rules {
        for prop in rule.declarations.keys() {
            total_props += 1;
            if let Some(p) = graph.get(prop) {
                if p.affects_layout { layout_count += 1; }
                if p.inherits { inherited_count += 1; }
            }
        }
    }
    let layout_pct = if total_props == 0 { 0.0 } else {
        layout_count as f64 / total_props as f64 * 100.0
    };
    let inheritance_pct = if total_props == 0 { 0.0 } else {
        inherited_count as f64 / total_props as f64 * 100.0
    };

    if layout_pct > 50.0 {
        issues.push(ComplexityIssue {
            category: "layout-triggers",
            severity: "warning",
            message: format!("{:.0}% of properties trigger layout recalculation", layout_pct),
            suggestion: "Use transform/opacity for animations instead of layout properties".into(),
        });
    }

    // ── Overall Score ──
    let overall_score = (
        specificity_score * 0.25 +
        cascade_depth_score * 0.25 +
        (dead_pct / 10.0).min(10.0) * 0.20 +
        (layout_pct / 10.0).min(10.0) * 0.15 +
        (duplicates as f64 / 2.0).min(10.0) * 0.15
    ).min(10.0);

    ComplexityReport {
        overall_score,
        specificity_score,
        cascade_depth_score,
        dead_rules_pct: dead_pct,
        duplicate_count: duplicates,
        layout_trigger_pct: layout_pct,
        inheritance_pct,
        issues,
    }
}

fn matches_selector(rule: &CssRule, elem: &DomElement) -> bool {
    let sel = rule.selector.to_lowercase();

    // ID match
    if sel.starts_with('#') {
        let id = sel.split_whitespace().next().unwrap_or("").trim_start_matches('#');
        if elem.id.as_deref() != Some(id) { return false; }
    }

    // Class match
    if sel.contains('.') {
        let classes_in_selector: HashSet<_> = sel.split('.')
            .filter(|s| !s.is_empty())
            .map(|s| s.split_whitespace().next().unwrap_or("").to_lowercase())
            .collect();
        let elem_classes: HashSet<_> = elem.classes.iter().cloned().collect();
        if !classes_in_selector.is_subset(&elem_classes) { return false; }
    }

    // Tag match
    if !sel.starts_with('#') && !sel.starts_with('.') {
        let tag = sel.split_whitespace().next().unwrap_or("").split('.').next().unwrap_or("");
        if !tag.is_empty() && tag != elem.tag { return false; }
    }

    true
}

impl ComplexityReport {
    /// Print a human-readable complexity report.
    pub fn print_summary(&self) {
        println!("┌─ CSS Complexity Report ──────────────────────────");
        println!("│ Overall Score:   {:.1}/10", self.overall_score);
        println!("│");
        println!("│ Specificity:     {:.1}/10  (bar: {})", self.specificity_score,
                 bar(self.specificity_score));
        println!("│ Cascade Depth:   {:.1}/10  (bar: {})", self.cascade_depth_score,
                 bar(self.cascade_depth_score));
        println!("│ Dead Rules:      {:.0}%      (bar: {})", self.dead_rules_pct,
                 bar(self.dead_rules_pct / 10.0));
        println!("│ Duplicates:      {}          (bar: {})", self.duplicate_count,
                 bar(self.duplicate_count as f64 / 2.0));
        println!("│ Layout Triggers: {:.0}%      (bar: {})", self.layout_trigger_pct,
                 bar(self.layout_trigger_pct / 10.0));
        println!("│ Inheritance:     {:.0}%      (bar: {})", self.inheritance_pct,
                 bar(self.inheritance_pct / 10.0));
        println!("│");
        if !self.issues.is_empty() {
            println!("│ Issues:");
            for issue in &self.issues {
                let icon = match issue.severity {
                    "error" => "❌",
                    "warning" => "⚠️ ",
                    _ => "ℹ️ ",
                };
                println!("│   {} {}: {}", icon, issue.category, issue.message);
            }
        } else {
            println!("│ No issues found ✅");
        }
        println!("└───────────────────────────────────────────────────");
    }
}

fn bar(score: f64) -> String {
    let filled = (score / 10.0 * 10.0).round() as usize;
    format!("{}{}", "█".repeat(filled.min(10)), "░".repeat(10 - filled.min(10)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rules() -> Vec<CssRule> {
        vec![
            CssRule {
                selector: "p".into(),
                specificity_a: 0, specificity_b: 0, specificity_c: 1,
                declarations: [("color".into(), "#666".into()), ("font-size".into(), "14px".into())].into_iter().collect(),
            },
            CssRule {
                selector: "h1".into(),
                specificity_a: 0, specificity_b: 0, specificity_c: 1,
                declarations: [("font-size".into(), "32px".into()), ("color".into(), "red".into())].into_iter().collect(),
            },
            CssRule {
                selector: "#main .content .title".into(),
                specificity_a: 1, specificity_b: 2, specificity_c: 0,
                declarations: [("color".into(), "blue".into())].into_iter().collect(),
            },
            CssRule {
                selector: ".nonexistent".into(),
                specificity_a: 0, specificity_b: 1, specificity_c: 0,
                declarations: [("display".into(), "none".into())].into_iter().collect(),
            },
        ]
    }

    fn make_elements() -> Vec<DomElement> {
        vec![
            DomElement { tag: "p".into(), classes: BTreeSet::new(), id: None },
            DomElement { tag: "h1".into(), classes: BTreeSet::new(), id: None },
            DomElement { tag: "div".into(), classes: ["content".into()].into_iter().collect(), id: Some("main".into()) },
        ]
    }

    #[test]
    fn test_simple_stylesheet() {
        let graph = PropertyGraph::new();
        let rules = make_rules();
        let elements = make_elements();
        let report = analyze(&rules, &elements, &graph);

        // Should have some dead rules (".nonexistent" matches nothing)
        assert!(report.dead_rules_pct > 0.0);
        // ID selector should trigger a warning
        assert!(report.issues.iter().any(|i| i.category == "specificity"));
        // Overall score should be moderate
        assert!(report.overall_score > 0.0 && report.overall_score <= 10.0);
    }

    #[test]
    fn test_complexity_scoring() {
        let graph = PropertyGraph::new();
        // Simple: 2 rules, no IDs, no dead rules
        let rules = vec![
            CssRule {
                selector: "p".into(),
                specificity_a: 0, specificity_b: 0, specificity_c: 1,
                declarations: [("color".into(), "black".into())].into_iter().collect(),
            },
        ];
        let elements = vec![
            DomElement { tag: "p".into(), classes: BTreeSet::new(), id: None },
        ];
        let report = analyze(&rules, &elements, &graph);

        // Simple stylesheet should have low score
        assert!(report.overall_score < 5.0, "Simple stylesheet score: {:.1}", report.overall_score);
        assert_eq!(report.dead_rules_pct, 0.0);
    }

    #[test]
    fn test_bar_formatting() {
        assert_eq!(bar(0.0), "░░░░░░░░░░");
        assert_eq!(bar(10.0), "██████████");
        assert_eq!(bar(5.0), "█████░░░░░");
    }
}
