use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Unified Intermediate Representation (UIR) for code analysis.
///
/// Core data structures representing functions and call relationships
/// across multiple languages.
use crate::edit::{FunctionId, Language, Linkage};

/// How a call edge was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallKind {
    Direct,
    Indirect,
    Macro,
    Unknown,
}

impl CallKind {
    pub fn confidence(&self) -> f32 {
        match self {
            CallKind::Direct => 1.0,
            CallKind::Indirect => 0.3,
            CallKind::Macro => 0.4,
            CallKind::Unknown => 0.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CallKind::Direct => "direct",
            CallKind::Indirect => "indirect",
            CallKind::Macro => "macro",
            CallKind::Unknown => "unknown",
        }
    }
}

/// Represents a single function definition extracted from source code.
#[derive(Debug, Clone)]
pub struct Function {
    pub id: String, // legacy "file::name" for serialization
    pub name: String,
    pub language: String,
    pub file: String,
    pub is_static: bool,
    pub last_modified_commit: Option<String>,
}

impl Function {
    /// Build the canonical FunctionId for this function.
    pub fn function_id(&self) -> FunctionId {
        let lang = self.language.parse().unwrap_or(Language::C);
        let linkage = if self.is_static {
            Linkage::Static
        } else {
            Linkage::Module
        };
        FunctionId::new(lang, &self.file, linkage, &self.name, 0)
    }
}

/// Represents a call relationship between two functions.
#[derive(Debug, Clone)]
pub struct CallEdge {
    pub caller: String,
    pub callee: String,
    pub kind: CallKind,
    #[allow(dead_code)]
    pub confidence: f32,
    #[allow(dead_code)]
    pub source: String,
}

/// The in-memory program representation.
///
/// Functions are keyed by FunctionId for stable cross-language identity.
/// A secondary index maps legacy string IDs to FunctionId for backward
/// compatibility with the viewer and existing code.
#[derive(Debug, Default, Clone)]
pub struct Program {
    pub functions: HashMap<FunctionId, Function>,
    pub edges: Vec<CallEdge>,
    /// Maps legacy "file::name" → FunctionId for O(1) lookup.
    legacy_index: HashMap<String, FunctionId>,
}

impl Program {
    /// Add or replace a function in the UIR.
    pub fn add_function(&mut self, func: Function) {
        let fid = func.function_id();
        self.legacy_index.insert(func.id.clone(), fid.clone());
        self.functions.insert(fid, func);
    }

    /// Look up a function by its legacy string ID.
    #[allow(dead_code)]
    pub fn get_by_legacy(&self, legacy_id: &str) -> Option<&Function> {
        self.legacy_index
            .get(legacy_id)
            .and_then(|fid| self.functions.get(fid))
    }

    /// Remove a function by its legacy string ID. Returns the removed function
    /// and its FunctionId.
    pub fn remove_by_legacy(&mut self, legacy_id: &str) -> Option<(FunctionId, Function)> {
        if let Some(fid) = self.legacy_index.remove(legacy_id) {
            if let Some(func) = self.functions.remove(&fid) {
                return Some((fid, func));
            }
        }
        None
    }

    /// Check if a function exists by legacy ID.
    pub fn contains_legacy(&self, legacy_id: &str) -> bool {
        self.legacy_index.contains_key(legacy_id)
    }

    /// Check if a function would collide by legacy ID.
    pub fn contains_legacy_id(&self, legacy_id: &str) -> bool {
        self.legacy_index.contains_key(legacy_id)
    }

    /// Iterate over functions in legacy-ID order (for stable output).
    #[allow(dead_code)]
    pub fn functions_ordered(&self) -> Vec<&Function> {
        let mut funcs: Vec<_> = self.functions.values().collect();
        funcs.sort_by(|a, b| a.id.cmp(&b.id));
        funcs
    }

    /// Add a call edge to the graph.
    pub fn add_edge(&mut self, edge: CallEdge) {
        self.edges.push(edge);
    }

    /// Remove all edges involving a given legacy ID.
    #[allow(dead_code)]
    pub fn remove_edges_for(&mut self, legacy_id: &str) -> Vec<CallEdge> {
        let (kept, removed): (Vec<_>, Vec<_>) = self
            .edges
            .drain(..)
            .partition(|e| e.caller != legacy_id && e.callee != legacy_id);
        self.edges = kept;
        removed
    }

    /// Get all edges where the given function is the callee.
    #[allow(dead_code)]
    pub fn get_callers(&self, callee_id: &str) -> Vec<&CallEdge> {
        self.edges
            .iter()
            .filter(|e| e.callee == callee_id)
            .collect()
    }

    /// Get all edges where the given function is the caller.
    #[allow(dead_code)]
    pub fn get_callees(&self, caller_id: &str) -> Vec<&CallEdge> {
        self.edges
            .iter()
            .filter(|e| e.caller == caller_id)
            .collect()
    }

    /// Count how many times each function is called.
    #[allow(dead_code)]
    pub fn compute_call_counts(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for edge in &self.edges {
            *counts.entry(edge.callee.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Connectivity = incoming + outgoing edges for a function.
    #[allow(dead_code)]
    pub fn compute_connectivity(&self) -> HashMap<String, usize> {
        let mut connectivity: HashMap<String, usize> = HashMap::new();
        for edge in &self.edges {
            *connectivity.entry(edge.caller.clone()).or_insert(0) += 1;
            *connectivity.entry(edge.callee.clone()).or_insert(0) += 1;
        }
        connectivity
    }
}
