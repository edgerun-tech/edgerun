#![allow(clippy::unwrap_used)]

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use edgerun_http::{
    handler::Handler,
    http1::upgrade::{is_websocket_upgrade, build_websocket_accept_headers},
    Request, Response, StatusCode, Method,
};

pub mod generated {
    pub mod codeanalyzer {
        include!("generated/codeanalyzer.rs");
    }
}

pub use generated::codeanalyzer::{
    GraphData, GraphEdge, GraphNode, TagGroup, DirListing, FileEntry,
    FilesResponse,
};

use crate::{analyzer, diagnostics};

pub struct AppState {
    pub graph: GraphData,
    pub graph_proto: Vec<u8>,
    pub root_dir: String,
    pub diagnostics_report: String,
}

impl AppState {
    pub fn new(root_dir: &str) -> Self {
        println!("[server] Analyzing {}...", root_dir);
        let (result, _changes) = analyzer::analyze_full(root_dir);
        let nc = result.program.functions.len();
        let ec = result.program.edges.len();
        println!("[server] {} functions, {} edges", nc, ec);

        let nodes: Vec<GraphNode> = result
            .program
            .functions
            .iter()
            .map(|(id, func)| GraphNode {
                id: id.to_legacy(),
                name: func.name.clone(),
                file: func.file.clone(),
                language: func.language.clone(),
                is_static: func.is_static,
                connections: 0,
                tags: Self::derive_tags(&func.file, &func.name, &func.language, func.is_static),
                commit: func.last_modified_commit.clone(),
            })
            .collect();

        let edges: Vec<GraphEdge> = result
            .program
            .edges
            .iter()
            .map(|e| GraphEdge {
                source: e.caller.clone(),
                target: e.callee.clone(),
                kind: e.kind.label().to_string(),
            })
            .collect();

        let tag_groups = vec![
            TagGroup {
                name: "Drivers".into(),
                tags: vec!["subsystem:drivers".into()],
                color: "#4CAF50".into(),
            },
            TagGroup {
                name: "Kernel".into(),
                tags: vec![
                    "subsystem:kernel".into(),
                    "subsystem:net".into(),
                    "subsystem:fs".into(),
                ],
                color: "#FF9800".into(),
            },
            TagGroup {
                name: "Memory".into(),
                tags: vec!["pattern:memory".into(), "pattern:alloc".into(), "pattern:buf".into()],
                color: "#2196F3".into(),
            },
            TagGroup {
                name: "IO".into(),
                tags: vec!["pattern:io".into(), "pattern:handler".into()],
                color: "#9C27B0".into(),
            },
            TagGroup {
                name: "Init".into(),
                tags: vec!["pattern:init".into()],
                color: "#E91E63".into(),
            },
            TagGroup {
                name: "Tests".into(),
                tags: vec!["pattern:test".into()],
                color: "#00BCD4".into(),
            },
            TagGroup {
                name: "Rust".into(),
                tags: vec!["lang:rust".into()],
                color: "#DEA584".into(),
            },
            TagGroup { name: "C".into(), tags: vec!["lang:c".into()], color: "#555555".into() },
            TagGroup {
                name: "Static".into(),
                tags: vec!["linkage:static".into()],
                color: "#795548".into(),
            },
        ];

        let graph = GraphData {
            nodes,
            edges,
            tag_groups,
            total_bytes: 0,
            node_count: nc as u32,
            edge_count: ec as u32,
        };

        let mut graph_proto = Vec::new();
        prost::Message::encode(&graph, &mut graph_proto).expect("graph serialization failed");

        println!("[server] Graph: {} bytes (proto)", graph_proto.len());

        let diagnostics_graph = diagnostics::GraphData {
            nodes: graph
                .nodes
                .iter()
                .map(|n| diagnostics::GraphNode { id: n.id.clone(), name: n.name.clone() })
                .collect(),
            edges: graph
                .edges
                .iter()
                .map(|e| diagnostics::GraphEdge {
                    source: e.source.clone(),
                    target: e.target.clone(),
                })
                .collect(),
        };

        println!("[server] Scanning for available linters/tools...");
        let _tools = diagnostics::scan_tools(root_dir);
        let diagnostics_report = diagnostics::get_diagnostics_summary(root_dir, &diagnostics_graph);

        Self { graph, graph_proto, root_dir: root_dir.to_string(), diagnostics_report }
    }

