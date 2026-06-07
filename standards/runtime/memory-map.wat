;; Unified Address Space Map — single source of truth for all memory offsets.
;; Include this fragment BEFORE any other fragment that references memory.
;; All offsets are relative to (memory (export "memory") 288).

;; ── Region: Character Classification LUTs (4 KB) ──────────────────────
(global $LUT_CHAR_CLASS (export "LUT_CHAR_CLASS") i32 (i32.const 0x1000))
(global $LUT_LOWER_MAP  (export "LUT_LOWER_MAP")  i32 (i32.const 0x2000))

;; ── Region: Runtime Scratch (4 KB) ───────────────────────────────────
;; Temporary buffers for crypto, codec, parser intermediates
(global $SCRATCH_BUF     (export "SCRATCH_BUF")     i32 (i32.const 0x3000))
(global $SCRATCH_SIZE    (export "SCRATCH_SIZE")    i32 (i32.const 0x1000))
(global $SHA256_OUT_BUF  (export "SHA256_OUT_BUF")  i32 (i32.const 0x5000))

;; ── Region: Pipeline Bump Allocator Heap (192 KB) ────────────────────
;; Pipes, pipeline descriptors, stage configs, intermediate buffers
(global $HEAP_START (export "HEAP_START") i32 (i32.const 0x40000))
(global $HEAP_END   (export "HEAP_END")   i32 (i32.const 0x70000))
(global $HEAP_SIZE  (export "HEAP_SIZE")  i32 (i32.const 0x30000))

;; ── Region: Interpreter Module State (128 KB) ────────────────────────
;; Error codes, scratch, type section, import section, function section
(global $INT_ERR        (export "INT_ERR")        i32 (i32.const 0x00000))
(global $INT_SCRATCH0   (export "INT_SCRATCH0")   i32 (i32.const 0x00008))
(global $INT_SCRATCH1   (export "INT_SCRATCH1")   i32 (i32.const 0x00010))
(global $INT_SCRATCH2   (export "INT_SCRATCH2")   i32 (i32.const 0x00018))
(global $INT_SCRATCH3   (export "INT_SCRATCH3")   i32 (i32.const 0x00020))
(global $INT_WASM_PTR   (export "INT_WASM_PTR")   i32 (i32.const 0x00040))
(global $INT_WASM_LEN   (export "INT_WASM_LEN")   i32 (i32.const 0x00048))
(global $INT_TYPE_COUNT (export "INT_TYPE_COUNT") i32 (i32.const 0x00100))
(global $INT_TYPE_BUF   (export "INT_TYPE_BUF")   i32 (i32.const 0x00108))
(global $INT_IMPORT_CNT (export "INT_IMPORT_CNT") i32 (i32.const 0x04108))
(global $INT_IMPORT_BUF (export "INT_IMPORT_BUF") i32 (i32.const 0x04110))
(global $INT_FUNC_BUF   (export "INT_FUNC_BUF")   i32 (i32.const 0x04518))
(global $INT_CODE_BUF   (export "INT_CODE_BUF")   i32 (i32.const 0x0551C))
(global $INT_FRAME_SAVE (export "INT_FRAME_SAVE") i32 (i32.const 0x70000))
(global $INT_SCRATCH_WK (export "INT_SCRATCH_WK") i32 (i32.const 0x80000))

;; ── Region: Interpreter Decoded Ops Cache (384 KB) ───────────────────
(global $INT_DECODED_OPS  (export "INT_DECODED_OPS")  i32 (i32.const 0xA0000))
(global $INT_DECODED_CNT  (export "INT_DECODED_CNT")  i32 (i32.const 0x15F08))
(global $INT_DEC_SZ       (export "INT_DEC_SZ")       i32 (i32.const 32))
(global $INT_DECODED_END  (export "INT_DECODED_END")  i32 (i32.const 0x100000))

