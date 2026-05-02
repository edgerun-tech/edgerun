use std::collections::HashMap;
use std::path::PathBuf;

use edgerun_json::{JsonValue, ToJson, FromJson};

#[derive(Debug, Clone)]
pub struct CrateIdentity {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub description: Option<String>,
    pub license: Option<String>,
    pub edition: String,
    pub rust_version: Option<String>,
    pub features: Vec<String>,
    pub lib_target: bool,
    pub bin_targets: Vec<String>,
    pub test_targets: Vec<String>,
    pub bench_targets: Vec<String>,
    pub visible_files: Vec<PathBuf>,
    pub hidden_files_count: usize,
    pub commit_hash: Option<String>,
}

impl ToJson for CrateIdentity {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("name".into(), self.name.to_json());
        map.insert("version".into(), self.version.to_json());
        map.insert("path".into(), self.path.to_json());
        map.insert("description".into(), self.description.to_json());
        map.insert("license".into(), self.license.to_json());
        map.insert("edition".into(), self.edition.to_json());
        map.insert("rust_version".into(), self.rust_version.to_json());
        map.insert("features".into(), self.features.to_json());
        map.insert("lib_target".into(), self.lib_target.to_json());
        map.insert("bin_targets".into(), self.bin_targets.to_json());
        map.insert("test_targets".into(), self.test_targets.to_json());
        map.insert("bench_targets".into(), self.bench_targets.to_json());
        map.insert("visible_files".into(), self.visible_files.to_json());
        map.insert("hidden_files_count".into(), self.hidden_files_count.to_json());
        map.insert("commit_hash".into(), self.commit_hash.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for CrateIdentity {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(CrateIdentity {
            name: FromJson::from_json(obj.get("name").ok_or("missing field")?)?,
            version: FromJson::from_json(obj.get("version").ok_or("missing field")?)?,
            path: FromJson::from_json(obj.get("path").ok_or("missing field")?)?,
            description: FromJson::from_json(obj.get("description").ok_or("missing field")?)?,
            license: FromJson::from_json(obj.get("license").ok_or("missing field")?)?,
            edition: FromJson::from_json(obj.get("edition").ok_or("missing field")?)?,
            rust_version: FromJson::from_json(obj.get("rust_version").ok_or("missing field")?)?,
            features: FromJson::from_json(obj.get("features").ok_or("missing field")?)?,
            lib_target: FromJson::from_json(obj.get("lib_target").ok_or("missing field")?)?,
            bin_targets: FromJson::from_json(obj.get("bin_targets").ok_or("missing field")?)?,
            test_targets: FromJson::from_json(obj.get("test_targets").ok_or("missing field")?)?,
            bench_targets: FromJson::from_json(obj.get("bench_targets").ok_or("missing field")?)?,
            visible_files: FromJson::from_json(obj.get("visible_files").ok_or("missing field")?)?,
            hidden_files_count: FromJson::from_json(obj.get("hidden_files_count").ok_or("missing field")?)?,
            commit_hash: FromJson::from_json(obj.get("commit_hash").ok_or("missing field")?)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version_req: Option<String>,
    pub kind: DependencyKind,
    pub optional: bool,
    pub features: Vec<String>,
    pub reason: Option<String>,
    pub is_workspace: bool,
    pub is_visible: bool,
}

impl ToJson for Dependency {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("name".into(), self.name.to_json());
        map.insert("version_req".into(), self.version_req.to_json());
        map.insert("kind".into(), self.kind.to_json());
        map.insert("optional".into(), self.optional.to_json());
        map.insert("features".into(), self.features.to_json());
        map.insert("reason".into(), self.reason.to_json());
        map.insert("is_workspace".into(), self.is_workspace.to_json());
        map.insert("is_visible".into(), self.is_visible.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for Dependency {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(Dependency {
            name: FromJson::from_json(obj.get("name").ok_or("missing field")?)?,
            version_req: FromJson::from_json(obj.get("version_req").ok_or("missing field")?)?,
            kind: FromJson::from_json(obj.get("kind").ok_or("missing field")?)?,
            optional: FromJson::from_json(obj.get("optional").ok_or("missing field")?)?,
            features: FromJson::from_json(obj.get("features").ok_or("missing field")?)?,
            reason: FromJson::from_json(obj.get("reason").ok_or("missing field")?)?,
            is_workspace: FromJson::from_json(obj.get("is_workspace").ok_or("missing field")?)?,
            is_visible: FromJson::from_json(obj.get("is_visible").ok_or("missing field")?)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}

impl ToJson for DependencyKind {
    fn to_json(&self) -> JsonValue {
        match self {
            DependencyKind::Normal => JsonValue::String("Normal".into()),
            DependencyKind::Dev => JsonValue::String("Dev".into()),
            DependencyKind::Build => JsonValue::String("Build".into()),
        }
    }
}

impl FromJson for DependencyKind {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let s = value.as_str().ok_or("expected string")?;
        match s {
            "Normal" => Ok(DependencyKind::Normal),
            "Dev" => Ok(DependencyKind::Dev),
            "Build" => Ok(DependencyKind::Build),
            _ => Err("invalid dependency kind".into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiItem {
    pub name: String,
    pub kind: ApiItemKind,
    pub file: PathBuf,
    pub line: usize,
    pub docs: bool,
    pub feature_gate: Option<String>,
    pub visibility: Visibility,
}

impl ToJson for ApiItem {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("name".into(), self.name.to_json());
        map.insert("kind".into(), self.kind.to_json());
        map.insert("file".into(), self.file.to_json());
        map.insert("line".into(), self.line.to_json());
        map.insert("docs".into(), self.docs.to_json());
        map.insert("feature_gate".into(), self.feature_gate.to_json());
        map.insert("visibility".into(), self.visibility.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for ApiItem {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(ApiItem {
            name: FromJson::from_json(obj.get("name").ok_or("missing field")?)?,
            kind: FromJson::from_json(obj.get("kind").ok_or("missing field")?)?,
            file: FromJson::from_json(obj.get("file").ok_or("missing field")?)?,
            line: FromJson::from_json(obj.get("line").ok_or("missing field")?)?,
            docs: FromJson::from_json(obj.get("docs").ok_or("missing field")?)?,
            feature_gate: FromJson::from_json(obj.get("feature_gate").ok_or("missing field")?)?,
            visibility: FromJson::from_json(obj.get("visibility").ok_or("missing field")?)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ApiItemKind {
    Module,
    Struct,
    Enum,
    Trait,
    Function,
    Impl,
}

impl ToJson for ApiItemKind {
    fn to_json(&self) -> JsonValue {
        let s = match self {
            ApiItemKind::Module => "Module",
            ApiItemKind::Struct => "Struct",
            ApiItemKind::Enum => "Enum",
            ApiItemKind::Trait => "Trait",
            ApiItemKind::Function => "Function",
            ApiItemKind::Impl => "Impl",
        };
        JsonValue::String(s.into())
    }
}

impl FromJson for ApiItemKind {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let s = value.as_str().ok_or("expected string")?;
        match s {
            "Module" => Ok(ApiItemKind::Module),
            "Struct" => Ok(ApiItemKind::Struct),
            "Enum" => Ok(ApiItemKind::Enum),
            "Trait" => Ok(ApiItemKind::Trait),
            "Function" => Ok(ApiItemKind::Function),
            "Impl" => Ok(ApiItemKind::Impl),
            _ => Err("invalid api item kind".into()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Visibility {
    Public,
    Private,
    Crate,
}

impl ToJson for Visibility {
    fn to_json(&self) -> JsonValue {
        let s = match self {
            Visibility::Public => "Public",
            Visibility::Private => "Private",
            Visibility::Crate => "Crate",
        };
        JsonValue::String(s.into())
    }
}

impl FromJson for Visibility {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let s = value.as_str().ok_or("expected string")?;
        match s {
            "Public" => Ok(Visibility::Public),
            "Private" => Ok(Visibility::Private),
            "Crate" => Ok(Visibility::Crate),
            _ => Err("invalid visibility".into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallGraphEdge {
    pub caller: String,
    pub callee: String,
    pub file: PathBuf,
    pub line: usize,
    pub confidence: Confidence,
}

impl ToJson for CallGraphEdge {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("caller".into(), self.caller.to_json());
        map.insert("callee".into(), self.callee.to_json());
        map.insert("file".into(), self.file.to_json());
        map.insert("line".into(), self.line.to_json());
        map.insert("confidence".into(), self.confidence.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for CallGraphEdge {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(CallGraphEdge {
            caller: FromJson::from_json(obj.get("caller").ok_or("missing field")?)?,
            callee: FromJson::from_json(obj.get("callee").ok_or("missing field")?)?,
            file: FromJson::from_json(obj.get("file").ok_or("missing field")?)?,
            line: FromJson::from_json(obj.get("line").ok_or("missing field")?)?,
            confidence: FromJson::from_json(obj.get("confidence").ok_or("missing field")?)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Confidence {
    Exact,
    Likely,
    Ambiguous,
}

impl ToJson for Confidence {
    fn to_json(&self) -> JsonValue {
        let s = match self {
            Confidence::Exact => "Exact",
            Confidence::Likely => "Likely",
            Confidence::Ambiguous => "Ambiguous",
        };
        JsonValue::String(s.into())
    }
}

impl FromJson for Confidence {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let s = value.as_str().ok_or("expected string")?;
        match s {
            "Exact" => Ok(Confidence::Exact),
            "Likely" => Ok(Confidence::Likely),
            "Ambiguous" => Ok(Confidence::Ambiguous),
            _ => Err("invalid confidence".into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecurityFinding {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub code_excerpt: Option<String>,
    pub explanation: String,
    pub confidence: Confidence,
    pub recommendation: String,
}

impl ToJson for SecurityFinding {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("id".into(), self.id.to_json());
        map.insert("severity".into(), self.severity.to_json());
        map.insert("title".into(), self.title.to_json());
        map.insert("file".into(), self.file.to_json());
        map.insert("line".into(), self.line.to_json());
        map.insert("code_excerpt".into(), self.code_excerpt.to_json());
        map.insert("explanation".into(), self.explanation.to_json());
        map.insert("confidence".into(), self.confidence.to_json());
        map.insert("recommendation".into(), self.recommendation.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for SecurityFinding {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(SecurityFinding {
            id: FromJson::from_json(obj.get("id").ok_or("missing field")?)?,
            severity: FromJson::from_json(obj.get("severity").ok_or("missing field")?)?,
            title: FromJson::from_json(obj.get("title").ok_or("missing field")?)?,
            file: FromJson::from_json(obj.get("file").ok_or("missing field")?)?,
            line: FromJson::from_json(obj.get("line").ok_or("missing field")?)?,
            code_excerpt: FromJson::from_json(obj.get("code_excerpt").ok_or("missing field")?)?,
            explanation: FromJson::from_json(obj.get("explanation").ok_or("missing field")?)?,
            confidence: FromJson::from_json(obj.get("confidence").ok_or("missing field")?)?,
            recommendation: FromJson::from_json(obj.get("recommendation").ok_or("missing field")?)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl ToJson for Severity {
    fn to_json(&self) -> JsonValue {
        let s = match self {
            Severity::Critical => "Critical",
            Severity::High => "High",
            Severity::Medium => "Medium",
            Severity::Low => "Low",
            Severity::Info => "Info",
        };
        JsonValue::String(s.into())
    }
}

impl FromJson for Severity {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let s = value.as_str().ok_or("expected string")?;
        match s {
            "Critical" => Ok(Severity::Critical),
            "High" => Ok(Severity::High),
            "Medium" => Ok(Severity::Medium),
            "Low" => Ok(Severity::Low),
            "Info" => Ok(Severity::Info),
            _ => Err("invalid severity".into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestInfo {
    pub total: usize,
    pub unit_tests: usize,
    pub integration_tests: usize,
    pub doc_tests: usize,
    pub ignored: usize,
    pub passing: Option<usize>,
    pub failing: Option<usize>,
    pub last_run_status: Option<String>,
    pub last_run_commit: Option<String>,
}

impl ToJson for TestInfo {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("total".into(), self.total.to_json());
        map.insert("unit_tests".into(), self.unit_tests.to_json());
        map.insert("integration_tests".into(), self.integration_tests.to_json());
        map.insert("doc_tests".into(), self.doc_tests.to_json());
        map.insert("ignored".into(), self.ignored.to_json());
        map.insert("passing".into(), self.passing.to_json());
        map.insert("failing".into(), self.failing.to_json());
        map.insert("last_run_status".into(), self.last_run_status.to_json());
        map.insert("last_run_commit".into(), self.last_run_commit.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for TestInfo {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(TestInfo {
            total: FromJson::from_json(obj.get("total").ok_or("missing field")?)?,
            unit_tests: FromJson::from_json(obj.get("unit_tests").ok_or("missing field")?)?,
            integration_tests: FromJson::from_json(obj.get("integration_tests").ok_or("missing field")?)?,
            doc_tests: FromJson::from_json(obj.get("doc_tests").ok_or("missing field")?)?,
            ignored: FromJson::from_json(obj.get("ignored").ok_or("missing field")?)?,
            passing: FromJson::from_json(obj.get("passing").ok_or("missing field")?)?,
            failing: FromJson::from_json(obj.get("failing").ok_or("missing field")?)?,
            last_run_status: FromJson::from_json(obj.get("last_run_status").ok_or("missing field")?)?,
            last_run_commit: FromJson::from_json(obj.get("last_run_commit").ok_or("missing field")?)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FootprintInfo {
    pub binary_size: Option<usize>,
    pub stripped_size: Option<usize>,
    pub compressed_size: Option<usize>,
    pub idle_rss: Option<usize>,
    pub peak_rss: Option<usize>,
    pub threads: Option<usize>,
    pub open_fds: Option<usize>,
    pub target_triple: Option<String>,
    pub measured: bool,
}

impl ToJson for FootprintInfo {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("binary_size".into(), self.binary_size.to_json());
        map.insert("stripped_size".into(), self.stripped_size.to_json());
        map.insert("compressed_size".into(), self.compressed_size.to_json());
        map.insert("idle_rss".into(), self.idle_rss.to_json());
        map.insert("peak_rss".into(), self.peak_rss.to_json());
        map.insert("threads".into(), self.threads.to_json());
        map.insert("open_fds".into(), self.open_fds.to_json());
        map.insert("target_triple".into(), self.target_triple.to_json());
        map.insert("measured".into(), self.measured.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for FootprintInfo {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(FootprintInfo {
            binary_size: FromJson::from_json(obj.get("binary_size").ok_or("missing field")?)?,
            stripped_size: FromJson::from_json(obj.get("stripped_size").ok_or("missing field")?)?,
            compressed_size: FromJson::from_json(obj.get("compressed_size").ok_or("missing field")?)?,
            idle_rss: FromJson::from_json(obj.get("idle_rss").ok_or("missing field")?)?,
            peak_rss: FromJson::from_json(obj.get("peak_rss").ok_or("missing field")?)?,
            threads: FromJson::from_json(obj.get("threads").ok_or("missing field")?)?,
            open_fds: FromJson::from_json(obj.get("open_fds").ok_or("missing field")?)?,
            target_triple: FromJson::from_json(obj.get("target_triple").ok_or("missing field")?)?,
            measured: FromJson::from_json(obj.get("measured").ok_or("missing field")?)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkArtifact {
    pub name: String,
    pub path: PathBuf,
    pub scenario: Option<String>,
    pub commit: Option<String>,
}

impl ToJson for BenchmarkArtifact {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("name".into(), self.name.to_json());
        map.insert("path".into(), self.path.to_json());
        map.insert("scenario".into(), self.scenario.to_json());
        map.insert("commit".into(), self.commit.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for BenchmarkArtifact {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(BenchmarkArtifact {
            name: FromJson::from_json(obj.get("name").ok_or("missing field")?)?,
            path: FromJson::from_json(obj.get("path").ok_or("missing field")?)?,
            scenario: FromJson::from_json(obj.get("scenario").ok_or("missing field")?)?,
            commit: FromJson::from_json(obj.get("commit").ok_or("missing field")?)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StandardCoverage {
    pub standard: String,
    pub status: String,
    pub code_refs: Vec<String>,
    pub tests: Vec<String>,
    pub confidence: Confidence,
    pub notes: Option<String>,
}

impl ToJson for StandardCoverage {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("standard".into(), self.standard.to_json());
        map.insert("status".into(), self.status.to_json());
        map.insert("code_refs".into(), self.code_refs.to_json());
        map.insert("tests".into(), self.tests.to_json());
        map.insert("confidence".into(), self.confidence.to_json());
        map.insert("notes".into(), self.notes.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for StandardCoverage {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(StandardCoverage {
            standard: FromJson::from_json(obj.get("standard").ok_or("missing field")?)?,
            status: FromJson::from_json(obj.get("status").ok_or("missing field")?)?,
            code_refs: FromJson::from_json(obj.get("code_refs").ok_or("missing field")?)?,
            tests: FromJson::from_json(obj.get("tests").ok_or("missing field")?)?,
            confidence: FromJson::from_json(obj.get("confidence").ok_or("missing field")?)?,
            notes: FromJson::from_json(obj.get("notes").ok_or("missing field")?)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct VisibilityReport {
    pub visible_count: usize,
    pub hidden_count: usize,
    pub public_surface: String,
    pub blocked_refs: Vec<String>,
    pub suspicious_visible: Vec<String>,
}

impl ToJson for VisibilityReport {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("visible_count".into(), self.visible_count.to_json());
        map.insert("hidden_count".into(), self.hidden_count.to_json());
        map.insert("public_surface".into(), self.public_surface.to_json());
        map.insert("blocked_refs".into(), self.blocked_refs.to_json());
        map.insert("suspicious_visible".into(), self.suspicious_visible.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for VisibilityReport {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(VisibilityReport {
            visible_count: FromJson::from_json(obj.get("visible_count").ok_or("missing field")?)?,
            hidden_count: FromJson::from_json(obj.get("hidden_count").ok_or("missing field")?)?,
            public_surface: FromJson::from_json(obj.get("public_surface").ok_or("missing field")?)?,
            blocked_refs: FromJson::from_json(obj.get("blocked_refs").ok_or("missing field")?)?,
            suspicious_visible: FromJson::from_json(obj.get("suspicious_visible").ok_or("missing field")?)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CrateReport {
    pub identity: CrateIdentity,
    pub dependencies: Vec<Dependency>,
    pub public_api: Vec<ApiItem>,
    pub call_graph: Vec<CallGraphEdge>,
    pub security_findings: Vec<SecurityFinding>,
    pub test_info: TestInfo,
    pub footprint: FootprintInfo,
    pub benchmarks: Vec<BenchmarkArtifact>,
    pub standards: Vec<StandardCoverage>,
    pub visibility: VisibilityReport,
    pub generated_at: String,
    pub parser_confidence: String,
}

impl ToJson for CrateReport {
    fn to_json(&self) -> JsonValue {
        let mut map = edgerun_json::Map::new();
        map.insert("identity".into(), self.identity.to_json());
        map.insert("dependencies".into(), self.dependencies.to_json());
        map.insert("public_api".into(), self.public_api.to_json());
        map.insert("call_graph".into(), self.call_graph.to_json());
        map.insert("security_findings".into(), self.security_findings.to_json());
        map.insert("test_info".into(), self.test_info.to_json());
        map.insert("footprint".into(), self.footprint.to_json());
        map.insert("benchmarks".into(), self.benchmarks.to_json());
        map.insert("standards".into(), self.standards.to_json());
        map.insert("visibility".into(), self.visibility.to_json());
        map.insert("generated_at".into(), self.generated_at.to_json());
        map.insert("parser_confidence".into(), self.parser_confidence.to_json());
        JsonValue::Object(map)
    }
}

impl FromJson for CrateReport {
    fn from_json(value: JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let obj = {
        let obj = value.as_object().ok_or("expected object")?;
        Ok(obj
    )?.ok_or("expected object")?;
        Ok(CrateReport {
            identity: FromJson::from_json(obj.get("identity").ok_or("missing field")?)?,
            dependencies: FromJson::from_json(obj.get("dependencies").ok_or("missing field")?)?,
            public_api: FromJson::from_json(obj.get("public_api").ok_or("missing field")?)?,
            call_graph: FromJson::from_json(obj.get("call_graph").ok_or("missing field")?)?,
            security_findings: FromJson::from_json(obj.get("security_findings").ok_or("missing field")?)?,
            test_info: FromJson::from_json(obj.get("test_info").ok_or("missing field")?)?,
            footprint: FromJson::from_json(obj.get("footprint").ok_or("missing field")?)?,
            benchmarks: FromJson::from_json(obj.get("benchmarks").ok_or("missing field")?)?,
            standards: FromJson::from_json(obj.get("standards").ok_or("missing field")?)?,
            visibility: FromJson::from_json(obj.get("visibility").ok_or("missing field")?)?,
            generated_at: FromJson::from_json(obj.get("generated_at").ok_or("missing field")?)?,
            parser_confidence: FromJson::from_json(obj.get("parser_confidence").ok_or("missing field")?)?,
        })
    }
}