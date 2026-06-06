(module
  (import "edgerun-core" "memory" (memory 1))
(import "crypto" "xtea_encrypt" (func $xtea_encrypt (param i32 i32 i32) (result i32)))
  (import "crypto" "xtea_decrypt" (func $xtea_decrypt (param i32 i32 i32) (result i32)))(func (export "proto_standard_id") (result i32) i32.const 710001)

  (global $OK                     i32 (i32.const 0))
  (global $ERR_BOUNDS             i32 (i32.const -1))
  (global $ERR_BAD_LENGTH         i32 (i32.const -2))
  (global $ERR_QUEUE_FULL         i32 (i32.const -3))
  (global $ERR_NO_FD              i32 (i32.const -4))
  (global $ERR_LOGIN_BLOCK_TODO   i32 (i32.const -6))
  (global $ERR_PARTIAL            i32 (i32.const -10))

  (global $BUF_DATA   i32 (i32.const 0))
  (global $BUF_CAP    i32 (i32.const 8))
  (global $BUF_POS    i32 (i32.const 16))
  (global $BUF_LIMIT  i32 (i32.const 24))
  (global $BUF_ISAAC  i32 (i32.const 32))
  (global $BUF_ERROR  i32 (i32.const 40))

  (global $NODE_OPCODE       i32 (i32.const 0))
  (global $NODE_LENGTH       i32 (i32.const 4))
  (global $NODE_PAYLOAD_LEN  i32 (i32.const 8))
  (global $NODE_BUF_PTR      i32 (i32.const 16))
  (global $NODE_BUF_CAP      i32 (i32.const 24))
  (global $NODE_SIZE         i32 (i32.const 32))

  (global $SESS_GAME_FD       i32 (i32.const 0))
  (global $SESS_FLAGS         i32 (i32.const 8))
  (global $SESS_IN_OPCODE     i32 (i32.const 32))
  (global $SESS_IN_LENGTH     i32 (i32.const 36))
  (global $SESS_IN_LENGTH_MODE i32 (i32.const 40))
  (global $SESS_IN_STATE      i32 (i32.const 44))
  (global $SESS_ACTIVE_NODE   i32 (i32.const 48))
  (global $SESS_QUEUED_BYTES  i32 (i32.const 56))
  (global $SESS_TICK_COUNT    i32 (i32.const 64))
  (global $SESS_LAST_TICK_STATUS i32 (i32.const 72))
  (global $SESS_SIZE          i32 (i32.const 80))

  (global $SESS_FLAG_GAME_OPEN  i32 (i32.const 1))
  (global $SESS_FLAG_SEED_READY i32 (i32.const 2))
  (global $SESS_FLAG_ISAAC_READY i32 (i32.const 4))

  (global $IN_STATE_OPCODE  i32 (i32.const 0))
  (global $IN_STATE_LENGTH  i32 (i32.const 1))
  (global $IN_STATE_PAYLOAD i32 (i32.const 2))

  (global $PACKET_LOGIN_RSA_PLAIN_BYTES i32 (i32.const 25))
  (global $PACKET_LOGIN_OUTER_MIN_CAP i32 (i32.const 20))
  (global $PACKET_LOGIN_OUTER_BUILD i32 (i32.const 238))
  (global $PACKET_LOGIN_OUTER_SUBBUILD i32 (i32.const 1))
  (global $PACKET_LOGIN_OUTER_PLATFORM_BYTE0 i32 (i32.const 1))
  (global $PACKET_LOGIN_OUTER_PLATFORM_BYTE1 i32 (i32.const 5))
  (global $PACKET_LOGIN_OUTER_PLATFORM_BYTE2 i32 (i32.const 0))
  (global $PACKET_LOGIN_RANDOM_DAT_BYTES i32 (i32.const 24))
  (global $PACKET_LOGIN_PLATFORM_INFO_MAX_BYTES i32 (i32.const 4096))
  (global $PACKET_LOGIN_ARCHIVE_CRC_MAX_BYTES i32 (i32.const 256))
  (global $PACKET_LOGIN_RESPONSE_ACCEPTED_PAYLOAD_BYTES i32 (i32.const 37))
  (global $PACKET_LOGIN_RESPONSE_PAYLOAD_MAX i32 (i32.const 256))

  (global $PACKET_LEN_VAR1 i32 (i32.const -1))
  (global $PACKET_LEN_VAR2 i32 (i32.const -2))

  (global $OUTER_CONF_OPCODE i32 (i32.const 1))
  (global $OUTER_CONF_LEN2 i32 (i32.const 2))
  (global $OUTER_CONF_BUILD_FIELDS i32 (i32.const 4))
  (global $OUTER_CONF_RSA_LEN_DATA i32 (i32.const 8))
  (global $OUTER_CONF_XTEA_START i32 (i32.const 16))
  (global $OUTER_CONF_AUTH_STRING i32 (i32.const 32))
  (global $OUTER_CONF_FLAGS_DIMENSIONS i32 (i32.const 64))
  (global $OUTER_CONF_RANDOM_DAT i32 (i32.const 128))
  (global $OUTER_CONF_SECOND_AUTH_STRING i32 (i32.const 256))
  (global $OUTER_CONF_PLATFORM_MARKER i32 (i32.const 512))
  (global $OUTER_CONF_PLATFORM_INFO_PRE_ZERO i32 (i32.const 1024))
  (global $OUTER_CONF_PLATFORM_INFO_BYTES i32 (i32.const 2048))
  (global $OUTER_CONF_PLATFORM_INFO_POST_ZERO i32 (i32.const 4096))
  (global $OUTER_CONF_PRE_CRC_ZERO i32 (i32.const 8192))
  (global $OUTER_CONF_XTEA_FINALIZED i32 (i32.const 16384))
  (global $OUTER_CONF_ARCHIVE_CRC_BLOCK i32 (i32.const 32768))
  (global $OUTER_CONF_REQUIRED_FINAL_MASK i32 (i32.const 49151))

  (global $LOGIN_RESPONSE_WAIT_STATUS i32 (i32.const 0))
  (global $LOGIN_RESPONSE_ACCEPTED_WAIT_LENGTH i32 (i32.const 1))
  (global $LOGIN_RESPONSE_STATUS_ONLY i32 (i32.const 2))
  (global $LOGIN_RESPONSE_ACCEPTED_WAIT_PAYLOAD i32 (i32.const 3))
  (global $LOGIN_RESPONSE_ACCEPTED_PAYLOAD_READY i32 (i32.const 4))
  (global $LOGIN_RESPONSE_ACCEPTED_CODE i32 (i32.const 2))
  (global $PACKET_SERVER_OPCODE_COUNT i32 (i32.const 139))
  (global $PACKET_CAPTURE_HEADER_BYTES i32 (i32.const 32))

  (global $LOGIN_STATE_RESET i32 (i32.const 0))
  (global $LOGIN_STATE_SEED_STATUS i32 (i32.const 1))
  (global $LOGIN_STATE_SEED_READY i32 (i32.const 2))
  (global $LOGIN_STATE_BLOCK_READY i32 (i32.const 3))
  (global $LOGIN_STATE_RSA_READY i32 (i32.const 4))
  (global $LOGIN_STATE_OUTER_TAIL_BLOCKED i32 (i32.const 5))
  (global $LOGIN_STATE_RESPONSE_WAIT i32 (i32.const 6))
  (global $LOGIN_STATE_RESPONSE_BLOCKED i32 (i32.const 7))
  (global $LOGIN_STATE_STATUS_BLOCKED i32 (i32.const 8))
  (global $LOGIN_STATE_OUTER_READY i32 (i32.const 9))

  (global $SCENE_EVENT_NONE i32 (i32.const 0))
  (global $SCENE_EVENT_LOCAL_PLAYER i32 (i32.const 1))
  (global $SCENE_EVENT_TYPE i32 (i32.const 0))
  (global $SCENE_EVENT_PLANE i32 (i32.const 4))
  (global $SCENE_EVENT_TILE_X i32 (i32.const 8))
  (global $SCENE_EVENT_TILE_Y i32 (i32.const 12))
  (global $SCENE_EVENT_FOOTPRINT i32 (i32.const 16))
  (global $SCENE_EVENT_REGION_BASE_X i32 (i32.const 20))
  (global $SCENE_EVENT_REGION_BASE_Y i32 (i32.const 24))
  (global $SCENE_EVENT_SEQ i32 (i32.const 28))
  (global $SCENE_EVENT_SIZE i32 (i32.const 32))

  (global $BUF_TINY_BYTES     i32 (i32.const 20))
  (global $BUF_SMALL_BYTES    i32 (i32.const 100))
  (global $BUF_MED_BYTES      i32 (i32.const 260))
  (global $BUF_LARGE_BYTES    i32 (i32.const 10000))
  (global $RX_BYTES           i32 (i32.const 10000))
  (global $NODE_RING_COUNT    i32 (i32.const 32))

  (global $MEM_TX_BUF_STATE       i32 (i32.const 100))
  (global $MEM_RX_BUF_STATE       i32 (i32.const 200))
  (global $MEM_TX_NODE_RING       i32 (i32.const 300))
  (global $MEM_TX_TINY_BUF        i32 (i32.const 1400))
  (global $MEM_TX_SMALL_BUF       i32 (i32.const 1420))
  (global $MEM_TX_MEDIUM_BUF      i32 (i32.const 1520))
  (global $MEM_TX_LARGE_BUF       i32 (i32.const 1780))
  (global $MEM_RX_BUF             i32 (i32.const 11780))
  (global $MEM_ISAAC_IN           i32 (i32.const 21780))
  (global $MEM_ISAAC_OUT          i32 (i32.const 23844))
  (global $MEM_XTEA_SEED          i32 (i32.const 25908))
  (global $MEM_LOGIN_STATE        i32 (i32.const 26000))
  (global $MEM_SCENE_EVENT        i32 (i32.const 26124))
  (global $MEM_LOGIN_RESPONSE_PAYLOAD i32 (i32.const 26156))
  (global $MEM_RETAINED_BITREADER i32 (i32.const 26412))
  (global $MEM_SERVER_LENGTHS     i32 (i32.const 26500))
  (global $MEM_SERVER_CANDIDATE_KINDS i32 (i32.const 26639))
  (global $MEM_SESSION_STATE      i32 (i32.const 26778))
  (global $MEM_SEED_BLOCK         i32 (i32.const 26858))
  (global $MEM_OUTER_BUF          i32 (i32.const 26900))

  (global $L_SRV_SEED     i32 (i32.const 0))
  (global $L_CLIENT_SEED  i32 (i32.const 8))
  (global $L_INBOUND_SEED i32 (i32.const 24))
  (global $L_XTEA_SEED2   i32 (i32.const 40))
  (global $L_BLOCK_PTR    i32 (i32.const 56))
  (global $L_BLOCK_CAP    i32 (i32.const 64))
  (global $L_BLOCK_LEN    i32 (i32.const 72))
  (global $L_RSA_OUT_PTR  i32 (i32.const 80))
  (global $L_RSA_OUT_LEN  i32 (i32.const 88))
  (global $L_OUTER_PTR    i32 (i32.const 96))
  (global $L_OUTER_CAP    i32 (i32.const 104))
  (global $L_OUTER_LEN    i32 (i32.const 112))
  (global $L_OUTER_XTEA_START i32 (i32.const 120))
  (global $L_OUTER_XTEA_END i32 (i32.const 128))
  (global $L_OUTER_LAST_XTEA_LEN i32 (i32.const 136))
  (global $L_OUTER_AUTH_PTR i32 (i32.const 144))
  (global $L_OUTER_AUTH_LEN i32 (i32.const 152))
  (global $L_OUTER_FLAGS_DIMENSIONS_VALID i32 (i32.const 160))
  (global $L_OUTER_FLAGS    i32 (i32.const 164))
  (global $L_OUTER_WIDTH    i32 (i32.const 168))
  (global $L_OUTER_HEIGHT   i32 (i32.const 172))
  (global $L_OUTER_RANDOM_DAT_PTR i32 (i32.const 176))
  (global $L_OUTER_SECOND_AUTH_PTR i32 (i32.const 184))
  (global $L_OUTER_SECOND_AUTH_LEN i32 (i32.const 192))
  (global $L_PLATFORM_MARKER_VALID i32 (i32.const 200))
  (global $L_PLATFORM_MARKER i32 (i32.const 204))
  (global $L_PLATFORM_INFO_PTR i32 (i32.const 208))
  (global $L_PLATFORM_INFO_LEN i32 (i32.const 216))
  (global $L_ARCHIVE_CRC_PTR i32 (i32.const 224))
  (global $L_ARCHIVE_CRC_LEN i32 (i32.const 232))
  (global $L_SEED_STATUS    i32 (i32.const 240))
  (global $L_LOGIN_STATE    i32 (i32.const 244))
  (global $L_RSA_STATUS     i32 (i32.const 248))
  (global $L_OUTER_STATUS   i32 (i32.const 252))
  (global $L_OUTER_CONFIDENCE i32 (i32.const 256))
  (global $L_RESPONSE_STATUS i32 (i32.const 260))
  (global $L_RESPONSE_STATE i32 (i32.const 264))
  (global $L_RESPONSE_CODE  i32 (i32.const 268))
  (global $L_RESPONSE_PAYLOAD_LEN i32 (i32.const 272))
  (global $L_BLOCKER_STATUS i32 (i32.const 276))
  (global $L_RESPONSE_BYTES i32 (i32.const 280))
  (global $L_ACCEPTED_DECODED i32 (i32.const 288))
  (global $L_ACCEPTED_RECONNECT_FLAG i32 (i32.const 292))
  (global $L_ACCEPTED_RECONNECT_SEED i32 (i32.const 296))
  (global $L_ACCEPTED_ACCOUNT_FLAGS i32 (i32.const 300))
  (global $L_ACCEPTED_MEMBER_FLAG i32 (i32.const 304))
  (global $L_ACCEPTED_WORLD_ID i32 (i32.const 308))
  (global $L_ACCEPTED_RIGHTS i32 (i32.const 312))
  (global $L_ACCEPTED_PRIMARY_ID i32 (i32.const 320))
  (global $L_ACCEPTED_SESSION_ID i32 (i32.const 328))
  (global $L_ACCEPTED_ACCOUNT_HASH i32 (i32.const 336))
  (global $L_CAPTURE_FD      i32 (i32.const 344))
  (global $L_CAPTURE_STATUS  i32 (i32.const 352))
  (global $L_FIRST_PAYLOAD_LEN i32 (i32.const 360))
  (global $L_FIRST_PAYLOAD_READY i32 (i32.const 368))
  (global $L_ACTOR_CANDIDATE_KIND i32 (i32.const 372))
  (global $L_SCENE_EVENT_SEQ_NEXT i32 (i32.const 400))
  (global $L_ISAAC_STATE_MEM_OFF i32 (i32.const 0))
  (global $L_ISAAC_STATE_RSL_OFF i32 (i32.const 1024))
  (global $L_ISAAC_STATE_AA_OFF i32 (i32.const 2048))
  (global $L_ISAAC_STATE_BB_OFF i32 (i32.const 2052))
  (global $L_ISAAC_STATE_CC_OFF i32 (i32.const 2056))
  (global $L_ISAAC_STATE_COUNT_OFF i32 (i32.const 2060))

  ;; ── Helper: write u32 big-endian ────────────────────────────────
  (func $store_be32 (param $p i32) (param $v i32)
    local.get $p local.get $v i32.const 24 i32.shr_u i32.store8
    local.get $p i32.const 1 i32.add local.get $v i32.const 16 i32.shr_u i32.const 0xff i32.and i32.store8
    local.get $p i32.const 2 i32.add local.get $v i32.const 8 i32.shr_u i32.const 0xff i32.and i32.store8
    local.get $p i32.const 3 i32.add local.get $v i32.const 0xff i32.and i32.store8
  )

  ;; ── Helper: read u32 big-endian ─────────────────────────────────
  (func $load_be32 (param $p i32) (result i32)
    local.get $p i32.load8_u i32.const 24 i32.shl
    local.get $p i32.const 1 i32.add i32.load8_u i32.const 16 i32.shl i32.or
    local.get $p i32.const 2 i32.add i32.load8_u i32.const 8 i32.shl i32.or
    local.get $p i32.const 3 i32.add i32.load8_u i32.or
  )

  ;; ── Helper: memset ──────────────────────────────────────────────
  (func $memset (param $base i32) (param $len i32) (param $val i32)
    (local $i i32)
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $base local.get $i i32.add local.get $val i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
  )

  ;; ── Helper: read u64 big-endian ─────────────────────────────────
  (func $load_be64 (param $p i32) (result i64)
    local.get $p i32.load8_u i64.extend_i32_u i64.const 56 i64.shl
    local.get $p i32.const 1 i32.add i32.load8_u i64.extend_i32_u i64.const 48 i64.shl i64.or
    local.get $p i32.const 2 i32.add i32.load8_u i64.extend_i32_u i64.const 40 i64.shl i64.or
    local.get $p i32.const 3 i32.add i32.load8_u i64.extend_i32_u i64.const 32 i64.shl i64.or
    local.get $p i32.const 4 i32.add i32.load8_u i64.extend_i32_u i64.const 24 i64.shl i64.or
    local.get $p i32.const 5 i32.add i32.load8_u i64.extend_i32_u i64.const 16 i64.shl i64.or
    local.get $p i32.const 6 i32.add i32.load8_u i64.extend_i32_u i64.const 8 i64.shl i64.or
    local.get $p i32.const 7 i32.add i32.load8_u i64.extend_i32_u i64.or
  )

  ;; ── Helper: memcpy ──────────────────────────────────────────────
  (func $memcpy (param $src i32) (param $dst i32) (param $len i32)
    (local $i i32)
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $dst local.get $i i32.add local.get $src local.get $i i32.add i32.load8_u i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
  )

  ;; ── Internal append_u8 (no export overhead) ─────────────────────
  (func $append_u8 (param $state i32) (param $val i32) (result i32)
    (local $pos i32) (local $data i32)
    local.get $state i32.load offset=16 local.set $pos
    local.get $pos
    local.get $state i32.load offset=8
    i32.ge_u
    if global.get $ERR_BOUNDS return end
    local.get $state i32.load offset=0 local.set $data
    local.get $data local.get $pos i32.add local.get $val i32.store8
    local.get $state local.get $pos i32.const 1 i32.add i32.store offset=16
    i32.const 0
  )

  ;; ── Internal read_u8 (no export overhead) ───────────────────────
  (func $read_u8 (param $state i32) (result i32)
    (local $pos i32) (local $data i32)
    local.get $state i32.load offset=16 local.set $pos
    local.get $pos
    local.get $state i32.load offset=24
    i32.ge_u
    if global.get $ERR_BOUNDS return end
    local.get $state i32.load offset=0 local.set $data
    local.get $data local.get $pos i32.add i32.load8_u
    local.get $state local.get $pos i32.const 1 i32.add i32.store offset=16
  )

  ;; ── ISAAC next-word (internal) ──────────────────────────────────
  (func $isaac_next_local (param $state i32) (result i32)
    (local $cnt i32)
    local.get $state i32.load offset=2060 local.tee $cnt
    if else
      local.get $state call $isaac_generate_local
      local.get $state i32.const 256 i32.store offset=2060
      i32.const 256 local.set $cnt
    end
    local.get $state
    local.get $cnt i32.const 1 i32.sub local.tee $cnt
    i32.store offset=2060
    local.get $state i32.const 1024 i32.add
    local.get $cnt i32.const 2 i32.shl i32.add
    i32.load
    i32.const 0xff i32.and
  )

  (func $isaac_generate_local (param $s i32)
    (local $i i32) (local $x i32) (local $aa i32) (local $y i32)
    (local $aai i32) (local $xi i32) (local $yi i32) (local $bb i32)
    local.get $s i32.const 2056 i32.add
    local.get $s i32.const 2056 i32.add i32.load i32.const 1 i32.add
    local.tee $aa
    i32.store
    local.get $s i32.const 2052 i32.add
    local.get $s i32.const 2052 i32.add i32.load
    local.get $aa i32.add
    i32.store
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 256 i32.ge_u br_if $done
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.load local.set $x
      local.get $s i32.const 2048 i32.add i32.load local.set $aa
      local.get $i i32.const 3 i32.and
      if
        local.get $i i32.const 3 i32.and i32.const 1 i32.eq
        if local.get $aa i32.const 6 i32.shr_u local.get $aa i32.xor local.set $aa
        else
          local.get $i i32.const 3 i32.and i32.const 2 i32.eq
          if local.get $aa i32.const 2 i32.shl local.get $aa i32.xor local.set $aa
          else local.get $aa i32.const 16 i32.shr_u local.get $aa i32.xor local.set $aa
          end
        end
      else
        local.get $aa i32.const 13 i32.shl local.get $aa i32.xor local.set $aa
      end
      local.get $i i32.const 128 i32.add i32.const 255 i32.and local.set $aai
      local.get $s i32.const 2048 i32.add
      local.get $aa
      local.get $s local.get $aai i32.const 2 i32.shl i32.add i32.load
      i32.add
      local.tee $aa
      i32.store
      local.get $x i32.const 2 i32.shr_u i32.const 255 i32.and local.set $xi
      local.get $s local.get $i i32.const 2 i32.shl i32.add
      local.get $s local.get $xi i32.const 2 i32.shl i32.add i32.load
      local.get $aa i32.add
      local.get $s i32.const 2052 i32.add i32.load
      i32.add
      local.tee $y
      i32.store
      local.get $y i32.const 10 i32.shr_u i32.const 255 i32.and local.set $yi
      local.get $s i32.const 2052 i32.add
      local.get $s local.get $yi i32.const 2 i32.shl i32.add i32.load
      local.get $x i32.add
      local.tee $bb
      i32.store
      local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add
      local.get $bb i32.store
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
  )

  ;; ══════════════════════════════════════════════════════════════════
  ;; EXPORTED FUNCTIONS
  ;; ══════════════════════════════════════════════════════════════════

  ;; ── Buffer operations ───────────────────────────────────────────

  (func $packet_buffer_reset (export "packet_buffer_reset") (param $state i32) (param $data i32) (param $cap i32) (result i32)
    local.get $state local.get $data i32.store offset=0
    local.get $state local.get $cap i32.store offset=8
    local.get $state i64.const 0 i64.store offset=16
    local.get $state local.get $cap i32.store offset=24
    local.get $state i64.const 0 i64.store offset=32
    local.get $state global.get $OK i32.store offset=40
    i32.const 0
  )

  (func (export "packet_buffer_clear") (param $state i32) (result i32)
    local.get $state i64.const 0 i64.store offset=16
    local.get $state global.get $OK i32.store offset=40
    i32.const 0
  )

  (func (export "packet_buffer_set_limit") (param $state i32) (param $limit i32) (result i32)
    local.get $limit
    local.get $state i32.load offset=8
    i32.gt_u
    if
      local.get $state global.get $ERR_BOUNDS i32.store offset=40
      global.get $ERR_BOUNDS return
    end
    local.get $state local.get $limit i32.store offset=24
    i32.const 0
  )

  (func (export "packet_buffer_remaining") (param $state i32) (result i32)
    local.get $state i32.load offset=24
    local.get $state i32.load offset=16
    i32.sub
  )

  (func (export "packet_buffer_append_u8") (param $state i32) (param $val i32) (result i32)
    local.get $state local.get $val call $append_u8
  )

  (func (export "packet_buffer_append_u16be") (param $state i32) (param $val i32) (result i32)
    (local $r i32)
    local.get $state local.get $val i32.const 8 i32.shr_u i32.const 0xff i32.and call $append_u8 local.tee $r
    if local.get $r return end
    local.get $state local.get $val i32.const 0xff i32.and call $append_u8
  )

  (func (export "packet_buffer_append_u24be") (param $state i32) (param $val i32) (result i32)
    (local $r i32)
    local.get $state local.get $val i32.const 16 i32.shr_u i32.const 0xff i32.and call $append_u8 local.tee $r
    if local.get $r return end
    local.get $state local.get $val i32.const 8 i32.shr_u i32.const 0xff i32.and call $append_u8 local.tee $r
    if local.get $r return end
    local.get $state local.get $val i32.const 0xff i32.and call $append_u8
  )

  (func (export "packet_buffer_append_u32be") (param $state i32) (param $val i32) (result i32)
    (local $r i32)
    local.get $state local.get $val i32.const 24 i32.shr_u call $append_u8 local.tee $r
    if local.get $r return end
    local.get $state local.get $val i32.const 16 i32.shr_u i32.const 0xff i32.and call $append_u8 local.tee $r
    if local.get $r return end
    local.get $state local.get $val i32.const 8 i32.shr_u i32.const 0xff i32.and call $append_u8 local.tee $r
    if local.get $r return end
    local.get $state local.get $val i32.const 0xff i32.and call $append_u8
  )

  (func (export "packet_buffer_append_bytes") (param $state i32) (param $src i32) (param $len i32) (result i32)
    (local $pos i32) (local $end i32) (local $data i32) (local $i i32)
    local.get $state i32.load offset=16 local.set $pos
    local.get $pos local.get $len i32.add local.set $end
    local.get $end
    local.get $state i32.load offset=8
    i32.gt_u
    if
      local.get $state global.get $ERR_BOUNDS i32.store offset=40
      global.get $ERR_BOUNDS return
    end
    local.get $state i32.load offset=0 local.set $data
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $data local.get $pos local.get $i i32.add i32.add
      local.get $src local.get $i i32.add i32.load8_u i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $state local.get $end i32.store offset=16
    i32.const 0
  )

  (func (export "packet_buffer_read_u8") (param $state i32) (result i32)
    local.get $state call $read_u8
  )

  (func (export "packet_buffer_read_u16be") (param $state i32) (result i32)
    (local $hi i32) (local $lo i32)
    local.get $state call $read_u8 local.tee $hi
    if local.get $hi return end
    local.get $state call $read_u8 local.tee $lo
    if local.get $lo return end
    local.get $hi i32.const 8 i32.shl local.get $lo i32.or
  )

  (func (export "packet_buffer_read_i16be") (param $state i32) (result i32)
    (local $hi i32) (local $lo i32)
    local.get $state call $read_u8 local.tee $hi
    if local.get $hi return end
    local.get $state call $read_u8 local.tee $lo
    if local.get $lo return end
    local.get $hi i32.const 8 i32.shl local.get $lo i32.or
    i32.extend16_s
  )

  (func (export "packet_buffer_read_u24be") (param $state i32) (result i32)
    (local $b1 i32) (local $b2 i32) (local $b3 i32)
    local.get $state call $read_u8 local.tee $b1
    if local.get $b1 return end
    local.get $state call $read_u8 local.tee $b2
    if local.get $b2 return end
    local.get $state call $read_u8 local.tee $b3
    if local.get $b3 return end
    local.get $b1 i32.const 16 i32.shl
    local.get $b2 i32.const 8 i32.shl i32.or
    local.get $b3 i32.or
  )

  (func (export "packet_buffer_read_u32be") (param $state i32) (result i32)
    (local $b1 i32) (local $b2 i32) (local $b3 i32) (local $b4 i32)
    local.get $state call $read_u8 local.tee $b1
    if local.get $b1 return end
    local.get $state call $read_u8 local.tee $b2
    if local.get $b2 return end
    local.get $state call $read_u8 local.tee $b3
    if local.get $b3 return end
    local.get $state call $read_u8 local.tee $b4
    if local.get $b4 return end
    local.get $b1 i32.const 24 i32.shl
    local.get $b2 i32.const 16 i32.shl i32.or
    local.get $b3 i32.const 8 i32.shl i32.or
    local.get $b4 i32.or
  )

  (func $packet_buffer_write_opcode (export "packet_buffer_write_opcode") (param $state i32) (param $opcode i32) (result i32)
    (local $isaac i32)
    local.get $state i32.load offset=32 local.set $isaac
    local.get $state
    local.get $opcode
    local.get $isaac call $isaac_next_local
    i32.add
    i32.const 0xff i32.and
    call $append_u8
  )

  (func (export "packet_buffer_read_opcode") (param $state i32) (result i32)
    (local $isaac i32) (local $mask i32) (local $op i32) (local $ext i32) (local $tmp i32)
    local.get $state i32.load offset=32 local.set $isaac
    local.get $isaac call $isaac_next_local local.set $mask
    local.get $state call $read_u8 local.tee $op
    if local.get $op return end
    local.get $op local.get $mask i32.sub i32.const 0xff i32.and local.set $op
    local.get $op i32.const 128 i32.lt_u
    if local.get $op return end
    local.get $op i32.const 128 i32.sub i32.const 8 i32.shl local.set $ext
    local.get $isaac call $isaac_next_local local.set $mask
    local.get $state call $read_u8 local.tee $op
    if local.get $op return end
    local.get $ext
    local.get $op local.get $mask i32.sub i32.const 0xff i32.and
    i32.or
  )

  ;; ── XTEA wrappers ───────────────────────────────────────────────

  (func $packet_xtea_encrypt (export "packet_xtea_encrypt") (param $buf i32) (param $len i32) (param $key i32) (result i32)
    local.get $buf local.get $len local.get $key call $xtea_encrypt
  )

  (func (export "packet_xtea_decrypt") (param $buf i32) (param $len i32) (param $key i32) (result i32)
    local.get $buf local.get $len local.get $key call $xtea_decrypt
  )

  ;; ── Session management ──────────────────────────────────────────

  (func $session_reset_inner
    global.get $MEM_SESSION_STATE
    global.get $SESS_SIZE
    i32.const 0
    call $memset
    global.get $MEM_SESSION_STATE global.get $SESS_GAME_FD i32.add i64.const -1 i64.store
    global.get $MEM_SESSION_STATE global.get $SESS_IN_OPCODE i32.add i32.const -1 i32.store
    global.get $MEM_SESSION_STATE global.get $SESS_IN_LENGTH i32.add i32.const -1 i32.store
    global.get $MEM_SESSION_STATE global.get $SESS_IN_STATE i32.add global.get $IN_STATE_OPCODE i32.store
  )

  (func (export "packet_session_reset") (result i32)
    call $session_reset_inner
    i32.const 0
  )

  (func (export "packet_core_init") (result i32)
    call $session_reset_inner
    global.get $MEM_TX_BUF_STATE
    global.get $MEM_TX_MEDIUM_BUF
    global.get $BUF_MED_BYTES
    call $packet_buffer_reset drop
    global.get $MEM_RX_BUF_STATE
    global.get $MEM_RX_BUF
    global.get $RX_BYTES
    call $packet_buffer_reset drop
    global.get $MEM_ISAAC_OUT
    global.get $MEM_TX_BUF_STATE i32.const 32 i32.add i32.store
    global.get $MEM_ISAAC_IN
    global.get $MEM_RX_BUF_STATE i32.const 32 i32.add i32.store
    global.get $MEM_LOGIN_STATE global.get $L_CAPTURE_FD i32.add i64.const -1 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_CAPTURE_STATUS i32.add global.get $OK i32.store
    i32.const 0
  )

  (func $packet_session_set_game_fd (export "packet_session_set_game_fd") (param $fd i32) (result i32)
    global.get $MEM_SESSION_STATE global.get $SESS_GAME_FD i32.add
    local.get $fd i64.extend_i32_s
    i64.store
    local.get $fd i32.const 0 i32.ge_s
    if
      global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add
      global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
      i64.const 1 i64.or
      i64.store
    else
      global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add
      global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
      i64.const -2 i64.and
      i64.store
    end
    i32.const 0
  )

  (func (export "packet_session_get_game_fd") (result i64)
    global.get $MEM_SESSION_STATE global.get $SESS_GAME_FD i32.add i64.load
  )

  (func $packet_session_attach_ciphers (export "packet_session_attach_ciphers") (param $in_isaac i32) (param $out_isaac i32) (result i32)
    global.get $MEM_RX_BUF_STATE i32.const 32 i32.add local.get $in_isaac i32.store
    global.get $MEM_TX_BUF_STATE i32.const 32 i32.add local.get $out_isaac i32.store
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
    i64.const 4 i64.or
    i64.store
    i32.const 0
  )

  (func (export "packet_capture_set_fd") (param $fd i32) (result i32)
    global.get $MEM_LOGIN_STATE global.get $L_CAPTURE_FD i32.add local.get $fd i64.extend_i32_s i64.store
    global.get $MEM_LOGIN_STATE global.get $L_CAPTURE_STATUS i32.add global.get $OK i32.store
    i32.const 0
  )

  (func (export "packet_capture_get_status") (result i32 i64)
    global.get $MEM_LOGIN_STATE global.get $L_CAPTURE_STATUS i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_CAPTURE_FD i32.add i64.load
  )

  ;; ── Login seed/state ────────────────────────────────────────────

  (func (export "packet_login_seed_response_set") (param $status i32) (param $seed_ptr i32) (result i32)
    global.get $MEM_LOGIN_STATE global.get $L_SEED_STATUS i32.add local.get $status i32.store
    global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add global.get $LOGIN_STATE_SEED_STATUS i32.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_NO_FD i32.store
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
    i64.const -3 i64.and
    i64.store
    call $packet_login_response_reset drop
    local.get $status
    if
      global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add global.get $LOGIN_STATE_STATUS_BLOCKED i32.store
      global.get $ERR_NO_FD return
    end
    local.get $seed_ptr i32.eqz
    if
      global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add global.get $LOGIN_STATE_RESET i32.store
      global.get $ERR_NO_FD return
    end
    global.get $MEM_LOGIN_STATE global.get $L_SRV_SEED i32.add
    local.get $seed_ptr i64.load
    i64.store
    global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add global.get $LOGIN_STATE_SEED_READY i32.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_NO_FD i32.store
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
    i64.const 2 i64.or
    i64.store
    i32.const 0
  )

  (func (export "packet_login_get_seed_state") (result i32 i32 i64)
    global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_SEED_STATUS i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_SRV_SEED i32.add i64.load
  )

  (func $login_state_reset_inner
    global.get $MEM_LOGIN_STATE global.get $L_SRV_SEED i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCK_PTR i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCK_CAP i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCK_LEN i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_RSA_OUT_PTR i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_RSA_OUT_LEN i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_PTR i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_CAP i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_START i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_END i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_LAST_XTEA_LEN i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_PTR i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_LEN i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_FLAGS_DIMENSIONS_VALID i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_FLAGS i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_WIDTH i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_HEIGHT i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_RANDOM_DAT_PTR i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_PTR i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_LEN i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_MARKER_VALID i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_MARKER i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_PTR i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_LEN i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_PTR i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_LEN i32.add i64.const 0 i64.store
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
    i64.const -7 i64.and
    i64.store
    global.get $MEM_LOGIN_STATE global.get $L_SEED_STATUS i32.add i32.const -1 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add global.get $LOGIN_STATE_RESET i32.store
    global.get $MEM_LOGIN_STATE global.get $L_RSA_STATUS i32.add global.get $ERR_NO_FD i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add global.get $ERR_NO_FD i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_NO_FD i32.store
  )

  (func (export "packet_login_state_reset") (result i32)
    call $login_state_reset_inner
    call $packet_login_response_reset drop
    i32.const 0
  )

  (func (export "packet_login_get_progress_state") (result i32 i32 i32 i32 i32)
    global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_SEED_STATUS i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_RSA_STATUS i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add i32.load
  )

  (func (export "packet_login_get_xtea_seed") (result i32)
    global.get $MEM_XTEA_SEED
  )

  (func (export "packet_login_get_isaac_states") (result i32 i32)
    global.get $MEM_ISAAC_IN
    global.get $MEM_ISAAC_OUT
  )

  ;; ── Login response ──────────────────────────────────────────────

  (func $packet_login_response_reset (export "packet_login_response_reset") (result i32)
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATUS i32.add global.get $ERR_PARTIAL i32.store
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATE i32.add global.get $LOGIN_RESPONSE_WAIT_STATUS i32.store
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_CODE i32.add i32.const -1 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_PAYLOAD_LEN i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_BYTES i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_DECODED i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_RECONNECT_FLAG i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_RECONNECT_SEED i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_ACCOUNT_FLAGS i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_MEMBER_FLAG i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_WORLD_ID i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_RIGHTS i32.add i32.const 0 i32.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_PRIMARY_ID i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_SESSION_ID i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_ACCOUNT_HASH i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_RESPONSE_PAYLOAD
    global.get $PACKET_LOGIN_RESPONSE_PAYLOAD_MAX
    i32.const 0
    call $memset
    global.get $ERR_PARTIAL
  )

  (func (export "packet_login_get_response_status") (result i32 i32 i32 i64 i32)
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATUS i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATE i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_CODE i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_BYTES i32.add i64.load
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_PAYLOAD_LEN i32.add i32.load
  )

  ;; ── ISAAC wrappers ──────────────────────────────────────────────

  ;; ISAAC mix: 8 i32 params -> 8 i32 results
  (func $mix (param $a i32)(param $b i32)(param $c i32)(param $d i32)
             (param $e i32)(param $f i32)(param $g i32)(param $h i32)
             (result i32 i32 i32 i32 i32 i32 i32 i32)
    local.get $b i32.const 11 i32.shl local.get $a i32.xor local.set $a
    local.get $d local.get $a i32.add local.set $d
    local.get $b local.get $c i32.add local.tee $b
    local.get $c i32.const 2 i32.shr_u i32.xor local.set $b
    local.get $e local.get $b i32.add local.set $e
    local.get $c local.get $d i32.add local.tee $c
    local.get $d i32.const 8 i32.shl i32.xor local.set $c
    local.get $f local.get $c i32.add local.set $f
    local.get $d local.get $e i32.add local.set $d
    local.get $e i32.const 16 i32.shr_u local.get $d i32.xor local.set $d
    local.get $g local.get $d i32.add local.set $g
    local.get $e local.get $f i32.add local.tee $e
    local.get $f i32.const 10 i32.shl i32.xor local.set $e
    local.get $h local.get $e i32.add local.set $h
    local.get $f local.get $g i32.add local.tee $f
    local.get $g i32.const 4 i32.shr_u i32.xor local.set $f
    local.get $a local.get $f i32.add local.set $a
    local.get $g local.get $h i32.add local.tee $g
    local.get $h i32.const 8 i32.shl i32.xor local.set $g
    local.get $b local.get $g i32.add local.set $b
    local.get $h local.get $a i32.add local.tee $h
    local.get $a i32.const 9 i32.shr_u i32.xor local.set $h
    local.get $c local.get $h i32.add local.set $c
    local.get $a local.get $b i32.add local.set $a
    local.get $a local.get $b local.get $c local.get $d
    local.get $e local.get $f local.get $g local.get $h
  )

  (func $packet_isaac_seed (export "packet_isaac_seed") (param $s i32) (param $seed i32) (param $n i32) (result i32)
    (local $i i32) (local $a i32) (local $b i32) (local $c i32)
    (local $d i32) (local $e i32) (local $f i32) (local $g i32) (local $h i32)
    (local $cnt i32)
    i32.const 0 local.set $i
    block $zero_done
    loop $zero_loop
      local.get $i i32.const 516 i32.ge_u br_if $zero_done
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 0 i32.store
      local.get $i i32.const 1 i32.add local.set $i
      br $zero_loop
    end
    end
    local.get $n i32.const 256 i32.gt_u
    if i32.const 256 local.set $n end
    i32.const 0 local.set $i
    block $copy_done
    loop $copy_loop
      local.get $i local.get $n i32.ge_u br_if $copy_done
      local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add
      local.get $seed local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.store
      local.get $i i32.const 1 i32.add local.set $i
      br $copy_loop
    end
    end
    i32.const 0x9e3779b9
    local.tee $a local.set $b local.get $a local.set $c
    local.get $a local.set $d local.get $a local.set $e
    local.get $a local.set $f local.get $a local.set $g
    local.get $a local.set $h
    local.get $a local.get $b local.get $c local.get $d
    local.get $e local.get $f local.get $g local.get $h call $mix
    local.set $h local.set $g local.set $f local.set $e
    local.set $d local.set $c local.set $b local.set $a
    local.get $a local.get $b local.get $c local.get $d
    local.get $e local.get $f local.get $g local.get $h call $mix
    local.set $h local.set $g local.set $f local.set $e
    local.set $d local.set $c local.set $b local.set $a
    local.get $a local.get $b local.get $c local.get $d
    local.get $e local.get $f local.get $g local.get $h call $mix
    local.set $h local.set $g local.set $f local.set $e
    local.set $d local.set $c local.set $b local.set $a
    local.get $a local.get $b local.get $c local.get $d
    local.get $e local.get $f local.get $g local.get $h call $mix
    local.set $h local.set $g local.set $f local.set $e
    local.set $d local.set $c local.set $b local.set $a
    i32.const 0 local.set $i
    block $fp_done
    loop $fp_loop
      local.get $i i32.const 256 i32.ge_u br_if $fp_done
      local.get $a local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.load i32.add local.set $a
      local.get $b local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 4 i32.add i32.load i32.add local.set $b
      local.get $c local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 8 i32.add i32.load i32.add local.set $c
      local.get $d local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 12 i32.add i32.load i32.add local.set $d
      local.get $e local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 16 i32.add i32.load i32.add local.set $e
      local.get $f local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 20 i32.add i32.load i32.add local.set $f
      local.get $g local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 24 i32.add i32.load i32.add local.set $g
      local.get $h local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 28 i32.add i32.load i32.add local.set $h
      local.get $a local.get $b local.get $c local.get $d
      local.get $e local.get $f local.get $g local.get $h call $mix
      local.set $h local.set $g local.set $f local.set $e
      local.set $d local.set $c local.set $b local.set $a
      local.get $s local.get $i i32.const 2 i32.shl i32.add local.get $a i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 4 i32.add local.get $b i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 8 i32.add local.get $c i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 12 i32.add local.get $d i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 16 i32.add local.get $e i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 20 i32.add local.get $f i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 24 i32.add local.get $g i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 28 i32.add local.get $h i32.store
      local.get $i i32.const 8 i32.add local.set $i
      br $fp_loop
    end
    end
    i32.const 0 local.set $i
    block $sp_done
    loop $sp_loop
      local.get $i i32.const 256 i32.ge_u br_if $sp_done
      local.get $a local.get $s local.get $i i32.const 2 i32.shl i32.add i32.load i32.add local.set $a
      local.get $b local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 4 i32.add i32.load i32.add local.set $b
      local.get $c local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 8 i32.add i32.load i32.add local.set $c
      local.get $d local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 12 i32.add i32.load i32.add local.set $d
      local.get $e local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 16 i32.add i32.load i32.add local.set $e
      local.get $f local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 20 i32.add i32.load i32.add local.set $f
      local.get $g local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 24 i32.add i32.load i32.add local.set $g
      local.get $h local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 28 i32.add i32.load i32.add local.set $h
      local.get $a local.get $b local.get $c local.get $d
      local.get $e local.get $f local.get $g local.get $h call $mix
      local.set $h local.set $g local.set $f local.set $e
      local.set $d local.set $c local.set $b local.set $a
      local.get $s local.get $i i32.const 2 i32.shl i32.add local.get $a i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 4 i32.add local.get $b i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 8 i32.add local.get $c i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 12 i32.add local.get $d i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 16 i32.add local.get $e i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 20 i32.add local.get $f i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 24 i32.add local.get $g i32.store
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 28 i32.add local.get $h i32.store
      local.get $i i32.const 8 i32.add local.set $i
      br $sp_loop
    end
    end
    local.get $s call $isaac_generate_local
    local.get $s i32.const 2060 i32.add i32.const 256 i32.store
    i32.const 0
  )

  (func (export "packet_isaac_next") (param $state i32) (result i32)
    local.get $state call $isaac_next_local
  )

  (func (export "packet_isaac_peek") (param $state i32) (result i32)
    (local $cnt i32)
    local.get $state i32.load offset=2060 local.tee $cnt
    if else
      local.get $state call $isaac_generate_local
      local.get $state i32.const 256 i32.store offset=2060
      i32.const 256 local.set $cnt
    end
    local.get $cnt i32.const 1 i32.sub
    i32.const 2 i32.shl
    local.get $state i32.add
    i32.const 1024 i32.add
    i32.load
    i32.const 0xff i32.and
  )

  ;; ── Login set_outer setters ─────────────────────────────────────

  (func (export "packet_login_set_outer_auth_string") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $ptr i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_LEN i32.add i64.const 0 i64.store
      i32.const 0 return
    end
    local.get $len i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_LEN i32.add i64.const 0 i64.store
      i32.const 0 return
    end
    local.get $len i32.const 65535 i32.gt_u if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_LEN i32.add i64.const 0 i64.store
      global.get $ERR_BAD_LENGTH return
    end
    block $scan
    loop $scan_loop
      local.get $i local.get $len i32.ge_u br_if $scan
      local.get $ptr local.get $i i32.add i32.load8_u i32.eqz if
        global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_PTR i32.add i64.const 0 i64.store
        global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_LEN i32.add i64.const 0 i64.store
        global.get $ERR_BAD_LENGTH return
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $scan_loop
    end
    end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_PTR i32.add local.get $ptr i64.extend_i32_u i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_LEN i32.add local.get $len i64.extend_i32_u i64.store
    i32.const 0
  )

  (func (export "packet_login_set_outer_flags_dimensions") (param $flags i32) (param $width i32) (param $height i32) (result i32)
    local.get $flags i32.const 255 i32.gt_u if global.get $ERR_BAD_LENGTH return end
    local.get $width i32.eqz if global.get $ERR_BAD_LENGTH return end
    local.get $width i32.const 65535 i32.gt_u if global.get $ERR_BAD_LENGTH return end
    local.get $height i32.eqz if global.get $ERR_BAD_LENGTH return end
    local.get $height i32.const 65535 i32.gt_u if global.get $ERR_BAD_LENGTH return end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_FLAGS i32.add local.get $flags i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_WIDTH i32.add local.get $width i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_HEIGHT i32.add local.get $height i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_FLAGS_DIMENSIONS_VALID i32.add i32.const 1 i32.store
    i32.const 0
  )

  (func (export "packet_login_set_outer_random_dat_bytes") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_RANDOM_DAT_PTR i32.add i64.const 0 i64.store
      i32.const 0 return
    end
    local.get $len global.get $PACKET_LOGIN_RANDOM_DAT_BYTES i32.ne if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_RANDOM_DAT_PTR i32.add i64.const 0 i64.store
      global.get $ERR_BAD_LENGTH return
    end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_RANDOM_DAT_PTR i32.add local.get $ptr i64.extend_i32_u i64.store
    i32.const 0
  )

  (func (export "packet_login_set_outer_second_auth_string") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_LEN i32.add i64.const 0 i64.store
      i32.const 0 return
    end
    local.get $len i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_LEN i32.add i64.const 0 i64.store
      i32.const 0 return
    end
    local.get $len i32.const 65535 i32.gt_u if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_LEN i32.add i64.const 0 i64.store
      global.get $ERR_BAD_LENGTH return
    end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_PTR i32.add local.get $ptr i64.extend_i32_u i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_LEN i32.add local.get $len i64.extend_i32_u i64.store
    i32.const 0
  )

  (func (export "packet_login_set_outer_platform_marker") (param $marker i32) (result i32)
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_MARKER i32.add local.get $marker i32.store
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_MARKER_VALID i32.add i32.const 1 i32.store
    i32.const 0
  )

  (func (export "packet_login_set_outer_platform_info_bytes") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_LEN i32.add i64.const 0 i64.store
      i32.const 0 return
    end
    local.get $len i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_LEN i32.add i64.const 0 i64.store
      i32.const 0 return
    end
    local.get $len global.get $PACKET_LOGIN_PLATFORM_INFO_MAX_BYTES i32.gt_u if
      global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_LEN i32.add i64.const 0 i64.store
      global.get $ERR_BAD_LENGTH return
    end
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_PTR i32.add local.get $ptr i64.extend_i32_u i64.store
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_LEN i32.add local.get $len i64.extend_i32_u i64.store
    i32.const 0
  )

  (func (export "packet_login_set_outer_archive_crc_block") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_LEN i32.add i64.const 0 i64.store
      i32.const 0 return
    end
    local.get $len i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_LEN i32.add i64.const 0 i64.store
      i32.const 0 return
    end
    local.get $len global.get $PACKET_LOGIN_ARCHIVE_CRC_MAX_BYTES i32.gt_u if
      global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_LEN i32.add i64.const 0 i64.store
      global.get $ERR_BAD_LENGTH return
    end
    local.get $len i32.const 3 i32.and if
      global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_PTR i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_LEN i32.add i64.const 0 i64.store
      global.get $ERR_BAD_LENGTH return
    end
    global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_PTR i32.add local.get $ptr i64.extend_i32_u i64.store
    global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_LEN i32.add local.get $len i64.extend_i32_u i64.store
    i32.const 0
  )

  ;; ── Scene events ────────────────────────────────────────────────

  (func (export "packet_scene_event_reset") (result i32)
    global.get $MEM_SCENE_EVENT
    global.get $SCENE_EVENT_SIZE
    i32.const 0
    call $memset
    i32.const 0
  )

  (func (export "packet_publish_local_player_scene_update")
    (param $plane i32) (param $tile_x i32) (param $tile_y i32)
    (param $footprint i32) (param $region_x i32) (param $region_y i32)
    (result i32)
    local.get $plane i32.const 4 i32.ge_u if global.get $ERR_BOUNDS return end
    local.get $tile_x i32.const 104 i32.ge_u if global.get $ERR_BOUNDS return end
    local.get $tile_y i32.const 104 i32.ge_u if global.get $ERR_BOUNDS return end
    global.get $MEM_SCENE_EVENT global.get $SCENE_EVENT_TYPE i32.add
    global.get $SCENE_EVENT_LOCAL_PLAYER i32.store
    global.get $MEM_SCENE_EVENT global.get $SCENE_EVENT_PLANE i32.add local.get $plane i32.store
    global.get $MEM_SCENE_EVENT global.get $SCENE_EVENT_TILE_X i32.add local.get $tile_x i32.store
    global.get $MEM_SCENE_EVENT global.get $SCENE_EVENT_TILE_Y i32.add local.get $tile_y i32.store
    local.get $footprint i32.const 1 i32.lt_s if i32.const 1 local.set $footprint end
    global.get $MEM_SCENE_EVENT global.get $SCENE_EVENT_FOOTPRINT i32.add local.get $footprint i32.store
    global.get $MEM_SCENE_EVENT global.get $SCENE_EVENT_REGION_BASE_X i32.add local.get $region_x i32.store
    global.get $MEM_SCENE_EVENT global.get $SCENE_EVENT_REGION_BASE_Y i32.add local.get $region_y i32.store
    global.get $MEM_SCENE_EVENT global.get $SCENE_EVENT_SEQ i32.add
    global.get $MEM_LOGIN_STATE global.get $L_SCENE_EVENT_SEQ_NEXT i32.add i32.load
    i32.store
    global.get $MEM_LOGIN_STATE global.get $L_SCENE_EVENT_SEQ_NEXT i32.add
    global.get $MEM_LOGIN_STATE global.get $L_SCENE_EVENT_SEQ_NEXT i32.add i32.load
    i32.const 1 i32.add
    i32.store
    i32.const 0
  )

  (func (export "packet_take_scene_update") (param $out i32) (param $cap i32) (result i32)
    (local $event_type i32)
    local.get $out i32.eqz if global.get $ERR_BOUNDS return end
    local.get $cap global.get $SCENE_EVENT_SIZE i32.lt_u if global.get $ERR_BOUNDS return end
    global.get $MEM_SCENE_EVENT global.get $SCENE_EVENT_TYPE i32.add i32.load local.set $event_type
    local.get $event_type i32.eqz if i32.const 0 return end
    global.get $MEM_SCENE_EVENT
    local.get $out
    global.get $SCENE_EVENT_SIZE
    call $memcpy
    global.get $MEM_SCENE_EVENT global.get $SCENE_EVENT_TYPE i32.add global.get $SCENE_EVENT_NONE i32.store
    local.get $event_type
  )

  ;; ── Misc ────────────────────────────────────────────────────────

  (func (export "packet_core_tick") (result i32)
    global.get $MEM_SESSION_STATE global.get $SESS_TICK_COUNT i32.add
    global.get $MEM_SESSION_STATE global.get $SESS_TICK_COUNT i32.add i64.load
    i64.const 1 i64.add
    i64.store
    i32.const 0
  )

  (func (export "packet_core_shutdown") (result i32)
    global.get $MEM_SESSION_STATE global.get $SESS_GAME_FD i32.add i64.const -1 i64.store
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
    i64.const -2 i64.and
    i64.store
    i32.const 0
  )

  (func (export "packet_game_close") (result i32)
    global.get $MEM_SESSION_STATE global.get $SESS_GAME_FD i32.add i64.const -1 i64.store
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
    i64.const -2 i64.and
    i64.store
    i32.const 0
  )

  (func (export "packet_game_open") (param $fd i32) (result i32)
    local.get $fd call $packet_session_set_game_fd
  )

  ;; ── Login build block begin ─────────────────────────────────────

  (func (export "packet_login_build_block_begin") (param $out i32) (param $cap i32) (param $client_seed i32) (result i32 i32)
    (local $i i32) (local $word i32) (local $store i32)
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
    i64.const 2 i64.and
    i64.eqz
    if
      global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_NO_FD i32.store
      global.get $ERR_NO_FD i32.const 0 return
    end
    local.get $out i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $ERR_BOUNDS i32.const 0 return
    end
    local.get $client_seed i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BAD_LENGTH i32.store
      global.get $ERR_BAD_LENGTH i32.const 0 return
    end
    local.get $cap global.get $PACKET_LOGIN_RSA_PLAIN_BYTES i32.lt_u if
      global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $ERR_BOUNDS i32.const 0 return
    end
    global.get $MEM_LOGIN_STATE global.get $L_BLOCK_PTR i32.add local.get $out i64.extend_i32_u i64.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCK_CAP i32.add local.get $cap i64.extend_i32_u i64.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCK_LEN i32.add global.get $PACKET_LOGIN_RSA_PLAIN_BYTES i64.extend_i32_u i64.store
    local.get $out i32.const 1 i32.store8
    i32.const 0 local.set $i
    block $seed_loop_done
    loop $seed_loop
      local.get $i i32.const 4 i32.ge_u br_if $seed_loop_done
      local.get $client_seed local.get $i i32.const 2 i32.shl i32.add i32.load local.set $word
      global.get $MEM_LOGIN_STATE global.get $L_CLIENT_SEED i32.add
      local.get $i i32.const 2 i32.shl i32.add
      local.get $word i32.store
      global.get $MEM_XTEA_SEED
      local.get $i i32.const 2 i32.shl i32.add
      local.get $word i32.store
      local.get $out i32.const 1 i32.add
      local.get $i i32.const 2 i32.shl i32.add
      local.get $word call $store_be32
      local.get $word i32.const 50 i32.add
      global.get $MEM_LOGIN_STATE global.get $L_INBOUND_SEED i32.add
      local.get $i i32.const 2 i32.shl i32.add
      i32.store
      local.get $i i32.const 1 i32.add local.set $i
      br $seed_loop
    end
    end
    local.get $out i32.const 17 i32.add
    global.get $MEM_LOGIN_STATE global.get $L_SRV_SEED i32.add i64.load
    i64.store
    global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add global.get $LOGIN_STATE_BLOCK_READY i32.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_NO_FD i32.store
    i32.const 0
    global.get $PACKET_LOGIN_RSA_PLAIN_BYTES
  )

  ;; ── Login get client seed ───────────────────────────────────────

  (func (export "packet_login_get_client_seed") (result i32 i32 i32)
    global.get $MEM_LOGIN_STATE global.get $L_CLIENT_SEED i32.add
    global.get $MEM_LOGIN_STATE global.get $L_INBOUND_SEED i32.add
    global.get $MEM_XTEA_SEED
  )

  ;; ── Login get RSA status ────────────────────────────────────────

  (func (export "packet_login_get_rsa_status") (result i32 i32 i32)
    global.get $MEM_LOGIN_STATE global.get $L_RSA_STATUS i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_RSA_OUT_PTR i32.add i64.load i32.wrap_i64
    global.get $MEM_LOGIN_STATE global.get $L_RSA_OUT_LEN i32.add i64.load i32.wrap_i64
  )

  ;; ── Login try RSA (stub — raw RSA modpow not yet ported) ────────

  (func (export "packet_login_try_rsa") (param $plain i32) (param $plain_len i32) (param $out i32) (param $out_cap i32) (result i32)
    (local $st i32)
    local.get $plain i32.eqz if global.get $ERR_BOUNDS return end
    local.get $out i32.eqz if global.get $ERR_BOUNDS return end
    global.get $MEM_LOGIN_STATE global.get $L_RSA_OUT_PTR i32.add local.get $out i64.extend_i32_u i64.store
    global.get $MEM_LOGIN_STATE global.get $L_RSA_OUT_LEN i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_RSA_STATUS i32.add global.get $ERR_BOUNDS i32.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BOUNDS i32.store
    global.get $ERR_BOUNDS
  )

  ;; ── Login publish cipher seeds ──────────────────────────────────

  (func $packet_login_publish_cipher_seeds (export "packet_login_publish_cipher_seeds") (param $out_seed i32) (param $in_seed i32) (param $xtea_seed i32) (result i32)
    (local $i i32)
    local.get $out_seed i32.eqz if global.get $ERR_NO_FD return end
    local.get $in_seed i32.eqz if global.get $ERR_NO_FD return end
    local.get $xtea_seed i32.eqz if global.get $ERR_NO_FD return end
    i32.const 0 local.set $i
    block $copy
    loop $copy_loop
      local.get $i i32.const 4 i32.ge_u br_if $copy
      global.get $MEM_XTEA_SEED local.get $i i32.const 2 i32.shl i32.add
      local.get $xtea_seed local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.store
      local.get $i i32.const 1 i32.add local.set $i
      br $copy_loop
    end
    end
    global.get $MEM_ISAAC_OUT
    local.get $out_seed
    i32.const 4
    call $packet_isaac_seed drop
    global.get $MEM_ISAAC_IN
    local.get $in_seed
    i32.const 4
    call $packet_isaac_seed drop
    global.get $MEM_ISAAC_IN
    global.get $MEM_ISAAC_OUT
    call $packet_session_attach_ciphers drop
    i32.const 0
  )

  ;; ── Login response feed ─────────────────────────────────────────

  (func (export "packet_login_response_feed") (param $data i32) (param $avail i32) (result i32)
    (local $state i32) (local $code i32) (local $r10 i32) (local $r11 i32)
    (local $plen i32) (local $i i32)
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATE i32.add i32.load local.set $state
    local.get $data i32.eqz if global.get $ERR_PARTIAL return end
    local.get $avail i32.eqz if global.get $ERR_PARTIAL return end
    local.get $data local.set $r10
    local.get $avail local.set $r11
    block $FSM
    loop $FSM_loop
      local.get $state
      global.get $LOGIN_RESPONSE_WAIT_STATUS
      i32.eq
      if
        local.get $r10 i32.load8_u local.set $code
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_CODE i32.add local.get $code i32.store
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_BYTES i32.add
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_BYTES i32.add i64.load
        i64.const 1 i64.add
        i64.store
        local.get $r10 i32.const 1 i32.add local.set $r10
        local.get $r11 i32.const 1 i32.sub local.set $r11
        local.get $code global.get $LOGIN_RESPONSE_ACCEPTED_CODE i32.eq
        if
          global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATE i32.add
          global.get $LOGIN_RESPONSE_ACCEPTED_WAIT_LENGTH i32.store
          global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add
          global.get $LOGIN_STATE_RESPONSE_WAIT i32.store
          local.get $r11 i32.eqz if global.get $ERR_PARTIAL return end
          global.get $LOGIN_RESPONSE_ACCEPTED_WAIT_LENGTH local.set $state
          br $FSM_loop
        else
          global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATE i32.add
          global.get $LOGIN_RESPONSE_STATUS_ONLY i32.store
          global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATUS i32.add global.get $ERR_PARTIAL i32.store
          global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add
          global.get $LOGIN_STATE_STATUS_BLOCKED i32.store
          global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_PARTIAL i32.store
          global.get $ERR_PARTIAL return
        end
      end
      local.get $state
      global.get $LOGIN_RESPONSE_ACCEPTED_WAIT_LENGTH
      i32.eq
      if
        local.get $r10 i32.load8_u local.set $plen
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_PAYLOAD_LEN i32.add local.get $plen i32.store
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_BYTES i32.add
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_BYTES i32.add i64.load
        i64.const 1 i64.add
        i64.store
        local.get $r10 i32.const 1 i32.add local.set $r10
        local.get $r11 i32.const 1 i32.sub local.set $r11
        local.get $plen global.get $PACKET_LOGIN_RESPONSE_ACCEPTED_PAYLOAD_BYTES i32.ne
        if
          global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATE i32.add
          global.get $LOGIN_RESPONSE_ACCEPTED_WAIT_PAYLOAD i32.store
          global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATUS i32.add global.get $ERR_BAD_LENGTH i32.store
          global.get $ERR_BAD_LENGTH return
        end
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATE i32.add
        global.get $LOGIN_RESPONSE_ACCEPTED_WAIT_PAYLOAD i32.store
        local.get $plen i32.eqz if
          global.get $LOGIN_RESPONSE_ACCEPTED_PAYLOAD_READY local.set $state
          br $FSM_loop
        end
        global.get $LOGIN_RESPONSE_ACCEPTED_WAIT_PAYLOAD local.set $state
        br $FSM_loop
      end
      local.get $state
      global.get $LOGIN_RESPONSE_ACCEPTED_WAIT_PAYLOAD
      i32.eq
      if
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_PAYLOAD_LEN i32.add i32.load local.set $plen
        local.get $r11 local.get $plen i32.lt_u
        if global.get $ERR_PARTIAL return end
        global.get $MEM_LOGIN_RESPONSE_PAYLOAD
        local.get $r10
        local.get $plen
        call $memcpy
        local.get $r10 local.get $plen i32.add local.set $r10
        local.get $r11 local.get $plen i32.sub local.set $r11
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_BYTES i32.add
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_BYTES i32.add i64.load
        local.get $plen i64.extend_i32_u
        i64.add
        i64.store
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATE i32.add
        global.get $LOGIN_RESPONSE_ACCEPTED_PAYLOAD_READY i32.store
        global.get $LOGIN_RESPONSE_ACCEPTED_PAYLOAD_READY local.set $state
        br $FSM_loop
      end
      local.get $state
      global.get $LOGIN_RESPONSE_ACCEPTED_PAYLOAD_READY
      i32.eq
      if
        call $publish_success
        local.tee $code
        i32.const 0 i32.lt_s
        if local.get $code return end
        call $decode_accepted_payload
        local.tee $code
        i32.const 0 i32.lt_s
        if
          global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATUS i32.add local.get $code i32.store
          global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add
          global.get $LOGIN_STATE_RESPONSE_BLOCKED i32.store
          global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add local.get $code i32.store
          local.get $code return
        end
        global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATUS i32.add global.get $OK i32.store
        global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add
        global.get $LOGIN_STATE_RESPONSE_WAIT i32.store
        global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $OK i32.store
        i32.const 0 return
      end
      br $FSM
    end
    end
    global.get $ERR_PARTIAL return
  )

  (func $publish_success (result i32)
    (local $rc i32)
    global.get $MEM_LOGIN_STATE global.get $L_BLOCK_LEN i32.add i64.load
    global.get $PACKET_LOGIN_RSA_PLAIN_BYTES i64.extend_i32_u
    i64.lt_u
    if global.get $ERR_PARTIAL return end
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
    global.get $SESS_FLAG_SEED_READY i64.extend_i32_u
    i64.and
    i64.eqz
    if global.get $ERR_PARTIAL return end
    global.get $MEM_LOGIN_STATE global.get $L_CLIENT_SEED i32.add
    global.get $MEM_LOGIN_STATE global.get $L_INBOUND_SEED i32.add
    global.get $MEM_LOGIN_STATE global.get $L_XTEA_SEED2 i32.add
    call $packet_login_publish_cipher_seeds
    local.tee $rc
    i32.const 0 i32.lt_s
    if local.get $rc return end
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATUS i32.add global.get $OK i32.store
    global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add
    global.get $LOGIN_STATE_RESPONSE_WAIT i32.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $OK i32.store
    i32.const 0
  )

  (func $decode_accepted_payload (result i32)
    (local $i i32) (local $isaac_byte i32) (local $accum i32) (local $flag i32) (local $base i32)
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_PAYLOAD_LEN i32.add i32.load
    global.get $PACKET_LOGIN_RESPONSE_ACCEPTED_PAYLOAD_BYTES
    i32.ne
    if global.get $ERR_BAD_LENGTH return end
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
    global.get $SESS_FLAG_ISAAC_READY i64.extend_i32_u
    i64.and
    i64.eqz
    if global.get $ERR_PARTIAL return end
    global.get $MEM_LOGIN_RESPONSE_PAYLOAD local.set $base
    global.get $MEM_ISAAC_IN call $isaac_next_local
    local.set $isaac_byte
    local.get $base i32.load8_u
    local.get $isaac_byte i32.sub
    i32.const 0xff i32.and
    local.tee $flag
    i32.const 1 i32.eq
    i32.const 1
    i32.and
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_RECONNECT_FLAG i32.add i32.store
    i32.const 1 local.set $i
    local.get $flag
    if
      i32.const 0 local.set $accum
      i32.const 0 local.set $i
      block $seed_done
      loop $seed_loop
        local.get $i i32.const 4 i32.ge_u br_if $seed_done
        local.get $base local.get $i i32.const 1 i32.add i32.add i32.load8_u
        global.get $MEM_ISAAC_IN call $isaac_next_local
        i32.sub
        i32.const 0xff i32.and
        local.get $accum i32.const 8 i32.shl i32.or
        local.set $accum
        local.get $i i32.const 1 i32.add local.set $i
        br $seed_loop
      end
      end
      global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_RECONNECT_SEED i32.add local.get $accum i32.store
    end
    local.get $base local.get $i i32.add i32.load8_u
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_ACCOUNT_FLAGS i32.add i32.store
    local.get $i i32.const 1 i32.add local.set $i
    local.get $base local.get $i i32.add i32.load8_u
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_MEMBER_FLAG i32.add i32.store
    local.get $i i32.const 1 i32.add local.set $i
    local.get $base local.get $i i32.add i32.load8_u
    i32.const 8 i32.shl
    local.get $base local.get $i i32.add i32.const 1 i32.add i32.load8_u
    i32.or
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_WORLD_ID i32.add i32.store
    local.get $i i32.const 2 i32.add local.set $i
    local.get $base local.get $i i32.add i32.load8_u
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_RIGHTS i32.add i32.store
    local.get $i i32.const 1 i32.add local.set $i
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_PRIMARY_ID i32.add
    local.get $base local.get $i i32.add
    call $load_be64
    i64.store
    local.get $i i32.const 8 i32.add local.set $i
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_SESSION_ID i32.add
    local.get $base local.get $i i32.add
    call $load_be64
    i64.store
    local.get $i i32.const 8 i32.add local.set $i
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_ACCOUNT_HASH i32.add
    local.get $base local.get $i i32.add
    call $load_be64
    i64.store
    global.get $MEM_LOGIN_STATE global.get $L_ACCEPTED_DECODED i32.add i32.const 1 i32.store
    i32.const 0
  )

  ;; ── Retained bitreader ──────────────────────────────────────────

  (global $BITREADER_PTR      i32 (i32.const 0))
  (global $BITREADER_BYTE_LEN i32 (i32.const 8))
  (global $BITREADER_BIT_POS  i32 (i32.const 16))
  (global $BITREADER_BIT_LEN  i32 (i32.const 24))
  (global $BITREADER_ERROR    i32 (i32.const 32))

  (func (export "packet_retained_bitreader_begin") (result i32)
    (local $len i32)
    global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_READY i32.add i32.load
    i32.const 1 i32.ne
    if
      global.get $MEM_RETAINED_BITREADER global.get $BITREADER_ERROR i32.add global.get $ERR_PARTIAL i32.store
      global.get $ERR_PARTIAL return
    end
    global.get $MEM_RX_BUF_STATE i32.load offset=0 local.set $len
    local.get $len i32.eqz if
      global.get $MEM_RETAINED_BITREADER global.get $BITREADER_ERROR i32.add global.get $ERR_PARTIAL i32.store
      global.get $ERR_PARTIAL return
    end
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_PTR i32.add local.get $len i32.store
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_BYTE_LEN i32.add
    global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_LEN i32.add i32.load
    i32.store
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_BIT_POS i32.add i64.const 0 i64.store
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_BIT_LEN i32.add
    global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_LEN i32.add i32.load
    i32.const 3 i32.shl
    i32.store
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_ERROR i32.add global.get $OK i32.store
    i32.const 0
  )

  (func (export "packet_retained_read_bits") (param $n i32) (result i32)
    (local $pos i32) (local $ptr i32) (local $byte_len i32) (local $bit_len i32)
    (local $result i32) (local $byte_off i32) (local $bit_off i32) (local $tmp i32)
    local.get $n i32.const 31 i32.gt_u if global.get $ERR_BOUNDS return end
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_BIT_POS i32.add i32.load local.set $pos
    local.get $pos local.get $n i32.add local.set $tmp
    local.get $tmp local.get $pos i32.lt_u if global.get $ERR_BOUNDS return end
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_BIT_LEN i32.add i32.load local.set $bit_len
    local.get $tmp local.get $bit_len i32.gt_u if global.get $ERR_BOUNDS return end
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_PTR i32.add i32.load local.set $ptr
    local.get $ptr i32.eqz if global.get $ERR_PARTIAL return end
    block $done
    loop $loop
      local.get $n i32.eqz br_if $done
      local.get $pos i32.const 3 i32.shr_u local.set $byte_off
      local.get $pos i32.const 7 i32.and local.set $bit_off
      i32.const 7 local.get $bit_off i32.sub local.set $tmp
      local.get $ptr local.get $byte_off i32.add i32.load8_u
      local.get $tmp i32.shr_u
      i32.const 1 i32.and
      local.get $result i32.const 1 i32.shl
      i32.or
      local.set $result
      local.get $pos i32.const 1 i32.add local.set $pos
      local.get $n i32.const 1 i32.sub local.set $n
      br $loop
    end
    end
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_BIT_POS i32.add local.get $pos i32.store
    local.get $result
  )

  (func (export "packet_retained_bitreader_finish") (result i32)
    (local $byte_cursor i32)
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_BIT_POS i32.add i32.load
    i32.const 7 i32.add
    i32.const 3 i32.shr_u
    local.set $byte_cursor
    local.get $byte_cursor
    global.get $MEM_RETAINED_BITREADER global.get $BITREADER_BYTE_LEN i32.add i32.load
    i32.gt_u
    if
      global.get $MEM_RETAINED_BITREADER global.get $BITREADER_ERROR i32.add global.get $ERR_BOUNDS i32.store
      global.get $ERR_BOUNDS return
    end
    global.get $MEM_RX_BUF_STATE i32.const 16 i32.add
    local.get $byte_cursor i32.store
    i32.const 0
  )

  ;; ── Outbound queue ──────────────────────────────────────────────

  (func $packet_node_begin (export "packet_node_begin") (param $node i32) (param $opcode i32) (param $length i32) (param $isaac_state i32) (result i32)
    (local $buf_ptr i32) (local $buf_cap i32)
    local.get $node local.get $opcode i32.store offset=0
    local.get $node local.get $length i32.store offset=4
    local.get $node i64.const 0 i64.store offset=8
    local.get $length global.get $PACKET_LEN_VAR2 i32.eq
    if
      global.get $MEM_TX_LARGE_BUF local.set $buf_ptr
      global.get $BUF_LARGE_BYTES local.set $buf_cap
    else
      local.get $length global.get $PACKET_LEN_VAR1 i32.eq
      if
        global.get $MEM_TX_MEDIUM_BUF local.set $buf_ptr
        global.get $BUF_MED_BYTES local.set $buf_cap
      else
        local.get $length i32.const 0 i32.lt_s
        if global.get $ERR_BAD_LENGTH return end
        local.get $length i32.const 18 i32.le_s
        if
          global.get $MEM_TX_TINY_BUF local.set $buf_ptr
          global.get $BUF_TINY_BYTES local.set $buf_cap
        else
          local.get $length i32.const 98 i32.le_s
          if
            global.get $MEM_TX_SMALL_BUF local.set $buf_ptr
            global.get $BUF_SMALL_BYTES local.set $buf_cap
          else
            global.get $MEM_TX_MEDIUM_BUF local.set $buf_ptr
            global.get $BUF_MED_BYTES local.set $buf_cap
          end
        end
      end
    end
    local.get $node local.get $buf_ptr i32.store offset=16
    local.get $node local.get $buf_cap i32.store offset=24
    global.get $MEM_TX_BUF_STATE
    local.get $buf_ptr
    local.get $buf_cap
    call $packet_buffer_reset drop
    global.get $MEM_TX_BUF_STATE i32.const 32 i32.add local.get $isaac_state i32.store
    global.get $MEM_TX_BUF_STATE
    local.get $opcode
    call $packet_buffer_write_opcode drop
    i32.const 0
  )

  (func (export "packet_outbound_queue_begin") (param $opcode i32) (param $length i32) (result i32 i32)
    (local $tail i32) (local $next i32) (local $node_ptr i32)
    global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_LEN i32.add i32.load local.set $tail
    local.get $tail i32.const 1 i32.add
    global.get $NODE_RING_COUNT i32.const 1 i32.sub
    i32.and
    local.set $next
    local.get $next
    global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_READY i32.add i32.load
    i32.eq
    if
      global.get $ERR_QUEUE_FULL i32.const 0 return
    end
    global.get $MEM_TX_NODE_RING
    local.get $tail global.get $NODE_SIZE i32.mul i32.add
    local.set $node_ptr
    local.get $node_ptr
    local.get $opcode
    local.get $length
    global.get $MEM_ISAAC_OUT
    call $packet_node_begin drop
    local.get $node_ptr
    global.get $MEM_SESSION_STATE global.get $SESS_ACTIVE_NODE i32.add i32.store
    global.get $MEM_TX_BUF_STATE
    local.get $node_ptr
  )

  (func (export "packet_outbound_queue_commit") (result i32)
    (local $node_ptr i32) (local $pos i32) (local $tail i32)
    global.get $MEM_SESSION_STATE global.get $SESS_ACTIVE_NODE i32.add i32.load local.set $node_ptr
    local.get $node_ptr i32.eqz if global.get $ERR_BAD_LENGTH return end
    global.get $MEM_TX_BUF_STATE i32.load offset=16 local.set $pos
    local.get $node_ptr i32.const 8 i32.add local.get $pos i64.extend_i32_u i64.store
    global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_LEN i32.add i32.load local.set $tail
    local.get $tail i32.const 1 i32.add
    global.get $NODE_RING_COUNT i32.const 1 i32.sub
    i32.and
    global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_LEN i32.add i32.store
    global.get $MEM_SESSION_STATE global.get $SESS_ACTIVE_NODE i32.add i64.const 0 i64.store
    i32.const 0
  )

  ;; ── Packet send queued (stub — needs WASI fd_write) ─────────────

  (func (export "packet_send_queued") (result i32)
    (local $head i32) (local $tail i32) (local $node_ptr i32) (local $plen i32) (local $buf_ptr i32)
    (local $written i32)
    global.get $MEM_SESSION_STATE global.get $SESS_GAME_FD i32.add i64.load i32.wrap_i64
    i32.const 0 i32.lt_s
    if global.get $ERR_NO_FD return end
    i32.const 0 local.set $written
    block $drain
    loop $drain_loop
      global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_READY i32.add i32.load local.set $head
      global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_LEN i32.add i32.load local.set $tail
      local.get $head local.get $tail i32.eq br_if $drain
      global.get $MEM_TX_NODE_RING
      local.get $head global.get $NODE_SIZE i32.mul i32.add
      local.set $node_ptr
      local.get $node_ptr i32.load offset=8 local.set $plen
      local.get $plen i32.eqz if
        local.get $head i32.const 1 i32.add
        global.get $NODE_RING_COUNT i32.const 1 i32.sub
        i32.and
        global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_READY i32.add i32.store
        local.get $written i32.const 1 i32.add local.set $written
        br $drain_loop
      end
      local.get $written local.get $plen i32.add local.set $written
      local.get $head i32.const 1 i32.add
      global.get $NODE_RING_COUNT i32.const 1 i32.sub
      i32.and
      global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_READY i32.add i32.store
      br $drain_loop
    end
    end
    local.get $written
  )

  ;; ── Packet read dispatch (stub — needs WASI fd_read) ────────────

  (func (export "packet_read_dispatch") (result i32)
    global.get $MEM_LOGIN_STATE global.get $L_RESPONSE_STATUS i32.add i32.load
    if global.get $ERR_PARTIAL return end
    global.get $MEM_SESSION_STATE global.get $SESS_FLAGS i32.add i64.load
    i64.const 4 i64.and
    i64.eqz
    if global.get $ERR_BOUNDS return end
    global.get $MEM_SESSION_STATE global.get $SESS_GAME_FD i32.add i64.load i32.wrap_i64
    i32.const 0 i32.lt_s
    if global.get $ERR_NO_FD return end
    global.get $ERR_PARTIAL
  )

  ;; ── Login build outer packet (stub — complex outer assembly) ────

  (func (export "packet_login_build_outer_packet") (param $out i32) (param $cap i32) (param $rsa_block i32) (param $rsa_len i32) (param $login_opcode i32) (param $revision i32) (result i32 i32 i32)
    (local $written i32) (local $conf i32) (local $ptr i32) (local $len i32) (local $xtea_len i32)
    local.get $out i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_START i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_END i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add i32.const 0 i32.store
      global.get $ERR_BOUNDS i32.const 0 i32.const 0 return
    end
    local.get $rsa_block i32.eqz if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_START i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_END i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add i32.const 0 i32.store
      global.get $ERR_BOUNDS i32.const 0 i32.const 0 return
    end
    local.get $rsa_len i32.const 1 i32.lt_u if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_START i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_END i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add i32.const 0 i32.store
      global.get $ERR_BOUNDS i32.const 0 i32.const 0 return
    end
    local.get $rsa_len i32.const 65535 i32.gt_u if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_START i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_END i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add i32.const 0 i32.store
      global.get $ERR_BOUNDS i32.const 0 i32.const 0 return
    end
    global.get $MEM_LOGIN_STATE global.get $L_RSA_STATUS i32.add i32.load
    local.tee $ptr
    if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add local.get $ptr i32.store
      global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add local.get $ptr i32.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_START i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_END i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add i32.const 0 i32.store
      local.get $ptr i32.const 0 i32.const 0 return
    end
    global.get $PACKET_LOGIN_OUTER_MIN_CAP
    local.get $rsa_len i32.add
    local.get $cap
    i32.gt_u
    if
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BOUNDS i32.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_START i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_END i32.add i64.const 0 i64.store
      global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add i32.const 0 i32.store
      global.get $ERR_BOUNDS i32.const 0 i32.const 0 return
    end
    local.get $out i32.const 0 i32.add local.get $login_opcode i32.store8
    local.get $out i32.const 1 i32.add i64.const 0 i64.store16
    local.get $out i32.const 3 i32.add i32.const 0 i32.store8
    local.get $out i32.const 4 i32.add i32.const 0 i32.store8
    local.get $out i32.const 5 i32.add i32.const 0 i32.store8
    local.get $out i32.const 6 i32.add global.get $PACKET_LOGIN_OUTER_BUILD i32.store8
    local.get $out i32.const 7 i32.add i32.const 0 i32.store8
    local.get $out i32.const 8 i32.add i32.const 0 i32.store8
    local.get $out i32.const 9 i32.add i32.const 0 i32.store8
    local.get $out i32.const 10 i32.add global.get $PACKET_LOGIN_OUTER_SUBBUILD i32.store8
    local.get $out i32.const 11 i32.add local.get $revision call $store_be32
    local.get $out i32.const 15 i32.add global.get $PACKET_LOGIN_OUTER_PLATFORM_BYTE0 i32.store8
    local.get $out i32.const 16 i32.add global.get $PACKET_LOGIN_OUTER_PLATFORM_BYTE1 i32.store8
    local.get $out i32.const 17 i32.add global.get $PACKET_LOGIN_OUTER_PLATFORM_BYTE2 i32.store8
    local.get $out i32.const 18 i32.add local.get $rsa_len i32.const 8 i32.shr_u i32.store8
    local.get $out i32.const 19 i32.add local.get $rsa_len i32.const 0xff i32.and i32.store8
    local.get $out i32.const 20 i32.add local.get $rsa_block local.get $rsa_len call $memcpy
    local.get $rsa_len i32.const 20 i32.add local.set $written
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_START i32.add local.get $written i64.extend_i32_u i64.store
    global.get $OUTER_CONF_OPCODE
    global.get $OUTER_CONF_LEN2 i32.or
    global.get $OUTER_CONF_BUILD_FIELDS i32.or
    global.get $OUTER_CONF_RSA_LEN_DATA i32.or
    global.get $OUTER_CONF_XTEA_START i32.or
    local.set $conf
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_PTR i32.add i32.load
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_LEN i32.add i32.load
    local.tee $len
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    local.get $written i32.const 1 i32.add local.get $len i32.add
    local.get $cap
    i32.gt_u
    if call $tail_bounds_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_AUTH_PTR i32.add i32.load
    local.set $ptr
    local.get $out local.get $written i32.add local.get $ptr local.get $len call $memcpy
    local.get $out local.get $written i32.add local.get $len i32.add i32.const 0 i32.store8
    local.get $written i32.const 1 i32.add local.get $len i32.add local.set $written
    local.get $conf global.get $OUTER_CONF_AUTH_STRING i32.or local.set $conf
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_FLAGS_DIMENSIONS_VALID i32.add i32.load
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    local.get $written i32.const 5 i32.add local.get $cap i32.gt_u if call $tail_bounds_outer return end
    local.get $out local.get $written i32.add
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_FLAGS i32.add i32.load
    i32.store8
    local.get $out local.get $written i32.add i32.const 1 i32.add
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_WIDTH i32.add i32.load
    local.tee $ptr
    i32.const 8 i32.shr_u
    i32.store8
    local.get $out local.get $written i32.add i32.const 2 i32.add
    local.get $ptr i32.const 0xff i32.and i32.store8
    local.get $out local.get $written i32.add i32.const 3 i32.add
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_HEIGHT i32.add i32.load
    local.tee $ptr
    i32.const 8 i32.shr_u
    i32.store8
    local.get $out local.get $written i32.add i32.const 4 i32.add
    local.get $ptr i32.const 0xff i32.and i32.store8
    local.get $written i32.const 5 i32.add local.set $written
    local.get $conf global.get $OUTER_CONF_FLAGS_DIMENSIONS i32.or local.set $conf
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_RANDOM_DAT_PTR i32.add i32.load
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    local.get $written global.get $PACKET_LOGIN_RANDOM_DAT_BYTES i32.add
    local.get $cap
    i32.gt_u
    if call $tail_bounds_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_RANDOM_DAT_PTR i32.add i32.load
    local.set $ptr
    local.get $out local.get $written i32.add
    local.get $ptr
    global.get $PACKET_LOGIN_RANDOM_DAT_BYTES
    call $memcpy
    local.get $written global.get $PACKET_LOGIN_RANDOM_DAT_BYTES i32.add local.set $written
    local.get $conf global.get $OUTER_CONF_RANDOM_DAT i32.or local.set $conf
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_PTR i32.add i32.load
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_LEN i32.add i32.load
    local.tee $len
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    local.get $written i32.const 1 i32.add local.get $len i32.add
    local.get $cap
    i32.gt_u
    if call $tail_bounds_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_SECOND_AUTH_PTR i32.add i32.load
    local.set $ptr
    local.get $out local.get $written i32.add local.get $ptr local.get $len call $memcpy
    local.get $out local.get $written i32.add local.get $len i32.add i32.const 0 i32.store8
    local.get $written i32.const 1 i32.add local.get $len i32.add local.set $written
    local.get $conf global.get $OUTER_CONF_SECOND_AUTH_STRING i32.or local.set $conf
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_MARKER_VALID i32.add i32.load
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_PTR i32.add i32.load
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_LEN i32.add i32.load
    local.tee $len
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    local.get $written i32.const 7 i32.add local.get $len i32.add
    local.get $cap
    i32.gt_u
    if call $tail_bounds_outer return end
    local.get $out local.get $written i32.add
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_MARKER i32.add i32.load
    call $store_be32
    local.get $out local.get $written i32.add i32.const 4 i32.add i32.const 0 i32.store8
    global.get $MEM_LOGIN_STATE global.get $L_PLATFORM_INFO_PTR i32.add i32.load
    local.set $ptr
    local.get $out local.get $written i32.add i32.const 5 i32.add
    local.get $ptr
    local.get $len
    call $memcpy
    local.get $out local.get $written i32.add i32.const 5 i32.add local.get $len i32.add i32.const 0 i32.store8
    local.get $out local.get $written i32.add i32.const 6 i32.add local.get $len i32.add i32.const 0 i32.store8
    local.get $written i32.const 7 i32.add local.get $len i32.add local.set $written
    local.get $conf
    global.get $OUTER_CONF_PLATFORM_MARKER i32.or
    global.get $OUTER_CONF_PLATFORM_INFO_PRE_ZERO i32.or
    global.get $OUTER_CONF_PLATFORM_INFO_BYTES i32.or
    global.get $OUTER_CONF_PLATFORM_INFO_POST_ZERO i32.or
    global.get $OUTER_CONF_PRE_CRC_ZERO i32.or
    local.set $conf
    global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_PTR i32.add i32.load
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_LEN i32.add i32.load
    local.tee $len
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    local.get $written local.get $len i32.add
    local.get $cap
    i32.gt_u
    if call $tail_bounds_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_ARCHIVE_CRC_PTR i32.add i32.load
    local.set $ptr
    local.get $out local.get $written i32.add
    local.get $ptr
    local.get $len
    call $memcpy
    local.get $written local.get $len i32.add local.set $written
    local.get $conf global.get $OUTER_CONF_ARCHIVE_CRC_BLOCK i32.or local.set $conf
    local.get $conf
    global.get $OUTER_CONF_REQUIRED_FINAL_MASK
    i32.and
    global.get $OUTER_CONF_REQUIRED_FINAL_MASK
    i32.ne
    if local.get $written local.get $conf call $publish_partial_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_START i32.add i64.load i32.wrap_i64
    local.set $ptr
    local.get $written local.get $ptr i32.sub
    local.tee $xtea_len
    i32.eqz
    if local.get $written local.get $conf call $publish_partial_outer return end
    local.get $xtea_len i32.const 7 i32.and if call $bad_xtea_outer return end
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_END i32.add local.get $written i64.extend_i32_u i64.store
    local.get $out local.get $ptr i32.add
    local.get $xtea_len
    global.get $MEM_XTEA_SEED
    call $packet_xtea_encrypt
    i32.const 0 i32.lt_s
    if call $bad_xtea_outer return end
    local.get $written i32.const 3 i32.sub local.set $ptr
    local.get $ptr i32.const 65535 i32.gt_u if call $tail_bounds_outer return end
    local.get $out i32.const 1 i32.add local.get $ptr i32.const 8 i32.shr_u i32.store8
    local.get $out i32.const 2 i32.add local.get $ptr i32.const 0xff i32.and i32.store8
    local.get $conf global.get $OUTER_CONF_XTEA_FINALIZED i32.or local.set $conf
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add global.get $OK i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add local.get $conf i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add local.get $written i64.extend_i32_u i64.store
    global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add global.get $LOGIN_STATE_OUTER_READY i32.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $OK i32.store
    i32.const 0
    local.get $written
    local.get $conf
  )

  (func $publish_partial_outer (param $written i32) (param $conf i32) (result i32 i32 i32)
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add global.get $ERR_LOGIN_BLOCK_TODO i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add local.get $conf i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add local.get $written i64.extend_i32_u i64.store
    global.get $MEM_LOGIN_STATE global.get $L_LOGIN_STATE i32.add global.get $LOGIN_STATE_OUTER_TAIL_BLOCKED i32.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_LOGIN_BLOCK_TODO i32.store
    global.get $ERR_LOGIN_BLOCK_TODO local.get $written local.get $conf
  )

  (func $bad_xtea_outer (result i32 i32 i32)
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add global.get $ERR_BAD_LENGTH i32.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BAD_LENGTH i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_END i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add i32.const 0 i32.store
    global.get $ERR_BAD_LENGTH i32.const 0 i32.const 0
  )

  (func $tail_bounds_outer (result i32 i32 i32)
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_STATUS i32.add global.get $ERR_BOUNDS i32.store
    global.get $MEM_LOGIN_STATE global.get $L_BLOCKER_STATUS i32.add global.get $ERR_BOUNDS i32.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_LEN i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_START i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_XTEA_END i32.add i64.const 0 i64.store
    global.get $MEM_LOGIN_STATE global.get $L_OUTER_CONFIDENCE i32.add i32.const 0 i32.store
    global.get $ERR_BOUNDS i32.const 0 i32.const 0
  )

  (func (export "packet_get_retained_packet_metadata") (result i32 i32 i32 i32 i32 i32)
    global.get $MEM_LOGIN_STATE global.get $L_ACTOR_CANDIDATE_KIND i32.add i32.load
    global.get $MEM_SESSION_STATE global.get $SESS_IN_OPCODE i32.add i32.load
    global.get $MEM_SESSION_STATE global.get $SESS_IN_LENGTH i32.add i32.load
    global.get $MEM_RX_BUF_STATE i32.load offset=0
    global.get $MEM_SESSION_STATE global.get $SESS_IN_LENGTH_MODE i32.add i32.load
    global.get $MEM_LOGIN_STATE global.get $L_FIRST_PAYLOAD_READY i32.add i32.load
  )
)
