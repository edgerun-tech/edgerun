use std::{
    collections::{HashMap, HashSet},
    fmt,
    hash::Hash,
};

/// Structural editing and incremental analysis for the UIR.
///
/// Four foundational abstractions:
/// 1. FunctionId — stable, cross-language identifiers
/// 2. ChangeSet — the delta between two graph states
/// 3. Edit — structural mutations with precondition checks and undo
/// 4. Analysis<T> — incremental derived views
use crate::uir::{CallKind, Program};

// ─── Language ──────────────────────────────────────────────────────────

/// Supported source languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    C,
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::C => "c",
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Python => "python",
            Language::Go => "go",
            Language::Java => "java",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Language {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "c" => Ok(Language::C),
            "rust" => Ok(Language::Rust),
            "typescript" => Ok(Language::TypeScript),
            "javascript" => Ok(Language::JavaScript),
            "python" => Ok(Language::Python),
            "go" => Ok(Language::Go),
            "java" => Ok(Language::Java),
            _ => Err(format!("unknown language: {s}")),
        }
    }
}

// ─── Linkage ───────────────────────────────────────────────────────────

/// Visibility / linkage of a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Linkage {
    /// Module-scoped: exported, visible from other files.
    Module,
    /// File-scoped: `static` in C, `private fn` in Rust.
    Static,
    /// Public API: externally visible, part of a library interface.
    Public,
}

impl Linkage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Linkage::Module => "module",
            Linkage::Static => "static",
            Linkage::Public => "public",
        }
    }
}

impl fmt::Display for Linkage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Linkage {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "module" => Ok(Linkage::Module),
            "static" => Ok(Linkage::Static),
            "public" => Ok(Linkage::Public),
            _ => Err(format!("unknown linkage: {s}")),
        }
    }
}

// ─── FunctionId ────────────────────────────────────────────────────────

/// Stable, cross-language identifier for a function.
///
/// Format: `<language>:<file>:<linkage>:<name>:<hash>`
/// Example: `c:kernel/sched/fair.c:static:pick_next_task_fair:1234567890`
///
/// The content hash distinguishes functions with the same name in the same
/// file (e.g., C static functions in different compilation units) and
/// survives file renames.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionId {
    pub language: Language,
    pub file: String,
    pub linkage: Linkage,
    pub name: String,
    /// FNV-1a hash of the function body (start_byte..end_byte).
    pub hash: u64,
}

impl FunctionId {
    /// Construct from components.
    pub fn new(
        language: Language,
        file: impl Into<String>,
        linkage: Linkage,
        name: impl Into<String>,
        hash: u64,
    ) -> Self {
        Self {
            language,
            file: file.into(),
            linkage,
            name: name.into(),
            hash,
        }
    }

    /// Build a FunctionId from the legacy "file::name" string format.
    /// Uses a zero hash since legacy IDs don't carry content hashes.
    pub fn from_legacy(legacy_id: &str, language: Language, is_static: bool) -> Self {
        let linkage = if is_static {
            Linkage::Static
        } else {
            Linkage::Module
        };
        if let Some(pos) = legacy_id.rfind("::") {
            let file = legacy_id[..pos].to_string();
            let name = legacy_id[pos + 2..].to_string();
            Self {
                language,
                file,
                linkage,
                name,
                hash: 0,
            }
        } else {
            Self {
                language,
                file: legacy_id.to_string(),
                linkage,
                name: String::new(),
                hash: 0,
            }
        }
    }

    /// Convert back to the legacy "file::name" string for compatibility.
    pub fn to_legacy(&self) -> String {
        format!("{}::{}", self.file, self.name)
    }

    /// Check if this ID matches the given legacy string.
    pub fn matches_legacy(&self, legacy_id: &str) -> bool {
        self.to_legacy() == legacy_id
    }
}

impl fmt::Display for FunctionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}:{}",
            self.language, self.file, self.linkage, self.name, self.hash
        )
    }
}

