(module
  ;; Node UI surface and authority-flow semantics plundered from crates/node.

  (func (export "node_surfaces_abi_version") (result i32) i32.const 1)
  (func (export "frontend_work_projection_schema_version") (result i32) i32.const 1)
  (func (export "frontend_input_buffer_capacity") (result i32) i32.const 4096)
  (func (export "frontend_color_scheme") (result i32) i32.const 0)

  (func (export "frontend_frame_active") (param $time_ms i64) (result i32)
    ;; active alternates every 800ms.
    (if (i64.eqz (i64.rem_u (i64.div_u (local.get $time_ms) (i64.const 800)) (i64.const 2))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "frontend_action_dirty") (param $action_kind i32) (result i32)
    ;; 1 hovered, 2 focused, 3 activated, 4 scroll changed, 5 open changed.
    (if (i32.and (i32.ge_u (local.get $action_kind) (i32.const 1)) (i32.le_u (local.get $action_kind) (i32.const 5))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "frontend_pointer_click_dirty") (param $down_dirty i32) (param $up_dirty i32) (result i32)
    (if (i32.or (local.get $down_dirty) (local.get $up_dirty)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "frontend_input_len") (param $requested_len i32) (result i32)
    (if (i32.gt_u (local.get $requested_len) (i32.const 4096)) (then (return (i32.const 4096))))
    local.get $requested_len)

  (func (export "frontend_hit_code") (param $has_hit i32) (param $kind_code i32) (param $id i32) (result i32)
    ;; u32::MAX for no hit, otherwise kind in high byte and 24-bit id.
    (if (i32.eqz (local.get $has_hit)) (then (return (i32.const -1))))
    (i32.or (i32.shl (i32.and (local.get $kind_code) (i32.const 255)) (i32.const 24)) (i32.and (local.get $id) (i32.const 0x00ffffff))))

  (func (export "web_key_code") (param $code i32) (result i32)
    ;; Returns stable UiKey codes: 1 backspace, 2 tab, 3 enter, 4 escape,
    ;; 5 pageup, 6 pagedown, 7 end, 8 home, 9..12 arrows, 13 delete, otherwise 1000+raw.
    (if (i32.eq (local.get $code) (i32.const 8)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $code) (i32.const 9)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $code) (i32.const 13)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $code) (i32.const 27)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $code) (i32.const 33)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $code) (i32.const 34)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $code) (i32.const 35)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $code) (i32.const 36)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $code) (i32.const 37)) (then (return (i32.const 9))))
    (if (i32.eq (local.get $code) (i32.const 38)) (then (return (i32.const 10))))
    (if (i32.eq (local.get $code) (i32.const 39)) (then (return (i32.const 11))))
    (if (i32.eq (local.get $code) (i32.const 40)) (then (return (i32.const 12))))
    (if (i32.eq (local.get $code) (i32.const 46)) (then (return (i32.const 13))))
    (i32.add (i32.const 1000) (local.get $code)))

  (func (export "native_arg_result") (param $arg_kind i32) (param $has_value i32) (param $value_valid i32) (result i32)
    ;; arg_kind: 1 --frames, 2 --dump-scene, 3 --scheme, 4 help, 5 unknown.
    ;; 0 ok, 1 missing value, 2 invalid value, 3 help exit, 4 unknown.
    (if (i32.eq (local.get $arg_kind) (i32.const 4)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $arg_kind) (i32.const 5)) (then (return (i32.const 4))))
    (if (i32.or (i32.eq (local.get $arg_kind) (i32.const 1)) (i32.eq (local.get $arg_kind) (i32.const 3)))
      (then
        (if (i32.eqz (local.get $has_value)) (then (return (i32.const 1))))
        (if (i32.eqz (local.get $value_valid)) (then (return (i32.const 2))))))
    i32.const 0)

  (func (export "scheme_code") (param $scheme i32) (result i32)
    ;; dark/light/terminal are 1..3.
    (if (i32.and (i32.ge_u (local.get $scheme) (i32.const 1)) (i32.le_u (local.get $scheme) (i32.const 3))) (then (return (local.get $scheme))))
    i32.const 0)

  (func (export "ws_masked_client_frame_header_len") (param $payload_len i64) (result i32)
    ;; Client binary frame always includes 2 base bytes plus mask, with optional extended length.
    (if (i64.lt_u (local.get $payload_len) (i64.const 126)) (then (return (i32.const 6))))
    (if (i64.le_u (local.get $payload_len) (i64.const 65535)) (then (return (i32.const 8))))
    i32.const 14)

  (func (export "ws_server_binary_result") (param $opcode i32) (param $masked i32) (result i32)
    ;; 0 ok, 1 non-binary, 2 server frame masked.
    (if (i32.ne (i32.and (local.get $opcode) (i32.const 15)) (i32.const 2)) (then (return (i32.const 1))))
    (if (local.get $masked) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "ws_handshake_result") (param $closed_early i32) (param $response_len i32) (param $has_101 i32) (param $has_protocol i32) (result i32)
    ;; 0 ok, 1 closed early, 2 too large, 3 not switching protocols, 4 missing edgerun-work-v1.
    (if (local.get $closed_early) (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $response_len) (i32.const 4096)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $has_101)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $has_protocol)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "authority_command_decision") (param $signature_ok i32) (param $duplicate i32) (result i32)
    ;; 1 committed, 2 rejected.
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 2))))
    (if (local.get $duplicate) (then (return (i32.const 1))))
    i32.const 1)

  (func (export "authority_reason_code") (param $signature_ok i32) (param $duplicate i32) (result i32)
    ;; 0 empty, 1 rejected, 2 duplicate_command.
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 1))))
    (if (local.get $duplicate) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "authority_stream_len_after_event") (param $current_len i32) (result i32)
    (i32.add (local.get $current_len) (i32.const 1)))

  (func (export "sdk_runtime_projection_result") (param $graph_decodes i32) (param $records_event i32) (param $payload_hash_matches i32) (param $projection_matches_package i32) (result i32)
    ;; 0 ok, 1 graph decode failed, 2 event missing/wrong kind, 3 payload hash mismatch, 4 projection mismatch.
    (if (i32.eqz (local.get $graph_decodes)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $records_event)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $payload_hash_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $projection_matches_package)) (then (return (i32.const 4))))
    i32.const 0)
)