;; ── Region: WAT Parser Workspace (32 KB) ────────────────────────────
(global $WAT_SRC_PTR   (export "WAT_SRC_PTR")   i32 (i32.const 0x8C000))
(global $WAT_SRC_LEN   (export "WAT_SRC_LEN")   i32 (i32.const 0x8C004))
(global $WAT_POS       (export "WAT_POS")       i32 (i32.const 0x8C008))
(global $WAT_SYMTAB    (export "WAT_SYMTAB")    i32 (i32.const 0x8B000))
(global $WAT_EMIT_BUF  (export "WAT_EMIT_BUF")  i32 (i32.const 0x8D000))

;; ── Region: Names Buffer (64 KB) ─────────────────────────────────────
(global $INT_NAMES_BUF (export "INT_NAMES_BUF") i32 (i32.const 0x90000))

;; ── Region: JIT Code Cache (1 MB) ───────────────────────────────────
(global $JIT_CACHE_BASE (export "JIT_CACHE_BASE") i32 (i32.const 0x100000))
(global $JIT_CACHE_SIZE (export "JIT_CACHE_SIZE") i32 (i32.const 0x100000))
(global $JIT_CACHE      (export "JIT_CACHE")      i32 (i32.const 0x100000))

;; ── Region: Guest WASM Heap (1 MB) ──────────────────────────────────
;; Shrunk to 1MB so it doesn't overlap JIT_STATE at 0x300000
(global $GUEST_HEAP_BASE (export "GUEST_HEAP_BASE") i32 (i32.const 0x200000))
(global $GUEST_HEAP_SIZE (export "GUEST_HEAP_SIZE") i32 (i32.const 0x100000))

;; ── Region: UI Writer Buffer (64 KB) ────────────────────────────────
(global $UI_WRITER_BASE (export "UI_WRITER_BASE") i32 (i32.const 0x30000))
(global $UI_WRITER_SIZE (export "UI_WRITER_SIZE") i32 (i32.const 0x10000))

;; ── Region: UI Command Buffer (64 KB) ───────────────────────────────
(global $UI_CMD_BUF (export "UI_CMD_BUF") i32 (i32.const 0x10000))
(global $UI_CMD_SIZE (export "UI_CMD_SIZE") i32 (i32.const 0x10000))

;; ── Region: Font Glyph Data (64 KB) ─────────────────────────────────
(global $FONT_GLYPH_BUF (export "FONT_GLYPH_BUF") i32 (i32.const 0x20000))
(global $FONT_GLYPH_SIZE (export "FONT_GLYPH_SIZE") i32 (i32.const 0x10000))

;; ── Region: Icon IR Bytecode (64 KB) ────────────────────────────────
(global $ICON_IR_BUF  (export "ICON_IR_BUF")  i32 (i32.const 0x30000))
(global $ICON_IR_SIZE (export "ICON_IR_SIZE") i32 (i32.const 0x10000))

;; ── Region: Protocol Parser Scratch (128 KB) ────────────────────────
;; At 0x1010000 (after test buffer area at 0x1000000-0x1005000)
(global $PROTO_SCRATCH (export "PROTO_SCRATCH") i32 (i32.const 0x1010000))
(global $PROTO_SCRATCH_SZ (export "PROTO_SCRATCH_SZ") i32 (i32.const 0x20000))

;; ── Region: Streaming Parser State (256 KB) ─────────────────────────
(global $STREAM_STATE_BUF (export "STREAM_STATE_BUF") i32 (i32.const 0xC0000))
(global $STREAM_STATE_SZ  (export "STREAM_STATE_SZ")  i32 (i32.const 0x40000))

;; ── Region: JIT State (1 MB @ 0x300000) ────────────────────────────
(global $JIT_STATE       (export "JIT_STATE")       i32 (i32.const 0x300000))

;; ── Region: Compiler Output Buffers (1 MB each) ────────────────────
(global $ELF_OUT_BUF     (export "ELF_OUT_BUF")     i32 (i32.const 0x400000))
(global $BIN_OUT_BUF     (export "BIN_OUT_BUF")     i32 (i32.const 0x500000))
(global $BSS_BUF         (export "BSS_BUF")         i32 (i32.const 0x600000))

