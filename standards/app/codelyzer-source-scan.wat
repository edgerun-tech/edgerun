(func (export "proto_standard_id") (result i32)
    i32.const 300089)

  ;; Status: 0 ok, 2 no function, 3 unsupported/invalid.
  ;; Languages: 1 rust, 2 ts/js, 3 c, 4 py, 5 go, 6 java, 0 unknown.
  ;; First function out record, 24 bytes:
  ;;   u32 name_start, u32 name_len, u32 start_byte, u32 end_byte, u32 is_static, u32 lang.

  (import "edgerun" "load8_u" (func $m44ch (param i32 i32) (result i32)))

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