impl std::str::FromStr for FunctionId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(5, ':').collect();
        if parts.len() != 5 {
            return Err(format!("invalid FunctionId: {s}"));
        }
        let language = parts[0].parse()?;
        let file = parts[1].to_string();
        let linkage = parts[2].parse()?;
        let name = parts[3].to_string();
        let hash = parts[4]
            .parse::<u64>()
            .map_err(|e| format!("invalid hash: {e}"))?;
        Ok(Self {
            language,
            file,
            linkage,
            name,
            hash,
        })
    }
}

// ─── ChangeSet ─────────────────────────────────────────────────────────

/// The delta between two graph states.
///
/// Every edit produces a ChangeSet. The ChangeSet drives:
/// - Incremental viewer updates (added/removed nodes and edges)
/// - Downstream invalidations (who needs to re-check their callers)
/// - Undo (apply the inverse of this ChangeSet)
#[derive(Debug, Clone, Default)]
pub struct ChangeSet {
    /// Functions added to the graph.
    pub added: Vec<(FunctionId, String)>, // (FunctionId, legacy_id)
    /// Functions removed from the graph.
    pub removed: Vec<FunctionId>,
    /// Functions modified (new definition replaces old).
    pub modified: Vec<(FunctionId, String)>, // (FunctionId, new_legacy_id)
    /// Call edges added.
    pub edges_added: Vec<(String, String, CallKind)>, // (caller_legacy, callee_legacy, kind)
    /// Call edges removed.
    pub edges_removed: Vec<(String, String)>, // (caller_legacy, callee_legacy)
    /// Downstream functions whose call edges may be stale because
    /// one of their callees changed. These are NOT the changed
    /// functions themselves — these are their callers.
    pub invalidations: Vec<String>, // legacy_ids
}

impl ChangeSet {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.modified.is_empty()
            && self.edges_added.is_empty()
            && self.edges_removed.is_empty()
            && self.invalidations.is_empty()
    }

    /// Create a ChangeSet from a single added function with its edges.
    pub fn add_function(
        id: FunctionId,
        legacy_id: String,
        edges: Vec<(String, String, CallKind)>,
    ) -> Self {
        Self {
            added: vec![(id, legacy_id.clone())],
            edges_added: edges,
            ..Default::default()
        }
    }

    /// Create a ChangeSet for a removed function and all its edges.
    pub fn remove_function(
        id: FunctionId,
        legacy_id: String,
        caller_edges: Vec<(String, String)>,
        callee_edges: Vec<(String, String)>,
    ) -> Self {
        let mut edges_removed = caller_edges;
        edges_removed.extend(callee_edges);
        Self {
            removed: vec![id],
            edges_removed,
            invalidations: vec![legacy_id],
            ..Default::default()
        }
    }

    /// Merge two ChangeSets.
    pub fn merge(mut self, other: ChangeSet) -> Self {
        self.added.extend(other.added);
        self.removed.extend(other.removed);
        self.modified.extend(other.modified);
        self.edges_added.extend(other.edges_added);
        self.edges_removed.extend(other.edges_removed);
        self.invalidations.extend(other.invalidations);
        self
    }
}

// ─── Edit Trait ────────────────────────────────────────────────────────

/// A structural mutation on the UIR with precondition checking and undo.
///
/// AI agents issue edits as structured commands. The edit validates its
/// precondition against the current graph, applies the mutation, and
/// returns a ChangeSet describing what changed.
pub trait Edit {
    /// Validate that this edit can be applied to the current graph.
    /// Returns an error with a human-readable reason if the precondition fails.
    fn precondition(&self, program: &Program) -> Result<(), String>;

    /// Apply the edit to the graph. Returns a ChangeSet describing
    /// exactly what changed (added, removed, edges, invalidations).
    fn apply(&self, program: &mut Program) -> ChangeSet;

    /// Undo this edit, given the ChangeSet that `apply` returned.
    /// Restores the graph to its pre-edit state.
    fn undo(&self, program: &mut Program, changes: &ChangeSet) -> Result<(), String>;

    /// Human-readable description of what this edit does.
    fn description(&self) -> String;
}

// ─── Concrete Edits ────────────────────────────────────────────────────

/// Rename a function. All call edges are preserved.
pub struct Rename {
    pub target: String, // legacy_id of the function to rename
    pub new_name: String,
}

