(module
  (import "edgerun-core" "memory" (memory 1))
  (func (export "proto_standard_id") (result i32) i32.const 710002)

  ;; ── Error codes ──
  (global $DEF_OK       i32 (i32.const 0))
  (global $DEF_ERR_BOUNDS   i32 (i32.const 1))
  (global $DEF_ERR_OPCODE   i32 (i32.const 2))
  (global $DEF_ERR_OVERFLOW i32 (i32.const 3))

  ;; ── ID sentinel ──
  (global $DEF_ID_NONE  i32 (i32.const -1))

  ;; ── Object clip flags ──
  (global $DEF_OBJECT_CLIP_SOLID        i32 (i32.const 1))
  (global $DEF_OBJECT_CLIP_PROJECTILE   i32 (i32.const 2))
  (global $DEF_OBJECT_CLIP_MODEL        i32 (i32.const 4))
  (global $DEF_OBJECT_CLIP_OBSTRUCTS    i32 (i32.const 8))
  (global $DEF_OBJECT_CLIP_HOLLOW       i32 (i32.const 16))

  ;; ── Object flags ──
  (global $DEF_OBJECT_FLAG_MIRRORED             i32 (i32.const 1))
  (global $DEF_OBJECT_FLAG_CASTS_SHADOW_FALSE   i32 (i32.const 2))
  (global $DEF_OBJECT_FLAG_MODEL_ARRAY_TRUNC    i32 (i32.const 4))
  (global $DEF_OBJECT_FLAG_TRANSFORM_ARRAY_TRUNC i32 (i32.const 8))

  ;; ── Object model type ──
  (global $DEF_OBJECT_MODEL_TYPE_ANY i32 (i32.const -1))

  ;; ── Scratch context for payload wrappers ──
  (global $SCRATCH_CTX i32 (i32.const 0))

  ;; ── Animation frame registry static memory addresses ──
  (global $REG_KEY           i32 (i32.const 1024))
  (global $REG_ACTIVE        i32 (i32.const 1028))
  (global $REG_FRAME         i32 (i32.const 1032))
  (global $REG_SKEL          i32 (i32.const 1088))
  (global $REG_INDICES       i32 (i32.const 1200))
  (global $REG_X             i32 (i32.const 1328))
  (global $REG_Y             i32 (i32.const 1456))
  (global $REG_Z             i32 (i32.const 1584))
  (global $REG_SKEL_TYPES    i32 (i32.const 1712))
  (global $REG_SKEL_COUNTS   i32 (i32.const 1840))
  (global $REG_SKEL_OFFSETS  i32 (i32.const 1968))
  (global $REG_SKEL_LABELS   i32 (i32.const 2096))
  (global $REG_DEC_FRAME     i32 (i32.const 2352))
  (global $REG_DEC_SKEL      i32 (i32.const 2408))
  (global $REG_DEC_INDICES   i32 (i32.const 2528))
  (global $REG_DEC_X         i32 (i32.const 2656))
  (global $REG_DEC_Y         i32 (i32.const 2784))
  (global $REG_DEC_Z         i32 (i32.const 2912))
  (global $REG_DEC_SKEL_TYPES  i32 (i32.const 3040))
  (global $REG_DEC_SKEL_COUNTS i32 (i32.const 3168))
  (global $REG_DEC_SKEL_OFFS   i32 (i32.const 3296))
  (global $REG_DEC_SKEL_LABELS i32 (i32.const 3424))
  (global $ANIM_FRAME_REGISTRY_TRANSFORM_CAP i32 (i32.const 32))
  (global $ANIM_FRAME_REGISTRY_LABEL_CAP     i32 (i32.const 64))

  ;; ── Struct: def_reader_ctx offsets ──
  (global $CTX_POS i32 (i32.const 0))
  (global $CTX_END i32 (i32.const 8))
  (global $CTX_ERR i32 (i32.const 16))

  ;; ── Struct: def_varbit ──
  (global $VB_VARP     i32 (i32.const 0))
  (global $VB_LOW_BIT  i32 (i32.const 4))
  (global $VB_HIGH_BIT i32 (i32.const 5))

  ;; ── Struct: def_inventory ──
  (global $INV_SIZE i32 (i32.const 0))

  ;; ── Struct: def_object_core ──
  (global $OBJ_NAME_PTR         i32 (i32.const 0))
  (global $OBJ_NAME_LEN         i32 (i32.const 8))
  (global $OBJ_MODEL_COUNT      i32 (i32.const 12))
  (global $OBJ_MODEL_IDS_PTR    i32 (i32.const 16))
  (global $OBJ_MODEL_TYPES_PTR  i32 (i32.const 24))
  (global $OBJ_SIZE_X           i32 (i32.const 32))
  (global $OBJ_SIZE_Y           i32 (i32.const 36))
  (global $OBJ_INTERACT_TYPE    i32 (i32.const 40))
  (global $OBJ_BLOCK_PROJECTILE i32 (i32.const 44))
  (global $OBJ_ANIMATION_ID     i32 (i32.const 48))
  (global $OBJ_ACTION_PTRS      i32 (i32.const 56))
  (global $OBJ_VARBIT_ID        i32 (i32.const 64))
  (global $OBJ_VARP_ID          i32 (i32.const 68))
  (global $OBJ_TRANSFORM_COUNT  i32 (i32.const 72))
  (global $OBJ_TRANSFORM_IDS_PTR i32 (i32.const 80))
  (global $OBJ_AMBIENT_SOUND_ID  i32 (i32.const 88))
  (global $OBJ_PARAMS_PTR       i32 (i32.const 96))
  (global $OBJ_OBJECT_ID        i32 (i32.const 100))
  (global $OBJ_MODEL_CAPACITY   i32 (i32.const 104))
  (global $OBJ_MODEL_STORED_COUNT i32 (i32.const 108))
  (global $OBJ_TRANSFORM_CAPACITY i32 (i32.const 112))
  (global $OBJ_TRANSFORM_STORED_COUNT i32 (i32.const 116))
  (global $OBJ_CLIP_FLAGS       i32 (i32.const 120))
  (global $OBJ_FLAGS            i32 (i32.const 124))
  (global $OBJ_BLOCK_WALK       i32 (i32.const 128))

  ;; ── Struct: def_npc_core ──
  (global $NPC_NAME_PTR          i32 (i32.const 0))
  (global $NPC_NAME_LEN          i32 (i32.const 8))
  (global $NPC_MODEL_COUNT       i32 (i32.const 12))
  (global $NPC_MODEL_IDS_PTR     i32 (i32.const 16))
  (global $NPC_SIZE              i32 (i32.const 24))
  (global $NPC_COMBAT_LEVEL      i32 (i32.const 28))
  (global $NPC_IDLE_SEQ          i32 (i32.const 32))
  (global $NPC_WALK_SEQ          i32 (i32.const 36))
  (global $NPC_ACTION_PTRS       i32 (i32.const 40))
  (global $NPC_VARBIT_ID         i32 (i32.const 48))
  (global $NPC_VARP_ID           i32 (i32.const 52))
  (global $NPC_TRANSFORM_COUNT   i32 (i32.const 56))
  (global $NPC_TRANSFORM_IDS_PTR i32 (i32.const 64))
  (global $NPC_PARAMS_PTR        i32 (i32.const 72))

  ;; ── Struct: def_item_core ──
  (global $ITEM_NAME_PTR            i32 (i32.const 0))
  (global $ITEM_NAME_LEN            i32 (i32.const 8))
  (global $ITEM_INVENTORY_MODEL     i32 (i32.const 12))
  (global $ITEM_ZOOM_2D             i32 (i32.const 16))
  (global $ITEM_XAN_2D              i32 (i32.const 20))
  (global $ITEM_YAN_2D              i32 (i32.const 24))
  (global $ITEM_ZAN_2D              i32 (i32.const 28))
  (global $ITEM_COST                i32 (i32.const 32))
  (global $ITEM_STACKABLE           i32 (i32.const 36))
  (global $ITEM_MEMBERS             i32 (i32.const 40))
  (global $ITEM_GROUND_ACTIONS      i32 (i32.const 48))
  (global $ITEM_INVENTORY_ACTIONS   i32 (i32.const 56))
  (global $ITEM_NOTE_ID             i32 (i32.const 60))
  (global $ITEM_NOTE_TEMPLATE_ID    i32 (i32.const 64))
  (global $ITEM_PARAMS_PTR          i32 (i32.const 72))

  ;; ── Struct: def_sequence_core ──
  (global $SEQ_FRAME_COUNT        i32 (i32.const 0))
  (global $SEQ_FRAME_LENGTHS_PTR  i32 (i32.const 8))
  (global $SEQ_FRAME_IDS_PTR      i32 (i32.const 16))
  (global $SEQ_FRAME_STEP         i32 (i32.const 24))
  (global $SEQ_FORCED_PRIORITY    i32 (i32.const 28))
  (global $SEQ_LEFT_HAND_ITEM     i32 (i32.const 32))
  (global $SEQ_RIGHT_HAND_ITEM    i32 (i32.const 36))
  (global $SEQ_SOUND_MAP_PTR      i32 (i32.const 40))
  (global $SEQ_FRAME_CAPACITY     i32 (i32.const 48))
  (global $SEQ_FRAME_STORED_COUNT i32 (i32.const 52))
  (global $SEQ_SOUND_COUNT        i32 (i32.const 56))

  ;; ── Struct: anim_skeleton_core ──
  (global $SKEL_TRANSFORM_COUNT       i32 (i32.const 0))
  (global $SKEL_TRANSFORM_TYPES_PTR   i32 (i32.const 8))
  (global $SKEL_LABEL_COUNTS_PTR      i32 (i32.const 16))
  (global $SKEL_LABEL_OFFSETS_PTR     i32 (i32.const 24))
  (global $SKEL_LABELS_PTR            i32 (i32.const 32))
  (global $SKEL_TRANSFORM_CAPACITY    i32 (i32.const 40))
  (global $SKEL_LABEL_CAPACITY        i32 (i32.const 44))
  (global $SKEL_STORED_COUNT          i32 (i32.const 48))
  (global $SKEL_TOTAL_LABEL_COUNT     i32 (i32.const 52))

  ;; ── Struct: anim_frame_core ──
  (global $FRAME_SKELETON_ID          i32 (i32.const 0))
  (global $FRAME_SLOT_COUNT           i32 (i32.const 4))
  (global $FRAME_TRANSFORM_COUNT      i32 (i32.const 8))
  (global $FRAME_TRANSFORM_INDICES_PTR i32 (i32.const 16))
  (global $FRAME_TRANSFORM_X_PTR      i32 (i32.const 24))
  (global $FRAME_TRANSFORM_Y_PTR      i32 (i32.const 32))
  (global $FRAME_TRANSFORM_Z_PTR      i32 (i32.const 40))
  (global $FRAME_TRANSFORM_CAPACITY   i32 (i32.const 48))
  (global $FRAME_HAS_ALPHA            i32 (i32.const 52))

  ;; ══════════════════════════════════════════════════════════════════
  ;; Internal reader helpers
  ;; ══════════════════════════════════════════════════════════════════

  (func $read_u8 (param $ctx i32) (result i32)
    (local $pos i32)
    local.get $ctx i32.load offset=0 local.tee $pos
    local.get $ctx i32.load offset=8
    i32.ge_u
    if
      local.get $ctx global.get $CTX_ERR i32.add global.get $DEF_ERR_BOUNDS i32.store
      i32.const 0 return
    end
    local.get $pos i32.load8_u
    local.get $ctx local.get $pos i32.const 1 i32.add i32.store offset=0
  )

  (func $read_u16be (param $ctx i32) (result i32)
    (local $pos i32)
    local.get $ctx i32.load offset=0 local.tee $pos
    i32.const 2 i32.add
    local.get $ctx i32.load offset=8
    i32.gt_u
    if
      local.get $ctx global.get $CTX_ERR i32.add global.get $DEF_ERR_BOUNDS i32.store
      i32.const 0 return
    end
    local.get $pos i32.load8_u i32.const 8 i32.shl
    local.get $pos i32.const 1 i32.add i32.load8_u
    i32.or
    local.get $ctx local.get $pos i32.const 2 i32.add i32.store offset=0
  )

  (func $read_u24be (param $ctx i32) (result i32)
    (local $pos i32)
    local.get $ctx i32.load offset=0 local.tee $pos
    i32.const 3 i32.add
    local.get $ctx i32.load offset=8
    i32.gt_u
    if
      local.get $ctx global.get $CTX_ERR i32.add global.get $DEF_ERR_BOUNDS i32.store
      i32.const 0 return
    end
    local.get $pos i32.load8_u i32.const 16 i32.shl
    local.get $pos i32.const 1 i32.add i32.load8_u i32.const 8 i32.shl i32.or
    local.get $pos i32.const 2 i32.add i32.load8_u i32.or
    local.get $ctx local.get $pos i32.const 3 i32.add i32.store offset=0
  )

  (func $read_i32be (param $ctx i32) (result i32)
    (local $pos i32)
    local.get $ctx i32.load offset=0 local.tee $pos
    i32.const 4 i32.add
    local.get $ctx i32.load offset=8
    i32.gt_u
    if
      local.get $ctx global.get $CTX_ERR i32.add global.get $DEF_ERR_BOUNDS i32.store
      i32.const 0 return
    end
    local.get $pos i32.load8_u i32.const 24 i32.shl
    local.get $pos i32.const 1 i32.add i32.load8_u i32.const 16 i32.shl i32.or
    local.get $pos i32.const 2 i32.add i32.load8_u i32.const 8 i32.shl i32.or
    local.get $pos i32.const 3 i32.add i32.load8_u i32.or
    local.get $ctx local.get $pos i32.const 4 i32.add i32.store offset=0
  )

  (func $read_nullable_large_smart (param $ctx i32) (result i32)
    (local $b i32)
    local.get $ctx call $read_u8
    local.tee $b
    i32.const 0x80 i32.and
    if
      local.get $ctx call $read_i32be
      i32.const 0x7fffffff i32.and
      return
    end
    local.get $b i32.const 8 i32.shl
    local.get $ctx call $read_u8
    i32.or
    local.tee $b
    i32.const 65535 i32.eq
    if global.get $DEF_ID_NONE return end
    local.get $b
  )

  (func $skip_string (param $ctx i32) (result i32)
    (local $cur i32)
    local.get $ctx i32.load offset=0
    local.set $cur
    block $found
      loop $continue
        local.get $cur
        local.get $ctx i32.load offset=8
        i32.ge_u
        if
          local.get $ctx global.get $CTX_ERR i32.add global.get $DEF_ERR_BOUNDS i32.store
          i32.const 0 return
        end
        local.get $cur i32.load8_u
        i32.eqz
        if br $found end
        local.get $cur i32.const 1 i32.add local.set $cur
        br $continue
      end
      unreachable
    end
    local.get $ctx local.get $cur i32.const 1 i32.add i32.store offset=0
    global.get $DEF_OK
  )

  (func $skip_bytes (param $ctx i32) (param $n i32) (result i32)
    (local $new_pos i32)
    local.get $ctx i32.load offset=0
    local.get $n i32.add
    local.tee $new_pos
    local.get $ctx i32.load offset=8
    i32.gt_u
    if
      local.get $ctx global.get $CTX_ERR i32.add global.get $DEF_ERR_BOUNDS i32.store
      global.get $DEF_ERR_BOUNDS return
    end
    local.get $ctx local.get $new_pos i32.store offset=0
    global.get $DEF_OK
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Exported primitives
  ;; ══════════════════════════════════════════════════════════════════

  (func $def_reader_init (export "def_reader_init") (param $ctx i32) (param $ptr i32) (param $len i32) (result i32)
    local.get $ctx i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $ctx local.get $ptr i32.store offset=0
    local.get $ctx local.get $ptr local.get $len i32.add i32.store offset=8
    local.get $ctx global.get $CTX_ERR i32.add global.get $DEF_OK i32.store
    global.get $DEF_OK
  )

  (func (export "def_reader_fail_bounds") (param $ctx i32) (result i32)
    local.get $ctx global.get $CTX_ERR i32.add global.get $DEF_ERR_BOUNDS i32.store
    global.get $DEF_ERR_BOUNDS
  )

  (func $def_reader_fail_opcode (export "def_reader_fail_opcode") (param $ctx i32) (result i32)
    local.get $ctx global.get $CTX_ERR i32.add global.get $DEF_ERR_OPCODE i32.store
    global.get $DEF_ERR_OPCODE
  )

  (func (export "def_read_u8") (param $ctx i32) (result i32)
    local.get $ctx call $read_u8
  )

  (func (export "def_read_i8") (param $ctx i32) (result i32)
    local.get $ctx call $read_u8
    i32.extend8_s
  )

  (func (export "def_read_u16be") (param $ctx i32) (result i32)
    local.get $ctx call $read_u16be
  )

  (func (export "def_read_i16be") (param $ctx i32) (result i32)
    local.get $ctx call $read_u16be
    i32.extend16_s
  )

  (func (export "def_read_u24be") (param $ctx i32) (result i32)
    local.get $ctx call $read_u24be
  )

  (func (export "def_read_i24be") (param $ctx i32) (result i32)
    (local $v i32)
    local.get $ctx call $read_u24be
    local.tee $v
    i32.const 0x00800000 i32.and
    if local.get $v i32.const 0xff000000 i32.or return end
    local.get $v
  )

  (func (export "def_read_i32be") (param $ctx i32) (result i32)
    local.get $ctx call $read_i32be
  )

  (func (export "def_read_nullable_large_smart") (param $ctx i32) (result i32)
    local.get $ctx call $read_nullable_large_smart
  )

  (func (export "def_read_varint") (param $ctx i32) (result i32)
    (local $val i32) (local $b i32)
    loop $continue
      local.get $ctx call $read_u8
      local.tee $b
      i32.const 0x80 i32.and
      if
        local.get $val
        local.get $b i32.const 0x7f i32.and
        i32.or
        i32.const 7 i32.shl
        local.set $val
        br $continue
      end
    end
    local.get $val
    local.get $b
    i32.or
  )

  (func $def_read_string_view (export "def_read_string_view") (param $ctx i32) (param $out_ptr i32) (param $out_len i32) (result i32)
    (local $start i32) (local $cur i32)
    local.get $ctx i32.load offset=0
    local.tee $start
    local.set $cur
    block $scan
      loop $continue
        local.get $cur
        local.get $ctx i32.load offset=8
        i32.ge_u
        if
          local.get $ctx global.get $CTX_ERR i32.add global.get $DEF_ERR_BOUNDS i32.store
          i32.const 0 return
        end
        local.get $cur i32.load8_u
        i32.eqz
        if br $scan end
        local.get $cur i32.const 1 i32.add local.set $cur
        br $continue
      end
      unreachable
    end
    local.get $out_ptr local.get $start i32.store offset=0
    local.get $cur local.get $start i32.sub
    local.tee $start
    local.get $out_len i32.store offset=0
    local.get $ctx local.get $cur i32.const 1 i32.add i32.store offset=0
    local.get $start
  )

  (func (export "def_skip_string") (param $ctx i32) (result i32)
    local.get $ctx call $skip_string
  )

  (func (export "def_skip_bytes") (param $ctx i32) (param $n i32) (result i32)
    local.get $ctx local.get $n call $skip_bytes
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Simple context decoders
  ;; ══════════════════════════════════════════════════════════════════

  (func $def_decode_params_skip (export "def_decode_params_skip") (param $ctx i32) (result i32)
    (local $count i32) (local $type i32)
    local.get $ctx call $read_u8
    local.tee $count
    i32.eqz
    if global.get $DEF_OK return end
    block $done
      loop $continue
        local.get $count i32.eqz
        if br $done end
        local.get $ctx call $read_u8
        local.set $type
        local.get $ctx call $read_u24be
        drop
        local.get $type
        i32.const 1 i32.eq
        if
          local.get $ctx call $skip_string
          drop
        else
          local.get $type i32.const 2 i32.eq
          if
            local.get $ctx i32.const 8 call $skip_bytes
            drop
          else
            local.get $ctx call $read_i32be
            drop
          end
        end
        local.get $count i32.const 1 i32.sub
        local.set $count
        br $continue
      end
      unreachable
    end
    global.get $DEF_OK
  )

  (func $def_decode_varbit_ctx (export "def_decode_varbit_ctx") (param $ctx i32) (param $vb i32) (result i32)
    (local $opcode i32)
    local.get $vb global.get $DEF_ID_NONE i32.store offset=0
    local.get $vb i32.const 0 i32.store8 offset=4
    local.get $vb i32.const 0 i32.store8 offset=5
    loop $continue
      local.get $ctx call $read_u8
      local.tee $opcode
      i32.eqz if global.get $DEF_OK return end
      local.get $opcode
      i32.const 1 i32.ne
      if
        local.get $ctx call $def_reader_fail_opcode
        return
      end
      local.get $ctx call $read_u16be
      local.get $vb i32.store offset=0
      local.get $ctx call $read_u8
      local.get $vb i32.store8 offset=4
      local.get $ctx call $read_u8
      local.get $vb i32.store8 offset=5
      br $continue
    end
    unreachable
  )

  (func $def_decode_varbit_payload (export "def_decode_varbit_payload") (param $ptr i32) (param $len i32) (param $vb i32) (result i32)
    local.get $vb i32.eqz if global.get $DEF_ERR_BOUNDS return end
    global.get $SCRATCH_CTX
    local.get $ptr local.get $len call $def_reader_init
    if global.get $DEF_ERR_BOUNDS return end
    global.get $SCRATCH_CTX
    local.get $vb
    call $def_decode_varbit_ctx
  )

  (func $def_decode_inventory_ctx (export "def_decode_inventory_ctx") (param $ctx i32) (param $inv i32) (result i32)
    (local $opcode i32)
    local.get $inv i32.const 0 i32.store offset=0
    loop $continue
      local.get $ctx call $read_u8
      local.tee $opcode
      i32.eqz if global.get $DEF_OK return end
      local.get $opcode
      i32.const 2 i32.ne
      if
        local.get $ctx call $def_reader_fail_opcode
        return
      end
      local.get $ctx call $read_u16be
      local.get $inv i32.store offset=0
      br $continue
    end
    unreachable
  )

  (func (export "def_decode_inventory_payload") (param $ptr i32) (param $len i32) (param $inv i32) (result i32)
    local.get $inv i32.eqz if global.get $DEF_ERR_BOUNDS return end
    global.get $SCRATCH_CTX
    local.get $ptr local.get $len call $def_reader_init
    if global.get $DEF_ERR_BOUNDS return end
    global.get $SCRATCH_CTX
    local.get $inv
    call $def_decode_inventory_ctx
  )

  (func (export "def_varbit_extract") (param $vb i32) (param $varp_val i32) (result i32)
    (local $low i32) (local $high i32) (local $width i32)
    local.get $vb i32.load8_u offset=4
    local.set $low
    local.get $vb i32.load8_u offset=5
    local.tee $high
    local.get $low
    i32.sub
    local.tee $width
    i32.const 31 i32.ge_u
    if
      local.get $varp_val
      local.get $low
      i32.shr_u
      return
    end
    local.get $varp_val
    local.get $low
    i32.shr_u
    i32.const 1
    local.get $width
    i32.shl
    i32.const 1
    i32.sub
    i32.and
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Object decoder helpers
  ;; ══════════════════════════════════════════════════════════════════

  (func $object_init_defaults (param $obj i32)
    local.get $obj global.get $OBJ_NAME_PTR i32.add i64.const 0 i64.store offset=0
    local.get $obj global.get $OBJ_NAME_LEN i32.add i32.const 0 i32.store
    local.get $obj global.get $OBJ_MODEL_COUNT i32.add i32.const 0 i32.store
    local.get $obj global.get $OBJ_MODEL_STORED_COUNT i32.add i32.const 0 i32.store
    local.get $obj global.get $OBJ_SIZE_X i32.add i32.const 1 i32.store
    local.get $obj global.get $OBJ_SIZE_Y i32.add i32.const 1 i32.store
    local.get $obj global.get $OBJ_INTERACT_TYPE i32.add i32.const 2 i32.store
    local.get $obj global.get $OBJ_BLOCK_PROJECTILE i32.add i32.const 1 i32.store
    local.get $obj global.get $OBJ_ANIMATION_ID i32.add global.get $DEF_ID_NONE i32.store
    local.get $obj global.get $OBJ_ACTION_PTRS i32.add i64.const 0 i64.store offset=0
    local.get $obj global.get $OBJ_VARBIT_ID i32.add global.get $DEF_ID_NONE i32.store
    local.get $obj global.get $OBJ_VARP_ID i32.add global.get $DEF_ID_NONE i32.store
    local.get $obj global.get $OBJ_TRANSFORM_COUNT i32.add i32.const 0 i32.store
    local.get $obj global.get $OBJ_TRANSFORM_STORED_COUNT i32.add i32.const 0 i32.store
    local.get $obj global.get $OBJ_AMBIENT_SOUND_ID i32.add global.get $DEF_ID_NONE i32.store
    local.get $obj global.get $OBJ_PARAMS_PTR i32.add i64.const 0 i64.store offset=0
    local.get $obj global.get $OBJ_CLIP_FLAGS i32.add global.get $DEF_OBJECT_CLIP_SOLID global.get $DEF_OBJECT_CLIP_PROJECTILE i32.or i32.store
    local.get $obj global.get $OBJ_FLAGS i32.add i32.const 0 i32.store
    local.get $obj global.get $OBJ_BLOCK_WALK i32.add i32.const 1 i32.store
  )

  (func $object_read_nullable_u16 (param $ctx i32) (result i32)
    (local $v i32)
    local.get $ctx call $read_u16be
    local.tee $v
    i32.const 65535 i32.eq
    if global.get $DEF_ID_NONE return end
    local.get $v
  )

  (func $object_store_model (param $obj i32) (param $idx i32) (param $id i32) (param $type i32)
    (local $capacity i32) (local $ids_ptr i32)
    local.get $obj global.get $OBJ_MODEL_CAPACITY i32.add i32.load
    local.tee $capacity
    local.get $idx
    i32.le_u
    if
      local.get $obj global.get $OBJ_FLAGS i32.add
      local.get $obj global.get $OBJ_FLAGS i32.add i32.load
      global.get $DEF_OBJECT_FLAG_MODEL_ARRAY_TRUNC
      i32.or
      i32.store
      return
    end
    local.get $obj global.get $OBJ_MODEL_IDS_PTR i32.add i32.load
    local.tee $ids_ptr
    i32.eqz
    if
      local.get $obj global.get $OBJ_FLAGS i32.add
      local.get $obj global.get $OBJ_FLAGS i32.add i32.load
      global.get $DEF_OBJECT_FLAG_MODEL_ARRAY_TRUNC
      i32.or
      i32.store
      return
    end
    local.get $ids_ptr local.get $idx i32.const 2 i32.shl i32.add local.get $id i32.store offset=0
    local.get $obj global.get $OBJ_MODEL_TYPES_PTR i32.add i32.load
    local.tee $ids_ptr
    if
      local.get $ids_ptr local.get $idx i32.const 2 i32.shl i32.add local.get $type i32.store offset=0
    end
    local.get $obj global.get $OBJ_MODEL_STORED_COUNT i32.add
    local.get $obj global.get $OBJ_MODEL_STORED_COUNT i32.add i32.load
    i32.const 1 i32.add
    i32.store
  )

  (func $object_store_transform (param $obj i32) (param $idx i32) (param $id i32)
    (local $capacity i32) (local $ids_ptr i32)
    local.get $obj global.get $OBJ_TRANSFORM_CAPACITY i32.add i32.load
    local.tee $capacity
    local.get $idx
    i32.le_u
    if
      local.get $obj global.get $OBJ_FLAGS i32.add
      local.get $obj global.get $OBJ_FLAGS i32.add i32.load
      global.get $DEF_OBJECT_FLAG_TRANSFORM_ARRAY_TRUNC
      i32.or
      i32.store
      return
    end
    local.get $obj global.get $OBJ_TRANSFORM_IDS_PTR i32.add i32.load
    local.tee $ids_ptr
    i32.eqz
    if
      local.get $obj global.get $OBJ_FLAGS i32.add
      local.get $obj global.get $OBJ_FLAGS i32.add i32.load
      global.get $DEF_OBJECT_FLAG_TRANSFORM_ARRAY_TRUNC
      i32.or
      i32.store
      return
    end
    local.get $ids_ptr local.get $idx i32.const 2 i32.shl i32.add local.get $id i32.store offset=0
    local.get $obj global.get $OBJ_TRANSFORM_STORED_COUNT i32.add
    local.get $obj global.get $OBJ_TRANSFORM_STORED_COUNT i32.add i32.load
    i32.const 1 i32.add
    i32.store
  )

  (func $read_model_id (param $ctx i32) (param $gameval i32) (result i32)
    local.get $gameval
    if
      local.get $ctx call $read_i32be
      return
    end
    local.get $ctx call $read_u16be
  )

  (func $object_models_typed (param $ctx i32) (param $obj i32) (param $gameval i32)
    (local $count i32) (local $i i32) (local $model_id i32) (local $model_type i32)
    local.get $ctx call $read_u8
    local.set $count
    local.get $count
    local.get $obj global.get $OBJ_MODEL_COUNT i32.add i32.store
    local.get $obj global.get $OBJ_MODEL_STORED_COUNT i32.add i32.const 0 i32.store
    block $done
      loop $continue
        local.get $i local.get $count i32.ge_s
        if br $done end
        local.get $ctx local.get $gameval call $read_model_id
        local.set $model_id
        local.get $ctx call $read_u8
        local.set $model_type
        local.get $obj local.get $i local.get $model_id local.get $model_type call $object_store_model
        local.get $i i32.const 1 i32.add local.set $i
        br $continue
      end
      unreachable
    end
  )

  (func $object_models_untyped (param $ctx i32) (param $obj i32) (param $gameval i32)
    (local $count i32) (local $i i32) (local $model_id i32)
    local.get $ctx call $read_u8
    local.set $count
    local.get $count
    local.get $obj global.get $OBJ_MODEL_COUNT i32.add i32.store
    local.get $obj global.get $OBJ_MODEL_STORED_COUNT i32.add i32.const 0 i32.store
    block $done
      loop $continue
        local.get $i local.get $count i32.ge_s
        if br $done end
        local.get $ctx local.get $gameval call $read_model_id
        local.set $model_id
        local.get $obj local.get $i local.get $model_id global.get $DEF_OBJECT_MODEL_TYPE_ANY call $object_store_model
        local.get $i i32.const 1 i32.add local.set $i
        br $continue
      end
      unreachable
    end
  )

  (func $object_transforms (param $ctx i32) (param $obj i32) (param $has_fallback i32)
    (local $count i32) (local $i i32) (local $fallback i32)
    local.get $ctx call $object_read_nullable_u16
    local.get $obj global.get $OBJ_VARBIT_ID i32.add i32.store
    local.get $ctx call $object_read_nullable_u16
    local.get $obj global.get $OBJ_VARP_ID i32.add i32.store
    local.get $has_fallback
    if
      local.get $ctx call $object_read_nullable_u16
      local.set $fallback
    end
    local.get $ctx call $read_u8
    i32.const 1 i32.add
    local.tee $count
    i32.const 1 i32.add
    local.get $obj global.get $OBJ_TRANSFORM_COUNT i32.add i32.store
    local.get $obj global.get $OBJ_TRANSFORM_STORED_COUNT i32.add i32.const 0 i32.store
    loop $ids
      local.get $i local.get $count i32.ge_s
      if
        local.get $obj local.get $i local.get $fallback call $object_store_transform
        return
      end
      local.get $ctx call $object_read_nullable_u16
      local.get $obj local.get $i call $object_store_transform
      local.get $i i32.const 1 i32.add local.set $i
      br $ids
    end
    unreachable
  )

  (func $object_sound_79 (param $ctx i32) (param $obj i32)
    (local $count i32)
    local.get $ctx call $read_u16be
    local.get $obj global.get $OBJ_AMBIENT_SOUND_ID i32.add i32.store
    local.get $ctx call $read_u16be
    drop
    local.get $ctx i32.const 2 call $skip_bytes
    drop
    local.get $ctx call $read_u8
    local.tee $count
    loop $continue
      local.get $count i32.eqz if return end
      local.get $ctx call $read_u16be drop
      local.get $count i32.const 1 i32.sub local.set $count
      br $continue
    end
    unreachable
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Object decoder (massive opcode dispatch)
  ;; ══════════════════════════════════════════════════════════════════

  (func $def_decode_object_ctx (export "def_decode_object_ctx") (param $ctx i32) (param $obj i32) (result i32)
    (local $opcode i32) (local $interact i32)
    local.get $obj call $object_init_defaults
    loop $loop
      local.get $ctx call $read_u8
      local.tee $opcode
      i32.eqz if global.get $DEF_OK return end

      ;; ── opcode dispatch ──
      (block $op_done
        local.get $opcode
        i32.const 1 i32.eq
        if
          local.get $ctx local.get $obj i32.const 0 call $object_models_typed
          br $op_done
        end
        local.get $opcode i32.const 2 i32.eq
        if
          local.get $obj global.get $OBJ_NAME_PTR i32.add
          local.get $obj global.get $OBJ_NAME_LEN i32.add
          local.get $ctx call $def_read_string_view
          drop
          br $op_done
        end
        local.get $opcode i32.const 5 i32.eq
        if
          local.get $ctx local.get $obj i32.const 0 call $object_models_untyped
          br $op_done
        end
        local.get $opcode i32.const 6 i32.eq
        if
          local.get $ctx local.get $obj i32.const 1 call $object_models_typed
          br $op_done
        end
        local.get $opcode i32.const 7 i32.eq
        if
          local.get $ctx local.get $obj i32.const 1 call $object_models_untyped
          br $op_done
        end
        local.get $opcode i32.const 14 i32.eq
        if
          local.get $ctx call $read_u8
          local.get $obj global.get $OBJ_SIZE_X i32.add i32.store
          br $op_done
        end
        local.get $opcode i32.const 15 i32.eq
        if
          local.get $ctx call $read_u8
          local.get $obj global.get $OBJ_SIZE_Y i32.add i32.store
          br $op_done
        end
        local.get $opcode i32.const 17 i32.eq
        if
          local.get $obj global.get $OBJ_INTERACT_TYPE i32.add i32.const 0 i32.store
          local.get $obj global.get $OBJ_BLOCK_PROJECTILE i32.add i32.const 0 i32.store
          local.get $obj global.get $OBJ_BLOCK_WALK i32.add i32.const 0 i32.store
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add i32.load
          global.get $DEF_OBJECT_CLIP_SOLID global.get $DEF_OBJECT_CLIP_PROJECTILE i32.or i32.const -1 i32.xor
          i32.and
          i32.store
          br $op_done
        end
        local.get $opcode i32.const 18 i32.eq
        if
          local.get $obj global.get $OBJ_BLOCK_PROJECTILE i32.add i32.const 0 i32.store
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add i32.load
          global.get $DEF_OBJECT_CLIP_PROJECTILE i32.const -1 i32.xor
          i32.and
          i32.store
          br $op_done
        end
        local.get $opcode i32.const 19 i32.eq
        if
          local.get $ctx call $read_u8
          local.tee $interact
          local.get $obj global.get $OBJ_INTERACT_TYPE i32.add i32.store
          local.get $interact
          if else
            local.get $obj global.get $OBJ_CLIP_FLAGS i32.add
            local.get $obj global.get $OBJ_CLIP_FLAGS i32.add i32.load
            global.get $DEF_OBJECT_CLIP_SOLID i32.const -1 i32.xor
            i32.and
            i32.store
            local.get $obj global.get $OBJ_BLOCK_WALK i32.add i32.const 0 i32.store
          end
          br $op_done
        end
        local.get $opcode i32.const 21 i32.eq  local.get $opcode i32.const 22 i32.eq  i32.or
        if br $op_done end

        local.get $opcode i32.const 23 i32.eq
        if
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add i32.load
          global.get $DEF_OBJECT_CLIP_MODEL
          i32.or
          i32.store
          br $op_done
        end
        local.get $opcode i32.const 24 i32.eq
        if
          local.get $ctx call $read_u16be
          local.tee $opcode
          i32.const 65535 i32.ne
          if
            local.get $obj global.get $OBJ_ANIMATION_ID i32.add local.get $opcode i32.store
          else
            local.get $obj global.get $OBJ_ANIMATION_ID i32.add global.get $DEF_ID_NONE i32.store
          end
          br $op_done
        end
        local.get $opcode i32.const 27 i32.eq
        if
          local.get $obj global.get $OBJ_INTERACT_TYPE i32.add i32.const 1 i32.store
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add i32.load
          global.get $DEF_OBJECT_CLIP_SOLID
          i32.or
          i32.store
          local.get $obj global.get $OBJ_BLOCK_WALK i32.add i32.const 1 i32.store
          br $op_done
        end
        local.get $opcode i32.const 28 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 29 i32.eq
        if
          local.get $ctx call $read_u8 drop
          br $op_done
        end
        local.get $opcode i32.const 30 i32.ge_s
        if
          local.get $opcode i32.const 34 i32.le_s
          if
            local.get $ctx call $skip_string drop
            br $op_done
          end
        end
        local.get $opcode i32.const 39 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 40 i32.eq  local.get $opcode i32.const 41 i32.eq  i32.or
        if
          local.get $ctx call $skip_count_u16_u16
          br $op_done
        end
        local.get $opcode i32.const 61 i32.eq
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 62 i32.eq
        if
          local.get $obj global.get $OBJ_FLAGS i32.add
          local.get $obj global.get $OBJ_FLAGS i32.add i32.load
          global.get $DEF_OBJECT_FLAG_MIRRORED
          i32.or
          i32.store
          br $op_done
        end
        local.get $opcode i32.const 64 i32.eq
        if
          local.get $obj global.get $OBJ_FLAGS i32.add
          local.get $obj global.get $OBJ_FLAGS i32.add i32.load
          global.get $DEF_OBJECT_FLAG_CASTS_SHADOW_FALSE
          i32.or
          i32.store
          br $op_done
        end
        local.get $opcode i32.const 65 i32.eq  local.get $opcode i32.const 66 i32.eq  i32.or
        local.get $opcode i32.const 67 i32.eq  local.get $opcode i32.const 68 i32.eq  i32.or
        i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 69 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 70 i32.eq  local.get $opcode i32.const 71 i32.eq  i32.or
        local.get $opcode i32.const 72 i32.eq  i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 73 i32.eq
        if
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add i32.load
          global.get $DEF_OBJECT_CLIP_OBSTRUCTS
          i32.or
          i32.store
          br $op_done
        end
        local.get $opcode i32.const 74 i32.eq
        if
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add
          local.get $obj global.get $OBJ_CLIP_FLAGS i32.add i32.load
          global.get $DEF_OBJECT_CLIP_HOLLOW
          i32.or
          i32.store
          local.get $obj global.get $OBJ_BLOCK_WALK i32.add i32.const 0 i32.store
          br $op_done
        end
        local.get $opcode i32.const 75 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 77 i32.eq
        if
          local.get $ctx local.get $obj i32.const 0 call $object_transforms
          br $op_done
        end
        local.get $opcode i32.const 78 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $obj global.get $OBJ_AMBIENT_SOUND_ID i32.add i32.store
          local.get $ctx i32.const 2 call $skip_bytes drop
          br $op_done
        end
        local.get $opcode i32.const 79 i32.eq
        if
          local.get $ctx local.get $obj call $object_sound_79
          br $op_done
        end
        local.get $opcode i32.const 81 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 82 i32.eq
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 89 i32.eq  local.get $opcode i32.const 90 i32.eq
        i32.or
        if br $op_done end
        local.get $opcode i32.const 91 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 92 i32.eq
        if
          local.get $ctx local.get $obj i32.const 1 call $object_transforms
          br $op_done
        end
        local.get $opcode i32.const 93 i32.eq
        if
          local.get $ctx i32.const 6 call $skip_bytes drop
          br $op_done
        end
        local.get $opcode i32.const 95 i32.eq  local.get $opcode i32.const 96 i32.eq i32.or
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 100 i32.ge_s
        if
          local.get $opcode i32.const 102 i32.le_s
          if
            local.get $ctx call $skip_string drop
            br $op_done
          end
        end
        local.get $opcode i32.const 249 i32.eq
        if
          local.get $ctx call $def_decode_params_skip
          drop
          br $op_done
        end
        local.get $ctx call $def_reader_fail_opcode
        return
      )
      br $loop
    end
    unreachable
  )

  (func (export "def_decode_object_payload") (param $ptr i32) (param $len i32) (param $obj i32) (result i32)
    local.get $obj i32.eqz if global.get $DEF_ERR_BOUNDS return end
    global.get $SCRATCH_CTX
    local.get $ptr local.get $len call $def_reader_init
    if global.get $DEF_ERR_BOUNDS return end
    global.get $SCRATCH_CTX
    local.get $obj
    call $def_decode_object_ctx
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Skip helpers
  ;; ══════════════════════════════════════════════════════════════════

  (func $skip_count_u8 (param $ctx i32) (result i32)
    (local $n i32)
    local.get $ctx call $read_u8
    local.tee $n
    if
      local.get $ctx local.get $n call $skip_bytes
      return
    end
    global.get $DEF_OK
  )

  (func $skip_count_u16 (param $ctx i32) (result i32)
    local.get $ctx call $read_u8
    i32.const 2 i32.mul
    local.get $ctx call $skip_bytes
  )

  (func $skip_count_u16_u16 (param $ctx i32) (result i32)
    local.get $ctx call $read_u8
    i32.const 2 i32.shl
    local.get $ctx call $skip_bytes
  )

  (func $skip_gameval_id (param $ctx i32) (result i32)
    local.get $ctx call $read_nullable_large_smart
    drop
    global.get $DEF_OK
  )

  (func $skip_count_gameval (param $ctx i32) (result i32)
    (local $count i32)
    local.get $ctx call $read_u8
    local.tee $count
    if
      loop $continue
        local.get $ctx call $skip_gameval_id drop
        local.get $count i32.const 1 i32.sub local.tee $count
        br_if $continue
      end
    end
    global.get $DEF_OK
  )

  (func $skip_transforms (param $ctx i32) (param $has_fallback i32) (result i32)
    (local $count i32)
    local.get $ctx call $read_u16be drop
    local.get $ctx call $read_u16be drop
    local.get $has_fallback
    if
      local.get $ctx call $read_u16be drop
    end
    local.get $ctx call $read_u8
    i32.const 1 i32.add
    local.set $count
    loop $continue
      local.get $count i32.eqz if global.get $DEF_OK return end
      local.get $ctx call $read_u16be drop
      local.get $count i32.const 1 i32.sub local.set $count
      br $continue
    end
    unreachable
  )

  (func $skip_npc_headicons (param $ctx i32) (result i32)
    (local $bitset i32) (local $i i32)
    local.get $ctx call $read_u8
    local.set $bitset
    loop $continue
      local.get $i i32.const 8 i32.ge_s if global.get $DEF_OK return end
      local.get $bitset
      local.get $i
      i32.shr_u
      i32.const 1 i32.and
      if
        local.get $ctx call $read_nullable_large_smart drop
        local.get $ctx call $read_nullable_large_smart drop
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $continue
    end
    unreachable
  )

  (func $skip_item_subactions (param $ctx i32) (result i32)
    local.get $ctx call $read_nullable_large_smart drop
    loop $continue
      local.get $ctx call $read_nullable_large_smart
      i32.eqz if global.get $DEF_OK return end
      local.get $ctx call $skip_string drop
      br $continue
    end
    unreachable
  )

  (func $skip_sequence_frames (param $ctx i32) (result i32)
    local.get $ctx call $read_u16be
    i32.const 6 i32.mul
    local.get $ctx call $skip_bytes
  )

  (func $skip_sequence_secondary_frames (param $ctx i32) (result i32)
    local.get $ctx call $read_u8
    i32.const 2 i32.shl
    local.get $ctx call $skip_bytes
  )

  (func $skip_sequence_sound_map (param $ctx i32) (result i32)
    (local $count i32)
    local.get $ctx call $read_u16be
    local.tee $count
    loop $continue
      local.get $count i32.eqz if global.get $DEF_OK return end
      local.get $ctx call $read_u16be drop
      local.get $ctx call $read_i32be drop
      local.get $count i32.const 1 i32.sub local.set $count
      br $continue
    end
    unreachable
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; NPC decoder
  ;; ══════════════════════════════════════════════════════════════════

  (func (export "def_decode_npc_ctx") (param $ctx i32) (param $npc i32) (result i32)
    (local $opcode i32)
    loop $loop
      local.get $ctx call $read_u8
      local.tee $opcode
      i32.eqz if global.get $DEF_OK return end
      (block $op_done
        local.get $opcode i32.const 1 i32.eq
        if local.get $ctx call $skip_count_u16 drop br $op_done end
        local.get $opcode i32.const 2 i32.eq
        if local.get $ctx call $skip_string drop br $op_done end
        local.get $opcode i32.const 12 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 13 i32.eq  local.get $opcode i32.const 14 i32.eq i32.or
        local.get $opcode i32.const 15 i32.eq  local.get $opcode i32.const 16 i32.eq i32.or
        i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 17 i32.eq  local.get $opcode i32.const 18 i32.eq i32.or
        if local.get $ctx i32.const 8 call $skip_bytes drop br $op_done end
        local.get $opcode i32.const 40 i32.eq  local.get $opcode i32.const 41 i32.eq i32.or
        if local.get $ctx call $skip_count_u16_u16 drop br $op_done end
        local.get $opcode i32.const 60 i32.eq
        if local.get $ctx call $skip_count_u16 drop br $op_done end
        local.get $opcode i32.const 61 i32.eq  local.get $opcode i32.const 62 i32.eq i32.or
        if local.get $ctx call $skip_count_gameval drop br $op_done end
        local.get $opcode i32.const 74 i32.ge_s
        if
          local.get $opcode i32.const 79 i32.le_s
          if local.get $ctx call $read_u16be drop br $op_done end
        end
        local.get $opcode i32.const 30 i32.ge_s
        if
          local.get $opcode i32.const 34 i32.le_s
          if local.get $ctx call $skip_string drop br $op_done end
        end
        local.get $opcode i32.const 251 i32.ge_s
        if
          local.get $opcode i32.const 253 i32.le_s
          if local.get $ctx call $skip_string drop br $op_done end
        end
        local.get $opcode i32.const 93 i32.eq
        if br $op_done end
        local.get $opcode i32.const 95 i32.eq  local.get $opcode i32.const 97 i32.eq i32.or
        local.get $opcode i32.const 98 i32.eq  i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 99 i32.eq
        if br $op_done end
        local.get $opcode i32.const 100 i32.eq  local.get $opcode i32.const 101 i32.eq i32.or
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 102 i32.eq
        if local.get $ctx call $skip_npc_headicons drop br $op_done end
        local.get $opcode i32.const 103 i32.eq
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 106 i32.eq
        if local.get $ctx i32.const 0 call $skip_transforms drop br $op_done end
        local.get $opcode i32.const 107 i32.eq  local.get $opcode i32.const 109 i32.eq i32.or
        local.get $opcode i32.const 111 i32.eq  i32.or
        if br $op_done end
        local.get $opcode i32.const 114 i32.eq  local.get $opcode i32.const 116 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 115 i32.eq  local.get $opcode i32.const 117 i32.eq i32.or
        if local.get $ctx i32.const 8 call $skip_bytes drop br $op_done end
        local.get $opcode i32.const 118 i32.eq
        if local.get $ctx i32.const 1 call $skip_transforms drop br $op_done end
        local.get $opcode i32.const 122 i32.eq  local.get $opcode i32.const 123 i32.eq i32.or
        if br $op_done end
        local.get $opcode i32.const 124 i32.eq  local.get $opcode i32.const 126 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 130 i32.eq  local.get $opcode i32.const 145 i32.eq i32.or
        local.get $opcode i32.const 147 i32.eq  i32.or
        if br $op_done end
        local.get $opcode i32.const 146 i32.eq
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 249 i32.eq
        if local.get $ctx call $def_decode_params_skip drop br $op_done end
        local.get $ctx call $def_reader_fail_opcode
        return
      )
      br $loop
    end
    unreachable
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Item decoder
  ;; ══════════════════════════════════════════════════════════════════

  (func (export "def_decode_item_ctx") (param $ctx i32) (param $item i32) (result i32)
    (local $opcode i32)
    local.get $item global.get $ITEM_NAME_PTR i32.add i64.const 0 i64.store offset=0
    local.get $item global.get $ITEM_NAME_LEN i32.add i32.const 0 i32.store
    local.get $item global.get $ITEM_INVENTORY_MODEL i32.add global.get $DEF_ID_NONE i32.store
    local.get $item global.get $ITEM_ZOOM_2D i32.add i32.const 2000 i32.store
    local.get $item global.get $ITEM_XAN_2D i32.add i32.const 0 i32.store
    local.get $item global.get $ITEM_YAN_2D i32.add i32.const 0 i32.store
    local.get $item global.get $ITEM_ZAN_2D i32.add i32.const 0 i32.store
    local.get $item global.get $ITEM_COST i32.add i32.const 1 i32.store
    local.get $item global.get $ITEM_STACKABLE i32.add i32.const 0 i32.store
    local.get $item global.get $ITEM_MEMBERS i32.add i32.const 0 i32.store
    local.get $item global.get $ITEM_GROUND_ACTIONS i32.add i64.const 0 i64.store offset=0
    local.get $item global.get $ITEM_INVENTORY_ACTIONS i32.add i64.const 0 i64.store offset=0
    local.get $item global.get $ITEM_NOTE_ID i32.add global.get $DEF_ID_NONE i32.store
    local.get $item global.get $ITEM_NOTE_TEMPLATE_ID i32.add global.get $DEF_ID_NONE i32.store
    local.get $item global.get $ITEM_PARAMS_PTR i32.add i64.const 0 i64.store offset=0
    loop $loop
      local.get $ctx call $read_u8
      local.tee $opcode
      i32.eqz if global.get $DEF_OK return end
      (block $op_done
        local.get $opcode i32.const 1 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $item global.get $ITEM_INVENTORY_MODEL i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 2 i32.eq  local.get $opcode i32.const 3 i32.eq i32.or
        local.get $opcode i32.const 9 i32.eq  i32.or
        if local.get $ctx call $skip_string drop br $op_done end
        local.get $opcode i32.const 4 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $item global.get $ITEM_ZOOM_2D i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 5 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $item global.get $ITEM_XAN_2D i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 6 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $item global.get $ITEM_YAN_2D i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 7 i32.eq  local.get $opcode i32.const 8 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 11 i32.eq
        if local.get $item global.get $ITEM_STACKABLE i32.add i32.const 1 i32.store br $loop end
        local.get $opcode i32.const 12 i32.eq
        if
          local.get $ctx call $read_i32be
          local.get $item global.get $ITEM_COST i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 13 i32.eq  local.get $opcode i32.const 14 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 16 i32.eq
        if local.get $item global.get $ITEM_MEMBERS i32.add i32.const 1 i32.store br $loop end
        local.get $opcode i32.const 23 i32.eq
        if local.get $ctx i32.const 5 call $skip_bytes drop br $op_done end
        local.get $opcode i32.const 24 i32.eq  local.get $opcode i32.const 26 i32.eq i32.or
        local.get $opcode i32.const 27 i32.eq  i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 25 i32.eq
        if local.get $ctx i32.const 4 call $skip_bytes drop br $op_done end
        local.get $opcode i32.const 30 i32.ge_s
        if
          local.get $opcode i32.const 34 i32.le_s
          if local.get $ctx call $skip_string drop br $op_done end
        end
        local.get $opcode i32.const 35 i32.ge_s
        if
          local.get $opcode i32.const 39 i32.le_s
          if local.get $ctx call $skip_string drop br $op_done end
        end
        local.get $opcode i32.const 40 i32.eq  local.get $opcode i32.const 41 i32.eq i32.or
        if local.get $ctx call $skip_count_u16_u16 drop br $op_done end
        local.get $opcode i32.const 42 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 43 i32.eq
        if local.get $ctx call $skip_item_subactions drop br $op_done end
        local.get $opcode i32.const 44 i32.eq
        if local.get $ctx call $skip_gameval_id drop br $op_done end
        local.get $opcode i32.const 45 i32.eq
        if
          local.get $ctx call $skip_gameval_id drop
          local.get $ctx call $read_u16be drop
          br $op_done
        end
        local.get $opcode i32.const 65 i32.eq
        if br $loop end
        local.get $opcode i32.const 75 i32.eq
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 78 i32.eq  local.get $opcode i32.const 79 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 90 i32.ge_s
        if
          local.get $opcode i32.const 93 i32.le_s
          if local.get $ctx call $read_u16be drop br $op_done end
        end
        local.get $opcode i32.const 94 i32.eq
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 95 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $item global.get $ITEM_ZAN_2D i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 97 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $item global.get $ITEM_NOTE_ID i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 98 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $item global.get $ITEM_NOTE_TEMPLATE_ID i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 100 i32.ge_s
        if
          local.get $opcode i32.const 109 i32.le_s
          if local.get $ctx i32.const 4 call $skip_bytes drop br $op_done end
        end
        local.get $opcode i32.const 110 i32.eq  local.get $opcode i32.const 111 i32.eq i32.or
        local.get $opcode i32.const 112 i32.eq  i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 113 i32.eq  local.get $opcode i32.const 114 i32.eq i32.or
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 115 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 139 i32.eq  local.get $opcode i32.const 140 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 148 i32.eq  local.get $opcode i32.const 149 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 249 i32.eq
        if local.get $ctx call $def_decode_params_skip drop br $op_done end
        local.get $ctx call $def_reader_fail_opcode
        return
      )
      br $loop
    end
    unreachable
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Sequence decoder
  ;; ══════════════════════════════════════════════════════════════════

  (func $def_decode_sequence_frames (param $ctx i32) (param $seq i32) (param $capacity i32)
    (local $count i32) (local $i i32) (local $val i32) (local $stored i32) (local $ptr i32)
    local.get $ctx call $read_u16be
    local.tee $count
    local.get $seq global.get $SEQ_FRAME_COUNT i32.add i32.store
    local.get $seq global.get $SEQ_FRAME_STORED_COUNT i32.add i32.const 0 i32.store
    local.get $seq global.get $SEQ_FRAME_LENGTHS_PTR i32.add i32.load
    local.set $ptr
    ;; lengths loop
    block $lengths_done
      loop $lengths
        local.get $i local.get $count i32.ge_s
        if
          i32.const 0 local.set $i
          br $lengths_done
        end
        local.get $ctx call $read_u16be
        local.set $val
        local.get $ptr
        if
          local.get $i local.get $capacity i32.lt_s
          if
            local.get $ptr local.get $i i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $lengths
      end
      unreachable
    end
    local.get $seq global.get $SEQ_FRAME_IDS_PTR i32.add i32.load
    local.set $ptr
    ;; low frame words
    block $lows_done
      loop $lows
        local.get $i local.get $count i32.ge_s
        if
          i32.const 0 local.set $i
          br $lows_done
        end
        local.get $ctx call $read_u16be
        local.set $val
        local.get $ptr
        if
          local.get $i local.get $capacity i32.lt_s
          if
            local.get $ptr local.get $i i32.const 2 i32.shl i32.add local.get $val i32.store offset=0
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $lows
      end
      unreachable
    end
    ;; high frame words + merge into low
    local.get $seq global.get $SEQ_FRAME_IDS_PTR i32.add i32.load
    local.set $ptr
    loop $highs
      local.get $i local.get $count i32.ge_s if return end
      local.get $ctx call $read_u16be
      local.set $val
      local.get $ptr
      if
        local.get $i local.get $capacity i32.lt_s
        if
          local.get $ptr local.get $i i32.const 2 i32.shl i32.add
          local.tee $ptr
          local.get $ptr i32.load offset=0
          local.get $val i32.const 16 i32.shl
          i32.or
          i32.store offset=0
          local.get $seq global.get $SEQ_FRAME_STORED_COUNT i32.add
          local.get $seq global.get $SEQ_FRAME_STORED_COUNT i32.add i32.load
          i32.const 1 i32.add
          i32.store
        end
        local.get $seq global.get $SEQ_FRAME_IDS_PTR i32.add i32.load
        local.set $ptr
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $highs
    end
    unreachable
  )

  (func $def_decode_sequence_sound_map_count (param $ctx i32) (param $seq i32)
    (local $count i32)
    local.get $ctx call $read_u16be
    local.tee $count
    local.get $seq global.get $SEQ_SOUND_COUNT i32.add i32.store
    loop $continue
      local.get $count i32.eqz if return end
      local.get $ctx call $read_u16be drop
      local.get $ctx call $read_i32be drop
      local.get $count i32.const 1 i32.sub local.set $count
      br $continue
    end
    unreachable
  )

  (func $def_decode_sequence_ctx (export "def_decode_sequence_ctx") (param $ctx i32) (param $seq i32) (result i32)
    (local $opcode i32) (local $capacity i32)
    local.get $seq global.get $SEQ_FRAME_CAPACITY i32.add i32.load
    local.set $capacity
    local.get $seq global.get $SEQ_FRAME_COUNT i32.add i32.const 0 i32.store
    local.get $seq global.get $SEQ_FRAME_STORED_COUNT i32.add i32.const 0 i32.store
    local.get $seq global.get $SEQ_FRAME_STEP i32.add global.get $DEF_ID_NONE i32.store
    local.get $seq global.get $SEQ_FORCED_PRIORITY i32.add i32.const 5 i32.store
    local.get $seq global.get $SEQ_LEFT_HAND_ITEM i32.add global.get $DEF_ID_NONE i32.store
    local.get $seq global.get $SEQ_RIGHT_HAND_ITEM i32.add global.get $DEF_ID_NONE i32.store
    local.get $seq global.get $SEQ_SOUND_COUNT i32.add i32.const 0 i32.store
    loop $loop
      local.get $ctx call $read_u8
      local.tee $opcode
      i32.eqz if global.get $DEF_OK return end
      (block $op_done
        local.get $opcode i32.const 1 i32.eq
        if
          local.get $ctx local.get $seq local.get $capacity call $def_decode_sequence_frames
          br $op_done
        end
        local.get $opcode i32.const 2 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $seq global.get $SEQ_FRAME_STEP i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 3 i32.eq
        if local.get $ctx call $skip_count_u8 drop br $op_done end
        local.get $opcode i32.const 4 i32.eq
        if br $op_done end
        local.get $opcode i32.const 5 i32.eq
        if
          local.get $ctx call $read_u8
          local.get $seq global.get $SEQ_FORCED_PRIORITY i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 6 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $seq global.get $SEQ_LEFT_HAND_ITEM i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 7 i32.eq
        if
          local.get $ctx call $read_u16be
          local.get $seq global.get $SEQ_RIGHT_HAND_ITEM i32.add i32.store
          br $loop
        end
        local.get $opcode i32.const 8 i32.eq  local.get $opcode i32.const 9 i32.eq i32.or
        local.get $opcode i32.const 10 i32.eq  local.get $opcode i32.const 11 i32.eq i32.or
        i32.or
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 12 i32.eq
        if local.get $ctx call $skip_sequence_secondary_frames drop br $op_done end
        local.get $opcode i32.const 13 i32.eq
        if local.get $ctx call $read_i32be drop br $op_done end
        local.get $opcode i32.const 14 i32.eq
        if
          local.get $ctx local.get $seq call $def_decode_sequence_sound_map_count
          br $op_done
        end
        local.get $opcode i32.const 15 i32.eq
        if local.get $ctx i32.const 4 call $skip_bytes drop br $op_done end
        local.get $opcode i32.const 16 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 17 i32.eq
        if local.get $ctx call $skip_count_u8 drop br $op_done end
        local.get $opcode i32.const 19 i32.eq
        if br $op_done end
        local.get $ctx call $def_reader_fail_opcode
        return
      )
      br $loop
    end
    unreachable
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Item/sequence skip decoders (parse but don't store)
  ;; ══════════════════════════════════════════════════════════════════

  (func (export "def_decode_item_skip_ctx") (param $ctx i32) (param $item i32) (result i32)
    (local $opcode i32)
    loop $loop
      local.get $ctx call $read_u8
      local.tee $opcode
      i32.eqz if global.get $DEF_OK return end
      (block $op_done
        local.get $opcode i32.const 1 i32.eq
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 2 i32.eq  local.get $opcode i32.const 3 i32.eq i32.or
        local.get $opcode i32.const 9 i32.eq  i32.or
        if local.get $ctx call $skip_string drop br $op_done end
        local.get $opcode i32.const 4 i32.eq  local.get $opcode i32.const 5 i32.eq i32.or
        local.get $opcode i32.const 6 i32.eq  i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 7 i32.eq  local.get $opcode i32.const 8 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 11 i32.eq  local.get $opcode i32.const 16 i32.eq i32.or
        if br $op_done end
        local.get $opcode i32.const 12 i32.eq
        if local.get $ctx call $read_i32be drop br $op_done end
        local.get $opcode i32.const 13 i32.eq  local.get $opcode i32.const 14 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 23 i32.eq
        if local.get $ctx i32.const 5 call $skip_bytes drop br $op_done end
        local.get $opcode i32.const 24 i32.eq  local.get $opcode i32.const 26 i32.eq i32.or
        local.get $opcode i32.const 27 i32.eq  i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 25 i32.eq
        if local.get $ctx i32.const 4 call $skip_bytes drop br $op_done end
        local.get $opcode i32.const 30 i32.ge_s
        if
          local.get $opcode i32.const 34 i32.le_s
          if local.get $ctx call $skip_string drop br $op_done end
        end
        local.get $opcode i32.const 35 i32.ge_s
        if
          local.get $opcode i32.const 39 i32.le_s
          if local.get $ctx call $skip_string drop br $op_done end
        end
        local.get $opcode i32.const 40 i32.eq  local.get $opcode i32.const 41 i32.eq i32.or
        if local.get $ctx call $skip_count_u16_u16 drop br $op_done end
        local.get $opcode i32.const 42 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 43 i32.eq
        if local.get $ctx call $skip_item_subactions drop br $op_done end
        local.get $opcode i32.const 44 i32.eq
        if local.get $ctx call $skip_gameval_id drop br $op_done end
        local.get $opcode i32.const 45 i32.eq
        if
          local.get $ctx call $skip_gameval_id drop
          local.get $ctx call $read_u16be drop
          br $op_done
        end
        local.get $opcode i32.const 65 i32.eq
        if br $op_done end
        local.get $opcode i32.const 75 i32.eq
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 78 i32.eq  local.get $opcode i32.const 79 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 90 i32.ge_s
        if
          local.get $opcode i32.const 93 i32.le_s
          if local.get $ctx call $read_u16be drop br $op_done end
        end
        local.get $opcode i32.const 94 i32.eq  local.get $opcode i32.const 95 i32.eq i32.or
        local.get $opcode i32.const 97 i32.eq  local.get $opcode i32.const 98 i32.eq i32.or
        i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 100 i32.ge_s
        if
          local.get $opcode i32.const 109 i32.le_s
          if local.get $ctx i32.const 4 call $skip_bytes drop br $op_done end
        end
        local.get $opcode i32.const 110 i32.eq  local.get $opcode i32.const 111 i32.eq i32.or
        local.get $opcode i32.const 112 i32.eq  i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 113 i32.eq  local.get $opcode i32.const 114 i32.eq i32.or
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 115 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 139 i32.eq  local.get $opcode i32.const 140 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 148 i32.eq  local.get $opcode i32.const 149 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 249 i32.eq
        if local.get $ctx call $def_decode_params_skip drop br $op_done end
        local.get $ctx call $def_reader_fail_opcode
        return
      )
      br $loop
    end
    unreachable
  )

  (func (export "def_decode_sequence_skip_ctx") (param $ctx i32) (param $seq i32) (result i32)
    (local $opcode i32)
    loop $loop
      local.get $ctx call $read_u8
      local.tee $opcode
      i32.eqz if global.get $DEF_OK return end
      (block $op_done
        local.get $opcode i32.const 1 i32.eq
        if local.get $ctx call $skip_sequence_frames drop br $op_done end
        local.get $opcode i32.const 2 i32.eq
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 3 i32.eq
        if local.get $ctx call $skip_count_u8 drop br $op_done end
        local.get $opcode i32.const 4 i32.eq
        if br $op_done end
        local.get $opcode i32.const 5 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 6 i32.eq  local.get $opcode i32.const 7 i32.eq i32.or
        if local.get $ctx call $read_u16be drop br $op_done end
        local.get $opcode i32.const 8 i32.eq  local.get $opcode i32.const 9 i32.eq i32.or
        local.get $opcode i32.const 10 i32.eq  local.get $opcode i32.const 11 i32.eq i32.or
        i32.or
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 12 i32.eq
        if local.get $ctx call $skip_sequence_secondary_frames drop br $op_done end
        local.get $opcode i32.const 13 i32.eq
        if local.get $ctx call $read_i32be drop br $op_done end
        local.get $opcode i32.const 14 i32.eq
        if local.get $ctx call $skip_sequence_sound_map drop br $op_done end
        local.get $opcode i32.const 15 i32.eq
        if local.get $ctx i32.const 4 call $skip_bytes drop br $op_done end
        local.get $opcode i32.const 16 i32.eq
        if local.get $ctx call $read_u8 drop br $op_done end
        local.get $opcode i32.const 17 i32.eq
        if local.get $ctx call $skip_count_u8 drop br $op_done end
        local.get $opcode i32.const 19 i32.eq
        if br $op_done end
        local.get $ctx call $def_reader_fail_opcode
        return
      )
      br $loop
    end
    unreachable
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Sequence payload wrapper
  ;; ══════════════════════════════════════════════════════════════════

  (func (export "def_decode_sequence_payload") (param $ptr i32) (param $len i32) (param $seq i32) (result i32)
    local.get $seq i32.eqz if global.get $DEF_ERR_BOUNDS return end
    global.get $SCRATCH_CTX
    local.get $ptr local.get $len call $def_reader_init
    if global.get $DEF_ERR_BOUNDS return end
    global.get $SCRATCH_CTX
    local.get $seq
    call $def_decode_sequence_ctx
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Animation decoder – skeleton
  ;; ══════════════════════════════════════════════════════════════════

  (func $anim_decode_skeleton_payload (export "anim_decode_skeleton_payload") (param $ptr i32) (param $len i32) (param $skel i32) (result i32)
    (local $end i32) (local $count i32) (local $i i32) (local $val i32) (local $lc i32)
    local.get $ptr i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $skel i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $ptr local.get $len i32.add
    local.set $end
    local.get $ptr local.get $end i32.ge_u if global.get $DEF_ERR_BOUNDS return end
    local.get $ptr i32.load8_u
    local.tee $count
    local.get $skel global.get $SKEL_TRANSFORM_COUNT i32.add i32.store
    local.get $skel global.get $SKEL_STORED_COUNT i32.add i32.const 0 i32.store
    local.get $skel global.get $SKEL_TOTAL_LABEL_COUNT i32.add i32.const 0 i32.store
    local.get $end local.get $ptr i32.const 1 i32.add i32.sub
    local.get $count i32.lt_u
    if global.get $DEF_ERR_BOUNDS return end
    ;; store transform types
    block $types_done
      loop $types
        local.get $i local.get $count i32.ge_s if br $types_done end
        local.get $ptr i32.const 1 i32.add local.get $i i32.add i32.load8_u
        local.set $val
        local.get $i local.get $skel global.get $SKEL_TRANSFORM_CAPACITY i32.add i32.load i32.lt_s
        if
          local.get $skel global.get $SKEL_TRANSFORM_TYPES_PTR i32.add i32.load
          if
            local.get $skel global.get $SKEL_TRANSFORM_TYPES_PTR i32.add i32.load
            local.get $i i32.const 2 i32.shl i32.add
            local.get $val i32.store offset=0
            local.get $skel global.get $SKEL_STORED_COUNT i32.add
            local.get $skel global.get $SKEL_STORED_COUNT i32.add i32.load
            i32.const 1 i32.add
            i32.store
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $types
      end
      unreachable
    end
    local.get $ptr i32.const 1 i32.add local.get $count i32.add
    local.set $ptr
    local.get $end local.get $ptr i32.sub
    local.get $count i32.lt_u
    if global.get $DEF_ERR_BOUNDS return end
    i32.const 0 local.set $i
    i32.const 0 local.set $val
    ;; label counts + offsets
    block $label_counts_done
      loop $label_counts
        local.get $i local.get $count i32.ge_s if br $label_counts_done end
        local.get $i local.get $skel global.get $SKEL_TRANSFORM_CAPACITY i32.add i32.load i32.lt_s
        if
          local.get $skel global.get $SKEL_LABEL_OFFSETS_PTR i32.add i32.load
          if
            local.get $skel global.get $SKEL_LABEL_OFFSETS_PTR i32.add i32.load
            local.get $i i32.const 2 i32.shl i32.add
            local.get $val i32.store offset=0
          end
        end
        local.get $ptr i32.load8_u
        local.tee $lc
        local.get $val i32.add
        local.set $val
        local.get $i local.get $skel global.get $SKEL_TRANSFORM_CAPACITY i32.add i32.load i32.lt_s
        if
          local.get $skel global.get $SKEL_LABEL_COUNTS_PTR i32.add i32.load
          if
            local.get $skel global.get $SKEL_LABEL_COUNTS_PTR i32.add i32.load
            local.get $i i32.const 2 i32.shl i32.add
            local.get $lc i32.store offset=0
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        local.get $ptr i32.const 1 i32.add local.set $ptr
        br $label_counts
      end
      unreachable
    end
    local.get $skel global.get $SKEL_TOTAL_LABEL_COUNT i32.add local.get $val i32.store
    local.get $end local.get $ptr i32.sub
    local.get $val i32.lt_u
    if global.get $DEF_ERR_BOUNDS return end
    i32.const 0 local.set $i
    ;; label values
    block $label_vals_done
      loop $label_vals
        local.get $i local.get $val i32.ge_s if br $label_vals_done end
        local.get $i local.get $skel global.get $SKEL_LABEL_CAPACITY i32.add i32.load i32.lt_s
        if
          local.get $skel global.get $SKEL_LABELS_PTR i32.add i32.load
          if
            local.get $skel global.get $SKEL_LABELS_PTR i32.add i32.load
            local.get $i i32.const 2 i32.shl i32.add
            local.get $ptr i32.load8_u
            i32.store offset=0
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        local.get $ptr i32.const 1 i32.add local.set $ptr
        br $label_vals
      end
      unreachable
    end
    global.get $DEF_OK
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Animation decoder – frame (reader smart)
  ;; ══════════════════════════════════════════════════════════════════

  (func $anim_frame_read_short_smart (param $ptr i32) (param $end i32) (result i32 i32)
    (local $v i32)
    local.get $ptr local.get $end i32.ge_u if i32.const 0 local.get $ptr return end
    local.get $ptr i32.load8_u
    local.tee $v
    i32.const 128 i32.ge_u
    if
      local.get $ptr i32.const 2 i32.add
      local.get $end i32.gt_u if i32.const 0 local.get $ptr return end
      local.get $ptr i32.load8_u i32.const 8 i32.shl
      local.get $ptr i32.const 1 i32.add i32.load8_u
      i32.or
      i32.const 49152 i32.sub
      local.get $ptr i32.const 2 i32.add
      return
    end
    local.get $v i32.const 64 i32.sub
    local.get $ptr i32.const 1 i32.add
  )

  (func $anim_decode_frame_payload (export "anim_decode_frame_payload") (param $ptr i32) (param $len i32) (param $frame i32) (param $skel i32) (result i32)
    (local $end i32) (local $slot_count i32) (local $i i32) (local $flags i32) (local $type i32)
    (local $x i32) (local $y i32) (local $z i32) (local $cursor i32) (local $smart_ptr i32)
    (local $tc i32) (local $tptr i32)
    local.get $ptr i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $len i32.const 3 i32.lt_s if global.get $DEF_ERR_BOUNDS return end
    local.get $ptr local.get $len i32.add
    local.set $end
    local.get $ptr i32.load8_u i32.const 8 i32.shl
    local.get $ptr i32.const 1 i32.add i32.load8_u
    i32.or
    local.get $frame global.get $FRAME_SKELETON_ID i32.add i32.store
    local.get $ptr i32.const 2 i32.add i32.load8_u
    local.tee $slot_count
    local.get $frame global.get $FRAME_SLOT_COUNT i32.add i32.store
    local.get $frame global.get $FRAME_TRANSFORM_COUNT i32.add i32.const 0 i32.store
    local.get $frame global.get $FRAME_HAS_ALPHA i32.add i32.const 0 i32.store
    local.get $ptr i32.const 3 i32.add
    local.set $cursor
    local.get $cursor local.get $slot_count i32.add
    local.tee $smart_ptr
    local.get $end i32.gt_u if global.get $DEF_ERR_BOUNDS return end
    loop $slots
      local.get $i local.get $slot_count i32.ge_s if global.get $DEF_OK return end
      local.get $cursor local.get $i i32.add i32.load8_u
      local.tee $flags
      i32.eqz
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $slots
      end
      ;; determine transform type from skeleton
      i32.const 0 local.set $type
      local.get $skel
      if
        local.get $i local.get $skel global.get $SKEL_TRANSFORM_COUNT i32.add i32.load i32.lt_s
        if
          local.get $skel global.get $SKEL_TRANSFORM_TYPES_PTR i32.add i32.load
          local.tee $tptr
          if
            local.get $tptr local.get $i i32.const 2 i32.shl i32.add i32.load
            local.set $type
          end
        end
      end
      local.get $type i32.const 5 i32.eq
      if
        local.get $frame global.get $FRAME_HAS_ALPHA i32.add i32.const 1 i32.store
      end
      ;; defaults
      i32.const 0 local.set $x
      local.get $type i32.const 3 i32.eq
      if i32.const 128 local.set $x end
      local.get $x local.set $y
      local.get $x local.set $z
      ;; read smart values for x/y/z
      local.get $flags i32.const 1 i32.and
      if
        local.get $smart_ptr local.get $end call $anim_frame_read_short_smart
        local.set $x
        local.set $smart_ptr
      end
      local.get $flags i32.const 2 i32.and
      if
        local.get $smart_ptr local.get $end call $anim_frame_read_short_smart
        local.set $y
        local.set $smart_ptr
      end
      local.get $flags i32.const 4 i32.and
      if
        local.get $smart_ptr local.get $end call $anim_frame_read_short_smart
        local.set $z
        local.set $smart_ptr
      end
      ;; store transform data
      local.get $frame global.get $FRAME_TRANSFORM_COUNT i32.add i32.load
      local.tee $tc
      local.get $frame global.get $FRAME_TRANSFORM_CAPACITY i32.add i32.load
      i32.lt_s
      if
        local.get $frame global.get $FRAME_TRANSFORM_INDICES_PTR i32.add i32.load
        local.tee $tptr
        if
          local.get $tptr local.get $tc i32.const 2 i32.shl i32.add local.get $i i32.store offset=0
        end
        local.get $frame global.get $FRAME_TRANSFORM_X_PTR i32.add i32.load
        local.tee $tptr
        if
          local.get $tptr local.get $tc i32.const 2 i32.shl i32.add local.get $x i32.store offset=0
        end
        local.get $frame global.get $FRAME_TRANSFORM_Y_PTR i32.add i32.load
        local.tee $tptr
        if
          local.get $tptr local.get $tc i32.const 2 i32.shl i32.add local.get $y i32.store offset=0
        end
        local.get $frame global.get $FRAME_TRANSFORM_Z_PTR i32.add i32.load
        local.tee $tptr
        if
          local.get $tptr local.get $tc i32.const 2 i32.shl i32.add local.get $z i32.store offset=0
        end
      end
      local.get $frame global.get $FRAME_TRANSFORM_COUNT i32.add
      local.get $tc i32.const 1 i32.add
      i32.store
      local.get $i i32.const 1 i32.add local.set $i
      br $slots
    end
    unreachable
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Animation – accumulate type delta
  ;; ══════════════════════════════════════════════════════════════════

  (func $anim_frame_accumulate_type_delta (export "anim_frame_accumulate_type_delta") (param $frame i32) (param $skel i32) (param $type i32) (param $out i32) (result i32)
    (local $count i32) (local $i i32) (local $idx i32) (local $match_count i32) (local $tptr i32)
    (local $indices i32) (local $xp i32) (local $yp i32) (local $zp i32)
    local.get $frame i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $skel i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $out i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $out i64.const 0 i64.store offset=0
    local.get $out i32.const 0 i32.store offset=8
    local.get $frame global.get $FRAME_TRANSFORM_INDICES_PTR i32.add i32.load
    local.tee $indices
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_X_PTR i32.add i32.load
    local.tee $xp
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_Y_PTR i32.add i32.load
    local.tee $yp
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_Z_PTR i32.add i32.load
    local.tee $zp
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_COUNT i32.add i32.load
    local.set $count
    loop $continue
      local.get $i local.get $count i32.ge_s if local.get $match_count return end
      local.get $indices local.get $i i32.const 2 i32.shl i32.add i32.load
      local.tee $idx
      local.get $skel global.get $SKEL_TRANSFORM_COUNT i32.add i32.load
      i32.ge_u
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $continue
      end
      local.get $skel global.get $SKEL_TRANSFORM_TYPES_PTR i32.add i32.load
      local.tee $tptr
      i32.eqz if global.get $DEF_ERR_BOUNDS return end
      local.get $tptr local.get $idx i32.const 2 i32.shl i32.add i32.load
      local.get $type i32.ne
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $continue
      end
      local.get $out
      local.get $out i32.load offset=0
      local.get $xp local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.add
      i32.store offset=0
      local.get $out i32.const 4 i32.add
      local.get $out i32.load offset=4
      local.get $yp local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.add
      i32.store offset=0
      local.get $out i32.const 8 i32.add
      local.get $out i32.load offset=8
      local.get $zp local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.add
      i32.store offset=0
      local.get $match_count i32.const 1 i32.add local.set $match_count
      local.get $i i32.const 1 i32.add local.set $i
      br $continue
    end
    unreachable
  )

  (func $anim_frame_accumulate_type_delta_target (export "anim_frame_accumulate_type_delta_target") (param $frame i32) (param $skel i32) (param $type i32) (param $delta_out i32) (param $label_out i32) (result i32)
    (local $count i32) (local $i i32) (local $idx i32) (local $match_count i32) (local $tptr i32)
    (local $indices i32) (local $xp i32) (local $yp i32) (local $zp i32)
    (local $label_idx i32) (local $lc i32)
    local.get $frame i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $skel i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $delta_out i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $label_out i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $delta_out i64.const 0 i64.store offset=0
    local.get $delta_out i32.const 0 i32.store offset=8
    local.get $label_out global.get $DEF_ID_NONE i32.store offset=0
    local.get $frame global.get $FRAME_TRANSFORM_INDICES_PTR i32.add i32.load
    local.tee $indices
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_X_PTR i32.add i32.load
    local.tee $xp
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_Y_PTR i32.add i32.load
    local.tee $yp
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_Z_PTR i32.add i32.load
    local.tee $zp
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_COUNT i32.add i32.load
    local.set $count
    loop $continue
      local.get $i local.get $count i32.ge_s if local.get $match_count return end
      local.get $indices local.get $i i32.const 2 i32.shl i32.add i32.load
      local.tee $idx
      local.get $skel global.get $SKEL_TRANSFORM_COUNT i32.add i32.load
      i32.ge_u
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $continue
      end
      local.get $skel global.get $SKEL_TRANSFORM_TYPES_PTR i32.add i32.load
      local.tee $tptr
      i32.eqz if global.get $DEF_ERR_BOUNDS return end
      local.get $tptr local.get $idx i32.const 2 i32.shl i32.add i32.load
      local.get $type i32.ne
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $continue
      end
      local.get $delta_out
      local.get $delta_out i32.load offset=0
      local.get $xp local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.add
      i32.store offset=0
      local.get $delta_out i32.const 4 i32.add
      local.get $delta_out i32.load offset=4
      local.get $yp local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.add
      i32.store offset=0
      local.get $delta_out i32.const 8 i32.add
      local.get $delta_out i32.load offset=8
      local.get $zp local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.add
      i32.store offset=0
      local.get $match_count i32.const 1 i32.add local.set $match_count
      local.get $label_out i32.load offset=0
      global.get $DEF_ID_NONE i32.ne
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $continue
      end
      ;; first match: try to get label
      local.get $skel global.get $SKEL_LABEL_COUNTS_PTR i32.add i32.load
      local.tee $tptr
      i32.eqz if local.get $i i32.const 1 i32.add local.set $i br $continue end
      local.get $tptr local.get $idx i32.const 2 i32.shl i32.add i32.load
      local.tee $lc
      i32.eqz if local.get $i i32.const 1 i32.add local.set $i br $continue end
      local.get $skel global.get $SKEL_LABEL_OFFSETS_PTR i32.add i32.load
      local.tee $tptr
      i32.eqz if local.get $i i32.const 1 i32.add local.set $i br $continue end
      local.get $tptr local.get $idx i32.const 2 i32.shl i32.add i32.load
      local.tee $label_idx
      local.get $skel global.get $SKEL_TOTAL_LABEL_COUNT i32.add i32.load
      i32.ge_u
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $continue
      end
      local.get $skel global.get $SKEL_LABELS_PTR i32.add i32.load
      local.tee $tptr
      i32.eqz if local.get $i i32.const 1 i32.add local.set $i br $continue end
      local.get $label_out local.get $tptr local.get $label_idx i32.const 2 i32.shl i32.add i32.load i32.store offset=0
      local.get $i i32.const 1 i32.add local.set $i
      br $continue
    end
    unreachable
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Animation – accumulate type group yaws
  ;; ══════════════════════════════════════════════════════════════════

  (func $anim_frame_accumulate_type_group_yaws (export "anim_frame_accumulate_type_group_yaws") (param $frame i32) (param $skel i32) (param $type i32) (param $label_out i32) (param $yaw_out i32) (param $max_out i32) (result i32)
    (local $count i32) (local $i i32) (local $idx i32) (local $out_count i32) (local $tptr i32)
    (local $indices i32) (local $yp i32) (local $label_idx i32) (local $lc i32) (local $label_val i32)
    local.get $frame i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $skel i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $label_out i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $yaw_out i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $max_out i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_INDICES_PTR i32.add i32.load
    local.tee $indices
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_Y_PTR i32.add i32.load
    local.tee $yp
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $skel global.get $SKEL_TRANSFORM_TYPES_PTR i32.add i32.load
    local.tee $tptr
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_COUNT i32.add i32.load
    local.set $count
    loop $continue
      local.get $i local.get $count i32.ge_s if local.get $out_count return end
      local.get $out_count local.get $max_out i32.ge_s if local.get $out_count return end
      local.get $indices local.get $i i32.const 2 i32.shl i32.add i32.load
      local.tee $idx
      local.get $skel global.get $SKEL_TRANSFORM_COUNT i32.add i32.load
      i32.ge_u
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $continue
      end
      local.get $tptr local.get $idx i32.const 2 i32.shl i32.add i32.load
      local.get $type i32.ne
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $continue
      end
      ;; get label
      global.get $DEF_ID_NONE
      local.set $label_val
      local.get $skel global.get $SKEL_LABEL_COUNTS_PTR i32.add i32.load
      local.tee $tptr
      if
        local.get $tptr local.get $idx i32.const 2 i32.shl i32.add i32.load
        local.tee $lc
        if
          local.get $skel global.get $SKEL_LABEL_OFFSETS_PTR i32.add i32.load
          local.tee $tptr
          if
            local.get $tptr local.get $idx i32.const 2 i32.shl i32.add i32.load
            local.tee $label_idx
            local.get $skel global.get $SKEL_TOTAL_LABEL_COUNT i32.add i32.load
            i32.lt_u
            if
              local.get $skel global.get $SKEL_LABELS_PTR i32.add i32.load
              local.tee $tptr
              if
                local.get $tptr local.get $label_idx i32.const 2 i32.shl i32.add i32.load
                local.set $label_val
              end
            end
          end
        end
      end
      local.get $label_out local.get $out_count i32.const 2 i32.shl i32.add local.get $label_val i32.store offset=0
      local.get $yaw_out local.get $out_count i32.const 2 i32.shl i32.add local.get $yp local.get $i i32.const 2 i32.shl i32.add i32.load i32.store offset=0
      local.get $out_count i32.const 1 i32.add local.set $out_count
      local.get $i i32.const 1 i32.add local.set $i
      br $continue
    end
    unreachable
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; Initialize static registry data
  ;; ══════════════════════════════════════════════════════════════════

  (func (export "anim_frame_registry_init")
    global.get $REG_FRAME global.get $FRAME_TRANSFORM_INDICES_PTR i32.add global.get $REG_INDICES i32.store offset=0
    global.get $REG_FRAME global.get $FRAME_TRANSFORM_X_PTR i32.add global.get $REG_X i32.store offset=0
    global.get $REG_FRAME global.get $FRAME_TRANSFORM_Y_PTR i32.add global.get $REG_Y i32.store offset=0
    global.get $REG_FRAME global.get $FRAME_TRANSFORM_Z_PTR i32.add global.get $REG_Z i32.store offset=0
    global.get $REG_FRAME global.get $FRAME_TRANSFORM_CAPACITY i32.add global.get $ANIM_FRAME_REGISTRY_TRANSFORM_CAP i32.store

    global.get $REG_SKEL global.get $SKEL_TRANSFORM_TYPES_PTR i32.add global.get $REG_SKEL_TYPES i32.store offset=0
    global.get $REG_SKEL global.get $SKEL_LABEL_COUNTS_PTR i32.add global.get $REG_SKEL_COUNTS i32.store offset=0
    global.get $REG_SKEL global.get $SKEL_LABEL_OFFSETS_PTR i32.add global.get $REG_SKEL_OFFSETS i32.store offset=0
    global.get $REG_SKEL global.get $SKEL_LABELS_PTR i32.add global.get $REG_SKEL_LABELS i32.store offset=0
    global.get $REG_SKEL global.get $SKEL_TRANSFORM_CAPACITY i32.add global.get $ANIM_FRAME_REGISTRY_TRANSFORM_CAP i32.store
    global.get $REG_SKEL global.get $SKEL_LABEL_CAPACITY i32.add global.get $ANIM_FRAME_REGISTRY_LABEL_CAP i32.store

    global.get $REG_DEC_FRAME global.get $FRAME_TRANSFORM_INDICES_PTR i32.add global.get $REG_DEC_INDICES i32.store offset=0
    global.get $REG_DEC_FRAME global.get $FRAME_TRANSFORM_X_PTR i32.add global.get $REG_DEC_X i32.store offset=0
    global.get $REG_DEC_FRAME global.get $FRAME_TRANSFORM_Y_PTR i32.add global.get $REG_DEC_Y i32.store offset=0
    global.get $REG_DEC_FRAME global.get $FRAME_TRANSFORM_Z_PTR i32.add global.get $REG_DEC_Z i32.store offset=0
    global.get $REG_DEC_FRAME global.get $FRAME_TRANSFORM_CAPACITY i32.add global.get $ANIM_FRAME_REGISTRY_TRANSFORM_CAP i32.store

    global.get $REG_DEC_SKEL global.get $SKEL_TRANSFORM_TYPES_PTR i32.add global.get $REG_DEC_SKEL_TYPES i32.store offset=0
    global.get $REG_DEC_SKEL global.get $SKEL_LABEL_COUNTS_PTR i32.add global.get $REG_DEC_SKEL_COUNTS i32.store offset=0
    global.get $REG_DEC_SKEL global.get $SKEL_LABEL_OFFSETS_PTR i32.add global.get $REG_DEC_SKEL_OFFS i32.store offset=0
    global.get $REG_DEC_SKEL global.get $SKEL_LABELS_PTR i32.add global.get $REG_DEC_SKEL_LABELS i32.store offset=0
    global.get $REG_DEC_SKEL global.get $SKEL_TRANSFORM_CAPACITY i32.add global.get $ANIM_FRAME_REGISTRY_TRANSFORM_CAP i32.store
    global.get $REG_DEC_SKEL global.get $SKEL_LABEL_CAPACITY i32.add global.get $ANIM_FRAME_REGISTRY_LABEL_CAP i32.store
  )

  ;; ── Registry operations ──

  (func (export "anim_frame_registry_clear")
    global.get $REG_KEY i32.const 0 i32.store offset=0
    global.get $REG_ACTIVE i32.const 0 i32.store offset=0
    global.get $REG_FRAME global.get $FRAME_TRANSFORM_COUNT i32.add i32.const 0 i32.store
    global.get $REG_SKEL global.get $SKEL_TRANSFORM_COUNT i32.add i32.const 0 i32.store
  )

  (func $anim_frame_registry_register_decoded (export "anim_frame_registry_register_decoded") (param $key i32) (param $frame i32) (param $skel i32) (result i32)
    (local $tc i32) (local $i i32) (local $src i32) (local $dst i32)
    local.get $key i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $skel i32.eqz if global.get $DEF_ERR_BOUNDS return end
    ;; copy frame transforms
    local.get $frame global.get $FRAME_TRANSFORM_COUNT i32.add i32.load
    local.tee $tc
    global.get $ANIM_FRAME_REGISTRY_TRANSFORM_CAP i32.load
    i32.gt_u
    if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_INDICES_PTR i32.add i32.load
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_X_PTR i32.add i32.load
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_Y_PTR i32.add i32.load
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame global.get $FRAME_TRANSFORM_Z_PTR i32.add i32.load
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    i32.const 0 local.set $i
    block $copy_frame_done
      loop $copy_frame
        local.get $i local.get $tc i32.ge_s if br $copy_frame_done end
        global.get $REG_INDICES local.get $i i32.const 2 i32.shl i32.add
        local.get $frame global.get $FRAME_TRANSFORM_INDICES_PTR i32.add i32.load
        local.get $i i32.const 2 i32.shl i32.add i32.load
        i32.store offset=0
        global.get $REG_X local.get $i i32.const 2 i32.shl i32.add
        local.get $frame global.get $FRAME_TRANSFORM_X_PTR i32.add i32.load
        local.get $i i32.const 2 i32.shl i32.add i32.load
        i32.store offset=0
        global.get $REG_Y local.get $i i32.const 2 i32.shl i32.add
        local.get $frame global.get $FRAME_TRANSFORM_Y_PTR i32.add i32.load
        local.get $i i32.const 2 i32.shl i32.add i32.load
        i32.store offset=0
        global.get $REG_Z local.get $i i32.const 2 i32.shl i32.add
        local.get $frame global.get $FRAME_TRANSFORM_Z_PTR i32.add i32.load
        local.get $i i32.const 2 i32.shl i32.add i32.load
        i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $copy_frame
      end
      unreachable
    end
    global.get $REG_FRAME global.get $FRAME_TRANSFORM_COUNT i32.add local.get $tc i32.store
    global.get $REG_FRAME global.get $FRAME_SKELETON_ID i32.add local.get $frame global.get $FRAME_SKELETON_ID i32.add i32.load i32.store
    global.get $REG_FRAME global.get $FRAME_SLOT_COUNT i32.add local.get $frame global.get $FRAME_SLOT_COUNT i32.add i32.load i32.store
    global.get $REG_FRAME global.get $FRAME_HAS_ALPHA i32.add local.get $frame global.get $FRAME_HAS_ALPHA i32.add i32.load i32.store
    ;; copy skeleton transforms
    local.get $skel global.get $SKEL_TRANSFORM_COUNT i32.add i32.load
    local.tee $tc
    global.get $ANIM_FRAME_REGISTRY_TRANSFORM_CAP i32.load
    i32.gt_u
    if global.get $DEF_ERR_BOUNDS return end
    local.get $skel global.get $SKEL_TRANSFORM_TYPES_PTR i32.add i32.load
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $skel global.get $SKEL_LABEL_COUNTS_PTR i32.add i32.load
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $skel global.get $SKEL_LABEL_OFFSETS_PTR i32.add i32.load
    i32.eqz if global.get $DEF_ERR_BOUNDS return end
    i32.const 0 local.set $i
    block $copy_skel_done
      loop $copy_skel
        local.get $i local.get $tc i32.ge_s if br $copy_skel_done end
        global.get $REG_SKEL_TYPES local.get $i i32.const 2 i32.shl i32.add
        local.get $skel global.get $SKEL_TRANSFORM_TYPES_PTR i32.add i32.load
        local.get $i i32.const 2 i32.shl i32.add i32.load
        i32.store offset=0
        global.get $REG_SKEL_COUNTS local.get $i i32.const 2 i32.shl i32.add
        local.get $skel global.get $SKEL_LABEL_COUNTS_PTR i32.add i32.load
        local.get $i i32.const 2 i32.shl i32.add i32.load
        i32.store offset=0
        global.get $REG_SKEL_OFFSETS local.get $i i32.const 2 i32.shl i32.add
        local.get $skel global.get $SKEL_LABEL_OFFSETS_PTR i32.add i32.load
        local.get $i i32.const 2 i32.shl i32.add i32.load
        i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $copy_skel
      end
      unreachable
    end
    global.get $REG_SKEL global.get $SKEL_TRANSFORM_COUNT i32.add local.get $tc i32.store
    global.get $REG_SKEL global.get $SKEL_STORED_COUNT i32.add local.get $skel global.get $SKEL_STORED_COUNT i32.add i32.load i32.store
    local.get $skel global.get $SKEL_TOTAL_LABEL_COUNT i32.add i32.load
    local.tee $tc
    global.get $ANIM_FRAME_REGISTRY_LABEL_CAP i32.load
    i32.gt_u
    if global.get $DEF_ERR_BOUNDS return end
    global.get $REG_SKEL global.get $SKEL_TOTAL_LABEL_COUNT i32.add local.get $tc i32.store
    local.get $skel global.get $SKEL_LABELS_PTR i32.add i32.load
    local.set $src
    i32.const 0 local.set $i
    block $copy_labels_done
      loop $copy_labels
        local.get $i local.get $tc i32.ge_s if br $copy_labels_done end
        global.get $REG_SKEL_LABELS local.get $i i32.const 2 i32.shl i32.add
        local.get $src local.get $i i32.const 2 i32.shl i32.add i32.load
        i32.store offset=0
        local.get $i i32.const 1 i32.add local.set $i
        br $copy_labels
      end
      unreachable
    end
    global.get $REG_KEY local.get $key i32.store offset=0
    global.get $REG_ACTIVE i32.const 1 i32.store offset=0
    global.get $DEF_OK
  )

  (func (export "anim_frame_registry_decode_and_register") (param $key i32) (param $frame_ptr i32) (param $frame_len i32) (param $skel_ptr i32) (param $skel_len i32) (result i32)
    local.get $key i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $frame_ptr i32.eqz if global.get $DEF_ERR_BOUNDS return end
    local.get $skel_ptr i32.eqz if global.get $DEF_ERR_BOUNDS return end
    ;; decode skeleton
    local.get $skel_ptr local.get $skel_len global.get $REG_DEC_SKEL call $anim_decode_skeleton_payload
    if
      global.get $REG_ACTIVE i32.const 0 i32.store offset=0
      global.get $DEF_ERR_BOUNDS return
    end
    ;; decode frame
    local.get $frame_ptr local.get $frame_len global.get $REG_DEC_FRAME global.get $REG_DEC_SKEL call $anim_decode_frame_payload
    if
      global.get $REG_ACTIVE i32.const 0 i32.store offset=0
      global.get $DEF_ERR_BOUNDS return
    end
    ;; register decoded data
    local.get $key global.get $REG_DEC_FRAME global.get $REG_DEC_SKEL call $anim_frame_registry_register_decoded
  )

  (func (export "anim_frame_registry_accumulate_type_delta") (param $key i32) (param $type i32) (param $out i32) (result i32)
    global.get $REG_ACTIVE i32.load offset=0
    i32.const 1 i32.ne
    if
      local.get $out
      if
        local.get $out i64.const 0 i64.store offset=0
        local.get $out i32.const 0 i32.store offset=8
      end
      global.get $DEF_ERR_BOUNDS return
    end
    global.get $REG_KEY i32.load offset=0
    local.get $key i32.ne
    if
      local.get $out
      if
        local.get $out i64.const 0 i64.store offset=0
        local.get $out i32.const 0 i32.store offset=8
      end
      global.get $DEF_ERR_BOUNDS return
    end
    global.get $REG_FRAME
    global.get $REG_SKEL
    local.get $type
    local.get $out
    call $anim_frame_accumulate_type_delta
  )

  (func (export "anim_frame_registry_accumulate_type_delta_target") (param $key i32) (param $type i32) (param $delta_out i32) (param $label_out i32) (result i32)
    global.get $REG_ACTIVE i32.load offset=0
    i32.const 1 i32.ne
    if
      local.get $delta_out
      if
        local.get $delta_out i64.const 0 i64.store offset=0
        local.get $delta_out i32.const 0 i32.store offset=8
      end
      local.get $label_out
      if
        local.get $label_out global.get $DEF_ID_NONE i32.store offset=0
      end
      global.get $DEF_ERR_BOUNDS return
    end
    global.get $REG_KEY i32.load offset=0
    local.get $key i32.ne
    if
      local.get $delta_out
      if
        local.get $delta_out i64.const 0 i64.store offset=0
        local.get $delta_out i32.const 0 i32.store offset=8
      end
      local.get $label_out
      if
        local.get $label_out global.get $DEF_ID_NONE i32.store offset=0
      end
      global.get $DEF_ERR_BOUNDS return
    end
    global.get $REG_FRAME
    global.get $REG_SKEL
    local.get $type
    local.get $delta_out
    local.get $label_out
    call $anim_frame_accumulate_type_delta_target
  )

  (func (export "anim_frame_registry_accumulate_type_group_yaws") (param $key i32) (param $type i32) (param $label_out i32) (param $yaw_out i32) (param $max_out i32) (result i32)
    global.get $REG_ACTIVE i32.load offset=0
    i32.const 1 i32.ne
    if
      local.get $label_out
      if
        local.get $label_out global.get $DEF_ID_NONE i32.store offset=0
      end
      local.get $yaw_out
      if
        local.get $yaw_out i32.const 0 i32.store offset=0
      end
      global.get $DEF_ERR_BOUNDS return
    end
    global.get $REG_KEY i32.load offset=0
    local.get $key i32.ne
    if
      local.get $label_out
      if
        local.get $label_out global.get $DEF_ID_NONE i32.store offset=0
      end
      local.get $yaw_out
      if
        local.get $yaw_out i32.const 0 i32.store offset=0
      end
      global.get $DEF_ERR_BOUNDS return
    end
    global.get $REG_FRAME
    global.get $REG_SKEL
    local.get $type
    local.get $label_out
    local.get $yaw_out
    local.get $max_out
    call $anim_frame_accumulate_type_group_yaws
  )
)
