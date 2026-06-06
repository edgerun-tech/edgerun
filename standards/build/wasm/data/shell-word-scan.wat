(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))


  (func (export "proto_standard_id") (result i32)
    i32.const 300070)


  (func $m164is_wsp (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.ge_u
    local.get $b
    i32.const 13
    i32.le_u
    i32.and
    i32.or)

  (func $m164write_record
    (param $rec i32) (param $next i32) (param $source_start i32)
    (param $source_len i32) (param $flags i32)
    local.get $rec
    local.get $next
    i32.store
    local.get $rec
    i32.const 4
    i32.add
    local.get $source_start
    i32.store
    local.get $rec
    i32.const 8
    i32.add
    local.get $source_len
    i32.store
    local.get $rec
    i32.const 12
    i32.add
    local.get $flags
    i32.store)

  (func $m164emit (param $out i32) (param $cap i32) (param $written i32) (param $b i32) (result i32)
    local.get $written
    local.get $cap
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $written
    i32.add
    local.get $b
    i32.store8
    local.get $written
    i32.const 1
    i32.add)

  ;; Decode one shell-like word from input bytes.
  ;;
  ;; Return: packed low u32 status, high u32 decoded byte count.
  ;; Status: 0 ok, 2 output cap too small, 3 unterminated quote, 5 eof.
  ;; Record at rec_ptr: next_offset, source_start, source_len, flags.
  ;; Flags: bit 0 saw single quotes, bit 1 saw double quotes, bit 2 saw escape.
  ;;
  ;; The scanner preserves the deleted Rust split() behavior for byte-oriented
  ;; shell words: ASCII whitespace separates words; single quotes copy literally
  ;; until the next single quote; double quotes only let backslash escape
  ;; $, `, ", \, and LF; outside quotes backslash copies the next byte or itself
  ;; at EOF. Comments are not special because the Rust crate did not implement
  ;; comment handling.
  (func (export "shell_word_next")
    (param $ptr i32) (param $len i32) (param $offset i32)
    (param $out i32) (param $out_cap i32) (param $rec i32)
    (result i64)
    (local $i i32)
    (local $b i32)
    (local $next_b i32)
    (local $source_start i32)
    (local $written i32)
    (local $flags i32)

    local.get $offset
    local.get $len
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $offset
    local.set $i
    (block $found
      (loop $skip
        local.get $i
        local.get $len
        i32.ge_u
        if
          local.get $rec
          local.get $len
          local.get $len
          i32.const 0
          i32.const 0
          call $m164write_record
          i32.const 5
          i32.const 0
          call $pack
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $m164is_wsp
        i32.eqz
        br_if $found
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $skip))

    local.get $i
    local.set $source_start
    i32.const 0
    local.set $written
    i32.const 0
    local.set $flags

    (block $word_done
      (loop $word
        local.get $i
        local.get $len
        i32.ge_u
        br_if $word_done

        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b

        local.get $b
        call $m164is_wsp
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $word_done
        end

        ;; Single quote.
        local.get $b
        i32.const 39
        i32.eq
        if
          local.get $flags
          i32.const 1
          i32.or
          local.set $flags
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          (block $single_done
            (loop $single
              local.get $i
              local.get $len
              i32.ge_u
              if
                local.get $rec
                local.get $i
                local.get $source_start
                local.get $i
                local.get $source_start
                i32.sub
                local.get $flags
                call $m164write_record
                i32.const 3
                local.get $written
                call $pack
                return
              end
              local.get $ptr
              local.get $i
              i32.add
              i32.load8_u
              local.set $b
              local.get $b
              i32.const 39
              i32.eq
              if
                local.get $i
                i32.const 1
                i32.add
                local.set $i
                br $single_done
              end
              local.get $out
              local.get $out_cap
              local.get $written
              local.get $b
              call $m164emit
              local.tee $written
              i32.const -1
              i32.eq
              if
                local.get $rec
                local.get $i
                local.get $source_start
                local.get $i
                local.get $source_start
                i32.sub
                local.get $flags
                call $m164write_record
                i32.const 2
                i32.const 0
                call $pack
                return
              end
              local.get $i
              i32.const 1
              i32.add
              local.set $i
              br $single))
          br $word
        end

        ;; Double quote.
        local.get $b
        i32.const 34
        i32.eq
        if
          local.get $flags
          i32.const 2
          i32.or
          local.set $flags
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          (block $double_done
            (loop $double
              local.get $i
              local.get $len
              i32.ge_u
              if
                local.get $rec
                local.get $i
                local.get $source_start
                local.get $i
                local.get $source_start
                i32.sub
                local.get $flags
                call $m164write_record
                i32.const 3
                local.get $written
                call $pack
                return
              end
              local.get $ptr
              local.get $i
              i32.add
              i32.load8_u
              local.set $b
              local.get $b
              i32.const 34
              i32.eq
              if
                local.get $i
                i32.const 1
                i32.add
                local.set $i
                br $double_done
              end

              local.get $b
              i32.const 92
              i32.eq
              if
                local.get $flags
                i32.const 4
                i32.or
                local.set $flags
                local.get $i
                i32.const 1
                i32.add
                local.get $len
                i32.lt_u
                if
                  local.get $ptr
                  local.get $i
                  i32.const 1
                  i32.add
                  i32.add
                  i32.load8_u
                  local.set $next_b
                  local.get $next_b
                  i32.const 36
                  i32.eq
                  local.get $next_b
                  i32.const 96
                  i32.eq
                  i32.or
                  local.get $next_b
                  i32.const 34
                  i32.eq
                  i32.or
                  local.get $next_b
                  i32.const 92
                  i32.eq
                  i32.or
                  local.get $next_b
                  i32.const 10
                  i32.eq
                  i32.or
                  if
                    local.get $next_b
                    local.set $b
                    local.get $i
                    i32.const 1
                    i32.add
                    local.set $i
                  end
                end
              end

              local.get $out
              local.get $out_cap
              local.get $written
              local.get $b
              call $m164emit
              local.tee $written
              i32.const -1
              i32.eq
              if
                local.get $rec
                local.get $i
                local.get $source_start
                local.get $i
                local.get $source_start
                i32.sub
                local.get $flags
                call $m164write_record
                i32.const 2
                i32.const 0
                call $pack
                return
              end
              local.get $i
              i32.const 1
              i32.add
              local.set $i
              br $double))
          br $word
        end

        ;; Backslash outside quotes.
        local.get $b
        i32.const 92
        i32.eq
        if
          local.get $flags
          i32.const 4
          i32.or
          local.set $flags
          local.get $i
          i32.const 1
          i32.add
          local.get $len
          i32.lt_u
          if
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            local.get $ptr
            local.get $i
            i32.add
            i32.load8_u
            local.set $b
          end
        end

        local.get $out
        local.get $out_cap
        local.get $written
        local.get $b
        call $m164emit
        local.tee $written
        i32.const -1
        i32.eq
        if
          local.get $rec
          local.get $i
          local.get $source_start
          local.get $i
          local.get $source_start
          i32.sub
          local.get $flags
          call $m164write_record
          i32.const 2
          i32.const 0
          call $pack
          return
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $word))

    local.get $rec
    local.get $i
    local.get $source_start
    local.get $i
    local.get $source_start
    i32.sub
    local.get $flags
    call $m164write_record
    i32.const 0
    local.get $written
    call $pack)

)