;; ── Region: Model/Render Data (1 MB @ 0x700000) ────────────────────
(global $MODEL_DATA_BASE (export "MODEL_DATA_BASE") i32 (i32.const 0x700000))

;; ── Region: RGBA Framebuffer (8 MB @ 0x800000) ──────────────────────
;; 1920×1080×4 = 8.3 MB — we reserve 8 MB starting at 0x800000
(global $FB_BASE    (export "FB_BASE")    i32 (i32.const 0x800000))
(global $FB_SIZE    (export "FB_SIZE")    i32 (i32.const 0x800000))

;; ── Pipeline descriptor offset constants (aliased from pipe-core) ──
(global $PD_MAGIC    (export "PD_MAGIC")    i32 (i32.const 0))
(global $PD_VERSION  (export "PD_VERSION")  i32 (i32.const 4))
(global $PD_PIPE_CAP (export "PD_PIPE_CAP") i32 (i32.const 8))
(global $PD_COUNT    (export "PD_COUNT")    i32 (i32.const 12))
(global $PD_TICK     (export "PD_TICK")     i32 (i32.const 16))
(global $PD_FRAME    (export "PD_FRAME")    i32 (i32.const 20))
(global $PD_STAGES   (export "PD_STAGES")   i32 (i32.const 24))
(global $PS_TYPE     (export "PS_TYPE")     i32 (i32.const 0))
(global $PS_CONFIG   (export "PS_CONFIG")   i32 (i32.const 4))
(global $PS_CLEN     (export "PS_CLEN")     i32 (i32.const 8))
(global $PS_STATE    (export "PS_STATE")    i32 (i32.const 12))
(global $PS_SIZE     (export "PS_SIZE")     i32 (i32.const 16))

;; ── Pipe struct offset constants ────────────────────────────────────
(global $PIPE_RD     (export "PIPE_RD")     i32 (i32.const 0))
(global $PIPE_WR     (export "PIPE_WR")     i32 (i32.const 4))
(global $PIPE_CAP    (export "PIPE_CAP")    i32 (i32.const 8))
(global $PIPE_STATE  (export "PIPE_STATE")  i32 (i32.const 12))
(global $PIPE_HEADER (export "PIPE_HEADER") i32 (i32.const 16))
(global $PIPE_MODE_CIRCULAR (export "PIPE_MODE_CIRCULAR") i32 (i32.const 2))
(global $PIPE_MODE_MASK     (export "PIPE_MODE_MASK")     i32 (i32.const 2))
(global $PIPE_CLOSED        (export "PIPE_CLOSED")        i32 (i32.const 1))

;; ── Alignment constants ─────────────────────────────────────────────
(global $PAGE_SIZE     (export "PAGE_SIZE")     i32 (i32.const 65536))
(global $CACHE_LINE    (export "CACHE_LINE")    i32 (i32.const 64))
(global $SIMD_ALIGN    (export "SIMD_ALIGN")    i32 (i32.const 16))
(global $VEC256_ALIGN  (export "VEC256_ALIGN")  i32 (i32.const 32))

;; ── Interpreter globals (compatibility with legacy $OFF_* names) ─────
;; Referenced by compiler/interpreter-core.wat fragments.
(global $OFF_TYPES_BUF       i32 (i32.const 0x00104))
(global $SZ_TYPE              i32 (i32.const 140))
(global $OFF_FUNCTIONS_BUF   i32 (i32.const 0x04514))
(global $SZ_FUNC              i32 (i32.const 16))
(global $OFF_CODE_BUF        i32 (i32.const 0x0551C))
(global $SZ_CODE              i32 (i32.const 64))
(global $OFF_DECODED_COUNT   i32 (i32.const 0x8C000))
(global $OFF_DECODED_OPS     i32 (i32.const 0xA0000))
(global $DEC_SZ               i32 (i32.const 32))
