// Custom binary protocol for codeanalyzer - no protobuf, no external dependencies

// Binary format uses simple TLV (Tag-Length-Value) encoding:
// - Tag: 1 byte for field identification
// - Length: variable-length encoding for strings/byte arrays
// - Value: the actual data

// Primitive types:
// - bool: 1 byte (0 = false, 1 = true)
// - u32: 4 bytes little-endian
// - u64: 8 bytes little-endian
// - f32: 4 bytes little-endian
// - f64: 8 bytes little-endian
// - string: length (u32) + bytes (UTF-8)
// - optional<T>: 1 byte presence flag + value if present
// - repeated<T>: count (u32) + values
// - message: series of fields with tags
// - oneof: tag indicating which variant + value

pub mod binary {
    use std::io::{Read, Write};

    pub fn encode_bool<W: Write>(w: &mut W, v: bool) -> std::io::Result<()> {
        w.write_all(&[if v { 1u8 } else { 0u8 }])
    }

    pub fn decode_bool<R: Read>(r: &mut R) -> std::io::Result<bool> {
        let mut buf = [0u8];
        r.read_exact(&mut buf)?;
        Ok(buf[0] != 0)
    }

    pub fn encode_u32<W: Write>(w: &mut W, v: u32) -> std::io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }

    pub fn decode_u32<R: Read>(r: &mut R) -> std::io::Result<u32> {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn encode_u64<W: Write>(w: &mut W, v: u64) -> std::io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }

    pub fn decode_u64<R: Read>(r: &mut R) -> std::io::Result<u64> {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    pub fn encode_f32<W: Write>(w: &mut W, v: f32) -> std::io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }

    pub fn decode_f32<R: Read>(r: &mut R) -> std::io::Result<f32> {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    pub fn encode_f64<W: Write>(w: &mut W, v: f64) -> std::io::Result<()> {
        w.write_all(&v.to_le_bytes())
    }

    pub fn decode_f64<R: Read>(r: &mut R) -> std::io::Result<f64> {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(f64::from_le_bytes(buf))
    }

    pub fn encode_string<W: Write>(w: &mut W, v: &str) -> std::io::Result<()> {
        let bytes = v.as_bytes();
        encode_u32(w, bytes.len() as u32)?;
        w.write_all(bytes)
    }

    pub fn decode_string<R: Read>(r: &mut R) -> std::io::Result<String> {
        let len = decode_u32(r)? as usize;
        let mut buf = vec![0u8; len];
        r.read_exact(&mut buf)?;
        String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn encode_optional_bool<W: Write>(w: &mut W, v: &Option<bool>) -> std::io::Result<()> {
        match v {
            Some(val) => {
                encode_bool(w, true)?;
                encode_bool(w, *val)
            }
            None => encode_bool(w, false),
        }
    }

    pub fn decode_optional_bool<R: Read>(r: &mut R) -> std::io::Result<Option<bool>> {
        if decode_bool(r)? {
            Ok(Some(decode_bool(r)?))
        } else {
            Ok(None)
        }
    }

    pub fn encode_optional_u32<W: Write>(w: &mut W, v: &Option<u32>) -> std::io::Result<()> {
        match v {
            Some(val) => {
                encode_bool(w, true)?;
                encode_u32(w, *val)
            }
            None => encode_bool(w, false),
        }
    }

    pub fn decode_optional_u32<R: Read>(r: &mut R) -> std::io::Result<Option<u32>> {
        if decode_bool(r)? {
            Ok(Some(decode_u32(r)?))
        } else {
            Ok(None)
        }
    }

    pub fn encode_optional_u64<W: Write>(w: &mut W, v: &Option<u64>) -> std::io::Result<()> {
        match v {
            Some(val) => {
                encode_bool(w, true)?;
                encode_u64(w, *val)
            }
            None => encode_bool(w, false),
        }
    }

    pub fn decode_optional_u64<R: Read>(r: &mut R) -> std::io::Result<Option<u64>> {
        if decode_bool(r)? {
            Ok(Some(decode_u64(r)?))
        } else {
            Ok(None)
        }
    }

    pub fn encode_optional_string<W: Write>(w: &mut W, v: &Option<String>) -> std::io::Result<()> {
        match v {
            Some(val) => {
                encode_bool(w, true)?;
                encode_string(w, val)
            }
            None => encode_bool(w, false),
        }
    }

    pub fn decode_optional_string<R: Read>(r: &mut R) -> std::io::Result<Option<String>> {
        if decode_bool(r)? {
            Ok(Some(decode_string(r)?))
        } else {
            Ok(None)
        }
    }

    pub fn encode_repeated_bool<W: Write>(w: &mut W, v: &[bool]) -> std::io::Result<()> {
        encode_u32(w, v.len() as u32)?;
        for &val in v {
            encode_bool(w, val)?;
        }
        Ok(())
    }

    pub fn decode_repeated_bool<R: Read>(r: &mut R) -> std::io::Result<Vec<bool>> {
        let len = decode_u32(r)? as usize;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(decode_bool(r)?);
        }
        Ok(result)
    }

    pub fn encode_repeated_u32<W: Write>(w: &mut W, v: &[u32]) -> std::io::Result<()> {
        encode_u32(w, v.len() as u32)?;
        for &val in v {
            encode_u32(w, val)?;
        }
        Ok(())
    }

    pub fn decode_repeated_u32<R: Read>(r: &mut R) -> std::io::Result<Vec<u32>> {
        let len = decode_u32(r)? as usize;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(decode_u32(r)?);
        }
        Ok(result)
    }

    pub fn encode_repeated_u64<W: Write>(w: &mut W, v: &[u64]) -> std::io::Result<()> {
        encode_u32(w, v.len() as u32)?;
        for &val in v {
            encode_u64(w, val)?;
        }
        Ok(())
    }

    pub fn decode_repeated_u64<R: Read>(r: &mut R) -> std::io::Result<Vec<u64>> {
        let len = decode_u32(r)? as usize;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(decode_u64(r)?);
        }
        Ok(result)
    }

    pub fn encode_repeated_string<W: Write>(w: &mut W, v: &[String]) -> std::io::Result<()> {
        encode_u32(w, v.len() as u32)?;
        for val in v {
            encode_string(w, val)?;
        }
        Ok(())
    }

    pub fn decode_repeated_string<R: Read>(r: &mut R) -> std::io::Result<Vec<String>> {
        let len = decode_u32(r)? as usize;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(decode_string(r)?);
        }
        Ok(result)
    }

    pub fn encode_bytes<W: Write>(w: &mut W, v: &[u8]) -> std::io::Result<()> {
        encode_u32(w, v.len() as u32)?;
        w.write_all(v)
    }

    pub fn decode_bytes<R: Read>(r: &mut R) -> std::io::Result<Vec<u8>> {
        let len = decode_u32(r)? as usize;
        let mut buf = vec![0u8; len];
        r.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn encode_map_string_u32<W: Write>(
        w: &mut W,
        v: &std::collections::HashMap<String, u32>,
    ) -> std::io::Result<()> {
        encode_u32(w, v.len() as u32)?;
        for (key, val) in v {
            encode_string(w, key)?;
            encode_u32(w, *val)?;
        }
        Ok(())
    }

    pub fn decode_map_string_u32<R: Read>(
        r: &mut R,
    ) -> std::io::Result<std::collections::HashMap<String, u32>> {
        let len = decode_u32(r)? as usize;
        let mut result = std::collections::HashMap::with_capacity(len);
        for _ in 0..len {
            let key = decode_string(r)?;
            let val = decode_u32(r)?;
            result.insert(key, val);
        }
        Ok(result)
    }
}

