// rkyv-normalized wire protocol for codeanalyzer.
#![allow(unused_imports)]

use rkyv::{Archive, Deserialize, Serialize};

// Wire protocol is rkyv only:

pub mod rkyv_wire {
    use rkyv::rancor::Error;

    use super::WsMessage;

    pub const WIRE_PROTOCOL: &str = crate::WIRE_PROTOCOL;

    pub fn encode_message(message: &WsMessage) -> Result<Vec<u8>, Error> {
        rkyv::to_bytes::<Error>(message).map(|bytes| bytes.to_vec())
    }

    pub fn decode_message(bytes: &[u8]) -> Result<WsMessage, Error> {
        let archived = rkyv::access::<super::ArchivedWsMessage, Error>(bytes)?;
        rkyv::deserialize::<WsMessage, Error>(archived)
    }
}

// GraphNode message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
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

// GraphEdge message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
}

// TagGroup message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct TagGroup {
    pub name: String,
    pub tags: Vec<String>,
    pub color: String,
}

// GraphData message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub tag_groups: Vec<TagGroup>,
    pub total_bytes: u64,
    pub node_count: u32,
    pub edge_count: u32,
}

// Diagnostic message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub severity: String,
    pub message: String,
    pub source: String,
}

// TestResult message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub file: Option<String>,
    pub message: Option<String>,
    pub duration_ms: Option<u64>,
}

// CoverageInfo message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct CoverageInfo {
    pub file: String,
    pub lines_total: u32,
    pub lines_covered: u32,
    pub coverage_pct: f64,
}

// FormatIssue message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct FormatIssue {
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
}

// MissingRef message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct MissingRef {
    pub name: String,
    pub referenced_by: String,
    pub kind: String,
}

// DiagnosticsReport message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    pub linters_available: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub tests: Vec<TestResult>,
    pub coverage: Vec<CoverageInfo>,
    pub formatting_issues: Vec<FormatIssue>,
    pub missing_references: Vec<MissingRef>,
    pub scan_timestamp: String,
}

// IndexState oneof variants
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub enum IndexStateEnum {
    NotIndexed,
    Indexing {
        progress: f32,
        files_scanned: u32,
        total_files: u32,
    },
    Indexed {
        functions: u32,
        edges: u32,
        index_time_ms: u64,
    },
    Stale {
        functions: u32,
        edges: u32,
        last_indexed: u64,
    },
    Error {
        message: String,
    },
}

// IndexState message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct IndexState {
    pub state: Option<IndexStateEnum>,
}

// RepoInfo message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct RepoInfo {
    pub path: String,
    pub name: String,
    pub is_git_repo: bool,
    pub git_remote: Option<String>,
    pub file_count: u32,
    pub total_size_bytes: u64,
    pub last_modified: u64,
    pub index_state: Option<IndexState>,
    pub languages: Vec<String>,
    pub added_at: u64,
}

// RegistryConfig message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub scan_roots: Vec<String>,
    pub excluded_paths: Vec<String>,
    pub max_concurrent_index: u32,
    pub auto_discover: bool,
}

// RepoList message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct RepoList {
    pub repos: Vec<RepoInfo>,
    pub active: Option<String>,
}

// FileInfo message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub hash: u64,
    pub modified_ts: u64,
    pub language: String,
    pub size: u64,
}

// FileEntry message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

// DirListing message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct DirListing {
    pub entries: Vec<FileEntry>,
}

// QueryMatch message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct QueryMatch {
    pub id: String,
    pub name: String,
    pub file: String,
    pub language: String,
}

// QueryResult message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct QueryResult {
    pub matches: Vec<QueryMatch>,
    pub total_count: u32,
}

// StatsResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct StatsResponse {
    pub languages: std::collections::HashMap<String, u32>,
    pub files: std::collections::HashMap<String, u32>,
    pub total_functions: u32,
    pub total_edges: u32,
}

// XrefResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct XrefResponse {
    pub function: String,
    pub callers: Vec<String>,
    pub callees: Vec<String>,
}

// FileData message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct FileData {
    pub file: String,
    pub functions: Vec<String>,
    pub dependencies: Vec<String>,
}

// FilesResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct FilesResponse {
    pub files: Vec<FileData>,
}

// AnalyzeResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct AnalyzeResponse {
    pub status: String,
    pub path: String,
    pub functions: u32,
    pub edges: u32,
}

// ApplyEditResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ApplyEditResponse {
    pub status: String,
    pub path: String,
    pub bytes: u32,
}

// ChatRequest message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub context: Option<String>,
    pub summary: Option<String>,
    pub session_id: Option<String>,
}

// ChatResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ChatResponse {
    pub reply: String,
    pub error: Option<String>,
}

// ToolCallInfo message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub name: String,
    pub args: String,
    pub output: String,
    pub content: Option<String>,
}

// ToolUseResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ToolUseResponse {
    pub reply: String,
    pub tools: Vec<ToolCallInfo>,
    pub error: Option<String>,
}

// StatusResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub error: Option<String>,
    pub functions: Option<u32>,
    pub edges: Option<u32>,
    pub found: Option<u32>,
}

// ErrorResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ViewNode message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ViewNode {
    pub id: String,
    pub name: String,
    pub r#type: String,
}

// ViewEdge message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ViewEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
}