    fn derive_tags(file: &str, name: &str, language: &str, is_static: bool) -> Vec<String> {
        let mut tags: HashSet<String> = HashSet::new();

        tags.insert(format!("lang:{}", language));

        if is_static {
            tags.insert("linkage:static".to_string());
        } else {
            tags.insert("linkage:module".to_string());
        }

        if let Some(ext) = file.rsplit('.').next() {
            if !ext.is_empty() && ext.len() < 10 {
                tags.insert(format!("ext:{}", ext));
            }
        }

        let path_parts: Vec<&str> = file.split('/').collect();
        for part in path_parts.iter().take(4) {
            if !part.is_empty() && !part.contains('.') {
                tags.insert(format!("path:{}", part));
            }
        }

        if let Some(filename) = file.split('/').next_back() {
            if let Some(name_without_ext) = filename.rsplit('.').next() {
                if !name_without_ext.is_empty() {
                    tags.insert(format!("file:{}", name_without_ext));
                }
            }
        }

        if !name.is_empty() {
            if name.contains("handler") || name.contains("_h_") {
                tags.insert("pattern:handler".to_string());
            }
            if name.starts_with("init_") || name.contains("_init") {
                tags.insert("pattern:init".to_string());
            }
            if name.starts_with("test_") || name.ends_with("_test") || name.contains("_tests") {
                tags.insert("pattern:test".to_string());
            }
            if name.contains("ioctl") || name.contains("syscall") || name.contains("_io_") {
                tags.insert("pattern:io".to_string());
            }
            if name.contains("alloc")
                || name.contains("free")
                || name.contains("buf")
                || name.contains("mem")
            {
                tags.insert("pattern:memory".to_string());
            }
        }

        let mut tags: Vec<String> = tags.into_iter().collect();
        tags.sort();
        tags
    }
}

// Global state
pub static APP_STATE: Mutex<Option<AppState>> = Mutex::new(None);

// Main server handler
pub struct CodelyzerHandler {
    pub viewer_dir: String,
}

impl Handler for CodelyzerHandler {
    fn handle(&self, req: Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            let path = req.uri().path();
            let method = req.method();

            // Check for WebSocket upgrade
            if is_websocket_upgrade(req.headers()) {
                return Self::handle_websocket_upgrade(req);
            }

            // Route HTTP requests
            match (method, path) {
                (&Method::GET, "/") | (&Method::GET, "/index.html") => {
                    Self::serve_file(&format!("{}/index.html", self.viewer_dir)).await
                }
                (&Method::GET, path) if !path.starts_with("/api/") => {
                    let filepath = format!("{}{}", self.viewer_dir, path);
                    Self::serve_file(&filepath).await
                }
                (&Method::GET, "/api/graph.proto") => Self::handle_graph_proto().await,
                (&Method::GET, "/api/stats") => Self::handle_stats().await,
                (&Method::GET, "/api/fs") => Self::handle_fs(req).await,
                (&Method::GET, "/api/files") => Self::handle_files().await,
                _ => Response::not_found(),
            }
        })
    }
}

impl CodelyzerHandler {
    fn handle_websocket_upgrade(req: Request) -> Response {
        if let Some(ws_key) = req.headers().get("Sec-WebSocket-Key") {
            let accept_headers = build_websocket_accept_headers(ws_key.as_str());
            Response::new(StatusCode::new(101).unwrap())
                .with_header("Upgrade", "websocket")
                .with_header("Connection", "Upgrade")
                .with_header("Sec-WebSocket-Accept", 
                    accept_headers.get("Sec-WebSocket-Accept").unwrap().as_str())
        } else {
            Response::not_found()
        }
    }

