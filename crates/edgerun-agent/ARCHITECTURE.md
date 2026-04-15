# edgerun-agent v2: Hardware-Optimized AI Coding Agent

## Hardware Utilization Strategy

### Available Hardware
- **CPU**: AMD Ryzen 7 7840U (8 cores / 16 threads, Zen 4)
- **iGPU**: AMD Radeon 780M (RDNA 3, 12 CUs)
- **NPU**: AMD XDNA AI Engine (4 TOPS INT8)
- **dGPU**: Discrete GPU with 16GB VRAM (running 24B dense model via TabbyAPI)

### Constraint Analysis
- **Context Window**: 32k tokens (~24k chars conservative)
- **VRAM**: 16GB (fully utilized by 24B model at FP16/INT8)
- **Model**: 24B dense parameters (cannot increase)

### Optimization Goals
1. Maximize hardware utilization across CPU+iGPU+NPU+dGPU
2. Reduce context pressure through intelligent caching and retrieval
3. Offload tasks to appropriate hardware accelerators
4. Maintain sub-100ms latency for common operations

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         edgerun-agent v2                            │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │                    Application Layer                          │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │ │
│  │  │ Web Server   │  │ Chat API     │  │ Tool Router  │        │ │
│  │  │ (Static +    │  │ (Streaming)  │  │ (Intelligent │        │ │
│  │  │  WebSocket)  │  │              │  │  Dispatch)   │        │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘        │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                              ↕                                      │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │                   Intelligence Layer                          │ │
│  │                                                               │ │
│  │  ┌─────────────────┐              ┌─────────────────┐        │ │
│  │  │  Small Model    │◄────────────►│  Context Manager│        │ │
│  │  │  (1-3B on iGPU) │              │  + RAG Engine   │        │ │
│  │  │                 │              │                 │        │ │
│  │  │  • Intent       │              │  • Embedding    │        │ │
│  │  │  • Routing      │              │    Index (NPU)  │        │ │
│  │  │  • Validation   │              │  • Semantic     │        │ │
│  │  │  • Simple tasks │              │    Search       │        │ │
│  │  └─────────────────┘              └─────────────────┘        │ │
│  │           ↕                              ↕                    │ │
│  │  ┌─────────────────────────────────────────────────┐          │ │
│  │  │        Large Model (24B on dGPU via Tabby)      │          │ │
│  │  │                                                 │          │ │
│  │  │  • Code generation  • Complex refactoring       │          │ │
│  │  │  • Architecture     • Multi-file reasoning      │          │ │
│  │  └─────────────────────────────────────────────────┘          │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                              ↕                                      │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │                   Tooling Layer                               │ │
│  │                                                               │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │ │
│  │  │ AST Editor   │  │ Code         │  │ Diagnostics  │        │ │
│  │  │ (syn+quote)  │  │ Analyzer     │  │ Engine       │        │ │
│  │  │              │  │ (tree-sitter)│  │              │        │ │
│  │  │  • Parse     │  │              │  │  • Linting   │        │ │
│  │  │  • Transform │  │  • Call graph│  │  • Type check│        │ │
│  │  │  • Refactor  │  │  • Symbols   │  │  • Errors    │        │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘        │ │
│  │                                                               │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │ │
│  │  │ Filesystem   │  │ Git          │  │ Build        │        │ │
│  │  │ Operations   │  │ Integration  │  │ Tools        │        │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘        │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                              ↕                                      │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │                   Caching Layer                               │ │
│  │                                                               │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │ │
│  │  │ Response     │  │ Code         │  │ Embedding    │        │ │
│  │  │ Cache        │  │ Embeddings   │  │ Vector DB    │        │ │
│  │  │ (semantic)   │  │ (file-level) │  │ (FAISS/HNSW) │        │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘        │ │
│  └───────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────────────┐
│                      Hardware Acceleration                          │
│                                                                     │
│  CPU (Zen 4)          iGPU (RDNA3)       NPU (XDNA)       dGPU     │
│  ───────────          ────────────       ──────────       ────     │
│  • Tool execution     • Embedding        • Vector         • 24B    │
│  • AST parsing          inference          search           LM     │
│  • RAG queries        • Small LM         • Semantic       • Code   │
│  • Git operations       (1-3B)             ranking          gen    │
│  • Compression        • FP16/INT8        • INT8/FP8                │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Key Innovations

