(module
  ;; Browser-core Rust semantics captured from browser-wire, browser-authoring,
  ;; browser-runtime, browser-host, browser-core-slice, and browser-work mirrors.
  ;;
  ;; Decisions: run-once=1 verify-cache=2 cancel=3.
  ;; Capability status: ok=0 denied=1 invalid=2 provider-failed=3.
  ;; Host status: ok=0 invalid-input=1 runtime-error=2.

  (func $bool (param $x i32) (result i32)
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
    call $bool
    if (result i32)
      i32.const 1
    else
      local.get $ends_edm
      call $bool
      if (result i32)
        i32.const 2
      else
        local.get $ends_wasm
        call $bool
        if (result i32)
          i32.const 3
        else
          local.get $ends_native_lib
          call $bool
          if (result i32)
            i32.const 4
          else
            local.get $ends_web_asset
            call $bool
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
    call $bool
    i32.eqz
    i32.and
    local.get $contains_backslash
    call $bool
    i32.eqz
    i32.and
    local.get $has_dot_segment
    call $bool
    i32.eqz
    i32.and
    local.get $has_empty_segment
    call $bool
    i32.eqz
    i32.and
    local.get $reserved_name
    call $bool
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
    call $bool
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $slug_matches
      call $bool
      i32.eqz
      if (result i32)
        i32.const 2
      else
        local.get $developer_matches
        call $bool
        i32.eqz
        if (result i32)
          i32.const 3
        else
          local.get $app_id_matches
          call $bool
          i32.eqz
          if (result i32)
            i32.const 4
          else
            local.get $binds_app_edapp
            call $bool
            i32.eqz
            if (result i32)
              i32.const 5
            else
              local.get $runtime_projection_matches
              call $bool
              i32.eqz
              if (result i32)
                i32.const 6
              else
                local.get $signature_ok
                call $bool
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
      call $bool
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
          call $bool
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
    call $bool
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $decision
      i32.const 2
      i32.eq
      if (result i32)
        local.get $cache_present
        call $bool
        i32.eqz
        if (result i32)
          i32.const 2
        else
          local.get $cache_matches
          call $bool
          if (result i32)
            i32.const 0
          else
            i32.const 4
          end
        end
      else
        local.get $cache_present
        call $bool
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
    call $bool
    local.get $release_matches
    call $bool
    i32.and
    local.get $valid_range
    call $bool
    i32.and
    local.get $grant_id_matches
    call $bool
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
    call $bool
    local.get $app_release_scope_matches
    call $bool
    i32.and
    local.get $kind_matches_storage
    call $bool
    i32.and
    local.get $route_live
    call $bool
    i32.and
    local.get $binding_id_matches
    call $bool
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
    call $bool
    local.get $app_release_matches
    call $bool
    i32.and
    local.get $status_open
    call $bool
    i32.and
    local.get $ids_nonzero
    call $bool
    i32.and
    local.get $session_id_matches
    call $bool
    i32.and
    local.get $binding_matches
    call $bool
    i32.and
    local.get $route_live
    call $bool
    i32.and
    local.get $valid_until_ok
    call $bool
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
    call $bool
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
    call $bool
    local.get $binding_matches_namespace
    call $bool
    i32.and
    local.get $grant_allows_operation
    call $bool
    i32.and
    local.get $assurance_ok
    call $bool
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
    call $bool
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $operation
      i32.const 7
      i32.eq
      if (result i32)
        local.get $payload_hash_matches
        call $bool
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
          call $bool
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
    call $bool
    i32.and
    local.get $payload_hash_matches
    call $bool
    i32.and
    local.get $event_hash_matches
    call $bool
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
    call $bool
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
)
