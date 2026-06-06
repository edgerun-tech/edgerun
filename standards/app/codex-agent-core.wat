(module
  (import "edgerun-core" "memory" (memory 1))
;; Agent/client/tool semantics plundered from crates/edgerun-codex.
  (func (export "codex_schema_type_code") (param $ptr i32) (param $len i32) (result i32)
    ;; string=1 number=2 boolean=3 integer=4 object=5 array=6 null=7 unknown=0.
    (if (call $eq6 (local.get $ptr) (local.get $len) (i32.const 115) (i32.const 116) (i32.const 114) (i32.const 105) (i32.const 110) (i32.const 103)) (then (return (i32.const 1))))
    (if (call $eq6 (local.get $ptr) (local.get $len) (i32.const 110) (i32.const 117) (i32.const 109) (i32.const 98) (i32.const 101) (i32.const 114)) (then (return (i32.const 2))))
    (if (call $eq7 (local.get $ptr) (local.get $len) (i32.const 98) (i32.const 111) (i32.const 111) (i32.const 108) (i32.const 101) (i32.const 97) (i32.const 110)) (then (return (i32.const 3))))
    (if (call $eq7 (local.get $ptr) (local.get $len) (i32.const 105) (i32.const 110) (i32.const 116) (i32.const 101) (i32.const 103) (i32.const 101) (i32.const 114)) (then (return (i32.const 4))))
    (if (call $eq6 (local.get $ptr) (local.get $len) (i32.const 111) (i32.const 98) (i32.const 106) (i32.const 101) (i32.const 99) (i32.const 116)) (then (return (i32.const 5))))
    (if (call $eq5 (local.get $ptr) (local.get $len) (i32.const 97) (i32.const 114) (i32.const 114) (i32.const 97) (i32.const 121)) (then (return (i32.const 6))))
    (if (call $eq4 (local.get $ptr) (local.get $len) (i32.const 110) (i32.const 117) (i32.const 108) (i32.const 108)) (then (return (i32.const 7))))
    i32.const 0)

  (func (export "codex_schema_infer_type")
    (param $has_type i32) (param $has_anyof i32) (param $has_object_keywords i32)
    (param $has_array_keywords i32) (param $has_enum_or_format i32) (param $has_number_keywords i32)
    (result i32)
    ;; mirrors sanitize_json_schema inference.
    (if (local.get $has_type) (then (return (i32.const 0))))
    (if (local.get $has_anyof) (then (return (i32.const 0))))
    (if (local.get $has_object_keywords) (then (return (i32.const 5))))
    (if (local.get $has_array_keywords) (then (return (i32.const 6))))
    (if (local.get $has_enum_or_format) (then (return (i32.const 1))))
    (if (local.get $has_number_keywords) (then (return (i32.const 2))))
    i32.const 1)

  (func (export "codex_schema_parse_result") (param $schema_type i32) (result i32)
    ;; singleton null tool input schema is rejected.
    (if (result i32) (i32.eq (local.get $schema_type) (i32.const 7))
      (then i32.const 1)
      (else i32.const 0)))

  (func (export "codex_tool_environment_mode") (param $count i32) (result i32)
    ;; none=0 single=1 multiple=2.
    (if (i32.eqz (local.get $count)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $count) (i32.const 1)) (then (return (i32.const 1))))
    i32.const 2)

  (func (export "codex_tool_shell_type")
    (param $shell_tool_feature i32) (param $zsh_fork_feature i32) (param $unified_exec_feature i32)
    (param $model_shell_type i32) (param $conpty_supported i32)
    (result i32)
    ;; disabled=0 shell_command=1 unified_exec=2.
    (if (i32.eqz (local.get $shell_tool_feature)) (then (return (i32.const 0))))
    (if (local.get $zsh_fork_feature) (then (return (i32.const 1))))
    (if (i32.and (local.get $unified_exec_feature) (local.get $conpty_supported)) (then (return (i32.const 2))))
    (if (i32.and (i32.eq (local.get $model_shell_type) (i32.const 2)) (i32.eqz (local.get $unified_exec_feature))) (then (return (i32.const 1))))
    local.get $model_shell_type)

  (func (export "codex_unified_exec_mode")
    (param $is_unix i32) (param $backend_zsh_fork i32) (param $user_shell_zsh i32)
    (param $has_zsh_path i32) (param $has_wrapper_path i32) (param $paths_absolute i32)
    (result i32)
    ;; direct=0 zsh_fork=1.
    (if (result i32) (i32.and
          (i32.and (local.get $is_unix) (local.get $backend_zsh_fork))
          (i32.and (i32.and (local.get $user_shell_zsh) (local.get $has_zsh_path))
                   (i32.and (local.get $has_wrapper_path) (local.get $paths_absolute))))
      (then i32.const 1)
      (else i32.const 0)))

  (func (export "codex_tool_gates")
    (param $model_supports_search i32) (param $tool_search i32)
    (param $apps i32) (param $plugins i32) (param $tool_suggest i32)
    (param $chatgpt_image_auth i32) (param $image_feature i32) (param $model_has_image_input i32)
    (result i32)
    ;; bit0 search, bit1 suggest, bit2 image_generation.
    (i32.or
      (i32.or
        (select (i32.const 1) (i32.const 0) (i32.and (local.get $model_supports_search) (local.get $tool_search)))
        (select (i32.const 2) (i32.const 0) (i32.and (i32.and (local.get $apps) (local.get $plugins)) (local.get $tool_suggest))))
      (select (i32.const 4) (i32.const 0) (i32.and (i32.and (local.get $chatgpt_image_auth) (local.get $image_feature)) (local.get $model_has_image_input)))))

  (func (export "codex_apply_patch_tool_type") (param $model_tool i32) (param $freeform_feature i32) (result i32)
    ;; none=0 function=1 freeform=2; model setting wins, feature adds freeform fallback.
    (if (i32.ne (local.get $model_tool) (i32.const 0)) (then (return (local.get $model_tool))))
    (if (result i32) (local.get $freeform_feature) (then i32.const 2) (else i32.const 0)))

  (func (export "codex_request_user_input_mode_available")
    (param $mode_allows i32) (param $default_mode i32) (param $default_feature i32) (result i32)
    (i32.or (local.get $mode_allows) (i32.and (local.get $default_mode) (local.get $default_feature))))

  (func (export "codex_request_body_result")
    (param $body_kind i32) (param $compression i32) (param $has_content_type i32) (param $has_content_encoding i32)
    (result i32)
    ;; none=0 ok, 1 raw+compression, 2 content-encoding conflict, 3 zstd disabled, 4 sets json content-type.
    (if (i32.eqz (local.get $body_kind)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $body_kind) (i32.const 2))
      (then
        (if (local.get $compression) (then (return (i32.const 1))))
        (return (i32.const 0))))
    (if (local.get $compression)
      (then
        (if (local.get $has_content_encoding) (then (return (i32.const 2))))
        (return (i32.const 3))))
    (if (result i32) (local.get $has_content_type) (then i32.const 0) (else i32.const 4)))

  (func (export "codex_retry_should_retry")
    (param $retry_429 i32) (param $retry_5xx i32) (param $retry_transport i32)
    (param $attempt i32) (param $max_attempts i32) (param $error_kind i32) (param $status i32)
    (result i32)
    ;; error_kind: 1 http, 2 timeout, 3 network, other no.
    (if (i32.ge_u (local.get $attempt) (local.get $max_attempts)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $error_kind) (i32.const 1))
      (then
        (if (i32.and (local.get $retry_429) (i32.eq (local.get $status) (i32.const 429))) (then (return (i32.const 1))))
        (return (i32.and (local.get $retry_5xx) (i32.and (i32.ge_u (local.get $status) (i32.const 500)) (i32.lt_u (local.get $status) (i32.const 600)))))))
    (if (i32.or (i32.eq (local.get $error_kind) (i32.const 2)) (i32.eq (local.get $error_kind) (i32.const 3)))
      (then (return (local.get $retry_transport))))
    i32.const 0)

  (func (export "codex_retry_backoff_nominal_ms") (param $base_ms i32) (param $attempt i32) (result i32)
    ;; nominal center of jittered backoff: attempt 0 base, attempt n base*2^(n-1).
    (if (i32.eqz (local.get $attempt)) (then (return (local.get $base_ms))))
    (i32.shl (local.get $base_ms) (i32.sub (local.get $attempt) (i32.const 1))))

  (func (export "codex_sse_event_result") (param $state i32) (result i32)
    ;; event=0, stream error=1, closed before completion=2, timeout=3.
    (if (i32.eq (local.get $state) (i32.const 1)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $state) (i32.const 2)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $state) (i32.const 3)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $state) (i32.const 4)) (then (return (i32.const 3))))
    i32.const 1)

  (func (export "codex_provider_validate_result")
    (param $has_aws i32) (param $supports_websockets i32) (param $has_env_key i32)
    (param $has_bearer i32) (param $has_auth i32) (param $requires_openai_auth i32)
    (param $auth_command_empty i32)
    (result i32)
    ;; 0 ok, 1 aws websocket conflict, 2 aws auth conflict, 3 empty command, 4 command auth conflict.
    (if (local.get $has_aws)
      (then
        (if (local.get $supports_websockets) (then (return (i32.const 1))))
        (if (i32.or (i32.or (local.get $has_env_key) (local.get $has_bearer)) (i32.or (local.get $has_auth) (local.get $requires_openai_auth)))
          (then (return (i32.const 2))))
        (return (i32.const 0))))
    (if (local.get $has_auth)
      (then
        (if (local.get $auth_command_empty) (then (return (i32.const 3))))
        (if (i32.or (i32.or (local.get $has_env_key) (local.get $has_bearer)) (local.get $requires_openai_auth))
          (then (return (i32.const 4))))))
    i32.const 0)

  (func (export "codex_provider_retry_cap") (param $configured i32) (param $is_stream i32) (result i32)
    ;; default request=4, stream=5; both cap at 100. configured -1 means unset.
    (local $value i32)
    (local.set $value
      (if (result i32) (i32.lt_s (local.get $configured) (i32.const 0))
        (then (if (result i32) (local.get $is_stream) (then i32.const 5) (else i32.const 4)))
        (else local.get $configured)))
    (if (result i32) (i32.gt_u (local.get $value) (i32.const 100)) (then i32.const 100) (else local.get $value)))

  (func (export "codex_provider_timeout_ms") (param $configured i32) (param $is_websocket i32) (result i32)
    ;; default stream idle 300000ms; websocket connect 15000ms.
    (if (i32.ge_s (local.get $configured) (i32.const 0))
      (then (return (local.get $configured))))
    (if (result i32) (local.get $is_websocket) (then i32.const 15000) (else i32.const 300000)))

  (func (export "codex_model_cache_ttl_seconds") (result i32)
    i32.const 300)

  (func (export "codex_refresh_strategy_allows_network") (param $strategy i32) (param $cache_fresh i32) (result i32)
    ;; online=1 offline=2 online_if_uncached=3.
    (if (i32.eq (local.get $strategy) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $strategy) (i32.const 2)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $strategy) (i32.const 3)) (then (return (i32.eqz (local.get $cache_fresh)))))
    i32.const 0)

  (func (export "codex_pipeline_stage_count") (result i32)
    i32.const 7)

  (func (export "codex_pipeline_stage_kind") (param $index i32) (result i32)
    ;; Router=1 Codebase=2 Architect=3 Toolsmith=4 Executor=5 Reviewer=6 Summarizer=7.
    (if (result i32) (i32.le_u (local.get $index) (i32.const 6))
      (then (i32.add (local.get $index) (i32.const 1)))
      (else i32.const 0)))

  (func (export "codex_pipeline_route_mask") (param $task_kind i32) (result i32)
    ;; bit index follows stage index. explain=1 build/debug=2 edit=3 architecture=4.
    ;; Router and Summarizer always present.
    (if (i32.eq (local.get $task_kind) (i32.const 1)) (then (return (i32.const 0x43))))
    (if (i32.eq (local.get $task_kind) (i32.const 2)) (then (return (i32.const 0x77))))
    (if (i32.eq (local.get $task_kind) (i32.const 3)) (then (return (i32.const 0x7f))))
    (if (i32.eq (local.get $task_kind) (i32.const 4)) (then (return (i32.const 0x67))))
    i32.const 0x7f)

  (func (export "codex_pipeline_limit") (param $which i32) (result i32)
    ;; 1 history, 2 stage output, 3 previous output, 4 repo context, 5 revealed context,
    ;; 6 reveal rounds, 7 reveals per round, 8 repo edits per stage.
    (if (i32.eq (local.get $which) (i32.const 1)) (then (return (i32.const 2400))))
    (if (i32.eq (local.get $which) (i32.const 2)) (then (return (i32.const 2400))))
    (if (i32.eq (local.get $which) (i32.const 3)) (then (return (i32.const 9000))))
    (if (i32.eq (local.get $which) (i32.const 4)) (then (return (i32.const 12000))))
    (if (i32.eq (local.get $which) (i32.const 5)) (then (return (i32.const 18000))))
    (if (i32.eq (local.get $which) (i32.const 6)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $which) (i32.const 7)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $which) (i32.const 8)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "codex_code_mode_parse_result")
    (param $trim_empty i32) (param $has_pragma i32) (param $rest_empty i32)
    (param $pragma_json_object i32) (param $unsupported_field i32) (param $safe_integer i32)
    (result i32)
    ;; 0 ok, 1 empty source, 2 pragma without source, 3 bad pragma object, 4 unsupported field, 5 unsafe integer.
    (if (local.get $trim_empty) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $has_pragma)) (then (return (i32.const 0))))
    (if (local.get $rest_empty) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $pragma_json_object)) (then (return (i32.const 3))))
    (if (local.get $unsupported_field) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $safe_integer)) (then (return (i32.const 5))))
    i32.const 0)

  (func (export "codex_code_mode_identifier_char") (param $ch i32) (param $index i32) (result i32)
    (local $alpha i32) (local $digit i32)
    (local.set $alpha (i32.or (i32.and (i32.ge_u (local.get $ch) (i32.const 65)) (i32.le_u (local.get $ch) (i32.const 90)))
                              (i32.and (i32.ge_u (local.get $ch) (i32.const 97)) (i32.le_u (local.get $ch) (i32.const 122)))))
    (local.set $digit (i32.and (i32.ge_u (local.get $ch) (i32.const 48)) (i32.le_u (local.get $ch) (i32.const 57))))
    (if (i32.eqz (local.get $index))
      (then (return (i32.or (i32.or (i32.eq (local.get $ch) (i32.const 95)) (i32.eq (local.get $ch) (i32.const 36))) (local.get $alpha)))))
    (i32.or (i32.or (i32.or (i32.eq (local.get $ch) (i32.const 95)) (i32.eq (local.get $ch) (i32.const 36))) (local.get $alpha)) (local.get $digit)))

  (func (export "codex_ui_patch_kind") (param $kind i32) (result i32)
    ;; ui_stream PatchKind valid range 0..50.
    (if (result i32) (i32.le_u (local.get $kind) (i32.const 50)) (then local.get $kind) (else i32.const -1)))

  (func (export "codex_ui_agent_component_id") (param $component i32) (result i32)
    ;; AssistantDraft=1 Status=2 ToolCall=3 Stdout=4 Stderr=5 DiffPreview=6 Input=7 RunButton=8.
    (if (result i32) (i32.and (i32.ge_u (local.get $component) (i32.const 1)) (i32.le_u (local.get $component) (i32.const 8)))
      (then local.get $component)
      (else i32.const 0)))

  (func (export "codex_ui_string_payload_len") (param $byte_len i32) (result i32)
    ;; encode_string prefixes a u8 length and rejects >255.
    (if (i32.gt_u (local.get $byte_len) (i32.const 255)) (then (return (i32.const -1))))
    (i32.add (local.get $byte_len) (i32.const 1)))

  (func $eq4 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (result i32)
    (i32.and (i32.eq (local.get $len) (i32.const 4))
      (i32.and
        (i32.and (i32.eq (i32.load8_u (local.get $ptr)) (local.get $a))
                 (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (local.get $b)))
        (i32.and (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (local.get $c))
                 (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))) (local.get $d))))))

  (func $eq5 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (result i32)
    (i32.and (i32.eq (local.get $len) (i32.const 5))
      (i32.and (call $eq4 (local.get $ptr) (i32.const 4) (local.get $a) (local.get $b) (local.get $c) (local.get $d))
               (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (local.get $e)))))

  (func $eq6 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (param $f i32) (result i32)
    (i32.and (i32.eq (local.get $len) (i32.const 6))
      (i32.and (call $eq5 (local.get $ptr) (i32.const 5) (local.get $a) (local.get $b) (local.get $c) (local.get $d) (local.get $e))
               (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (local.get $f)))))

  (func $eq7 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (param $f i32) (param $g i32) (result i32)
    (i32.and (i32.eq (local.get $len) (i32.const 7))
      (i32.and (call $eq6 (local.get $ptr) (i32.const 6) (local.get $a) (local.get $b) (local.get $c) (local.get $d) (local.get $e) (local.get $f))
               (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (local.get $g)))))
)