impl Edit for Rename {
    fn precondition(&self, program: &Program) -> Result<(), String> {
        if !program.contains_legacy(&self.target) {
            return Err(format!("function not found: {}", self.target));
        }
        let file = match self.target.rfind("::") {
            Some(pos) => &self.target[..pos],
            None => "",
        };
        let new_id = format!("{}::{}", file, self.new_name);
        if program.contains_legacy_id(&new_id) {
            return Err(format!("rename target already exists: {}", new_id));
        }
        Ok(())
    }

    fn apply(&self, program: &mut Program) -> ChangeSet {
        let mut changes = ChangeSet::default();

        let (_old_fid, func) = match program.remove_by_legacy(&self.target) {
            Some(pair) => pair,
            None => return changes,
        };

        let file = match self.target.rfind("::") {
            Some(pos) => &self.target[..pos],
            None => "",
        };
        let new_id = format!("{}::{}", file, self.new_name);

        // Update the function's name and ID
        let mut updated = func.clone();
        updated.id = new_id.clone();
        updated.name = self.new_name.clone();
        program.add_function(updated);

        // Update edges: fix caller/callee references
        let mut new_edges = Vec::new();
        let mut edges_removed = Vec::new();

        program.edges.retain(|e| {
            let needs_update = e.caller == self.target || e.callee == self.target;
            if needs_update {
                let new_caller = if e.caller == self.target {
                    new_id.clone()
                } else {
                    e.caller.clone()
                };
                let new_callee = if e.callee == self.target {
                    new_id.clone()
                } else {
                    e.callee.clone()
                };
                edges_removed.push((e.caller.clone(), e.callee.clone()));
                new_edges.push((new_caller, new_callee, e.kind));
                false
            } else {
                true
            }
        });

        for (caller, callee, kind) in new_edges {
            program.edges.push(crate::uir::CallEdge {
                caller,
                callee,
                kind,
                confidence: kind.confidence(),
                source: "edit".to_string(),
            });
        }

        // Build a new FunctionId with the updated name
        let lang = func.language.parse().unwrap_or(Language::C);
        let new_fid = FunctionId::new(
            lang,
            file,
            if func.is_static {
                Linkage::Static
            } else {
                Linkage::Module
            },
            &self.new_name,
            0, // hash will be updated by re-parsing
        );

        changes.modified.push((new_fid, new_id.clone()));
        changes.edges_removed = edges_removed;
        changes.edges_added = program
            .edges
            .iter()
            .filter(|e| e.caller == new_id || e.callee == new_id)
            .map(|e| (e.caller.clone(), e.callee.clone(), e.kind))
            .collect();

        // Invalidate all callers of this function (they reference by old name)
        for edge in &program.edges {
            if edge.callee == new_id {
                changes.invalidations.push(edge.caller.clone());
            }
        }

        changes
    }

    fn undo(&self, program: &mut Program, changes: &ChangeSet) -> Result<(), String> {
        let file = match self.target.rfind("::") {
            Some(pos) => &self.target[..pos],
            None => "",
        };
        let new_id = format!("{}::{}", file, self.new_name);
        let (_fid, mut func) = match program.remove_by_legacy(&new_id) {
            Some(pair) => pair,
            None => {
                return Err(format!(
                    "cannot undo: renamed function not found: {}",
                    new_id
                ))
            }
        };

        // Restore original name
        func.id = self.target.clone();
        func.name = match self.target.rfind("::") {
            Some(pos) => self.target[pos + 2..].to_string(),
            None => self.target.clone(),
        };
        program.add_function(func);

        // Restore edges
        for (old_caller, old_callee) in &changes.edges_removed {
            program
                .edges
                .retain(|e| !(e.caller == *old_caller && e.callee == *old_callee));
        }
        // Re-add old edges with original IDs
        for (caller, callee, kind) in &changes.edges_added {
            let orig_caller = if *caller == new_id {
                self.target.clone()
            } else {
                caller.clone()
            };
            let orig_callee = if *callee == new_id {
                self.target.clone()
            } else {
                callee.clone()
            };
            program.edges.push(crate::uir::CallEdge {
                caller: orig_caller,
                callee: orig_callee,
                kind: *kind,
                confidence: kind.confidence(),
                source: "edit".to_string(),
            });
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!("rename '{}' → '{}'", self.target, self.new_name)
    }
}

/// Delete a function and all its edges.
pub struct Delete {
    pub target: String, // legacy_id
}

impl Edit for Delete {
    fn precondition(&self, program: &Program) -> Result<(), String> {
        if !program.contains_legacy(&self.target) {
            return Err(format!("function not found: {}", self.target));
        }
        Ok(())
    }

