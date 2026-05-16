#[cfg(feature = "wire-rkyv")]
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "wire-rkyv", derive(Archive, Serialize, Deserialize))]
pub struct SourceBlob {
    pub path: String,
    pub source: String,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "wire-rkyv", derive(Archive, Serialize, Deserialize))]
pub struct SourceSnapshot {
    pub files: Vec<SourceBlob>,
    pub total_bytes: u64,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "wire-rkyv", derive(Archive, Serialize, Deserialize))]
pub struct LocalConnection {
    pub network_transport: String,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: String,
    pub remote_port: u16,
    pub state: String,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "wire-rkyv", derive(Archive, Serialize, Deserialize))]
pub struct ConnectionsEnvelope {
    pub connections: Vec<LocalConnection>,
    pub connection_count: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub file: String,
    pub language: String,
    pub is_static: bool,
    pub connections: u32,
    pub tags: Vec<String>,
    pub commit: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TagGroup {
    pub name: String,
    pub tags: Vec<String>,
    pub color: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub tag_groups: Vec<TagGroup>,
    pub total_bytes: u64,
    pub node_count: u32,
    pub edge_count: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Diagnostic {
    pub file: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub severity: String,
    pub message: String,
    pub source: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub file: Option<String>,
    pub message: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct CoverageInfo {
    pub file: String,
    pub lines_total: u32,
    pub lines_covered: u32,
    pub coverage_pct: f64,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FormatIssue {
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct MissingRef {
    pub name: String,
    pub referenced_by: String,
    pub kind: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DiagnosticsReport {
    pub linters_available: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub tests: Vec<TestResult>,
    pub coverage: Vec<CoverageInfo>,
    pub formatting_issues: Vec<FormatIssue>,
    pub missing_references: Vec<MissingRef>,
    pub scan_timestamp: String,
}
