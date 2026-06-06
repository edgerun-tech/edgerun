; EdgeRun freestanding WASM interpreter — x86_64 assembly
; Shared definitions (equates, %defines, error codes) — no BSS, no data, no macros.inc.
; Designed for inclusion by driver files that need error codes without the ~18MB WASM BSS.

%ifndef WASM_DEFINES_INC
%define WASM_DEFINES_INC

; ==================================================================
; WASM resource limits
; ==================================================================
%define MAX_FUNCTIONS        1024
%define MAX_IMPORTS          16
%define MAX_TYPES            128
%define MAX_TYPE_PARAMS      32
%define MAX_TYPE_RESULTS     4
%define MAX_EXPORTS          256
%define MAX_CODES            1024
%define MAX_GLOBALS          256
%define MAX_ELEMENTS         128
%define MAX_DATAS            128
%define MAX_INDIRECT_TABLE   1024      ; also MAX_TABLE_ENTRIES
%define MAX_LOCALS           128
%define MAX_STACK            4096
%define MAX_CONTROL          256       ; also MAX_CONTROL_DEPTH
%define MAX_HOST_IMPORTS     256
; FRAME_SAVE_SIZE defined in wasm_constants.inc (1MB)

; ==================================================================
; WASM binary format constants
; ==================================================================
wasm_magic: db 0x00, 0x61, 0x73, 0x6d
wasm_version: db 0x01, 0x00, 0x00, 0x00

; ==================================================================
; WASM section IDs
; ==================================================================
%define WASM_SECTION_CUSTOM      0
%define WASM_SECTION_TYPE        1
%define WASM_SECTION_IMPORT      2
%define WASM_SECTION_FUNCTION    3
%define WASM_SECTION_TABLE       4
%define WASM_SECTION_MEMORY      5
%define WASM_SECTION_GLOBAL      6
%define WASM_SECTION_EXPORT      7
%define WASM_SECTION_START       8
%define WASM_SECTION_ELEMENT     9
%define WASM_SECTION_CODE        10
%define WASM_SECTION_DATA        11

; ==================================================================
; WASM type opcodes (used during decode)
; ==================================================================
%define WASM_TYPE_I32        0x7f
%define WASM_TYPE_I64        0x7e
%define WASM_TYPE_F32        0x7d
%define WASM_TYPE_F64        0x7c
%define WASM_TYPE_FUNCREF    0x70
%define WASM_TYPE_EMPTY      0x40

; ==================================================================
; WASM opcodes
; ==================================================================
%define WASM_OP_UNREACHABLE 0x00
%define WASM_OP_NOP         0x01
%define WASM_OP_BLOCK       0x02
%define WASM_OP_LOOP        0x03
%define WASM_OP_IF          0x04
%define WASM_OP_ELSE        0x05
%define WASM_OP_END         0x0B
%define WASM_OP_BR          0x0C
%define WASM_OP_BR_IF       0x0D
%define WASM_OP_BR_TABLE    0x0E
%define WASM_OP_RETURN      0x0F
%define WASM_OP_CALL        0x10
%define WASM_OP_CALL_INDIRECT 0x11
%define WASM_OP_DROP        0x1A
%define WASM_OP_SELECT      0x1B
%define WASM_OP_LOCAL_GET   0x20
%define WASM_OP_LOCAL_SET   0x21
%define WASM_OP_LOCAL_TEE   0x22
%define WASM_OP_GLOBAL_GET  0x23
%define WASM_OP_GLOBAL_SET  0x24
%define WASM_OP_I32_LOAD    0x28
%define WASM_OP_I64_LOAD    0x29
%define WASM_OP_F32_LOAD    0x2A
%define WASM_OP_F64_LOAD    0x2B
%define WASM_OP_I32_LOAD8_S 0x2C
%define WASM_OP_I32_LOAD8_U 0x2D
%define WASM_OP_I32_LOAD16_S 0x2E
%define WASM_OP_I32_LOAD16_U 0x2F
%define WASM_OP_I32_STORE   0x36
%define WASM_OP_I64_STORE   0x37
%define WASM_OP_F32_STORE   0x38
%define WASM_OP_F64_STORE   0x39
%define WASM_OP_I32_STORE8  0x3A
%define WASM_OP_I32_STORE16 0x3B
%define WASM_OP_MEMORY_SIZE 0x3F
%define WASM_OP_MEMORY_GROW 0x40
%define WASM_OP_I32_CONST   0x41
%define WASM_OP_I64_CONST   0x42
%define WASM_OP_F32_CONST   0x43
%define WASM_OP_F64_CONST   0x44

