;; Baseline JPEG entropy decoder for the retained title.jpg metadata.
  ;;
  ;; Decodes the first MCU block from a JPEG entropy-coded scan using the
  ;; caller-populated Huffman and quantization tables.
  ;;
  ;; State buffer layout (caller-allocated, passed via state_ptr):
  ;;   Offset  Size  Field
  ;;       0     4   entropy_ptr (offset into scan_data within state buffer)
  ;;       4     4   entropy_end  (total scan data length)
  ;;       8     4   bit_buf
  ;;      12     4   bit_count
  ;;      16    12   dc_predictors[3] (i32 × 3)
  ;;      28     4   component_count
  ;;      32     4   scan_component_count
  ;;      36     4   max_h_samp
  ;;      40     4   max_v_samp
  ;;      44    16   scan_component_ids[4]    (i32 × 4)
  ;;      60    16   frame_component_ids[4]   (i32 × 4)
  ;;      76    16   frame_component_qtables[4] (i32 × 4)
  ;;      92    16   scan_component_dc_tables[4] (i32 × 4)
  ;;     108    16   scan_component_ac_tables[4] (i32 × 4)
  ;;     124   128   huffman_code_counts (8 tables × 16 bytes)
  ;;     252    32   huffman_symbol_counts (8 tables × i32)
  ;;     284  2048   huffman_symbols (8 tables × 256 bytes)
  ;;    2332   256   quant_tables (4 tables × 64 bytes)
  ;;    2588   N     scan data (variable length)
  ;; Total header: 2588 bytes.  Callers should allocate ≥ 4096 bytes.
  ;;
  ;; The zigzag-to-natural mapping table is at memory address 0 (64 bytes).
  ;;
  ;; Exports:
  ;;   jpeg_decode_scan_start(state_ptr) → 0 ok, -1 fail
  ;;   jpeg_decode_first_mcu_block(state_ptr, output_64coeffs) → 0 ok, -1 fail
  ;;   jpeg_decode_next_mcu_dequantized(state_ptr, dst_y, dst_cb, dst_cr) → 0 ok, -1 fail(data (i32.const 0) "\00\01\08\10\09\02\03\0a")
  (data (i32.const 8) "\11\18\20\19\12\0b\04\05")
  (data (i32.const 16) "\0c\13\1a\21\28\30\29\22")
  (data (i32.const 24) "\1b\14\0d\06\07\0e\15\1c")
  (data (i32.const 32) "\23\2a\31\38\39\32\2b\24")
  (data (i32.const 40) "\1d\16\0f\17\1e\25\2c\33")
  (data (i32.const 48) "\3a\3b\34\2d\26\1f\27\2e")
  (data (i32.const 56) "\35\3c\3d\36\2f\37\3e\3f")

  ;; Read byte from entropy stream, handling 0xFF-stuffing.
  ;; state_ptr → byte (0-255) or -1 on failure.
  (func $entropy_read_byte (param $s i32) (result i32)
    (local $ptr i32) (local $end i32) (local $byte i32) (local $scan_base i32)
    i32.const 2588 local.set $scan_base
    local.get $s i32.load local.set $ptr
    local.get $s i32.load offset=4 local.set $end
    local.get $ptr local.get $end i32.ge_u if i32.const -1 return end
    local.get $s local.get $scan_base i32.add local.get $ptr i32.add i32.load8_u
    local.set $byte
    local.get $ptr i32.const 1 i32.add local.set $ptr
    local.get $byte i32.const 0xff i32.ne
    if
      local.get $s local.get $ptr i32.store
      local.get $byte return
    end
    local.get $ptr local.get $end i32.ge_u if i32.const -1 return end
    local.get $s local.get $scan_base i32.add local.get $ptr i32.add i32.load8_u
    i32.const 0x00 i32.ne if i32.const -1 return end
    local.get $ptr i32.const 1 i32.add local.set $ptr
    local.get $s local.get $ptr i32.store
    i32.const 0xff
  )

  ;; Read one MSB-first entropy-coded bit.
  ;; state_ptr → bit (0/1) or -1 on failure.
  (func $entropy_read_bit (param $s i32) (result i32)
    (local $bc i32) (local $bb i32) (local $bit i32)
    local.get $s i32.load offset=12
    local.tee $bc
    if
      local.get $bc i32.const 1 i32.sub local.set $bc
      local.get $s i32.load offset=8
      local.get $bc i32.shr_u
      i32.const 1 i32.and
      local.set $bit
      local.get $s local.get $bc i32.store offset=12
      local.get $bit return
    end
    local.get $s call $entropy_read_byte
    local.tee $bb
    i32.const -1 i32.eq if i32.const -1 return end
    local.get $s local.get $bb i32.store offset=8
    local.get $s i32.const 8 i32.store offset=12
    local.get $bb i32.const 7 i32.shr_u i32.const 1 i32.and
    local.get $s i32.const 7 i32.store offset=12
  )

  ;; Receive and sign-extend a JPEG bit pattern.
  ;; state_ptr, category (0-15) → signed value, error_flag (0=ok, -1=fail)
  (func $receive_extend (param $s i32) (param $cat i32) (result i32 i32)
    (local $val i32) (local $i i32) (local $bit i32) (local $thresh i32)
    local.get $cat i32.const 16 i32.gt_u if i32.const 0 i32.const -1 return end
    local.get $cat i32.eqz if i32.const 0 i32.const 0 return end
    i32.const 0 local.set $val
    i32.const 0 local.set $i
    block $read_done
    loop $read_loop
      local.get $i local.get $cat i32.ge_u br_if $read_done
      local.get $s call $entropy_read_bit
      local.tee $bit
      i32.const -1 i32.eq if i32.const 0 i32.const -1 return end
      local.get $val i32.const 1 i32.shl local.get $bit i32.or local.set $val
      local.get $i i32.const 1 i32.add local.set $i
      br $read_loop
    end
    end
    i32.const 1
    local.get $cat i32.const 1 i32.sub
    i32.shl
    local.set $thresh
    local.get $val local.get $thresh i32.ge_u
    if
      local.get $val i32.const 0 return
    end
    local.get $val
    i32.const 1
    local.get $cat
    i32.shl
    i32.const 1 i32.sub
    i32.sub
    i32.const 0
  )

  ;; Decode a canonical Huffman code symbol.
  ;; state_ptr, table_index → symbol (0-255) or -1 on failure.
  (func $huffman_decode_symbol (param $s i32) (param $tbl i32) (result i32)
    (local $sym_cnt i32) (local $code i32) (local $first_code i32)
    (local $first_sym i32) (local $len_idx i32) (local $cnt i32)
    (local $codecnt_off i32) (local $sym_off i32) (local $bit i32)
    (local $diff i32)

    local.get $tbl i32.const 8 i32.ge_u if i32.const -1 return end
    local.get $s i32.const 252 i32.add local.get $tbl i32.const 2 i32.shl i32.add i32.load
    local.tee $sym_cnt
    i32.eqz if i32.const -1 return end
    local.get $sym_cnt i32.const 256 i32.gt_u if i32.const -1 return end

    local.get $tbl i32.const 4 i32.shl local.set $codecnt_off
    local.get $tbl i32.const 8 i32.shl local.set $sym_off
    i32.const 0 local.set $code
    i32.const 0 local.set $first_code
    i32.const 0 local.set $first_sym
    i32.const 0 local.set $len_idx

    block $fail
    loop $len_loop
      local.get $len_idx i32.const 16 i32.ge_u br_if $fail

      local.get $s call $entropy_read_bit
      local.tee $bit
      i32.const -1 i32.eq br_if $fail

      local.get $code i32.const 1 i32.shl local.get $bit i32.or local.set $code
      local.get $first_code i32.const 1 i32.shl local.set $first_code

      local.get $s i32.const 124 i32.add
      local.get $codecnt_off local.get $len_idx i32.add i32.add
      i32.load8_u
      local.tee $cnt
      i32.eqz
      if else
        local.get $code local.get $first_code i32.ge_u
        if
          local.get $code local.get $first_code i32.sub
          local.tee $diff
          local.get $cnt i32.lt_u
          if
            local.get $first_sym local.get $diff i32.add
            local.tee $diff
            local.get $sym_cnt i32.ge_u br_if $fail
            local.get $s i32.const 284 i32.add
            local.get $sym_off i32.add
            local.get $diff i32.add
            i32.load8_u
            return
          end
        end
      end

      local.get $first_code local.get $cnt i32.add local.set $first_code
      local.get $first_sym local.get $cnt i32.add local.set $first_sym
      local.get $len_idx i32.const 1 i32.add local.set $len_idx
      br $len_loop
    end
    end
    i32.const -1
  )

  ;; Decode one component's 64 coefficients (DC + AC) with dequantization.
  ;; state_ptr, component_index (0-2), output_dst → 0 ok, -1 fail
  (func $decode_component_dequantized (param $s i32) (param $comp i32) (param $dst i32) (result i32)
    (local $cursor i32) (local $symbol i32) (local $value i32)
    (local $dc_tbl i32) (local $ac_tbl i32) (local $qtbl_idx i32)
    (local $nat_idx i32) (local $qtbl_off i32) (local $err i32)

    local.get $comp i32.const 3 i32.ge_u if i32.const -1 return end

    ;; Validate component IDs match
    local.get $s i32.const 44 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    local.get $s i32.const 60 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    i32.ne if i32.const -1 return end

    ;; Get quant table index
    local.get $s i32.const 76 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    local.tee $qtbl_idx
    i32.const 4 i32.ge_u if i32.const -1 return end

    ;; Zero output buffer (64 int16 = 128 bytes)
    i32.const 0 local.set $cursor
    block $zero_done
    loop $zero_loop
      local.get $cursor i32.const 128 i32.ge_s br_if $zero_done
      local.get $dst local.get $cursor i32.add i32.const 0 i32.store16
      local.get $cursor i32.const 2 i32.add local.set $cursor
      br $zero_loop
    end
    end

    ;; Get DC table index for this component
    local.get $s i32.const 92 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    local.tee $dc_tbl
    i32.const 4 i32.ge_u if i32.const -1 return end

    ;; Decode DC coefficient
    local.get $s local.get $dc_tbl call $huffman_decode_symbol
    local.tee $symbol
    i32.const -1 i32.eq if i32.const -1 return end
    local.get $symbol i32.const 16 i32.gt_u if i32.const -1 return end

    local.get $s local.get $symbol call $receive_extend
    local.set $err
    local.set $value
    local.get $err if i32.const -1 return end

    ;; Update DC predictor
    local.get $s i32.const 16 i32.add local.get $comp i32.const 2 i32.shl i32.add
    local.get $s i32.const 16 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    local.get $value i32.add
    local.tee $value
    i32.store
    local.get $dst local.get $value i32.store16

    ;; Get AC table index
    local.get $s i32.const 108 i32.add local.get $comp i32.const 2 i32.shl i32.add i32.load
    local.tee $ac_tbl
    i32.const 4 i32.ge_u if i32.const -1 return end

    i32.const 1 local.set $cursor
    block $ac_done
    loop $ac_loop
      local.get $cursor i32.const 64 i32.ge_u br_if $ac_done

      local.get $s local.get $ac_tbl i32.const 4 i32.add call $huffman_decode_symbol
      local.tee $symbol
      i32.const -1 i32.eq if i32.const -1 return end

      local.get $symbol i32.eqz if br $ac_done end

      local.get $symbol i32.const 0xf0 i32.eq
      if
        local.get $cursor i32.const 16 i32.add local.set $cursor
        local.get $cursor i32.const 64 i32.gt_u if i32.const -1 return end
        br $ac_loop
      end

      ;; Extract zero-run and amplitude bits
      local.get $symbol i32.const 4 i32.shr_u
      local.get $cursor i32.add local.set $cursor
      local.get $symbol i32.const 15 i32.and
      local.tee $symbol
      i32.eqz if i32.const -1 return end
      local.get $cursor i32.const 64 i32.ge_u if i32.const -1 return end

      local.get $s local.get $symbol call $receive_extend
      local.set $err
      local.set $value
      local.get $err if i32.const -1 return end

      local.get $cursor i32.load8_u
      local.tee $nat_idx
      i32.const 1 i32.shl
      local.get $dst i32.add
      local.get $value i32.store16

      local.get $cursor i32.const 1 i32.add local.set $cursor
      br $ac_loop
    end
    end

    ;; Dequantize
    local.get $qtbl_idx i32.const 6 i32.shl
    local.set $qtbl_off
    i32.const 0 local.set $cursor
    block $dequant_done
    loop $dequant_loop
      local.get $cursor i32.const 64 i32.ge_u br_if $dequant_done

      local.get $cursor i32.load8_u
      local.tee $nat_idx
      i32.const 1 i32.shl
      local.get $dst i32.add

      local.get $dst
      local.get $nat_idx i32.const 1 i32.shl i32.add
      i32.load16_s
      local.get $s i32.const 2332 i32.add local.get $qtbl_off local.get $cursor i32.add i32.add i32.load8_u
      i32.mul
      i32.store16

      local.get $cursor i32.const 1 i32.add local.set $cursor
      br $dequant_loop
    end
    end
    i32.const 0
  )

  ;; Initialize entropy decoder state for a new scan.
  ;; Resets bit buffer, bit count, and DC predictors.  Sets entropy pointer
  ;; to the beginning of the scan data within the state buffer.
  ;; state_ptr → 0 ok, -1 if no scan data
  (func (export "jpeg_decode_scan_start") (param $s i32) (result i32)
    local.get $s i32.load offset=4
    i32.eqz if i32.const -1 return end
    local.get $s i32.const 0 i32.store
    local.get $s i32.const 0 i32.store offset=8
    local.get $s i32.const 0 i32.store offset=12
    local.get $s i32.const 0 i32.store offset=16
    local.get $s i32.const 0 i32.store offset=20
    local.get $s i32.const 0 i32.store offset=24
    i32.const 0
  )

  ;; Decode the first MCU block (64 raw int16 coefficients) from the
  ;; entropy-coded scan, using hardcoded DC table 0 and AC table 4.
  ;; This does NOT use component tables or dequantization — it decodes
  ;; the raw DC+AC coefficients exactly as `title_jpeg_decode_first_mcu_block`.
  ;; state_ptr, output_64coeffs → 0 ok, -1 fail
  (func (export "jpeg_decode_first_mcu_block") (param $s i32) (param $out i32) (result i32)
    (local $cursor i32) (local $symbol i32) (local $value i32)
    (local $nat_idx i32) (local $err i32)

    local.get $s i32.load offset=4 i32.eqz if i32.const -1 return end
    local.get $s i32.const 0 i32.store
    local.get $s i32.const 0 i32.store offset=8
    local.get $s i32.const 0 i32.store offset=12

    ;; Zero output (64 int16 = 128 bytes)
    i32.const 0 local.set $cursor
    block $zero_done
    loop $zero_loop
      local.get $cursor i32.const 128 i32.ge_s br_if $zero_done
      local.get $out local.get $cursor i32.add i32.const 0 i32.store16
      local.get $cursor i32.const 2 i32.add local.set $cursor
      br $zero_loop
    end
    end

    ;; Decode DC coefficient (table 0)
    local.get $s i32.const 0 call $huffman_decode_symbol
    local.tee $symbol
    i32.const -1 i32.eq if i32.const -1 return end
    local.get $symbol i32.const 16 i32.gt_u if i32.const -1 return end

    local.get $s local.get $symbol call $receive_extend
    local.set $err
    local.set $value
    local.get $err if i32.const -1 return end
    local.get $out local.get $value i32.store16

    ;; Decode AC coefficients (table 4 = AC table 0)
    i32.const 1 local.set $cursor
    block $ac_done
    loop $ac_loop
      local.get $cursor i32.const 64 i32.ge_u br_if $ac_done

      local.get $s i32.const 4 call $huffman_decode_symbol
      local.tee $symbol
      i32.const -1 i32.eq if i32.const -1 return end

      local.get $symbol i32.eqz if br $ac_done end

      local.get $symbol i32.const 0xf0 i32.eq
      if
        local.get $cursor i32.const 16 i32.add local.set $cursor
        local.get $cursor i32.const 64 i32.gt_u if i32.const -1 return end
        br $ac_loop
      end

      local.get $symbol i32.const 4 i32.shr_u
      local.get $cursor i32.add local.set $cursor
      local.get $symbol i32.const 15 i32.and
      local.tee $symbol
      i32.eqz if i32.const -1 return end
      local.get $cursor i32.const 64 i32.ge_u if i32.const -1 return end

      local.get $s local.get $symbol call $receive_extend
      local.set $err
      local.set $value
      local.get $err if i32.const -1 return end

      local.get $cursor i32.load8_u
      local.tee $nat_idx
      i32.const 1 i32.shl
      local.get $out i32.add
      local.get $value i32.store16

      local.get $cursor i32.const 1 i32.add local.set $cursor
      br $ac_loop
    end
    end
    i32.const 0
  )

  ;; Decode and dequantize a complete 3-component 1x1-sampled MCU.
  ;; state_ptr must be initialized with jpeg_decode_scan_start first.
  ;; Components are decoded in natural order (Y, Cb, Cr).
  ;; Each output buffer must be 64 int16 coefficients (128 bytes).
  ;; state_ptr, dst_y, dst_cb, dst_cr → 0 ok, -1 fail
  (func (export "jpeg_decode_next_mcu_dequantized")
    (param $s i32) (param $y i32) (param $cb i32) (param $cr i32)
    (result i32)

    local.get $y i32.eqz if i32.const -1 return end
    local.get $cb i32.eqz if i32.const -1 return end
    local.get $cr i32.eqz if i32.const -1 return end

    local.get $s i32.load offset=28
    i32.const 3 i32.ne if i32.const -1 return end
    local.get $s i32.load offset=32
    i32.const 3 i32.ne if i32.const -1 return end
    local.get $s i32.load offset=36
    i32.const 1 i32.ne if i32.const -1 return end
    local.get $s i32.load offset=40
    i32.const 1 i32.ne if i32.const -1 return end

    local.get $s i32.const 0 local.get $y call $decode_component_dequantized
    if i32.const -1 return end

    local.get $s i32.const 1 local.get $cb call $decode_component_dequantized
    if i32.const -1 return end

    local.get $s i32.const 2 local.get $cr call $decode_component_dequantized
    if i32.const -1 return end

    i32.const 0
  )