    async fn serve_file(filepath: &str) -> Response {
        match tokio::fs::read(filepath).await {
            Ok(data) => {
                let mime = if filepath.ends_with(".html") {
                    "text/html; charset=utf-8"
                } else if filepath.ends_with(".css") {
                    "text/css; charset=utf-8"
                } else if filepath.ends_with(".js") || filepath.ends_with(".mjs") {
                    "application/javascript; charset=utf-8"
                } else if filepath.ends_with(".json") {
                    "application/json"
                } else if filepath.ends_with(".wasm") {
                    "application/wasm"
                } else if filepath.ends_with(".png") {
                    "image/png"
                } else if filepath.ends_with(".jpg") || filepath.ends_with(".jpeg") {
                    "image/jpeg"
                } else {
                    "application/octet-stream"
                };
                Response::new(StatusCode::OK)
                    .with_header("Content-Type", mime)
                    .with_header("Access-Control-Allow-Origin", "*")
                    .with_body(data)
            }
            Err(_) => Response::not_found(),
        }
    }

    async fn handle_graph_proto() -> Response {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(state) = state_guard.as_ref() {
            Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/octet-stream")
                .with_header("Access-Control-Allow-Origin", "*")
                .with_body(state.graph_proto.clone())
        } else {
            Response::not_found()
        }
    }

    async fn handle_stats() -> Response {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(state) = state_guard.as_ref() {
            let graph = &state.graph;
            
            let mut lang_counts: HashMap<String, usize> = HashMap::new();
            let mut file_set = HashSet::new();
            
            for node in &graph.nodes {
                *lang_counts.entry(node.language.clone()).or_insert(0) += 1;
                file_set.insert(&node.file);
            }
            
            let mut output = String::new();
            output.push_str("## Codebase Statistics\n\n");
            output.push_str(&format!("- Files: {}\n", file_set.len()));
            output.push_str(&format!("- Functions: {}\n", graph.nodes.len()));
            output.push_str(&format!("- Relationships: {}\n", graph.edges.len()));
            output.push_str("\n### Languages\n");
            
            let mut langs: Vec<_> = lang_counts.into_iter().collect();
            langs.sort_by(|a, b| b.1.cmp(&a.1));
            
            for (lang, count) in langs {
                output.push_str(&format!("- {}: {} functions\n", lang, count));
            }
            
            Response::new(StatusCode::OK)
                .with_header("Content-Type", "text/plain")
                .with_body(output)
        } else {
            Response::not_found()
        }
    }

    async fn handle_fs(req: Request) -> Response {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(state) = state_guard.as_ref() {
            let query = req.uri().query().unwrap_or("");
            let path = query.strip_prefix("path=")
                .map(Self::url_decode)
                .unwrap_or_else(|| "/".to_string());
            
            let full_path = if path == "/" {
                state.root_dir.clone()
            } else {
                format!("{}/{}", state.root_dir.trim_end_matches('/'), path.trim_start_matches('/'))
            };
            
            let mut entries: Vec<FileEntry> = Vec::new();
            if let Ok(dir) = std::fs::read_dir(&full_path) {
                for entry in dir.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.path().is_dir();
                    let entry_path = if path == "/" {
                        format!("/{}", name)
                    } else {
                        format!("{}/{}", path.trim_end_matches('/'), name)
                    };
                    entries.push(FileEntry { name, path: entry_path, is_dir });
                }
            }
            
            entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            });
            
            let dir_listing = DirListing { entries };
            let mut buf = Vec::new();
            if let Err(e) = prost::Message::encode(&dir_listing, &mut buf) {
                return Response::new(StatusCode::INTERNAL_SERVER_ERROR)
                    .with_body(format!("Encoding error: {}", e));
            }
            
            Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/octet-stream")
                .with_body(buf)
        } else {
            Response::not_found()
        }
    }

    async fn handle_files() -> Response {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(state) = state_guard.as_ref() {
            let mut files: Vec<String> = state.graph.nodes.iter()
                .map(|n| n.file.clone())
                .collect();
            files.sort();
            files.dedup();
            
            let response = FilesResponse { files };
            let mut buf = Vec::new();
            if let Err(e) = prost::Message::encode(&response, &mut buf) {
                return Response::new(StatusCode::INTERNAL_SERVER_ERROR)
                    .with_body(format!("Encoding error: {}", e));
            }
            
            Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/octet-stream")
                .with_body(buf)
        } else {
            Response::not_found()
        }
    }

    fn url_decode(s: &str) -> String {
        let mut out = String::new();
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(h) = u8::from_str_radix(&s[i+1..i+3], 16) {
                    out.push(h as char);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i] as char);
            i += 1;
        }
        out
    }
}