; ==================================================================
; Boolean constants
; ==================================================================
%define TRUE    1
%define FALSE   0

; ==================================================================
; Math constants (shared with ER)
; ==================================================================
%define MASK_32_BIT         0xFFFFFFFF

; ==================================================================
; WASM page / memory constants
; ==================================================================
%define WASM_PAGE_SIZE       65536
%define WASM_PAGE_SHIFT      16
%define MAX_TABLE_ENTRIES    MAX_INDIRECT_TABLE
%define MAX_DATA_SEGMENTS    128
%define MAX_DECODED_OPS      512 * 1024
%define MAX_CONTROL_DEPTH    MAX_CONTROL

; LEB128 constant definitions
%define LEB32_MAX_BYTES      5
%define LEB64_MAX_BYTES      10
%define LEB_PAYLOAD_MASK     0x7f
%define LEB_CONTINUE_MASK    0x80
%define LEB_SIGN_MASK        0x40
%define LEB_BITS_PER_BYTE    7

; Limits encoding
%define LIMITS_MIN_ONLY      0
%define LIMITS_MIN_MAX       1

; Short section aliases (used by wasm_decode.asm)
%define SECTION_TYPE          WASM_SECTION_TYPE
%define SECTION_IMPORT        WASM_SECTION_IMPORT
%define SECTION_FUNCTION      WASM_SECTION_FUNCTION
%define SECTION_TABLE         WASM_SECTION_TABLE
%define SECTION_MEMORY        WASM_SECTION_MEMORY
%define SECTION_GLOBAL        WASM_SECTION_GLOBAL
%define SECTION_EXPORT        WASM_SECTION_EXPORT
%define SECTION_START         WASM_SECTION_START
%define SECTION_ELEMENT       WASM_SECTION_ELEMENT
%define SECTION_CODE          WASM_SECTION_CODE
%define SECTION_DATA          WASM_SECTION_DATA
%define SECTION_DATA_COUNT    12

; External kind constants (for import/export parsing)
%define EXTERNAL_FUNCTION     0
%define EXTERNAL_TABLE        1
%define EXTERNAL_MEMORY       2
%define EXTERNAL_GLOBAL       3

; WASM type / block aliases used by decoder
%define WASM_EMPTY_BLOCK_TYPE WASM_TYPE_EMPTY
%define WASM_FUNCREF_TYPE    WASM_TYPE_FUNCREF
%define WASM_EXTENDED_PREFIX  0xfc

; Extended opcode constants (prefix 0xFC sub-ops)
%define EXT_I32_TRUNC_SAT_F32_S   0
%define EXT_I32_TRUNC_SAT_F32_U   1
%define EXT_I32_TRUNC_SAT_F64_S   2
%define EXT_I32_TRUNC_SAT_F64_U   3
%define EXT_I64_TRUNC_SAT_F32_S   4
%define EXT_I64_TRUNC_SAT_F32_U   5
%define EXT_I64_TRUNC_SAT_F64_S   6
%define EXT_I64_TRUNC_SAT_F64_U   7
%define EXT_MEMORY_INIT           8
%define EXT_DATA_DROP             9
%define EXT_MEMORY_COPY           10
%define EXT_MEMORY_FILL           11
%define EXT_TABLE_INIT            12
%define EXT_ELEM_DROP             13
%define EXT_TABLE_COPY            14
%define EXT_TABLE_GROW            15
%define EXT_TABLE_SIZE            16
%define EXT_TABLE_FILL            17