// GraphNode message
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

impl GraphNode {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.id)?;
        encode_string(w, &self.name)?;
        encode_string(w, &self.file)?;
        encode_string(w, &self.language)?;
        encode_bool(w, self.is_static)?;
        encode_u32(w, self.connections)?;
        encode_repeated_string(w, &self.tags)?;
        encode_optional_string(w, &self.commit)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(GraphNode {
            id: decode_string(r)?,
            name: decode_string(r)?,
            file: decode_string(r)?,
            language: decode_string(r)?,
            is_static: decode_bool(r)?,
            connections: decode_u32(r)?,
            tags: decode_repeated_string(r)?,
            commit: decode_optional_string(r)?,
        })
    }
}

// GraphEdge message
#[derive(Clone, PartialEq, Debug)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
}

impl GraphEdge {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.source)?;
        encode_string(w, &self.target)?;
        encode_string(w, &self.kind)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(GraphEdge {
            source: decode_string(r)?,
            target: decode_string(r)?,
            kind: decode_string(r)?,
        })
    }
}

// TagGroup message
#[derive(Clone, PartialEq, Debug)]
pub struct TagGroup {
    pub name: String,
    pub tags: Vec<String>,
    pub color: String,
}

impl TagGroup {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.name)?;
        encode_repeated_string(w, &self.tags)?;
        encode_string(w, &self.color)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(TagGroup {
            name: decode_string(r)?,
            tags: decode_repeated_string(r)?,
            color: decode_string(r)?,
        })
    }
}

// GraphData message
#[derive(Clone, PartialEq, Debug)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub tag_groups: Vec<TagGroup>,
    pub total_bytes: u64,
    pub node_count: u32,
    pub edge_count: u32,
}

impl GraphData {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.nodes.len() as u32)?;
        for node in &self.nodes {
            node.encode(w)?;
        }
        encode_u32(w, self.edges.len() as u32)?;
        for edge in &self.edges {
            edge.encode(w)?;
        }
        encode_u32(w, self.tag_groups.len() as u32)?;
        for tg in &self.tag_groups {
            tg.encode(w)?;
        }
        encode_u64(w, self.total_bytes)?;
        encode_u32(w, self.node_count)?;
        encode_u32(w, self.edge_count)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let nodes_len = decode_u32(r)? as usize;
        let mut nodes = Vec::with_capacity(nodes_len);
        for _ in 0..nodes_len {
            nodes.push(GraphNode::decode(r)?);
        }
        let edges_len = decode_u32(r)? as usize;
        let mut edges = Vec::with_capacity(edges_len);
        for _ in 0..edges_len {
            edges.push(GraphEdge::decode(r)?);
        }
        let tag_groups_len = decode_u32(r)? as usize;
        let mut tag_groups = Vec::with_capacity(tag_groups_len);
        for _ in 0..tag_groups_len {
            tag_groups.push(TagGroup::decode(r)?);
        }
        Ok(GraphData {
            nodes,
            edges,
            tag_groups,
            total_bytes: decode_u64(r)?,
            node_count: decode_u32(r)?,
            edge_count: decode_u32(r)?,
        })
    }
}

// Diagnostic message
#[derive(Clone, PartialEq, Debug)]
pub struct Diagnostic {
    pub file: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub severity: String,
    pub message: String,
    pub source: String,
}

impl Diagnostic {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.file)?;
        encode_optional_u32(w, &self.line)?;
        encode_optional_u32(w, &self.column)?;
        encode_string(w, &self.severity)?;
        encode_string(w, &self.message)?;
        encode_string(w, &self.source)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(Diagnostic {
            file: decode_string(r)?,
            line: decode_optional_u32(r)?,
            column: decode_optional_u32(r)?,
            severity: decode_string(r)?,
            message: decode_string(r)?,
            source: decode_string(r)?,
        })
    }
}

// TestResult message
#[derive(Clone, PartialEq, Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub file: Option<String>,
    pub message: Option<String>,
    pub duration_ms: Option<u64>,
}

impl TestResult {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.name)?;
        encode_bool(w, self.passed)?;
        encode_optional_string(w, &self.file)?;
        encode_optional_string(w, &self.message)?;
        encode_optional_u64(w, &self.duration_ms)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(TestResult {
            name: decode_string(r)?,
            passed: decode_bool(r)?,
            file: decode_optional_string(r)?,
            message: decode_optional_string(r)?,
            duration_ms: decode_optional_u64(r)?,
        })
    }
}

