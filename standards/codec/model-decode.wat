(module
  (memory (export "memory") 72)

  ;; ── BSS memory layout ──
  (global $MODEL_VERTEX_X        i32 (i32.const 0x00000))
  (global $MODEL_VERTEX_Y        i32 (i32.const 0x08000))
  (global $MODEL_VERTEX_Z        i32 (i32.const 0x10000))
  (global $MODEL_VERTEX_GROUP    i32 (i32.const 0x18000))
  (global $MODEL_FACE_A          i32 (i32.const 0x20000))
  (global $MODEL_FACE_B          i32 (i32.const 0x28000))
  (global $MODEL_FACE_C          i32 (i32.const 0x30000))
  (global $MODEL_FACE_COLOR      i32 (i32.const 0x38000))

  (global $S_DECODE_VERTEX_WRITE_BASE i32 (i32.const 0x40000))
  (global $S_DECODE_FACE_WRITE_BASE   i32 (i32.const 0x40004))
  (global $S_REGISTRY_NEXT_VERTEX     i32 (i32.const 0x40008))
  (global $S_REGISTRY_NEXT_FACE       i32 (i32.const 0x4000c))
  (global $S_LAST_DECODED_VERTEX_COUNT i32 (i32.const 0x40010))
  (global $S_LAST_DECODED_FACE_COUNT  i32 (i32.const 0x40014))
  (global $S_LAST_DECODED_VERTEX_BASE i32 (i32.const 0x40018))
  (global $S_LAST_DECODED_FACE_BASE   i32 (i32.const 0x4001c))
  (global $S_ASSET_COUNT              i32 (i32.const 0x40020))
  (global $MODEL_ASSETS               i32 (i32.const 0x40100))

  ;; ── Struct offsets: model_meta_core (96 bytes) ──
  (global $META_FORMAT            i32 (i32.const 0))
  (global $META_VERTEX_COUNT      i32 (i32.const 4))
  (global $META_FACE_COUNT        i32 (i32.const 8))
  (global $META_TEXTURE_COUNT     i32 (i32.const 12))
  (global $META_FLAGS             i32 (i32.const 16))
  (global $META_MIN_X             i32 (i32.const 20))
  (global $META_MAX_X             i32 (i32.const 24))
  (global $META_MIN_Y             i32 (i32.const 28))
  (global $META_MAX_Y             i32 (i32.const 32))
  (global $META_MIN_Z             i32 (i32.const 36))
  (global $META_MAX_Z             i32 (i32.const 40))
  (global $META_VERTEX_FLAGS_OFF  i32 (i32.const 44))
  (global $META_VERTEX_X_OFF      i32 (i32.const 48))
  (global $META_VERTEX_Y_OFF      i32 (i32.const 52))
  (global $META_VERTEX_Z_OFF      i32 (i32.const 56))
  (global $META_VERTEX_GROUPS_OFF i32 (i32.const 60))
  (global $META_DATA_END_OFF      i32 (i32.const 64))
  (global $META_FACE_TYPES_OFF    i32 (i32.const 68))
  (global $META_FACE_INDICES_OFF  i32 (i32.const 72))
  (global $META_FACE_COLORS_OFF   i32 (i32.const 76))
  (global $META_DECODED_VERTEX_BASE i32 (i32.const 80))
  (global $META_DECODED_FACE_BASE  i32 (i32.const 84))
  (global $META_DECODED_VERTEX_COUNT i32 (i32.const 88))
  (global $META_DECODED_FACE_COUNT i32 (i32.const 92))

  ;; ── Struct offsets: model_asset_core (48 bytes) ──
  (global $ASSET_MODEL_ID         i32 (i32.const 0))
  (global $ASSET_VERTEX_BASE      i32 (i32.const 4))
  (global $ASSET_VERTEX_COUNT     i32 (i32.const 8))
  (global $ASSET_VERTEX_GROUP_BASE i32 (i32.const 12))
  (global $ASSET_FACE_BASE        i32 (i32.const 16))
  (global $ASSET_FACE_COUNT       i32 (i32.const 20))
  (global $ASSET_MIN_X            i32 (i32.const 24))
  (global $ASSET_MAX_X            i32 (i32.const 28))
  (global $ASSET_MIN_Y            i32 (i32.const 32))
  (global $ASSET_MAX_Y            i32 (i32.const 36))
  (global $ASSET_MIN_Z            i32 (i32.const 40))
  (global $ASSET_MAX_Z            i32 (i32.const 44))
  (global $ASSET_SIZE             i32 (i32.const 48))

  ;; ── Constants ──
  (global $MODEL_OK            i32 (i32.const 0))
  (global $MODEL_ERR_BOUNDS    i32 (i32.const -1))
  (global $MODEL_ERR_FORMAT    i32 (i32.const -2))
  (global $MODEL_ERR_TRUNCATED i32 (i32.const -3))
  (global $MODEL_FORMAT_OLD    i32 (i32.const 0))
  (global $MODEL_FORMAT_TYPE1  i32 (i32.const 1))
  (global $MODEL_FORMAT_TYPE2  i32 (i32.const 2))
  (global $MODEL_FORMAT_TYPE3  i32 (i32.const 3))
  (global $MODEL_MAX_VERTICES  i32 (i32.const 8192))
  (global $MODEL_MAX_FACES     i32 (i32.const 8192))
  (global $MODEL_MAX_ASSETS    i32 (i32.const 512))

  ;; ── Exported array base pointers ──
  (global (export "model_vertex_x") i32 (i32.const 0x00000))
  (global (export "model_vertex_y") i32 (i32.const 0x08000))
  (global (export "model_vertex_z") i32 (i32.const 0x10000))
  (global (export "model_vertex_group") i32 (i32.const 0x18000))
  (global (export "model_face_a") i32 (i32.const 0x20000))
  (global (export "model_face_b") i32 (i32.const 0x28000))
  (global (export "model_face_c") i32 (i32.const 0x30000))
  (global (export "model_face_color") i32 (i32.const 0x38000))
  (global (export "model_assets") i32 (i32.const 0x40100))

  ;; ═══════════════════════════════════════════════════════════════════════
  ;; Internal helpers
  ;; ═══════════════════════════════════════════════════════════════════════

  (func $read_u16be (param $p i32) (param $o i32) (result i32 i32)
    local.get $p local.get $o i32.add i32.load8_u
    i32.const 8 i32.shl
    local.get $p local.get $o i32.const 1 i32.add i32.add i32.load8_u
    i32.or
    local.get $o i32.const 2 i32.add
  )

  (func $read_short_smart (param $p i32) (param $len i32) (param $o i32) (result i32 i32)
    (local $b i32)
    local.get $o local.get $len i32.ge_u if i32.const 0 local.get $o return end
    local.get $p local.get $o i32.add i32.load8_u
    local.tee $b
    i32.const 128 i32.ge_u
    if
      local.get $o i32.const 2 i32.add local.get $len i32.gt_u if i32.const 0 local.get $o return end
      local.get $p local.get $o i32.add i32.load8_u i32.const 8 i32.shl
      local.get $p local.get $o i32.const 1 i32.add i32.add i32.load8_u
      i32.or
      i32.const 0xc000 i32.sub
      local.get $o i32.const 2 i32.add
      return
    end
    local.get $b i32.const 64 i32.sub
    local.get $o i32.const 1 i32.add
  )

  ;; ═══════════════════════════════════════════════════════════════════════
  ;; model_decode_metadata_payload
  ;; ═══════════════════════════════════════════════════════════════════════
  (func (export "model_decode_metadata_payload")
        (param $payload i32) (param $len i32) (param $meta i32) (result i32)
    i32.const 0 global.get $S_DECODE_VERTEX_WRITE_BASE i32.store offset=0
    i32.const 0 global.get $S_DECODE_FACE_WRITE_BASE i32.store offset=0
    local.get $payload local.get $len local.get $meta
    call $metadata_payload_with_bases
  )

  ;; ═══════════════════════════════════════════════════════════════════════
  ;; metadata_payload_with_bases (internal)
  ;; ═══════════════════════════════════════════════════════════════════════
  (func $metadata_payload_with_bases
        (param $payload i32) (param $len i32) (param $meta i32) (result i32)
    (local $end i32) (local $off i32)
    (local $vertex_cnt i32) (local $face_cnt i32) (local $tex_cnt i32)
    (local $var11 i32) (local $var12 i32) (local $var13 i32) (local $var14 i32)
    (local $var15 i32) (local $var16 i32)
    (local $x_bytes i32) (local $y_bytes i32) (local $z_bytes i32)
    (local $face_idx_bytes i32) (local $tex_vert_bytes i32)
    (local $i i32) (local $v i32) (local $cur_x i32) (local $cur_y i32) (local $cur_z i32)
    (local $vtx_off i32) (local $f_types_off i32) (local $f_idx_off i32) (local $f_colors_off i32)
    (local $simple i32) (local $complex i32) (local $type2 i32) (local $tex_type i32)
    (local $prev_a i32) (local $prev_b i32) (local $prev_c i32) (local $prev_high i32)
    (local $da i32) (local $db i32) (local $dc i32) (local $tmp i32)

    local.get $payload i32.eqz if global.get $MODEL_ERR_BOUNDS return end
    local.get $meta i32.eqz if global.get $MODEL_ERR_BOUNDS return end
    local.get $len i32.const 23 i32.lt_u if global.get $MODEL_ERR_TRUNCATED return end

    local.get $len local.set $end

    ;; ── Check format marker ──
    local.get $payload local.get $end i32.const 2 i32.sub i32.add i32.load8_u
    local.set $var12
    local.get $payload local.get $end i32.const 1 i32.sub i32.add i32.load8_u
    local.set $var13

    (block $fmt_ok
      local.get $var12 i32.const 0xff i32.ne
      if
        local.get $meta global.get $META_FORMAT i32.add global.get $MODEL_FORMAT_OLD i32.store offset=0
        global.get $MODEL_ERR_FORMAT return
        br $fmt_ok
      end
      local.get $var13 i32.const 0xfd i32.eq
      if
        local.get $meta global.get $META_FORMAT i32.add global.get $MODEL_FORMAT_TYPE3 i32.store offset=0
        global.get $MODEL_ERR_FORMAT return
        br $fmt_ok
      end
      local.get $var13 i32.const 0xfe i32.eq
      if
        local.get $meta global.get $META_FORMAT i32.add global.get $MODEL_FORMAT_TYPE2 i32.store offset=0
        br $fmt_ok
      end
      ;; type1 or old
      local.get $meta global.get $META_FORMAT i32.add global.get $MODEL_FORMAT_TYPE1 i32.store offset=0
      global.get $MODEL_ERR_FORMAT return
    )

    ;; ── Read header fields ──
    local.get $end i32.const 23 i32.sub
    local.set $off

    local.get $payload local.get $off call $read_u16be
    local.set $off
    local.set $vertex_cnt
    local.get $meta global.get $META_VERTEX_COUNT i32.add local.get $vertex_cnt i32.store offset=0

    local.get $payload local.get $off call $read_u16be
    local.set $off
    local.set $face_cnt
    local.get $meta global.get $META_FACE_COUNT i32.add local.get $face_cnt i32.store offset=0

    local.get $payload local.get $off i32.add i32.load8_u
    local.set $tex_cnt
    local.get $off i32.const 1 i32.add local.set $off
    local.get $meta global.get $META_TEXTURE_COUNT i32.add local.get $tex_cnt i32.store offset=0

    local.get $payload local.get $off i32.add i32.load8_u
    local.set $var11
    local.get $off i32.const 1 i32.add local.set $off

    local.get $payload local.get $off i32.add i32.load8_u
    local.set $var12
    local.get $off i32.const 1 i32.add local.set $off

    local.get $payload local.get $off i32.add i32.load8_u
    local.set $var13
    local.get $off i32.const 1 i32.add local.set $off

    local.get $payload local.get $off i32.add i32.load8_u
    local.set $var14
    local.get $off i32.const 1 i32.add local.set $off

    local.get $payload local.get $off i32.add i32.load8_u
    local.set $var15
    local.get $off i32.const 1 i32.add local.set $off

    local.get $payload local.get $off i32.add i32.load8_u
    local.set $var16
    local.get $off i32.const 1 i32.add local.set $off

    local.get $payload local.get $off call $read_u16be
    local.set $off
    local.set $x_bytes

    local.get $payload local.get $off call $read_u16be
    local.set $off
    local.set $y_bytes

    local.get $payload local.get $off call $read_u16be
    local.set $off
    local.set $z_bytes

    local.get $payload local.get $off call $read_u16be
    local.set $off
    local.set $face_idx_bytes

    local.get $payload local.get $off call $read_u16be
    local.set $off
    local.set $tex_vert_bytes

    ;; ── Capacity checks ──
    local.get $meta global.get $META_FLAGS i32.add i32.const 0 i32.store offset=0

    local.get $meta global.get $META_DECODED_VERTEX_BASE i32.add
    global.get $S_DECODE_VERTEX_WRITE_BASE i32.load offset=0
    i32.store offset=0
    global.get $S_DECODE_VERTEX_WRITE_BASE i32.load offset=0
    local.get $vertex_cnt i32.add
    global.get $MODEL_MAX_VERTICES i32.gt_u
    if global.get $MODEL_ERR_BOUNDS return end

    local.get $meta global.get $META_DECODED_FACE_BASE i32.add
    global.get $S_DECODE_FACE_WRITE_BASE i32.load offset=0
    i32.store offset=0
    global.get $S_DECODE_FACE_WRITE_BASE i32.load offset=0
    local.get $face_cnt i32.add
    global.get $MODEL_MAX_FACES i32.gt_u
    if global.get $MODEL_ERR_BOUNDS return end

    local.get $meta global.get $META_DECODED_VERTEX_COUNT i32.add i32.const 0 i32.store offset=0
    local.get $meta global.get $META_DECODED_FACE_COUNT i32.add i32.const 0 i32.store offset=0

    local.get $vertex_cnt global.get $MODEL_MAX_VERTICES i32.gt_u
    if global.get $MODEL_ERR_BOUNDS return end
    local.get $face_cnt global.get $MODEL_MAX_FACES i32.gt_u
    if global.get $MODEL_ERR_BOUNDS return end

    ;; ── Choose offset strategy ──
    local.get $meta global.get $META_FORMAT i32.add i32.load offset=0
    global.get $MODEL_FORMAT_TYPE1 i32.eq
    if
      ;; ── Type 1 offsets ──
      i32.const 0 local.set $simple
      i32.const 0 local.set $complex
      i32.const 0 local.set $type2
      i32.const 0 local.set $i
      (loop $tex_loop
        local.get $i local.get $tex_cnt i32.lt_u
        if
          local.get $payload local.get $i i32.add i32.load8_u
          local.tee $tex_type
          i32.eqz
          if local.get $simple i32.const 1 i32.add local.set $simple end
          local.get $tex_type i32.const 1 i32.ge_s
          if
            local.get $tex_type i32.const 3 i32.le_s
            if local.get $complex i32.const 1 i32.add local.set $complex end
          end
          local.get $tex_type i32.const 2 i32.eq
          if local.get $type2 i32.const 1 i32.add local.set $type2 end
          local.get $i i32.const 1 i32.add local.set $i
          br $tex_loop
        end
      )

      local.get $meta global.get $META_VERTEX_FLAGS_OFF i32.add local.get $tex_cnt i32.store offset=0
      local.get $tex_cnt local.get $vertex_cnt i32.add local.set $v

      local.get $var11 i32.eqz if local.get $v local.get $face_cnt i32.add local.set $v end
      local.get $v local.get $face_cnt i32.add local.set $v
      local.get $var12 i32.const 255 i32.ne if local.get $v local.get $face_cnt i32.add local.set $v end
      local.get $var14 i32.eqz if local.get $v local.get $face_cnt i32.add local.set $v end
      local.get $var16 i32.eqz if local.get $v local.get $vertex_cnt i32.add local.set $v end
      local.get $meta global.get $META_VERTEX_GROUPS_OFF i32.add i32.const -1 i32.store offset=0
      local.get $var13 i32.eqz if local.get $v local.get $face_cnt i32.add local.set $v end
      local.get $v local.get $face_idx_bytes i32.add local.set $v
      local.get $var15 i32.const 1 i32.and if local.get $face_cnt i32.const 1 i32.shl local.get $v i32.add local.set $v end
      local.get $v local.get $tex_vert_bytes i32.add local.set $v
      local.get $face_cnt i32.const 1 i32.shl local.get $v i32.add local.set $v

      local.get $meta global.get $META_VERTEX_X_OFF i32.add local.get $v i32.store offset=0
      local.get $v local.get $x_bytes i32.add local.set $v
      local.get $meta global.get $META_VERTEX_Y_OFF i32.add local.get $v i32.store offset=0
      local.get $v local.get $y_bytes i32.add local.set $v
      local.get $meta global.get $META_VERTEX_Z_OFF i32.add local.get $v i32.store offset=0
      local.get $v local.get $z_bytes i32.add local.set $v

      local.get $simple i32.const 3 i32.mul i32.const 1 i32.shl local.get $v i32.add local.set $v
      local.get $complex i32.const 3 i32.mul i32.const 1 i32.shl local.get $v i32.add local.set $v
      local.get $complex i32.const 3 i32.mul i32.const 1 i32.shl local.get $v i32.add local.set $v
      local.get $complex i32.const 1 i32.shl local.get $v i32.add local.set $v
      local.get $complex local.get $v i32.add local.set $v
      local.get $type2 i32.const 1 i32.shl local.get $v i32.add local.set $v

      local.get $meta global.get $META_DATA_END_OFF i32.add local.get $v i32.store offset=0
    else
      ;; ── Type 2 offsets ──
      local.get $meta global.get $META_VERTEX_FLAGS_OFF i32.add i32.const 0 i32.store offset=0
      local.get $vertex_cnt local.set $v

      local.get $meta global.get $META_FACE_TYPES_OFF i32.add local.get $v i32.store offset=0
      local.get $v local.get $face_cnt i32.add local.set $v
      local.get $var12 i32.const 255 i32.ne if local.get $v local.get $face_cnt i32.add local.set $v end
      local.get $var14 i32.eqz if local.get $v local.get $face_cnt i32.add local.set $v end
      local.get $var11 i32.eqz if local.get $v local.get $face_cnt i32.add local.set $v end
      local.get $meta global.get $META_VERTEX_GROUPS_OFF i32.add i32.const -1 i32.store offset=0
      local.get $tex_vert_bytes if
        local.get $meta global.get $META_VERTEX_GROUPS_OFF i32.add local.get $v i32.store offset=0
        local.get $v local.get $tex_vert_bytes i32.add local.set $v
      end
      local.get $var13 i32.eqz if local.get $v local.get $face_cnt i32.add local.set $v end
      local.get $meta global.get $META_FACE_INDICES_OFF i32.add local.get $v i32.store offset=0
      local.get $v local.get $face_idx_bytes i32.add local.set $v
      local.get $meta global.get $META_FACE_COLORS_OFF i32.add local.get $v i32.store offset=0
      local.get $face_cnt i32.const 1 i32.shl local.get $v i32.add local.set $v
      local.get $tex_cnt i32.const 3 i32.mul i32.const 1 i32.shl local.get $v i32.add local.set $v
      local.get $meta global.get $META_VERTEX_X_OFF i32.add local.get $v i32.store offset=0
      local.get $v local.get $x_bytes i32.add local.set $v
      local.get $meta global.get $META_VERTEX_Y_OFF i32.add local.get $v i32.store offset=0
      local.get $v local.get $y_bytes i32.add local.set $v
      local.get $meta global.get $META_VERTEX_Z_OFF i32.add local.get $v i32.store offset=0
      local.get $v local.get $z_bytes i32.add local.set $v
      local.get $meta global.get $META_DATA_END_OFF i32.add local.get $v i32.store offset=0
    end

    ;; ── Verify data_end ──
    local.get $meta global.get $META_DATA_END_OFF i32.add i32.load offset=0
    local.get $end i32.gt_u
    if global.get $MODEL_ERR_TRUNCATED return end

    ;; ── Initialize bounds ──
    local.get $meta global.get $META_MIN_X i32.add i32.const 0x7fffffff i32.store offset=0
    local.get $meta global.get $META_MIN_Y i32.add i32.const 0x7fffffff i32.store offset=0
    local.get $meta global.get $META_MIN_Z i32.add i32.const 0x7fffffff i32.store offset=0
    local.get $meta global.get $META_MAX_X i32.add i32.const 0x80000000 i32.store offset=0
    local.get $meta global.get $META_MAX_Y i32.add i32.const 0x80000000 i32.store offset=0
    local.get $meta global.get $META_MAX_Z i32.add i32.const 0x80000000 i32.store offset=0

    ;; ── Decode vertices ──
    local.get $meta global.get $META_VERTEX_FLAGS_OFF i32.add i32.load offset=0
    local.set $vtx_off
    local.get $meta global.get $META_VERTEX_X_OFF i32.add i32.load offset=0
    local.set $v
    local.get $meta global.get $META_VERTEX_Y_OFF i32.add i32.load offset=0
    local.set $f_types_off
    local.get $meta global.get $META_VERTEX_Z_OFF i32.add i32.load offset=0
    local.set $f_colors_off

    i32.const 0 local.set $i
    i32.const 0 local.set $cur_x
    i32.const 0 local.set $cur_y
    i32.const 0 local.set $cur_z

    (block $vtx_done
    (loop $vtx_loop
      local.get $i local.get $vertex_cnt i32.ge_u
      br_if $vtx_done

      local.get $vtx_off local.get $end i32.ge_u
      if global.get $MODEL_ERR_TRUNCATED return end

      local.get $payload local.get $vtx_off i32.add i32.load8_u
      local.set $var11
      local.get $vtx_off i32.const 1 i32.add local.set $vtx_off

      local.get $var11 i32.const 1 i32.and
      if
        local.get $payload local.get $len local.get $v
        call $read_short_smart
        local.set $v
        local.set $da
        local.get $cur_x local.get $da i32.add local.set $cur_x
      end

      local.get $var11 i32.const 2 i32.and
      if
        local.get $payload local.get $len local.get $f_types_off
        call $read_short_smart
        local.set $f_types_off
        local.set $da
        local.get $cur_y local.get $da i32.add local.set $cur_y
      end

      local.get $var11 i32.const 4 i32.and
      if
        local.get $payload local.get $len local.get $f_colors_off
        call $read_short_smart
        local.set $f_colors_off
        local.set $da
        local.get $cur_z local.get $da i32.add local.set $cur_z
      end

      ;; update bounds
      local.get $cur_x
      local.get $meta global.get $META_MIN_X i32.add i32.load offset=0
      i32.lt_s
      if local.get $meta global.get $META_MIN_X i32.add local.get $cur_x i32.store offset=0 end
      local.get $cur_x
      local.get $meta global.get $META_MAX_X i32.add i32.load offset=0
      i32.gt_s
      if local.get $meta global.get $META_MAX_X i32.add local.get $cur_x i32.store offset=0 end

      local.get $cur_y
      local.get $meta global.get $META_MIN_Y i32.add i32.load offset=0
      i32.lt_s
      if local.get $meta global.get $META_MIN_Y i32.add local.get $cur_y i32.store offset=0 end
      local.get $cur_y
      local.get $meta global.get $META_MAX_Y i32.add i32.load offset=0
      i32.gt_s
      if local.get $meta global.get $META_MAX_Y i32.add local.get $cur_y i32.store offset=0 end

      local.get $cur_z
      local.get $meta global.get $META_MIN_Z i32.add i32.load offset=0
      i32.lt_s
      if local.get $meta global.get $META_MIN_Z i32.add local.get $cur_z i32.store offset=0 end
      local.get $cur_z
      local.get $meta global.get $META_MAX_Z i32.add i32.load offset=0
      i32.gt_s
      if local.get $meta global.get $META_MAX_Z i32.add local.get $cur_z i32.store offset=0 end

      ;; write vertex
      global.get $S_DECODE_VERTEX_WRITE_BASE i32.load offset=0
      local.get $i i32.add
      local.set $da

      global.get $MODEL_VERTEX_X local.get $da i32.const 2 i32.shl i32.add
      local.get $cur_x i32.store offset=0
      global.get $MODEL_VERTEX_Y local.get $da i32.const 2 i32.shl i32.add
      local.get $cur_y i32.store offset=0
      global.get $MODEL_VERTEX_Z local.get $da i32.const 2 i32.shl i32.add
      local.get $cur_z i32.store offset=0
      global.get $MODEL_VERTEX_GROUP local.get $da i32.const 2 i32.shl i32.add
      i32.const -1 i32.store offset=0

      ;; vertex group
      local.get $meta global.get $META_VERTEX_GROUPS_OFF i32.add i32.load offset=0
      local.tee $var13
      i32.const -1 i32.ne
      if
        local.get $var13 local.get $i i32.add
        local.set $var13
        local.get $var13 local.get $end i32.ge_u
        if global.get $MODEL_ERR_TRUNCATED return end
        global.get $MODEL_VERTEX_GROUP local.get $da i32.const 2 i32.shl i32.add
        local.get $payload local.get $var13 i32.add i32.load8_u
        i32.store offset=0
      end

      local.get $i i32.const 1 i32.add local.set $i
      br $vtx_loop
    )
    )

    ;; ── Decode faces ──
    local.get $meta global.get $META_DECODED_VERTEX_COUNT i32.add local.get $vertex_cnt i32.store offset=0
    global.get $S_DECODE_VERTEX_WRITE_BASE i32.load offset=0
    local.set $v
    global.get $S_LAST_DECODED_VERTEX_BASE local.get $v i32.store offset=0
    global.get $S_LAST_DECODED_VERTEX_COUNT local.get $vertex_cnt i32.store offset=0

    ;; face colors
    local.get $meta global.get $META_FACE_COLORS_OFF i32.add i32.load offset=0
    local.set $v

    i32.const 0 local.set $i
    (block $color_done
    (loop $color_loop
      local.get $i local.get $face_cnt i32.ge_u
      br_if $color_done

      local.get $v i32.const 2 i32.add
      local.get $end i32.gt_u
      if global.get $MODEL_ERR_TRUNCATED return end

      global.get $S_DECODE_FACE_WRITE_BASE i32.load offset=0
      local.get $i i32.add
      local.set $da

      global.get $MODEL_FACE_COLOR local.get $da i32.const 2 i32.shl i32.add
      local.get $payload local.get $v i32.add i32.load8_u
      i32.const 8 i32.shl
      local.get $payload local.get $v i32.const 1 i32.add i32.add i32.load8_u
      i32.or
      i32.store offset=0

      local.get $v i32.const 2 i32.add local.set $v
      local.get $i i32.const 1 i32.add local.set $i
      br $color_loop
    )
    )

    ;; face indices
    local.get $meta global.get $META_FACE_TYPES_OFF i32.add i32.load offset=0
    local.set $vtx_off
    local.get $meta global.get $META_FACE_INDICES_OFF i32.add i32.load offset=0
    local.set $f_idx_off

    i32.const 0 local.set $i
    i32.const 0 local.set $prev_a
    i32.const 0 local.set $prev_b
    i32.const 0 local.set $prev_c
    i32.const 0 local.set $prev_high

    (block $face_done
    (loop $face_loop
      local.get $i local.get $face_cnt i32.ge_u
      br_if $face_done

      local.get $vtx_off local.get $end i32.ge_u
      if global.get $MODEL_ERR_TRUNCATED return end

      local.get $payload local.get $vtx_off i32.add i32.load8_u
      local.set $f_colors_off
      local.get $vtx_off i32.const 1 i32.add local.set $vtx_off

      local.get $f_colors_off
      i32.const 1 i32.eq
      if
        local.get $payload local.get $len local.get $f_idx_off
        call $read_short_smart
        local.set $f_idx_off
        local.set $da
        local.get $da local.get $prev_high i32.add
        local.set $prev_a

        local.get $payload local.get $len local.get $f_idx_off
        call $read_short_smart
        local.set $f_idx_off
        local.set $da
        local.get $da local.get $prev_a i32.add
        local.set $prev_b

        local.get $payload local.get $len local.get $f_idx_off
        call $read_short_smart
        local.set $f_idx_off
        local.set $da
        local.get $da local.get $prev_b i32.add
        local.set $prev_c
        local.get $prev_c local.set $prev_high
      else
        local.get $f_colors_off
        i32.const 2 i32.eq
        if
          local.get $prev_c local.set $prev_b
          local.get $payload local.get $len local.get $f_idx_off
          call $read_short_smart
          local.set $f_idx_off
          local.set $da
          local.get $da local.get $prev_high i32.add
          local.set $prev_c
          local.get $prev_c local.set $prev_high
        else
          local.get $f_colors_off
          i32.const 3 i32.eq
          if
            local.get $prev_c local.set $prev_a
            local.get $payload local.get $len local.get $f_idx_off
            call $read_short_smart
            local.set $f_idx_off
            local.set $da
            local.get $da local.get $prev_high i32.add
            local.set $prev_c
            local.get $prev_c local.set $prev_high
          else
            local.get $f_colors_off
            i32.const 4 i32.eq
            if
              local.get $prev_a local.set $tmp
              local.get $prev_b local.set $prev_a
              local.get $tmp local.set $prev_b
              local.get $payload local.get $len local.get $f_idx_off
              call $read_short_smart
              local.set $f_idx_off
              local.set $da
              local.get $da local.get $prev_high i32.add
              local.set $prev_c
              local.get $prev_c local.set $prev_high
            else
              global.get $MODEL_ERR_TRUNCATED return
            end
          end
        end
      end

      ;; store_face
      local.get $prev_a local.get $vertex_cnt i32.ge_u
      if global.get $MODEL_ERR_TRUNCATED return end
      local.get $prev_b local.get $vertex_cnt i32.ge_u
      if global.get $MODEL_ERR_TRUNCATED return end
      local.get $prev_c local.get $vertex_cnt i32.ge_u
      if global.get $MODEL_ERR_TRUNCATED return end

      global.get $S_DECODE_FACE_WRITE_BASE i32.load offset=0
      local.get $i i32.add
      local.set $da

      global.get $MODEL_FACE_A local.get $da i32.const 2 i32.shl i32.add
      local.get $prev_a i32.store offset=0
      global.get $MODEL_FACE_B local.get $da i32.const 2 i32.shl i32.add
      local.get $prev_b i32.store offset=0
      global.get $MODEL_FACE_C local.get $da i32.const 2 i32.shl i32.add
      local.get $prev_c i32.store offset=0

      local.get $i i32.const 1 i32.add local.set $i
      br $face_loop
    )
    )

    ;; ── Success ──
    local.get $meta global.get $META_DECODED_FACE_COUNT i32.add local.get $face_cnt i32.store offset=0
    global.get $S_DECODE_FACE_WRITE_BASE i32.load offset=0
    local.set $v
    global.get $S_LAST_DECODED_FACE_BASE local.get $v i32.store offset=0
    global.get $S_LAST_DECODED_FACE_COUNT local.get $face_cnt i32.store offset=0
    global.get $MODEL_OK
  )

  ;; ═══════════════════════════════════════════════════════════════════════
  ;; model_registry_decode_payload
  ;; ═══════════════════════════════════════════════════════════════════════
  (func (export "model_registry_decode_payload")
        (param $model_id i32) (param $payload i32) (param $len i32) (param $meta i32) (result i32)
    (local $count i32) (local $asset_ptr i32) (local $v i32)

    local.get $payload i32.eqz if global.get $MODEL_ERR_BOUNDS return end
    local.get $meta i32.eqz if global.get $MODEL_ERR_BOUNDS return end

    global.get $S_ASSET_COUNT i32.load offset=0
    local.tee $count
    global.get $MODEL_MAX_ASSETS i32.ge_u
    if global.get $MODEL_ERR_BOUNDS return end

    global.get $S_REGISTRY_NEXT_VERTEX i32.load offset=0
    global.get $S_DECODE_VERTEX_WRITE_BASE i32.store offset=0
    global.get $S_REGISTRY_NEXT_FACE i32.load offset=0
    global.get $S_DECODE_FACE_WRITE_BASE i32.store offset=0

    local.get $payload local.get $len local.get $meta
    call $metadata_payload_with_bases
    local.tee $v
    if local.get $v return end

    global.get $S_ASSET_COUNT i32.load offset=0
    local.set $count
    global.get $MODEL_ASSETS
    local.get $count global.get $ASSET_SIZE i32.mul i32.add
    local.set $asset_ptr

    local.get $asset_ptr global.get $ASSET_MODEL_ID i32.add local.get $model_id i32.store offset=0

    local.get $meta global.get $META_DECODED_VERTEX_BASE i32.add i32.load offset=0
    local.get $asset_ptr global.get $ASSET_VERTEX_BASE i32.add i32.store offset=0

    local.get $meta global.get $META_DECODED_VERTEX_COUNT i32.add i32.load offset=0
    local.tee $v
    local.get $asset_ptr global.get $ASSET_VERTEX_COUNT i32.add i32.store offset=0

    local.get $asset_ptr global.get $ASSET_VERTEX_GROUP_BASE i32.add
    local.get $meta global.get $META_DECODED_VERTEX_BASE i32.add i32.load offset=0
    i32.store offset=0

    local.get $meta global.get $META_DECODED_VERTEX_BASE i32.add i32.load offset=0
    local.get $v i32.add
    global.get $S_REGISTRY_NEXT_VERTEX i32.store offset=0

    local.get $meta global.get $META_DECODED_FACE_BASE i32.add i32.load offset=0
    local.get $asset_ptr global.get $ASSET_FACE_BASE i32.add i32.store offset=0

    local.get $meta global.get $META_DECODED_FACE_COUNT i32.add i32.load offset=0
    local.tee $v
    local.get $asset_ptr global.get $ASSET_FACE_COUNT i32.add i32.store offset=0

    local.get $meta global.get $META_DECODED_FACE_BASE i32.add i32.load offset=0
    local.get $v i32.add
    global.get $S_REGISTRY_NEXT_FACE i32.store offset=0

    local.get $meta global.get $META_MIN_X i32.add i32.load offset=0
    local.get $asset_ptr global.get $ASSET_MIN_X i32.add i32.store offset=0
    local.get $meta global.get $META_MAX_X i32.add i32.load offset=0
    local.get $asset_ptr global.get $ASSET_MAX_X i32.add i32.store offset=0
    local.get $meta global.get $META_MIN_Y i32.add i32.load offset=0
    local.get $asset_ptr global.get $ASSET_MIN_Y i32.add i32.store offset=0
    local.get $meta global.get $META_MAX_Y i32.add i32.load offset=0
    local.get $asset_ptr global.get $ASSET_MAX_Y i32.add i32.store offset=0
    local.get $meta global.get $META_MIN_Z i32.add i32.load offset=0
    local.get $asset_ptr global.get $ASSET_MIN_Z i32.add i32.store offset=0
    local.get $meta global.get $META_MAX_Z i32.add i32.load offset=0
    local.get $asset_ptr global.get $ASSET_MAX_Z i32.add i32.store offset=0

    global.get $S_ASSET_COUNT
    global.get $S_ASSET_COUNT i32.load offset=0 i32.const 1 i32.add
    i32.store offset=0
    global.get $MODEL_OK
  )

  ;; ═══════════════════════════════════════════════════════════════════════
  ;; model_registry_lookup
  ;; ═══════════════════════════════════════════════════════════════════════
  (func (export "model_registry_lookup") (param $model_id i32) (result i32)
    (local $count i32) (local $i i32) (local $ap i32)
    global.get $S_ASSET_COUNT i32.load offset=0 local.set $count
    i32.const 0 local.set $i
    (loop $loop
      local.get $i local.get $count i32.lt_u
      if
        global.get $MODEL_ASSETS
        local.get $i global.get $ASSET_SIZE i32.mul i32.add
        local.tee $ap
        i32.load offset=0
        local.get $model_id i32.eq
        if local.get $ap return end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    )
    i32.const 0
  )

  ;; ═══════════════════════════════════════════════════════════════════════
  ;; model_asset_has_vertex_group
  ;; ═══════════════════════════════════════════════════════════════════════
  (func (export "model_asset_has_vertex_group")
        (param $asset_ptr i32) (param $group_id i32) (result i32)
    (local $count i32) (local $base i32) (local $i i32)
    local.get $asset_ptr i32.eqz if i32.const 0 return end
    local.get $group_id i32.const 0 i32.lt_s if i32.const 0 return end
    local.get $asset_ptr global.get $ASSET_VERTEX_COUNT i32.add i32.load offset=0
    local.set $count
    local.get $asset_ptr global.get $ASSET_VERTEX_GROUP_BASE i32.add i32.load offset=0
    local.set $base
    i32.const 0 local.set $i
    (loop $loop
      local.get $i local.get $count i32.lt_u
      if
        global.get $MODEL_VERTEX_GROUP
        local.get $base local.get $i i32.add i32.const 2 i32.shl i32.add
        i32.load offset=0
        local.get $group_id i32.eq
        if i32.const 1 return end
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
    )
    i32.const 0
  )

  ;; ═══════════════════════════════════════════════════════════════════════
  ;; Scalar accessors
  ;; ═══════════════════════════════════════════════════════════════════════
  (func (export "model_last_decoded_vertex_count") (result i32)
    global.get $S_LAST_DECODED_VERTEX_COUNT i32.load offset=0
  )
  (func (export "model_last_decoded_face_count") (result i32)
    global.get $S_LAST_DECODED_FACE_COUNT i32.load offset=0
  )
  (func (export "model_last_decoded_vertex_base") (result i32)
    global.get $S_LAST_DECODED_VERTEX_BASE i32.load offset=0
  )
  (func (export "model_last_decoded_face_base") (result i32)
    global.get $S_LAST_DECODED_FACE_BASE i32.load offset=0
  )
  (func (export "model_asset_count") (result i32)
    global.get $S_ASSET_COUNT i32.load offset=0
  )

  ;; ═══════════════════════════════════════════════════════════════════════
  ;; Render BSS
  ;; ═══════════════════════════════════════════════════════════════════════
  (global $R_DEPTH_BUF    i32 (i32.const 0x50000))
  (global $R_ANIM_COUNT   i32 (i32.const 0x450000))
  (global $R_ANIM_GROUPS  i32 (i32.const 0x450004))
  (global $R_ANIM_ROT_X   i32 (i32.const 0x450024))
  (global $R_ANIM_ROT_Y   i32 (i32.const 0x450044))
  (global $R_ANIM_ROT_Z   i32 (i32.const 0x450064))
  (global $RENDER_CAP     i32 (i32.const 8))
  (global $RENDER_MAX_PX  i32 (i32.const 1048576))

  (func (export "model_render_anim_clear")
    i32.const 0 global.get $R_ANIM_COUNT i32.store offset=0
  )

  (func (export "model_render_anim_add_group_yaw")
        (param $gid i32) (param $yaw i32) (result i32)
    (local $c i32)
    global.get $R_ANIM_COUNT i32.load offset=0 local.tee $c
    global.get $RENDER_CAP i32.ge_u if i32.const -1 return end
    global.get $R_ANIM_GROUPS local.get $c i32.const 2 i32.shl i32.add local.get $gid i32.store offset=0
    global.get $R_ANIM_ROT_X local.get $c i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    global.get $R_ANIM_ROT_Y local.get $c i32.const 2 i32.shl i32.add local.get $yaw i32.store offset=0
    global.get $R_ANIM_ROT_Z local.get $c i32.const 2 i32.shl i32.add i32.const 0 i32.store offset=0
    local.get $c i32.const 1 i32.add global.get $R_ANIM_COUNT i32.store offset=0
    local.get $c
  )

  (func (export "model_render_anim_add_group_rotation")
        (param $gid i32) (param $rx i32) (param $ry i32) (param $rz i32) (result i32)
    (local $c i32)
    global.get $R_ANIM_COUNT i32.load offset=0 local.tee $c
    global.get $RENDER_CAP i32.ge_u if i32.const -1 return end
    global.get $R_ANIM_GROUPS local.get $c i32.const 2 i32.shl i32.add local.get $gid i32.store offset=0
    global.get $R_ANIM_ROT_X local.get $c i32.const 2 i32.shl i32.add local.get $rx i32.store offset=0
    global.get $R_ANIM_ROT_Y local.get $c i32.const 2 i32.shl i32.add local.get $ry i32.store offset=0
    global.get $R_ANIM_ROT_Z local.get $c i32.const 2 i32.shl i32.add local.get $rz i32.store offset=0
    local.get $c i32.const 1 i32.add global.get $R_ANIM_COUNT i32.store offset=0
    local.get $c
  )

  (func $project_vertex
        (param $vi i32) (param $cx i32) (param $cy i32)
        (param $ss i32) (param $rot i32)
        (param $ay i32) (param $at i32)
        (result i32 i32)
    (local $x i32) (local $y i32) (local $z i32) (local $vg i32)
    (local $sx i32) (local $sy i32) (local $t i32) (local $k i32) (local $i i32)

    global.get $MODEL_VERTEX_X local.get $vi i32.const 2 i32.shl i32.add i32.load offset=0 local.set $x
    global.get $MODEL_VERTEX_Z local.get $vi i32.const 2 i32.shl i32.add i32.load offset=0 local.set $z
    global.get $MODEL_VERTEX_GROUP local.get $vi i32.const 2 i32.shl i32.add i32.load offset=0 local.set $vg
    global.get $MODEL_VERTEX_Y local.get $vi i32.const 2 i32.shl i32.add i32.load offset=0 local.set $sy

    local.get $rot i32.const 1 i32.eq
    if
      local.get $x local.set $t
      local.get $z local.set $x
      local.get $t local.set $z
      i32.const 0 local.get $z i32.sub local.set $z
    else
      local.get $rot i32.const 2 i32.eq
      if
        i32.const 0 local.get $x i32.sub local.set $x
        i32.const 0 local.get $z i32.sub local.set $z
      else
        local.get $rot i32.const 3 i32.eq
        if
          local.get $x local.set $t
          local.get $z local.set $x
          local.get $t local.set $z
          i32.const 0 local.get $x i32.sub local.set $x
        end
      end
    end

    global.get $R_ANIM_COUNT i32.load offset=0
    if
      i32.const 0 local.set $k
      (block $no_batch
      (loop $bl
        local.get $k local.get $t i32.lt_u  ;; reuse t as anim_count (was loaded above)
        if
          global.get $R_ANIM_GROUPS local.get $k i32.const 2 i32.shl i32.add i32.load offset=0
          local.tee $t
          i32.const -1 i32.eq if br $no_batch end
          local.get $t local.get $vg i32.eq if br $no_batch end
          local.get $k i32.const 1 i32.add local.set $k
          br $bl
        end
      ))
      ;; batch apply
      local.get $t i32.const -1 i32.eq local.set $t
      (block $b_apply
        global.get $R_ANIM_ROT_X local.get $k i32.const 2 i32.shl i32.add i32.load offset=0 local.tee $t
        if
          local.get $t local.get $z i32.mul i32.const 7 i32.shr_s local.set $i
          local.get $sy local.get $t local.get $i i32.mul i32.const 7 i32.shr_s i32.sub local.set $z
          local.get $i local.get $sy i32.add local.set $sy
        end
        global.get $R_ANIM_ROT_Y local.get $k i32.const 2 i32.shl i32.add i32.load offset=0 local.tee $t
        if
          local.get $t local.get $z i32.mul i32.const 7 i32.shr_s local.set $i
          local.get $x local.get $t local.get $i i32.mul i32.const 7 i32.shr_s i32.sub local.set $z
          local.get $i local.get $x i32.add local.set $x
        end
        global.get $R_ANIM_ROT_Z local.get $k i32.const 2 i32.shl i32.add i32.load offset=0 local.tee $t
        if
          local.get $t local.get $sy i32.mul i32.const 7 i32.shr_s local.set $i
          local.get $x local.get $t local.get $i i32.mul i32.const 7 i32.shr_s i32.sub local.set $sy
          local.get $i local.get $x i32.add local.set $x
        end
      )
    else
      local.get $ay
      if
        local.get $at i32.const -1 i32.eq if else
          local.get $at local.get $vg i32.ne if
            i32.const 0 local.set $ay
          end
        end
      end
      local.get $ay
      if
        local.get $ay local.get $z i32.mul i32.const 7 i32.shr_s local.set $t
        local.get $x local.get $ay local.get $t i32.mul i32.const 7 i32.shr_s i32.sub local.set $z
        local.get $t local.get $x i32.add local.set $x
      end
    end

    local.get $x local.get $ss i32.shr_s local.get $cx i32.add local.set $sx
    local.get $z local.get $ss i32.shr_s local.get $cy i32.add local.set $sy
    local.get $sx local.get $sy
  )

  (func (export "model_project_faces_ortho_xrgb")
        (param $asset i32) (param $dst i32)
        (param $width i32) (param $height i32) (param $stride i32)
        (param $cent_x i32) (param $cent_y i32)
        (param $ss i32) (param $rot i32) (param $db i32)
        (param $ay i32) (param $at i32)
        (result i32)
    (local $spx i32) (local $dpx i32) (local $i i32)
    (local $fc i32) (local $fb i32) (local $vb i32) (local $vc i32)
    (local $drawn i32) (local $va i32) (local $vb_idx i32) (local $vc_idx i32)
    (local $x0 i32) (local $y0 i32) (local $x1 i32) (local $y1 i32) (local $x2 i32) (local $y2 i32)
    (local $area i32) (local $cr i32) (local $ce i32) (local $fd i32)
    (local $mnx i32) (local $mxx i32) (local $mny i32) (local $mxy i32)
    (local $fy i32) (local $fx i32)
    (local $w0 i32) (local $w1 i32) (local $w2 i32)

    local.get $asset i32.eqz if i32.const 0 return end
    local.get $dst i32.eqz if i32.const 0 return end
    local.get $width i32.eqz if i32.const 0 return end
    local.get $height i32.eqz if i32.const 0 return end
    local.get $stride i32.eqz if i32.const 0 return end

    local.get $stride i32.const 2 i32.shr_u local.set $spx
    local.get $spx local.get $height i32.mul local.tee $dpx
    global.get $RENDER_MAX_PX i32.gt_u if i32.const 0 return end

    i32.const 0 local.set $i
    (block $dc
    (loop $dl
      local.get $i local.get $dpx i32.ge_u br_if $dc
      global.get $R_DEPTH_BUF local.get $i i32.const 2 i32.shl i32.add i32.const 0x80000000 i32.store offset=0
      local.get $i i32.const 1 i32.add local.set $i br $dl
    ))

    local.get $ss i32.const 30 i32.gt_u if i32.const 30 local.set $ss end
    local.get $rot i32.const 3 i32.and local.set $rot

    local.get $asset global.get $ASSET_FACE_COUNT i32.add i32.load offset=0 local.set $fc
    local.get $asset global.get $ASSET_FACE_BASE i32.add i32.load offset=0 local.set $fb
    local.get $asset global.get $ASSET_VERTEX_BASE i32.add i32.load offset=0 local.set $vb
    local.get $asset global.get $ASSET_VERTEX_COUNT i32.add i32.load offset=0 local.set $vc

    i32.const 0 local.set $i
    i32.const 0 local.set $drawn

    (block $fdone
    (loop $fl
      local.get $i local.get $fc i32.ge_u br_if $fdone

      global.get $MODEL_FACE_A local.get $fb local.get $i i32.add i32.const 2 i32.shl i32.add i32.load offset=0
      local.get $vb i32.add local.set $va
      global.get $MODEL_FACE_B local.get $fb local.get $i i32.add i32.const 2 i32.shl i32.add i32.load offset=0
      local.get $vb i32.add local.set $vb_idx
      global.get $MODEL_FACE_C local.get $fb local.get $i i32.add i32.const 2 i32.shl i32.add i32.load offset=0
      local.get $vb i32.add local.set $vc_idx

      local.get $va local.get $vc i32.ge_u if br $fl end
      local.get $vb_idx local.get $vc i32.ge_u if br $fl end
      local.get $vc_idx local.get $vc i32.ge_u if br $fl end

      local.get $va local.get $cent_x local.get $cent_y local.get $ss local.get $rot local.get $ay local.get $at call $project_vertex
      local.set $y0 local.set $x0
      local.get $vb_idx local.get $cent_x local.get $cent_y local.get $ss local.get $rot local.get $ay local.get $at call $project_vertex
      local.set $y1 local.set $x1
      local.get $vc_idx local.get $cent_x local.get $cent_y local.get $ss local.get $rot local.get $ay local.get $at call $project_vertex
      local.set $y2 local.set $x2

      ;; signed area
      local.get $x1 local.get $x0 i32.sub
      local.get $y2 local.get $y0 i32.sub
      i32.mul
      local.get $x2 local.get $x0 i32.sub
      local.get $y1 local.get $y0 i32.sub
      i32.mul
      i32.sub
      local.tee $area
      i32.eqz if br $fl end

      ;; face color
      global.get $MODEL_FACE_COLOR
      local.get $fb local.get $i i32.add i32.const 2 i32.shl i32.add
      i32.load offset=0
      local.set $cr

      ;; expand 5-5-5 to 8-8-8
      local.get $cr i32.const 0x1f i32.and i32.const 3 i32.shl
      i32.const 16 i32.shl
      local.get $cr i32.const 5 i32.shr_u i32.const 0x1f i32.and i32.const 3 i32.shl
      i32.const 8 i32.shl
      i32.or
      local.get $cr i32.const 10 i32.shr_u i32.const 0x1f i32.and i32.const 3 i32.shl
      i32.or
      i32.const 0x00202020 i32.or
      local.set $ce

      ;; face depth = avg y + bias
      local.get $va global.get $MODEL_VERTEX_Y local.get $va i32.const 2 i32.shl i32.add i32.load offset=0
      local.get $vb_idx global.get $MODEL_VERTEX_Y local.get $vb_idx i32.const 2 i32.shl i32.add i32.load offset=0
      i32.add
      local.get $vc_idx global.get $MODEL_VERTEX_Y local.get $vc_idx i32.const 2 i32.shl i32.add i32.load offset=0
      i32.add
      local.set $fd
      ;; fd = (fd + 1) / 3  -- integer division, rounding toward -inf
      local.get $fd i32.const 1 i32.add
      local.get $fd i32.const 0 i32.lt_s
      if (result i32)
        local.get $fd i32.const 1 i32.sub
      else
        local.get $fd i32.const 1 i32.add
      end
      i32.const 3 i32.div_s
      local.get $db i32.add
      local.set $fd

      ;; compute bounds
      local.get $x0 local.get $x1 local.get $x2 call $min3
      local.set $mnx
      local.get $x0 local.get $x1 local.get $x2 call $max3
      local.set $mxx
      local.get $y0 local.get $y1 local.get $y2 call $min3
      local.set $mny
      local.get $y0 local.get $y1 local.get $y2 call $max3
      local.set $mxy

      ;; clamp to viewport
      local.get $mnx i32.const 0 i32.lt_s if i32.const 0 local.set $mnx end
      local.get $mny i32.const 0 i32.lt_s if i32.const 0 local.set $mny end
      local.get $mnx local.get $width i32.ge_s if br $fl end
      local.get $mny local.get $height i32.ge_s if br $fl end
      local.get $mxx i32.const 0 i32.lt_s if br $fl end
      local.get $mxy i32.const 0 i32.lt_s if br $fl end
      local.get $width i32.const 1 i32.sub local.tee $width
      local.get $mxx i32.gt_s if local.get $width local.set $mxx end
      local.get $height i32.const 1 i32.sub local.tee $height
      local.get $mxy i32.gt_s if local.get $height local.set $mxy end

      ;; fill y loop
      local.get $mny local.set $fy
      (block $fy_done
      (loop $fy_loop
        local.get $fy local.get $mxy i32.gt_s br_if $fy_done

        local.get $mnx local.set $fx
        (block $fx_done
        (loop $fx_loop
          local.get $fx local.get $mxx i32.gt_s br_if $fx_done

          ;; barycentric weights
          local.get $fx local.get $x1 i32.sub
          local.get $y2 local.get $y1 i32.sub
          i32.mul
          local.get $fy local.get $y1 i32.sub
          local.get $x2 local.get $x1 i32.sub
          i32.mul
          i32.sub
          local.set $w0

          local.get $fx local.get $x2 i32.sub
          local.get $y0 local.get $y2 i32.sub
          i32.mul
          local.get $fy local.get $y2 i32.sub
          local.get $x0 local.get $x2 i32.sub
          i32.mul
          i32.sub
          local.set $w1

          local.get $fx local.get $x0 i32.sub
          local.get $y1 local.get $y0 i32.sub
          i32.mul
          local.get $fy local.get $y0 i32.sub
          local.get $x1 local.get $x0 i32.sub
          i32.mul
          i32.sub
          local.set $w2

          ;; inside test
          local.get $area i32.const 0 i32.lt_s
          if
            ;; negative area: all weights >= 0
            local.get $w0 i32.const 0 i32.lt_s if br $fx_loop end
            local.get $w1 i32.const 0 i32.lt_s if br $fx_loop end
            local.get $w2 i32.const 0 i32.lt_s if br $fx_loop end
          else
            ;; positive area: all weights <= 0
            local.get $w0 i32.const 0 i32.gt_s if br $fx_loop end
            local.get $w1 i32.const 0 i32.gt_s if br $fx_loop end
            local.get $w2 i32.const 0 i32.gt_s if br $fx_loop end
          end

          ;; depth test
          local.get $fy local.get $spx i32.mul local.get $fx i32.add
          local.set $w0
          global.get $R_DEPTH_BUF local.get $w0 i32.const 2 i32.shl i32.add i32.load offset=0
          local.get $fd i32.lt_s
          if br $fx_loop end

          ;; write pixel
          global.get $R_DEPTH_BUF local.get $w0 i32.const 2 i32.shl i32.add local.get $fd i32.store offset=0
          local.get $dst local.get $w0 i32.const 2 i32.shl i32.add local.get $ce i32.store offset=0
          local.get $drawn i32.const 1 i32.add local.set $drawn

          local.get $fx i32.const 1 i32.add local.set $fx
          br $fx_loop
        ))
        local.get $fy i32.const 1 i32.add local.set $fy
        br $fy_loop
      ))
      local.get $i i32.const 1 i32.add local.set $i
      br $fl
    ))
    local.get $drawn
  )

  ;; helper: min3(a,b,c)
  (func $min3 (param $a i32) (param $b i32) (param $c i32) (result i32)
    local.get $a local.get $b local.get $c call $min2 call $min2
  )
  ;; helper: max3(a,b,c)
  (func $max3 (param $a i32) (param $b i32) (param $c i32) (result i32)
    local.get $a local.get $b local.get $c call $max2 call $max2
  )
  (func $min2 (param $a i32) (param $b i32) (result i32)
    local.get $a local.get $b i32.lt_s if local.get $a return end local.get $b
  )
  (func $max2 (param $a i32) (param $b i32) (result i32)
    local.get $a local.get $b i32.gt_s if local.get $a return end local.get $b
  )
)
