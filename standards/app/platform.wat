(module
  (import "edgerun" "load8_u" (func $m44ch (param i32 i32) (result i32)))
  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun" "lo" (func $lo (param i64) (result i32)))
  (import "edgerun" "hi" (func $hi (param i64) (result i32)))
  (import "edgerun" "to_lower" (func $to_lower (param i32) (result i32)))
  (import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))
  (import "edgerun" "is_hex" (func $is_hex (param i32) (result i32)))
  (import "edgerun" "is_alnum" (func $is_alnum (param i32) (result i32)))
  (import "edgerun" "memcpy" (func $memcpy (param i32 i32 i32)))
  (memory (export "memory") 1)

;; Browser-core Rust semantics captured from browser-wire, browser-authoring,
  ;; browser-runtime, browser-host, browser-core-slice, and browser-work mirrors.
  ;;
  ;; Decisions: run-once=1 verify-cache=2 cancel=3.
  ;; Capability status: ok=0 denied=1 invalid=2 provider-failed=3.
  ;; Host status: ok=0 invalid-input=1 runtime-error=2.

  (func $m33bool (param $x i32) (result i32)
    local.get $x
    i32.const 0
    i32.ne)

  (func $min64 (param $a i64) (param $b i64) (result i64)
    local.get $a
    local.get $b
    i64.lt_u
    if (result i64)
      local.get $a
    else
      local.get $b
    end)

  (func $max64 (param $a i64) (param $b i64) (result i64)
    local.get $a
    local.get $b
    i64.gt_u
    if (result i64)
      local.get $a
    else
      local.get $b
    end)

  (export "browser_wire_constant" (func $browser_wire_constant))
  (func $browser_wire_constant (param $kind i32) (result i32)
    ;; sdk-abi=1, ed25519=2, app-run-cache=3, cache-verified=4,
    ;; storage-kind=5, storage-write=6, session-open=7.
    local.get $kind
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 2
    else
      local.get $kind
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 1
      else
        local.get $kind
        i32.const 3
        i32.eq
        if (result i32)
          i32.const 2
        else
          local.get $kind
          i32.const 4
          i32.eq
          if (result i32)
            i32.const 1
          else
            local.get $kind
            i32.const 5
            i32.eq
            if (result i32)
              i32.const 4
            else
              local.get $kind
              i32.const 6
              i32.eq
              if (result i32)
                i32.const 7
              else
                local.get $kind
                i32.const 7
                i32.eq
                if (result i32)
                  i32.const 1
                else
                  i32.const 0
                end
              end
            end
          end
        end
      end
    end)

  (export "browser_artifact_kind" (func $browser_artifact_kind))
  (func $browser_artifact_kind
    (param $reserved_manifest i32)
    (param $ends_edm i32)
    (param $ends_wasm i32)
    (param $ends_native_lib i32)
    (param $ends_web_asset i32)
    (result i32)
    ;; manifest=1 edm=2 wasm=3 native=4 web=5 other=0.
    local.get $reserved_manifest
    call $m33bool
    if (result i32)
      i32.const 1
    else
      local.get $ends_edm
      call $m33bool
      if (result i32)
        i32.const 2
      else
        local.get $ends_wasm
        call $m33bool
        if (result i32)
          i32.const 3
        else
          local.get $ends_native_lib
          call $m33bool
          if (result i32)
            i32.const 4
          else
            local.get $ends_web_asset
            call $m33bool
            if (result i32)
              i32.const 5
            else
              i32.const 0
            end
          end
        end
      end
    end)

  (export "browser_artifact_path_valid" (func $browser_artifact_path_valid))
  (func $browser_artifact_path_valid
    (param $len i32)
    (param $starts_slash i32)
    (param $contains_backslash i32)
    (param $has_dot_segment i32)
    (param $has_empty_segment i32)
    (param $reserved_name i32)
    (result i32)
    local.get $len
    i32.const 0
    i32.gt_s
    local.get $starts_slash
    call $m33bool
    i32.eqz
    i32.and
    local.get $contains_backslash
    call $m33bool
    i32.eqz
    i32.and
    local.get $has_dot_segment
    call $m33bool
    i32.eqz
    i32.and
    local.get $has_empty_segment
    call $m33bool
    i32.eqz
    i32.and
    local.get $reserved_name
    call $m33bool
    i32.eqz
    i32.and)

  (export "browser_code_hash_artifact_included" (func $browser_code_hash_artifact_included))
  (func $browser_code_hash_artifact_included (param $artifact_kind i32) (result i32)
    local.get $artifact_kind
    i32.const 2
    i32.ge_s
    local.get $artifact_kind
    i32.const 5
    i32.le_s
    i32.and)

  (export "browser_verify_package_result" (func $browser_verify_package_result))
  (func $browser_verify_package_result
    (param $graph_manifest_hash_matches i32)
    (param $slug_matches i32)
    (param $developer_matches i32)
    (param $app_id_matches i32)
    (param $binds_app_edapp i32)
    (param $runtime_projection_matches i32)
    (param $signature_ok i32)
    (result i32)
    ;; ok=0 then failure stage 1..7 in verifier order.
    local.get $graph_manifest_hash_matches
    call $m33bool
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $slug_matches
      call $m33bool
      i32.eqz
      if (result i32)
        i32.const 2
      else
        local.get $developer_matches
        call $m33bool
        i32.eqz
        if (result i32)
          i32.const 3
        else
          local.get $app_id_matches
          call $m33bool
          i32.eqz
          if (result i32)
            i32.const 4
          else
            local.get $binds_app_edapp
            call $m33bool
            i32.eqz
            if (result i32)
              i32.const 5
            else
              local.get $runtime_projection_matches
              call $m33bool
              i32.eqz
              if (result i32)
                i32.const 6
              else
                local.get $signature_ok
                call $m33bool
                i32.eqz
                if (result i32)
                  i32.const 7
                else
                  i32.const 0
                end
              end
            end
          end
        end
      end
    end)

  (export "browser_signature_verify_result" (func $browser_signature_verify_result))
  (func $browser_signature_verify_result
    (param $algorithm i32)
    (param $artifact_hash_matches i32)
    (param $public_key_len i32)
    (param $ed25519_valid i32)
    (result i32)
    ;; ok=0 bad-alg=1 bad-hash=2 bad-key-len=3 verify-failed=4.
    local.get $algorithm
    i32.const 1
    i32.ne
    if (result i32)
      i32.const 1
    else
      local.get $artifact_hash_matches
      call $m33bool
      i32.eqz
      if (result i32)
        i32.const 2
      else
        local.get $public_key_len
        i32.const 32
        i32.ne
        if (result i32)
          i32.const 3
        else
          local.get $ed25519_valid
          call $m33bool
          if (result i32)
            i32.const 0
          else
            i32.const 4
          end
        end
      end
    end)

  (export "browser_first_run_event_count" (func $browser_first_run_event_count))
  (func $browser_first_run_event_count (param $decision i32) (result i32)
    local.get $decision
    i32.const 2
    i32.eq
    if (result i32)
      i32.const 4
    else
      i32.const 3
    end)

  (export "browser_first_run_cache_required" (func $browser_first_run_cache_required))
  (func $browser_first_run_cache_required (param $decision i32) (result i32)
    local.get $decision
    i32.const 2
    i32.eq)

  (export "browser_first_run_installs_app" (func $browser_first_run_installs_app))
  (func $browser_first_run_installs_app (param $decision i32) (result i32)
    local.get $decision
    i32.const 3
    i32.ne)

  (export "browser_decision_valid" (func $browser_decision_valid))
  (func $browser_decision_valid (param $decision i32) (result i32)
    local.get $decision
    i32.const 1
    i32.ge_s
    local.get $decision
    i32.const 3
    i32.le_s
    i32.and)

  (export "browser_runtime_first_run_result" (func $browser_runtime_first_run_result))
  (func $browser_runtime_first_run_result
    (param $decision_matches i32)
    (param $decision i32)
    (param $cache_present i32)
    (param $cache_matches i32)
    (result i32)
    ;; ok=0 bad-decision=1 missing-cache=2 unexpected-cache=3 bad-cache=4.
    local.get $decision_matches
    call $m33bool
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $decision
      i32.const 2
      i32.eq
      if (result i32)
        local.get $cache_present
        call $m33bool
        i32.eqz
        if (result i32)
          i32.const 2
        else
          local.get $cache_matches
          call $m33bool
          if (result i32)
            i32.const 0
          else
            i32.const 4
          end
        end
      else
        local.get $cache_present
        call $m33bool
        if (result i32)
          i32.const 3
        else
          i32.const 0
        end
      end
    end)

  (export "browser_route_is_live" (func $browser_route_is_live))
  (func $browser_route_is_live
    (param $valid_from i64)
    (param $valid_until i64)
    (param $time i64)
    (result i32)
    local.get $valid_from
    local.get $time
    i64.le_u
    local.get $time
    local.get $valid_until
    i64.le_u
    i32.and)

  (export "browser_grant_valid" (func $browser_grant_valid))
  (func $browser_grant_valid
    (param $app_verified i32)
    (param $release_matches i32)
    (param $valid_range i32)
    (param $grant_id_matches i32)
    (result i32)
    local.get $app_verified
    call $m33bool
    local.get $release_matches
    call $m33bool
    i32.and
    local.get $valid_range
    call $m33bool
    i32.and
    local.get $grant_id_matches
    call $m33bool
    i32.and)

  (export "browser_binding_matches_grant" (func $browser_binding_matches_grant))
  (func $browser_binding_matches_grant
    (param $grant_exists i32)
    (param $app_release_scope_matches i32)
    (param $kind_matches_storage i32)
    (param $route_live i32)
    (param $binding_id_matches i32)
    (result i32)
    local.get $grant_exists
    call $m33bool
    local.get $app_release_scope_matches
    call $m33bool
    i32.and
    local.get $kind_matches_storage
    call $m33bool
    i32.and
    local.get $route_live
    call $m33bool
    i32.and
    local.get $binding_id_matches
    call $m33bool
    i32.and)

  (export "browser_session_valid" (func $browser_session_valid))
  (func $browser_session_valid
    (param $grant_exists i32)
    (param $app_release_matches i32)
    (param $status_open i32)
    (param $ids_nonzero i32)
    (param $session_id_matches i32)
    (param $binding_matches i32)
    (param $route_live i32)
    (param $valid_until_ok i32)
    (result i32)
    local.get $grant_exists
    call $m33bool
    local.get $app_release_matches
    call $m33bool
    i32.and
    local.get $status_open
    call $m33bool
    i32.and
    local.get $ids_nonzero
    call $m33bool
    i32.and
    local.get $session_id_matches
    call $m33bool
    i32.and
    local.get $binding_matches
    call $m33bool
    i32.and
    local.get $route_live
    call $m33bool
    i32.and
    local.get $valid_until_ok
    call $m33bool
    i32.and)

  (export "browser_storage_envelope_supported" (func $browser_storage_envelope_supported))
  (func $browser_storage_envelope_supported
    (param $shape_ok i32)
    (param $kind i32)
    (param $content_type i32)
    (param $operation i32)
    (result i32)
    ;; capability invoke kind=1, object content=1, get=1 put=2 in browser-work mirror.
    local.get $shape_ok
    call $m33bool
    local.get $kind
    i32.const 1
    i32.eq
    i32.and
    local.get $content_type
    i32.const 1
    i32.eq
    i32.and
    local.get $operation
    i32.const 1
    i32.eq
    local.get $operation
    i32.const 2
    i32.eq
    i32.or
    i32.and)

  (export "browser_envelope_operation_matches_request" (func $browser_envelope_operation_matches_request))
  (func $browser_envelope_operation_matches_request
    (param $envelope_operation i32)
    (param $request_operation i32)
    (result i32)
    ;; envelope object-get=1 maps to read=6; object-put=2 maps to write=7.
    local.get $envelope_operation
    i32.const 1
    i32.eq
    local.get $request_operation
    i32.const 6
    i32.eq
    i32.and
    local.get $envelope_operation
    i32.const 2
    i32.eq
    local.get $request_operation
    i32.const 7
    i32.eq
    i32.and
    i32.or)

  (export "browser_storage_request_bound" (func $browser_storage_request_bound))
  (func $browser_storage_request_bound
    (param $app_declares_namespace i32)
    (param $binding_matches_namespace i32)
    (param $grant_allows_operation i32)
    (param $assurance_ok i32)
    (result i32)
    local.get $app_declares_namespace
    call $m33bool
    local.get $binding_matches_namespace
    call $m33bool
    i32.and
    local.get $grant_allows_operation
    call $m33bool
    i32.and
    local.get $assurance_ok
    call $m33bool
    i32.and)

  (export "browser_storage_response_status" (func $browser_storage_response_status))
  (func $browser_storage_response_status
    (param $bound i32)
    (param $operation i32)
    (param $payload_hash_matches i32)
    (param $read_hash_constraint_ok i32)
    (result i32)
    ;; ok=0 denied=1 invalid=2.
    local.get $bound
    call $m33bool
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $operation
      i32.const 7
      i32.eq
      if (result i32)
        local.get $payload_hash_matches
        call $m33bool
        if (result i32)
          i32.const 0
        else
          i32.const 2
        end
      else
        local.get $operation
        i32.const 6
        i32.eq
        if (result i32)
          local.get $read_hash_constraint_ok
          call $m33bool
          if (result i32)
            i32.const 0
          else
            i32.const 2
          end
        else
          i32.const 1
        end
      end
    end)

  (export "browser_event_chain_step_valid" (func $browser_event_chain_step_valid))
  (func $browser_event_chain_step_valid
    (param $seq i64)
    (param $index i64)
    (param $previous_matches i32)
    (param $payload_hash_matches i32)
    (param $event_hash_matches i32)
    (result i32)
    local.get $seq
    local.get $index
    i64.eq
    local.get $previous_matches
    call $m33bool
    i32.and
    local.get $payload_hash_matches
    call $m33bool
    i32.and
    local.get $event_hash_matches
    call $m33bool
    i32.and)

  (export "browser_retrieval_policy_valid_at" (func $browser_retrieval_policy_valid_at))
  (func $browser_retrieval_policy_valid_at
    (param $valid_from i64)
    (param $valid_until i64)
    (param $requested_at i64)
    (result i32)
    local.get $valid_from
    local.get $requested_at
    i64.le_u
    local.get $requested_at
    local.get $valid_until
    i64.le_u
    i32.and)

  (export "browser_retrieval_cost" (func $browser_retrieval_cost))
  (func $browser_retrieval_cost
    (param $base_cost i64)
    (param $cost_per_byte i64)
    (param $min_cost i64)
    (param $max_cost i64)
    (param $manifest_len i64)
    (param $graph_len i64)
    (param $signature_len i64)
    (result i64)
    (local $bytes i64)
    (local $cost i64)
    local.get $manifest_len
    local.get $graph_len
    i64.add
    local.get $signature_len
    i64.add
    local.set $bytes
    local.get $base_cost
    local.get $bytes
    local.get $cost_per_byte
    i64.mul
    i64.add
    local.get $min_cost
    call $max64
    local.set $cost
    local.get $max_cost
    i64.const 0
    i64.ne
    if (result i64)
      local.get $cost
      local.get $max_cost
      call $min64
    else
      local.get $cost
    end)

  (export "browser_policy_hash_source" (func $browser_policy_hash_source))
  (func $browser_policy_hash_source (param $schedule_len i32) (result i32)
    ;; default-domain=1 schedule-blake3=2.
    local.get $schedule_len
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      i32.const 2
    end)

  (export "browser_work_request_valid_until" (func $browser_work_request_valid_until))
  (func $browser_work_request_valid_until (param $requested_at i64) (result i64)
    local.get $requested_at
    i64.const 60000
    i64.add)

  (export "browser_admitted_budget" (func $browser_admitted_budget))
  (func $browser_admitted_budget (param $retrieval_cost i64) (result i64)
    local.get $retrieval_cost
    i64.const 1
    call $max64)

  (export "browser_host_error_status" (func $browser_host_error_status))
  (func $browser_host_error_status (param $error_kind i32) (result i32)
    ;; invalid-wire/unexpected=1 runtime=2.
    local.get $error_kind
    i32.const 2
    i32.eq
    if (result i32)
      i32.const 2
    else
      i32.const 1
    end)

  (export "browser_host_invoke_operation" (func $browser_host_invoke_operation))
  (func $browser_host_invoke_operation (param $request_operation i32) (result i32)
    ;; read=6 -> object-get=1; write=7 -> object-put=2; invalid=0.
    local.get $request_operation
    i32.const 6
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $request_operation
      i32.const 7
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "browser_ffi_slice_valid" (func $browser_ffi_slice_valid))
  (func $browser_ffi_slice_valid (param $ptr_nonzero i32) (param $len i32) (result i32)
    local.get $ptr_nonzero
    call $m33bool
    local.get $len
    i32.const 0
    i32.gt_s
    i32.and)

  (export "browser_wasm_pages_needed" (func $browser_wasm_pages_needed))
  (func $browser_wasm_pages_needed
    (param $required_end i32)
    (param $current_pages i32)
    (result i32)
    (local $current_bytes i32)
    local.get $current_pages
    i32.const 65536
    i32.mul
    local.set $current_bytes
    local.get $required_end
    local.get $current_bytes
    i32.le_u
    if (result i32)
      i32.const 0
    else
      local.get $required_end
      local.get $current_bytes
      i32.sub
      i32.const 65535
      i32.add
      i32.const 65536
      i32.div_u
    end)