    fn apply(&self, program: &mut Program) -> ChangeSet {
        let mut changes = ChangeSet::default();

        let (fid, _func) = match program.remove_by_legacy(&self.target) {
            Some(pair) => pair,
            None => return changes,
        };

        // Collect edges to remove
        let mut caller_edges = Vec::new();
        let mut callee_edges = Vec::new();
        let before_count = program.edges.len();
        program.edges.retain(|e| {
            if e.caller == self.target {
                callee_edges.push((e.caller.clone(), e.callee.clone()));
                false
            } else if e.callee == self.target {
                caller_edges.push((e.caller.clone(), e.callee.clone()));
                false
            } else {
                true
            }
        });
        let removed_count = before_count - program.edges.len();

        changes.removed.push(fid);
        changes.edges_removed = caller_edges
            .iter()
            .cloned()
            .chain(callee_edges.iter().cloned())
            .collect();

        // Invalidate callers: their outgoing edge is now dangling
        for (caller, _) in &caller_edges {
            if !changes.invalidations.contains(caller) {
                changes.invalidations.push(caller.clone());
            }
        }

        // Warn: remaining callers still reference the deleted function
        let remaining_callers: Vec<_> = program
            .edges
            .iter()
            .filter(|e| e.callee == self.target)
            .map(|e| e.caller.clone())
            .collect();
        if !remaining_callers.is_empty() {
            // These edges weren't removed because the callee was the target
            // (edge removal above already handled this, so this is defensive)
        }

        eprintln!(
            "[edit] deleted {} ({} edges removed, {} remaining callers)",
            self.target,
            removed_count,
            remaining_callers.len()
        );

        changes
    }

    fn undo(&self, _program: &mut Program, _changes: &ChangeSet) -> Result<(), String> {
        // Undo only works if we know the original function data.
        // In practice, Delete should store a snapshot. For now,
        // we rely on re-parsing the file to recover the function.
        Err("Delete::undo requires re-parsing the source file".into())
    }

    fn description(&self) -> String {
        format!("delete '{}'", self.target)
    }
}

/// Move a function to a different file.
pub struct Move {
    pub target: String,  // legacy_id
    pub to_file: String, // new file path
}

impl Edit for Move {
    fn precondition(&self, program: &Program) -> Result<(), String> {
        if !program.contains_legacy(&self.target) {
            return Err(format!("function not found: {}", self.target));
        }
        Ok(())
    }

    fn apply(&self, program: &mut Program) -> ChangeSet {
        let mut changes = ChangeSet::default();

        let (_old_fid, func) = match program.remove_by_legacy(&self.target) {
            Some(pair) => pair,
            None => return changes,
        };

        let name = match self.target.rfind("::") {
            Some(pos) => self.target[pos + 2..].to_string(),
            None => self.target.clone(),
        };
        let new_id = format!("{}::{}", self.to_file, name);

        let mut updated = func.clone();
        updated.id = new_id.clone();
        updated.file = self.to_file.clone();
        program.add_function(updated);

        // Update edges
        program.edges.iter_mut().for_each(|e| {
            if e.caller == self.target {
                e.caller = new_id.clone();
            }
            if e.callee == self.target {
                e.callee = new_id.clone();
            }
        });

        let lang = func.language.parse().unwrap_or(Language::C);
        let fid = FunctionId::new(
            lang,
            &self.to_file,
            if func.is_static {
                Linkage::Static
            } else {
                Linkage::Module
            },
            &name,
            0,
        );
        changes.modified.push((fid, new_id.clone()));

        // Invalidate all edges involving this function
        for edge in &program.edges {
            if edge.caller == new_id || edge.callee == new_id {
                changes.invalidations.push(edge.caller.clone());
            }
        }

        changes
    }