### 1. Dual-Model Architecture

**Small Model (1-3B) on iGPU**
- Runs locally via `candle` or `ort` (ONNX Runtime)
- Handles:
  - Intent classification (route to tool vs LLM)
  - Simple Q&A (factual, no generation)
  - Output validation (syntax, type checking)
  - Embedding generation for RAG
- Latency: <50ms for most tasks
- VRAM usage: ~2-4GB (leaves room for context cache)

**Large Model (24B) on dGPU**
- Remote TabbyAPI server
- Handles:
  - Complex code generation
  - Multi-file refactoring
  - Architecture decisions
  - Creative problem-solving
- Context-optimized: only receives compressed, relevant context

### 2. NPU-Accelerated Semantic Search

**Embedding Pipeline**
```
File Change → Embedding Model (NPU) → Vector Index → Semantic Search
     ↓                                        ↓
  Update cache                          Query during chat
```

- **Model**: All-MiniLM-L6-v2 (384 dim, INT8 quantized)
- **Index**: HNSW (Hierarchical Navigable Small World)
- **Latency**: <10ms for similarity search across 10k code snippets
- **NPU utilization**: 4 TOPS for batch embedding generation

### 3. Context Compression Strategies

**Hierarchical Context Management**
```
┌────────────────────────────────────────────┐
│ Working Memory (2k tokens)                 │
│ - Last 3 conversation turns                │
│ - Current file being edited                │
│ - Immediate task context                   │
└────────────────────────────────────────────┘
              ↕ compress/summarize
┌────────────────────────────────────────────┐
│ Short-term Memory (8k tokens)              │
│ - Conversation summary (semantic)          │
│ - Related files (embeddings matched)       │
│ - Recent tool results                      │
└────────────────────────────────────────────┘
              ↕ retrieve on-demand
┌────────────────────────────────────────────┐
│ Long-term Memory (external index)          │
│ - Full codebase embeddings                 │
│ - Historical conversations                 │
│ - Project documentation                    │
└────────────────────────────────────────────┘
```

**Compression Techniques**
1. **Semantic Summarization**: Small model summarizes old turns
2. **AST Abstraction**: Store function signatures, not bodies
3. **Embedding-based Retrieval**: Only fetch relevant context
4. **KV Cache Prefix Caching**: Reuse system prompt + common context

### 4. AST-Safe Editing Pipeline

**From edgerun-edit**
```rust
// LLM generates high-level intent
intent = "rename function foo to bar"

// Agent translates to AST operation
edit = RenameFunction {
    file: "src/lib.rs",
    old_name: "foo",
    new_name: "bar"
}

// AST engine applies transformation
parsed = syn::parse_file(source)
transformed = edit.apply(parsed)
formatted = prettyplease::unparse(transformed)

// Validation before write
if cargo_check(formatted).success {
    write_file(formatted)
}
```

**Benefits**
- No syntax errors from string replacement
- Type-safe refactoring across files
- Automatic formatting
- Rollback capability

### 5. Intelligent Tool Routing

**Small Model as Router**
```
User: "Fix the compilation error in main.rs"
          ↓
  Small Model (iGPU)
          ↓
  Intent: RUN_TOOL
  Tool: run_cargo_check
  Args: {file: "src/main.rs"}
          ↓
  Execute tool → Get errors
          ↓
  Small Model validates output
          ↓
  If complex → Route to Large Model
  If simple → Generate fix directly
```