// CoverageInfo message
#[derive(Clone, PartialEq, Debug)]
pub struct CoverageInfo {
    pub file: String,
    pub lines_total: u32,
    pub lines_covered: u32,
    pub coverage_pct: f64,
}

impl CoverageInfo {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.file)?;
        encode_u32(w, self.lines_total)?;
        encode_u32(w, self.lines_covered)?;
        encode_f64(w, self.coverage_pct)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(CoverageInfo {
            file: decode_string(r)?,
            lines_total: decode_u32(r)?,
            lines_covered: decode_u32(r)?,
            coverage_pct: decode_f64(r)?,
        })
    }
}

// FormatIssue message
#[derive(Clone, PartialEq, Debug)]
pub struct FormatIssue {
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
}

impl FormatIssue {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.file)?;
        encode_optional_u32(w, &self.line)?;
        encode_string(w, &self.message)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(FormatIssue {
            file: decode_string(r)?,
            line: decode_optional_u32(r)?,
            message: decode_string(r)?,
        })
    }
}

// MissingRef message
#[derive(Clone, PartialEq, Debug)]
pub struct MissingRef {
    pub name: String,
    pub referenced_by: String,
    pub kind: String,
}

impl MissingRef {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.name)?;
        encode_string(w, &self.referenced_by)?;
        encode_string(w, &self.kind)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(MissingRef {
            name: decode_string(r)?,
            referenced_by: decode_string(r)?,
            kind: decode_string(r)?,
        })
    }
}

// DiagnosticsReport message
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

impl DiagnosticsReport {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_repeated_string(w, &self.linters_available)?;
        encode_u32(w, self.diagnostics.len() as u32)?;
        for d in &self.diagnostics {
            d.encode(w)?;
        }
        encode_u32(w, self.tests.len() as u32)?;
        for t in &self.tests {
            t.encode(w)?;
        }
        encode_u32(w, self.coverage.len() as u32)?;
        for c in &self.coverage {
            c.encode(w)?;
        }
        encode_u32(w, self.formatting_issues.len() as u32)?;
        for f in &self.formatting_issues {
            f.encode(w)?;
        }
        encode_u32(w, self.missing_references.len() as u32)?;
        for m in &self.missing_references {
            m.encode(w)?;
        }
        encode_string(w, &self.scan_timestamp)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let linters = decode_repeated_string(r)?;
        let diagnostics_len = decode_u32(r)? as usize;
        let mut diagnostics = Vec::with_capacity(diagnostics_len);
        for _ in 0..diagnostics_len {
            diagnostics.push(Diagnostic::decode(r)?);
        }
        let tests_len = decode_u32(r)? as usize;
        let mut tests = Vec::with_capacity(tests_len);
        for _ in 0..tests_len {
            tests.push(TestResult::decode(r)?);
        }
        let coverage_len = decode_u32(r)? as usize;
        let mut coverage = Vec::with_capacity(coverage_len);
        for _ in 0..coverage_len {
            coverage.push(CoverageInfo::decode(r)?);
        }
        let formatting_len = decode_u32(r)? as usize;
        let mut formatting_issues = Vec::with_capacity(formatting_len);
        for _ in 0..formatting_len {
            formatting_issues.push(FormatIssue::decode(r)?);
        }
        let missing_len = decode_u32(r)? as usize;
        let mut missing_references = Vec::with_capacity(missing_len);
        for _ in 0..missing_len {
            missing_references.push(MissingRef::decode(r)?);
        }
        Ok(DiagnosticsReport {
            linters_available: linters,
            diagnostics,
            tests,
            coverage,
            formatting_issues,
            missing_references,
            scan_timestamp: decode_string(r)?,
        })
    }
}

// IndexState oneof variants
#[derive(Clone, PartialEq, Debug)]
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
#[derive(Clone, PartialEq, Debug)]
pub struct IndexState {
    pub state: Option<IndexStateEnum>,
}

impl IndexState {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        match &self.state {
            Some(IndexStateEnum::NotIndexed) => {
                encode_u32(w, 1)?;
            }
            Some(IndexStateEnum::Indexing {
                progress,
                files_scanned,
                total_files,
            }) => {
                encode_u32(w, 2)?;
                encode_f32(w, *progress)?;
                encode_u32(w, *files_scanned)?;
                encode_u32(w, *total_files)?;
            }
            Some(IndexStateEnum::Indexed {
                functions,
                edges,
                index_time_ms,
            }) => {
                encode_u32(w, 3)?;
                encode_u32(w, *functions)?;
                encode_u32(w, *edges)?;
                encode_u64(w, *index_time_ms)?;
            }
            Some(IndexStateEnum::Stale {
                functions,
                edges,
                last_indexed,
            }) => {
                encode_u32(w, 4)?;
                encode_u32(w, *functions)?;
                encode_u32(w, *edges)?;
                encode_u64(w, *last_indexed)?;
            }
            Some(IndexStateEnum::Error { message }) => {
                encode_u32(w, 5)?;
                encode_string(w, message)?;
            }
            None => {
                encode_u32(w, 0)?;
            }
        }
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let tag = decode_u32(r)?;
        let state = match tag {
            1 => Some(IndexStateEnum::NotIndexed),
            2 => Some(IndexStateEnum::Indexing {
                progress: decode_f32(r)?,
                files_scanned: decode_u32(r)?,
                total_files: decode_u32(r)?,
            }),
            3 => Some(IndexStateEnum::Indexed {
                functions: decode_u32(r)?,
                edges: decode_u32(r)?,
                index_time_ms: decode_u64(r)?,
            }),
            4 => Some(IndexStateEnum::Stale {
                functions: decode_u32(r)?,
                edges: decode_u32(r)?,
                last_indexed: decode_u64(r)?,
            }),
            5 => Some(IndexStateEnum::Error {
                message: decode_string(r)?,
            }),
            _ => None,
        };
        Ok(IndexState { state })
    }
}

// RepoInfo message
#[derive(Clone, PartialEq, Debug)]
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

