  ;; JS5 multi-file archive splitter.
  ;;
  ;; Parses a decompressed JS5 archive payload with its entry table (last byte =
  ;; chunk count, preceding bytes = chunk_count × file_count big-endian i32
  ;; deltas).  Writes per-file entries (ptr, len) into a caller-provided buffer
  ;; entry layout: [ptr:i32, len:i32, write_offset:i32] = 12 bytes per entry.
  ;;
  ;; For single-file or single-chunk multi-file archives, entries point directly
  ;; into the payload.  For multi-chunk archives, bytes are de-interleaved into
  ;; the caller-provided arena.
  ;;
  ;; Delta encoding: within each chunk, raw deltas are stored as DIFFERENCES
  ;; such that the running sum r12d after processing file i equals file i's
  ;; per-chunk byte contribution.  delta[0] = size_of_file_0_in_chunk,
  ;; delta[i] = size_of_file_i_in_chunk - delta[0] - delta[1] - ... - delta[i-1].
  ;;
  ;; Export:
  ;;   cache_archive_split_payload(
  ;;     payload_offset,     ;; address of JS5 archive payload in WASM memory
  ;;     payload_len,        ;; payload length in bytes
  ;;     file_count,         ;; expected number of files
  ;;     entries_offset,     ;; buffer for entry table (12 * file_count bytes)
  ;;     arena_offset,       ;; buffer for multi-chunk de-interleave
  ;;     arena_len           ;; arena length in bytes
  ;;   ) → status (0=OK, <0=error)

  (func $read_be32 (param $p i32) (result i32)
    local.get $p i32.load8_u
    i32.const 24 i32.shl
    local.get $p i32.load8_u offset=1
    i32.const 16 i32.shl
    i32.or
    local.get $p i32.load8_u offset=2
    i32.const 8 i32.shl
    i32.or
    local.get $p i32.load8_u offset=3
    i32.or
  )

  (func (export "cache_archive_split_payload")
    (param $payload i32) (param $len i32) (param $file_count i32)
    (param $entries i32) (param $arena i32) (param $arena_len i32)
    (result i32)
    (local $chunk_count i32) (local $table_ptr i32) (local $data_body_len i32)
    (local $i i32) (local $j i32) (local $delta i32) (local $acc i32)
    (local $total_desc i32) (local $src i32) (local $entry_base i32)
    (local $stride i32)

    i32.const 12 local.set $stride

    ;; Validate inputs
    local.get $payload i32.eqz if i32.const -1 return end
    local.get $len i32.eqz if i32.const -1 return end
    local.get $file_count i32.eqz if i32.const -2 return end
    local.get $file_count i32.const 65536 i32.gt_u if i32.const -2 return end

    ;; Single file: entire payload is the file
    local.get $file_count i32.const 1 i32.ne
    if
      ;; Multi-file: read chunk count from last byte
      local.get $payload local.get $len i32.add i32.const -1 i32.add i32.load8_u
      local.tee $chunk_count
      i32.eqz if i32.const -3 return end

      ;; Table size = chunk_count * file_count * 4
      local.get $chunk_count local.get $file_count i32.mul i32.const 2 i32.shl
      local.set $i

      ;; Validate table fits in payload (minus 1 for chunk count byte)
      local.get $i local.get $len i32.const 1 i32.sub i32.gt_u
      if i32.const -3 return end

      ;; table_ptr = payload + len - 1 - table_size
      local.get $payload local.get $len i32.add i32.const 1 i32.sub
      local.get $i i32.sub
      local.tee $table_ptr

      ;; data_body_len = table_ptr - payload
      local.get $table_ptr local.get $payload i32.sub
      local.set $data_body_len

      ;; Zero entries
      i32.const 0 local.set $i
      block $clr_done
      loop $clr_loop
        local.get $i local.get $file_count i32.ge_u br_if $clr_done
        local.get $entries local.get $i local.get $stride i32.mul i32.add
        i32.const 0 i32.store       ;; ptr = 0
        local.get $entries local.get $i local.get $stride i32.mul i32.add
        i32.const 0 i32.store offset=4  ;; len = 0
        local.get $entries local.get $i local.get $stride i32.mul i32.add
        i32.const 0 i32.store offset=8  ;; offset = 0
        local.get $i i32.const 1 i32.add local.set $i
        br $clr_loop
      end
      end

      ;; Parse table: for each chunk, running cumulative = per-file contribution
      i32.const 0 local.set $total_desc
      local.get $table_ptr local.set $src
      i32.const 0 local.set $i           ;; chunk index
      block $table_done
      loop $table_chunk_loop
        local.get $i local.get $chunk_count i32.ge_u br_if $table_done
        i32.const 0 local.set $acc        ;; chunk byte accumulator
        i32.const 0 local.set $j          ;; file index
        block $chunk_done
        loop $table_file_loop
          local.get $j local.get $file_count i32.ge_u br_if $chunk_done

          local.get $src call $read_be32
          local.get $acc i32.add
          local.tee $acc                 ;; acc = cumulative = per-file contribution
          i32.const 0 i32.lt_s if i32.const -3 return end

          local.get $entries local.get $j local.get $stride i32.mul i32.add
          local.get $entries local.get $j local.get $stride i32.mul i32.add i32.load offset=4
          local.get $acc i32.add
          i32.store offset=4             ;; entry[j].len += cumul

          local.get $total_desc local.get $acc i32.add local.set $total_desc
          local.get $src i32.const 4 i32.add local.set $src
          local.get $j i32.const 1 i32.add local.set $j
          br $table_file_loop
        end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $table_chunk_loop
      end
      end

      ;; Verify total described bytes matches data_body_len
      local.get $total_desc local.get $data_body_len i32.ne
      if i32.const -3 return end

      ;; If single chunk, point directly into payload
      local.get $chunk_count i32.const 1 i32.eq
      if
        local.get $payload local.set $src
        i32.const 0 local.set $j
        block $span_done
        loop $span_loop
          local.get $j local.get $file_count i32.ge_u br_if $span_done
          local.get $entries local.get $j local.get $stride i32.mul i32.add
          local.get $src i32.store      ;; ptr
          local.get $src
          local.get $entries local.get $j local.get $stride i32.mul i32.add i32.load offset=4
          i32.add local.set $src
          local.get $j i32.const 1 i32.add local.set $j
          br $span_loop
        end
        end
        i32.const 0 return
      end

      ;; Multi-chunk: copy into arena
      i32.const 0 local.set $src           ;; running arena offset
      i32.const 0 local.set $j
      block $adr_done
      loop $adr_loop
        local.get $j local.get $file_count i32.ge_u br_if $adr_done
        local.get $src
        local.get $entries local.get $j local.get $stride i32.mul i32.add i32.load offset=4
        local.tee $delta
        i32.add local.tee $i
        local.get $arena_len i32.gt_u if i32.const -4 return end

        local.get $entries local.get $j local.get $stride i32.mul i32.add
        local.get $arena local.get $src i32.add i32.store  ;; ptr = arena + src
        local.get $entries local.get $j local.get $stride i32.mul i32.add
        i32.const 0 i32.store offset=8    ;; reset write_offset

        local.get $i local.set $src
        local.get $j i32.const 1 i32.add local.set $j
        br $adr_loop
      end
      end

      ;; Copy each chunk: interleaved source → per-file arena destinations
      local.get $table_ptr local.set $src  ;; rewind table pointer
      local.get $payload local.set $i      ;; source = payload data start
      i32.const 0 local.set $j             ;; chunk index
      block $copy_done
      loop $copy_chunk_loop
        local.get $j local.get $chunk_count i32.ge_u br_if $copy_done

        i32.const 0 local.set $acc
        i32.const 0 local.set $table_ptr   ;; reuse as file index
        block $cf_done
        loop $copy_file_loop
          local.get $table_ptr local.get $file_count i32.ge_u br_if $cf_done

          local.get $src call $read_be32
          local.get $acc i32.add
          local.tee $acc                   ;; acc = per-file contribution in this chunk
          i32.const 0 i32.lt_s if i32.const -3 return end

          local.get $entries local.get $table_ptr local.get $stride i32.mul i32.add
          local.tee $entry_base

          ;; rdi = dst = entry.ptr + write_offset
          local.get $entry_base i32.load               ;; ptr
          local.get $entry_base i32.load offset=8      ;; write_offset
          i32.add                                       ;; dst
          local.get $i                                  ;; src = data source
          local.get $acc                                ;; count = acc = per-file contribution
          call $memcpy

          ;; write_offset += acc
          local.get $entry_base
          local.get $entry_base i32.load offset=8
          local.get $acc i32.add
          i32.store offset=8

          ;; advance data source by acc
          local.get $i local.get $acc i32.add local.set $i
          local.get $src i32.const 4 i32.add local.set $src
          local.get $table_ptr i32.const 1 i32.add local.set $table_ptr
          br $copy_file_loop
        end
        end
        local.get $j i32.const 1 i32.add local.set $j
        br $copy_chunk_loop
      end
      end

      ;; Verify final source position
      local.get $payload local.get $data_body_len i32.add
      local.get $i i32.ne if i32.const -3 return end

      i32.const 0 return
    end

    ;; Single file case
    local.get $entries local.get $payload i32.store
    local.get $entries local.get $len i32.store offset=4
    i32.const 0
  )
