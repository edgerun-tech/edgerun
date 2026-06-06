
  ;; Jagex string escape/unescape.
  ;; Escape: < -> <lt>, > -> <gt>, \n -> <br>
  ;; Unescape: <lt> -> <, <gt> -> >, <br> -> \n
  ;;
  ;; Exports:
  ;;   escape_text(in_ptr, in_len, out_ptr) -> out_len
  ;;   unescape_text(in_ptr, in_len, out_ptr) -> out_len
  (func (export "escape_text") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $c i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $o
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.set $c
      local.get $c i32.const 60 i32.eq  ;; '<'
      if
        local.get $out local.get $o i32.add i32.const 60 i32.store8  ;; '<'
        local.get $out local.get $o i32.const 1 i32.add i32.add i32.const 108 i32.store8  ;; 'l'
        local.get $out local.get $o i32.const 2 i32.add i32.add i32.const 116 i32.store8  ;; 't'
        local.get $out local.get $o i32.const 3 i32.add i32.add i32.const 62 i32.store8  ;; '>'
        local.get $o i32.const 4 i32.add local.set $o
      else
        local.get $c i32.const 62 i32.eq  ;; '>'
        if
          local.get $out local.get $o i32.add i32.const 60 i32.store8
          local.get $out local.get $o i32.const 1 i32.add i32.add i32.const 103 i32.store8  ;; 'g'
          local.get $out local.get $o i32.const 2 i32.add i32.add i32.const 116 i32.store8  ;; 't'
          local.get $out local.get $o i32.const 3 i32.add i32.add i32.const 62 i32.store8  ;; '>'
          local.get $o i32.const 4 i32.add local.set $o
        else
          local.get $c i32.const 10 i32.eq  ;; '\n'
          if
            local.get $out local.get $o i32.add i32.const 60 i32.store8
            local.get $out local.get $o i32.const 1 i32.add i32.add i32.const 98 i32.store8  ;; 'b'
            local.get $out local.get $o i32.const 2 i32.add i32.add i32.const 114 i32.store8  ;; 'r'
            local.get $out local.get $o i32.const 3 i32.add i32.add i32.const 62 i32.store8  ;; '>'
            local.get $o i32.const 4 i32.add local.set $o
          else
            local.get $c i32.const 13 i32.ne  ;; skip '\r'
            if
              local.get $out local.get $o i32.add local.get $c i32.store8
              local.get $o i32.const 1 i32.add local.set $o
            end
          end
        end
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $o
  )
  (func (export "unescape_text") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $c i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $o
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.set $c
      local.get $c i32.const 60 i32.eq  ;; '<'
      if
        local.get $i i32.const 4 i32.add local.get $len i32.le_u
        if
          local.get $in local.get $i i32.add i32.load
          local.tee $c
          i32.const 0x3E746C3C i32.eq  ;; "<lt>"
          if
            local.get $out local.get $o i32.add i32.const 60 i32.store8
            local.get $o i32.const 1 i32.add local.set $o
            local.get $i i32.const 4 i32.add local.set $i
            br $loop
          end
          local.get $c i32.const 0x3E74673C i32.eq  ;; "<gt>"
          if
            local.get $out local.get $o i32.add i32.const 62 i32.store8
            local.get $o i32.const 1 i32.add local.set $o
            local.get $i i32.const 4 i32.add local.set $i
            br $loop
          end
          local.get $c i32.const 0x3E72623C i32.eq  ;; "<br>"
          if
            local.get $out local.get $o i32.add i32.const 10 i32.store8
            local.get $o i32.const 1 i32.add local.set $o
            local.get $i i32.const 4 i32.add local.set $i
            br $loop
          end
        end
      end
      local.get $out local.get $o i32.add local.get $c i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $o
  )