impl RepoInfo {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.path)?;
        encode_string(w, &self.name)?;
        encode_bool(w, self.is_git_repo)?;
        encode_optional_string(w, &self.git_remote)?;
        encode_u32(w, self.file_count)?;
        encode_u64(w, self.total_size_bytes)?;
        encode_u64(w, self.last_modified)?;
        encode_optional_index_state(w, &self.index_state)?;
        encode_repeated_string(w, &self.languages)?;
        encode_u64(w, self.added_at)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(RepoInfo {
            path: decode_string(r)?,
            name: decode_string(r)?,
            is_git_repo: decode_bool(r)?,
            git_remote: decode_optional_string(r)?,
            file_count: decode_u32(r)?,
            total_size_bytes: decode_u64(r)?,
            last_modified: decode_u64(r)?,
            index_state: decode_optional_index_state(r)?,
            languages: decode_repeated_string(r)?,
            added_at: decode_u64(r)?,
        })
    }
}

fn encode_optional_index_state<W: std::io::Write>(
    w: &mut W,
    v: &Option<IndexState>,
) -> std::io::Result<()> {
    use binary::*;
    match v {
        Some(val) => {
            encode_bool(w, true)?;
            val.encode(w)
        }
        None => encode_bool(w, false),
    }
}

fn decode_optional_index_state<R: std::io::Read>(r: &mut R) -> std::io::Result<Option<IndexState>> {
    use binary::*;
    if decode_bool(r)? {
        Ok(Some(IndexState::decode(r)?))
    } else {
        Ok(None)
    }
}

// RegistryConfig message
#[derive(Clone, PartialEq, Debug)]
pub struct RegistryConfig {
    pub scan_roots: Vec<String>,
    pub excluded_paths: Vec<String>,
    pub max_concurrent_index: u32,
    pub auto_discover: bool,
}

impl RegistryConfig {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_repeated_string(w, &self.scan_roots)?;
        encode_repeated_string(w, &self.excluded_paths)?;
        encode_u32(w, self.max_concurrent_index)?;
        encode_bool(w, self.auto_discover)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(RegistryConfig {
            scan_roots: decode_repeated_string(r)?,
            excluded_paths: decode_repeated_string(r)?,
            max_concurrent_index: decode_u32(r)?,
            auto_discover: decode_bool(r)?,
        })
    }
}

// RepoList message
#[derive(Clone, PartialEq, Debug)]
pub struct RepoList {
    pub repos: Vec<RepoInfo>,
    pub active: Option<String>,
}

impl RepoList {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.repos.len() as u32)?;
        for repo in &self.repos {
            repo.encode(w)?;
        }
        encode_optional_string(w, &self.active)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let repos_len = decode_u32(r)? as usize;
        let mut repos = Vec::with_capacity(repos_len);
        for _ in 0..repos_len {
            repos.push(RepoInfo::decode(r)?);
        }
        Ok(RepoList {
            repos,
            active: decode_optional_string(r)?,
        })
    }
}

// FileInfo message
#[derive(Clone, PartialEq, Debug)]
pub struct FileInfo {
    pub path: String,
    pub hash: u64,
    pub modified_ts: u64,
    pub language: String,
    pub size: u64,
}

impl FileInfo {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.path)?;
        encode_u64(w, self.hash)?;
        encode_u64(w, self.modified_ts)?;
        encode_string(w, &self.language)?;
        encode_u64(w, self.size)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(FileInfo {
            path: decode_string(r)?,
            hash: decode_u64(r)?,
            modified_ts: decode_u64(r)?,
            language: decode_string(r)?,
            size: decode_u64(r)?,
        })
    }
}

// FileEntry message
#[derive(Clone, PartialEq, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

impl FileEntry {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.name)?;
        encode_string(w, &self.path)?;
        encode_bool(w, self.is_dir)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(FileEntry {
            name: decode_string(r)?,
            path: decode_string(r)?,
            is_dir: decode_bool(r)?,
        })
    }
}

// DirListing message
#[derive(Clone, PartialEq, Debug)]
pub struct DirListing {
    pub entries: Vec<FileEntry>,
}

impl DirListing {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.entries.len() as u32)?;
        for entry in &self.entries {
            entry.encode(w)?;
        }
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let entries_len = decode_u32(r)? as usize;
        let mut entries = Vec::with_capacity(entries_len);
        for _ in 0..entries_len {
            entries.push(FileEntry::decode(r)?);
        }
        Ok(DirListing { entries })
    }
}

// QueryMatch message
#[derive(Clone, PartialEq, Debug)]
pub struct QueryMatch {
    pub id: String,
    pub name: String,
    pub file: String,
    pub language: String,
}

impl QueryMatch {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.id)?;
        encode_string(w, &self.name)?;
        encode_string(w, &self.file)?;
        encode_string(w, &self.language)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(QueryMatch {
            id: decode_string(r)?,
            name: decode_string(r)?,
            file: decode_string(r)?,
            language: decode_string(r)?,
        })
    }
}

// QueryResult message
#[derive(Clone, PartialEq, Debug)]
pub struct QueryResult {
    pub matches: Vec<QueryMatch>,
    pub total_count: u32,
}

impl QueryResult {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.matches.len() as u32)?;
        for m in &self.matches {
            m.encode(w)?;
        }
        encode_u32(w, self.total_count)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let matches_len = decode_u32(r)? as usize;
        let mut matches = Vec::with_capacity(matches_len);
        for _ in 0..matches_len {
            matches.push(QueryMatch::decode(r)?);
        }
        Ok(QueryResult {
            matches,
            total_count: decode_u32(r)?,
        })
    }
}

// StatsResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct StatsResponse {
    pub languages: std::collections::HashMap<String, u32>,
    pub files: std::collections::HashMap<String, u32>,
    pub total_functions: u32,
    pub total_edges: u32,
}

impl StatsResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_map_string_u32(w, &self.languages)?;
        encode_map_string_u32(w, &self.files)?;
        encode_u32(w, self.total_functions)?;
        encode_u32(w, self.total_edges)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(StatsResponse {
            languages: decode_map_string_u32(r)?,
            files: decode_map_string_u32(r)?,
            total_functions: decode_u32(r)?,
            total_edges: decode_u32(r)?,
        })
    }
}

// XrefResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct XrefResponse {
    pub function: String,
    pub callers: Vec<String>,
    pub callees: Vec<String>,
}

impl XrefResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.function)?;
        encode_repeated_string(w, &self.callers)?;
        encode_repeated_string(w, &self.callees)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(XrefResponse {
            function: decode_string(r)?,
            callers: decode_repeated_string(r)?,
            callees: decode_repeated_string(r)?,
        })
    }
}

// FileData message
#[derive(Clone, PartialEq, Debug)]
pub struct FileData {
    pub file: String,
    pub functions: Vec<String>,
    pub dependencies: Vec<String>,
}

impl FileData {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.file)?;
        encode_repeated_string(w, &self.functions)?;
        encode_repeated_string(w, &self.dependencies)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(FileData {
            file: decode_string(r)?,
            functions: decode_repeated_string(r)?,
            dependencies: decode_repeated_string(r)?,
        })
    }
}

// FilesResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct FilesResponse {
    pub files: Vec<FileData>,
}

impl FilesResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.files.len() as u32)?;
        for f in &self.files {
            f.encode(w)?;
        }
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let files_len = decode_u32(r)? as usize;
        let mut files = Vec::with_capacity(files_len);
        for _ in 0..files_len {
            files.push(FileData::decode(r)?);
        }
        Ok(FilesResponse { files })
    }
}

// AnalyzeResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct AnalyzeResponse {
    pub status: String,
    pub path: String,
    pub functions: u32,
    pub edges: u32,
}

impl AnalyzeResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.status)?;
        encode_string(w, &self.path)?;
        encode_u32(w, self.functions)?;
        encode_u32(w, self.edges)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(AnalyzeResponse {
            status: decode_string(r)?,
            path: decode_string(r)?,
            functions: decode_u32(r)?,
            edges: decode_u32(r)?,
        })
    }
}

// ApplyEditResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct ApplyEditResponse {
    pub status: String,
    pub path: String,
    pub bytes: u32,
}

impl ApplyEditResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.status)?;
        encode_string(w, &self.path)?;
        encode_u32(w, self.bytes)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(ApplyEditResponse {
            status: decode_string(r)?,
            path: decode_string(r)?,
            bytes: decode_u32(r)?,
        })
    }
}

// ChatRequest message
#[derive(Clone, PartialEq, Debug)]
pub struct ChatRequest {
    pub message: String,
    pub context: Option<String>,
    pub summary: Option<String>,
    pub session_id: Option<String>,
}

impl ChatRequest {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.message)?;
        encode_optional_string(w, &self.context)?;
        encode_optional_string(w, &self.summary)?;
        encode_optional_string(w, &self.session_id)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(ChatRequest {
            message: decode_string(r)?,
            context: decode_optional_string(r)?,
            summary: decode_optional_string(r)?,
            session_id: decode_optional_string(r)?,
        })
    }
}

// ChatResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct ChatResponse {
    pub reply: String,
    pub error: Option<String>,
}

impl ChatResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.reply)?;
        encode_optional_string(w, &self.error)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(ChatResponse {
            reply: decode_string(r)?,
            error: decode_optional_string(r)?,
        })
    }
}

// ToolCallInfo message
#[derive(Clone, PartialEq, Debug)]
pub struct ToolCallInfo {
    pub name: String,
    pub args: String,
    pub output: String,
    pub content: Option<String>,
}

impl ToolCallInfo {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.name)?;
        encode_string(w, &self.args)?;
        encode_string(w, &self.output)?;
        encode_optional_string(w, &self.content)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(ToolCallInfo {
            name: decode_string(r)?,
            args: decode_string(r)?,
            output: decode_string(r)?,
            content: decode_optional_string(r)?,
        })
    }
}

// ToolUseResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct ToolUseResponse {
    pub reply: String,
    pub tools: Vec<ToolCallInfo>,
    pub error: Option<String>,
}

impl ToolUseResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.reply)?;
        encode_u32(w, self.tools.len() as u32)?;
        for t in &self.tools {
            t.encode(w)?;
        }
        encode_optional_string(w, &self.error)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let reply = decode_string(r)?;
        let tools_len = decode_u32(r)? as usize;
        let mut tools = Vec::with_capacity(tools_len);
        for _ in 0..tools_len {
            tools.push(ToolCallInfo::decode(r)?);
        }
        let error = decode_optional_string(r)?;
        Ok(ToolUseResponse {
            reply,
            tools,
            error,
        })
    }
}

// StatusResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct StatusResponse {
    pub status: String,
    pub error: Option<String>,
    pub functions: Option<u32>,
    pub edges: Option<u32>,
    pub found: Option<u32>,
}

impl StatusResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.status)?;
        encode_optional_string(w, &self.error)?;
        encode_optional_u32(w, &self.functions)?;
        encode_optional_u32(w, &self.edges)?;
        encode_optional_u32(w, &self.found)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(StatusResponse {
            status: decode_string(r)?,
            error: decode_optional_string(r)?,
            functions: decode_optional_u32(r)?,
            edges: decode_optional_u32(r)?,
            found: decode_optional_u32(r)?,
        })
    }
}

// ErrorResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.error)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(ErrorResponse {
            error: decode_string(r)?,
        })
    }
}

// ViewNode message
#[derive(Clone, PartialEq, Debug)]
pub struct ViewNode {
    pub id: String,
    pub name: String,
    pub r#type: String,
}

impl ViewNode {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.id)?;
        encode_string(w, &self.name)?;
        encode_string(w, &self.r#type)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(ViewNode {
            id: decode_string(r)?,
            name: decode_string(r)?,
            r#type: decode_string(r)?,
        })
    }
}

// ViewEdge message
#[derive(Clone, PartialEq, Debug)]
pub struct ViewEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
}