**Routing Decisions**
| Task Type | Route To | Reason |
|-----------|----------|--------|
| File read/list | Tool | Fast, deterministic |
| Cargo check/test | Tool | Needs compiler |
| Simple rename | AST Editor | Syntax-safe |
| Bug fix (1-2 lines) | Small Model | Low latency |
| New feature | Large Model | Complex reasoning |
| Refactor (multi-file) | Large Model | Cross-file context |

### 6. Response Caching with Semantic Similarity

**Cache Architecture**
```
Request → Embed (NPU) → Similarity Search → Cache Hit?
  ↓                                          ↓
  ↓                                    Return cached
  ↓                                    (with adaptation)
  ↓                                          ↓
  └──────────────→ Cache Miss ──────────────┘
              ↓
         Generate (Large Model)
              ↓
         Store in cache (key: embedding)
```

**Cache Hit Scenarios**
- Similar code generation requests (e.g., "write a JSON parser" → "write a YAML parser")
- Repeated debugging questions
- Common patterns (boilerplate, tests, error handling)

**Hit Rate Target**: 30-40% for typical development sessions

---

## Implementation Plan

### Phase 1: Foundation (Week 1-2)
- [x] Remove tokio dependency (completed)
- [ ] Merge edgerun-edit AST operations
- [ ] Merge codeanalyzer tree-sitter parsing
- [ ] Unified tool system

### Phase 2: Embedding & RAG (Week 3-4)
- [ ] Integrate `candle-transformers` for embeddings
- [ ] Build vector index (HNSW via `hnswlib` or `usearch`)
- [ ] NPU acceleration via `ort` (ONNX Runtime)
- [ ] Semantic search API

### Phase 3: Dual-Model Support (Week 5-6)
- [ ] Small model inference on iGPU (1-3B)
- [ ] Intent classification pipeline
- [ ] Smart router (tool vs small LM vs large LM)
- [ ] Output validation layer

### Phase 4: Context Optimization (Week 7-8)
- [ ] Hierarchical memory system
- [ ] Semantic summarization of old turns
- [ ] AST-based code compression
- [ ] KV cache prefix caching

### Phase 5: Caching & Performance (Week 9-10)
- [ ] Semantic response cache
- [ ] Embedding cache for files
- [ ] Incremental re-embedding on file changes
- [ ] Performance benchmarks

---

## Expected Performance Gains

| Metric | Current | Target | Improvement |
|--------|---------|--------|-------------|
| Simple Q&A latency | 2-5s | <100ms | 20-50x |
| Context efficiency | 32k raw | ~100k effective | 3x |
| Cache hit rate | 0% | 30-40% | - |
| Tool routing accuracy | N/A | >90% | - |
| Syntax errors in edits | ~5% | <0.1% | 50x |
| Multi-file refactors | Manual | AST-automated | 10x |

---

## Dependencies to Add

```toml
[dependencies]
# Embedding & ML
candle-core = "0.8"
candle-transformers = "0.8"
candle-nn = "0.8"
ort = { version = "2.0", features = ["download-binaries"] }  # NPU
tokenizers = "0.19"

# Vector search
usearch = "2.0"  # or hnswlib

# Tree-sitter (from codeanalyzer)
tree-sitter = "0.22"
tree-sitter-rust = "0.21"
tree-sitter-typescript = "0.21"
tree-sitter-c = "0.21"
tree-sitter-python = "0.21"

# AST editing (from edgerun-edit)
syn = { version = "2", features = ["full", "extra-traits"] }
prettyplease = "0.2"
quote = "1"
proc-macro2 = "1"
```

---

## Hardware Utilization Targets

| Component | Current | Target | Utilization |
|-----------|---------|--------|-------------|
| dGPU (24B LM) | ~80% | ~95% | Optimized batching |
| iGPU | 0% | 60-80% | Embeddings + small LM |
| NPU | 0% | 70-90% | Vector search + ranking |
| CPU | ~30% | 80-100% | Tools + AST + RAG |

**Power Efficiency**: Shift workload from dGPU to NPU/iGPU where possible, reducing overall power consumption by ~40%.

// Benchmark comment