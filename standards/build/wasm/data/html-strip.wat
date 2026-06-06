;; HTML/XML tag stripping — removes <...> tags from text.
  ;;
  ;; Exports:
  ;;   strip_tags(in_ptr, in_len, out_ptr) -> out_len
  ;;   strip_formatting_tags(in_ptr, in_len, out_ptr) -> out_len
  ;;     (preserves <lt>, <gt>, <br>)
  (func (export "proto_abi_version") (result i32) i32.const 2)
  (func (export "proto_standard_id") (result i32) i32.const 300529)

  (func (export "strip_tags") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $c i32) (local $in_tag i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $o
    i32.const 0 local.set $in_tag
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.set $c
      local.get $c i32.const 60 i32.eq  ;; '<'
      if
        i32.const 1 local.set $in_tag
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $c i32.const 62 i32.eq  ;; '>'
      if
        i32.const 0 local.set $in_tag
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $in_tag if
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $out local.get $o i32.add local.get $c i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $o
  )

  (func (export "strip_formatting_tags") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $c i32) (local $in_tag i32)
    (local $tag_start i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $o
    i32.const 0 local.set $in_tag
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.set $c
      local.get $c i32.const 60 i32.eq
      if
        i32.const 1 local.set $in_tag
        local.get $i local.set $tag_start
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $c i32.const 62 i32.eq
      if
        i32.const 0 local.set $in_tag
        ;; Check if it's a preserved tag: <lt>, <gt>, <br>
        local.get $in local.get $tag_start i32.add
        local.tee $c
        i32.load8_u i32.const 60 i32.ne
        if
          local.get $i i32.const 1 i32.add local.set $i
          br $loop
        end
        local.get $c i32.const 1 i32.add i32.load8_u i32.const 108 i32.eq  ;; 'l'
        if
          local.get $c i32.const 2 i32.add i32.load8_u i32.const 116 i32.eq  ;; 't'
          if
            local.get $c i32.const 3 i32.add i32.load8_u i32.const 62 i32.eq  ;; '>'
            if
              local.get $out local.get $o i32.add i32.const 60 i32.store8  ;; '<'
              local.get $o i32.const 1 i32.add local.set $o
              local.get $out local.get $o i32.add i32.const 108 i32.store8  ;; 'l'
              local.get $o i32.const 1 i32.add local.set $o
              local.get $out local.get $o i32.add i32.const 116 i32.store8  ;; 't'
              local.get $o i32.const 1 i32.add local.set $o
            end
          end
        else
          local.get $c i32.const 1 i32.add i32.load8_u i32.const 103 i32.eq  ;; 'g'
          if
            local.get $c i32.const 2 i32.add i32.load8_u i32.const 116 i32.eq  ;; 't'
            if
              local.get $c i32.const 3 i32.add i32.load8_u i32.const 62 i32.eq  ;; '>'
              if
                local.get $out local.get $o i32.add i32.const 60 i32.store8
                local.get $o i32.const 1 i32.add local.set $o
                local.get $out local.get $o i32.add i32.const 103 i32.store8
                local.get $o i32.const 1 i32.add local.set $o
                local.get $out local.get $o i32.add i32.const 116 i32.store8
                local.get $o i32.const 1 i32.add local.set $o
              end
            end
          else
            local.get $c i32.const 1 i32.add i32.load8_u i32.const 98 i32.eq  ;; 'b'
            if
              local.get $c i32.const 2 i32.add i32.load8_u i32.const 114 i32.eq  ;; 'r'
              if
                local.get $c i32.const 3 i32.add i32.load8_u i32.const 62 i32.eq  ;; '>'
                if
                  local.get $out local.get $o i32.add i32.const 10 i32.store8  ;; '\n'
                  local.get $o i32.const 1 i32.add local.set $o
                end
              end
            end
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $in_tag if
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $out local.get $o i32.add local.get $c i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $o
  )