impl ViewEdge {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.source)?;
        encode_string(w, &self.target)?;
        encode_string(w, &self.kind)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(ViewEdge {
            source: decode_string(r)?,
            target: decode_string(r)?,
            kind: decode_string(r)?,
        })
    }
}

// ViewData message
#[derive(Clone, PartialEq, Debug)]
pub struct ViewData {
    pub nodes: Vec<ViewNode>,
    pub edges: Vec<ViewEdge>,
}

impl ViewData {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.nodes.len() as u32)?;
        for n in &self.nodes {
            n.encode(w)?;
        }
        encode_u32(w, self.edges.len() as u32)?;
        for e in &self.edges {
            e.encode(w)?;
        }
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let nodes_len = decode_u32(r)? as usize;
        let mut nodes = Vec::with_capacity(nodes_len);
        for _ in 0..nodes_len {
            nodes.push(ViewNode::decode(r)?);
        }
        let edges_len = decode_u32(r)? as usize;
        let mut edges = Vec::with_capacity(edges_len);
        for _ in 0..edges_len {
            edges.push(ViewEdge::decode(r)?);
        }
        Ok(ViewData { nodes, edges })
    }
}

// GetFileResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct GetFileResponse {
    pub content: String,
}

impl GetFileResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.content)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(GetFileResponse {
            content: decode_string(r)?,
        })
    }
}

// GetViewResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct GetViewResponse {
    pub data: Option<ViewData>,
}

impl GetViewResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_optional_view_data(w, &self.data)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(GetViewResponse {
            data: decode_optional_view_data(r)?,
        })
    }
}

fn encode_optional_view_data<W: std::io::Write>(
    w: &mut W,
    v: &Option<ViewData>,
) -> std::io::Result<()> {
    use binary::*;
    match v {
        Some(val) => {
            encode_bool(w, true)?;
            val.encode(w)
        }
        None => encode_bool(w, false),
    }
}

fn decode_optional_view_data<R: std::io::Read>(r: &mut R) -> std::io::Result<Option<ViewData>> {
    use binary::*;
    if decode_bool(r)? {
        Ok(Some(ViewData::decode(r)?))
    } else {
        Ok(None)
    }
}

// GetReposResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct GetReposResponse {
    pub repos: Option<RepoList>,
}

impl GetReposResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_optional_repo_list(w, &self.repos)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(GetReposResponse {
            repos: decode_optional_repo_list(r)?,
        })
    }
}

fn encode_optional_repo_list<W: std::io::Write>(
    w: &mut W,
    v: &Option<RepoList>,
) -> std::io::Result<()> {
    use binary::*;
    match v {
        Some(val) => {
            encode_bool(w, true)?;
            val.encode(w)
        }
        None => encode_bool(w, false),
    }
}

fn decode_optional_repo_list<R: std::io::Read>(r: &mut R) -> std::io::Result<Option<RepoList>> {
    use binary::*;
    if decode_bool(r)? {
        Ok(Some(RepoList::decode(r)?))
    } else {
        Ok(None)
    }
}

// SwitchRepoResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct SwitchRepoResponse {
    pub status: Option<StatusResponse>,
}

impl SwitchRepoResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_optional_status_response(w, &self.status)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(SwitchRepoResponse {
            status: decode_optional_status_response(r)?,
        })
    }
}

fn encode_optional_status_response<W: std::io::Write>(
    w: &mut W,
    v: &Option<StatusResponse>,
) -> std::io::Result<()> {
    use binary::*;
    match v {
        Some(val) => {
            encode_bool(w, true)?;
            val.encode(w)
        }
        None => encode_bool(w, false),
    }
}

fn decode_optional_status_response<R: std::io::Read>(
    r: &mut R,
) -> std::io::Result<Option<StatusResponse>> {
    use binary::*;
    if decode_bool(r)? {
        Ok(Some(StatusResponse::decode(r)?))
    } else {
        Ok(None)
    }
}

// GetFilesResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct GetFilesResponse {
    pub files: Vec<FileData>,
}

impl GetFilesResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.files.len() as u32)?;
        for f in &self.files {
            f.encode(w)?;
        }
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let files_len = decode_u32(r)? as usize;
        let mut files = Vec::with_capacity(files_len);
        for _ in 0..files_len {
            files.push(FileData::decode(r)?);
        }
        Ok(GetFilesResponse { files })
    }
}

// GetDiagnosticsResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct GetDiagnosticsResponse {
    pub diagnostics: Vec<Diagnostic>,
}

impl GetDiagnosticsResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.diagnostics.len() as u32)?;
        for d in &self.diagnostics {
            d.encode(w)?;
        }
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let diagnostics_len = decode_u32(r)? as usize;
        let mut diagnostics = Vec::with_capacity(diagnostics_len);
        for _ in 0..diagnostics_len {
            diagnostics.push(Diagnostic::decode(r)?);
        }
        Ok(GetDiagnosticsResponse { diagnostics })
    }
}

// AddRepoResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct AddRepoResponse {
    pub status: Option<StatusResponse>,
    pub path: String,
}

impl AddRepoResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_optional_status_response(w, &self.status)?;
        encode_string(w, &self.path)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(AddRepoResponse {
            status: decode_optional_status_response(r)?,
            path: decode_string(r)?,
        })
    }
}

// RemoveRepoResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct RemoveRepoResponse {
    pub status: Option<StatusResponse>,
}

impl RemoveRepoResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_optional_status_response(w, &self.status)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(RemoveRepoResponse {
            status: decode_optional_status_response(r)?,
        })
    }
}

// DiscoverReposResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct DiscoverReposResponse {
    pub found: u32,
    pub repos: Option<RepoList>,
}

impl DiscoverReposResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.found)?;
        encode_optional_repo_list(w, &self.repos)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(DiscoverReposResponse {
            found: decode_u32(r)?,
            repos: decode_optional_repo_list(r)?,
        })
    }
}

// GetConfigResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct GetConfigResponse {
    pub config: Option<RegistryConfig>,
}