// ViewData message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ViewData {
    pub nodes: Vec<ViewNode>,
    pub edges: Vec<ViewEdge>,
}

// GetFileResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GetFileResponse {
    pub content: String,
}

// GetViewResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GetViewResponse {
    pub data: Option<ViewData>,
}

// GetReposResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GetReposResponse {
    pub repos: Option<RepoList>,
}

// SwitchRepoResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct SwitchRepoResponse {
    pub status: Option<StatusResponse>,
}

// GetFilesResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GetFilesResponse {
    pub files: Vec<FileData>,
}

// GetDiagnosticsResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GetDiagnosticsResponse {
    pub diagnostics: Vec<Diagnostic>,
}

// AddRepoResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct AddRepoResponse {
    pub status: Option<StatusResponse>,
    pub path: String,
}

// RemoveRepoResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct RemoveRepoResponse {
    pub status: Option<StatusResponse>,
}

// DiscoverReposResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct DiscoverReposResponse {
    pub found: u32,
    pub repos: Option<RepoList>,
}

// GetConfigResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GetConfigResponse {
    pub config: Option<RegistryConfig>,
}

// UpdateConfigResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct UpdateConfigResponse {
    pub status: Option<StatusResponse>,
}

// GetIndexStateResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GetIndexStateResponse {
    pub state: Option<IndexState>,
}

// GetWsConnectionsResponse message
#[derive(Clone, Copy, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GetWsConnectionsResponse {
    pub count: u32,
}

// WsRequest messages
#[derive(Clone, Copy, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct EmptyRequest;

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct PathRequest {
    pub path: String,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ViewRequest {
    pub view: String,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ApplyEditRequest {
    pub path: String,
    pub content: String,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct XrefRequest {
    pub function: String,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub enum WsRequestPayload {
    GetGraph(EmptyRequest),
    GetFile(PathRequest),
    GetFs(PathRequest),
    GetView(ViewRequest),
    GetDiagnostics(PathRequest),
    GetFiles(EmptyRequest),
    Analyze(PathRequest),
    ApplyEdit(ApplyEditRequest),
    GetRepos(EmptyRequest),
    DiscoverRepos(EmptyRequest),
    AddRepo(PathRequest),
    RemoveRepo(PathRequest),
    SwitchRepo(PathRequest),
    Chat(ChatRequest),
    Query(QueryRequest),
    Stats(EmptyRequest),
    Xref(XrefRequest),
    Files(EmptyRequest),
    GetConfig(EmptyRequest),
    UpdateConfig(RegistryConfig),
    GetIndexState(EmptyRequest),
    GetWsConnections(EmptyRequest),
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct WsRequest {
    pub id: u64,
    pub request: WsRequestPayload,
}

// WsResponse message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct WsResponse {
    pub id: u64,
    pub result: WsResponseResult,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub enum WsResponseResult {
    Empty,
    GraphData(GraphData),
    GetFile(GetFileResponse),
    GetFs(DirListing),
    GetView(GetViewResponse),
    GetDiagnostics(GetDiagnosticsResponse),
    GetFiles(GetFilesResponse),
    Analyze(AnalyzeResponse),
    ApplyEdit(ApplyEditResponse),
    GetRepos(GetReposResponse),
    DiscoverRepos(DiscoverReposResponse),
    AddRepo(AddRepoResponse),
    RemoveRepo(RemoveRepoResponse),
    SwitchRepo(SwitchRepoResponse),
    Chat(ToolUseResponse),
    Query(QueryResult),
    Stats(StatsResponse),
    Xref(XrefResponse),
    Files(FilesResponse),
    GetConfig(GetConfigResponse),
    UpdateConfig(UpdateConfigResponse),
    GetIndexState(GetIndexStateResponse),
    GetWsConnections(GetWsConnectionsResponse),
    Error(String),
}

// GraphUpdate messages
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct NodeUpdate {
    pub added: Vec<GraphNode>,
    pub modified: Vec<GraphNode>,
    pub removed: Vec<String>,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct EdgeUpdate {
    pub added: Vec<GraphEdge>,
    pub removed: Vec<String>,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct GraphUpdate {
    pub nodes: Option<NodeUpdate>,
    pub edges: Option<EdgeUpdate>,
    pub is_full: bool,
    pub timestamp: u64,
}

// WsMessage types
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ClientReady;

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct DebugLog {
    pub level: String,
    pub message: String,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct EvalCommand {
    pub id: u64,
    pub code: String,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct EvalResult {
    pub id: u64,
    pub result: Option<String>,
    pub error: Option<String>,
}

// WsMessage oneof
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub enum WsMessageEnum {
    Request(WsRequest),
    Response(WsResponse),
    GraphData(GraphData),
    GraphUpdate(GraphUpdate),
    EvalCommand(EvalCommand),
    EvalResult(EvalResult),
    ClientReady(ClientReady),
    DebugLog(DebugLog),
}

// WsMessage message
#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct WsMessage {
    pub msg: Option<WsMessageEnum>,
}

impl WsMessage {
    pub fn encode_rkyv(&self) -> Result<Vec<u8>, rkyv::rancor::Error> {
        rkyv_wire::encode_message(self)
    }

    pub fn decode_rkyv(bytes: &[u8]) -> Result<Self, rkyv::rancor::Error> {
        rkyv_wire::decode_message(bytes)
    }
}