;; Chrome DevTools Protocol (CDP) message builder — JSON-RPC over WebSocket.
  ;; Uses mutable globals $g_out/$g_o shared across helper functions.
  (global $g_out (mut i32) (i32.const 0))
  (global $g_o (mut i32) (i32.const 0))

  ;; cdp_build_navigate(ptr, cap, url_ptr, url_len) -> status:i32, written:i32 packed as i64
  ;; Uses id=1 always. Builds: {"id":1,"method":"Page.navigate","params":{"url":"URL"}}
  (func (export "cdp_build_navigate") (param $out i32) (param $ocap i32) (param $url i32) (param $ulen i32) (result i64)
    (local $i i32)
    local.get $out
    global.set $g_out
    i32.const 0
    global.set $g_o

    ;; {"id":1
    i32.const 123 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 105 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 100 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 58 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 49 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o

    call $emit_comma_method_navigate

    call $emit_comma_params_url

    ;; copy URL bytes
    i32.const 0 local.set $i
    block $url_done
    loop $url_loop
      local.get $i local.get $ulen i32.ge_u br_if $url_done
      local.get $url local.get $i i32.add i32.load8_u
      global.get $g_out global.get $g_o i32.add i32.store8
      global.get $g_o i32.const 1 i32.add global.set $g_o
      local.get $i i32.const 1 i32.add local.set $i
      br $url_loop
    end
    end

    ;; "}}
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 125 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o

    i64.const 0
    global.get $g_o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; cdp_build_evaluate(ptr, cap, expr_ptr, expr_len) -> packed i64
  ;; id=1, Runtime.evaluate with returnByValue
  (func (export "cdp_build_evaluate") (param $out i32) (param $ocap i32) (param $expr i32) (param $elen i32) (result i64)
    (local $i i32)
    local.get $out
    global.set $g_out
    i32.const 0
    global.set $g_o

    ;; {"id":1
    i32.const 123 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 105 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 100 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 58 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 49 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o

    call $emit_comma_method_evaluate

    call $emit_comma_params_expression

    ;; copy expression bytes
    i32.const 0 local.set $i
    block $expr_done
    loop $expr_loop
      local.get $i local.get $elen i32.ge_u br_if $expr_done
      local.get $expr local.get $i i32.add i32.load8_u
      global.get $g_out global.get $g_o i32.add i32.store8
      global.get $g_o i32.const 1 i32.add global.set $g_o
      local.get $i i32.const 1 i32.add local.set $i
      br $expr_loop
    end
    end

    call $emit_close_returnbyvalue

    i64.const 0
    global.get $g_o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; cdp_extract_result_value(json_ptr, json_len, out_ptr, out_cap) -> status, len as i64
  ;; Scan for "value":" then copy string value bytes until closing quote
  (func (export "cdp_extract_result_value") (param $json i32) (param $jlen i32) (param $out i32) (param $ocap i32) (result i64)
    (local $i i32) (local $o i32) (local $b i32)

    block $done
    loop $scan
      local.get $i local.get $jlen i32.ge_u br_if $done
      local.get $json local.get $i i32.add i32.load8_u local.set $b
      local.get $b i32.const 34 i32.ne           ;; '"'
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $scan
      end
      ;; Check "value":"
      ;; Need 9 more bytes from $i to be in bounds for the full pattern
      local.get $i i32.const 8 i32.add local.get $jlen i32.ge_u
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $scan
      end
      local.get $json local.get $i i32.add i32.load8_u offset=1 i32.const 118 i32.ne
      if local.get $i i32.const 1 i32.add local.set $i br $scan end  ;; 'v'
      local.get $json local.get $i i32.add i32.load8_u offset=2 i32.const 97 i32.ne
      if local.get $i i32.const 1 i32.add local.set $i br $scan end   ;; 'a'
      local.get $json local.get $i i32.add i32.load8_u offset=3 i32.const 108 i32.ne
      if local.get $i i32.const 1 i32.add local.set $i br $scan end  ;; 'l'
      local.get $json local.get $i i32.add i32.load8_u offset=4 i32.const 117 i32.ne
      if local.get $i i32.const 1 i32.add local.set $i br $scan end  ;; 'u'
      local.get $json local.get $i i32.add i32.load8_u offset=5 i32.const 101 i32.ne
      if local.get $i i32.const 1 i32.add local.set $i br $scan end  ;; 'e'
      local.get $json local.get $i i32.add i32.load8_u offset=6 i32.const 34 i32.ne
      if local.get $i i32.const 1 i32.add local.set $i br $scan end   ;; '"'
      local.get $json local.get $i i32.add i32.load8_u offset=7 i32.const 58 i32.ne
      if local.get $i i32.const 1 i32.add local.set $i br $scan end   ;; ':'
      local.get $json local.get $i i32.add i32.load8_u offset=8 i32.const 34 i32.ne
      if local.get $i i32.const 1 i32.add local.set $i br $scan end   ;; '"'
      ;; found "value":" — copy from json+$i+9
      local.get $i i32.const 9 i32.add local.set $i
      loop $val
        local.get $i local.get $jlen i32.ge_u br_if $done
        local.get $json local.get $i i32.add i32.load8_u local.set $b
        local.get $b i32.const 34 i32.eq br_if $done  ;; closing quote
        local.get $o local.get $ocap i32.ge_u br_if $done
        local.get $out local.get $o i32.add local.get $b i32.store8
        local.get $o i32.const 1 i32.add local.set $o
        local.get $i i32.const 1 i32.add local.set $i
        br $val
      end
    end
    end

    i64.const 1
    local.get $o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; emit ","method":"Page.navigate","params":{"url":"
  (func $emit_comma_method_navigate
    i32.const 44 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 109 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 116 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 104 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 111 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 100 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 58 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 80 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 97 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 103 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 46 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 110 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 97 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 118 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 105 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 103 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 97 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 116 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o)

  ;; emit ","params":{"url":"
  (func $emit_comma_params_url
    i32.const 44 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 112 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 97 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 114 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 97 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 109 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 115 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 58 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 123 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 117 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 114 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 108 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 58 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o)

  ;; emit ","method":"Runtime.evaluate","params":{"expression":"
  (func $emit_comma_method_evaluate
    i32.const 44 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 109 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 116 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 104 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 111 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 100 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 58 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 82 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 117 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 110 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 116 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 105 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 109 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 46 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 118 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 97 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 108 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 117 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 97 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 116 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o)

  ;; emit ","params":{"expression":"
  (func $emit_comma_params_expression
    i32.const 44 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 112 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 97 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 114 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 97 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 109 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 115 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 58 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 123 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 120 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 112 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 114 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 115 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 115 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 105 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 111 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 110 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 58 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o)

  ;; emit ","returnByValue":true}}
  (func $emit_close_returnbyvalue
    i32.const 44 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 34 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 114 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 116 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 117 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 114 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 110 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 66 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 121 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 86 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 97 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 108 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 117 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 58 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 116 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 114 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 117 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 101 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 125 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o
    i32.const 125 global.get $g_out global.get $g_o i32.add i32.store8
    global.get $g_o i32.const 1 i32.add global.set $g_o)

;; Status: 0 ok, 2 no function, 3 unsupported/invalid.
  ;; Languages: 1 rust, 2 ts/js, 3 c, 4 py, 5 go, 6 java, 0 unknown.
  ;; First function out record, 24 bytes:
  ;;   u32 name_start, u32 name_len, u32 start_byte, u32 end_byte, u32 is_static, u32 lang.


  (func $m44is_ws (param $c i32) (result i32)
    local.get $c
    i32.const 32
    i32.eq
    local.get $c
    i32.const 9
    i32.eq
    i32.or
    local.get $c
    i32.const 10
    i32.eq
    i32.or
    local.get $c
    i32.const 13
    i32.eq
    i32.or)

  (func $is_ident_start (param $c i32) (result i32)
    local.get $c
    i32.const 95
    i32.eq
    local.get $c
    i32.const 65
    i32.ge_u
    local.get $c
    i32.const 90
    i32.le_u
    i32.and
    i32.or
    local.get $c
    i32.const 97
    i32.ge_u
    local.get $c
    i32.const 122
    i32.le_u
    i32.and
    i32.or)

  (func $is_ident (param $c i32) (result i32)
    local.get $c
    call $is_ident_start
    local.get $c
    i32.const 48
    i32.ge_u
    local.get $c
    i32.const 57
    i32.le_u
    i32.and
    i32.or)

  (func $prev_ident (param $ptr i32) (param $pos i32) (result i32)
    local.get $pos
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $pos
    i32.const 1
    i32.sub
    call $m44ch
    call $is_ident)

  (func $m44skip_ws (param $ptr i32) (param $len i32) (param $pos i32) (result i32)
    (local $i i32)
    local.get $pos
    local.set $i
    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        call $m44ch
        call $m44is_ws
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $i)

  (func $ident_end (param $ptr i32) (param $len i32) (param $pos i32) (result i32)
    (local $i i32)
    local.get $pos
    local.get $len
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $ptr
    local.get $pos
    call $m44ch
    call $is_ident_start
    i32.eqz
    if
      i32.const -1
      return
    end
    local.get $pos
    i32.const 1
    i32.add
    local.set $i
    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        call $m44ch
        call $is_ident
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $i)

  (func $m44ends (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (result i32)
    local.get $len
    i32.const 3
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $len
    i32.const 3
    i32.sub
    call $m44ch
    local.get $a
    i32.eq
    local.get $ptr
    local.get $len
    i32.const 2
    i32.sub
    call $m44ch
    local.get $b
    i32.eq
    i32.and
    local.get $ptr
    local.get $len
    i32.const 1
    i32.sub
    call $m44ch
    local.get $c
    i32.eq
    i32.and)

  (func $m44ends2 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (result i32)
    local.get $len
    i32.const 2
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $len
    i32.const 2
    i32.sub
    call $m44ch
    local.get $a
    i32.eq
    local.get $ptr
    local.get $len
    i32.const 1
    i32.sub
    call $m44ch
    local.get $b
    i32.eq
    i32.and)

  (func $lang_for_path (export "codelyzer_lang_for_path") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len i32.const 46 i32.const 114 i32.const 115 call $m44ends
    if i32.const 1 return end
    local.get $ptr local.get $len i32.const 46 i32.const 116 i32.const 115 call $m44ends
    local.get $ptr local.get $len i32.const 46 i32.const 106 i32.const 115 call $m44ends
    i32.or
    if i32.const 2 return end
    local.get $ptr local.get $len i32.const 46 i32.const 99 call $m44ends2
    local.get $ptr local.get $len i32.const 46 i32.const 104 call $m44ends2
    i32.or
    if i32.const 3 return end
    local.get $ptr local.get $len i32.const 46 i32.const 112 i32.const 121 call $m44ends
    if i32.const 4 return end
    local.get $ptr local.get $len i32.const 46 i32.const 103 i32.const 111 call $m44ends
    if i32.const 5 return end
    local.get $ptr local.get $len i32.const 106 i32.const 97 i32.const 118 call $m44ends
    local.get $ptr local.get $len i32.const 46 i32.const 97 i32.const 118 call $m44ends
    i32.and
    if i32.const 6 return end
    local.get $ptr local.get $len i32.const 106 i32.const 97 i32.const 97 call $m44ends
    drop
    local.get $ptr local.get $len i32.const 46 i32.const 97 i32.const 118 call $m44ends
    drop
    local.get $len
    i32.const 5
    i32.ge_u
    if
      local.get $ptr local.get $len i32.const 5 i32.sub call $m44ch i32.const 46 i32.eq
      local.get $ptr local.get $len i32.const 4 i32.sub call $m44ch i32.const 106 i32.eq i32.and
      local.get $ptr local.get $len i32.const 3 i32.sub call $m44ch i32.const 97 i32.eq i32.and
      local.get $ptr local.get $len i32.const 2 i32.sub call $m44ch i32.const 118 i32.eq i32.and
      local.get $ptr local.get $len i32.const 1 i32.sub call $m44ch i32.const 97 i32.eq i32.and
      if i32.const 6 return end
    end
    i32.const 0)

  (func $find_close_brace (param $ptr i32) (param $len i32) (param $start i32) (result i32)
    (local $i i32) (local $depth i32) (local $c i32)
    local.get $start
    local.set $i
    i32.const 0
    local.set $depth
    block $done
      loop $loop
        local.get $i local.get $len i32.ge_u br_if $done
        local.get $ptr local.get $i call $m44ch local.set $c
        local.get $c i32.const 123 i32.eq
        if
          local.get $depth i32.const 1 i32.add local.set $depth
        end
        local.get $c i32.const 125 i32.eq
        if
          local.get $depth i32.const 1 i32.sub local.tee $depth
          i32.eqz
          if
            local.get $i i32.const 1 i32.add return
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    end
    local.get $len)

  (func $m44write_out
    (param $out i32) (param $name_start i32) (param $name_end i32)
    (param $start i32) (param $end i32) (param $static i32) (param $lang i32)
    local.get $out local.get $name_start i32.store align=1
    local.get $out i32.const 4 i32.add local.get $name_end local.get $name_start i32.sub i32.store align=1
    local.get $out i32.const 8 i32.add local.get $start i32.store align=1
    local.get $out i32.const 12 i32.add local.get $end i32.store align=1
    local.get $out i32.const 16 i32.add local.get $static i32.store align=1
    local.get $out i32.const 20 i32.add local.get $lang i32.store align=1)

  (func $keyword_at (param $ptr i32) (param $len i32) (param $pos i32) (param $lang i32) (result i32)
    ;; Returns keyword byte length: rust 2, py 3, js 8, go 4, else 0.
    local.get $lang
    i32.const 1
    i32.eq
    if
      local.get $pos i32.const 1 i32.add local.get $len i32.lt_u
      local.get $ptr local.get $pos call $m44ch i32.const 102 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 1 i32.add call $m44ch i32.const 110 i32.eq i32.and
      local.get $pos i32.eqz local.get $ptr local.get $pos call $prev_ident i32.eqz i32.or i32.and
      if i32.const 2 return end
    end
    local.get $lang
    i32.const 4
    i32.eq
    if
      local.get $pos i32.const 2 i32.add local.get $len i32.lt_u
      local.get $ptr local.get $pos call $m44ch i32.const 100 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 1 i32.add call $m44ch i32.const 101 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 2 i32.add call $m44ch i32.const 102 i32.eq i32.and
      local.get $pos i32.eqz local.get $ptr local.get $pos call $prev_ident i32.eqz i32.or i32.and
      if i32.const 3 return end
    end
    local.get $lang
    i32.const 5
    i32.eq
    if
      local.get $pos i32.const 3 i32.add local.get $len i32.lt_u
      local.get $ptr local.get $pos call $m44ch i32.const 102 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 1 i32.add call $m44ch i32.const 117 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 2 i32.add call $m44ch i32.const 110 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 3 i32.add call $m44ch i32.const 99 i32.eq i32.and
      local.get $pos i32.eqz local.get $ptr local.get $pos call $prev_ident i32.eqz i32.or i32.and
      if i32.const 4 return end
    end
    local.get $lang
    i32.const 2
    i32.eq
    if
      local.get $pos i32.const 7 i32.add local.get $len i32.lt_u
      local.get $ptr local.get $pos call $m44ch i32.const 102 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 1 i32.add call $m44ch i32.const 117 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 2 i32.add call $m44ch i32.const 110 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 3 i32.add call $m44ch i32.const 99 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 4 i32.add call $m44ch i32.const 116 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 5 i32.add call $m44ch i32.const 105 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 6 i32.add call $m44ch i32.const 111 i32.eq i32.and
      local.get $ptr local.get $pos i32.const 7 i32.add call $m44ch i32.const 110 i32.eq i32.and
      local.get $pos i32.eqz local.get $ptr local.get $pos call $prev_ident i32.eqz i32.or i32.and
      if i32.const 8 return end
    end
    i32.const 0)

  (func $find_func (param $ptr i32) (param $len i32) (param $from i32) (param $lang i32) (param $out i32) (result i32)
    (local $i i32) (local $c i32) (local $kw i32) (local $name_start i32) (local $name_end i32)
    (local $paren i32) (local $brace i32) (local $line_start i32) (local $static i32)
    local.get $from local.set $i
    local.get $from local.set $line_start
    block $not_found
      loop $scan
        local.get $i local.get $len i32.ge_u br_if $not_found
        local.get $ptr local.get $i call $m44ch local.set $c

        ;; Skip line comments, including Python # comments.
        local.get $c i32.const 35 i32.eq
        local.get $c i32.const 47 i32.eq
        local.get $i i32.const 1 i32.add local.get $len i32.lt_u i32.and
        local.get $ptr local.get $i i32.const 1 i32.add call $m44ch i32.const 47 i32.eq i32.and
        i32.or
        if
          loop $line
            local.get $i local.get $len i32.ge_u br_if $scan
            local.get $ptr local.get $i call $m44ch i32.const 10 i32.eq br_if $scan
            local.get $i i32.const 1 i32.add local.set $i
            br $line
          end
        end

        ;; Skip block comments.
        local.get $c i32.const 47 i32.eq
        local.get $i i32.const 1 i32.add local.get $len i32.lt_u i32.and
        local.get $ptr local.get $i i32.const 1 i32.add call $m44ch i32.const 42 i32.eq i32.and
        if
          local.get $i i32.const 2 i32.add local.set $i
          loop $block
            local.get $i i32.const 1 i32.add local.get $len i32.ge_u br_if $not_found
            local.get $ptr local.get $i call $m44ch i32.const 42 i32.eq
            local.get $ptr local.get $i i32.const 1 i32.add call $m44ch i32.const 47 i32.eq i32.and
            if
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
            local.get $i i32.const 1 i32.add local.set $i
            br $block
          end
        end

        ;; Skip simple quoted strings and template literals.
        local.get $c i32.const 34 i32.eq
        local.get $c i32.const 39 i32.eq i32.or
        local.get $c i32.const 96 i32.eq i32.or
        if
          local.get $i i32.const 1 i32.add local.set $i
          loop $str
            local.get $i local.get $len i32.ge_u br_if $not_found
            local.get $ptr local.get $i call $m44ch i32.const 92 i32.eq
            if
              local.get $i i32.const 2 i32.add local.set $i
              br $str
            end
            local.get $ptr local.get $i call $m44ch local.get $c i32.eq
            if
              local.get $i i32.const 1 i32.add local.set $i
              br $scan
            end
            local.get $i i32.const 1 i32.add local.set $i
            br $str
          end
        end

        local.get $c i32.const 10 i32.eq
        if
          local.get $i i32.const 1 i32.add local.set $line_start
        end

        local.get $ptr local.get $len local.get $i local.get $lang call $keyword_at
        local.tee $kw
        if
          local.get $ptr local.get $len local.get $i local.get $kw i32.add call $m44skip_ws local.set $name_start
          local.get $lang i32.const 5 i32.eq
          local.get $name_start local.get $len i32.lt_u i32.and
          local.get $ptr local.get $name_start call $m44ch i32.const 40 i32.eq i32.and
          if
            ;; Go receiver: func (r Receiver) Name(
            local.get $name_start i32.const 1 i32.add local.set $paren
            loop $recv
              local.get $paren local.get $len i32.ge_u br_if $not_found
              local.get $ptr local.get $paren call $m44ch i32.const 41 i32.eq
              if
                local.get $ptr local.get $len local.get $paren i32.const 1 i32.add call $m44skip_ws local.set $name_start
                br $recv
              end
              local.get $paren i32.const 1 i32.add local.set $paren
              br $recv
            end
          end
          local.get $ptr local.get $len local.get $name_start call $ident_end local.tee $name_end
          i32.const -1
          i32.ne
          if
            local.get $lang i32.const 4 i32.eq
            if
              local.get $len local.set $brace
            else
              local.get $i local.set $brace
              loop $brace_scan
                local.get $brace local.get $len i32.ge_u br_if $not_found
                local.get $ptr local.get $brace call $m44ch i32.const 123 i32.eq
                if
                  local.get $ptr local.get $len local.get $brace call $find_close_brace local.set $brace
                  local.get $out local.get $name_start local.get $name_end local.get $i local.get $brace i32.const 0 local.get $lang call $m44write_out
                  i32.const 0
                  return
                end
                local.get $brace i32.const 1 i32.add local.set $brace
                br $brace_scan
              end
            end
            local.get $out local.get $name_start local.get $name_end local.get $i local.get $brace i32.const 0 local.get $lang call $m44write_out
            i32.const 0
            return
          end
        end

        ;; C/Java: previous identifier before (...) with a body.
        local.get $lang i32.const 3 i32.eq
        local.get $lang i32.const 6 i32.eq
        i32.or
        local.get $c i32.const 40 i32.eq
        i32.and
        if
          local.get $i local.set $name_end
          block $have_name
            loop $back
              local.get $name_end local.get $line_start i32.le_u br_if $have_name
              local.get $ptr local.get $name_end i32.const 1 i32.sub call $m44ch call $m44is_ws
              i32.eqz br_if $have_name
              local.get $name_end i32.const 1 i32.sub local.set $name_end
              br $back
            end
          end
          local.get $name_end local.set $name_start
          block $back_done
            loop $back_ident
              local.get $name_start local.get $line_start i32.le_u br_if $back_done
              local.get $ptr local.get $name_start i32.const 1 i32.sub call $m44ch call $is_ident
              i32.eqz br_if $back_done
              local.get $name_start i32.const 1 i32.sub local.set $name_start
              br $back_ident
            end
          end
          local.get $name_start local.get $name_end i32.lt_u
          if
            local.get $i local.set $paren
            loop $sig
              local.get $paren local.get $len i32.ge_u br_if $not_found
              local.get $ptr local.get $paren call $m44ch i32.const 41 i32.eq
              if
                local.get $ptr local.get $len local.get $paren i32.const 1 i32.add call $m44skip_ws local.set $brace
                local.get $brace local.get $len i32.lt_u
                local.get $ptr local.get $brace call $m44ch i32.const 123 i32.eq i32.and
                if
                  local.get $ptr local.get $len local.get $brace call $find_close_brace local.set $brace
                  i32.const 0 local.set $static
                  local.get $out local.get $name_start local.get $name_end local.get $line_start local.get $brace local.get $static local.get $lang call $m44write_out
                  i32.const 0 return
                end
                br $sig
              end
              local.get $paren i32.const 1 i32.add local.set $paren
              br $sig
            end
          end
        end

        local.get $i i32.const 1 i32.add local.set $i
        br $scan
      end
    end
    i32.const 2)

  (func (export "codelyzer_scan_first_function")
    (param $path_ptr i32) (param $path_len i32) (param $src_ptr i32) (param $src_len i32) (param $out i32) (result i32)
    (local $lang i32)
    local.get $path_ptr local.get $path_len call $lang_for_path local.set $lang
    local.get $lang i32.eqz
    if
      i32.const 3
      return
    end
    local.get $src_ptr local.get $src_len i32.const 0 local.get $lang local.get $out call $find_func)

  (func (export "codelyzer_count_functions")
    (param $path_ptr i32) (param $path_len i32) (param $src_ptr i32) (param $src_len i32) (result i64)
    (local $lang i32) (local $pos i32) (local $status i32) (local $count i32) (local $out i32) (local $end i32)
    i32.const 65500
    local.set $out
    local.get $path_ptr local.get $path_len call $lang_for_path local.set $lang
    local.get $lang i32.eqz
    if
      i64.const 3
      return
    end
    i32.const 0 local.set $pos
    i32.const 0 local.set $count
    block $done
      loop $loop
        local.get $src_ptr local.get $src_len local.get $pos local.get $lang local.get $out call $find_func
        local.tee $status
        i32.const 2
        i32.eq
        br_if $done
        local.get $status
        if
          local.get $status
          i64.extend_i32_u
          return
        end
        local.get $count i32.const 1 i32.add local.set $count
        local.get $out i32.const 12 i32.add i32.load align=1 local.set $end
        local.get $end local.get $pos i32.le_u
        if
          local.get $pos i32.const 1 i32.add local.set $pos
        else
          local.get $end local.set $pos
        end
        local.get $pos local.get $src_len i32.ge_u br_if $done
        br $loop
      end
    end
    local.get $count
    i64.extend_i32_u
    i64.const 32
    i64.shl)
)


  (func $m148hash (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $h i32)
    i32.const 5381
    local.set $h
    loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $h
        i32.const 33
        i32.mul
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.add
        local.set $h
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    local.get $h)

  (func $seg (param $len i32) (param $m148hash i32) (param $want_len i32) (param $want_hash i32) (result i32)
    local.get $len
    local.get $want_len
    i32.eq
    local.get $m148hash
    local.get $want_hash
    i32.eq
    i32.and)

  (func $is_valid_name_byte (param $b i32) (result i32)
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and
    i32.or
    local.get $b
    i32.const 97
    i32.ge_u
    local.get $b
    i32.const 122
    i32.le_u
    i32.and
    i32.or
    local.get $b
    i32.const 95
    i32.eq
    i32.or)

  (func (export "pocketbase_valid_name") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $len
    i32.eqz
    if
      i32.const 0
      return
    end
    loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_valid_name_byte
        i32.eqz
        if
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 1)

  (func $pocketbase_method_code (export "pocketbase_method_code") (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 3
    i32.eq
    if
      local.get $ptr
      i32.load8_u
      i32.const 71
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 69
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      if
        i32.const 1
        return
      end
      local.get $ptr
      i32.load8_u
      i32.const 80
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 85
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      if
        i32.const 4
        return
      end
    end
    local.get $len
    i32.const 4
    i32.eq
    if
      local.get $ptr
      i32.load8_u
      i32.const 80
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 79
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 83
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      if
        i32.const 2
        return
      end
    end
    local.get $len
    i32.const 5
    i32.eq
    if
      local.get $ptr
      i32.load8_u
      i32.const 80
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 65
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 67
      i32.eq
      i32.and
      local.get $ptr
      i32.const 4
      i32.add
      i32.load8_u
      i32.const 72
      i32.eq
      i32.and
      if
        i32.const 3
        return
      end
    end
    local.get $len
    i32.const 6
    i32.eq
    if
      local.get $ptr
      i32.load8_u
      i32.const 68
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 69
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 76
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 69
      i32.eq
      i32.and
      local.get $ptr
      i32.const 4
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      local.get $ptr
      i32.const 5
      i32.add
      i32.load8_u
      i32.const 69
      i32.eq
      i32.and
      if
        i32.const 5
        return
      end
    end
    local.get $len
    i32.const 7
    i32.eq
    if
      local.get $ptr
      i32.load8_u
      i32.const 79
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 80
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 73
      i32.eq
      i32.and
      local.get $ptr
      i32.const 4
      i32.add
      i32.load8_u
      i32.const 79
      i32.eq
      i32.and
      local.get $ptr
      i32.const 5
      i32.add
      i32.load8_u
      i32.const 78
      i32.eq
      i32.and
      local.get $ptr
      i32.const 6
      i32.add
      i32.load8_u
      i32.const 83
      i32.eq
      i32.and
      if
        i32.const 6
        return
      end
    end
    i32.const 0)

  (func $method_allowed (param $method i32) (param $mask i32) (result i32)
    local.get $method
    i32.const 0
    i32.gt_s
    if (result i32)
      local.get $mask
      local.get $method
      i32.shr_u
      i32.const 1
      i32.and
    else
      i32.const 0
    end)

  (func $finish (param $out i32) (param $route i32) (param $method i32) (param $mask i32) (param $auth i32) (param $ok_status i32) (param $activity i32) (result i32)
    (local $allowed i32)
    local.get $method
    local.get $mask
    call $method_allowed
    local.set $allowed
    local.get $out
    local.get $route
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $method
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $allowed
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $auth
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $allowed
    if (result i32)
      local.get $ok_status
    else
      i32.const 405
    end
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $activity
    i32.store
    local.get $route)

  (func $finish_404 (param $out i32) (param $method i32) (result i32)
    local.get $out
    i32.const 99
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $method
    i32.store
    local.get $out
    i32.const 8
    i32.add
    i32.const 1
    i32.store
    local.get $out
    i32.const 12
    i32.add
    i32.const 0
    i32.store
    local.get $out
    i32.const 16
    i32.add
    i32.const 404
    i32.store
    local.get $out
    i32.const 20
    i32.add
    i32.const 1
    i32.store
    i32.const 99)

  (func (export "pocketbase_classify_route") (param $m_ptr i32) (param $m_len i32) (param $p_ptr i32) (param $p_len i32) (param $out i32) (result i32)
    (local $method i32)
    (local $start i32)
    (local $end i32)
    (local $i i32)
    (local $idx i32)
    (local $cur_len i32)
    (local $cur_hash i32)
    (local $l1 i32) (local $h1 i32)
    (local $l2 i32) (local $h2 i32)
    (local $l3 i32) (local $h3 i32)
    (local $l4 i32) (local $h4 i32)
    (local $l5 i32) (local $h5 i32)
    (local $b i32)
    local.get $m_ptr
    local.get $m_len
    call $pocketbase_method_code
    local.set $method

    local.get $p_len
    local.set $end
    i32.const 0
    local.set $i
    loop $query
      local.get $i
      local.get $end
      i32.lt_u
      if
        local.get $p_ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 63
        i32.eq
        if
          local.get $i
          local.set $end
        else
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $query
        end
      end
    end
    loop $lead
      local.get $start
      local.get $end
      i32.lt_u
      local.get $p_ptr
      local.get $start
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      if
        local.get $start
        i32.const 1
        i32.add
        local.set $start
        br $lead
      end
    end
    loop $trail
      local.get $end
      local.get $start
      i32.gt_u
      local.get $p_ptr
      local.get $end
      i32.const 1
      i32.sub
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      if
        local.get $end
        i32.const 1
        i32.sub
        local.set $end
        br $trail
      end
    end

    local.get $start
    local.get $end
    i32.eq
    if
      local.get $out
      i32.const 1
      local.get $method
      i32.const 2
      i32.const 0
      i32.const 200
      i32.const 1
      call $finish
      return
    end

    i32.const 1
    local.set $idx
    i32.const 5381
    local.set $cur_hash
    local.get $start
    local.set $i
    loop $parse
      local.get $i
      local.get $end
      i32.le_u
      if
        local.get $i
        local.get $end
        i32.eq
        if
          i32.const 47
          local.set $b
        else
          local.get $p_ptr
          local.get $i
          i32.add
          i32.load8_u
          local.set $b
        end
        local.get $b
        i32.const 47
        i32.eq
        if
          local.get $idx
          i32.const 1
          i32.eq
          if
            local.get $cur_len
            local.set $l1
            local.get $cur_hash
            local.set $h1
          end
          local.get $idx
          i32.const 2
          i32.eq
          if
            local.get $cur_len
            local.set $l2
            local.get $cur_hash
            local.set $h2
          end
          local.get $idx
          i32.const 3
          i32.eq
          if
            local.get $cur_len
            local.set $l3
            local.get $cur_hash
            local.set $h3
          end
          local.get $idx
          i32.const 4
          i32.eq
          if
            local.get $cur_len
            local.set $l4
            local.get $cur_hash
            local.set $h4
          end
          local.get $idx
          i32.const 5
          i32.eq
          if
            local.get $cur_len
            local.set $l5
            local.get $cur_hash
            local.set $h5
          end
          local.get $idx
          i32.const 1
          i32.add
          local.set $idx
          i32.const 0
          local.set $cur_len
          i32.const 5381
          local.set $cur_hash
        else
          local.get $cur_hash
          i32.const 33
          i32.mul
          local.get $b
          i32.add
          local.set $cur_hash
          local.get $cur_len
          i32.const 1
          i32.add
          local.set $cur_len
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $parse
      end
    end
    local.get $idx
    i32.const 1
    i32.sub
    local.set $idx

    local.get $l1
    local.get $h1
    i32.const 1
    i32.const 0x0002b604
    call $seg
    if
      local.get $idx
      i32.const 1
      i32.eq
      if
        local.get $out
        i32.const 1
        local.get $method
        i32.const 2
        i32.const 0
        i32.const 200
        i32.const 1
        call $finish
        return
      end
      local.get $idx
      i32.const 2
      i32.eq
      if
        local.get $l2
        local.get $h2
        i32.const 11
        i32.const 0x0ca56be4
        call $seg
        if
          local.get $out
          i32.const 2
          local.get $method
          i32.const 2
          i32.const 0
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l2
        local.get $h2
        i32.const 20
        i32.const 0xf5593098
        call $seg
        local.get $l2
        local.get $h2
        i32.const 16
        i32.const 0x7d7aa7f1
        call $seg
        i32.or
        if
          local.get $out
          i32.const 2
          local.get $method
          i32.const 2
          i32.const 0
          i32.const 200
          i32.const 1
          call $finish
          return
        end
      end
      local.get $idx
      i32.const 3
      i32.eq
      if
        local.get $l2
        local.get $h2
        i32.const 6
        i32.const 0xf2853858
        call $seg
        if
          local.get $out
          i32.const 2
          local.get $method
          i32.const 2
          i32.const 0
          i32.const 200
          i32.const 0
          call $finish
          return
        end
      end
    end

    local.get $idx
    i32.const 3
    i32.eq
    local.get $l1
    local.get $h1
    i32.const 11
    i32.const 0x4e327181
    call $seg
    i32.and
    local.get $l2
    local.get $h2
    i32.const 14
    i32.const 0x9202368b
    call $seg
    i32.and
    if
      local.get $out
      i32.const 41
      local.get $method
      i32.const 2
      i32.const 0
      i32.const 200
      i32.const 1
      call $finish
      return
    end

    local.get $l1
    local.get $h1
    i32.const 3
    i32.const 0x0b885e5f
    call $seg
    i32.eqz
    if
      local.get $out
      local.get $method
      call $finish_404
      return
    end

    local.get $idx
    i32.const 2
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 6
    i32.const 0x01d23c9b
    call $seg
    i32.and
    if
      local.get $out
      i32.const 3
      local.get $method
      i32.const 2
      i32.const 0
      i32.const 200
      i32.const 0
      call $finish
      return
    end

    local.get $l2
    local.get $h2
    i32.const 11
    i32.const 0x5d358c24
    call $seg
    if
      local.get $idx
      i32.const 2
      i32.eq
      if
        local.get $out
        i32.const 10
        local.get $method
        i32.const 6
        i32.const 2
        i32.const 200
        i32.const 1
        call $finish
        return
      end
      local.get $idx
      i32.const 3
      i32.eq
      if
        local.get $l3
        local.get $h3
        i32.const 6
        i32.const 0x04c06f80
        call $seg
        if
          local.get $out
          i32.const 11
          local.get $method
          i32.const 16
          i32.const 2
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l3
        local.get $h3
        i32.const 4
        i32.const 0x7c9a91cc
        call $seg
        if
          local.get $out
          i32.const 12
          local.get $method
          i32.const 6
          i32.const 2
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $out
        i32.const 13
        local.get $method
        i32.const 42
        i32.const 2
        i32.const 200
        i32.const 1
        call $finish
        return
      end
      local.get $idx
      i32.const 4
      i32.eq
      if
        local.get $l4
        local.get $h4
        i32.const 8
        i32.const 0xe9e0dc6b
        call $seg
        if
          local.get $out
          i32.const 14
          local.get $method
          i32.const 32
          i32.const 2
          i32.const 204
          i32.const 1
          call $finish
          return
        end
        local.get $l4
        local.get $h4
        i32.const 7
        i32.const 0x3e05fd17
        call $seg
        if
          local.get $out
          i32.const 15
          local.get $method
          i32.const 6
          i32.const 4
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l4
        local.get $h4
        i32.const 18
        i32.const 0x1b71ace0
        call $seg
        local.get $l4
        local.get $h4
        i32.const 16
        i32.const 0xaba79580
        call $seg
        i32.or
        if
          local.get $out
          i32.const 19
          local.get $method
          i32.const 4
          i32.const 0
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l4
        local.get $h4
        i32.const 12
        i32.const 0xc1a02cd3
        call $seg
        if
          local.get $out
          i32.const 21
          local.get $method
          i32.const 4
          i32.const 3
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l4
        local.get $h4
        i32.const 12
        i32.const 0x41b500f8
        call $seg
        if
          local.get $out
          i32.const 22
          local.get $method
          i32.const 2
          i32.const 0
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $out
        i32.const 13
        local.get $method
        i32.const 42
        i32.const 2
        i32.const 200
        i32.const 1
        call $finish
        return
      end
      local.get $idx
      i32.const 5
      i32.eq
      if
        local.get $l4
        local.get $h4
        i32.const 7
        i32.const 0x3e05fd17
        call $seg
        if
          local.get $out
          i32.const 16
          local.get $method
          i32.const 42
          i32.const 4
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l4
        local.get $h4
        i32.const 11
        i32.const 0xdf2143ac
        call $seg
        if
          local.get $out
          i32.const 24
          local.get $method
          i32.const 4
          i32.const 2
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $out
        i32.const 23
        local.get $method
        i32.const 4
        i32.const 0
        i32.const 200
        i32.const 1
        call $finish
        return
      end
    end

    local.get $idx
    i32.const 5
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 5
    i32.const 0x0f703038
    call $seg
    i32.and
    if
      local.get $out
      i32.const 17
      local.get $method
      i32.const 2
      i32.const 4
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $idx
    i32.const 2
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 8
    i32.const 0xf9e632d8
    call $seg
    i32.and
    if
      local.get $out
      i32.const 25
      local.get $method
      i32.const 6
      i32.const 4
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $idx
    i32.const 2
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 6
    i32.const 0xf1728ec1
    call $seg
    i32.and
    if
      local.get $out
      i32.const 30
      local.get $method
      i32.const 6
      i32.const 5
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $idx
    i32.const 3
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 6
    i32.const 0xf1728ec1
    call $seg
    i32.and
    if
      local.get $out
      i32.const 31
      local.get $method
      i32.const 4
      i32.const 0
      i32.const 200
      i32.const 1
      call $finish
      return
    end

    local.get $l2
    local.get $h2
    i32.const 7
    i32.const 0x650b6b4e
    call $seg
    if
      local.get $out
      i32.const 32
      local.get $method
      local.get $idx
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 6
      else
        local.get $idx
        i32.const 3
        i32.eq
        if (result i32)
          i32.const 34
        else
          i32.const 4
        end
      end
      i32.const 2
      local.get $idx
      i32.const 3
      i32.eq
      local.get $l4
      local.get $h4
      i32.const 7
      i32.const 0x3f2a3809
      call $seg
      i32.and
      if (result i32)
        i32.const 200
      else
        i32.const 200
      end
      i32.const 1
      call $finish
      return
    end

    local.get $idx
    i32.const 2
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 10
    i32.const 0xf5d6b242
    call $seg
    i32.and
    if
      local.get $out
      i32.const 34
      local.get $method
      i32.const 6
      i32.const 2
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $l2
    local.get $h2
    i32.const 8
    i32.const 0x1304dc16
    call $seg
    if
      local.get $out
      i32.const 35
      local.get $method
      local.get $idx
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 10
      else
        i32.const 4
      end
      i32.const 2
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $l2
    local.get $h2
    i32.const 4
    i32.const 0x7c9a2e5a
    call $seg
    if
      local.get $out
      i32.const 36
      local.get $method
      i32.const 2
      i32.const 2
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $l2
    local.get $h2
    i32.const 5
    i32.const 0x0f3ee40a
    call $seg
    if
      local.get $out
      i32.const 37
      local.get $method
      local.get $idx
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 4
      end
      i32.const 2
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $idx
    i32.const 2
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 5
    i32.const 0x0f238ce7
    call $seg
    i32.and
    if
      local.get $out
      i32.const 38
      local.get $method
      i32.const 4
      i32.const 0
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $l2
    local.get $h2
    i32.const 7
    i32.const 0x533ea70f
    call $seg
    if
      local.get $out
      i32.const 40
      local.get $method
      local.get $idx
      i32.const 5
      i32.eq
      if (result i32)
        i32.const 34
      else
        i32.const 22
      end
      i32.const 2
      i32.const 200
      i32.const 1
      call $finish
      return
    end

    local.get $out
    local.get $method
    call $finish_404)

  (func (export "pocketbase_collection_kind_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $m148hash
    local.set $h
    local.get $len
    i32.const 4
    i32.eq
    local.get $h
    i32.const 0x7c944157
    i32.eq
    i32.and
    if
      i32.const 2
      return
    end
    i32.const 1)

  (func (export "pocketbase_actor_kind_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $m148hash
    local.set $h
    local.get $len
    i32.const 5
    i32.eq
    local.get $h
    i32.const 0x0f12fc8e
    i32.eq
    i32.and
    if
      i32.const 1
      return
    end
    local.get $len
    i32.const 6
    i32.eq
    local.get $h
    i32.const 0x1926f824
    i32.eq
    i32.and
    if
      i32.const 2
      return
    end
    local.get $len
    i32.const 4
    i32.eq
    local.get $h
    i32.const 0x7c96cb25
    i32.eq
    i32.and
    if
      i32.const 3
      return
    end
    i32.const 0)

  (func (export "pocketbase_auth_status") (param $auth_req i32) (param $actor_kind i32) (param $admin_exists i32) (result i32)
    local.get $auth_req
    i32.eqz
    if
      i32.const 200
      return
    end
    local.get $auth_req
    i32.const 5
    i32.eq
    local.get $admin_exists
    i32.eqz
    i32.and
    if
      i32.const 200
      return
    end
    local.get $actor_kind
    i32.eqz
    if
      i32.const 401
      return
    end
    local.get $auth_req
    i32.const 1
    i32.eq
    if
      i32.const 200
      return
    end
    local.get $auth_req
    i32.const 2
    i32.eq
    local.get $actor_kind
    i32.const 1
    i32.eq
    i32.and
    if
      i32.const 200
      return
    end
    local.get $auth_req
    i32.const 3
    i32.eq
    local.get $actor_kind
    i32.const 2
    i32.eq
    i32.and
    if
      i32.const 200
      return
    end
    local.get $auth_req
    i32.const 4
    i32.eq
    if
      i32.const 200
      return
    end
    local.get $auth_req
    i32.const 5
    i32.eq
    local.get $admin_exists
    i32.eqz
    i32.or
    local.get $actor_kind
    i32.const 1
    i32.eq
    i32.or
    if
      i32.const 200
      return
    end
    i32.const 403)

  (func (export "pocketbase_field_type_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $m148hash
    local.set $h
    local.get $len
    i32.const 4
    i32.eq
    if
      local.get $h
      i32.const 0x7c9e690a
      i32.eq
      if i32.const 1 return end
      local.get $h
      i32.const 0x7c94b391
      i32.eq
      if i32.const 6 return end
      local.get $h
      i32.const 0x7c959163
      i32.eq
      if i32.const 7 return end
      local.get $h
      i32.const 0x7c99279f
      i32.eq
      if i32.const 10 return end
    end
    local.get $len
    i32.const 3
    i32.eq
    local.get $h
    i32.const 0x0b88b3b8
    i32.eq
    i32.and
    if i32.const 3 return end
    local.get $len
    i32.const 5
    i32.eq
    local.get $h
    i32.const 0x0f601aed
    i32.eq
    i32.and
    if i32.const 4 return end
    local.get $len
    i32.const 6
    i32.eq
    if
      local.get $h
      i32.const 0xfac52eac
      i32.eq
      if i32.const 2 return end
      local.get $h
      i32.const 0x10f9208e
      i32.eq
      if i32.const 5 return end
      local.get $h
      i32.const 0x1b80e3c5
      i32.eq
      if i32.const 9 return end
    end
    local.get $len
    i32.const 8
    i32.eq
    if
      local.get $h
      i32.const 0x45aa423c
      i32.eq
      if i32.const 8 return end
      local.get $h
      i32.const 0x12c7e483
      i32.eq
      if i32.const 12 return end
      local.get $h
      i32.const 0xf1bcda2a
      i32.eq
      if i32.const 13 return end
      local.get $h
      i32.const 0x17f6dc38
      i32.eq
      if i32.const 14 return end
    end
    i32.const 0)

  (func (export "pocketbase_field_category") (param $type_code i32) (result i32)
    local.get $type_code
    i32.const 1
    i32.eq
    local.get $type_code
    i32.const 2
    i32.eq
    i32.or
    local.get $type_code
    i32.const 3
    i32.eq
    i32.or
    local.get $type_code
    i32.const 4
    i32.eq
    i32.or
    local.get $type_code
    i32.const 14
    i32.eq
    i32.or
    if i32.const 1 return end
    local.get $type_code
    i32.const 5
    i32.eq
    if i32.const 2 return end
    local.get $type_code
    i32.const 6
    i32.eq
    if i32.const 3 return end
    local.get $type_code
    i32.const 7
    i32.eq
    local.get $type_code
    i32.const 8
    i32.eq
    i32.or
    if i32.const 4 return end
    local.get $type_code
    i32.const 9
    i32.eq
    if i32.const 5 return end
    local.get $type_code
    i32.const 10
    i32.eq
    if i32.const 6 return end
    local.get $type_code
    i32.const 11
    i32.eq
    if i32.const 7 return end
    local.get $type_code
    i32.const 12
    i32.eq
    if i32.const 8 return end
    local.get $type_code
    i32.const 13
    i32.eq
    if i32.const 9 return end
    i32.const 10)

  (func (export "pocketbase_email_shape") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $at i32)
    (local $dot_after i32)
    local.get $len
    i32.eqz
    if i32.const 0 return end
    loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 64
        i32.eq
        if
          local.get $i
          i32.eqz
          if i32.const 0 return end
          i32.const 1
          local.set $at
        else
          local.get $at
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          i32.const 46
          i32.eq
          i32.and
          if
            i32.const 1
            local.set $dot_after
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    local.get $at
    local.get $dot_after
    i32.and
    local.get $ptr
    local.get $len
    i32.const 1
    i32.sub
    i32.add
    i32.load8_u
    i32.const 46
    i32.ne
    i32.and)

  (func (export "pocketbase_url_shape") (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 7
    i32.ge_u
    if
      local.get $ptr
      i32.load8_u
      i32.const 104
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 116
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 116
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 112
      i32.eq
      i32.and
      local.get $ptr
      i32.const 4
      i32.add
      i32.load8_u
      i32.const 58
      i32.eq
      i32.and
      local.get $ptr
      i32.const 5
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      local.get $ptr
      i32.const 6
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      if i32.const 1 return end
    end
    local.get $len
    i32.const 8
    i32.ge_u
    if
      local.get $ptr
      i32.load8_u
      i32.const 104
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 116
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 116
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 112
      i32.eq
      i32.and
      local.get $ptr
      i32.const 4
      i32.add
      i32.load8_u
      i32.const 115
      i32.eq
      i32.and
      local.get $ptr
      i32.const 5
      i32.add
      i32.load8_u
      i32.const 58
      i32.eq
      i32.and
      local.get $ptr
      i32.const 6
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      local.get $ptr
      i32.const 7
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      if i32.const 1 return end
    end
    i32.const 0)

  (func (export "pocketbase_pack_status") (param $code i32) (result i32)
    (local $status i32)
    local.get $code
    i32.const 100
    i32.lt_u
    local.get $code
    i32.const 999
    i32.gt_u
    i32.or
    if
      i32.const 500
      local.set $status
    else
      local.get $code
      local.set $status
    end
    local.get $status
    i32.const 100
    i32.div_u
    i32.const 16
    i32.shl
    local.get $status
    i32.or)

  ;; Status values: 0 ok, 3 invalid.
  ;; Record layout, all little-endian u32:
  ;;  0 scheme_off,  4 scheme_len
  ;;  8 authority_off, 12 authority_len
  ;; 16 host_off, 20 host_len
  ;; 24 path_off, 28 path_len
  ;; 32 query_off, 36 query_len
  ;; 40 fragment_off, 44 fragment_len
  ;; 48 port, 52 has_port
  (func $m189write_record
    (param $out i32)
    (param $scheme_off i32) (param $scheme_len i32)
    (param $authority_off i32) (param $authority_len i32)
    (param $host_off i32) (param $host_len i32)
    (param $path_off i32) (param $path_len i32)
    (param $query_off i32) (param $query_len i32)
    (param $fragment_off i32) (param $fragment_len i32)
    (param $port i32) (param $has_port i32)
    local.get $out
    local.get $scheme_off
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $scheme_len
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $authority_off
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $authority_len
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $host_off
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $host_len
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $path_off
    i32.store
    local.get $out
    i32.const 28
    i32.add
    local.get $path_len
    i32.store
    local.get $out
    i32.const 32
    i32.add
    local.get $query_off
    i32.store
    local.get $out
    i32.const 36
    i32.add
    local.get $query_len
    i32.store
    local.get $out
    i32.const 40
    i32.add
    local.get $fragment_off
    i32.store
    local.get $out
    i32.const 44
    i32.add
    local.get $fragment_len
    i32.store
    local.get $out
    i32.const 48
    i32.add
    local.get $port
    i32.store
    local.get $out
    i32.const 52
    i32.add
    local.get $has_port
    i32.store)

  (func $is_ascii_ws (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.eq
    i32.or
    local.get $b
    i32.const 10
    i32.eq
    i32.or
    local.get $b
    i32.const 13
    i32.eq
    i32.or)

  (func $url_scheme_validate (export "url_scheme_validate") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $len
    i32.eqz
    if
      i32.const 3
      return
    end
    i32.const 0
    local.set $i
    (block $done
      (loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_scheme_byte
        i32.eqz
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop))
    i32.const 0)

  (func (export "url_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $start i32)
    (local $end i32)
    (local $i i32)
    (local $b i32)
    (local $colon i32)
    (local $auth_start i32)
    (local $auth_end i32)
    (local $last_colon i32)
    (local $host_len i32)
    (local $port i32)
    (local $digit i32)
    (local $has_port i32)
    (local $path_off i32)
    (local $path_len i32)
    (local $query_off i32)
    (local $query_len i32)
    (local $fragment_off i32)
    (local $fragment_len i32)

    i32.const 0
    local.set $start
    local.get $len
    local.set $end

    (block $trim_start_done
      (loop $trim_start
        local.get $start
        local.get $end
        i32.ge_u
        br_if $trim_start_done
        local.get $ptr
        local.get $start
        i32.add
        i32.load8_u
        call $is_ascii_ws
        i32.eqz
        br_if $trim_start_done
        local.get $start
        i32.const 1
        i32.add
        local.set $start
        br $trim_start))

    (block $trim_end_done
      (loop $trim_end
        local.get $end
        local.get $start
        i32.le_u
        br_if $trim_end_done
        local.get $ptr
        local.get $end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $is_ascii_ws
        i32.eqz
        br_if $trim_end_done
        local.get $end
        i32.const 1
        i32.sub
        local.set $end
        br $trim_end))

    local.get $start
    local.get $end
    i32.ge_u
    if
      i32.const 3
      return
    end

    i32.const -1
    local.set $colon
    local.get $start
    local.set $i
    (block $scheme_done
      (loop $scheme_scan
        local.get $i
        local.get $end
        i32.ge_u
        br_if $scheme_done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.tee $b
        i32.const 58
        i32.eq
        if
          local.get $i
          local.set $colon
          br $scheme_done
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scheme_scan))

    local.get $colon
    i32.const -1
    i32.eq
    if
      i32.const 3
      return
    end

    local.get $ptr
    local.get $start
    i32.add
    local.get $colon
    local.get $start
    i32.sub
    call $url_scheme_validate
    if
      i32.const 3
      return
    end

    local.get $colon
    i32.const 3
    i32.add
    local.get $end
    i32.gt_u
    if
      i32.const 3
      return
    end
    local.get $ptr
    local.get $colon
    i32.const 1
    i32.add
    i32.add
    i32.load8_u
    i32.const 47
    i32.ne
    if
      i32.const 3
      return
    end
    local.get $ptr
    local.get $colon
    i32.const 2
    i32.add
    i32.add
    i32.load8_u
    i32.const 47
    i32.ne
    if
      i32.const 3
      return
    end

    local.get $colon
    i32.const 3
    i32.add
    local.set $auth_start
    local.get $auth_start
    local.set $auth_end
    i32.const -1
    local.set $last_colon

    (block $auth_done
      (loop $auth_scan
        local.get $auth_end
        local.get $end
        i32.ge_u
        br_if $auth_done
        local.get $ptr
        local.get $auth_end
        i32.add
        i32.load8_u
        local.tee $b
        i32.const 47
        i32.eq
        local.get $b
        i32.const 63
        i32.eq
        i32.or
        local.get $b
        i32.const 35
        i32.eq
        i32.or
        br_if $auth_done
        local.get $b
        i32.const 64
        i32.eq
        local.get $b
        i32.const 91
        i32.eq
        i32.or
        local.get $b
        i32.const 93
        i32.eq
        i32.or
        if
          i32.const 3
          return
        end
        local.get $b
        i32.const 58
        i32.eq
        if
          local.get $auth_end
          local.set $last_colon
        end
        local.get $auth_end
        i32.const 1
        i32.add
        local.set $auth_end
        br $auth_scan))

    local.get $auth_start
    local.get $auth_end
    i32.ge_u
    if
      i32.const 3
      return
    end

    local.get $auth_end
    local.get $auth_start
    i32.sub
    local.set $host_len
    i32.const 0
    local.set $port
    i32.const 0
    local.set $has_port

    local.get $last_colon
    i32.const -1
    i32.ne
    if
      local.get $last_colon
      local.get $auth_start
      i32.eq
      if
        i32.const 3
        return
      end
      local.get $last_colon
      i32.const 1
      i32.add
      local.get $auth_end
      i32.ge_u
      if
        i32.const 3
        return
      end
      local.get $last_colon
      local.get $auth_start
      i32.sub
      local.set $host_len
      i32.const 1
      local.set $has_port
      i32.const 0
      local.set $port
      local.get $last_colon
      i32.const 1
      i32.add
      local.set $i
      (block $port_done
        (loop $port_loop
          local.get $i
          local.get $auth_end
          i32.ge_u
          br_if $port_done
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          local.tee $b
          call $is_digit
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $b
          i32.const 48
          i32.sub
          local.set $digit
          local.get $port
          i32.const 6553
          i32.gt_u
          if
            i32.const 3
            return
          end
          local.get $port
          i32.const 6553
          i32.eq
          local.get $digit
          i32.const 5
          i32.gt_u
          i32.and
          if
            i32.const 3
            return
          end
          local.get $port
          i32.const 10
          i32.mul
          local.get $digit
          i32.add
          local.set $port
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $port_loop))
    end

    local.get $auth_end
    local.set $i
    local.get $i
    local.set $path_off
    i32.const 0
    local.set $path_len
    local.get $i
    local.set $query_off
    i32.const 0
    local.set $query_len
    local.get $i
    local.set $fragment_off
    i32.const 0
    local.set $fragment_len

    local.get $i
    local.get $end
    i32.lt_u
    if
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      if
        local.get $i
        local.set $path_off
        (block $path_done
          (loop $path_scan
            local.get $i
            local.get $end
            i32.ge_u
            br_if $path_done
            local.get $ptr
            local.get $i
            i32.add
            i32.load8_u
            local.tee $b
            i32.const 63
            i32.eq
            local.get $b
            i32.const 35
            i32.eq
            i32.or
            br_if $path_done
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $path_scan))
        local.get $i
        local.get $path_off
        i32.sub
        local.set $path_len
      end
    end

    local.get $i
    local.get $end
    i32.lt_u
    if
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.const 63
      i32.eq
      if
        local.get $i
        i32.const 1
        i32.add
        local.set $query_off
        local.get $query_off
        local.set $i
        (block $query_done
          (loop $query_scan
            local.get $i
            local.get $end
            i32.ge_u
            br_if $query_done
            local.get $ptr
            local.get $i
            i32.add
            i32.load8_u
            i32.const 35
            i32.eq
            br_if $query_done
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $query_scan))
        local.get $i
        local.get $query_off
        i32.sub
        local.set $query_len
      end
    end

    local.get $i
    local.get $end
    i32.lt_u
    if
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.const 35
      i32.eq
      if
        local.get $i
        i32.const 1
        i32.add
        local.set $fragment_off
        local.get $end
        local.get $fragment_off
        i32.sub
        local.set $fragment_len
      else
        i32.const 3
        return
      end
    end

    local.get $out
    local.get $start
    local.get $colon
    local.get $start
    i32.sub
    local.get $auth_start
    local.get $auth_end
    local.get $auth_start
    i32.sub
    local.get $auth_start
    local.get $host_len
    local.get $path_off
    local.get $path_len
    local.get $query_off
    local.get $query_len
    local.get $fragment_off
    local.get $fragment_len
    local.get $port
    local.get $has_port
    call $m189write_record
    i32.const 0)

  ;; ============================================================
  ;; 256-byte lookup table at offset 0x2000 (8192)
  ;; bit 0: scheme byte (+ - . 0-9 A-Z a-z)
  ;; bit 1: digit (0-9)
  ;; bit 2: whitespace (HT LF CR SP)
  ;; ============================================================

  ;; -----------------------------------------------------------
  ;; LUT-based scheme byte check (reads bit 0 of LUT)
  ;; -----------------------------------------------------------
  (func $is_scheme_byte_lut (param $b i32) (result i32)
    i32.const 8192
    local.get $b
    i32.add
    i32.load8_u
    i32.const 1
    i32.and)

  ;; -----------------------------------------------------------
  ;; simd_capabilities: returns bitmask of SIMD features
  ;; bit 0 = v128 available
  ;; -----------------------------------------------------------
  (func (export "simd_capabilities") (result i32)
    i32.const 1)

  ;; -----------------------------------------------------------
  ;; url_scheme_validate_simd: LUT + SIMD batch pre-check
  ;; Uses v128 to verify all bytes in [0x2B, 0x7A] before
  ;; falling back to per-byte LUT lookups.
  ;; -----------------------------------------------------------
  (func $url_scheme_validate_simd (export "url_scheme_validate_simd")
    (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $v v128)
    (local $ge v128)
    (local $le v128)
    (local $in_range v128)

    local.get $len
    i32.eqz
    if
      i32.const 3
      return
    end

    i32.const 0
    local.set $i

    ;; SIMD batch pre-check: verify all bytes in [0x2B, 0x7A]
    (block $simd_done
      (loop $simd_loop
        local.get $i
        i32.const 16
        i32.add
        local.get $len
        i32.gt_u
        br_if $simd_done

        local.get $ptr
        local.get $i
        i32.add
        v128.load align=1
        local.set $v

        local.get $v
        i32.const 0x2B
        i8x16.splat
        i8x16.ge_u
        local.set $ge

        local.get $v
        i32.const 0x7A
        i8x16.splat
        i8x16.le_u
        local.set $le

        local.get $ge
        local.get $le
        v128.and
        local.set $in_range

        local.get $in_range
        v128.not
        v128.any_true
        if
          i32.const 3
          return
        end

        local.get $i
        i32.const 16
        i32.add
        local.set $i
        br $simd_loop
      )
    )

    ;; Per-byte LUT check for remaining bytes
    (block $done
      (loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done

        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_scheme_byte_lut
        i32.eqz
        if
          i32.const 3
          return
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      )
    )

    i32.const 0
  )

  ;; -----------------------------------------------------------
  ;; $simd_find: find first occurrence of any of up to 3 bytes
  ;; using v128 pre-scan. byte2=0 or byte3=0 means "skip".
  ;; Returns position or -1 if not found.
  ;; -----------------------------------------------------------
  (func $simd_find
    (param $ptr i32)
    (param $start i32)
    (param $end i32)
    (param $byte1 i32)
    (param $byte2 i32)
    (param $byte3 i32)
    (result i32)
    (local $i i32)
    (local $b i32)
    (local $v v128)
    (local $combined v128)

    local.get $start
    local.set $i

    (block $found_simd
      (loop $simd_loop
        local.get $i
        i32.const 16
        i32.add
        local.get $end
        i32.gt_u
        br_if $found_simd

        local.get $ptr
        local.get $i
        i32.add
        v128.load align=1
        local.set $v

        ;; compare against byte1 (always checked)
        local.get $v
        local.get $byte1
        i8x16.splat
        i8x16.eq
        local.set $combined

        ;; compare against byte2 if non-zero
        local.get $byte2
        i32.eqz
        if
        else
          local.get $combined
          local.get $v
          local.get $byte2
          i8x16.splat
          i8x16.eq
          v128.or
          local.set $combined
        end

        ;; compare against byte3 if non-zero
        local.get $byte3
        i32.eqz
        if
        else
          local.get $combined
          local.get $v
          local.get $byte3
          i8x16.splat
          i8x16.eq
          v128.or
          local.set $combined
        end

        local.get $combined
        v128.any_true
        if
          br $found_simd
        end

        local.get $i
        i32.const 16
        i32.add
        local.set $i
        br $simd_loop
      )
    )

    ;; scalar fallback from position i
    (loop $scalar_loop
      local.get $i
      local.get $end
      i32.ge_u
      if
        i32.const -1
        return
      end

      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      local.set $b

      local.get $b
      local.get $byte1
      i32.eq
      if
        local.get $i
        return
      end

      local.get $byte2
      i32.eqz
      if
      else
        local.get $b
        local.get $byte2
        i32.eq
        if
          local.get $i
          return
        end
      end

      local.get $byte3
      i32.eqz
      if
      else
        local.get $b
        local.get $byte3
        i32.eq
        if
          local.get $i
          return
        end
      end

      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $scalar_loop
    )

    unreachable
  )

  ;; -----------------------------------------------------------
  ;; url_scan_simd: SIMD-accelerated URL scanner
  ;;
  ;; Same as url_scan but uses $simd_find for delimiter scans
  ;; and $url_scheme_validate_simd for scheme validation.
  ;; Auth/port/detailed validation uses original byte-by-byte.
  ;; -----------------------------------------------------------
  (func (export "url_scan_simd")
    (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $start i32)
    (local $end i32)
    (local $i i32)
    (local $b i32)
    (local $colon i32)
    (local $auth_start i32)
    (local $auth_end i32)
    (local $last_colon i32)
    (local $host_len i32)
    (local $port i32)
    (local $digit i32)
    (local $has_port i32)
    (local $path_off i32)
    (local $path_len i32)
    (local $query_off i32)
    (local $query_len i32)
    (local $fragment_off i32)
    (local $fragment_len i32)

    i32.const 0
    local.set $start
    local.get $len
    local.set $end

    (block $trim_start_done
      (loop $trim_start
        local.get $start
        local.get $end
        i32.ge_u
        br_if $trim_start_done
        local.get $ptr
        local.get $start
        i32.add
        i32.load8_u
        call $is_ascii_ws
        i32.eqz
        br_if $trim_start_done
        local.get $start
        i32.const 1
        i32.add
        local.set $start
        br $trim_start))

    (block $trim_end_done
      (loop $trim_end
        local.get $end
        local.get $start
        i32.le_u
        br_if $trim_end_done
        local.get $ptr
        local.get $end
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $is_ascii_ws
        i32.eqz
        br_if $trim_end_done
        local.get $end
        i32.const 1
        i32.sub
        local.set $end
        br $trim_end))

    local.get $start
    local.get $end
    i32.ge_u
    if
      i32.const 3
      return
    end

    ;; --- Scheme colon scan using $simd_find ---
    local.get $ptr
    local.get $start
    local.get $end
    i32.const 58
    i32.const 0
    i32.const 0
    call $simd_find
    local.set $colon

    local.get $colon
    i32.const -1
    i32.eq
    if
      i32.const 3
      return
    end

    local.get $ptr
    local.get $start
    i32.add
    local.get $colon
    local.get $start
    i32.sub
    call $url_scheme_validate_simd
    if
      i32.const 3
      return
    end

    local.get $colon
    i32.const 3
    i32.add
    local.get $end
    i32.gt_u
    if
      i32.const 3
      return
    end
    local.get $ptr
    local.get $colon
    i32.const 1
    i32.add
    i32.add
    i32.load8_u
    i32.const 47
    i32.ne
    if
      i32.const 3
      return
    end
    local.get $ptr
    local.get $colon
    i32.const 2
    i32.add
    i32.add
    i32.load8_u
    i32.const 47
    i32.ne
    if
      i32.const 3
      return
    end

    local.get $colon
    i32.const 3
    i32.add
    local.set $auth_start
    local.get $auth_start
    local.set $auth_end
    i32.const -1
    local.set $last_colon

    ;; --- Auth scan (byte-by-byte, stateful) ---
    (block $auth_done
      (loop $auth_scan
        local.get $auth_end
        local.get $end
        i32.ge_u
        br_if $auth_done
        local.get $ptr
        local.get $auth_end
        i32.add
        i32.load8_u
        local.tee $b
        i32.const 47
        i32.eq
        local.get $b
        i32.const 63
        i32.eq
        i32.or
        local.get $b
        i32.const 35
        i32.eq
        i32.or
        br_if $auth_done
        local.get $b
        i32.const 64
        i32.eq
        local.get $b
        i32.const 91
        i32.eq
        i32.or
        local.get $b
        i32.const 93
        i32.eq
        i32.or
        if
          i32.const 3
          return
        end
        local.get $b
        i32.const 58
        i32.eq
        if
          local.get $auth_end
          local.set $last_colon
        end
        local.get $auth_end
        i32.const 1
        i32.add
        local.set $auth_end
        br $auth_scan))

    local.get $auth_start
    local.get $auth_end
    i32.ge_u
    if
      i32.const 3
      return
    end

    local.get $auth_end
    local.get $auth_start
    i32.sub
    local.set $host_len
    i32.const 0
    local.set $port
    i32.const 0
    local.set $has_port

    local.get $last_colon
    i32.const -1
    i32.ne
    if
      local.get $last_colon
      local.get $auth_start
      i32.eq
      if
        i32.const 3
        return
      end
      local.get $last_colon
      i32.const 1
      i32.add
      local.get $auth_end
      i32.ge_u
      if
        i32.const 3
        return
      end
      local.get $last_colon
      local.get $auth_start
      i32.sub
      local.set $host_len
      i32.const 1
      local.set $has_port
      i32.const 0
      local.set $port
      local.get $last_colon
      i32.const 1
      i32.add
      local.set $i
      (block $port_done
        (loop $port_loop
          local.get $i
          local.get $auth_end
          i32.ge_u
          br_if $port_done
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          local.tee $b
          call $is_digit
          i32.eqz
          if
            i32.const 3
            return
          end
          local.get $b
          i32.const 48
          i32.sub
          local.set $digit
          local.get $port
          i32.const 6553
          i32.gt_u
          if
            i32.const 3
            return
          end
          local.get $port
          i32.const 6553
          i32.eq
          local.get $digit
          i32.const 5
          i32.gt_u
          i32.and
          if
            i32.const 3
            return
          end
          local.get $port
          i32.const 10
          i32.mul
          local.get $digit
          i32.add
          local.set $port
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $port_loop))
    end

    local.get $auth_end
    local.set $i
    local.get $i
    local.set $path_off
    i32.const 0
    local.set $path_len
    local.get $i
    local.set $query_off
    i32.const 0
    local.set $query_len
    local.get $i
    local.set $fragment_off
    i32.const 0
    local.set $fragment_len

    local.get $i
    local.get $end
    i32.lt_u
    if
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      if
        local.get $i
        local.set $path_off
        ;; --- Path scan using $simd_find for '?' or '#' ---
        local.get $ptr
        local.get $i
        local.get $end
        i32.const 63
        i32.const 35
        i32.const 0
        call $simd_find
        local.tee $i
        i32.const -1
        i32.eq
        if
          local.get $end
          local.set $i
        end
        local.get $i
        local.get $path_off
        i32.sub
        local.set $path_len
      end
    end

    local.get $i
    local.get $end
    i32.lt_u
    if
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.const 63
      i32.eq
      if
        local.get $i
        i32.const 1
        i32.add
        local.set $query_off
        local.get $query_off
        local.set $i
        ;; --- Query scan using $simd_find for '#' ---
        local.get $ptr
        local.get $i
        local.get $end
        i32.const 35
        i32.const 0
        i32.const 0
        call $simd_find
        local.tee $i
        i32.const -1
        i32.eq
        if
          local.get $end
          local.set $i
        end
        local.get $i
        local.get $query_off
        i32.sub
        local.set $query_len
      end
    end

    local.get $i
    local.get $end
    i32.lt_u
    if
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.const 35
      i32.eq
      if
        local.get $i
        i32.const 1
        i32.add
        local.set $fragment_off
        local.get $end
        local.get $fragment_off
        i32.sub
        local.set $fragment_len
      else
        i32.const 3
        return
      end
    end

    local.get $out
    local.get $start
    local.get $colon
    local.get $start
    i32.sub
    local.get $auth_start
    local.get $auth_end
    local.get $auth_start
    i32.sub
    local.get $auth_start
    local.get $host_len
    local.get $path_off
    local.get $path_len
    local.get $query_off
    local.get $query_len
    local.get $fragment_off
    local.get $fragment_len
    local.get $port
    local.get $has_port
    call $m189write_record
    i32.const 0)
(data (i32.const 32768) "android.permission.")
  (data (i32.const 32832) "uses-permission")
  (data (i32.const 32896) "package=")
  (data (i32.const 32960) "import android.")
  (data (i32.const 33024) "import java.")
  (data (i32.const 33088) "import javax.")
  (data (i32.const 33152) "landroid/")
  (data (i32.const 33216) "ljava/")
  (data (i32.const 33280) "ljavax/")
  (data (i32.const 33344) "lorg/apache/http/")
  (data (i32.const 33408) "lokhttp3/")
  (data (i32.const 33472) "lretrofit2/")
  (data (i32.const 33536) "okhttpclient")
  (data (i32.const 33600) "request.builder")
  (data (i32.const 33664) ".url(")
  (data (i32.const 33728) ".baseurl(")
  (data (i32.const 33792) "httpurlconnection")
  (data (i32.const 33856) "websocket")
  (data (i32.const 33920) "uri.parse(")
  (data (i32.const 33984) "landroid/net/uri;->parse")
  (data (i32.const 34048) "landroid/webkit/webview;")
  (data (i32.const 34112) "http://")
  (data (i32.const 34176) "https://")
  (data (i32.const 34240) "ws://")
  (data (i32.const 34304) "wss://")
  (data (i32.const 34368) "content://")
  (data (i32.const 34432) "intent://")
  (data (i32.const 34496) "ljavax/crypto/")
  (data (i32.const 34560) "javax.crypto.")
  (data (i32.const 34624) "ljava/security/")
  (data (i32.const 34688) "java.security.")
  (data (i32.const 34752) "keystore")
  (data (i32.const 34816) "cipher")
  (data (i32.const 34880) "signature")
  (data (i32.const 34944) "landroid/location/")
  (data (i32.const 35008) "android.location.")
  (data (i32.const 35072) "location")
  (data (i32.const 35136) "landroid/hardware/camera")
  (data (i32.const 35200) "android.hardware.camera")
  (data (i32.const 35264) "camera")
  (data (i32.const 35328) "landroid/bluetooth/")
  (data (i32.const 35392) "android.bluetooth.")
  (data (i32.const 35456) "bluetooth")
  (data (i32.const 35520) "landroid/telephony/smsmanager;")
  (data (i32.const 35584) "android.telephony.")
  (data (i32.const 35648) "sms")
  (data (i32.const 35712) "landroid/provider/contactscontract;")
  (data (i32.const 35776) "landroid/provider/calendarcontract;")
  (data (i32.const 35840) "contacts")
  (data (i32.const 35904) "calendar")
  (data (i32.const 35968) "ldalvik/system/dexclassloader;")
  (data (i32.const 36032) "ldalvik/system/pathclassloader;")
  (data (i32.const 36096) "ljava/lang/runtime;->exec")
  (data (i32.const 36160) "ljava/lang/processbuilder;")
  (data (i32.const 36224) "addjavascriptinterface")
  (data (i32.const 36288) "graphql")
  (data (i32.const 36352) "analytics")
  (data (i32.const 36416) "token")
  (data (i32.const 36480) "auth")
  (data (i32.const 36544) "login")

  ;; Status: 0 ok, 2 no finding, 3 unsupported/invalid.
  ;; Finding record, 20 bytes:
  ;;   u32 kind, u32 start_byte, u32 end_byte, u32 flags, u32 status.
  ;; Kinds: 1 package, 2 permission, 3 platform_api, 4 import_api, 5 uri,
  ;;   6 network_web, 7 crypto_security, 8 location, 9 camera_media,
  ;;   10 bluetooth_nearby, 11 sms_telephony, 12 contacts_calendar,
  ;;   13 dynamic_process, 14 javascript_bridge, 15 endpoint_string.
  ;; Flags: bit0 matched inside a quoted string, bit1 Android/platform API,
  ;;   bit2 network/URI, bit3 sensitive capability, bit4 manifest-ish.

  (import "edgerun" "load8_u" (func $m31ch (param i32 i32) (result i32)))
  (import "edgerun" "to_lower" (func $m31lower (param i32) (result i32)))

  (func $packed_kind (param $v i64) (result i32)
    local.get $v
    i64.const 32
    i64.shr_u
    i32.wrap_i64)

  (func $packed_len (param $v i64) (result i32)
    local.get $v
    i32.wrap_i64)

  (func $m31match_lit (param $ptr i32) (param $len i32) (param $pos i32) (param $pat i32) (param $plen i32) (result i32)
    (local $j i32)
    local.get $pos
    local.get $plen
    i32.add
    local.get $len
    i32.gt_u
    if
      i32.const 0
      return
    end
    i32.const 0
    local.set $j
    block $no
      loop $scan
        local.get $j
        local.get $plen
        i32.ge_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        local.get $pos
        local.get $j
        i32.add
        call $m31ch
        call $m31lower
        local.get $pat
        local.get $j
        i32.add
        i32.load8_u
        i32.ne
        br_if $no
        local.get $j
        i32.const 1
        i32.add
        local.set $j
        br $scan
      end
    end
    i32.const 0)

  (func $classify_string (param $ptr i32) (param $len i32) (param $pos i32) (result i64)
    local.get $ptr local.get $len local.get $pos i32.const 32768 i32.const 19 call $m31match_lit
    if i32.const 2 i32.const 19 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 32832 i32.const 15 call $m31match_lit
    if i32.const 2 i32.const 15 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34112 i32.const 7 call $m31match_lit
    if i32.const 5 i32.const 7 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34176 i32.const 8 call $m31match_lit
    if i32.const 5 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34240 i32.const 5 call $m31match_lit
    if i32.const 5 i32.const 5 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34304 i32.const 6 call $m31match_lit
    if i32.const 5 i32.const 6 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34368 i32.const 10 call $m31match_lit
    if i32.const 5 i32.const 10 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34432 i32.const 9 call $m31match_lit
    if i32.const 5 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36288 i32.const 7 call $m31match_lit
    if i32.const 15 i32.const 7 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36352 i32.const 9 call $m31match_lit
    if i32.const 15 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36416 i32.const 5 call $m31match_lit
    if i32.const 15 i32.const 5 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36480 i32.const 4 call $m31match_lit
    if i32.const 15 i32.const 4 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36544 i32.const 5 call $m31match_lit
    if i32.const 15 i32.const 5 call $pack return end
    i64.const 0)

  (func $classify_code (param $ptr i32) (param $len i32) (param $pos i32) (result i64)
    local.get $ptr local.get $len local.get $pos i32.const 32896 i32.const 8 call $m31match_lit
    if i32.const 1 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 32768 i32.const 19 call $m31match_lit
    if i32.const 2 i32.const 19 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 32832 i32.const 15 call $m31match_lit
    if i32.const 2 i32.const 15 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 32960 i32.const 15 call $m31match_lit
    if i32.const 4 i32.const 15 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33024 i32.const 12 call $m31match_lit
    if i32.const 4 i32.const 12 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33088 i32.const 13 call $m31match_lit
    if i32.const 4 i32.const 13 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33408 i32.const 9 call $m31match_lit
    if i32.const 6 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33472 i32.const 11 call $m31match_lit
    if i32.const 6 i32.const 11 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33536 i32.const 12 call $m31match_lit
    if i32.const 6 i32.const 12 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33600 i32.const 15 call $m31match_lit
    if i32.const 6 i32.const 15 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33664 i32.const 5 call $m31match_lit
    if i32.const 6 i32.const 5 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33728 i32.const 9 call $m31match_lit
    if i32.const 6 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33792 i32.const 17 call $m31match_lit
    if i32.const 6 i32.const 17 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33856 i32.const 9 call $m31match_lit
    if i32.const 6 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33920 i32.const 10 call $m31match_lit
    if i32.const 6 i32.const 10 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33984 i32.const 24 call $m31match_lit
    if i32.const 6 i32.const 24 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34048 i32.const 23 call $m31match_lit
    if i32.const 6 i32.const 23 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34112 i32.const 7 call $m31match_lit
    if i32.const 5 i32.const 7 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34176 i32.const 8 call $m31match_lit
    if i32.const 5 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34240 i32.const 5 call $m31match_lit
    if i32.const 5 i32.const 5 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34304 i32.const 6 call $m31match_lit
    if i32.const 5 i32.const 6 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34368 i32.const 10 call $m31match_lit
    if i32.const 5 i32.const 10 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34432 i32.const 9 call $m31match_lit
    if i32.const 5 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34496 i32.const 14 call $m31match_lit
    if i32.const 7 i32.const 14 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34560 i32.const 13 call $m31match_lit
    if i32.const 7 i32.const 13 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34624 i32.const 15 call $m31match_lit
    if i32.const 7 i32.const 15 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34688 i32.const 14 call $m31match_lit
    if i32.const 7 i32.const 14 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34752 i32.const 8 call $m31match_lit
    if i32.const 7 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34816 i32.const 6 call $m31match_lit
    if i32.const 7 i32.const 6 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34880 i32.const 9 call $m31match_lit
    if i32.const 7 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 34944 i32.const 18 call $m31match_lit
    if i32.const 8 i32.const 18 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35008 i32.const 17 call $m31match_lit
    if i32.const 8 i32.const 17 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35072 i32.const 8 call $m31match_lit
    if i32.const 8 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35136 i32.const 24 call $m31match_lit
    if i32.const 9 i32.const 24 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35200 i32.const 23 call $m31match_lit
    if i32.const 9 i32.const 23 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35264 i32.const 6 call $m31match_lit
    if i32.const 9 i32.const 6 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35328 i32.const 19 call $m31match_lit
    if i32.const 10 i32.const 19 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35392 i32.const 18 call $m31match_lit
    if i32.const 10 i32.const 18 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35456 i32.const 9 call $m31match_lit
    if i32.const 10 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35520 i32.const 30 call $m31match_lit
    if i32.const 11 i32.const 30 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35584 i32.const 18 call $m31match_lit
    if i32.const 11 i32.const 18 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35648 i32.const 3 call $m31match_lit
    if i32.const 11 i32.const 3 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35712 i32.const 35 call $m31match_lit
    if i32.const 12 i32.const 35 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35776 i32.const 35 call $m31match_lit
    if i32.const 12 i32.const 35 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35840 i32.const 8 call $m31match_lit
    if i32.const 12 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35904 i32.const 8 call $m31match_lit
    if i32.const 12 i32.const 8 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 35968 i32.const 29 call $m31match_lit
    if i32.const 13 i32.const 29 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36032 i32.const 30 call $m31match_lit
    if i32.const 13 i32.const 30 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36096 i32.const 26 call $m31match_lit
    if i32.const 13 i32.const 26 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36160 i32.const 26 call $m31match_lit
    if i32.const 13 i32.const 26 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 36224 i32.const 22 call $m31match_lit
    if i32.const 14 i32.const 22 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33152 i32.const 9 call $m31match_lit
    if i32.const 3 i32.const 9 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33216 i32.const 6 call $m31match_lit
    if i32.const 3 i32.const 6 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33280 i32.const 7 call $m31match_lit
    if i32.const 3 i32.const 7 call $pack return end
    local.get $ptr local.get $len local.get $pos i32.const 33344 i32.const 16 call $m31match_lit
    if i32.const 3 i32.const 16 call $pack return end
    i64.const 0)

  (func $flags_for (param $kind i32) (param $in_string i32) (result i32)
    (local $flags i32)
    local.get $in_string
    local.set $flags
    local.get $kind i32.const 3 i32.eq
    local.get $kind i32.const 4 i32.eq i32.or
    if
      local.get $flags i32.const 2 i32.or local.set $flags
    end
    local.get $kind i32.const 5 i32.eq
    local.get $kind i32.const 6 i32.eq i32.or
    if
      local.get $flags i32.const 4 i32.or local.set $flags
    end
    local.get $kind i32.const 7 i32.eq
    local.get $kind i32.const 8 i32.eq i32.or
    local.get $kind i32.const 9 i32.eq i32.or
    local.get $kind i32.const 10 i32.eq i32.or
    local.get $kind i32.const 11 i32.eq i32.or
    local.get $kind i32.const 12 i32.eq i32.or
    local.get $kind i32.const 13 i32.eq i32.or
    local.get $kind i32.const 14 i32.eq i32.or
    if
      local.get $flags i32.const 8 i32.or local.set $flags
    end
    local.get $kind i32.const 1 i32.eq
    local.get $kind i32.const 2 i32.eq i32.or
    if
      local.get $flags i32.const 16 i32.or local.set $flags
    end
    local.get $flags)

  (func $m31write_out (param $out i32) (param $kind i32) (param $start i32) (param $end i32) (param $flags i32) (param $status i32)
    local.get $out local.get $kind i32.store align=1
    local.get $out i32.const 4 i32.add local.get $start i32.store align=1
    local.get $out i32.const 8 i32.add local.get $end i32.store align=1
    local.get $out i32.const 12 i32.add local.get $flags i32.store align=1
    local.get $out i32.const 16 i32.add local.get $status i32.store align=1)

  (func $state_until (param $ptr i32) (param $len i32) (param $limit i32) (result i64)
    (local $i i32) (local $c i32) (local $next i32) (local $state i32) (local $quote i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $state
    i32.const 0 local.set $quote
    block $done
      loop $scan
        local.get $i local.get $limit i32.ge_u br_if $done
        local.get $i local.get $len i32.ge_u br_if $done
        local.get $ptr local.get $i call $m31ch local.set $c
        local.get $state i32.eqz
        if
          local.get $i i32.const 1 i32.add local.get $limit i32.lt_u
          local.get $i i32.const 1 i32.add local.get $len i32.lt_u i32.and
          if
            local.get $ptr local.get $i i32.const 1 i32.add call $m31ch local.set $next
            local.get $c i32.const 47 i32.eq
            local.get $next i32.const 47 i32.eq i32.and
            if
              i32.const 1 local.set $state
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
            local.get $c i32.const 47 i32.eq
            local.get $next i32.const 42 i32.eq i32.and
            if
              i32.const 2 local.set $state
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
          end
          local.get $c i32.const 34 i32.eq
          local.get $c i32.const 39 i32.eq i32.or
          local.get $c i32.const 96 i32.eq i32.or
          if
            i32.const 3 local.set $state
            local.get $c local.set $quote
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
        else
          local.get $state i32.const 1 i32.eq
          if
            local.get $c i32.const 10 i32.eq
            local.get $c i32.const 13 i32.eq i32.or
            if i32.const 0 local.set $state end
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $state i32.const 2 i32.eq
          if
            local.get $i i32.const 1 i32.add local.get $limit i32.lt_u
            local.get $i i32.const 1 i32.add local.get $len i32.lt_u i32.and
            if
              local.get $c i32.const 42 i32.eq
              local.get $ptr local.get $i i32.const 1 i32.add call $m31ch i32.const 47 i32.eq
              i32.and
              if
                i32.const 0 local.set $state
                local.get $i i32.const 2 i32.add local.set $i
                br $scan
              end
            end
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $state i32.const 3 i32.eq
          if
            local.get $c i32.const 92 i32.eq
            if
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
            local.get $c local.get $quote i32.eq
            if
              i32.const 0 local.set $state
              i32.const 0 local.set $quote
            end
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $scan
      end
    end
    local.get $state i64.extend_i32_u i64.const 32 i64.shl
    local.get $quote i64.extend_i32_u i64.or)

  (func $apk_api_scan_next (export "apk_api_scan_next") (param $ptr i32) (param $len i32) (param $start i32) (param $out i32) (result i32)
    (local $i i32) (local $c i32) (local $next i32) (local $state i32) (local $quote i32)
    (local $hit i64) (local $kind i32) (local $plen i32) (local $in_string i32) (local $packed_state i64)
    local.get $ptr local.get $len i32.add i32.const 65500 i32.gt_u
    local.get $out i32.const 20 i32.add i32.const 65500 i32.gt_u i32.or
    local.get $start local.get $len i32.gt_u i32.or
    if
      local.get $out i32.const 0 i32.const 0 i32.const 0 i32.const 0 i32.const 3 call $m31write_out
      i32.const 3
      return
    end
    local.get $ptr local.get $len local.get $start call $state_until local.set $packed_state
    local.get $packed_state i64.const 32 i64.shr_u i32.wrap_i64 local.set $state
    local.get $packed_state i32.wrap_i64 local.set $quote
    local.get $start local.set $i
    block $done
      loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr local.get $i call $m31ch
        local.set $c
        local.get $state
        i32.eqz
        if
          local.get $i i32.const 1 i32.add local.set $next
          local.get $i i32.const 1 i32.add local.get $len i32.lt_u
          if
            local.get $ptr local.get $i i32.const 1 i32.add call $m31ch
            local.set $next
            local.get $c i32.const 47 i32.eq
            local.get $next i32.const 47 i32.eq i32.and
            if
              i32.const 1 local.set $state
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
            local.get $c i32.const 47 i32.eq
            local.get $next i32.const 42 i32.eq i32.and
            if
              i32.const 2 local.set $state
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
          end
          local.get $c i32.const 34 i32.eq
          local.get $c i32.const 39 i32.eq i32.or
          local.get $c i32.const 96 i32.eq i32.or
          if
            i32.const 3 local.set $state
            local.get $c local.set $quote
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $ptr local.get $len local.get $i call $classify_code local.set $hit
          i32.const 0 local.set $in_string
        else
          local.get $state i32.const 1 i32.eq
          if
            local.get $c i32.const 10 i32.eq
            local.get $c i32.const 13 i32.eq i32.or
            if i32.const 0 local.set $state end
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $state i32.const 2 i32.eq
          if
            local.get $i i32.const 1 i32.add local.get $len i32.lt_u
            if
              local.get $c i32.const 42 i32.eq
              local.get $ptr local.get $i i32.const 1 i32.add call $m31ch i32.const 47 i32.eq
              i32.and
              if
                i32.const 0 local.set $state
                local.get $i i32.const 2 i32.add local.set $i
                br $scan
              end
            end
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $state i32.const 3 i32.eq
          if
            local.get $c i32.const 92 i32.eq
            if
              local.get $i i32.const 2 i32.add local.set $i
              br $scan
            end
            local.get $c local.get $quote i32.eq
            if
              i32.const 0 local.set $state
              local.get $i i32.const 1 i32.add local.set $i
              br $scan
            end
            local.get $ptr local.get $len local.get $i call $classify_string local.set $hit
            i32.const 1 local.set $in_string
          end
        end
        local.get $hit i64.eqz
        i32.eqz
        if
          local.get $hit call $packed_kind local.set $kind
          local.get $hit call $packed_len local.set $plen
          local.get $out
          local.get $kind
          local.get $i
          local.get $i local.get $plen i32.add
          local.get $kind local.get $in_string call $flags_for
          i32.const 0
          call $m31write_out
          i32.const 0
          return
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $scan
      end
    end
    local.get $out i32.const 0 i32.const 0 i32.const 0 i32.const 0 i32.const 2 call $m31write_out
    i32.const 2)

  (func (export "apk_api_scan_count") (param $ptr i32) (param $len i32) (result i64)
    (local $pos i32) (local $count i32) (local $status i32) (local $out i32) (local $end i32)
    i32.const 65472
    local.set $out
    local.get $ptr local.get $len i32.add i32.const 65500 i32.gt_u
    if i64.const 3 return end
    i32.const 0 local.set $pos
    i32.const 0 local.set $count
    block $done
      loop $loop
        local.get $ptr local.get $len local.get $pos local.get $out call $apk_api_scan_next
        local.tee $status
        i32.const 2
        i32.eq
        br_if $done
        local.get $status
        if
          local.get $status i64.extend_i32_u
          return
        end
        local.get $count i32.const 1 i32.add local.set $count
        local.get $out i32.const 8 i32.add i32.load align=1 local.set $end
        local.get $end local.get $pos i32.le_u
        if
          local.get $pos i32.const 1 i32.add local.set $pos
        else
          local.get $end local.set $pos
        end
        local.get $pos local.get $len i32.ge_u br_if $done
        br $loop
      end
    end
    local.get $count
    i64.extend_i32_u
    i64.const 32
    i64.shl)
)

  (data (i32.const 8192)
    "\00\00\00\00\00\00\00\00\00\04\04\00\00\04\00\00"
    "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00"
    "\04\00\00\00\00\00\00\00\00\00\00\01\00\01\01\00"
    "\03\03\03\03\03\03\03\03\03\03\00\00\00\00\00\00"
    "\00\01\01\01\01\01\01\01\01\01\01\01\01\01\01\01"
    "\01\01\01\01\01\01\01\01\01\01\01\00\00\00\00\00"
    "\00\01\01\01\01\01\01\01\01\01\01\01\01\01\01\01"
    "\01\01\01\01\01\01\01\01\01\01\01\00\00\00\00\00"
    "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00"
    "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00"
    "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00"
    "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00"
    "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00"
    "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00"
    "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00"
    "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00"
  )
)