impl GetConfigResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_optional_registry_config(w, &self.config)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(GetConfigResponse {
            config: decode_optional_registry_config(r)?,
        })
    }
}

fn encode_optional_registry_config<W: std::io::Write>(
    w: &mut W,
    v: &Option<RegistryConfig>,
) -> std::io::Result<()> {
    use binary::*;
    match v {
        Some(val) => {
            encode_bool(w, true)?;
            val.encode(w)
        }
        None => encode_bool(w, false),
    }
}

fn decode_optional_registry_config<R: std::io::Read>(
    r: &mut R,
) -> std::io::Result<Option<RegistryConfig>> {
    use binary::*;
    if decode_bool(r)? {
        Ok(Some(RegistryConfig::decode(r)?))
    } else {
        Ok(None)
    }
}

// UpdateConfigResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct UpdateConfigResponse {
    pub status: Option<StatusResponse>,
}

impl UpdateConfigResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_optional_status_response(w, &self.status)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(UpdateConfigResponse {
            status: decode_optional_status_response(r)?,
        })
    }
}

// GetIndexStateResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct GetIndexStateResponse {
    pub state: Option<IndexState>,
}

impl GetIndexStateResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_optional_index_state(w, &self.state)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(GetIndexStateResponse {
            state: decode_optional_index_state(r)?,
        })
    }
}

// GetWsConnectionsResponse message
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GetWsConnectionsResponse {
    pub count: u32,
}

impl GetWsConnectionsResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.count)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(GetWsConnectionsResponse {
            count: decode_u32(r)?,
        })
    }
}

// WsRequest message
#[derive(Clone, PartialEq, Debug)]
pub struct WsRequest {
    pub id: u64,
    pub action: String,
    pub payload: Vec<u8>,
}

impl WsRequest {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u64(w, self.id)?;
        encode_string(w, &self.action)?;
        encode_bytes(w, &self.payload)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(WsRequest {
            id: decode_u64(r)?,
            action: decode_string(r)?,
            payload: decode_bytes(r)?,
        })
    }
}

// WsResponse message
#[derive(Clone, PartialEq, Debug)]
pub struct WsResponse {
    pub id: u64,
    pub result: Option<WsResponseResult>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum WsResponseResult {
    Data(Vec<u8>),
    Error(String),
}

impl WsResponse {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u64(w, self.id)?;
        match &self.result {
            Some(WsResponseResult::Data(data)) => {
                encode_u32(w, 1)?;
                encode_bytes(w, data)?;
            }
            Some(WsResponseResult::Error(err)) => {
                encode_u32(w, 2)?;
                encode_string(w, err)?;
            }
            None => {
                encode_u32(w, 0)?;
            }
        }
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let id = decode_u64(r)?;
        let tag = decode_u32(r)?;
        let result = match tag {
            1 => Some(WsResponseResult::Data(decode_bytes(r)?)),
            2 => Some(WsResponseResult::Error(decode_string(r)?)),
            _ => None,
        };
        Ok(WsResponse { id, result })
    }
}

// GraphUpdate messages
#[derive(Clone, PartialEq, Debug)]
pub struct NodeUpdate {
    pub added: Vec<GraphNode>,
    pub modified: Vec<GraphNode>,
    pub removed: Vec<String>,
}

