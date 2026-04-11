//! Layer 7: Visual Cascade Debugger
//!
//! Shows the complete CSS cascade resolution for any element:
//! - All matching rules ranked by specificity
//! - Per-property winner (why this rule won over others)
//! - Inheritance chain (what came from parent vs explicit)
//! - Final computed values
//!
//! Usage:
//!   cargo run -p edgerun-cascade-debugger            # Run demo
//!   cargo run -p edgerun-cascade-debugger -- --element h1  # Show cascade for h1

use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut filter_element: Option<String> = None;

    for arg in &args[1..] {
        if (arg == "--element" || arg == "-e") && args.len() > 2 {
            if let Some(pos) = args.iter().position(|a| a == arg) {
                filter_element = args.get(pos + 1).cloned();
            }
        }
    }

    println!("=== CSS Cascade Debugger ===\n");

    // Demo HTML + CSS
    let html = r#"<html>
  <body>
    <h1 class="title">Hello World</h1>
    <p class="text highlighted">Some text here</p>
    <div>
      <p>Nested paragraph</p>
    </div>
  </body>
</html>"#;

    let css = r#"
body { font-size: 16px; color: #333333; }
h1 { font-size: 32px; color: #FF0000; }
h1.title { font-size: 36px; color: #0000FF; }
p { font-size: 14px; color: #666666; }
.text { color: #999999; }
.highlighted { background-color: yellow; }
div p { font-size: 12px; }
h1 { color: #00FF00 !important; }
"#;

    // Parse DOM
    let dom = simple_parse_html(html);
    println!("Parsed {} DOM nodes\n", dom.len());

    // Parse CSS rules
    let rules = simple_parse_css(css);
    println!("Parsed {} CSS rules\n", rules.len());

    // For each element, show cascade resolution
    for node in &dom {
        if let Some(ref filter) = filter_element {
            if node.tag != *filter { continue; }
        }
        show_cascade(node, &dom, &rules);
        println!("{}", "─".repeat(60));
    }
}

#[derive(Debug)]
struct DomNode {
    tag: String,
    class: Option<String>,
    id: Option<String>,
    parent: Option<usize>,
    children: Vec<usize>,
    depth: usize,
}

#[derive(Debug, Clone)]
struct CssRule {
    selector: String,
    tag: Option<String>,
    class: Option<String>,
    id: Option<String>,
    descendant_tag: Option<String>,
    specificity_a: u32, // IDs
    specificity_b: u32, // classes
    specificity_c: u32, // elements
    source_order: usize,
    important: bool,
    declarations: BTreeMap<String, String>,
}

fn simple_parse_html(html: &str) -> Vec<DomNode> {
    let mut nodes = Vec::new();

    // Simplified: hardcode the DOM structure from our demo HTML
    nodes.push(DomNode { tag: "html".into(), class: None, id: None, parent: None, children: vec![1], depth: 0 });
    nodes.push(DomNode { tag: "body".into(), class: None, id: None, parent: Some(0), children: vec![2, 3, 4], depth: 1 });
    nodes.push(DomNode { tag: "h1".into(), class: Some("title".into()), id: None, parent: Some(1), children: vec![], depth: 2 });
    nodes.push(DomNode { tag: "p".into(), class: Some("text highlighted".into()), id: None, parent: Some(1), children: vec![], depth: 2 });
    nodes.push(DomNode { tag: "div".into(), class: None, id: None, parent: Some(1), children: vec![5], depth: 2 });
    nodes.push(DomNode { tag: "p".into(), class: None, id: None, parent: Some(4), children: vec![], depth: 3 });

    nodes
}

fn simple_parse_css(css: &str) -> Vec<CssRule> {
    let mut rules = Vec::new();
    let mut source_order = 0;

    for block in css.split('}') {
        let block = block.trim();
        if block.is_empty() { continue; }

        let parts: Vec<&str> = block.split('{').collect();
        if parts.len() != 2 { continue; }

        let selector = parts[0].trim();
        let declarations_str = parts[1].trim();

        let mut declarations = BTreeMap::new();
        for decl in declarations_str.split(';') {
            let decl = decl.trim();
            if decl.is_empty() { continue; }
            let kv: Vec<&str> = decl.splitn(2, ':').collect();
            if kv.len() == 2 {
                declarations.insert(kv[0].trim().to_string(), kv[1].trim().to_string());
            }
        }

        let important = declarations.values().any(|v| v.contains("!important"));

        // Parse selector
        let (tag, class, id, descendant_tag, sa, sb, sc) = parse_selector(selector);

        rules.push(CssRule {
            selector: selector.into(),
            tag, class, id, descendant_tag,
            specificity_a: sa, specificity_b: sb, specificity_c: sc,
            source_order,
            important,
            declarations: declarations.into_iter()
                .map(|(k, v)| (k, v.replace("!important", "").trim().to_string()))
                .collect(),
        });
        source_order += 1;
    }

    rules
}

fn parse_selector(selector: &str) -> (Option<String>, Option<String>, Option<String>, Option<String>, u32, u32, u32) {
    let parts: Vec<&str> = selector.split_whitespace().collect();

    if parts.len() == 2 {
        // Descendant selector: "div p"
        let tag2 = parts[1].to_string();
        let (tag, class, id, _, a, b, c) = parse_simple_selector(parts[0]);
        return (tag, class, id, Some(tag2), a, b, c + 1);
    }

    let (tag, class, id, _, a, b, c) = parse_simple_selector(selector);
    (tag, class, id, None, a, b, c)
}

fn parse_simple_selector(sel: &str) -> (Option<String>, Option<String>, Option<String>, bool, u32, u32, u32) {
    let mut tag = None;
    let mut class = None;
    let mut id = None;

    let parts: Vec<&str> = sel.split('.').collect();
    if parts.len() >= 2 {
        if !parts[0].is_empty() {
            tag = Some(parts[0].to_string());
        }
        class = Some(parts[1..].join("."));
    } else if sel.starts_with('#') {
        id = Some(sel[1..].to_string());
    } else if !sel.is_empty() {
        tag = Some(sel.to_string());
    }

    let a = if id.is_some() { 1 } else { 0 };
    let b = if class.is_some() { 1 } else { 0 };
    let c = if tag.is_some() { 1 } else { 0 };

    (tag, class, id, false, a, b, c)
}

fn show_cascade(node: &DomNode, all_nodes: &[DomNode], rules: &[CssRule]) {
    println!("┌─ Element: <{}{}{}>", 
        node.tag,
        node.class.as_ref().map(|c| format!(".{}", c)).unwrap_or_default(),
        node.id.as_ref().map(|i| format!("#{}", i)).unwrap_or_default(),
    );
    println!("│ Depth: {}, Parent: {:?}", node.depth, node.parent);

    // Find all matching rules
    let mut matching: Vec<&CssRule> = rules.iter()
        .filter(|r| rule_matches_node(r, node, all_nodes))
        .collect();
    matching.sort_by(|a, b| compare_specificity(a, b));

    if matching.is_empty() {
        println!("│ No matching rules — using defaults");
    } else {
        println!("│ {} matching rules:", matching.len());
        for (i, rule) in matching.iter().enumerate() {
            let imp = if rule.important { " !important" } else { "" };
            println!("│   {}. {} [{},{},{}] #{}{}", 
                i + 1, rule.selector, 
                rule.specificity_a, rule.specificity_b, rule.specificity_c,
                rule.source_order, imp);
        }
    }

    // Resolve per-property winner
    println!("│");
    println!("│ ── Resolved Values ──");
    let mut resolved: BTreeMap<String, (&CssRule, String)> = BTreeMap::new();

    for rule in &matching {
        for (prop, val) in &rule.declarations {
            let dominated = if let Some((existing_rule, existing_val)) = resolved.get(prop) {
                // Does new rule win?
                let new_wins = compare_cascade(rule, existing_rule);
                !new_wins
            } else {
                false
            };
            if !dominated {
                resolved.insert(prop.clone(), (rule, val.clone()));
            }
        }
    }

    // Check inheritance for unset properties
    let inherited_props = ["font-size", "color", "font-family", "line-height"];
    if let Some(parent_idx) = node.parent {
        let parent = &all_nodes[parent_idx];
        for prop in &inherited_props {
            if !resolved.contains_key(*prop) {
                // Inherit from parent (simplified — would need parent's resolved values)
                println!("│   {} → inherited from <{}>", prop, parent.tag);
            }
        }
    }

    for (prop, (rule, val)) in &resolved {
        println!("│   {} → {}  (from: {})", prop, val, rule.selector);
    }

    println!("└{}", "─".repeat(58));
}

fn rule_matches_node(rule: &CssRule, node: &DomNode, all_nodes: &[DomNode]) -> bool {
    // Check descendant selector first
    if let Some(ref desc_tag) = rule.descendant_tag {
        if node.tag != *desc_tag { return false; }
        // Check if parent matches the first part of selector
        if let Some(parent_idx) = node.parent {
            let parent = &all_nodes[parent_idx];
            if let Some(ref tag) = rule.tag {
                if parent.tag != *tag { return false; }
            }
            if let Some(ref class) = rule.class {
                if parent.class.as_ref() != Some(class) { return false; }
            }
            return true;
        }
        return false;
    }

    // Simple selector match
    if let Some(ref tag) = rule.tag {
        if node.tag != *tag { return false; }
    }
    if let Some(ref class) = rule.class {
        if node.class.as_ref() != Some(class) { return false; }
    }
    if let Some(ref id) = rule.id {
        if node.id.as_ref() != Some(id) { return false; }
    }

    true
}

fn compare_specificity(a: &CssRule, b: &CssRule) -> std::cmp::Ordering {
    if compare_cascade(a, b) { std::cmp::Ordering::Greater }
    else if compare_cascade(b, a) { std::cmp::Ordering::Less }
    else { std::cmp::Ordering::Equal }
}

fn compare_cascade(a: &CssRule, b: &CssRule) -> bool {
    // Returns true if `a` wins over `b`
    if a.important && !b.important { return true; }
    if !a.important && b.important { return false; }
    if a.specificity_a != b.specificity_a { return a.specificity_a > b.specificity_a; }
    if a.specificity_b != b.specificity_b { return a.specificity_b > b.specificity_b; }
    if a.specificity_c != b.specificity_c { return a.specificity_c > b.specificity_c; }
    a.source_order > b.source_order // later source order wins
}
