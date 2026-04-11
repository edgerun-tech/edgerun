//! Fluent query API for the property graph.

use alloc::vec::Vec;
use alloc::string::String;
use crate::property::CssProperty;
use crate::PropertyGraph;

/// A fluent query builder for CSS properties.
pub struct PropertyQuery<'a> {
    graph: &'a PropertyGraph,
    filter_layout: Option<bool>,
    filter_paint: Option<bool>,
    filter_inherits: Option<bool>,
    filter_animatable: Option<bool>,
    filter_proto_file: Option<String>,
    filter_value_syntax: Option<String>,
    limit: Option<usize>,
}

impl<'a> PropertyQuery<'a> {
    pub fn new(graph: &'a PropertyGraph) -> Self {
        Self { graph, filter_layout: None, filter_paint: None, filter_inherits: None, filter_animatable: None, filter_proto_file: None, filter_value_syntax: None, limit: None }
    }
    pub fn affects_layout(mut self, v: bool) -> Self { self.filter_layout = Some(v); self }
    pub fn affects_paint(mut self, v: bool) -> Self { self.filter_paint = Some(v); self }
    pub fn inherits(mut self, v: bool) -> Self { self.filter_inherits = Some(v); self }
    pub fn animatable(mut self, v: bool) -> Self { self.filter_animatable = Some(v); self }
    pub fn from_proto(mut self, f: &str) -> Self { self.filter_proto_file = Some(f.into()); self }
    pub fn value_syntax_contains(mut self, s: &str) -> Self { self.filter_value_syntax = Some(s.into()); self }
    pub fn limit(mut self, n: usize) -> Self { self.limit = Some(n); self }

    pub fn execute(&self) -> Vec<&CssProperty> {
        let mut r: Vec<&CssProperty> = self.graph.properties.values().filter(|p| {
            if let Some(v) = self.filter_layout { if p.affects_layout != v { return false; } }
            if let Some(v) = self.filter_paint { if p.affects_paint != v { return false; } }
            if let Some(v) = self.filter_inherits { if p.inherits != v { return false; } }
            if let Some(v) = self.filter_animatable { if p.animatable != v { return false; } }
            if let Some(f) = &self.filter_proto_file { if !p.proto_file.contains(f.as_str()) { return false; } }
            if let Some(s) = &self.filter_value_syntax { if !p.value_syntax.contains(s.as_str()) { return false; } }
            true
        }).collect();
        r.sort_by(|a, b| a.name.cmp(&b.name));
        if let Some(n) = self.limit { r.truncate(n); }
        r
    }

    pub fn names(&self) -> Vec<String> {
        self.execute().iter().map(|p| p.name.clone()).collect()
    }
}