    fn undo(&self, program: &mut Program, changes: &ChangeSet) -> Result<(), String> {
        let new_id = match changes.modified.first() {
            Some((_, id)) => id.clone(),
            None => return Err("no modified entries in changeset".into()),
        };

        let (_fid, mut func) = match program.remove_by_legacy(&new_id) {
            Some(pair) => pair,
            None => return Err(format!("moved function not found: {}", new_id)),
        };
        func.id = self.target.clone();
        func.file = match self.target.rfind("::") {
            Some(pos) => self.target[..pos].to_string(),
            None => self.target.clone(),
        };
        program.add_function(func);

        program.edges.iter_mut().for_each(|e| {
            if e.caller == new_id {
                e.caller = self.target.clone();
            }
            if e.callee == new_id {
                e.callee = self.target.clone();
            }
        });

        Ok(())
    }

    fn description(&self) -> String {
        format!("move '{}' → '{}'", self.target, self.to_file)
    }
}

/// Annotate a function with metadata. Non-destructive.
pub struct Annotate {
    pub target: String,
    pub annotation: String,
}

impl Edit for Annotate {
    fn precondition(&self, program: &Program) -> Result<(), String> {
        if !program.contains_legacy(&self.target) {
            return Err(format!("function not found: {}", self.target));
        }
        Ok(())
    }

    fn apply(&self, _program: &mut Program) -> ChangeSet {
        // Annotation is metadata-only. In a fuller implementation,
        // Function would have an annotation field. For now,
        // we return an empty ChangeSet since the graph structure
        // doesn't change.
        ChangeSet::default()
    }

    fn undo(&self, _program: &mut Program, _changes: &ChangeSet) -> Result<(), String> {
        Ok(())
    }

    fn description(&self) -> String {
        format!("annotate '{}' = '{}'", self.target, self.annotation)
    }
}

// ─── Analysis Trait ────────────────────────────────────────────────────

/// An incremental derived view over the UIR.
///
/// Analyses subscribe to ChangeSets and update their internal state
/// incrementally. They never rebuild from scratch — they apply only
/// the deltas.
pub trait Analysis<T> {
    /// Get the current value of this analysis.
    fn value(&self) -> &T;

    /// Incrementally update based on a ChangeSet.
    fn on_change(&mut self, changes: &ChangeSet);
}

// ─── Concrete Analyses ─────────────────────────────────────────────────

/// Dead code analysis: functions with zero incoming call edges.
#[derive(Debug, Default)]
pub struct DeadCode {
    /// Set of function legacy_ids that have at least one incoming edge.
    called: HashSet<String>,
    /// All known function legacy_ids.
    all: HashSet<String>,
}

impl DeadCode {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize from a full program (for bootstrap).
    pub fn bootstrap(&mut self, program: &Program) {
        self.all.clear();
        self.called.clear();
        for func in program.functions.values() {
            self.all.insert(func.id.clone());
        }
        for edge in &program.edges {
            self.called.insert(edge.callee.clone());
        }
    }
}

impl Analysis<HashSet<String>> for DeadCode {
    fn value(&self) -> &HashSet<String> {
        &self.all // caller filters to !called
    }

    fn on_change(&mut self, changes: &ChangeSet) {
        // Update all functions
        for (_, legacy_id) in &changes.added {
            self.all.insert(legacy_id.clone());
        }
        for (_, _legacy_id) in &changes.modified {
            // Already in all, no change needed
        }
        for id in &changes.removed {
            self.all.remove(&id.to_legacy());
        }

        // Update called set
        for (_, callee, _) in &changes.edges_added {
            self.called.insert(callee.clone());
        }
        for (_, callee) in &changes.edges_removed {
            // Check if any other edge still calls this callee
            // (we don't have fast access to remaining edges here,
            //  so this is approximate — a full rebootstrap fixes it)
            self.called.remove(callee);
        }
    }
}

impl DeadCode {
    /// Returns the set of dead function legacy_ids.
    pub fn dead_functions(&self) -> Vec<String> {
        self.all
            .iter()
            .filter(|id| !self.called.contains(*id))
            .cloned()
            .collect()
    }
}

/// Coupling score analysis: ranks functions by structural complexity.
/// Score = in_degree × out_degree. High scores = coupling hotspots.
#[derive(Debug, Default)]
pub struct CouplingScore {
    in_degree: HashMap<String, usize>,
    out_degree: HashMap<String, usize>,
}

impl CouplingScore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bootstrap(&mut self, program: &Program) {
        self.in_degree.clear();
        self.out_degree.clear();
        for edge in &program.edges {
            *self.out_degree.entry(edge.caller.clone()).or_insert(0) += 1;
            *self.in_degree.entry(edge.callee.clone()).or_insert(0) += 1;
        }
    }

