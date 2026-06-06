(module
  (import "edgerun-core" "memory" (memory 1))
;; Chrome DevTools Protocol (CDP) message builder — JSON-RPC over WebSocket.
  ;; Uses mutable globals $g_out/$g_o shared across helper functions.
  (global $g_out (mut i32) (i32.const 0))
  (global $g_o (mut i32) (i32.const 0))

  (func (export "proto_standard_id") (result i32)
    i32.const 300503)

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
)
