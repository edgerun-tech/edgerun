use std::path::PathBuf;
use crate::codealyzer::crate_model::*;
use crate::codealyzer::rust_parser;

pub fn build_call_graph(visible_files: &[PathBuf]) -> Vec<CallGraphEdge> {
    let (_items, edges) = rust_parser::analyze_source_files(visible_files);
    edges
}

pub fn generate_dot_graph(edges: &[CallGraphEdge]) -> String {
    let mut dot = String::from("digraph CallGraph {\n");
    dot.push_str("  node [shape=box, style=filled, fillcolor=lightblue];\n");

    for edge in edges {
        let confidence_label = match edge.confidence {
            Confidence::Exact => " [color=green]",
            Confidence::Likely => " [color=blue]",
            Confidence::Ambiguous => " [color=gray, style=dashed]",
        };
        dot.push_str(&format!("  \"{}\" -> \"{}\"{}\n", edge.caller, edge.callee, confidence_label));
    }

    dot.push_str("}\n");
    dot
}
