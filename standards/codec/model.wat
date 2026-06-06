(module
  (import "math" "min" (func $min2 (param i32 i32) (result i32)))
  (import "math" "max" (func $max2 (param i32 i32) (result i32)))
  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun" "lo" (func $lo (param i64) (result i32)))
  (import "edgerun" "hi" (func $hi (param i64) (result i32)))
  (memory (export "memory") 1)
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

  (func $read_u16be_m (param $p i32) (param $o i32) (result i32 i32)
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


)