impl NodeUpdate {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.added.len() as u32)?;
        for n in &self.added {
            n.encode(w)?;
        }
        encode_u32(w, self.modified.len() as u32)?;
        for n in &self.modified {
            n.encode(w)?;
        }
        encode_repeated_string(w, &self.removed)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let added_len = decode_u32(r)? as usize;
        let mut added = Vec::with_capacity(added_len);
        for _ in 0..added_len {
            added.push(GraphNode::decode(r)?);
        }
        let modified_len = decode_u32(r)? as usize;
        let mut modified = Vec::with_capacity(modified_len);
        for _ in 0..modified_len {
            modified.push(GraphNode::decode(r)?);
        }
        let removed = decode_repeated_string(r)?;
        Ok(NodeUpdate {
            added,
            modified,
            removed,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct EdgeUpdate {
    pub added: Vec<GraphEdge>,
    pub removed: Vec<String>,
}

impl EdgeUpdate {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u32(w, self.added.len() as u32)?;
        for e in &self.added {
            e.encode(w)?;
        }
        encode_repeated_string(w, &self.removed)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let added_len = decode_u32(r)? as usize;
        let mut added = Vec::with_capacity(added_len);
        for _ in 0..added_len {
            added.push(GraphEdge::decode(r)?);
        }
        let removed = decode_repeated_string(r)?;
        Ok(EdgeUpdate { added, removed })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct GraphUpdate {
    pub nodes: Option<NodeUpdate>,
    pub edges: Option<EdgeUpdate>,
    pub is_full: bool,
    pub timestamp: u64,
}

impl GraphUpdate {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_optional_node_update(w, &self.nodes)?;
        encode_optional_edge_update(w, &self.edges)?;
        encode_bool(w, self.is_full)?;
        encode_u64(w, self.timestamp)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(GraphUpdate {
            nodes: decode_optional_node_update(r)?,
            edges: decode_optional_edge_update(r)?,
            is_full: decode_bool(r)?,
            timestamp: decode_u64(r)?,
        })
    }
}

fn encode_optional_node_update<W: std::io::Write>(
    w: &mut W,
    v: &Option<NodeUpdate>,
) -> std::io::Result<()> {
    use binary::*;
    match v {
        Some(val) => {
            encode_bool(w, true)?;
            val.encode(w)
        }
        None => encode_bool(w, false),
    }
}

fn decode_optional_node_update<R: std::io::Read>(r: &mut R) -> std::io::Result<Option<NodeUpdate>> {
    use binary::*;
    if decode_bool(r)? {
        Ok(Some(NodeUpdate::decode(r)?))
    } else {
        Ok(None)
    }
}

fn encode_optional_edge_update<W: std::io::Write>(
    w: &mut W,
    v: &Option<EdgeUpdate>,
) -> std::io::Result<()> {
    use binary::*;
    match v {
        Some(val) => {
            encode_bool(w, true)?;
            val.encode(w)
        }
        None => encode_bool(w, false),
    }
}

fn decode_optional_edge_update<R: std::io::Read>(r: &mut R) -> std::io::Result<Option<EdgeUpdate>> {
    use binary::*;
    if decode_bool(r)? {
        Ok(Some(EdgeUpdate::decode(r)?))
    } else {
        Ok(None)
    }
}

// WsMessage types
#[derive(Clone, PartialEq, Debug)]
pub struct ClientReady;

#[derive(Clone, PartialEq, Debug)]
pub struct DebugLog {
    pub level: String,
    pub message: String,
}

impl DebugLog {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.level)?;
        encode_string(w, &self.message)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(DebugLog {
            level: decode_string(r)?,
            message: decode_string(r)?,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct EvalCommand {
    pub id: u64,
    pub code: String,
}

impl EvalCommand {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u64(w, self.id)?;
        encode_string(w, &self.code)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(EvalCommand {
            id: decode_u64(r)?,
            code: decode_string(r)?,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct EvalResult {
    pub id: u64,
    pub result: Option<String>,
    pub error: Option<String>,
}

impl EvalResult {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_u64(w, self.id)?;
        encode_optional_string(w, &self.result)?;
        encode_optional_string(w, &self.error)?;
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(EvalResult {
            id: decode_u64(r)?,
            result: decode_optional_string(r)?,
            error: decode_optional_string(r)?,
        })
    }
}

// WsMessage oneof
#[derive(Clone, PartialEq, Debug)]
pub enum WsMessageEnum {
    Request(WsRequest),
    Response(WsResponse),
    GraphData(GraphData),
    GraphUpdate(GraphUpdate),
    RawJson(Vec<u8>),
    EvalCommand(EvalCommand),
    EvalResult(EvalResult),
    ClientReady(ClientReady),
    DebugLog(DebugLog),
}

// WsMessage message
#[derive(Clone, PartialEq, Debug)]
pub struct WsMessage {
    pub msg: Option<WsMessageEnum>,
}

impl WsMessage {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        match &self.msg {
            Some(WsMessageEnum::Request(req)) => {
                encode_u32(w, 1)?;
                req.encode(w)?;
            }
            Some(WsMessageEnum::Response(resp)) => {
                encode_u32(w, 2)?;
                resp.encode(w)?;
            }
            Some(WsMessageEnum::GraphData(gd)) => {
                encode_u32(w, 3)?;
                gd.encode(w)?;
            }
            Some(WsMessageEnum::GraphUpdate(gu)) => {
                encode_u32(w, 4)?;
                gu.encode(w)?;
            }
            Some(WsMessageEnum::RawJson(bytes)) => {
                encode_u32(w, 5)?;
                encode_bytes(w, bytes)?;
            }
            Some(WsMessageEnum::EvalCommand(ec)) => {
                encode_u32(w, 6)?;
                ec.encode(w)?;
            }
            Some(WsMessageEnum::EvalResult(er)) => {
                encode_u32(w, 7)?;
                er.encode(w)?;
            }
            Some(WsMessageEnum::ClientReady(_)) => {
                encode_u32(w, 8)?;
            }
            Some(WsMessageEnum::DebugLog(dl)) => {
                encode_u32(w, 9)?;
                dl.encode(w)?;
            }
            None => {
                encode_u32(w, 0)?;
            }
        }
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let tag = decode_u32(r)?;
        let msg = match tag {
            1 => Some(WsMessageEnum::Request(WsRequest::decode(r)?)),
            2 => Some(WsMessageEnum::Response(WsResponse::decode(r)?)),
            3 => Some(WsMessageEnum::GraphData(GraphData::decode(r)?)),
            4 => Some(WsMessageEnum::GraphUpdate(GraphUpdate::decode(r)?)),
            5 => Some(WsMessageEnum::RawJson(decode_bytes(r)?)),
            6 => Some(WsMessageEnum::EvalCommand(EvalCommand::decode(r)?)),
            7 => Some(WsMessageEnum::EvalResult(EvalResult::decode(r)?)),
            8 => Some(WsMessageEnum::ClientReady(ClientReady)),
            9 => Some(WsMessageEnum::DebugLog(DebugLog::decode(r)?)),
            _ => None,
        };
        Ok(WsMessage { msg })
    }
}

// WsFrame types
#[derive(Clone, PartialEq, Debug)]
pub struct BinaryFrame {
    pub data: Vec<u8>,
}

impl BinaryFrame {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_bytes(w, &self.data)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(BinaryFrame {
            data: decode_bytes(r)?,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct TextFrame {
    pub text: String,
}

impl TextFrame {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        encode_string(w, &self.text)
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        Ok(TextFrame {
            text: decode_string(r)?,
        })
    }
}

// WsFrame oneof
#[derive(Clone, PartialEq, Debug)]
pub enum WsFrameEnum {
    Binary(BinaryFrame),
    Text(TextFrame),
}

// WsFrame message
#[derive(Clone, PartialEq, Debug)]
pub struct WsFrame {
    pub frame_type: Option<WsFrameEnum>,
}

impl WsFrame {
    pub fn encode<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        use binary::*;
        match &self.frame_type {
            Some(WsFrameEnum::Binary(bf)) => {
                encode_u32(w, 1)?;
                bf.encode(w)?;
            }
            Some(WsFrameEnum::Text(tf)) => {
                encode_u32(w, 2)?;
                tf.encode(w)?;
            }
            None => {
                encode_u32(w, 0)?;
            }
        }
        Ok(())
    }

    pub fn decode<R: std::io::Read>(r: &mut R) -> std::io::Result<Self> {
        use binary::*;
        let tag = decode_u32(r)?;
        let frame_type = match tag {
            1 => Some(WsFrameEnum::Binary(BinaryFrame::decode(r)?)),
            2 => Some(WsFrameEnum::Text(TextFrame::decode(r)?)),
            _ => None,
        };
        Ok(WsFrame { frame_type })
    }
}