    /// Get the score for a specific function.
    pub fn score(&self, id: &str) -> usize {
        self.in_degree.get(id).copied().unwrap_or(0) * self.out_degree.get(id).copied().unwrap_or(0)
    }

    /// Get all functions ranked by coupling score (descending).
    pub fn ranked(&self) -> Vec<(String, usize)> {
        let mut scores: Vec<_> = self
            .in_degree
            .keys()
            .chain(self.out_degree.keys())
            .collect::<HashSet<_>>()
            .into_iter()
            .map(|id| (id.clone(), self.score(id)))
            .collect();
        scores.sort_by_key(|(_, s)| -(*s as isize));
        scores
    }
}

impl Analysis<HashMap<String, usize>> for CouplingScore {
    fn value(&self) -> &HashMap<String, usize> {
        &self.in_degree
    }

    fn on_change(&mut self, changes: &ChangeSet) {
        for (caller, callee, _) in &changes.edges_added {
            *self.out_degree.entry(caller.clone()).or_insert(0) += 1;
            *self.in_degree.entry(callee.clone()).or_insert(0) += 1;
        }
        for (caller, callee) in &changes.edges_removed {
            if let Some(d) = self.out_degree.get_mut(caller) {
                *d = d.saturating_sub(1);
            }
            if let Some(d) = self.in_degree.get_mut(callee) {
                *d = d.saturating_sub(1);
            }
        }
    }
}

/// Path index: BFS shortest-path cache between any two functions.
/// Maintains an adjacency list for fast lookups.
#[derive(Debug, Default)]
pub struct PathIndex {
    /// caller → set of callees (adjacency list)
    adjacency: HashMap<String, Vec<String>>,
}

impl PathIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bootstrap(&mut self, program: &Program) {
        self.adjacency.clear();
        for edge in &program.edges {
            self.adjacency
                .entry(edge.caller.clone())
                .or_default()
                .push(edge.callee.clone());
        }
    }

    /// Find the shortest call path from `source` to `target`.
    /// Returns the sequence of function legacy_ids, or None if unreachable.
    pub fn shortest_path(&self, source: &str, target: &str) -> Option<Vec<String>> {
        if source == target {
            return Some(vec![source.to_string()]);
        }

        let mut visited = HashSet::new();
        let mut parent: HashMap<String, String> = HashMap::new();
        let mut queue = vec![source.to_string()];
        visited.insert(source.to_string());
        let mut head = 0;

        while head < queue.len() {
            let current = queue[head].clone();
            head += 1;

            if let Some(neighbors) = self.adjacency.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        parent.insert(neighbor.clone(), current.clone());
                        if neighbor == target {
                            // Reconstruct path
                            let mut path = vec![target.to_string()];
                            let mut node = target;
                            while let Some(p) = parent.get(node) {
                                path.push(p.clone());
                                node = p;
                            }
                            path.reverse();
                            return Some(path);
                        }
                        queue.push(neighbor.clone());
                    }
                }
            }
        }

        None
    }
}

impl Analysis<HashMap<String, Vec<String>>> for PathIndex {
    fn value(&self) -> &HashMap<String, Vec<String>> {
        &self.adjacency
    }

    fn on_change(&mut self, changes: &ChangeSet) {
        for (caller, callee, _) in &changes.edges_added {
            self.adjacency
                .entry(caller.clone())
                .or_default()
                .push(callee.clone());
        }
        for (caller, callee) in &changes.edges_removed {
            if let Some(neighbors) = self.adjacency.get_mut(caller) {
                neighbors.retain(|n| n != callee);
            }
        }
    }
}