; ==================================================================
; RuntimeConfig struct offsets (88 bytes, defined in kernel_main.asm)
; ==================================================================
%define RUNTIME_MEMORY_PTR_OFF       0
%define RUNTIME_MEMORY_LEN_OFF       8
%define RUNTIME_TICKS_PTR_OFF        16
%define RUNTIME_MEM_GROW_FN_OFF      24
%define RUNTIME_MEM_GROW_CTX_OFF     32
%define RUNTIME_TABLE_GROW_FN_OFF    40
%define RUNTIME_TABLE_GROW_CTX_OFF   48
%define RUNTIME_INITIAL_PAGES_OFF    56
%define RUNTIME_HAS_PAGES_OFF        64
%define RUNTIME_IMPORTS_PTR_OFF      72
%define RUNTIME_IMPORTS_LEN_OFF      80
%define RUNTIME_SIZE                 88

; ==================================================================
; HostImport struct offsets
; ==================================================================
HOST_IMPORT_SIZE        equ 40
%define HOST_IMPORT_MODULE_PTR_OFF    0
%define HOST_IMPORT_MODULE_LEN_OFF    8
%define HOST_IMPORT_NAME_PTR_OFF      16
%define HOST_IMPORT_NAME_LEN_OFF      24
%define HOST_IMPORT_FN_PTR_OFF        32

; ==================================================================
; Error codes (returned in rdx)
; Range 1-19: WASM interpreter errors
; Range 20-29: Generic driver / system errors
; Range 30-39: Virtio-specific errors
; 0 = OK (no error)
; ==================================================================
; WASM interpreter errors (1-19)
%define ERROR_OK               0
%define ERROR_UNSUPPORTED      1
%define ERROR_CORRUPT          2
%define ERROR_STACK_UNDERFLOW  3
%define ERROR_STACK_OVERFLOW   4
%define ERROR_MEMORY           5
%define ERROR_PARSE            7
%define ERROR_NOT_IMPLEMENTED  8
%define ERROR_TYPE_MISMATCH    9
%define ERROR_UNKNOWN_OPCODE   10
%define ERROR_INVALID_IMPORT   11
%define ERROR_NO_MEMORY        12
%define ERROR_INVALID_INDEX    13
%define ERROR_IMPORT_MISMATCH  14
%define ERROR_MEMORY_GROWTH    15
%define ERROR_TRAP             16
%define ERROR_ARITHMETIC_TRAP  17
%define ERROR_MISSING_IMPORT   18
%define ERROR_MISSING_EXPORT   19
%define ERROR_RECURSION        31

; Generic driver / system errors (20-29)
%define ERROR_TIMEOUT          20
%define ERROR_IO               21
%define ERROR_NOT_PRESENT      22
%define ERROR_NO_DATA          23
%define ERROR_INVALID_PARAM    24
%define ERROR_BUSY             25
%define ERROR_AUTH             26
%define ERROR_NO_SPACE         27
%define ERROR_NOT_FOUND        28
%define ERROR_BAD_ARGUMENT      29

; Virtio-specific error codes (range 30-39)
%define ERROR_DEVICE_NOT_FOUND     30
%define ERROR_MAP_FAILED           31
%define ERROR_FEATURE_NEGOTIATION  32
%define ERROR_QUEUE_SETUP          33
%define ERROR_INVALID_RESPONSE     34

; ==================================================================
; WasmEdge / EdgeRun-specific opcodes (prefixed opcodes ≥ 0xFC)
; ==================================================================
; WASM_OP_I32_TRUNC_SAT_F32_S etc. are defined separately in wasm_exec.asm
; because they require the dispatch table.

; Value type tags — aliases for WASM_TYPE_* constants
%define VALUE_TAG_I32      WASM_TYPE_I32
%define VALUE_TAG_I64      WASM_TYPE_I64
%define VALUE_TAG_F32      WASM_TYPE_F32
%define VALUE_TAG_F64      WASM_TYPE_F64
%define VALUE_TAG_FUNCREF  WASM_TYPE_FUNCREF

; Control frame kinds
%define CONTROL_BLOCK      0
%define CONTROL_LOOP       1
%define CONTROL_IF_THEN    2
%define CONTROL_IF_ELSE    3

%endif ; WASM_DEFINES_INC
