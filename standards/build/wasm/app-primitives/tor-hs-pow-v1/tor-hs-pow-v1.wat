(module $tor_hs_pow_v1.wasm
  (type (;0;) (func (param i32)))
  (type (;1;) (func (param i32 i32)))
  (type (;2;) (func (param i32 i32 i32) (result i32)))
  (type (;3;) (func (param i32 i32) (result i32)))
  (type (;4;) (func (result i32)))
  (type (;5;) (func (param i32 i32 i32 i32) (result i32)))
  (type (;6;) (func (param i32 i32 i32 i32 i32) (result i32)))
  (type (;7;) (func (param i32 i32 i32 i32 i32 i32) (result i32)))
  (type (;8;) (func (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (type (;9;) (func (param i32 i32 i32)))
  (type (;10;) (func))
  (type (;11;) (func (param i32 i32 i32 i32 i32)))
  (type (;12;) (func (param i32 i32 i32 i32)))
  (type (;13;) (func (param i32 i32 i32 i32 i32 i32 i32 i32)))
  (type (;14;) (func (param i32 i64) (result i64)))
  (type (;15;) (func (param i32 i32 i64)))
  (type (;16;) (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (;17;) (func (param i32 i32 i64 i32)))
  (type (;18;) (func (param i32 i32 i64 i64)))
  (type (;19;) (func (param i32 i32 i32 i32 i32 i32)))
  (type (;20;) (func (param i32) (result i32)))
  (type (;21;) (func (param i32 i64 i64 i64 i64)))
  (func $proto_abi_version (type 4) (result i32)
    i32.const 1)
  (func $proto_standard_id (type 4) (result i32)
    i32.const 300219)
  (func $tor_hs_pow_v1_blake2b_result (type 5) (param i32 i32 i32 i32) (result i32)
    (local i32 i32)
    global.get $__stack_pointer
    i32.const 416
    i32.sub
    local.tee 4
    global.set $__stack_pointer
    i32.const -1
    local.set 5
    block  ;; label = @1
      local.get 0
      i32.eqz
      br_if 0 (;@1;)
      local.get 2
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.eqz
      br_if 0 (;@1;)
      i32.const -2
      local.set 5
      local.get 1
      i32.const 100
      i32.ne
      br_if 0 (;@1;)
      local.get 4
      i32.const 12
      i32.add
      local.get 0
      i32.const 100
      memory.copy
      local.get 4
      local.get 2
      i64.load offset=8 align=1
      i64.store offset=120 align=1
      local.get 4
      local.get 2
      i64.load align=1
      i64.store offset=112 align=1
      i32.const 0
      local.set 5
      local.get 4
      i32.const 336
      i32.add
      i32.const 8
      i32.add
      local.tee 2
      i32.const 1
      i32.const 0
      i32.const 1
      i32.const 0
      i32.const 0
      i32.const 4
      call $_ZN6blake214Blake2bVarCore15new_with_params17hbddb78c2cf65de78E
      local.get 4
      i32.const 128
      i32.add
      local.get 2
      i32.const 72
      memory.copy
      local.get 4
      i32.const 4
      i32.store offset=200
      local.get 4
      i32.const 204
      i32.add
      local.tee 2
      local.get 4
      i32.const 12
      i32.add
      i32.const 116
      memory.copy
      local.get 4
      i64.const 0
      i64.store offset=320
      local.get 4
      i64.const 0
      i64.store offset=325 align=1
      local.get 4
      i64.const 0
      i64.store offset=392
      local.get 4
      i64.const 0
      i64.store offset=384
      local.get 4
      i64.const 0
      i64.store offset=376
      local.get 4
      i64.const 0
      i64.store offset=368
      local.get 4
      i64.const 0
      i64.store offset=360
      local.get 4
      i64.const 0
      i64.store offset=352
      local.get 4
      i64.const 0
      i64.store offset=344
      local.get 4
      i64.const 0
      i64.store offset=336
      local.get 4
      local.get 4
      i64.load offset=192
      i64.const 116
      i64.add
      i64.store offset=192
      local.get 4
      i32.const 128
      i32.add
      local.get 2
      i64.const 0
      local.get 4
      i32.const 336
      i32.add
      call $_ZN6blake214Blake2bVarCore18finalize_with_flag17h8280626bb9c29f91E
      local.get 3
      local.get 4
      i32.load offset=336
      local.tee 2
      i32.const 16711935
      i32.and
      i32.const 8
      i32.rotr
      local.get 2
      i32.const 24
      i32.rotr
      i32.const 16711935
      i32.and
      i32.or
      i32.store
    end
    local.get 4
    i32.const 416
    i32.add
    global.set $__stack_pointer
    local.get 5)
  (func $tor_hs_pow_v1_challenge (type 6) (param i32 i32 i32 i32 i32) (result i32)
    (local i32)
    i32.const -1
    local.set 5
    block  ;; label = @1
      local.get 0
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      i32.eqz
      br_if 0 (;@1;)
      local.get 2
      i32.eqz
      br_if 0 (;@1;)
      local.get 4
      i32.eqz
      br_if 0 (;@1;)
      local.get 4
      i32.const 0
      i64.load offset=1049505 align=1
      i64.store offset=8 align=1
      local.get 4
      i32.const 0
      i64.load offset=1049497 align=1
      i64.store align=1
      local.get 4
      local.get 0
      i64.load offset=24 align=1
      i64.store offset=40 align=1
      local.get 4
      local.get 0
      i64.load offset=16 align=1
      i64.store offset=32 align=1
      local.get 4
      local.get 0
      i64.load offset=8 align=1
      i64.store offset=24 align=1
      local.get 4
      local.get 0
      i64.load align=1
      i64.store offset=16 align=1
      local.get 4
      local.get 1
      i64.load align=1
      i64.store offset=48 align=1
      local.get 4
      local.get 1
      i64.load offset=8 align=1
      i64.store offset=56 align=1
      local.get 4
      local.get 1
      i64.load offset=16 align=1
      i64.store offset=64 align=1
      local.get 4
      local.get 1
      i64.load offset=24 align=1
      i64.store offset=72 align=1
      local.get 4
      local.get 2
      i64.load align=1
      i64.store offset=80 align=1
      local.get 4
      local.get 2
      i64.load offset=8 align=1
      i64.store offset=88 align=1
      local.get 4
      local.get 3
      i32.const 16711935
      i32.and
      i32.const 8
      i32.rotr
      local.get 3
      i32.const 24
      i32.rotr
      i32.const 16711935
      i32.and
      i32.or
      i32.store offset=96 align=1
      i32.const 100
      local.set 5
    end
    local.get 5)
  (func $tor_hs_pow_v1_challenge_len (type 4) (result i32)
    i32.const 100)
  (func $tor_hs_pow_v1_equix_verify (type 2) (param i32 i32 i32) (result i32)
    (local i32 i32)
    global.get $__stack_pointer
    i32.const 48
    i32.sub
    local.tee 3
    global.set $__stack_pointer
    i32.const -1
    local.set 4
    block  ;; label = @1
      local.get 0
      i32.eqz
      br_if 0 (;@1;)
      local.get 2
      i32.eqz
      br_if 0 (;@1;)
      i32.const -2
      local.set 4
      local.get 1
      i32.const 100
      i32.ne
      br_if 0 (;@1;)
      local.get 3
      local.get 2
      i64.load offset=8 align=1
      i64.store offset=24
      local.get 3
      local.get 2
      i64.load align=1
      i64.store offset=16
      i32.const 0
      local.set 4
      local.get 3
      i32.const 0
      i32.store8 offset=39
      local.get 3
      i32.const 8
      i32.add
      local.get 3
      i32.const 39
      i32.add
      local.get 0
      i32.const 100
      local.get 3
      i32.const 16
      i32.add
      call $_ZN5equix12EquiXBuilder12verify_bytes17he8cd41b059ffe8d4E
      local.get 3
      local.get 3
      i32.load offset=12
      local.tee 0
      i32.store offset=44
      local.get 3
      local.get 3
      i32.load offset=8
      local.tee 2
      i32.store offset=40
      local.get 2
      i32.const 4
      i32.eq
      br_if 0 (;@1;)
      i32.const -3
      local.set 4
      local.get 2
      i32.const 1
      i32.ne
      br_if 0 (;@1;)
      local.get 0
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      local.get 0
      i32.load
      local.tee 2
      i32.const -1
      i32.add
      i32.store
      local.get 2
      i32.const 1
      i32.ne
      br_if 0 (;@1;)
      local.get 3
      i32.const 40
      i32.add
      i32.const 4
      i32.add
      call $_ZN5alloc4sync16Arc$LT$T$C$A$GT$9drop_slow17h543bbdef373111b6E
    end
    local.get 3
    i32.const 48
    i32.add
    global.set $__stack_pointer
    local.get 4)
  (func $tor_hs_pow_v1_replay_capacity (type 4) (result i32)
    i32.const 256)
  (func $tor_hs_pow_v1_replay_count (type 4) (result i32)
    i32.const 0
    i32.load offset=1051024)
  (func $tor_hs_pow_v1_replay_reset (type 4) (result i32)
    i32.const 1050768
    i32.const 0
    i32.const 256
    memory.fill
    i32.const 0
    i32.const 0
    i32.store offset=1051024
    i32.const 0
    i32.const 0
    i32.store offset=1052052
    i32.const 0)
  (func $tor_hs_pow_v1_solution_len (type 4) (result i32)
    i32.const 16)
  (func $tor_hs_pow_v1_solve_first (type 7) (param i32 i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32)
    global.get $__stack_pointer
    i32.const 704
    i32.sub
    local.tee 6
    global.set $__stack_pointer
    i32.const -1
    local.set 7
    block  ;; label = @1
      local.get 0
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      i32.eqz
      br_if 0 (;@1;)
      local.get 2
      i32.eqz
      br_if 0 (;@1;)
      local.get 4
      i32.eqz
      br_if 0 (;@1;)
      local.get 5
      i32.eqz
      br_if 0 (;@1;)
      local.get 6
      i32.const 0
      i64.load offset=1049505 align=1
      i64.store offset=16
      local.get 6
      i32.const 0
      i64.load offset=1049497 align=1
      i64.store offset=8
      local.get 6
      local.get 0
      i64.load align=1
      i64.store offset=24
      local.get 6
      local.get 0
      i64.load offset=8 align=1
      i64.store offset=32
      local.get 6
      local.get 0
      i64.load offset=16 align=1
      i64.store offset=40
      local.get 6
      local.get 0
      i64.load offset=24 align=1
      i64.store offset=48
      local.get 6
      local.get 1
      i64.load align=1
      i64.store offset=56
      local.get 6
      local.get 1
      i64.load offset=8 align=1
      i64.store offset=64
      local.get 6
      local.get 1
      i64.load offset=16 align=1
      i64.store offset=72
      local.get 6
      local.get 1
      i64.load offset=24 align=1
      i64.store offset=80
      local.get 6
      local.get 2
      i64.load offset=8 align=1
      i64.store offset=96
      local.get 6
      local.get 2
      i64.load align=1
      i64.store offset=88
      local.get 6
      local.get 3
      i32.const 16711935
      i32.and
      i32.const 8
      i32.rotr
      local.get 3
      i32.const 24
      i32.rotr
      i32.const 16711935
      i32.and
      i32.or
      i32.store offset=104
      local.get 6
      i32.const 0
      i32.store8 offset=111
      local.get 6
      i32.const 416
      i32.add
      local.get 6
      i32.const 111
      i32.add
      local.get 6
      i32.const 8
      i32.add
      i32.const 100
      call $_ZN5equix12EquiXBuilder5build17h3045b0a3930df7a1E
      block  ;; label = @2
        local.get 6
        i32.load offset=416
        br_if 0 (;@2;)
        local.get 6
        local.get 6
        i64.load offset=456
        i64.store offset=144
        local.get 6
        local.get 6
        i64.load offset=448
        i64.store offset=136
        local.get 6
        local.get 6
        i64.load offset=440
        i64.store offset=128
        local.get 6
        local.get 6
        i64.load offset=432
        i64.store offset=120
        local.get 6
        local.get 6
        i64.load offset=424
        i64.store offset=112
        local.get 6
        i32.const 152
        i32.add
        local.get 6
        i32.const 112
        i32.add
        call $_ZN5equix5EquiX5solve17hae13e53cffa3fde3E
        block  ;; label = @3
          block  ;; label = @4
            local.get 6
            i32.load offset=152
            local.tee 0
            i32.eqz
            br_if 0 (;@4;)
            local.get 0
            i32.const 4
            i32.shl
            local.set 8
            local.get 6
            i32.const 152
            i32.add
            i32.const 4
            i32.add
            local.set 0
            local.get 6
            i32.const 608
            i32.add
            local.set 1
            local.get 6
            i32.const 492
            i32.add
            local.set 2
            local.get 6
            i32.const 624
            i32.add
            i32.const 8
            i32.add
            local.set 7
            local.get 6
            i32.const 300
            i32.add
            i32.const 100
            i32.add
            local.set 9
            loop  ;; label = @5
              local.get 6
              i32.const 284
              i32.add
              local.get 0
              call $_ZN5equix8solution8Solution8to_bytes17hc2695acc66895ba7E
              local.get 6
              i32.const 300
              i32.add
              local.get 6
              i32.const 8
              i32.add
              i32.const 100
              memory.copy
              local.get 9
              local.get 6
              i64.load offset=292 align=1
              i64.store offset=8 align=1
              local.get 9
              local.get 6
              i64.load offset=284 align=1
              i64.store align=1
              local.get 7
              i32.const 1
              i32.const 0
              i32.const 1
              i32.const 0
              i32.const 0
              i32.const 4
              call $_ZN6blake214Blake2bVarCore15new_with_params17hbddb78c2cf65de78E
              local.get 6
              i32.const 416
              i32.add
              local.get 7
              i32.const 72
              memory.copy
              local.get 6
              i32.const 4
              i32.store offset=488
              local.get 2
              local.get 6
              i32.const 300
              i32.add
              i32.const 116
              memory.copy
              local.get 1
              i64.const 0
              i64.store
              local.get 1
              i64.const 0
              i64.store offset=5 align=1
              local.get 6
              i64.const 0
              i64.store offset=680
              local.get 6
              i64.const 0
              i64.store offset=672
              local.get 6
              i64.const 0
              i64.store offset=664
              local.get 6
              i64.const 0
              i64.store offset=656
              local.get 6
              i64.const 0
              i64.store offset=648
              local.get 6
              i64.const 0
              i64.store offset=640
              local.get 6
              i64.const 0
              i64.store offset=632
              local.get 6
              i64.const 0
              i64.store offset=624
              local.get 6
              local.get 6
              i64.load offset=480
              i64.const 116
              i64.add
              i64.store offset=480
              local.get 6
              i32.const 416
              i32.add
              local.get 2
              i64.const 0
              local.get 6
              i32.const 624
              i32.add
              call $_ZN6blake214Blake2bVarCore18finalize_with_flag17h8280626bb9c29f91E
              local.get 6
              i32.load offset=624
              local.tee 10
              i32.const 16711935
              i32.and
              i32.const 8
              i32.rotr
              local.get 10
              i32.const 24
              i32.rotr
              i32.const 16711935
              i32.and
              i32.or
              local.tee 10
              i64.extend_i32_u
              local.get 3
              i64.extend_i32_u
              i64.mul
              i64.const 32
              i64.shr_u
              i32.wrap_i64
              i32.eqz
              br_if 2 (;@3;)
              local.get 0
              i32.const 16
              i32.add
              local.set 0
              local.get 8
              i32.const -16
              i32.add
              local.tee 8
              br_if 0 (;@5;)
            end
          end
          i32.const -6
          local.set 7
          local.get 6
          i32.load offset=144
          local.tee 0
          i32.eqz
          br_if 2 (;@1;)
          local.get 0
          i32.const 4096
          i32.const 4
          call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
          br 2 (;@1;)
        end
        local.get 4
        local.get 6
        i64.load offset=292 align=1
        i64.store offset=8 align=1
        local.get 4
        local.get 6
        i64.load offset=284 align=1
        i64.store align=1
        local.get 5
        local.get 10
        i32.store
        i32.const 0
        local.set 7
        local.get 6
        i32.load offset=144
        local.tee 0
        i32.eqz
        br_if 1 (;@1;)
        local.get 0
        i32.const 4096
        i32.const 4
        call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
        br 1 (;@1;)
      end
      block  ;; label = @2
        local.get 6
        i32.load offset=420
        local.tee 0
        i32.const 1
        i32.gt_u
        br_if 0 (;@2;)
        local.get 0
        i32.eqz
        br_if 0 (;@2;)
        local.get 6
        i32.load offset=424
        local.tee 0
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        local.get 0
        i32.load
        local.tee 1
        i32.const -1
        i32.add
        i32.store
        local.get 1
        i32.const 1
        i32.ne
        br_if 0 (;@2;)
        local.get 6
        i32.const 424
        i32.add
        call $_ZN5alloc4sync16Arc$LT$T$C$A$GT$9drop_slow17h543bbdef373111b6E
      end
      i32.const -4
      local.set 7
    end
    local.get 6
    i32.const 704
    i32.add
    global.set $__stack_pointer
    local.get 7)
  (func $tor_hs_pow_v1_verify (type 8) (param i32 i32 i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i64 i64)
    global.get $__stack_pointer
    i32.const 528
    i32.sub
    local.tee 7
    global.set $__stack_pointer
    i32.const -1
    local.set 8
    block  ;; label = @1
      local.get 0
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      i32.eqz
      br_if 0 (;@1;)
      local.get 2
      i32.eqz
      br_if 0 (;@1;)
      local.get 4
      i32.eqz
      br_if 0 (;@1;)
      local.get 5
      i32.eqz
      br_if 0 (;@1;)
      local.get 6
      i32.eqz
      br_if 0 (;@1;)
      i32.const -3
      local.set 8
      local.get 4
      i32.load8_u
      local.get 1
      i32.load8_u
      local.tee 9
      i32.ne
      br_if 0 (;@1;)
      local.get 4
      i32.load8_u offset=1
      i32.const 255
      i32.and
      local.get 1
      i32.load8_u offset=1
      local.tee 10
      i32.const 255
      i32.and
      i32.ne
      br_if 0 (;@1;)
      local.get 4
      i32.load8_u offset=2
      i32.const 255
      i32.and
      local.get 1
      i32.load8_u offset=2
      local.tee 11
      i32.const 255
      i32.and
      i32.ne
      br_if 0 (;@1;)
      local.get 4
      i32.load8_u offset=3
      i32.const 255
      i32.and
      local.get 1
      i32.load8_u offset=3
      i32.const 255
      i32.and
      local.tee 12
      i32.ne
      br_if 0 (;@1;)
      i32.const -4096
      local.set 8
      i32.const 1051028
      local.set 13
      i32.const 1050768
      local.set 14
      loop  ;; label = @2
        block  ;; label = @3
          local.get 14
          i32.load8_u
          i32.eqz
          br_if 0 (;@3;)
          local.get 9
          local.get 13
          i32.load8_u
          i32.ne
          br_if 0 (;@3;)
          local.get 10
          i32.const 255
          i32.and
          local.get 13
          i32.const 1
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 11
          i32.const 255
          i32.and
          local.get 13
          i32.const 2
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 12
          local.get 13
          i32.const 3
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u
          local.get 8
          i32.const 1056152
          i32.add
          i32.load8_u
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=1
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056153
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=2
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056154
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=3
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056155
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=4
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056156
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=5
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056157
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=6
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056158
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=7
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056159
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=8
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056160
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=9
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056161
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=10
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056162
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=11
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056163
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=12
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056164
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=13
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056165
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=14
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056166
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u offset=15
          i32.const 255
          i32.and
          local.get 8
          i32.const 1056167
          i32.add
          i32.load8_u
          i32.const 255
          i32.and
          i32.ne
          br_if 0 (;@3;)
          i32.const -5
          local.set 8
          br 2 (;@1;)
        end
        local.get 14
        i32.const 1
        i32.add
        local.set 14
        local.get 13
        i32.const 4
        i32.add
        local.set 13
        local.get 8
        i32.const 16
        i32.add
        local.tee 8
        br_if 0 (;@2;)
      end
      local.get 7
      i32.const 0
      i64.load offset=1049505 align=1
      i64.store offset=24
      local.get 7
      i32.const 0
      i64.load offset=1049497 align=1
      i64.store offset=16
      local.get 7
      local.get 0
      i64.load align=1
      i64.store offset=32
      local.get 7
      local.get 0
      i64.load offset=8 align=1
      i64.store offset=40
      local.get 7
      local.get 0
      i64.load offset=16 align=1
      i64.store offset=48
      local.get 7
      local.get 0
      i64.load offset=24 align=1
      i64.store offset=56
      local.get 7
      local.get 1
      i64.load align=1
      i64.store offset=64
      local.get 7
      local.get 1
      i64.load offset=8 align=1
      i64.store offset=72
      local.get 7
      local.get 1
      i64.load offset=16 align=1
      i64.store offset=80
      local.get 7
      local.get 1
      i64.load offset=24 align=1
      i64.store offset=88
      local.get 7
      local.get 2
      i64.load offset=8 align=1
      i64.store offset=104
      local.get 7
      local.get 2
      i64.load align=1
      i64.store offset=96
      local.get 7
      local.get 3
      i32.const 16711935
      i32.and
      i32.const 8
      i32.rotr
      local.get 3
      i32.const 24
      i32.rotr
      i32.const 16711935
      i32.and
      i32.or
      i32.store offset=112
      local.get 7
      i32.const 116
      i32.add
      local.get 7
      i32.const 16
      i32.add
      i32.const 100
      memory.copy
      local.get 7
      local.get 5
      i64.load offset=8 align=1
      i64.store offset=224 align=1
      local.get 7
      local.get 5
      i64.load align=1
      i64.store offset=216 align=1
      local.get 7
      i32.const 440
      i32.add
      i32.const 8
      i32.add
      local.tee 8
      i32.const 1
      i32.const 0
      i32.const 1
      i32.const 0
      i32.const 0
      i32.const 4
      call $_ZN6blake214Blake2bVarCore15new_with_params17hbddb78c2cf65de78E
      local.get 7
      i32.const 232
      i32.add
      local.get 8
      i32.const 72
      memory.copy
      local.get 7
      i32.const 4
      i32.store offset=304
      local.get 7
      i32.const 308
      i32.add
      local.tee 8
      local.get 7
      i32.const 116
      i32.add
      i32.const 116
      memory.copy
      local.get 7
      i64.const 0
      i64.store offset=424
      local.get 7
      i64.const 0
      i64.store offset=429 align=1
      local.get 7
      i64.const 0
      i64.store offset=496
      local.get 7
      i64.const 0
      i64.store offset=488
      local.get 7
      i64.const 0
      i64.store offset=480
      local.get 7
      i64.const 0
      i64.store offset=472
      local.get 7
      i64.const 0
      i64.store offset=464
      local.get 7
      i64.const 0
      i64.store offset=456
      local.get 7
      i64.const 0
      i64.store offset=448
      local.get 7
      i64.const 0
      i64.store offset=440
      local.get 7
      local.get 7
      i64.load offset=296
      i64.const 116
      i64.add
      i64.store offset=296
      local.get 7
      i32.const 232
      i32.add
      local.get 8
      i64.const 0
      local.get 7
      i32.const 440
      i32.add
      call $_ZN6blake214Blake2bVarCore18finalize_with_flag17h8280626bb9c29f91E
      local.get 6
      local.get 7
      i32.load offset=440
      local.tee 8
      i32.const 16711935
      i32.and
      i32.const 8
      i32.rotr
      local.get 8
      i32.const 24
      i32.rotr
      i32.const 16711935
      i32.and
      i32.or
      local.tee 13
      i32.store
      i32.const -3
      local.set 8
      local.get 13
      i64.extend_i32_u
      local.get 3
      i64.extend_i32_u
      i64.mul
      i64.const 32
      i64.shr_u
      i32.wrap_i64
      br_if 0 (;@1;)
      local.get 7
      local.get 5
      i64.load offset=8 align=1
      i64.store offset=240
      local.get 7
      local.get 5
      i64.load align=1
      i64.store offset=232
      local.get 7
      i32.const 0
      i32.store8 offset=527
      local.get 7
      i32.const 8
      i32.add
      local.get 7
      i32.const 527
      i32.add
      local.get 7
      i32.const 16
      i32.add
      i32.const 100
      local.get 7
      i32.const 232
      i32.add
      call $_ZN5equix12EquiXBuilder12verify_bytes17he8cd41b059ffe8d4E
      local.get 7
      local.get 7
      i32.load offset=12
      local.tee 14
      i32.store offset=444
      local.get 7
      local.get 7
      i32.load offset=8
      local.tee 13
      i32.store offset=440
      block  ;; label = @2
        local.get 13
        i32.const 4
        i32.eq
        br_if 0 (;@2;)
        local.get 13
        i32.const 1
        i32.ne
        br_if 1 (;@1;)
        local.get 14
        i32.eqz
        br_if 1 (;@1;)
        local.get 14
        local.get 14
        i32.load
        local.tee 2
        i32.const -1
        i32.add
        i32.store
        local.get 2
        i32.const 1
        i32.ne
        br_if 1 (;@1;)
        local.get 7
        i32.const 440
        i32.add
        i32.const 4
        i32.add
        call $_ZN5alloc4sync16Arc$LT$T$C$A$GT$9drop_slow17h543bbdef373111b6E
        br 1 (;@1;)
      end
      i32.const 0
      local.set 8
      i32.const 0
      i32.load offset=1052052
      local.tee 14
      i32.const 255
      i32.and
      local.tee 13
      i32.const 2
      i32.shl
      local.get 4
      i32.load align=1
      i32.store offset=1051028 align=1
      i32.const 0
      local.get 14
      i32.const 1
      i32.add
      i32.const 255
      i32.and
      i32.store offset=1052052
      local.get 2
      i64.load offset=8 align=1
      local.set 15
      local.get 2
      i64.load align=1
      local.set 16
      local.get 13
      i32.const 1
      i32.store8 offset=1050768
      local.get 13
      i32.const 4
      i32.shl
      local.tee 2
      local.get 16
      i64.store offset=1052056 align=1
      local.get 2
      local.get 15
      i64.store offset=1052064 align=1
      i32.const 0
      i32.load offset=1051024
      local.tee 2
      i32.const 255
      i32.gt_u
      br_if 0 (;@1;)
      i32.const 0
      local.set 8
      i32.const 0
      local.get 2
      i32.const 1
      i32.add
      i32.store offset=1051024
    end
    local.get 7
    i32.const 528
    i32.add
    global.set $__stack_pointer
    local.get 8)
  (func $_RNvCsfLfy6EI15iL_7___rustc12___rust_alloc (type 3) (param i32 i32) (result i32)
    local.get 0
    local.get 1
    call $_RNvCsfLfy6EI15iL_7___rustc11___rdl_alloc
    return)
  (func $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc (type 9) (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    call $_RNvCsfLfy6EI15iL_7___rustc13___rdl_dealloc
    return)
  (func $_RNvCsfLfy6EI15iL_7___rustc14___rust_realloc (type 5) (param i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    call $_RNvCsfLfy6EI15iL_7___rustc13___rdl_realloc
    return)
  (func $_RNvCsfLfy6EI15iL_7___rustc35___rust_no_alloc_shim_is_unstable_v2 (type 10)
    return)
  (func $_ZN5equix6solver14find_solutions17hbf43fc419e357594E (type 9) (param i32 i32 i32)
    (local i32 i32 i32 i32 i64 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $__stack_pointer
    i32.const 2240
    i32.sub
    local.tee 3
    global.set $__stack_pointer
    local.get 1
    i32.load
    local.set 4
    local.get 3
    i32.const 8
    i32.add
    local.tee 5
    i32.const 0
    i32.const 512
    memory.fill
    local.get 3
    local.get 4
    i32.const 1720320
    i32.add
    local.tee 6
    i32.store offset=4
    local.get 3
    local.get 4
    i32.store
    i64.const 0
    local.set 7
    loop  ;; label = @1
      block  ;; label = @2
        local.get 5
        local.get 0
        local.get 7
        call $_ZN5hashx5HashX11hash_to_u6417h4c9db866b9cd67d7E
        local.tee 8
        i32.wrap_i64
        i32.const 255
        i32.and
        local.tee 9
        i32.const 1
        i32.shl
        i32.add
        local.tee 10
        i32.load16_u
        local.tee 1
        i32.const 335
        i32.gt_u
        br_if 0 (;@2;)
        local.get 10
        local.get 1
        i32.const 1
        i32.add
        i32.store16
        local.get 6
        local.get 9
        i32.const 672
        i32.mul
        i32.add
        local.get 1
        i32.const 1
        i32.shl
        i32.add
        local.get 7
        i64.store16
        local.get 4
        local.get 9
        i32.const 2688
        i32.mul
        i32.add
        local.get 1
        i32.const 3
        i32.shl
        i32.add
        local.get 8
        i64.const 8
        i64.shr_u
        i64.store
      end
      local.get 7
      i64.const 1
      i64.add
      local.tee 7
      i64.const 65536
      i64.ne
      br_if 0 (;@1;)
    end
    local.get 3
    i32.const 520
    i32.add
    i32.const 8
    i32.add
    local.tee 11
    i32.const 0
    i32.const 512
    memory.fill
    local.get 3
    local.get 4
    i32.const 1376256
    i32.add
    local.tee 12
    i32.store offset=524
    local.get 3
    local.get 4
    i32.const 688128
    i32.add
    local.tee 13
    i32.store offset=520
    local.get 4
    i32.const 1892352
    i32.add
    local.set 14
    local.get 3
    i32.const 1560
    i32.add
    local.set 15
    i32.const 0
    local.set 16
    block  ;; label = @1
      loop  ;; label = @2
        local.get 15
        i32.const 0
        i32.const 128
        memory.fill
        block  ;; label = @3
          local.get 5
          local.get 16
          i32.const 1
          i32.shl
          i32.add
          i32.load16_u
          local.tee 10
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          local.get 16
          i32.const 2688
          i32.mul
          i32.add
          local.set 9
          i32.const 0
          local.set 1
          loop  ;; label = @4
            local.get 1
            i32.const 336
            i32.eq
            br_if 3 (;@1;)
            block  ;; label = @5
              local.get 15
              local.get 9
              i32.load
              i32.const 127
              i32.and
              local.tee 17
              i32.add
              local.tee 18
              i32.load8_u
              local.tee 0
              i32.const 11
              i32.gt_u
              br_if 0 (;@5;)
              local.get 18
              local.get 0
              i32.const 1
              i32.add
              i32.store8
              local.get 14
              local.get 17
              i32.const 24
              i32.mul
              i32.add
              local.get 0
              i32.const 1
              i32.shl
              i32.add
              local.get 1
              i32.store16
            end
            local.get 9
            i32.const 8
            i32.add
            local.set 9
            local.get 10
            local.get 1
            i32.const 1
            i32.add
            local.tee 1
            i32.ne
            br_if 0 (;@4;)
          end
        end
        block  ;; label = @3
          local.get 5
          i32.const 0
          local.get 16
          i32.sub
          i32.const 255
          i32.and
          local.tee 1
          i32.const 1
          i32.shl
          i32.add
          i32.load16_u
          local.tee 19
          i32.eqz
          br_if 0 (;@3;)
          local.get 16
          i32.const 18
          i32.shl
          local.set 20
          local.get 4
          local.get 16
          i32.const 2688
          i32.mul
          i32.add
          local.set 18
          local.get 4
          local.get 1
          i32.const 2688
          i32.mul
          i32.add
          local.set 21
          local.get 16
          i64.extend_i32_u
          local.set 22
          local.get 1
          i64.extend_i32_u
          local.set 23
          i32.const 0
          local.set 24
          loop  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 24
                i32.const 336
                i32.eq
                br_if 0 (;@6;)
                local.get 15
                i32.const 0
                local.get 21
                local.get 24
                i32.const 3
                i32.shl
                i32.add
                i64.load
                i64.const 8
                i64.shl
                local.get 23
                i64.or
                local.tee 7
                i32.wrap_i64
                i32.sub
                i32.const 8
                i32.shr_u
                i32.const 127
                i32.and
                local.tee 1
                i32.add
                i32.load8_u
                local.tee 9
                i32.eqz
                br_if 1 (;@5;)
                local.get 9
                i32.const 1
                i32.shl
                local.set 17
                local.get 20
                local.get 24
                i32.or
                local.set 25
                local.get 7
                local.get 22
                i64.add
                local.set 8
                local.get 14
                local.get 1
                i32.const 24
                i32.mul
                i32.add
                local.set 0
                i32.const 0
                local.set 1
                loop  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          local.get 1
                          i32.const 24
                          i32.eq
                          br_if 0 (;@11;)
                          local.get 0
                          local.get 1
                          i32.add
                          i32.load16_u
                          local.tee 9
                          local.get 10
                          i32.ge_u
                          br_if 1 (;@10;)
                          local.get 9
                          i32.const 336
                          i32.ge_u
                          br_if 2 (;@9;)
                          local.get 8
                          local.get 18
                          local.get 9
                          i32.const 3
                          i32.shl
                          i32.add
                          i64.load
                          i64.const 8
                          i64.shl
                          i64.add
                          local.tee 7
                          i64.const 32767
                          i64.and
                          i64.eqz
                          i32.eqz
                          br_if 3 (;@8;)
                          local.get 11
                          local.get 7
                          i32.wrap_i64
                          i32.const 15
                          i32.shr_u
                          i32.const 255
                          i32.and
                          local.tee 26
                          i32.const 1
                          i32.shl
                          i32.add
                          local.tee 27
                          i32.load16_u
                          local.tee 28
                          i32.const 335
                          i32.gt_u
                          br_if 3 (;@8;)
                          local.get 27
                          local.get 28
                          i32.const 1
                          i32.add
                          i32.store16
                          local.get 12
                          local.get 26
                          i32.const 1344
                          i32.mul
                          i32.add
                          local.get 28
                          i32.const 2
                          i32.shl
                          i32.add
                          local.get 9
                          i32.const 9
                          i32.shl
                          local.get 25
                          i32.or
                          i32.store
                          local.get 13
                          local.get 26
                          i32.const 2688
                          i32.mul
                          i32.add
                          local.get 28
                          i32.const 3
                          i32.shl
                          i32.add
                          local.get 7
                          i64.const 23
                          i64.shr_u
                          i64.store
                          br 3 (;@8;)
                        end
                        i32.const 12
                        i32.const 12
                        i32.const 1049592
                        call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                        unreachable
                      end
                      i32.const 1049513
                      i32.const 63
                      i32.const 1049608
                      call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
                      unreachable
                    end
                    local.get 9
                    i32.const 336
                    i32.const 1049624
                    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                    unreachable
                  end
                  local.get 17
                  local.get 1
                  i32.const 2
                  i32.add
                  local.tee 1
                  i32.eq
                  br_if 2 (;@5;)
                  br 0 (;@7;)
                end
              end
              i32.const 336
              i32.const 336
              i32.const 1049624
              call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
              unreachable
            end
            local.get 24
            i32.const 1
            i32.add
            local.tee 24
            local.get 19
            i32.ne
            br_if 0 (;@4;)
          end
        end
        block  ;; label = @3
          local.get 16
          i32.const 128
          i32.eq
          local.tee 1
          br_if 0 (;@3;)
          i32.const 128
          local.get 16
          i32.const 1
          i32.add
          local.get 1
          select
          local.tee 16
          i32.const 128
          i32.le_u
          br_if 1 (;@2;)
        end
      end
      local.get 3
      i32.const 1040
      i32.add
      i32.const 4
      i32.add
      local.tee 16
      local.get 5
      i32.const 512
      memory.copy
      local.get 3
      i32.const 1556
      i32.add
      i32.const 8
      i32.add
      local.tee 25
      i32.const 0
      i32.const 512
      memory.fill
      local.get 4
      i32.const 344064
      i32.add
      local.set 21
      local.get 3
      i32.const 2076
      i32.add
      i32.const 4
      i32.add
      local.set 20
      i32.const 0
      local.set 29
      block  ;; label = @2
        loop  ;; label = @3
          local.get 20
          i32.const 0
          i32.const 128
          memory.fill
          block  ;; label = @4
            local.get 11
            local.get 29
            i32.const 1
            i32.shl
            i32.add
            i32.load16_u
            local.tee 10
            i32.eqz
            br_if 0 (;@4;)
            local.get 13
            local.get 29
            i32.const 2688
            i32.mul
            i32.add
            local.set 9
            i32.const 0
            local.set 1
            loop  ;; label = @5
              local.get 1
              i32.const 336
              i32.eq
              br_if 3 (;@2;)
              block  ;; label = @6
                local.get 20
                local.get 9
                i32.load
                i32.const 127
                i32.and
                local.tee 17
                i32.add
                local.tee 18
                i32.load8_u
                local.tee 0
                i32.const 11
                i32.gt_u
                br_if 0 (;@6;)
                local.get 18
                local.get 0
                i32.const 1
                i32.add
                i32.store8
                local.get 14
                local.get 17
                i32.const 24
                i32.mul
                i32.add
                local.get 0
                i32.const 1
                i32.shl
                i32.add
                local.get 1
                i32.store16
              end
              local.get 9
              i32.const 8
              i32.add
              local.set 9
              local.get 10
              local.get 1
              i32.const 1
              i32.add
              local.tee 1
              i32.ne
              br_if 0 (;@5;)
            end
          end
          block  ;; label = @4
            local.get 11
            i32.const 0
            local.get 29
            i32.sub
            i32.const 255
            i32.and
            local.tee 1
            i32.const 1
            i32.shl
            i32.add
            i32.load16_u
            local.tee 5
            i32.eqz
            br_if 0 (;@4;)
            local.get 29
            i32.const 18
            i32.shl
            local.set 30
            local.get 13
            local.get 29
            i32.const 2688
            i32.mul
            i32.add
            local.set 18
            local.get 13
            local.get 1
            i32.const 2688
            i32.mul
            i32.add
            local.set 31
            local.get 29
            i64.extend_i32_u
            local.set 22
            local.get 1
            i64.extend_i32_u
            local.set 23
            i32.const 0
            local.set 15
            loop  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  local.get 15
                  i32.const 336
                  i32.eq
                  br_if 0 (;@7;)
                  local.get 20
                  i32.const 0
                  local.get 31
                  local.get 15
                  i32.const 3
                  i32.shl
                  i32.add
                  i64.load
                  i64.const 8
                  i64.shl
                  local.get 23
                  i64.or
                  local.tee 7
                  i32.wrap_i64
                  i32.sub
                  i32.const 8
                  i32.shr_u
                  i32.const 127
                  i32.and
                  local.tee 1
                  i32.add
                  i32.load8_u
                  local.tee 9
                  i32.eqz
                  br_if 1 (;@6;)
                  local.get 9
                  i32.const 1
                  i32.shl
                  local.set 17
                  local.get 30
                  local.get 15
                  i32.or
                  local.set 19
                  local.get 7
                  local.get 22
                  i64.add
                  local.set 8
                  local.get 14
                  local.get 1
                  i32.const 24
                  i32.mul
                  i32.add
                  local.set 0
                  i32.const 0
                  local.set 1
                  loop  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          block  ;; label = @12
                            local.get 1
                            i32.const 24
                            i32.eq
                            br_if 0 (;@12;)
                            local.get 0
                            local.get 1
                            i32.add
                            i32.load16_u
                            local.tee 9
                            local.get 10
                            i32.ge_u
                            br_if 1 (;@11;)
                            local.get 9
                            i32.const 336
                            i32.ge_u
                            br_if 2 (;@10;)
                            local.get 8
                            local.get 18
                            local.get 9
                            i32.const 3
                            i32.shl
                            i32.add
                            i64.load
                            i64.const 8
                            i64.shl
                            i64.add
                            local.tee 7
                            i64.const 32767
                            i64.and
                            i64.eqz
                            i32.eqz
                            br_if 3 (;@9;)
                            local.get 25
                            local.get 7
                            i64.const 15
                            i64.shr_u
                            i32.wrap_i64
                            local.tee 26
                            i32.const 255
                            i32.and
                            local.tee 27
                            i32.const 1
                            i32.shl
                            i32.add
                            local.tee 24
                            i32.load16_u
                            local.tee 28
                            i32.const 335
                            i32.gt_u
                            br_if 3 (;@9;)
                            local.get 24
                            local.get 28
                            i32.const 1
                            i32.add
                            i32.store16
                            local.get 21
                            local.get 27
                            i32.const 1344
                            i32.mul
                            local.tee 27
                            i32.add
                            local.get 28
                            i32.const 2
                            i32.shl
                            local.tee 28
                            i32.add
                            local.get 9
                            i32.const 9
                            i32.shl
                            local.get 19
                            i32.or
                            i32.store
                            local.get 4
                            local.get 27
                            i32.add
                            local.get 28
                            i32.add
                            local.get 26
                            i32.const 8
                            i32.shr_u
                            i32.store
                            br 3 (;@9;)
                          end
                          i32.const 12
                          i32.const 12
                          i32.const 1049592
                          call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                          unreachable
                        end
                        i32.const 1049513
                        i32.const 63
                        i32.const 1049608
                        call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
                        unreachable
                      end
                      local.get 9
                      i32.const 336
                      i32.const 1049624
                      call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                      unreachable
                    end
                    local.get 17
                    local.get 1
                    i32.const 2
                    i32.add
                    local.tee 1
                    i32.eq
                    br_if 2 (;@6;)
                    br 0 (;@8;)
                  end
                end
                i32.const 336
                i32.const 336
                i32.const 1049624
                call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                unreachable
              end
              local.get 15
              i32.const 1
              i32.add
              local.tee 15
              local.get 5
              i32.ne
              br_if 0 (;@5;)
            end
          end
          block  ;; label = @4
            local.get 29
            i32.const 128
            i32.eq
            local.tee 1
            br_if 0 (;@4;)
            i32.const 128
            local.get 29
            i32.const 1
            i32.add
            local.get 1
            select
            local.tee 29
            i32.const 128
            i32.le_u
            br_if 1 (;@3;)
          end
        end
        local.get 2
        i32.const 4
        i32.add
        local.set 32
        local.get 2
        i32.const -12
        i32.add
        local.set 33
        local.get 2
        i32.load
        local.set 34
        local.get 3
        i32.const 2076
        i32.add
        i32.const 4
        i32.add
        local.set 35
        i32.const 0
        local.set 36
        block  ;; label = @3
          loop  ;; label = @4
            local.get 35
            i32.const 0
            i32.const 128
            memory.fill
            block  ;; label = @5
              local.get 25
              local.get 36
              i32.const 1
              i32.shl
              i32.add
              i32.load16_u
              local.tee 17
              i32.eqz
              br_if 0 (;@5;)
              local.get 4
              local.get 36
              i32.const 1344
              i32.mul
              i32.add
              local.set 9
              i32.const 0
              local.set 1
              loop  ;; label = @6
                local.get 1
                i32.const 336
                i32.eq
                br_if 3 (;@3;)
                block  ;; label = @7
                  local.get 35
                  local.get 9
                  i32.load
                  i32.const 127
                  i32.and
                  local.tee 0
                  i32.add
                  local.tee 18
                  i32.load8_u
                  local.tee 10
                  i32.const 11
                  i32.gt_u
                  br_if 0 (;@7;)
                  local.get 18
                  local.get 10
                  i32.const 1
                  i32.add
                  i32.store8
                  local.get 14
                  local.get 0
                  i32.const 24
                  i32.mul
                  i32.add
                  local.get 10
                  i32.const 1
                  i32.shl
                  i32.add
                  local.get 1
                  i32.store16
                end
                local.get 9
                i32.const 4
                i32.add
                local.set 9
                local.get 17
                local.get 1
                i32.const 1
                i32.add
                local.tee 1
                i32.ne
                br_if 0 (;@6;)
              end
            end
            block  ;; label = @5
              local.get 25
              i32.const 0
              local.get 36
              i32.sub
              i32.const 255
              i32.and
              local.tee 37
              i32.const 1
              i32.shl
              i32.add
              i32.load16_u
              local.tee 38
              i32.eqz
              br_if 0 (;@5;)
              local.get 21
              local.get 37
              i32.const 1344
              i32.mul
              local.tee 1
              i32.add
              local.set 39
              local.get 21
              local.get 36
              i32.const 1344
              i32.mul
              local.tee 9
              i32.add
              local.set 40
              local.get 4
              local.get 9
              i32.add
              local.set 5
              local.get 4
              local.get 1
              i32.add
              local.set 41
              i32.const 0
              local.set 42
              local.get 34
              local.set 0
              loop  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 42
                    i32.const 336
                    i32.eq
                    br_if 0 (;@8;)
                    local.get 35
                    i32.const 0
                    local.get 41
                    local.get 42
                    i32.const 2
                    i32.shl
                    local.tee 1
                    i32.add
                    i32.load
                    i32.const 8
                    i32.shl
                    local.get 37
                    i32.or
                    local.tee 9
                    i32.sub
                    i32.const 8
                    i32.shr_u
                    i32.const 127
                    i32.and
                    local.tee 10
                    i32.add
                    i32.load8_u
                    local.tee 18
                    i32.eqz
                    br_if 1 (;@7;)
                    local.get 18
                    i32.const 1
                    i32.shl
                    local.set 20
                    local.get 9
                    local.get 36
                    i32.add
                    local.set 19
                    local.get 39
                    local.get 1
                    i32.add
                    local.set 43
                    local.get 14
                    local.get 10
                    i32.const 24
                    i32.mul
                    i32.add
                    local.set 15
                    i32.const 0
                    local.set 1
                    loop  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          block  ;; label = @12
                            block  ;; label = @13
                              block  ;; label = @14
                                block  ;; label = @15
                                  block  ;; label = @16
                                    block  ;; label = @17
                                      block  ;; label = @18
                                        block  ;; label = @19
                                          block  ;; label = @20
                                            block  ;; label = @21
                                              local.get 1
                                              i32.const 24
                                              i32.eq
                                              br_if 0 (;@21;)
                                              local.get 15
                                              local.get 1
                                              i32.add
                                              i32.load16_u
                                              local.tee 9
                                              local.get 17
                                              i32.ge_u
                                              br_if 1 (;@20;)
                                              local.get 9
                                              i32.const 336
                                              i32.ge_u
                                              br_if 2 (;@19;)
                                              local.get 19
                                              local.get 5
                                              local.get 9
                                              i32.const 2
                                              i32.shl
                                              local.tee 9
                                              i32.add
                                              i32.load
                                              i32.const 8
                                              i32.shl
                                              i32.add
                                              i32.const 1073741823
                                              i32.and
                                              br_if 11 (;@10;)
                                              local.get 40
                                              local.get 9
                                              i32.add
                                              i32.load
                                              local.tee 10
                                              i32.const 9
                                              i32.shr_u
                                              i32.const 511
                                              i32.and
                                              local.tee 9
                                              local.get 11
                                              local.get 10
                                              i32.const 18
                                              i32.shr_u
                                              local.tee 18
                                              i32.const 255
                                              i32.and
                                              local.tee 28
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 3 (;@18;)
                                              local.get 9
                                              i32.const 336
                                              i32.ge_u
                                              br_if 4 (;@17;)
                                              local.get 10
                                              i32.const 511
                                              i32.and
                                              local.tee 10
                                              local.get 11
                                              i32.const 0
                                              local.get 18
                                              i32.sub
                                              i32.const 255
                                              i32.and
                                              local.tee 26
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 5 (;@16;)
                                              local.get 10
                                              i32.const 336
                                              i32.ge_u
                                              br_if 6 (;@15;)
                                              local.get 12
                                              local.get 28
                                              i32.const 1344
                                              i32.mul
                                              i32.add
                                              local.get 9
                                              i32.const 2
                                              i32.shl
                                              i32.add
                                              i32.load
                                              local.tee 9
                                              i32.const 9
                                              i32.shr_u
                                              i32.const 511
                                              i32.and
                                              local.tee 18
                                              local.get 16
                                              local.get 9
                                              i32.const 18
                                              i32.shr_u
                                              local.tee 27
                                              i32.const 255
                                              i32.and
                                              local.tee 31
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 7 (;@14;)
                                              local.get 18
                                              i32.const 336
                                              i32.ge_u
                                              br_if 10 (;@11;)
                                              local.get 9
                                              i32.const 511
                                              i32.and
                                              local.tee 28
                                              local.get 16
                                              i32.const 0
                                              local.get 27
                                              i32.sub
                                              i32.const 255
                                              i32.and
                                              local.tee 30
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 8 (;@13;)
                                              local.get 28
                                              i32.const 336
                                              i32.ge_u
                                              br_if 9 (;@12;)
                                              local.get 12
                                              local.get 26
                                              i32.const 1344
                                              i32.mul
                                              i32.add
                                              local.get 10
                                              i32.const 2
                                              i32.shl
                                              i32.add
                                              i32.load
                                              local.tee 9
                                              i32.const 9
                                              i32.shr_u
                                              i32.const 511
                                              i32.and
                                              local.tee 26
                                              local.get 16
                                              local.get 9
                                              i32.const 18
                                              i32.shr_u
                                              local.tee 10
                                              i32.const 255
                                              i32.and
                                              local.tee 29
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 7 (;@14;)
                                              block  ;; label = @22
                                                local.get 26
                                                i32.const 335
                                                i32.le_u
                                                br_if 0 (;@22;)
                                                local.get 26
                                                local.set 18
                                                br 11 (;@11;)
                                              end
                                              local.get 9
                                              i32.const 511
                                              i32.and
                                              local.tee 27
                                              local.get 16
                                              i32.const 0
                                              local.get 10
                                              i32.sub
                                              i32.const 255
                                              i32.and
                                              local.tee 44
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 8 (;@13;)
                                              block  ;; label = @22
                                                local.get 27
                                                i32.const 335
                                                i32.le_u
                                                br_if 0 (;@22;)
                                                local.get 27
                                                local.set 28
                                                br 10 (;@12;)
                                              end
                                              local.get 43
                                              i32.load
                                              local.tee 10
                                              i32.const 9
                                              i32.shr_u
                                              i32.const 511
                                              i32.and
                                              local.tee 9
                                              local.get 11
                                              local.get 10
                                              i32.const 18
                                              i32.shr_u
                                              local.tee 24
                                              i32.const 255
                                              i32.and
                                              local.tee 13
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 3 (;@18;)
                                              local.get 9
                                              i32.const 335
                                              i32.gt_u
                                              br_if 4 (;@17;)
                                              local.get 10
                                              i32.const 511
                                              i32.and
                                              local.tee 10
                                              local.get 11
                                              i32.const 0
                                              local.get 24
                                              i32.sub
                                              i32.const 255
                                              i32.and
                                              local.tee 45
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 5 (;@16;)
                                              local.get 10
                                              i32.const 335
                                              i32.gt_u
                                              br_if 6 (;@15;)
                                              local.get 12
                                              local.get 13
                                              i32.const 1344
                                              i32.mul
                                              i32.add
                                              local.get 9
                                              i32.const 2
                                              i32.shl
                                              i32.add
                                              i32.load
                                              local.tee 9
                                              i32.const 9
                                              i32.shr_u
                                              i32.const 511
                                              i32.and
                                              local.tee 24
                                              local.get 16
                                              local.get 9
                                              i32.const 18
                                              i32.shr_u
                                              local.tee 13
                                              i32.const 255
                                              i32.and
                                              local.tee 46
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 7 (;@14;)
                                              block  ;; label = @22
                                                local.get 24
                                                i32.const 335
                                                i32.le_u
                                                br_if 0 (;@22;)
                                                local.get 24
                                                local.set 18
                                                br 11 (;@11;)
                                              end
                                              local.get 9
                                              i32.const 511
                                              i32.and
                                              local.tee 9
                                              local.get 16
                                              i32.const 0
                                              local.get 13
                                              i32.sub
                                              i32.const 255
                                              i32.and
                                              local.tee 47
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 8 (;@13;)
                                              block  ;; label = @22
                                                local.get 9
                                                i32.const 335
                                                i32.le_u
                                                br_if 0 (;@22;)
                                                local.get 9
                                                local.set 28
                                                br 10 (;@12;)
                                              end
                                              local.get 12
                                              local.get 45
                                              i32.const 1344
                                              i32.mul
                                              i32.add
                                              local.get 10
                                              i32.const 2
                                              i32.shl
                                              i32.add
                                              i32.load
                                              local.tee 10
                                              i32.const 9
                                              i32.shr_u
                                              i32.const 511
                                              i32.and
                                              local.tee 13
                                              local.get 16
                                              local.get 10
                                              i32.const 18
                                              i32.shr_u
                                              local.tee 45
                                              i32.const 255
                                              i32.and
                                              local.tee 48
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 7 (;@14;)
                                              block  ;; label = @22
                                                local.get 13
                                                i32.const 335
                                                i32.le_u
                                                br_if 0 (;@22;)
                                                local.get 13
                                                local.set 18
                                                br 11 (;@11;)
                                              end
                                              local.get 10
                                              i32.const 511
                                              i32.and
                                              local.tee 10
                                              local.get 16
                                              i32.const 0
                                              local.get 45
                                              i32.sub
                                              i32.const 255
                                              i32.and
                                              local.tee 45
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.ge_u
                                              br_if 8 (;@13;)
                                              block  ;; label = @22
                                                local.get 10
                                                i32.const 335
                                                i32.le_u
                                                br_if 0 (;@22;)
                                                local.get 10
                                                local.set 28
                                                br 10 (;@12;)
                                              end
                                              local.get 6
                                              local.get 31
                                              i32.const 672
                                              i32.mul
                                              i32.add
                                              local.get 18
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              local.set 31
                                              local.get 6
                                              local.get 30
                                              i32.const 672
                                              i32.mul
                                              i32.add
                                              local.get 28
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              local.set 28
                                              local.get 6
                                              local.get 29
                                              i32.const 672
                                              i32.mul
                                              i32.add
                                              local.get 26
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              local.set 26
                                              local.get 6
                                              local.get 44
                                              i32.const 672
                                              i32.mul
                                              i32.add
                                              local.get 27
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              local.set 18
                                              local.get 6
                                              local.get 46
                                              i32.const 672
                                              i32.mul
                                              i32.add
                                              local.get 24
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              local.set 27
                                              local.get 6
                                              local.get 48
                                              i32.const 672
                                              i32.mul
                                              i32.add
                                              local.get 13
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              local.set 24
                                              local.get 6
                                              local.get 45
                                              i32.const 672
                                              i32.mul
                                              i32.add
                                              local.get 10
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              local.set 10
                                              local.get 3
                                              local.get 6
                                              local.get 47
                                              i32.const 672
                                              i32.mul
                                              i32.add
                                              local.get 9
                                              i32.const 1
                                              i32.shl
                                              i32.add
                                              i32.load16_u
                                              i32.store16 offset=2212
                                              local.get 3
                                              local.get 18
                                              local.get 27
                                              i32.const 16
                                              i32.shl
                                              i32.or
                                              i32.store offset=2208
                                              local.get 3
                                              local.get 18
                                              i32.store16 offset=2230
                                              local.get 3
                                              local.get 28
                                              local.get 26
                                              i32.const 16
                                              i32.shl
                                              i32.or
                                              i32.store offset=2226 align=2
                                              local.get 3
                                              local.get 31
                                              i32.store16 offset=2224
                                              local.get 3
                                              local.get 24
                                              local.get 10
                                              i32.const 16
                                              i32.shl
                                              i32.or
                                              i32.store offset=2236 align=2
                                              local.get 3
                                              local.get 3
                                              i32.load offset=2210 align=2
                                              i32.store offset=2232 align=2
                                              local.get 3
                                              i32.const 2224
                                              i32.add
                                              i32.const 8
                                              call $_ZN5equix8solution20sort_into_tree_order17h39428a4012a0a9dcE.llvm.2140353953980798599
                                              local.get 3
                                              local.get 3
                                              i64.load offset=2232 align=2
                                              i64.store offset=2216
                                              local.get 3
                                              local.get 3
                                              i64.load offset=2224 align=2
                                              i64.store offset=2208
                                              block  ;; label = @22
                                                block  ;; label = @23
                                                  local.get 0
                                                  br_if 0 (;@23;)
                                                  local.get 3
                                                  local.get 3
                                                  i64.load offset=2216
                                                  i64.store offset=2232
                                                  local.get 3
                                                  local.get 3
                                                  i64.load offset=2208
                                                  i64.store offset=2224
                                                  br 1 (;@22;)
                                                end
                                                local.get 33
                                                local.get 0
                                                i32.const 4
                                                i32.shl
                                                i32.add
                                                local.tee 9
                                                i64.load align=1
                                                local.get 3
                                                i64.load offset=2208
                                                i64.xor
                                                local.get 9
                                                i32.const 8
                                                i32.add
                                                i64.load align=1
                                                local.get 3
                                                i32.const 2208
                                                i32.add
                                                i32.const 8
                                                i32.add
                                                i64.load
                                                i64.xor
                                                i64.or
                                                i64.eqz
                                                br_if 12 (;@10;)
                                                local.get 3
                                                local.get 3
                                                i64.load offset=2216
                                                i64.store offset=2232
                                                local.get 3
                                                local.get 3
                                                i64.load offset=2208
                                                i64.store offset=2224
                                                local.get 0
                                                i32.const 7
                                                i32.gt_u
                                                br_if 12 (;@10;)
                                              end
                                              local.get 2
                                              local.get 0
                                              i32.const 1
                                              i32.add
                                              local.tee 34
                                              i32.store
                                              local.get 32
                                              local.get 0
                                              i32.const 4
                                              i32.shl
                                              i32.add
                                              local.tee 9
                                              local.get 3
                                              i64.load offset=2232
                                              i64.store offset=8 align=2
                                              local.get 9
                                              local.get 3
                                              i64.load offset=2224
                                              i64.store align=2
                                              local.get 34
                                              local.set 0
                                              br 11 (;@10;)
                                            end
                                            i32.const 12
                                            i32.const 12
                                            i32.const 1049592
                                            call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                            unreachable
                                          end
                                          i32.const 1049513
                                          i32.const 63
                                          i32.const 1049608
                                          call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
                                          unreachable
                                        end
                                        local.get 9
                                        i32.const 336
                                        i32.const 1049624
                                        call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                        unreachable
                                      end
                                      i32.const 1049513
                                      i32.const 63
                                      i32.const 1049640
                                      call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
                                      unreachable
                                    end
                                    local.get 9
                                    i32.const 336
                                    i32.const 1049656
                                    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                    unreachable
                                  end
                                  i32.const 1049513
                                  i32.const 63
                                  i32.const 1049640
                                  call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
                                  unreachable
                                end
                                local.get 10
                                i32.const 336
                                i32.const 1049656
                                call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                unreachable
                              end
                              i32.const 1049513
                              i32.const 63
                              i32.const 1049576
                              call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
                              unreachable
                            end
                            i32.const 1049513
                            i32.const 63
                            i32.const 1049576
                            call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
                            unreachable
                          end
                          local.get 28
                          i32.const 336
                          i32.const 1049592
                          call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                          unreachable
                        end
                        local.get 18
                        i32.const 336
                        i32.const 1049592
                        call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                        unreachable
                      end
                      local.get 20
                      local.get 1
                      i32.const 2
                      i32.add
                      local.tee 1
                      i32.eq
                      br_if 2 (;@7;)
                      br 0 (;@9;)
                    end
                  end
                  i32.const 336
                  i32.const 336
                  i32.const 1049624
                  call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                  unreachable
                end
                local.get 42
                i32.const 1
                i32.add
                local.tee 42
                local.get 38
                i32.ne
                br_if 0 (;@6;)
              end
            end
            block  ;; label = @5
              local.get 36
              i32.const 128
              i32.eq
              local.tee 1
              br_if 0 (;@5;)
              i32.const 128
              local.get 36
              i32.const 1
              i32.add
              local.get 1
              select
              local.tee 36
              i32.const 128
              i32.le_u
              br_if 1 (;@4;)
            end
          end
          local.get 3
          i32.const 2240
          i32.add
          global.set $__stack_pointer
          return
        end
        i32.const 336
        i32.const 336
        i32.const 1049624
        call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
        unreachable
      end
      i32.const 336
      i32.const 336
      i32.const 1049624
      call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
      unreachable
    end
    i32.const 336
    i32.const 336
    i32.const 1049624
    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
    unreachable)
  (func $_ZN5equix8solution16check_tree_order17h1c76aff39c15ed47E (type 3) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    loop  ;; label = @1
      local.get 1
      i32.const 1
      i32.shl
      local.set 2
      local.get 0
      i32.const -2
      i32.add
      local.set 3
      i32.const 0
      local.get 1
      i32.const -2
      i32.and
      local.tee 4
      i32.sub
      local.set 5
      local.get 1
      local.get 1
      i32.const 1
      i32.shr_u
      local.tee 6
      i32.sub
      local.set 7
      local.get 0
      local.get 4
      i32.add
      local.set 8
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            loop  ;; label = @5
              local.get 4
              i32.eqz
              br_if 1 (;@4;)
              i32.const 0
              local.set 9
              local.get 5
              local.get 2
              i32.add
              i32.eqz
              br_if 2 (;@3;)
              local.get 3
              local.get 4
              i32.add
              local.set 10
              local.get 3
              local.get 2
              i32.add
              local.set 11
              local.get 4
              i32.const -2
              i32.add
              local.set 4
              local.get 2
              i32.const -2
              i32.add
              local.set 2
              local.get 10
              i32.load16_u
              i32.const 65535
              i32.and
              local.tee 10
              local.get 11
              i32.load16_u
              local.tee 11
              i32.eq
              br_if 0 (;@5;)
            end
            local.get 10
            local.get 11
            i32.gt_u
            br_if 1 (;@3;)
          end
          local.get 1
          i32.const 2
          i32.ne
          br_if 1 (;@2;)
          i32.const 1
          local.set 9
        end
        local.get 9
        return
      end
      local.get 0
      local.get 6
      call $_ZN5equix8solution16check_tree_order17h1c76aff39c15ed47E
      local.set 4
      local.get 8
      local.set 0
      local.get 7
      local.set 1
      local.get 4
      br_if 0 (;@1;)
    end
    i32.const 0)
  (func $_ZN5equix8solution15check_tree_sums17h7bac851a97a6d1d3E (type 11) (param i32 i32 i32 i32 i32)
    (local i32 i32 i32 i64 i64)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 5
    global.set $__stack_pointer
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 3
            i32.const 2
            i32.eq
            br_if 0 (;@4;)
            local.get 5
            local.get 1
            local.get 2
            local.get 3
            i32.const 1
            i32.shr_u
            local.tee 6
            local.get 4
            i32.const 1
            i32.shr_u
            local.tee 7
            call $_ZN5equix8solution15check_tree_sums17h7bac851a97a6d1d3E
            local.get 5
            i32.load
            i32.eqz
            br_if 1 (;@3;)
            i64.const 1
            local.set 8
            br 3 (;@1;)
          end
          local.get 1
          local.get 2
          i64.load16_u
          call $_ZN5hashx5HashX11hash_to_u6417h4c9db866b9cd67d7E
          local.get 1
          local.get 2
          i64.load16_u offset=2
          call $_ZN5hashx5HashX11hash_to_u6417h4c9db866b9cd67d7E
          i64.add
          local.set 9
          br 1 (;@2;)
        end
        local.get 5
        i64.load offset=8
        local.set 9
        local.get 5
        local.get 1
        local.get 2
        local.get 6
        i32.const 1
        i32.shl
        i32.add
        local.get 3
        local.get 6
        i32.sub
        local.get 7
        call $_ZN5equix8solution15check_tree_sums17h7bac851a97a6d1d3E
        i64.const 1
        local.set 8
        local.get 5
        i64.load
        i64.const 1
        i64.eq
        br_if 1 (;@1;)
        local.get 5
        i64.load offset=8
        local.get 9
        i64.add
        local.set 9
      end
      i64.const 1
      local.set 8
      local.get 9
      i64.const -1
      local.get 4
      i64.extend_i32_u
      i64.shl
      i64.const -1
      i64.xor
      i64.and
      i64.eqz
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      local.get 9
      i64.store offset=8
      i64.const 0
      local.set 8
    end
    local.get 0
    local.get 8
    i64.store
    local.get 5
    i32.const 16
    i32.add
    global.set $__stack_pointer)
  (func $_ZN5equix12EquiXBuilder12verify_bytes17he8cd41b059ffe8d4E (type 11) (param i32 i32 i32 i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 112
    i32.sub
    local.tee 5
    global.set $__stack_pointer
    local.get 5
    local.get 4
    i64.load offset=8 align=1
    i64.store offset=72
    local.get 5
    local.get 4
    i64.load align=1
    i64.store offset=64
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          local.get 5
          i32.const 64
          i32.add
          i32.const 8
          call $_ZN5equix8solution16check_tree_order17h1c76aff39c15ed47E
          br_if 0 (;@3;)
          i32.const 2
          local.set 4
          br 1 (;@2;)
        end
        local.get 5
        i32.const 8
        i32.add
        i32.const 14
        i32.add
        local.get 4
        i32.const 14
        i32.add
        i64.load16_u align=1
        i64.store16
        local.get 5
        local.get 4
        i64.load32_u offset=10 align=1
        i64.store32 offset=18 align=2
        local.get 5
        local.get 4
        i64.load offset=2 align=1
        i64.store offset=10 align=2
        local.get 5
        local.get 4
        i32.load16_u align=1
        i32.store16 offset=8
        local.get 5
        i32.const 64
        i32.add
        local.get 1
        local.get 2
        local.get 3
        call $_ZN5hashx12HashXBuilder5build17h2fb587a851cf5b39E
        block  ;; label = @3
          local.get 5
          i32.load offset=64
          i32.const 1
          i32.ne
          br_if 0 (;@3;)
          local.get 5
          i32.load offset=72
          local.set 3
          local.get 5
          i32.load offset=68
          local.set 4
          br 2 (;@1;)
        end
        local.get 5
        local.get 5
        i64.load offset=76 align=4
        i64.store offset=28 align=4
        local.get 5
        local.get 5
        i64.load offset=84 align=4
        i64.store offset=36 align=4
        local.get 5
        local.get 5
        i64.load offset=92 align=4
        i64.store offset=44 align=4
        local.get 5
        local.get 5
        i64.load offset=100 align=4
        i64.store offset=52 align=4
        local.get 5
        local.get 5
        i32.load offset=108
        i32.store offset=60
        local.get 5
        local.get 5
        i32.load offset=72
        i32.store offset=24
        local.get 5
        i32.const 64
        i32.add
        local.get 5
        i32.const 24
        i32.add
        local.get 5
        i32.const 8
        i32.add
        i32.const 8
        i32.const 60
        call $_ZN5equix8solution15check_tree_sums17h7bac851a97a6d1d3E
        i32.const 3
        i32.const 4
        local.get 5
        i32.load offset=64
        select
        local.set 4
        local.get 5
        i32.load offset=56
        local.tee 3
        i32.eqz
        br_if 0 (;@2;)
        local.get 3
        i32.const 4096
        i32.const 4
        call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
      end
    end
    local.get 0
    local.get 3
    i32.store offset=4
    local.get 0
    local.get 4
    i32.store
    local.get 5
    i32.const 112
    i32.add
    global.set $__stack_pointer)
  (func $_ZN5equix12EquiXBuilder5build17h3045b0a3930df7a1E (type 12) (param i32 i32 i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 48
    i32.sub
    local.tee 4
    global.set $__stack_pointer
    local.get 4
    local.get 1
    local.get 2
    local.get 3
    call $_ZN5hashx12HashXBuilder5build17h2fb587a851cf5b39E
    i32.const 1
    local.set 3
    block  ;; label = @1
      block  ;; label = @2
        local.get 4
        i32.load
        i32.const 1
        i32.ne
        br_if 0 (;@2;)
        local.get 0
        local.get 4
        i64.load offset=4 align=4
        i64.store offset=4 align=4
        br 1 (;@1;)
      end
      local.get 0
      local.get 4
      i64.load offset=40
      i64.store offset=40
      local.get 0
      local.get 4
      i64.load offset=32
      i64.store offset=32
      local.get 0
      local.get 4
      i64.load offset=24
      i64.store offset=24
      local.get 0
      local.get 4
      i64.load offset=16
      i64.store offset=16
      local.get 0
      local.get 4
      i64.load offset=8
      i64.store offset=8
      i32.const 0
      local.set 3
    end
    local.get 0
    local.get 3
    i32.store
    local.get 4
    i32.const 48
    i32.add
    global.set $__stack_pointer)
  (func $_ZN5equix5EquiX5solve17hae13e53cffa3fde3E (type 1) (param i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 144
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    call $_RNvCsfLfy6EI15iL_7___rustc35___rust_no_alloc_shim_is_unstable_v2
    local.get 2
    i32.const 1895424
    i32.const 8
    call $_RNvCsfLfy6EI15iL_7___rustc12___rust_alloc
    i32.store offset=8
    local.get 2
    i32.const 0
    i32.store offset=12
    local.get 1
    local.get 2
    i32.const 8
    i32.add
    local.get 2
    i32.const 12
    i32.add
    call $_ZN5equix6solver14find_solutions17hbf43fc419e357594E
    local.get 0
    local.get 2
    i32.const 12
    i32.add
    i32.const 132
    memory.copy
    local.get 2
    i32.load offset=8
    i32.const 1895424
    i32.const 8
    call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
    local.get 2
    i32.const 144
    i32.add
    global.set $__stack_pointer)
  (func $_ZN5equix8solution20sort_into_tree_order17h39428a4012a0a9dcE.llvm.2140353953980798599 (type 1) (param i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i64)
    local.get 1
    local.get 1
    i32.const 1
    i32.shr_u
    local.tee 2
    i32.sub
    local.set 3
    local.get 0
    local.get 1
    i32.const -2
    i32.and
    i32.add
    local.set 4
    block  ;; label = @1
      local.get 1
      i32.const 2
      i32.le_u
      br_if 0 (;@1;)
      local.get 0
      local.get 2
      call $_ZN5equix8solution20sort_into_tree_order17h39428a4012a0a9dcE.llvm.2140353953980798599
      local.get 4
      local.get 3
      call $_ZN5equix8solution20sort_into_tree_order17h39428a4012a0a9dcE.llvm.2140353953980798599
    end
    local.get 1
    i32.const 1
    i32.shl
    local.set 5
    local.get 0
    i32.const -2
    i32.add
    local.set 6
    i32.const 0
    local.get 2
    i32.const 1
    i32.shl
    local.tee 7
    i32.sub
    local.set 8
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          loop  ;; label = @4
            local.get 7
            i32.eqz
            br_if 2 (;@2;)
            local.get 8
            local.get 5
            i32.add
            i32.eqz
            br_if 1 (;@3;)
            local.get 6
            local.get 7
            i32.add
            local.set 9
            local.get 6
            local.get 5
            i32.add
            local.set 10
            local.get 7
            i32.const -2
            i32.add
            local.set 7
            local.get 5
            i32.const -2
            i32.add
            local.set 5
            local.get 9
            i32.load16_u
            local.tee 9
            i32.const 65535
            i32.and
            local.get 10
            i32.load16_u
            local.tee 10
            i32.eq
            br_if 0 (;@4;)
          end
          local.get 9
          i32.const 65535
          i32.and
          local.get 10
          i32.le_u
          br_if 1 (;@2;)
        end
        local.get 2
        local.get 3
        i32.ne
        br_if 1 (;@1;)
        local.get 2
        i32.eqz
        br_if 0 (;@2;)
        block  ;; label = @3
          local.get 1
          i32.const 2
          i32.shr_u
          local.tee 7
          i32.eqz
          br_if 0 (;@3;)
          local.get 7
          i32.const 3
          i32.and
          local.set 5
          i32.const 0
          local.set 8
          block  ;; label = @4
            local.get 1
            i32.const 16
            i32.lt_u
            br_if 0 (;@4;)
            local.get 1
            i32.const -16
            i32.and
            local.set 10
            local.get 1
            i32.const 2
            i32.shr_u
            i32.const 1073741820
            i32.and
            local.set 8
            i32.const 0
            local.set 7
            loop  ;; label = @5
              local.get 0
              local.get 7
              i32.add
              local.tee 6
              i64.load align=2
              local.set 11
              local.get 6
              local.get 4
              local.get 7
              i32.add
              local.tee 9
              i64.load align=2
              i64.store align=2
              local.get 9
              local.get 11
              i64.store align=2
              local.get 6
              i32.const 8
              i32.add
              local.tee 6
              i64.load align=2
              local.set 11
              local.get 6
              local.get 9
              i32.const 8
              i32.add
              local.tee 9
              i64.load align=2
              i64.store align=2
              local.get 9
              local.get 11
              i64.store align=2
              local.get 10
              local.get 7
              i32.const 16
              i32.add
              local.tee 7
              i32.ne
              br_if 0 (;@5;)
            end
            local.get 5
            i32.eqz
            br_if 1 (;@3;)
          end
          local.get 2
          i32.const 1
          i32.shl
          local.set 10
          local.get 0
          local.get 8
          i32.const 2
          i32.shl
          i32.add
          local.set 7
          loop  ;; label = @4
            local.get 7
            i32.load align=2
            local.set 6
            local.get 7
            local.get 7
            local.get 10
            i32.add
            local.tee 9
            i32.load align=2
            i32.store align=2
            local.get 9
            local.get 6
            i32.store align=2
            local.get 7
            i32.const 4
            i32.add
            local.set 7
            local.get 5
            i32.const -1
            i32.add
            local.tee 5
            br_if 0 (;@4;)
          end
        end
        local.get 1
        i32.const 2
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        local.get 1
        i32.const 1073741820
        i32.and
        local.tee 7
        i32.add
        local.tee 5
        i32.load16_u
        local.set 6
        local.get 5
        local.get 4
        local.get 7
        i32.add
        local.tee 7
        i32.load16_u
        i32.store16
        local.get 7
        local.get 6
        i32.store16
      end
      return
    end
    i32.const 1049672
    i32.const 105
    i32.const 1049724
    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking9panic_fmt
    unreachable)
  (func $_ZN5equix8solution8Solution8to_bytes17hc2695acc66895ba7E (type 1) (param i32 i32)
    local.get 0
    local.get 1
    i64.load offset=8 align=1
    i64.store offset=8 align=1
    local.get 0
    local.get 1
    i64.load align=1
    i64.store align=1)
  (func $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E (type 13) (param i32 i32 i32 i32 i32 i32 i32 i32)
    (local i32 i32 i32 i64 i32 i32 i32 i32 i64 i64 i64 i64 i64)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.set 8
    block  ;; label = @1
      block  ;; label = @2
        local.get 2
        i32.const 255
        i32.and
        local.tee 9
        i32.const 6
        i32.gt_u
        br_if 0 (;@2;)
        i32.const 1
        local.get 9
        i32.shl
        i32.const 105
        i32.and
        br_if 1 (;@1;)
      end
      i32.const 0
      local.set 5
    end
    i32.const 0
    local.set 9
    local.get 8
    i32.const 0
    i32.store offset=4
    local.get 1
    i32.load8_u offset=92
    local.get 7
    i32.load8_u offset=2
    local.tee 7
    i32.gt_u
    local.get 5
    local.get 6
    i32.const 255
    i32.and
    i32.eqz
    i32.and
    i32.or
    local.set 10
    local.get 4
    i64.load align=4
    local.tee 11
    i32.wrap_i64
    local.set 4
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          local.get 3
          br_if 0 (;@3;)
          local.get 4
          i32.const 255
          i32.and
          i32.const 1
          i32.ne
          br_if 0 (;@3;)
          i32.const 0
          local.set 9
          block  ;; label = @4
            local.get 10
            local.get 1
            i32.load8_u offset=24
            i32.const 255
            i32.and
            i32.const 1
            i32.eq
            i32.or
            br_if 0 (;@4;)
            i32.const 1
            local.set 9
            local.get 8
            i32.const 1
            i32.store offset=4
            local.get 8
            i32.const 0
            i32.store8 offset=8
          end
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=93
            local.get 7
            i32.gt_u
            br_if 0 (;@4;)
            local.get 5
            local.get 6
            i32.const 255
            i32.and
            i32.const 1
            i32.eq
            i32.and
            br_if 0 (;@4;)
            local.get 1
            i32.load8_u offset=32
            i32.const 255
            i32.and
            i32.const 1
            i32.eq
            br_if 0 (;@4;)
            local.get 8
            i32.const 4
            i32.add
            local.get 9
            i32.or
            i32.const 1
            i32.store8 offset=4
            local.get 8
            local.get 9
            i32.const 1
            i32.add
            local.tee 9
            i32.store offset=4
          end
          local.get 6
          i32.const 255
          i32.and
          local.set 4
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=94
            local.get 7
            i32.gt_u
            br_if 0 (;@4;)
            local.get 5
            local.get 4
            i32.const 2
            i32.eq
            i32.and
            br_if 0 (;@4;)
            local.get 1
            i32.load8_u offset=40
            i32.const 255
            i32.and
            i32.const 1
            i32.eq
            br_if 0 (;@4;)
            local.get 8
            i32.const 4
            i32.add
            local.get 9
            i32.or
            i32.const 2
            i32.store8 offset=4
            local.get 8
            local.get 9
            i32.const 1
            i32.add
            local.tee 9
            i32.store offset=4
          end
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=95
            local.get 7
            i32.gt_u
            br_if 0 (;@4;)
            local.get 5
            local.get 4
            i32.const 3
            i32.eq
            i32.and
            br_if 0 (;@4;)
            local.get 1
            i32.load8_u offset=48
            i32.const 255
            i32.and
            i32.const 1
            i32.eq
            br_if 0 (;@4;)
            local.get 8
            i32.const 4
            i32.add
            local.get 9
            i32.add
            i32.const 3
            i32.store8 offset=4
            local.get 8
            local.get 9
            i32.const 1
            i32.add
            local.tee 9
            i32.store offset=4
          end
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=96
            local.get 7
            i32.gt_u
            br_if 0 (;@4;)
            local.get 5
            local.get 6
            i32.const 255
            i32.and
            i32.const 4
            i32.eq
            i32.and
            br_if 0 (;@4;)
            local.get 1
            i32.load8_u offset=56
            i32.const 255
            i32.and
            i32.const 1
            i32.eq
            br_if 0 (;@4;)
            local.get 8
            i32.const 4
            i32.add
            local.get 9
            i32.add
            i32.const 4
            i32.store8 offset=4
            local.get 8
            local.get 9
            i32.const 1
            i32.add
            local.tee 9
            i32.store offset=4
          end
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=97
            local.get 7
            i32.gt_u
            br_if 0 (;@4;)
            local.get 2
            i32.const 255
            i32.and
            i32.const 3
            i32.eq
            br_if 0 (;@4;)
            local.get 5
            local.get 6
            i32.const 255
            i32.and
            i32.const 5
            i32.eq
            i32.and
            br_if 0 (;@4;)
            local.get 1
            i32.load8_u offset=64
            i32.const 255
            i32.and
            i32.const 1
            i32.eq
            br_if 0 (;@4;)
            local.get 8
            i32.const 4
            i32.add
            local.get 9
            i32.add
            i32.const 5
            i32.store8 offset=4
            local.get 8
            local.get 9
            i32.const 1
            i32.add
            local.tee 9
            i32.store offset=4
          end
          local.get 6
          i32.const 255
          i32.and
          local.set 6
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=98
            local.get 7
            i32.gt_u
            br_if 0 (;@4;)
            local.get 5
            local.get 6
            i32.const 6
            i32.eq
            i32.and
            br_if 0 (;@4;)
            local.get 1
            i32.load8_u offset=72
            i32.const 255
            i32.and
            i32.const 1
            i32.eq
            br_if 0 (;@4;)
            local.get 8
            i32.const 4
            i32.add
            local.get 9
            i32.add
            i32.const 6
            i32.store8 offset=4
            local.get 8
            local.get 9
            i32.const 1
            i32.add
            local.tee 9
            i32.store offset=4
          end
          local.get 1
          i32.load8_u offset=99
          local.get 7
          i32.gt_u
          br_if 2 (;@1;)
          local.get 5
          local.get 6
          i32.const 7
          i32.eq
          i32.and
          br_if 2 (;@1;)
          local.get 1
          i32.load8_u offset=80
          i32.const 255
          i32.and
          i32.const 1
          i32.ne
          br_if 1 (;@2;)
          br 2 (;@1;)
        end
        local.get 11
        i64.const 32
        i64.shr_u
        i32.wrap_i64
        local.set 12
        local.get 11
        i64.const 8
        i64.shr_u
        i32.wrap_i64
        local.set 13
        block  ;; label = @3
          local.get 10
          br_if 0 (;@3;)
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=24
            local.get 4
            i32.const 255
            i32.and
            local.tee 3
            i32.ne
            br_if 0 (;@4;)
            local.get 1
            i32.load offset=28
            local.set 14
            local.get 1
            i32.load8_u offset=25
            local.set 10
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 3
                      i32.const -1
                      i32.add
                      br_table 4 (;@5;) 3 (;@6;) 2 (;@7;) 1 (;@8;) 6 (;@3;) 0 (;@9;) 6 (;@3;)
                    end
                    local.get 10
                    i32.const 255
                    i32.and
                    local.get 13
                    i32.const 255
                    i32.and
                    i32.ne
                    br_if 4 (;@4;)
                    br 5 (;@3;)
                  end
                  local.get 10
                  i32.const 255
                  i32.and
                  local.get 13
                  i32.const 255
                  i32.and
                  i32.ne
                  br_if 3 (;@4;)
                  br 4 (;@3;)
                end
                local.get 14
                local.get 12
                i32.ne
                br_if 2 (;@4;)
                br 3 (;@3;)
              end
              local.get 14
              local.get 12
              i32.ne
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            local.get 10
            i32.const 255
            i32.and
            local.get 13
            i32.const 255
            i32.and
            i32.eq
            br_if 1 (;@3;)
          end
          i32.const 1
          local.set 9
          local.get 8
          i32.const 1
          i32.store offset=4
          local.get 8
          i32.const 0
          i32.store8 offset=8
        end
        local.get 6
        i32.const 255
        i32.and
        local.set 3
        block  ;; label = @3
          local.get 1
          i32.load8_u offset=93
          local.get 7
          i32.gt_u
          br_if 0 (;@3;)
          local.get 5
          local.get 3
          i32.const 1
          i32.eq
          i32.and
          br_if 0 (;@3;)
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=32
            local.get 4
            i32.const 255
            i32.and
            local.tee 10
            i32.ne
            br_if 0 (;@4;)
            local.get 1
            i32.load offset=36
            local.set 15
            local.get 1
            i32.load8_u offset=33
            local.set 14
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 10
                      i32.const -1
                      i32.add
                      br_table 4 (;@5;) 3 (;@6;) 2 (;@7;) 1 (;@8;) 6 (;@3;) 0 (;@9;) 6 (;@3;)
                    end
                    local.get 14
                    i32.const 255
                    i32.and
                    local.get 13
                    i32.const 255
                    i32.and
                    i32.ne
                    br_if 4 (;@4;)
                    br 5 (;@3;)
                  end
                  local.get 14
                  i32.const 255
                  i32.and
                  local.get 13
                  i32.const 255
                  i32.and
                  i32.ne
                  br_if 3 (;@4;)
                  br 4 (;@3;)
                end
                local.get 15
                local.get 12
                i32.ne
                br_if 2 (;@4;)
                br 3 (;@3;)
              end
              local.get 15
              local.get 12
              i32.ne
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            local.get 14
            i32.const 255
            i32.and
            local.get 13
            i32.const 255
            i32.and
            i32.eq
            br_if 1 (;@3;)
          end
          local.get 8
          i32.const 4
          i32.add
          local.get 9
          i32.or
          i32.const 1
          i32.store8 offset=4
          local.get 8
          local.get 9
          i32.const 1
          i32.add
          local.tee 9
          i32.store offset=4
        end
        block  ;; label = @3
          local.get 1
          i32.load8_u offset=94
          local.get 7
          i32.gt_u
          br_if 0 (;@3;)
          local.get 5
          local.get 3
          i32.const 2
          i32.eq
          i32.and
          br_if 0 (;@3;)
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=40
            local.get 4
            i32.const 255
            i32.and
            local.tee 3
            i32.ne
            br_if 0 (;@4;)
            local.get 1
            i32.load offset=44
            local.set 14
            local.get 1
            i32.load8_u offset=41
            local.set 10
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 3
                      i32.const -1
                      i32.add
                      br_table 4 (;@5;) 3 (;@6;) 2 (;@7;) 1 (;@8;) 6 (;@3;) 0 (;@9;) 6 (;@3;)
                    end
                    local.get 10
                    i32.const 255
                    i32.and
                    local.get 13
                    i32.const 255
                    i32.and
                    i32.ne
                    br_if 4 (;@4;)
                    br 5 (;@3;)
                  end
                  local.get 10
                  i32.const 255
                  i32.and
                  local.get 13
                  i32.const 255
                  i32.and
                  i32.ne
                  br_if 3 (;@4;)
                  br 4 (;@3;)
                end
                local.get 14
                local.get 12
                i32.ne
                br_if 2 (;@4;)
                br 3 (;@3;)
              end
              local.get 14
              local.get 12
              i32.ne
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            local.get 10
            i32.const 255
            i32.and
            local.get 13
            i32.const 255
            i32.and
            i32.eq
            br_if 1 (;@3;)
          end
          local.get 8
          i32.const 4
          i32.add
          local.get 9
          i32.or
          i32.const 2
          i32.store8 offset=4
          local.get 8
          local.get 9
          i32.const 1
          i32.add
          local.tee 9
          i32.store offset=4
        end
        local.get 6
        i32.const 255
        i32.and
        local.set 3
        block  ;; label = @3
          local.get 1
          i32.load8_u offset=95
          local.get 7
          i32.gt_u
          br_if 0 (;@3;)
          local.get 5
          local.get 3
          i32.const 3
          i32.eq
          i32.and
          br_if 0 (;@3;)
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=48
            local.get 4
            i32.const 255
            i32.and
            local.tee 10
            i32.ne
            br_if 0 (;@4;)
            local.get 1
            i32.load offset=52
            local.set 15
            local.get 1
            i32.load8_u offset=49
            local.set 14
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 10
                      i32.const -1
                      i32.add
                      br_table 4 (;@5;) 3 (;@6;) 2 (;@7;) 1 (;@8;) 6 (;@3;) 0 (;@9;) 6 (;@3;)
                    end
                    local.get 14
                    i32.const 255
                    i32.and
                    local.get 13
                    i32.const 255
                    i32.and
                    i32.ne
                    br_if 4 (;@4;)
                    br 5 (;@3;)
                  end
                  local.get 14
                  i32.const 255
                  i32.and
                  local.get 13
                  i32.const 255
                  i32.and
                  i32.ne
                  br_if 3 (;@4;)
                  br 4 (;@3;)
                end
                local.get 15
                local.get 12
                i32.ne
                br_if 2 (;@4;)
                br 3 (;@3;)
              end
              local.get 15
              local.get 12
              i32.ne
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            local.get 14
            i32.const 255
            i32.and
            local.get 13
            i32.const 255
            i32.and
            i32.eq
            br_if 1 (;@3;)
          end
          local.get 8
          i32.const 4
          i32.add
          local.get 9
          i32.add
          i32.const 3
          i32.store8 offset=4
          local.get 8
          local.get 9
          i32.const 1
          i32.add
          local.tee 9
          i32.store offset=4
        end
        block  ;; label = @3
          local.get 1
          i32.load8_u offset=96
          local.get 7
          i32.gt_u
          br_if 0 (;@3;)
          local.get 5
          local.get 3
          i32.const 4
          i32.eq
          i32.and
          br_if 0 (;@3;)
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=56
            local.get 4
            i32.const 255
            i32.and
            local.tee 3
            i32.ne
            br_if 0 (;@4;)
            local.get 1
            i32.load offset=60
            local.set 14
            local.get 1
            i32.load8_u offset=57
            local.set 10
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 3
                      i32.const -1
                      i32.add
                      br_table 4 (;@5;) 3 (;@6;) 2 (;@7;) 1 (;@8;) 6 (;@3;) 0 (;@9;) 6 (;@3;)
                    end
                    local.get 10
                    i32.const 255
                    i32.and
                    local.get 13
                    i32.const 255
                    i32.and
                    i32.ne
                    br_if 4 (;@4;)
                    br 5 (;@3;)
                  end
                  local.get 10
                  i32.const 255
                  i32.and
                  local.get 13
                  i32.const 255
                  i32.and
                  i32.ne
                  br_if 3 (;@4;)
                  br 4 (;@3;)
                end
                local.get 14
                local.get 12
                i32.ne
                br_if 2 (;@4;)
                br 3 (;@3;)
              end
              local.get 14
              local.get 12
              i32.ne
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            local.get 10
            i32.const 255
            i32.and
            local.get 13
            i32.const 255
            i32.and
            i32.eq
            br_if 1 (;@3;)
          end
          local.get 8
          i32.const 4
          i32.add
          local.get 9
          i32.add
          i32.const 4
          i32.store8 offset=4
          local.get 8
          local.get 9
          i32.const 1
          i32.add
          local.tee 9
          i32.store offset=4
        end
        block  ;; label = @3
          local.get 1
          i32.load8_u offset=97
          local.get 7
          i32.gt_u
          br_if 0 (;@3;)
          local.get 2
          i32.const 255
          i32.and
          i32.const 3
          i32.eq
          br_if 0 (;@3;)
          local.get 5
          local.get 6
          i32.const 255
          i32.and
          i32.const 5
          i32.eq
          i32.and
          br_if 0 (;@3;)
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=64
            local.get 4
            i32.const 255
            i32.and
            local.tee 2
            i32.ne
            br_if 0 (;@4;)
            local.get 1
            i32.load offset=68
            local.set 10
            local.get 1
            i32.load8_u offset=65
            local.set 3
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 2
                      i32.const -1
                      i32.add
                      br_table 4 (;@5;) 3 (;@6;) 2 (;@7;) 1 (;@8;) 6 (;@3;) 0 (;@9;) 6 (;@3;)
                    end
                    local.get 3
                    i32.const 255
                    i32.and
                    local.get 13
                    i32.const 255
                    i32.and
                    i32.ne
                    br_if 4 (;@4;)
                    br 5 (;@3;)
                  end
                  local.get 3
                  i32.const 255
                  i32.and
                  local.get 13
                  i32.const 255
                  i32.and
                  i32.ne
                  br_if 3 (;@4;)
                  br 4 (;@3;)
                end
                local.get 10
                local.get 12
                i32.ne
                br_if 2 (;@4;)
                br 3 (;@3;)
              end
              local.get 10
              local.get 12
              i32.ne
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            local.get 3
            i32.const 255
            i32.and
            local.get 13
            i32.const 255
            i32.and
            i32.eq
            br_if 1 (;@3;)
          end
          local.get 8
          i32.const 4
          i32.add
          local.get 9
          i32.add
          i32.const 5
          i32.store8 offset=4
          local.get 8
          local.get 9
          i32.const 1
          i32.add
          local.tee 9
          i32.store offset=4
        end
        local.get 6
        i32.const 255
        i32.and
        local.set 6
        block  ;; label = @3
          local.get 1
          i32.load8_u offset=98
          local.get 7
          i32.gt_u
          br_if 0 (;@3;)
          local.get 5
          local.get 6
          i32.const 6
          i32.eq
          i32.and
          br_if 0 (;@3;)
          block  ;; label = @4
            local.get 1
            i32.load8_u offset=72
            local.get 4
            i32.const 255
            i32.and
            local.tee 2
            i32.ne
            br_if 0 (;@4;)
            local.get 1
            i32.load offset=76
            local.set 10
            local.get 1
            i32.load8_u offset=73
            local.set 3
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 2
                      i32.const -1
                      i32.add
                      br_table 4 (;@5;) 3 (;@6;) 2 (;@7;) 1 (;@8;) 6 (;@3;) 0 (;@9;) 6 (;@3;)
                    end
                    local.get 3
                    i32.const 255
                    i32.and
                    local.get 13
                    i32.const 255
                    i32.and
                    i32.ne
                    br_if 4 (;@4;)
                    br 5 (;@3;)
                  end
                  local.get 3
                  i32.const 255
                  i32.and
                  local.get 13
                  i32.const 255
                  i32.and
                  i32.ne
                  br_if 3 (;@4;)
                  br 4 (;@3;)
                end
                local.get 10
                local.get 12
                i32.ne
                br_if 2 (;@4;)
                br 3 (;@3;)
              end
              local.get 10
              local.get 12
              i32.ne
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            local.get 3
            i32.const 255
            i32.and
            local.get 13
            i32.const 255
            i32.and
            i32.eq
            br_if 1 (;@3;)
          end
          local.get 8
          i32.const 4
          i32.add
          local.get 9
          i32.add
          i32.const 6
          i32.store8 offset=4
          local.get 8
          local.get 9
          i32.const 1
          i32.add
          local.tee 9
          i32.store offset=4
        end
        local.get 1
        i32.load8_u offset=99
        local.get 7
        i32.gt_u
        br_if 1 (;@1;)
        local.get 5
        local.get 6
        i32.const 7
        i32.eq
        i32.and
        br_if 1 (;@1;)
        local.get 1
        i32.load8_u offset=80
        local.get 4
        i32.const 255
        i32.and
        local.tee 7
        i32.ne
        br_if 0 (;@2;)
        local.get 1
        i32.load offset=84
        local.set 6
        local.get 1
        i32.load8_u offset=81
        local.set 5
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  local.get 7
                  i32.const -1
                  i32.add
                  br_table 4 (;@3;) 3 (;@4;) 2 (;@5;) 1 (;@6;) 6 (;@1;) 0 (;@7;) 6 (;@1;)
                end
                local.get 5
                i32.const 255
                i32.and
                local.get 13
                i32.const 255
                i32.and
                i32.ne
                br_if 4 (;@2;)
                br 5 (;@1;)
              end
              local.get 5
              i32.const 255
              i32.and
              local.get 13
              i32.const 255
              i32.and
              i32.ne
              br_if 3 (;@2;)
              br 4 (;@1;)
            end
            local.get 6
            local.get 12
            i32.ne
            br_if 2 (;@2;)
            br 3 (;@1;)
          end
          local.get 6
          local.get 12
          i32.ne
          br_if 1 (;@2;)
          br 2 (;@1;)
        end
        local.get 5
        i32.const 255
        i32.and
        local.get 13
        i32.const 255
        i32.and
        i32.eq
        br_if 1 (;@1;)
      end
      local.get 8
      i32.const 4
      i32.add
      local.get 9
      i32.add
      i32.const 7
      i32.store8 offset=4
      local.get 8
      local.get 9
      i32.const 1
      i32.add
      i32.store offset=4
    end
    i32.const 1
    local.set 9
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 8
            i32.load offset=4
            local.tee 5
            br_table 3 (;@1;) 0 (;@4;) 1 (;@3;)
          end
          local.get 8
          i32.load8_u offset=8
          local.set 7
          br 1 (;@2;)
        end
        block  ;; label = @3
          block  ;; label = @4
            local.get 1
            i32.load
            br_if 0 (;@4;)
            local.get 1
            i32.load offset=8
            local.tee 9
            local.get 9
            i64.load offset=32
            local.tee 11
            i64.const 1
            i64.add
            i64.store offset=32
            local.get 1
            local.get 11
            local.get 9
            i64.load offset=24
            i64.xor
            local.tee 16
            i64.const 16
            i64.rotl
            local.get 16
            local.get 9
            i64.load offset=16
            i64.add
            local.tee 16
            i64.xor
            local.tee 17
            i64.const 21
            i64.rotl
            local.get 17
            local.get 9
            i64.load offset=8
            local.tee 18
            local.get 9
            i64.load
            i64.add
            local.tee 19
            i64.const 32
            i64.rotl
            i64.add
            local.tee 17
            i64.xor
            local.tee 20
            i64.const 16
            i64.rotl
            local.get 16
            local.get 18
            i64.const 13
            i64.rotl
            local.get 19
            i64.xor
            local.tee 18
            i64.add
            local.tee 16
            i64.const 32
            i64.rotl
            i64.const 255
            i64.xor
            local.get 20
            i64.add
            local.tee 19
            i64.xor
            local.tee 20
            i64.const 21
            i64.rotl
            local.get 17
            local.get 11
            i64.xor
            local.get 16
            local.get 18
            i64.const 17
            i64.rotl
            i64.xor
            local.tee 11
            i64.add
            local.tee 16
            i64.const 32
            i64.rotl
            local.get 20
            i64.add
            local.tee 17
            i64.xor
            local.tee 18
            i64.const 16
            i64.rotl
            local.get 16
            local.get 11
            i64.const 13
            i64.rotl
            i64.xor
            local.tee 11
            local.get 19
            i64.add
            local.tee 16
            i64.const 32
            i64.rotl
            local.get 18
            i64.add
            local.tee 18
            i64.xor
            local.tee 19
            i64.const 21
            i64.rotl
            local.get 11
            i64.const 17
            i64.rotl
            local.get 16
            i64.xor
            local.tee 11
            local.get 17
            i64.add
            local.tee 16
            i64.const 32
            i64.rotl
            local.get 19
            i64.add
            local.tee 17
            i64.xor
            local.tee 19
            i64.const 16
            i64.rotl
            local.get 11
            i64.const 13
            i64.rotl
            local.get 16
            i64.xor
            local.tee 11
            local.get 18
            i64.add
            local.tee 16
            i64.const 32
            i64.rotl
            local.get 19
            i64.add
            local.tee 18
            i64.xor
            i64.const 21
            i64.rotl
            local.get 11
            i64.const 17
            i64.rotl
            local.get 16
            i64.xor
            local.tee 11
            i64.const 13
            i64.rotl
            local.get 11
            local.get 17
            i64.add
            i64.xor
            local.tee 11
            i64.const 17
            i64.rotl
            i64.xor
            local.get 11
            local.get 18
            i64.add
            local.tee 11
            i64.const 32
            i64.rotl
            i64.xor
            local.get 11
            i64.xor
            local.tee 11
            i64.store32 offset=4
            local.get 11
            i64.const 32
            i64.shr_u
            i32.wrap_i64
            local.set 9
            i32.const 1
            local.set 7
            br 1 (;@3;)
          end
          local.get 1
          i32.load offset=4
          local.set 9
          i32.const 0
          local.set 7
        end
        local.get 1
        local.get 7
        i32.store
        local.get 8
        i32.const 4
        i32.add
        local.get 9
        local.get 5
        i32.rem_u
        i32.add
        i32.load8_u offset=4
        local.set 7
      end
      i32.const 0
      local.set 9
    end
    local.get 0
    local.get 7
    i32.store8 offset=1
    local.get 0
    local.get 9
    i32.const 1
    i32.and
    i32.store8)
  (func $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE (type 12) (param i32 i32 i32 i32)
    (local i32 i32 i32 i64 i64 i64 i64 i64 i64)
    i32.const 0
    local.set 4
    global.get $__stack_pointer
    i32.const 32
    i32.sub
    local.tee 5
    i32.const 0
    i32.store offset=20
    block  ;; label = @1
      local.get 1
      i32.load8_u offset=92
      local.get 3
      i32.load8_u offset=2
      local.tee 3
      i32.gt_u
      br_if 0 (;@1;)
      i32.const 1
      local.set 4
      local.get 5
      i32.const 1
      i32.store offset=20
      local.get 5
      i32.const 0
      i32.store8 offset=24
    end
    block  ;; label = @1
      local.get 1
      i32.load8_u offset=93
      local.get 3
      i32.gt_u
      br_if 0 (;@1;)
      local.get 5
      i32.const 20
      i32.add
      local.get 4
      i32.or
      i32.const 1
      i32.store8 offset=4
      local.get 5
      local.get 4
      i32.const 1
      i32.add
      local.tee 4
      i32.store offset=20
    end
    block  ;; label = @1
      local.get 1
      i32.load8_u offset=94
      local.get 3
      i32.gt_u
      br_if 0 (;@1;)
      local.get 5
      i32.const 20
      i32.add
      local.get 4
      i32.or
      i32.const 2
      i32.store8 offset=4
      local.get 5
      local.get 4
      i32.const 1
      i32.add
      local.tee 4
      i32.store offset=20
    end
    block  ;; label = @1
      local.get 1
      i32.load8_u offset=95
      local.get 3
      i32.gt_u
      br_if 0 (;@1;)
      local.get 5
      i32.const 20
      i32.add
      local.get 4
      i32.add
      i32.const 3
      i32.store8 offset=4
      local.get 5
      local.get 4
      i32.const 1
      i32.add
      local.tee 4
      i32.store offset=20
    end
    block  ;; label = @1
      local.get 1
      i32.load8_u offset=96
      local.get 3
      i32.gt_u
      br_if 0 (;@1;)
      local.get 5
      i32.const 20
      i32.add
      local.get 4
      i32.add
      i32.const 4
      i32.store8 offset=4
      local.get 5
      local.get 4
      i32.const 1
      i32.add
      local.tee 4
      i32.store offset=20
    end
    block  ;; label = @1
      local.get 1
      i32.load8_u offset=97
      local.get 3
      i32.gt_u
      br_if 0 (;@1;)
      local.get 5
      i32.const 20
      i32.add
      local.get 4
      i32.add
      i32.const 5
      i32.store8 offset=4
      local.get 5
      local.get 4
      i32.const 1
      i32.add
      local.tee 4
      i32.store offset=20
    end
    block  ;; label = @1
      local.get 1
      i32.load8_u offset=98
      local.get 3
      i32.gt_u
      br_if 0 (;@1;)
      local.get 5
      i32.const 20
      i32.add
      local.get 4
      i32.add
      i32.const 6
      i32.store8 offset=4
      local.get 5
      local.get 4
      i32.const 1
      i32.add
      local.tee 4
      i32.store offset=20
    end
    block  ;; label = @1
      local.get 1
      i32.load8_u offset=99
      local.get 3
      i32.gt_u
      br_if 0 (;@1;)
      local.get 5
      i32.const 20
      i32.add
      local.get 4
      i32.add
      i32.const 7
      i32.store8 offset=4
      local.get 5
      local.get 4
      i32.const 1
      i32.add
      i32.store offset=20
    end
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 2
            i32.const 255
            i32.and
            i32.const 3
            i32.ne
            br_if 0 (;@4;)
            local.get 5
            i32.const 24
            i32.add
            local.set 6
            i32.const 0
            local.set 4
            local.get 5
            i32.load offset=20
            local.set 2
            loop  ;; label = @5
              local.get 2
              local.get 4
              i32.eq
              br_if 1 (;@4;)
              local.get 6
              local.get 4
              i32.add
              local.set 3
              local.get 4
              i32.const 1
              i32.add
              local.set 4
              local.get 3
              i32.load8_u
              i32.const 5
              i32.ne
              br_if 0 (;@5;)
            end
            local.get 2
            i32.const 2
            i32.ne
            br_if 0 (;@4;)
            local.get 5
            i32.const 5
            i32.store8 offset=12
            br 1 (;@3;)
          end
          local.get 5
          local.get 5
          i32.load offset=28
          i32.store offset=16
          local.get 5
          local.get 5
          i64.load offset=20 align=4
          local.tee 7
          i64.store offset=8
          i32.const 1
          local.set 3
          block  ;; label = @4
            local.get 7
            i32.wrap_i64
            local.tee 2
            br_table 3 (;@1;) 1 (;@3;) 0 (;@4;)
          end
          block  ;; label = @4
            block  ;; label = @5
              local.get 1
              i32.load
              br_if 0 (;@5;)
              local.get 1
              i32.load offset=8
              local.tee 4
              local.get 4
              i64.load offset=32
              local.tee 7
              i64.const 1
              i64.add
              i64.store offset=32
              local.get 1
              local.get 7
              local.get 4
              i64.load offset=24
              i64.xor
              local.tee 8
              i64.const 16
              i64.rotl
              local.get 8
              local.get 4
              i64.load offset=16
              i64.add
              local.tee 8
              i64.xor
              local.tee 9
              i64.const 21
              i64.rotl
              local.get 9
              local.get 4
              i64.load offset=8
              local.tee 10
              local.get 4
              i64.load
              i64.add
              local.tee 11
              i64.const 32
              i64.rotl
              i64.add
              local.tee 9
              i64.xor
              local.tee 12
              i64.const 16
              i64.rotl
              local.get 8
              local.get 10
              i64.const 13
              i64.rotl
              local.get 11
              i64.xor
              local.tee 10
              i64.add
              local.tee 8
              i64.const 32
              i64.rotl
              i64.const 255
              i64.xor
              local.get 12
              i64.add
              local.tee 11
              i64.xor
              local.tee 12
              i64.const 21
              i64.rotl
              local.get 9
              local.get 7
              i64.xor
              local.get 8
              local.get 10
              i64.const 17
              i64.rotl
              i64.xor
              local.tee 7
              i64.add
              local.tee 8
              i64.const 32
              i64.rotl
              local.get 12
              i64.add
              local.tee 9
              i64.xor
              local.tee 10
              i64.const 16
              i64.rotl
              local.get 8
              local.get 7
              i64.const 13
              i64.rotl
              i64.xor
              local.tee 7
              local.get 11
              i64.add
              local.tee 8
              i64.const 32
              i64.rotl
              local.get 10
              i64.add
              local.tee 10
              i64.xor
              local.tee 11
              i64.const 21
              i64.rotl
              local.get 7
              i64.const 17
              i64.rotl
              local.get 8
              i64.xor
              local.tee 7
              local.get 9
              i64.add
              local.tee 8
              i64.const 32
              i64.rotl
              local.get 11
              i64.add
              local.tee 9
              i64.xor
              local.tee 11
              i64.const 16
              i64.rotl
              local.get 7
              i64.const 13
              i64.rotl
              local.get 8
              i64.xor
              local.tee 7
              local.get 10
              i64.add
              local.tee 8
              i64.const 32
              i64.rotl
              local.get 11
              i64.add
              local.tee 10
              i64.xor
              i64.const 21
              i64.rotl
              local.get 7
              i64.const 17
              i64.rotl
              local.get 8
              i64.xor
              local.tee 7
              i64.const 13
              i64.rotl
              local.get 7
              local.get 9
              i64.add
              i64.xor
              local.tee 7
              i64.const 17
              i64.rotl
              i64.xor
              local.get 7
              local.get 10
              i64.add
              local.tee 7
              i64.const 32
              i64.rotl
              i64.xor
              local.get 7
              i64.xor
              local.tee 7
              i64.store32 offset=4
              local.get 7
              i64.const 32
              i64.shr_u
              i32.wrap_i64
              local.set 4
              i32.const 1
              local.set 3
              br 1 (;@4;)
            end
            local.get 1
            i32.load offset=4
            local.set 4
            i32.const 0
            local.set 3
          end
          local.get 1
          local.get 3
          i32.store
          local.get 5
          i32.const 8
          i32.add
          local.get 4
          local.get 2
          i32.rem_u
          i32.add
          i32.load8_u offset=4
          local.set 4
          br 1 (;@2;)
        end
        local.get 5
        i32.load8_u offset=12
        local.set 4
      end
      i32.const 0
      local.set 3
    end
    local.get 0
    local.get 4
    i32.store8 offset=1
    local.get 0
    local.get 3
    i32.const 1
    i32.and
    i32.store8)
  (func $_ZN77_$LT$hashx..compiler..Executable$u20$as$u20$hashx..compiler..Architecture$GT$6invoke17hfb0f54ba72cfdb13E (type 1) (param i32 i32)
    i32.const 1049740
    i32.const 40
    i32.const 1049780
    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
    unreachable)
  (func $_RNvXss_NtCsgXGp5Oqx2Ny_4core3fmtuNtB5_5Debug3fmt (type 3) (param i32 i32) (result i32)
    local.get 1
    i32.const 1049796
    i32.const 2
    call $_RNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB5_9Formatter3pad)
  (func $_ZN5hashx7program7Program8generate17h6731d6e09f18a3ccE (type 1) (param i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i64 i32 i32 i32 i64 i64 i64 i64 i64 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i64 i32 i32 i64 i32 i32 i64 i32 i32 i64 i32 i32 i64 i64 i64 i32 i64 i32 i64 i32 i32 i64 i32 i32 i64 i32 i32 i64 i32 i32 i64)
    global.get $__stack_pointer
    i32.const 448
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    call $_RNvCsfLfy6EI15iL_7___rustc35___rust_no_alloc_shim_is_unstable_v2
    block  ;; label = @1
      i32.const 4096
      i32.const 4
      call $_RNvCsfLfy6EI15iL_7___rustc12___rust_alloc
      local.tee 3
      i32.eqz
      br_if 0 (;@1;)
      local.get 2
      i32.const 336
      i32.add
      local.tee 4
      i32.const 0
      i32.const 84
      memory.fill
      local.get 2
      i64.const 0
      i64.store offset=316 align=4
      local.get 2
      i64.const 0
      i64.store offset=308 align=4
      local.get 2
      i64.const 0
      i64.store offset=300 align=4
      local.get 2
      i64.const 0
      i64.store offset=292 align=4
      local.get 2
      i64.const 0
      i64.store offset=284 align=4
      local.get 2
      i64.const 0
      i64.store offset=276 align=4
      local.get 2
      i64.const 0
      i64.store offset=268 align=4
      local.get 2
      i64.const 0
      i64.store offset=260 align=4
      local.get 2
      i32.const 0
      i32.store8 offset=422
      local.get 2
      i32.const 0
      i32.store16 offset=420
      local.get 2
      i64.const 0
      i64.store offset=328 align=4
      local.get 2
      i32.const 0
      i32.store offset=248
      local.get 2
      local.get 1
      i32.store offset=244
      local.get 2
      i32.const 0
      i32.store offset=236
      local.get 2
      i32.const 11
      i32.store8 offset=424
      local.get 2
      i32.const 0
      i32.store offset=324
      local.get 2
      i32.const 236
      i32.add
      i32.const 16
      i32.add
      local.set 5
      local.get 2
      i32.const 328
      i32.add
      local.set 6
      local.get 2
      i32.const 236
      i32.add
      i32.const 24
      i32.add
      local.set 7
      local.get 2
      i32.const 258
      i32.add
      local.set 8
      local.get 2
      i32.const 256
      i32.add
      local.set 9
      i32.const 0
      local.set 10
      i32.const 0
      local.set 1
      loop  ;; label = @2
        local.get 12
        local.set 11
        loop  ;; label = @3
          local.get 2
          i32.load offset=244
          local.set 13
          local.get 2
          i32.load offset=248
          local.set 14
          loop  ;; label = @4
            i32.const 9
            local.set 15
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 1
                    i32.const 65535
                    i32.and
                    i32.const 36
                    i32.rem_u
                    local.tee 1
                    i32.const -1
                    i32.add
                    br_table 3 (;@5;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 0 (;@8;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 1 (;@7;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 2 (;@6;) 0 (;@8;) 2 (;@6;)
                  end
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 14
                      i32.eqz
                      br_if 0 (;@9;)
                      local.get 2
                      local.get 14
                      i32.const -1
                      i32.add
                      local.tee 14
                      i32.store offset=248
                      local.get 5
                      local.get 14
                      i32.add
                      i32.load8_u
                      local.set 1
                      br 1 (;@8;)
                    end
                    local.get 13
                    local.get 13
                    i64.load offset=32
                    local.tee 16
                    i64.const 1
                    i64.add
                    i64.store offset=32
                    local.get 2
                    local.get 16
                    local.get 13
                    i64.load offset=24
                    i64.xor
                    local.tee 17
                    i64.const 16
                    i64.rotl
                    local.get 17
                    local.get 13
                    i64.load offset=16
                    i64.add
                    local.tee 17
                    i64.xor
                    local.tee 18
                    i64.const 21
                    i64.rotl
                    local.get 18
                    local.get 13
                    i64.load offset=8
                    local.tee 19
                    local.get 13
                    i64.load
                    i64.add
                    local.tee 20
                    i64.const 32
                    i64.rotl
                    i64.add
                    local.tee 18
                    i64.xor
                    local.tee 21
                    i64.const 16
                    i64.rotl
                    local.get 17
                    local.get 19
                    i64.const 13
                    i64.rotl
                    local.get 20
                    i64.xor
                    local.tee 19
                    i64.add
                    local.tee 17
                    i64.const 32
                    i64.rotl
                    i64.const 255
                    i64.xor
                    local.get 21
                    i64.add
                    local.tee 20
                    i64.xor
                    local.tee 21
                    i64.const 21
                    i64.rotl
                    local.get 18
                    local.get 16
                    i64.xor
                    local.get 17
                    local.get 19
                    i64.const 17
                    i64.rotl
                    i64.xor
                    local.tee 16
                    i64.add
                    local.tee 17
                    i64.const 32
                    i64.rotl
                    local.get 21
                    i64.add
                    local.tee 18
                    i64.xor
                    local.tee 19
                    i64.const 16
                    i64.rotl
                    local.get 17
                    local.get 16
                    i64.const 13
                    i64.rotl
                    i64.xor
                    local.tee 16
                    local.get 20
                    i64.add
                    local.tee 17
                    i64.const 32
                    i64.rotl
                    local.get 19
                    i64.add
                    local.tee 19
                    i64.xor
                    local.tee 20
                    i64.const 21
                    i64.rotl
                    local.get 16
                    i64.const 17
                    i64.rotl
                    local.get 17
                    i64.xor
                    local.tee 16
                    local.get 18
                    i64.add
                    local.tee 17
                    i64.const 32
                    i64.rotl
                    local.get 20
                    i64.add
                    local.tee 18
                    i64.xor
                    local.tee 20
                    i64.const 16
                    i64.rotl
                    local.get 16
                    i64.const 13
                    i64.rotl
                    local.get 17
                    i64.xor
                    local.tee 16
                    local.get 19
                    i64.add
                    local.tee 17
                    i64.const 32
                    i64.rotl
                    local.get 20
                    i64.add
                    local.tee 19
                    i64.xor
                    i64.const 21
                    i64.rotl
                    local.get 16
                    i64.const 17
                    i64.rotl
                    local.get 17
                    i64.xor
                    local.tee 16
                    i64.const 13
                    i64.rotl
                    local.get 16
                    local.get 18
                    i64.add
                    i64.xor
                    local.tee 16
                    i64.const 17
                    i64.rotl
                    i64.xor
                    local.get 16
                    local.get 19
                    i64.add
                    local.tee 16
                    i64.const 32
                    i64.rotl
                    i64.xor
                    local.get 16
                    i64.xor
                    local.tee 16
                    i64.store32 offset=252
                    local.get 8
                    local.get 16
                    i64.const 48
                    i64.shr_u
                    i64.store8
                    local.get 9
                    local.get 16
                    i64.const 32
                    i64.shr_u
                    i64.store16
                    i32.const 7
                    local.set 14
                    local.get 2
                    i32.const 7
                    i32.store offset=248
                    local.get 16
                    i64.const 56
                    i64.shr_u
                    i32.wrap_i64
                    local.set 1
                  end
                  local.get 1
                  i32.const 1
                  i32.and
                  i32.const 1049932
                  i32.add
                  i32.load8_u
                  local.set 15
                  br 2 (;@5;)
                end
                i32.const 10
                local.set 15
                br 1 (;@5;)
              end
              block  ;; label = @6
                local.get 1
                i32.const 3
                i32.rem_u
                br_if 0 (;@6;)
                i32.const 0
                local.set 15
                br 1 (;@5;)
              end
              block  ;; label = @6
                block  ;; label = @7
                  local.get 14
                  i32.eqz
                  br_if 0 (;@7;)
                  local.get 2
                  local.get 14
                  i32.const -1
                  i32.add
                  local.tee 14
                  i32.store offset=248
                  local.get 5
                  local.get 14
                  i32.add
                  i32.load8_u
                  local.set 1
                  br 1 (;@6;)
                end
                local.get 13
                local.get 13
                i64.load offset=32
                local.tee 16
                i64.const 1
                i64.add
                i64.store offset=32
                local.get 2
                local.get 16
                local.get 13
                i64.load offset=24
                i64.xor
                local.tee 17
                i64.const 16
                i64.rotl
                local.get 17
                local.get 13
                i64.load offset=16
                i64.add
                local.tee 17
                i64.xor
                local.tee 18
                i64.const 21
                i64.rotl
                local.get 18
                local.get 13
                i64.load offset=8
                local.tee 19
                local.get 13
                i64.load
                i64.add
                local.tee 20
                i64.const 32
                i64.rotl
                i64.add
                local.tee 18
                i64.xor
                local.tee 21
                i64.const 16
                i64.rotl
                local.get 17
                local.get 19
                i64.const 13
                i64.rotl
                local.get 20
                i64.xor
                local.tee 19
                i64.add
                local.tee 17
                i64.const 32
                i64.rotl
                i64.const 255
                i64.xor
                local.get 21
                i64.add
                local.tee 20
                i64.xor
                local.tee 21
                i64.const 21
                i64.rotl
                local.get 18
                local.get 16
                i64.xor
                local.get 17
                local.get 19
                i64.const 17
                i64.rotl
                i64.xor
                local.tee 16
                i64.add
                local.tee 17
                i64.const 32
                i64.rotl
                local.get 21
                i64.add
                local.tee 18
                i64.xor
                local.tee 19
                i64.const 16
                i64.rotl
                local.get 17
                local.get 16
                i64.const 13
                i64.rotl
                i64.xor
                local.tee 16
                local.get 20
                i64.add
                local.tee 17
                i64.const 32
                i64.rotl
                local.get 19
                i64.add
                local.tee 19
                i64.xor
                local.tee 20
                i64.const 21
                i64.rotl
                local.get 16
                i64.const 17
                i64.rotl
                local.get 17
                i64.xor
                local.tee 16
                local.get 18
                i64.add
                local.tee 17
                i64.const 32
                i64.rotl
                local.get 20
                i64.add
                local.tee 18
                i64.xor
                local.tee 20
                i64.const 16
                i64.rotl
                local.get 16
                i64.const 13
                i64.rotl
                local.get 17
                i64.xor
                local.tee 16
                local.get 19
                i64.add
                local.tee 17
                i64.const 32
                i64.rotl
                local.get 20
                i64.add
                local.tee 19
                i64.xor
                i64.const 21
                i64.rotl
                local.get 16
                i64.const 17
                i64.rotl
                local.get 17
                i64.xor
                local.tee 16
                i64.const 13
                i64.rotl
                local.get 16
                local.get 18
                i64.add
                i64.xor
                local.tee 16
                i64.const 17
                i64.rotl
                i64.xor
                local.get 16
                local.get 19
                i64.add
                local.tee 16
                i64.const 32
                i64.rotl
                i64.xor
                local.get 16
                i64.xor
                local.tee 16
                i64.store32 offset=252
                local.get 8
                local.get 16
                i64.const 48
                i64.shr_u
                i64.store8
                local.get 9
                local.get 16
                i64.const 32
                i64.shr_u
                i64.store16
                i32.const 7
                local.set 14
                local.get 2
                i32.const 7
                i32.store offset=248
                local.get 16
                i64.const 56
                i64.shr_u
                i32.wrap_i64
                local.set 1
              end
              local.get 1
              i32.const 7
              i32.and
              i32.const 1049920
              i32.add
              i32.load8_u
              local.set 15
            end
            block  ;; label = @5
              block  ;; label = @6
                local.get 2
                i32.load8_u offset=424
                local.tee 1
                i32.const 11
                i32.eq
                br_if 0 (;@6;)
                block  ;; label = @7
                  i32.const 1
                  local.get 15
                  i32.const 255
                  i32.and
                  local.tee 22
                  i32.shl
                  local.tee 23
                  i32.const 464
                  i32.and
                  br_if 0 (;@7;)
                  local.get 23
                  i32.const 40
                  i32.and
                  i32.eqz
                  br_if 1 (;@6;)
                  local.get 1
                  i32.const -3
                  i32.add
                  br_table 2 (;@5;) 1 (;@6;) 2 (;@5;) 1 (;@6;)
                end
                local.get 1
                local.get 22
                i32.eq
                br_if 1 (;@5;)
              end
              local.get 2
              local.get 15
              i32.store8 offset=424
              i32.const 1
              local.set 13
              i32.const 4
              local.set 1
              i32.const 0
              local.set 24
              local.get 2
              i32.load8_u offset=422
              local.set 14
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          block  ;; label = @12
                            block  ;; label = @13
                              block  ;; label = @14
                                block  ;; label = @15
                                  block  ;; label = @16
                                    local.get 15
                                    i32.const 255
                                    i32.and
                                    local.tee 25
                                    br_table 3 (;@13;) 5 (;@11;) 5 (;@11;) 2 (;@14;) 0 (;@16;) 0 (;@16;) 0 (;@16;) 0 (;@16;) 1 (;@15;) 4 (;@12;) 4 (;@12;) 3 (;@13;)
                                  end
                                  i32.const 7
                                  local.set 1
                                  br 2 (;@13;)
                                end
                                i32.const 3
                                local.set 1
                                i32.const 1
                                local.set 24
                                br 1 (;@13;)
                              end
                              i32.const 6
                              local.set 1
                            end
                            block  ;; label = @13
                              local.get 14
                              i32.const 5
                              i32.shr_u
                              local.tee 22
                              i32.const 7
                              i32.eq
                              br_if 0 (;@13;)
                              local.get 14
                              i32.const 195
                              local.get 14
                              i32.const 195
                              i32.gt_u
                              select
                              local.set 26
                              local.get 1
                              i32.const 2
                              i32.and
                              local.set 27
                              local.get 1
                              i32.const 1
                              i32.and
                              local.set 13
                              local.get 14
                              i32.const 16
                              i32.shl
                              local.set 23
                              loop  ;; label = @14
                                i32.const 1
                                local.get 14
                                i32.shl
                                local.set 1
                                local.get 4
                                local.get 22
                                i32.const 2
                                i32.shl
                                i32.add
                                local.set 22
                                block  ;; label = @15
                                  block  ;; label = @16
                                    block  ;; label = @17
                                      local.get 13
                                      i32.eqz
                                      br_if 0 (;@17;)
                                      local.get 22
                                      i32.load
                                      local.get 1
                                      i32.and
                                      br_if 0 (;@17;)
                                      i32.const 0
                                      local.set 1
                                      br 1 (;@16;)
                                    end
                                    block  ;; label = @17
                                      local.get 27
                                      i32.eqz
                                      br_if 0 (;@17;)
                                      local.get 22
                                      i32.load offset=28
                                      local.get 1
                                      i32.and
                                      br_if 0 (;@17;)
                                      i32.const 16777216
                                      local.set 1
                                      br 1 (;@16;)
                                    end
                                    local.get 24
                                    br_if 1 (;@15;)
                                    local.get 22
                                    i32.load offset=56
                                    local.get 1
                                    i32.and
                                    br_if 1 (;@15;)
                                    i32.const 33554432
                                    local.set 1
                                  end
                                  local.get 1
                                  local.get 23
                                  i32.const 16711680
                                  i32.and
                                  i32.or
                                  local.set 24
                                  br 7 (;@8;)
                                end
                                local.get 26
                                local.get 14
                                i32.eq
                                br_if 4 (;@10;)
                                local.get 23
                                i32.const 65536
                                i32.add
                                local.set 23
                                local.get 14
                                i32.const 1
                                i32.add
                                local.tee 14
                                i32.const 5
                                i32.shr_u
                                local.tee 22
                                i32.const 7
                                i32.ne
                                br_if 0 (;@14;)
                              end
                            end
                            i32.const 7
                            i32.const 7
                            i32.const 1049936
                            call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                            unreachable
                          end
                          i32.const 0
                          local.set 13
                          i32.const 7
                          local.set 1
                        end
                        local.get 1
                        i32.const 2
                        i32.and
                        local.set 26
                        local.get 1
                        i32.const 1
                        i32.and
                        local.set 28
                        local.get 14
                        local.set 29
                        loop  ;; label = @11
                          block  ;; label = @12
                            block  ;; label = @13
                              block  ;; label = @14
                                block  ;; label = @15
                                  local.get 14
                                  i32.const 5
                                  i32.shr_u
                                  local.tee 24
                                  i32.const 7
                                  i32.eq
                                  br_if 0 (;@15;)
                                  local.get 14
                                  i32.const 195
                                  local.get 14
                                  i32.const 195
                                  i32.gt_u
                                  select
                                  local.set 30
                                  i32.const 1
                                  local.get 14
                                  i32.shl
                                  local.set 1
                                  local.get 4
                                  local.get 24
                                  i32.const 2
                                  i32.shl
                                  local.tee 31
                                  i32.add
                                  local.set 23
                                  block  ;; label = @16
                                    local.get 28
                                    i32.eqz
                                    br_if 0 (;@16;)
                                    i32.const 1
                                    local.set 27
                                    local.get 14
                                    local.set 22
                                    local.get 23
                                    i32.load
                                    local.get 1
                                    i32.and
                                    br_if 2 (;@14;)
                                    i32.const 0
                                    local.set 32
                                    local.get 14
                                    local.set 30
                                    br 4 (;@12;)
                                  end
                                  block  ;; label = @16
                                    block  ;; label = @17
                                      local.get 26
                                      i32.eqz
                                      br_if 0 (;@17;)
                                      i32.const 1
                                      local.set 27
                                      i32.const 16777216
                                      local.set 32
                                      local.get 14
                                      local.set 22
                                      local.get 23
                                      i32.load offset=28
                                      local.get 1
                                      i32.and
                                      br_if 1 (;@16;)
                                      local.get 14
                                      local.set 30
                                      br 5 (;@12;)
                                    end
                                    i32.const 33554432
                                    local.set 32
                                    block  ;; label = @17
                                      local.get 4
                                      local.get 31
                                      i32.add
                                      i32.load offset=56
                                      local.get 1
                                      i32.and
                                      br_if 0 (;@17;)
                                      i32.const 1
                                      local.set 27
                                      local.get 14
                                      local.set 30
                                      br 5 (;@12;)
                                    end
                                    local.get 14
                                    local.set 1
                                    loop  ;; label = @17
                                      local.get 1
                                      i32.const 195
                                      i32.lt_u
                                      local.set 27
                                      block  ;; label = @18
                                        local.get 1
                                        i32.const 194
                                        i32.le_u
                                        br_if 0 (;@18;)
                                        i32.const 50331648
                                        local.set 32
                                        br 6 (;@12;)
                                      end
                                      local.get 4
                                      local.get 1
                                      i32.const 1
                                      i32.add
                                      local.tee 1
                                      i32.const 3
                                      i32.shr_u
                                      i32.const 536870908
                                      i32.and
                                      i32.add
                                      i32.load offset=56
                                      local.get 1
                                      i32.shr_u
                                      i32.const 1
                                      i32.and
                                      br_if 0 (;@17;)
                                    end
                                    local.get 1
                                    local.set 30
                                    br 4 (;@12;)
                                  end
                                  loop  ;; label = @16
                                    block  ;; label = @17
                                      local.get 4
                                      local.get 24
                                      i32.const 2
                                      i32.shl
                                      i32.add
                                      i32.load offset=56
                                      local.get 1
                                      i32.and
                                      br_if 0 (;@17;)
                                      i32.const 33554432
                                      local.set 32
                                      local.get 22
                                      local.set 30
                                      br 5 (;@12;)
                                    end
                                    local.get 22
                                    i32.const 194
                                    i32.gt_u
                                    br_if 3 (;@13;)
                                    local.get 4
                                    local.get 22
                                    i32.const 1
                                    i32.add
                                    local.tee 22
                                    i32.const 5
                                    i32.shr_u
                                    local.tee 24
                                    i32.const 2
                                    i32.shl
                                    i32.add
                                    i32.load offset=28
                                    i32.const 1
                                    local.get 22
                                    i32.shl
                                    local.tee 1
                                    i32.and
                                    br_if 0 (;@16;)
                                  end
                                  local.get 22
                                  local.set 30
                                  i32.const 1
                                  local.set 27
                                  br 3 (;@12;)
                                end
                                i32.const 7
                                i32.const 7
                                i32.const 1049936
                                call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                unreachable
                              end
                              block  ;; label = @14
                                block  ;; label = @15
                                  loop  ;; label = @16
                                    block  ;; label = @17
                                      local.get 26
                                      i32.eqz
                                      br_if 0 (;@17;)
                                      local.get 23
                                      i32.load offset=28
                                      local.get 1
                                      i32.and
                                      br_if 0 (;@17;)
                                      i32.const 16777216
                                      local.set 32
                                      br 3 (;@14;)
                                    end
                                    local.get 23
                                    i32.load offset=56
                                    local.get 1
                                    i32.and
                                    i32.eqz
                                    br_if 1 (;@15;)
                                    local.get 22
                                    i32.const 194
                                    i32.gt_u
                                    br_if 3 (;@13;)
                                    local.get 4
                                    local.get 22
                                    i32.const 1
                                    i32.add
                                    local.tee 22
                                    i32.const 3
                                    i32.shr_u
                                    i32.const 536870908
                                    i32.and
                                    i32.add
                                    local.tee 23
                                    i32.load
                                    i32.const 1
                                    local.get 22
                                    i32.shl
                                    local.tee 1
                                    i32.and
                                    br_if 0 (;@16;)
                                  end
                                  i32.const 0
                                  local.set 32
                                  local.get 22
                                  local.set 30
                                  i32.const 1
                                  local.set 27
                                  br 3 (;@12;)
                                end
                                i32.const 33554432
                                local.set 32
                              end
                              local.get 22
                              local.set 30
                              br 1 (;@12;)
                            end
                            i32.const 0
                            local.set 27
                            i32.const 50331648
                            local.set 32
                          end
                          i32.const 0
                          local.set 24
                          local.get 14
                          local.set 1
                          block  ;; label = @12
                            block  ;; label = @13
                              block  ;; label = @14
                                local.get 4
                                local.get 31
                                i32.add
                                local.tee 22
                                i32.load
                                i32.const 1
                                local.get 14
                                i32.shl
                                local.tee 23
                                i32.and
                                br_if 0 (;@14;)
                                local.get 14
                                local.set 1
                                br 1 (;@13;)
                              end
                              block  ;; label = @14
                                loop  ;; label = @15
                                  block  ;; label = @16
                                    local.get 13
                                    br_if 0 (;@16;)
                                    block  ;; label = @17
                                      local.get 22
                                      i32.load offset=28
                                      local.get 23
                                      i32.and
                                      br_if 0 (;@17;)
                                      i32.const 256
                                      local.set 24
                                      br 4 (;@13;)
                                    end
                                    local.get 22
                                    i32.load offset=56
                                    local.get 23
                                    i32.and
                                    i32.eqz
                                    br_if 2 (;@14;)
                                  end
                                  local.get 1
                                  i32.const 194
                                  i32.gt_u
                                  br_if 3 (;@12;)
                                  local.get 4
                                  local.get 1
                                  i32.const 1
                                  i32.add
                                  local.tee 1
                                  i32.const 3
                                  i32.shr_u
                                  i32.const 536870908
                                  i32.and
                                  i32.add
                                  local.tee 22
                                  i32.load
                                  i32.const 1
                                  local.get 1
                                  i32.shl
                                  local.tee 23
                                  i32.and
                                  i32.eqz
                                  br_if 2 (;@13;)
                                  br 0 (;@15;)
                                end
                              end
                              i32.const 512
                              local.set 24
                            end
                            local.get 27
                            local.get 30
                            i32.const 255
                            i32.and
                            local.get 1
                            i32.const 255
                            i32.and
                            i32.eq
                            i32.and
                            br_if 3 (;@9;)
                          end
                          local.get 14
                          i32.const 1
                          i32.add
                          local.set 14
                          local.get 29
                          i32.const 255
                          i32.and
                          local.set 1
                          local.get 29
                          i32.const 1
                          i32.add
                          local.set 29
                          local.get 1
                          i32.const 195
                          i32.lt_u
                          br_if 0 (;@11;)
                        end
                      end
                      local.get 11
                      local.set 12
                      br 2 (;@7;)
                    end
                    local.get 30
                    i32.const 16
                    i32.shl
                    i32.const 16711680
                    i32.and
                    local.get 32
                    i32.or
                    local.get 24
                    i32.or
                    i32.const 1
                    i32.or
                    local.set 24
                  end
                  local.get 2
                  local.get 24
                  i32.store offset=428
                  i64.const 0
                  local.set 33
                  i64.const 0
                  local.set 12
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          block  ;; label = @12
                            block  ;; label = @13
                              block  ;; label = @14
                                block  ;; label = @15
                                  block  ;; label = @16
                                    block  ;; label = @17
                                      block  ;; label = @18
                                        block  ;; label = @19
                                          block  ;; label = @20
                                            block  ;; label = @21
                                              block  ;; label = @22
                                                block  ;; label = @23
                                                  block  ;; label = @24
                                                    block  ;; label = @25
                                                      block  ;; label = @26
                                                        block  ;; label = @27
                                                          block  ;; label = @28
                                                            local.get 25
                                                            br_table 0 (;@28;) 3 (;@25;) 4 (;@24;) 5 (;@23;) 6 (;@22;) 1 (;@27;) 2 (;@26;) 9 (;@19;) 10 (;@18;) 19 (;@9;) 11 (;@17;) 0 (;@28;)
                                                          end
                                                          local.get 2
                                                          i32.const 128
                                                          i32.add
                                                          local.get 2
                                                          i32.const 236
                                                          i32.add
                                                          i32.const 0
                                                          local.get 2
                                                          i32.const 428
                                                          i32.add
                                                          call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                                                          block  ;; label = @28
                                                            block  ;; label = @29
                                                              local.get 2
                                                              i32.load8_u offset=128
                                                              i32.eqz
                                                              br_if 0 (;@29;)
                                                              local.get 34
                                                              i64.const -256
                                                              i64.and
                                                              i64.const 9
                                                              i64.or
                                                              local.set 16
                                                              br 1 (;@28;)
                                                            end
                                                            local.get 2
                                                            local.get 2
                                                            i32.load8_u offset=129
                                                            local.tee 1
                                                            i64.extend_i32_u
                                                            i64.const 255
                                                            i64.and
                                                            i64.const 8
                                                            i64.shl
                                                            i64.const 1
                                                            i64.or
                                                            local.tee 16
                                                            i64.store offset=432
                                                            local.get 2
                                                            i32.const 120
                                                            i32.add
                                                            local.get 2
                                                            i32.const 236
                                                            i32.add
                                                            i32.const 0
                                                            i32.const 0
                                                            local.get 2
                                                            i32.const 432
                                                            i32.add
                                                            i32.const 1
                                                            local.get 1
                                                            local.get 2
                                                            i32.const 428
                                                            i32.add
                                                            call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                            block  ;; label = @29
                                                              local.get 2
                                                              i32.load8_u offset=120
                                                              i32.eqz
                                                              br_if 0 (;@29;)
                                                              local.get 34
                                                              i64.const -256
                                                              i64.and
                                                              i64.const 9
                                                              i64.or
                                                              local.set 16
                                                              br 1 (;@28;)
                                                            end
                                                            local.get 2
                                                            i32.load8_u offset=121
                                                            local.set 35
                                                            local.get 1
                                                            local.set 36
                                                          end
                                                          local.get 16
                                                          i64.const 255
                                                          i64.and
                                                          i64.const 9
                                                          i64.eq
                                                          br_if 11 (;@16;)
                                                          local.get 16
                                                          local.set 34
                                                          local.get 35
                                                          local.set 14
                                                          local.get 36
                                                          local.set 22
                                                          local.get 16
                                                          local.set 12
                                                          br 18 (;@9;)
                                                        end
                                                        local.get 2
                                                        i32.const 200
                                                        i32.add
                                                        local.get 2
                                                        i32.const 236
                                                        i32.add
                                                        i32.const 5
                                                        local.get 2
                                                        i32.const 428
                                                        i32.add
                                                        call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                                                        block  ;; label = @27
                                                          block  ;; label = @28
                                                            local.get 2
                                                            i32.load8_u offset=200
                                                            i32.eqz
                                                            br_if 0 (;@28;)
                                                            local.get 37
                                                            i64.const -256
                                                            i64.and
                                                            i64.const 9
                                                            i64.or
                                                            local.set 16
                                                            br 1 (;@27;)
                                                          end
                                                          local.get 2
                                                          local.get 2
                                                          i32.load8_u offset=201
                                                          local.tee 1
                                                          i64.extend_i32_u
                                                          i64.const 255
                                                          i64.and
                                                          i64.const 8
                                                          i64.shl
                                                          i64.const 4
                                                          i64.or
                                                          local.tee 16
                                                          i64.store offset=432
                                                          local.get 2
                                                          i32.const 192
                                                          i32.add
                                                          local.get 2
                                                          i32.const 236
                                                          i32.add
                                                          i32.const 5
                                                          i32.const 0
                                                          local.get 2
                                                          i32.const 432
                                                          i32.add
                                                          i32.const 1
                                                          local.get 1
                                                          local.get 2
                                                          i32.const 428
                                                          i32.add
                                                          call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                          block  ;; label = @28
                                                            local.get 2
                                                            i32.load8_u offset=192
                                                            i32.eqz
                                                            br_if 0 (;@28;)
                                                            local.get 37
                                                            i64.const -256
                                                            i64.and
                                                            i64.const 9
                                                            i64.or
                                                            local.set 16
                                                            br 1 (;@27;)
                                                          end
                                                          local.get 2
                                                          i32.load8_u offset=193
                                                          local.set 38
                                                          local.get 1
                                                          local.set 39
                                                        end
                                                        local.get 16
                                                        i64.const 255
                                                        i64.and
                                                        i64.const 9
                                                        i64.eq
                                                        br_if 5 (;@21;)
                                                        local.get 16
                                                        local.set 37
                                                        local.get 38
                                                        local.set 14
                                                        local.get 39
                                                        local.set 22
                                                        local.get 16
                                                        local.set 12
                                                        br 17 (;@9;)
                                                      end
                                                      local.get 2
                                                      i32.const 216
                                                      i32.add
                                                      local.get 2
                                                      i32.const 236
                                                      i32.add
                                                      i32.const 6
                                                      local.get 2
                                                      i32.const 428
                                                      i32.add
                                                      call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                                                      block  ;; label = @26
                                                        block  ;; label = @27
                                                          local.get 2
                                                          i32.load8_u offset=216
                                                          i32.eqz
                                                          br_if 0 (;@27;)
                                                          local.get 40
                                                          i64.const -256
                                                          i64.and
                                                          i64.const 9
                                                          i64.or
                                                          local.set 16
                                                          br 1 (;@26;)
                                                        end
                                                        local.get 2
                                                        local.get 2
                                                        i32.load8_u offset=217
                                                        local.tee 1
                                                        i64.extend_i32_u
                                                        i64.const 255
                                                        i64.and
                                                        i64.const 8
                                                        i64.shl
                                                        i64.const 6
                                                        i64.or
                                                        local.tee 16
                                                        i64.store offset=432
                                                        local.get 2
                                                        i32.const 208
                                                        i32.add
                                                        local.get 2
                                                        i32.const 236
                                                        i32.add
                                                        i32.const 6
                                                        i32.const 0
                                                        local.get 2
                                                        i32.const 432
                                                        i32.add
                                                        i32.const 1
                                                        local.get 1
                                                        local.get 2
                                                        i32.const 428
                                                        i32.add
                                                        call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                        block  ;; label = @27
                                                          local.get 2
                                                          i32.load8_u offset=208
                                                          i32.eqz
                                                          br_if 0 (;@27;)
                                                          local.get 40
                                                          i64.const -256
                                                          i64.and
                                                          i64.const 9
                                                          i64.or
                                                          local.set 16
                                                          br 1 (;@26;)
                                                        end
                                                        local.get 2
                                                        i32.load8_u offset=209
                                                        local.set 41
                                                        local.get 1
                                                        local.set 42
                                                      end
                                                      local.get 16
                                                      i64.const 255
                                                      i64.and
                                                      i64.const 9
                                                      i64.eq
                                                      br_if 5 (;@20;)
                                                      local.get 16
                                                      local.set 40
                                                      local.get 41
                                                      local.set 14
                                                      local.get 42
                                                      local.set 22
                                                      local.get 16
                                                      local.set 12
                                                      br 16 (;@9;)
                                                    end
                                                    local.get 2
                                                    i32.load offset=236
                                                    br_if 13 (;@11;)
                                                    local.get 2
                                                    i32.load offset=244
                                                    local.tee 1
                                                    local.get 1
                                                    i64.load offset=32
                                                    local.tee 16
                                                    i64.const 1
                                                    i64.add
                                                    i64.store offset=32
                                                    local.get 2
                                                    local.get 16
                                                    local.get 1
                                                    i64.load offset=24
                                                    i64.xor
                                                    local.tee 17
                                                    i64.const 16
                                                    i64.rotl
                                                    local.get 17
                                                    local.get 1
                                                    i64.load offset=16
                                                    i64.add
                                                    local.tee 17
                                                    i64.xor
                                                    local.tee 18
                                                    i64.const 21
                                                    i64.rotl
                                                    local.get 18
                                                    local.get 1
                                                    i64.load offset=8
                                                    local.tee 19
                                                    local.get 1
                                                    i64.load
                                                    i64.add
                                                    local.tee 20
                                                    i64.const 32
                                                    i64.rotl
                                                    i64.add
                                                    local.tee 18
                                                    i64.xor
                                                    local.tee 21
                                                    i64.const 16
                                                    i64.rotl
                                                    local.get 17
                                                    local.get 19
                                                    i64.const 13
                                                    i64.rotl
                                                    local.get 20
                                                    i64.xor
                                                    local.tee 19
                                                    i64.add
                                                    local.tee 17
                                                    i64.const 32
                                                    i64.rotl
                                                    i64.const 255
                                                    i64.xor
                                                    local.get 21
                                                    i64.add
                                                    local.tee 20
                                                    i64.xor
                                                    local.tee 21
                                                    i64.const 21
                                                    i64.rotl
                                                    local.get 18
                                                    local.get 16
                                                    i64.xor
                                                    local.get 17
                                                    local.get 19
                                                    i64.const 17
                                                    i64.rotl
                                                    i64.xor
                                                    local.tee 16
                                                    i64.add
                                                    local.tee 17
                                                    i64.const 32
                                                    i64.rotl
                                                    local.get 21
                                                    i64.add
                                                    local.tee 18
                                                    i64.xor
                                                    local.tee 19
                                                    i64.const 16
                                                    i64.rotl
                                                    local.get 17
                                                    local.get 16
                                                    i64.const 13
                                                    i64.rotl
                                                    i64.xor
                                                    local.tee 16
                                                    local.get 20
                                                    i64.add
                                                    local.tee 17
                                                    i64.const 32
                                                    i64.rotl
                                                    local.get 19
                                                    i64.add
                                                    local.tee 19
                                                    i64.xor
                                                    local.tee 20
                                                    i64.const 21
                                                    i64.rotl
                                                    local.get 16
                                                    i64.const 17
                                                    i64.rotl
                                                    local.get 17
                                                    i64.xor
                                                    local.tee 16
                                                    local.get 18
                                                    i64.add
                                                    local.tee 17
                                                    i64.const 32
                                                    i64.rotl
                                                    local.get 20
                                                    i64.add
                                                    local.tee 18
                                                    i64.xor
                                                    local.tee 20
                                                    i64.const 16
                                                    i64.rotl
                                                    local.get 16
                                                    i64.const 13
                                                    i64.rotl
                                                    local.get 17
                                                    i64.xor
                                                    local.tee 16
                                                    local.get 19
                                                    i64.add
                                                    local.tee 17
                                                    i64.const 32
                                                    i64.rotl
                                                    local.get 20
                                                    i64.add
                                                    local.tee 19
                                                    i64.xor
                                                    i64.const 21
                                                    i64.rotl
                                                    local.get 16
                                                    i64.const 17
                                                    i64.rotl
                                                    local.get 17
                                                    i64.xor
                                                    local.tee 16
                                                    i64.const 13
                                                    i64.rotl
                                                    local.get 16
                                                    local.get 18
                                                    i64.add
                                                    i64.xor
                                                    local.tee 16
                                                    i64.const 17
                                                    i64.rotl
                                                    i64.xor
                                                    local.get 16
                                                    local.get 19
                                                    i64.add
                                                    local.tee 16
                                                    i64.const 32
                                                    i64.rotl
                                                    i64.xor
                                                    local.get 16
                                                    i64.xor
                                                    local.tee 16
                                                    i64.store32 offset=240
                                                    local.get 16
                                                    i64.const 32
                                                    i64.shr_u
                                                    i32.wrap_i64
                                                    local.set 1
                                                    i32.const 1
                                                    local.set 14
                                                    br 14 (;@10;)
                                                  end
                                                  local.get 2
                                                  i32.load offset=236
                                                  br_if 10 (;@13;)
                                                  local.get 2
                                                  i32.load offset=244
                                                  local.tee 1
                                                  local.get 1
                                                  i64.load offset=32
                                                  local.tee 16
                                                  i64.const 1
                                                  i64.add
                                                  i64.store offset=32
                                                  local.get 2
                                                  local.get 16
                                                  local.get 1
                                                  i64.load offset=24
                                                  i64.xor
                                                  local.tee 17
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 17
                                                  local.get 1
                                                  i64.load offset=16
                                                  i64.add
                                                  local.tee 17
                                                  i64.xor
                                                  local.tee 18
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 18
                                                  local.get 1
                                                  i64.load offset=8
                                                  local.tee 19
                                                  local.get 1
                                                  i64.load
                                                  i64.add
                                                  local.tee 20
                                                  i64.const 32
                                                  i64.rotl
                                                  i64.add
                                                  local.tee 18
                                                  i64.xor
                                                  local.tee 21
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 17
                                                  local.get 19
                                                  i64.const 13
                                                  i64.rotl
                                                  local.get 20
                                                  i64.xor
                                                  local.tee 19
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  i64.const 255
                                                  i64.xor
                                                  local.get 21
                                                  i64.add
                                                  local.tee 20
                                                  i64.xor
                                                  local.tee 21
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 18
                                                  local.get 16
                                                  i64.xor
                                                  local.get 17
                                                  local.get 19
                                                  i64.const 17
                                                  i64.rotl
                                                  i64.xor
                                                  local.tee 16
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 21
                                                  i64.add
                                                  local.tee 18
                                                  i64.xor
                                                  local.tee 19
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 17
                                                  local.get 16
                                                  i64.const 13
                                                  i64.rotl
                                                  i64.xor
                                                  local.tee 16
                                                  local.get 20
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 19
                                                  i64.add
                                                  local.tee 19
                                                  i64.xor
                                                  local.tee 20
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 16
                                                  i64.const 17
                                                  i64.rotl
                                                  local.get 17
                                                  i64.xor
                                                  local.tee 16
                                                  local.get 18
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 20
                                                  i64.add
                                                  local.tee 18
                                                  i64.xor
                                                  local.tee 20
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 16
                                                  i64.const 13
                                                  i64.rotl
                                                  local.get 17
                                                  i64.xor
                                                  local.tee 16
                                                  local.get 19
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 20
                                                  i64.add
                                                  local.tee 19
                                                  i64.xor
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 16
                                                  i64.const 17
                                                  i64.rotl
                                                  local.get 17
                                                  i64.xor
                                                  local.tee 16
                                                  i64.const 13
                                                  i64.rotl
                                                  local.get 16
                                                  local.get 18
                                                  i64.add
                                                  i64.xor
                                                  local.tee 16
                                                  i64.const 17
                                                  i64.rotl
                                                  i64.xor
                                                  local.get 16
                                                  local.get 19
                                                  i64.add
                                                  local.tee 16
                                                  i64.const 32
                                                  i64.rotl
                                                  i64.xor
                                                  local.get 16
                                                  i64.xor
                                                  local.tee 16
                                                  i64.store32 offset=240
                                                  local.get 16
                                                  i64.const 32
                                                  i64.shr_u
                                                  i32.wrap_i64
                                                  local.set 1
                                                  i32.const 1
                                                  local.set 14
                                                  br 11 (;@12;)
                                                end
                                                local.get 2
                                                i32.load offset=236
                                                br_if 7 (;@15;)
                                                local.get 2
                                                i32.load offset=244
                                                local.tee 1
                                                local.get 1
                                                i64.load offset=32
                                                local.tee 16
                                                i64.const 1
                                                i64.add
                                                i64.store offset=32
                                                local.get 2
                                                local.get 16
                                                local.get 1
                                                i64.load offset=24
                                                i64.xor
                                                local.tee 17
                                                i64.const 16
                                                i64.rotl
                                                local.get 17
                                                local.get 1
                                                i64.load offset=16
                                                i64.add
                                                local.tee 17
                                                i64.xor
                                                local.tee 18
                                                i64.const 21
                                                i64.rotl
                                                local.get 18
                                                local.get 1
                                                i64.load offset=8
                                                local.tee 19
                                                local.get 1
                                                i64.load
                                                i64.add
                                                local.tee 20
                                                i64.const 32
                                                i64.rotl
                                                i64.add
                                                local.tee 18
                                                i64.xor
                                                local.tee 21
                                                i64.const 16
                                                i64.rotl
                                                local.get 17
                                                local.get 19
                                                i64.const 13
                                                i64.rotl
                                                local.get 20
                                                i64.xor
                                                local.tee 19
                                                i64.add
                                                local.tee 17
                                                i64.const 32
                                                i64.rotl
                                                i64.const 255
                                                i64.xor
                                                local.get 21
                                                i64.add
                                                local.tee 20
                                                i64.xor
                                                local.tee 21
                                                i64.const 21
                                                i64.rotl
                                                local.get 18
                                                local.get 16
                                                i64.xor
                                                local.get 17
                                                local.get 19
                                                i64.const 17
                                                i64.rotl
                                                i64.xor
                                                local.tee 16
                                                i64.add
                                                local.tee 17
                                                i64.const 32
                                                i64.rotl
                                                local.get 21
                                                i64.add
                                                local.tee 18
                                                i64.xor
                                                local.tee 19
                                                i64.const 16
                                                i64.rotl
                                                local.get 17
                                                local.get 16
                                                i64.const 13
                                                i64.rotl
                                                i64.xor
                                                local.tee 16
                                                local.get 20
                                                i64.add
                                                local.tee 17
                                                i64.const 32
                                                i64.rotl
                                                local.get 19
                                                i64.add
                                                local.tee 19
                                                i64.xor
                                                local.tee 20
                                                i64.const 21
                                                i64.rotl
                                                local.get 16
                                                i64.const 17
                                                i64.rotl
                                                local.get 17
                                                i64.xor
                                                local.tee 16
                                                local.get 18
                                                i64.add
                                                local.tee 17
                                                i64.const 32
                                                i64.rotl
                                                local.get 20
                                                i64.add
                                                local.tee 18
                                                i64.xor
                                                local.tee 20
                                                i64.const 16
                                                i64.rotl
                                                local.get 16
                                                i64.const 13
                                                i64.rotl
                                                local.get 17
                                                i64.xor
                                                local.tee 16
                                                local.get 19
                                                i64.add
                                                local.tee 17
                                                i64.const 32
                                                i64.rotl
                                                local.get 20
                                                i64.add
                                                local.tee 19
                                                i64.xor
                                                i64.const 21
                                                i64.rotl
                                                local.get 16
                                                i64.const 17
                                                i64.rotl
                                                local.get 17
                                                i64.xor
                                                local.tee 16
                                                i64.const 13
                                                i64.rotl
                                                local.get 16
                                                local.get 18
                                                i64.add
                                                i64.xor
                                                local.tee 16
                                                i64.const 17
                                                i64.rotl
                                                i64.xor
                                                local.get 16
                                                local.get 19
                                                i64.add
                                                local.tee 16
                                                i64.const 32
                                                i64.rotl
                                                i64.xor
                                                local.get 16
                                                i64.xor
                                                local.tee 16
                                                i64.store32 offset=240
                                                local.get 16
                                                i64.const 32
                                                i64.shr_u
                                                i32.wrap_i64
                                                local.set 14
                                                i32.const 1
                                                local.set 1
                                                br 8 (;@14;)
                                              end
                                              local.get 2
                                              i32.load offset=244
                                              local.set 14
                                              local.get 2
                                              i32.load offset=240
                                              local.set 13
                                              local.get 2
                                              i32.load offset=236
                                              local.set 22
                                              loop  ;; label = @22
                                                local.get 22
                                                i32.const 1
                                                i32.and
                                                local.set 23
                                                i32.const 0
                                                local.set 22
                                                local.get 13
                                                local.set 1
                                                block  ;; label = @23
                                                  local.get 23
                                                  br_if 0 (;@23;)
                                                  local.get 14
                                                  local.get 14
                                                  i64.load offset=32
                                                  local.tee 16
                                                  i64.const 1
                                                  i64.add
                                                  i64.store offset=32
                                                  local.get 16
                                                  local.get 14
                                                  i64.load offset=24
                                                  i64.xor
                                                  local.tee 17
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 17
                                                  local.get 14
                                                  i64.load offset=16
                                                  i64.add
                                                  local.tee 17
                                                  i64.xor
                                                  local.tee 18
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 18
                                                  local.get 14
                                                  i64.load offset=8
                                                  local.tee 19
                                                  local.get 14
                                                  i64.load
                                                  i64.add
                                                  local.tee 20
                                                  i64.const 32
                                                  i64.rotl
                                                  i64.add
                                                  local.tee 18
                                                  i64.xor
                                                  local.tee 21
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 17
                                                  local.get 19
                                                  i64.const 13
                                                  i64.rotl
                                                  local.get 20
                                                  i64.xor
                                                  local.tee 19
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  i64.const 255
                                                  i64.xor
                                                  local.get 21
                                                  i64.add
                                                  local.tee 20
                                                  i64.xor
                                                  local.tee 21
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 18
                                                  local.get 16
                                                  i64.xor
                                                  local.get 17
                                                  local.get 19
                                                  i64.const 17
                                                  i64.rotl
                                                  i64.xor
                                                  local.tee 16
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 21
                                                  i64.add
                                                  local.tee 18
                                                  i64.xor
                                                  local.tee 19
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 17
                                                  local.get 16
                                                  i64.const 13
                                                  i64.rotl
                                                  i64.xor
                                                  local.tee 16
                                                  local.get 20
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 19
                                                  i64.add
                                                  local.tee 19
                                                  i64.xor
                                                  local.tee 20
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 16
                                                  i64.const 17
                                                  i64.rotl
                                                  local.get 17
                                                  i64.xor
                                                  local.tee 16
                                                  local.get 18
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 20
                                                  i64.add
                                                  local.tee 18
                                                  i64.xor
                                                  local.tee 20
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 16
                                                  i64.const 13
                                                  i64.rotl
                                                  local.get 17
                                                  i64.xor
                                                  local.tee 16
                                                  local.get 19
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 20
                                                  i64.add
                                                  local.tee 19
                                                  i64.xor
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 16
                                                  i64.const 17
                                                  i64.rotl
                                                  local.get 17
                                                  i64.xor
                                                  local.tee 16
                                                  i64.const 13
                                                  i64.rotl
                                                  local.get 16
                                                  local.get 18
                                                  i64.add
                                                  i64.xor
                                                  local.tee 16
                                                  i64.const 17
                                                  i64.rotl
                                                  i64.xor
                                                  local.get 16
                                                  local.get 19
                                                  i64.add
                                                  local.tee 16
                                                  i64.const 32
                                                  i64.rotl
                                                  i64.xor
                                                  local.get 16
                                                  i64.xor
                                                  local.tee 16
                                                  i32.wrap_i64
                                                  local.set 13
                                                  local.get 16
                                                  i64.const 32
                                                  i64.shr_u
                                                  i32.wrap_i64
                                                  local.set 1
                                                  i32.const 1
                                                  local.set 22
                                                end
                                                local.get 1
                                                i32.eqz
                                                br_if 0 (;@22;)
                                              end
                                              local.get 2
                                              local.get 13
                                              i32.store offset=240
                                              local.get 2
                                              local.get 22
                                              i32.store offset=236
                                              local.get 2
                                              i32.const 5
                                              i32.store8 offset=432
                                              local.get 2
                                              i32.const 184
                                              i32.add
                                              local.get 2
                                              i32.const 236
                                              i32.add
                                              i32.const 4
                                              i32.const 0
                                              local.get 2
                                              i32.const 432
                                              i32.add
                                              i32.const 0
                                              local.get 4
                                              local.get 2
                                              i32.const 428
                                              i32.add
                                              call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                              block  ;; label = @22
                                                local.get 2
                                                i32.load8_u offset=184
                                                i32.eqz
                                                br_if 0 (;@22;)
                                                local.get 11
                                                i64.const -256
                                                i64.and
                                                i64.const 9
                                                i64.or
                                                local.set 12
                                                br 14 (;@8;)
                                              end
                                              local.get 2
                                              i32.load8_u offset=185
                                              local.set 14
                                              i64.const 5
                                              local.set 12
                                              br 12 (;@9;)
                                            end
                                            local.get 11
                                            i64.const -256
                                            i64.and
                                            i64.const 9
                                            i64.or
                                            local.set 12
                                            local.get 16
                                            local.set 37
                                            br 12 (;@8;)
                                          end
                                          local.get 11
                                          i64.const -256
                                          i64.and
                                          i64.const 9
                                          i64.or
                                          local.set 12
                                          local.get 16
                                          local.set 40
                                          br 11 (;@8;)
                                        end
                                        local.get 2
                                        i32.load offset=244
                                        local.set 14
                                        local.get 2
                                        i32.load offset=240
                                        local.set 13
                                        local.get 2
                                        i32.load offset=236
                                        local.set 22
                                        loop  ;; label = @19
                                          local.get 22
                                          i32.const 1
                                          i32.and
                                          local.set 23
                                          i32.const 0
                                          local.set 22
                                          local.get 13
                                          local.set 1
                                          block  ;; label = @20
                                            local.get 23
                                            br_if 0 (;@20;)
                                            local.get 14
                                            local.get 14
                                            i64.load offset=32
                                            local.tee 16
                                            i64.const 1
                                            i64.add
                                            i64.store offset=32
                                            local.get 16
                                            local.get 14
                                            i64.load offset=24
                                            i64.xor
                                            local.tee 17
                                            i64.const 16
                                            i64.rotl
                                            local.get 17
                                            local.get 14
                                            i64.load offset=16
                                            i64.add
                                            local.tee 17
                                            i64.xor
                                            local.tee 18
                                            i64.const 21
                                            i64.rotl
                                            local.get 18
                                            local.get 14
                                            i64.load offset=8
                                            local.tee 19
                                            local.get 14
                                            i64.load
                                            i64.add
                                            local.tee 20
                                            i64.const 32
                                            i64.rotl
                                            i64.add
                                            local.tee 18
                                            i64.xor
                                            local.tee 21
                                            i64.const 16
                                            i64.rotl
                                            local.get 17
                                            local.get 19
                                            i64.const 13
                                            i64.rotl
                                            local.get 20
                                            i64.xor
                                            local.tee 19
                                            i64.add
                                            local.tee 17
                                            i64.const 32
                                            i64.rotl
                                            i64.const 255
                                            i64.xor
                                            local.get 21
                                            i64.add
                                            local.tee 20
                                            i64.xor
                                            local.tee 21
                                            i64.const 21
                                            i64.rotl
                                            local.get 18
                                            local.get 16
                                            i64.xor
                                            local.get 17
                                            local.get 19
                                            i64.const 17
                                            i64.rotl
                                            i64.xor
                                            local.tee 16
                                            i64.add
                                            local.tee 17
                                            i64.const 32
                                            i64.rotl
                                            local.get 21
                                            i64.add
                                            local.tee 18
                                            i64.xor
                                            local.tee 19
                                            i64.const 16
                                            i64.rotl
                                            local.get 17
                                            local.get 16
                                            i64.const 13
                                            i64.rotl
                                            i64.xor
                                            local.tee 16
                                            local.get 20
                                            i64.add
                                            local.tee 17
                                            i64.const 32
                                            i64.rotl
                                            local.get 19
                                            i64.add
                                            local.tee 19
                                            i64.xor
                                            local.tee 20
                                            i64.const 21
                                            i64.rotl
                                            local.get 16
                                            i64.const 17
                                            i64.rotl
                                            local.get 17
                                            i64.xor
                                            local.tee 16
                                            local.get 18
                                            i64.add
                                            local.tee 17
                                            i64.const 32
                                            i64.rotl
                                            local.get 20
                                            i64.add
                                            local.tee 18
                                            i64.xor
                                            local.tee 20
                                            i64.const 16
                                            i64.rotl
                                            local.get 16
                                            i64.const 13
                                            i64.rotl
                                            local.get 17
                                            i64.xor
                                            local.tee 16
                                            local.get 19
                                            i64.add
                                            local.tee 17
                                            i64.const 32
                                            i64.rotl
                                            local.get 20
                                            i64.add
                                            local.tee 19
                                            i64.xor
                                            i64.const 21
                                            i64.rotl
                                            local.get 16
                                            i64.const 17
                                            i64.rotl
                                            local.get 17
                                            i64.xor
                                            local.tee 16
                                            i64.const 13
                                            i64.rotl
                                            local.get 16
                                            local.get 18
                                            i64.add
                                            i64.xor
                                            local.tee 16
                                            i64.const 17
                                            i64.rotl
                                            i64.xor
                                            local.get 16
                                            local.get 19
                                            i64.add
                                            local.tee 16
                                            i64.const 32
                                            i64.rotl
                                            i64.xor
                                            local.get 16
                                            i64.xor
                                            local.tee 16
                                            i32.wrap_i64
                                            local.set 13
                                            local.get 16
                                            i64.const 32
                                            i64.shr_u
                                            i32.wrap_i64
                                            local.set 1
                                            i32.const 1
                                            local.set 22
                                          end
                                          local.get 1
                                          i32.eqz
                                          br_if 0 (;@19;)
                                        end
                                        local.get 2
                                        local.get 13
                                        i32.store offset=240
                                        local.get 2
                                        local.get 22
                                        i32.store offset=236
                                        local.get 2
                                        i32.const 7
                                        i32.store8 offset=432
                                        local.get 2
                                        i32.const 224
                                        i32.add
                                        local.get 2
                                        i32.const 236
                                        i32.add
                                        i32.const 7
                                        i32.const 0
                                        local.get 2
                                        i32.const 432
                                        i32.add
                                        i32.const 0
                                        local.get 4
                                        local.get 2
                                        i32.const 428
                                        i32.add
                                        call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                        block  ;; label = @19
                                          local.get 2
                                          i32.load8_u offset=224
                                          i32.eqz
                                          br_if 0 (;@19;)
                                          local.get 11
                                          i64.const -256
                                          i64.and
                                          i64.const 9
                                          i64.or
                                          local.set 12
                                          br 11 (;@8;)
                                        end
                                        local.get 2
                                        i32.load8_u offset=225
                                        local.set 14
                                        i64.const 7
                                        local.set 12
                                        br 9 (;@9;)
                                      end
                                      local.get 2
                                      i32.load offset=244
                                      local.set 1
                                      local.get 2
                                      i32.load offset=240
                                      local.set 13
                                      local.get 2
                                      i32.load offset=236
                                      local.set 14
                                      loop  ;; label = @18
                                        local.get 14
                                        i32.const 1
                                        i32.and
                                        local.set 23
                                        i32.const 0
                                        local.set 14
                                        local.get 13
                                        local.set 22
                                        block  ;; label = @19
                                          local.get 23
                                          br_if 0 (;@19;)
                                          local.get 1
                                          local.get 1
                                          i64.load offset=32
                                          local.tee 16
                                          i64.const 1
                                          i64.add
                                          i64.store offset=32
                                          local.get 16
                                          local.get 1
                                          i64.load offset=24
                                          i64.xor
                                          local.tee 17
                                          i64.const 16
                                          i64.rotl
                                          local.get 17
                                          local.get 1
                                          i64.load offset=16
                                          i64.add
                                          local.tee 17
                                          i64.xor
                                          local.tee 18
                                          i64.const 21
                                          i64.rotl
                                          local.get 18
                                          local.get 1
                                          i64.load offset=8
                                          local.tee 19
                                          local.get 1
                                          i64.load
                                          i64.add
                                          local.tee 20
                                          i64.const 32
                                          i64.rotl
                                          i64.add
                                          local.tee 18
                                          i64.xor
                                          local.tee 21
                                          i64.const 16
                                          i64.rotl
                                          local.get 17
                                          local.get 19
                                          i64.const 13
                                          i64.rotl
                                          local.get 20
                                          i64.xor
                                          local.tee 19
                                          i64.add
                                          local.tee 17
                                          i64.const 32
                                          i64.rotl
                                          i64.const 255
                                          i64.xor
                                          local.get 21
                                          i64.add
                                          local.tee 20
                                          i64.xor
                                          local.tee 21
                                          i64.const 21
                                          i64.rotl
                                          local.get 18
                                          local.get 16
                                          i64.xor
                                          local.get 17
                                          local.get 19
                                          i64.const 17
                                          i64.rotl
                                          i64.xor
                                          local.tee 16
                                          i64.add
                                          local.tee 17
                                          i64.const 32
                                          i64.rotl
                                          local.get 21
                                          i64.add
                                          local.tee 18
                                          i64.xor
                                          local.tee 19
                                          i64.const 16
                                          i64.rotl
                                          local.get 17
                                          local.get 16
                                          i64.const 13
                                          i64.rotl
                                          i64.xor
                                          local.tee 16
                                          local.get 20
                                          i64.add
                                          local.tee 17
                                          i64.const 32
                                          i64.rotl
                                          local.get 19
                                          i64.add
                                          local.tee 19
                                          i64.xor
                                          local.tee 20
                                          i64.const 21
                                          i64.rotl
                                          local.get 16
                                          i64.const 17
                                          i64.rotl
                                          local.get 17
                                          i64.xor
                                          local.tee 16
                                          local.get 18
                                          i64.add
                                          local.tee 17
                                          i64.const 32
                                          i64.rotl
                                          local.get 20
                                          i64.add
                                          local.tee 18
                                          i64.xor
                                          local.tee 20
                                          i64.const 16
                                          i64.rotl
                                          local.get 16
                                          i64.const 13
                                          i64.rotl
                                          local.get 17
                                          i64.xor
                                          local.tee 16
                                          local.get 19
                                          i64.add
                                          local.tee 17
                                          i64.const 32
                                          i64.rotl
                                          local.get 20
                                          i64.add
                                          local.tee 19
                                          i64.xor
                                          i64.const 21
                                          i64.rotl
                                          local.get 16
                                          i64.const 17
                                          i64.rotl
                                          local.get 17
                                          i64.xor
                                          local.tee 16
                                          i64.const 13
                                          i64.rotl
                                          local.get 16
                                          local.get 18
                                          i64.add
                                          i64.xor
                                          local.tee 16
                                          i64.const 17
                                          i64.rotl
                                          i64.xor
                                          local.get 16
                                          local.get 19
                                          i64.add
                                          local.tee 16
                                          i64.const 32
                                          i64.rotl
                                          i64.xor
                                          local.get 16
                                          i64.xor
                                          local.tee 16
                                          i32.wrap_i64
                                          local.set 13
                                          local.get 16
                                          i64.const 32
                                          i64.shr_u
                                          i32.wrap_i64
                                          local.set 22
                                          i32.const 1
                                          local.set 14
                                        end
                                        local.get 22
                                        i32.const 63
                                        i32.and
                                        local.tee 22
                                        i32.eqz
                                        br_if 0 (;@18;)
                                      end
                                      local.get 2
                                      local.get 13
                                      i32.store offset=240
                                      local.get 2
                                      local.get 14
                                      i32.store offset=236
                                      local.get 2
                                      i32.const 8
                                      i32.store8 offset=432
                                      local.get 2
                                      i32.const 232
                                      i32.add
                                      local.get 2
                                      i32.const 236
                                      i32.add
                                      i32.const 8
                                      i32.const 0
                                      local.get 2
                                      i32.const 432
                                      i32.add
                                      i32.const 0
                                      local.get 4
                                      local.get 2
                                      i32.const 428
                                      i32.add
                                      call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                      block  ;; label = @18
                                        local.get 2
                                        i32.load8_u offset=232
                                        i32.eqz
                                        br_if 0 (;@18;)
                                        local.get 11
                                        i64.const -256
                                        i64.and
                                        i64.const 9
                                        i64.or
                                        local.set 12
                                        br 10 (;@8;)
                                      end
                                      local.get 2
                                      i32.load8_u offset=233
                                      local.set 14
                                      i64.const 8
                                      local.set 12
                                      br 8 (;@9;)
                                    end
                                    i32.const 0
                                    local.set 1
                                    local.get 2
                                    i32.load offset=244
                                    local.set 23
                                    local.get 2
                                    i32.load offset=248
                                    local.set 14
                                    i32.const 0
                                    local.set 22
                                    loop  ;; label = @17
                                      block  ;; label = @18
                                        block  ;; label = @19
                                          local.get 14
                                          i32.eqz
                                          br_if 0 (;@19;)
                                          local.get 2
                                          local.get 14
                                          i32.const -1
                                          i32.add
                                          local.tee 14
                                          i32.store offset=248
                                          local.get 5
                                          local.get 14
                                          i32.add
                                          i32.load8_u
                                          local.set 13
                                          br 1 (;@18;)
                                        end
                                        local.get 23
                                        local.get 23
                                        i64.load offset=32
                                        local.tee 16
                                        i64.const 1
                                        i64.add
                                        i64.store offset=32
                                        local.get 2
                                        local.get 16
                                        local.get 23
                                        i64.load offset=24
                                        i64.xor
                                        local.tee 17
                                        i64.const 16
                                        i64.rotl
                                        local.get 17
                                        local.get 23
                                        i64.load offset=16
                                        i64.add
                                        local.tee 17
                                        i64.xor
                                        local.tee 18
                                        i64.const 21
                                        i64.rotl
                                        local.get 18
                                        local.get 23
                                        i64.load offset=8
                                        local.tee 19
                                        local.get 23
                                        i64.load
                                        i64.add
                                        local.tee 20
                                        i64.const 32
                                        i64.rotl
                                        i64.add
                                        local.tee 18
                                        i64.xor
                                        local.tee 21
                                        i64.const 16
                                        i64.rotl
                                        local.get 17
                                        local.get 19
                                        i64.const 13
                                        i64.rotl
                                        local.get 20
                                        i64.xor
                                        local.tee 19
                                        i64.add
                                        local.tee 17
                                        i64.const 32
                                        i64.rotl
                                        i64.const 255
                                        i64.xor
                                        local.get 21
                                        i64.add
                                        local.tee 20
                                        i64.xor
                                        local.tee 21
                                        i64.const 21
                                        i64.rotl
                                        local.get 18
                                        local.get 16
                                        i64.xor
                                        local.get 17
                                        local.get 19
                                        i64.const 17
                                        i64.rotl
                                        i64.xor
                                        local.tee 16
                                        i64.add
                                        local.tee 17
                                        i64.const 32
                                        i64.rotl
                                        local.get 21
                                        i64.add
                                        local.tee 18
                                        i64.xor
                                        local.tee 19
                                        i64.const 16
                                        i64.rotl
                                        local.get 17
                                        local.get 16
                                        i64.const 13
                                        i64.rotl
                                        i64.xor
                                        local.tee 16
                                        local.get 20
                                        i64.add
                                        local.tee 17
                                        i64.const 32
                                        i64.rotl
                                        local.get 19
                                        i64.add
                                        local.tee 19
                                        i64.xor
                                        local.tee 20
                                        i64.const 21
                                        i64.rotl
                                        local.get 16
                                        i64.const 17
                                        i64.rotl
                                        local.get 17
                                        i64.xor
                                        local.tee 16
                                        local.get 18
                                        i64.add
                                        local.tee 17
                                        i64.const 32
                                        i64.rotl
                                        local.get 20
                                        i64.add
                                        local.tee 18
                                        i64.xor
                                        local.tee 20
                                        i64.const 16
                                        i64.rotl
                                        local.get 16
                                        i64.const 13
                                        i64.rotl
                                        local.get 17
                                        i64.xor
                                        local.tee 16
                                        local.get 19
                                        i64.add
                                        local.tee 17
                                        i64.const 32
                                        i64.rotl
                                        local.get 20
                                        i64.add
                                        local.tee 19
                                        i64.xor
                                        i64.const 21
                                        i64.rotl
                                        local.get 16
                                        i64.const 17
                                        i64.rotl
                                        local.get 17
                                        i64.xor
                                        local.tee 16
                                        i64.const 13
                                        i64.rotl
                                        local.get 16
                                        local.get 18
                                        i64.add
                                        i64.xor
                                        local.tee 16
                                        i64.const 17
                                        i64.rotl
                                        i64.xor
                                        local.get 16
                                        local.get 19
                                        i64.add
                                        local.tee 16
                                        i64.const 32
                                        i64.rotl
                                        i64.xor
                                        local.get 16
                                        i64.xor
                                        local.tee 16
                                        i64.store32 offset=252
                                        local.get 8
                                        local.get 16
                                        i64.const 48
                                        i64.shr_u
                                        i64.store8
                                        local.get 9
                                        local.get 16
                                        i64.const 32
                                        i64.shr_u
                                        i64.store16
                                        i32.const 7
                                        local.set 14
                                        local.get 2
                                        i32.const 7
                                        i32.store offset=248
                                        local.get 16
                                        i64.const 56
                                        i64.shr_u
                                        i32.wrap_i64
                                        local.set 13
                                      end
                                      i32.const 0
                                      i32.const 1
                                      local.get 13
                                      i32.shl
                                      local.tee 13
                                      local.get 13
                                      local.get 1
                                      i32.and
                                      local.tee 13
                                      select
                                      local.get 1
                                      i32.or
                                      local.set 1
                                      local.get 22
                                      local.get 13
                                      i32.eqz
                                      i32.add
                                      local.tee 22
                                      i32.const 4
                                      i32.lt_u
                                      br_if 0 (;@17;)
                                    end
                                    i64.const 0
                                    local.set 12
                                    br 7 (;@9;)
                                  end
                                  local.get 11
                                  i64.const -256
                                  i64.and
                                  i64.const 9
                                  i64.or
                                  local.set 12
                                  local.get 16
                                  local.set 34
                                  br 7 (;@8;)
                                end
                                i32.const 0
                                local.set 1
                                local.get 2
                                i32.load offset=240
                                local.set 14
                              end
                              local.get 2
                              local.get 1
                              i32.store offset=236
                              local.get 2
                              i32.const 176
                              i32.add
                              local.get 2
                              i32.const 236
                              i32.add
                              i32.const 3
                              local.get 2
                              i32.const 428
                              i32.add
                              call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                              block  ;; label = @14
                                block  ;; label = @15
                                  local.get 2
                                  i32.load8_u offset=176
                                  i32.eqz
                                  br_if 0 (;@15;)
                                  local.get 43
                                  i64.const -256
                                  i64.and
                                  i64.const 9
                                  i64.or
                                  local.set 16
                                  br 1 (;@14;)
                                end
                                local.get 2
                                local.get 2
                                i32.load8_u offset=177
                                local.tee 1
                                i64.extend_i32_u
                                i64.const 255
                                i64.and
                                i64.const 8
                                i64.shl
                                i64.const 4
                                i64.or
                                local.tee 16
                                i64.store offset=432
                                local.get 2
                                i32.const 168
                                i32.add
                                local.get 2
                                i32.const 236
                                i32.add
                                i32.const 3
                                i32.const 0
                                local.get 2
                                i32.const 432
                                i32.add
                                i32.const 1
                                local.get 1
                                local.get 2
                                i32.const 428
                                i32.add
                                call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                block  ;; label = @15
                                  local.get 2
                                  i32.load8_u offset=168
                                  i32.eqz
                                  br_if 0 (;@15;)
                                  local.get 43
                                  i64.const -256
                                  i64.and
                                  i64.const 9
                                  i64.or
                                  local.set 16
                                  br 1 (;@14;)
                                end
                                local.get 2
                                i32.load8_u offset=169
                                local.set 44
                                local.get 1
                                local.set 45
                              end
                              block  ;; label = @14
                                local.get 16
                                i64.const 255
                                i64.and
                                i64.const 9
                                i64.ne
                                br_if 0 (;@14;)
                                local.get 11
                                i64.const -256
                                i64.and
                                i64.const 9
                                i64.or
                                local.set 12
                                local.get 16
                                local.set 43
                                br 6 (;@8;)
                              end
                              local.get 14
                              i32.const 24
                              i32.shl
                              i32.const 50331648
                              i32.and
                              i64.extend_i32_u
                              local.set 33
                              local.get 16
                              local.set 43
                              local.get 44
                              local.set 14
                              local.get 45
                              local.set 22
                              local.get 16
                              local.set 12
                              br 4 (;@9;)
                            end
                            i32.const 0
                            local.set 14
                            local.get 2
                            i32.load offset=240
                            local.set 1
                          end
                          local.get 2
                          local.get 14
                          i32.store offset=236
                          local.get 2
                          local.get 1
                          i32.store offset=436
                          local.get 2
                          i32.const 3
                          i32.store8 offset=432
                          local.get 2
                          i32.const 160
                          i32.add
                          local.get 2
                          i32.const 236
                          i32.add
                          i32.const 2
                          local.get 2
                          i32.const 428
                          i32.add
                          call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                          block  ;; label = @12
                            block  ;; label = @13
                              local.get 2
                              i32.load8_u offset=160
                              br_if 0 (;@13;)
                              local.get 2
                              i32.const 152
                              i32.add
                              local.get 2
                              i32.const 236
                              i32.add
                              i32.const 2
                              i32.const 0
                              local.get 2
                              i32.const 432
                              i32.add
                              i32.const 1
                              local.get 2
                              i32.load8_u offset=161
                              local.tee 22
                              local.get 2
                              i32.const 428
                              i32.add
                              call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                              local.get 2
                              i32.load8_u offset=152
                              i32.eqz
                              br_if 1 (;@12;)
                            end
                            local.get 11
                            i64.const -256
                            i64.and
                            i64.const 9
                            i64.or
                            local.set 12
                            br 4 (;@8;)
                          end
                          local.get 2
                          i32.load8_u offset=153
                          local.set 14
                          local.get 1
                          i64.extend_i32_u
                          i64.const 32
                          i64.shl
                          i64.const 3
                          i64.or
                          local.set 12
                          br 2 (;@9;)
                        end
                        i32.const 0
                        local.set 14
                        local.get 2
                        i32.load offset=240
                        local.set 1
                      end
                      local.get 2
                      local.get 14
                      i32.store offset=236
                      local.get 2
                      local.get 1
                      i32.store offset=436
                      local.get 2
                      i32.const 2
                      i32.store8 offset=432
                      local.get 2
                      i32.const 144
                      i32.add
                      local.get 2
                      i32.const 236
                      i32.add
                      i32.const 1
                      local.get 2
                      i32.const 428
                      i32.add
                      call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                      block  ;; label = @10
                        block  ;; label = @11
                          local.get 2
                          i32.load8_u offset=144
                          br_if 0 (;@11;)
                          local.get 2
                          i32.const 136
                          i32.add
                          local.get 2
                          i32.const 236
                          i32.add
                          i32.const 1
                          i32.const 0
                          local.get 2
                          i32.const 432
                          i32.add
                          i32.const 1
                          local.get 2
                          i32.load8_u offset=145
                          local.tee 22
                          local.get 2
                          i32.const 428
                          i32.add
                          call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                          local.get 2
                          i32.load8_u offset=136
                          i32.eqz
                          br_if 1 (;@10;)
                        end
                        local.get 11
                        i64.const -256
                        i64.and
                        i64.const 9
                        i64.or
                        local.set 12
                        br 2 (;@8;)
                      end
                      local.get 2
                      i32.load8_u offset=137
                      local.set 14
                      local.get 1
                      i64.extend_i32_u
                      i64.const 32
                      i64.shl
                      i64.const 2
                      i64.or
                      local.set 12
                    end
                    local.get 33
                    local.get 14
                    i64.extend_i32_u
                    i64.const 255
                    i64.and
                    i64.const 8
                    i64.shl
                    local.get 15
                    i64.extend_i32_u
                    i64.const 255
                    i64.and
                    i64.or
                    local.get 22
                    i64.extend_i32_u
                    i64.const 255
                    i64.and
                    i64.const 16
                    i64.shl
                    i64.or
                    i64.or
                    local.get 1
                    i64.extend_i32_u
                    i64.const 32
                    i64.shl
                    i64.or
                    local.set 46
                  end
                  local.get 12
                  i64.const 255
                  i64.and
                  i64.const 9
                  i64.eq
                  br_if 0 (;@7;)
                  local.get 24
                  i32.const 24
                  i32.shr_u
                  local.set 1
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          local.get 24
                          i32.const 50331647
                          i32.gt_u
                          br_if 0 (;@11;)
                          local.get 24
                          i32.const 16
                          i32.shr_u
                          local.tee 14
                          i32.const 255
                          i32.and
                          local.tee 23
                          i32.const 5
                          i32.shr_u
                          local.tee 22
                          i32.const 7
                          i32.eq
                          br_if 1 (;@10;)
                          local.get 4
                          local.get 1
                          i32.const 28
                          i32.mul
                          i32.add
                          local.get 22
                          i32.const 2
                          i32.shl
                          local.tee 22
                          i32.add
                          local.tee 1
                          local.get 1
                          i32.load
                          i32.const 1
                          local.get 14
                          i32.shl
                          local.tee 1
                          i32.or
                          i32.store
                          block  ;; label = @12
                            local.get 24
                            i32.const 1
                            i32.and
                            i32.eqz
                            br_if 0 (;@12;)
                            local.get 24
                            i32.const 8
                            i32.shr_u
                            i32.const 255
                            i32.and
                            local.tee 14
                            i32.const 3
                            i32.ge_u
                            br_if 3 (;@9;)
                            local.get 4
                            local.get 14
                            i32.const 28
                            i32.mul
                            i32.add
                            local.get 22
                            i32.add
                            local.tee 14
                            local.get 14
                            i32.load
                            local.get 1
                            i32.or
                            i32.store
                          end
                          i32.const 3
                          local.set 1
                          local.get 12
                          local.set 47
                          local.get 46
                          local.set 48
                          block  ;; label = @12
                            block  ;; label = @13
                              block  ;; label = @14
                                local.get 46
                                i32.wrap_i64
                                local.tee 14
                                i32.const 255
                                i32.and
                                br_table 2 (;@12;) 0 (;@14;) 0 (;@14;) 1 (;@13;) 1 (;@13;) 1 (;@13;) 1 (;@13;) 1 (;@13;) 1 (;@13;) 8 (;@6;) 8 (;@6;) 2 (;@12;)
                              end
                              i32.const 4
                              local.set 1
                              br 1 (;@12;)
                            end
                            i32.const 1
                            local.set 1
                          end
                          local.get 1
                          local.get 23
                          i32.add
                          local.tee 1
                          i32.const 196
                          i32.ge_u
                          br_if 3 (;@8;)
                          local.get 14
                          i32.const 8
                          i32.shr_u
                          i32.const 255
                          i32.and
                          local.set 14
                          block  ;; label = @12
                            local.get 46
                            i64.const 63488
                            i64.and
                            i64.const 0
                            i64.ne
                            br_if 0 (;@12;)
                            local.get 6
                            local.get 14
                            i32.add
                            local.get 1
                            i32.store8
                            local.get 12
                            local.set 47
                            local.get 46
                            local.set 48
                            br 6 (;@6;)
                          end
                          local.get 14
                          i32.const 8
                          i32.const 1050044
                          call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                          unreachable
                        end
                        local.get 1
                        i32.const 3
                        i32.const 1049952
                        call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                        unreachable
                      end
                      i32.const 7
                      i32.const 7
                      i32.const 1049968
                      call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                      unreachable
                    end
                    local.get 14
                    i32.const 3
                    i32.const 1049952
                    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                    unreachable
                  end
                  i32.const 1049984
                  i32.const 44
                  local.get 2
                  i32.const 447
                  i32.add
                  i32.const 1049816
                  i32.const 1050028
                  call $_RNvNtCsgXGp5Oqx2Ny_4core6result13unwrap_failed
                  unreachable
                end
                local.get 47
                i64.const -256
                i64.and
                i64.const 9
                i64.or
                local.set 47
              end
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          block  ;; label = @12
                            block  ;; label = @13
                              block  ;; label = @14
                                block  ;; label = @15
                                  block  ;; label = @16
                                    block  ;; label = @17
                                      block  ;; label = @18
                                        block  ;; label = @19
                                          block  ;; label = @20
                                            local.get 47
                                            i64.const 255
                                            i64.and
                                            i64.const 9
                                            i64.ne
                                            br_if 0 (;@20;)
                                            local.get 2
                                            i32.load offset=244
                                            local.set 27
                                            local.get 2
                                            i32.load offset=248
                                            local.set 29
                                            loop  ;; label = @21
                                              i32.const 9
                                              local.set 15
                                              block  ;; label = @22
                                                block  ;; label = @23
                                                  block  ;; label = @24
                                                    block  ;; label = @25
                                                      local.get 2
                                                      i32.load16_u offset=420
                                                      i32.const 36
                                                      i32.rem_u
                                                      local.tee 1
                                                      i32.const -1
                                                      i32.add
                                                      br_table 3 (;@22;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 0 (;@25;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 1 (;@24;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 2 (;@23;) 0 (;@25;) 2 (;@23;)
                                                    end
                                                    block  ;; label = @25
                                                      block  ;; label = @26
                                                        local.get 29
                                                        i32.eqz
                                                        br_if 0 (;@26;)
                                                        local.get 2
                                                        local.get 29
                                                        i32.const -1
                                                        i32.add
                                                        local.tee 29
                                                        i32.store offset=248
                                                        local.get 5
                                                        local.get 29
                                                        i32.add
                                                        i32.load8_u
                                                        local.set 1
                                                        br 1 (;@25;)
                                                      end
                                                      local.get 27
                                                      local.get 27
                                                      i64.load offset=32
                                                      local.tee 16
                                                      i64.const 1
                                                      i64.add
                                                      i64.store offset=32
                                                      local.get 2
                                                      local.get 16
                                                      local.get 27
                                                      i64.load offset=24
                                                      i64.xor
                                                      local.tee 17
                                                      i64.const 16
                                                      i64.rotl
                                                      local.get 17
                                                      local.get 27
                                                      i64.load offset=16
                                                      i64.add
                                                      local.tee 17
                                                      i64.xor
                                                      local.tee 18
                                                      i64.const 21
                                                      i64.rotl
                                                      local.get 18
                                                      local.get 27
                                                      i64.load offset=8
                                                      local.tee 19
                                                      local.get 27
                                                      i64.load
                                                      i64.add
                                                      local.tee 20
                                                      i64.const 32
                                                      i64.rotl
                                                      i64.add
                                                      local.tee 18
                                                      i64.xor
                                                      local.tee 21
                                                      i64.const 16
                                                      i64.rotl
                                                      local.get 17
                                                      local.get 19
                                                      i64.const 13
                                                      i64.rotl
                                                      local.get 20
                                                      i64.xor
                                                      local.tee 19
                                                      i64.add
                                                      local.tee 17
                                                      i64.const 32
                                                      i64.rotl
                                                      i64.const 255
                                                      i64.xor
                                                      local.get 21
                                                      i64.add
                                                      local.tee 20
                                                      i64.xor
                                                      local.tee 21
                                                      i64.const 21
                                                      i64.rotl
                                                      local.get 18
                                                      local.get 16
                                                      i64.xor
                                                      local.get 17
                                                      local.get 19
                                                      i64.const 17
                                                      i64.rotl
                                                      i64.xor
                                                      local.tee 16
                                                      i64.add
                                                      local.tee 17
                                                      i64.const 32
                                                      i64.rotl
                                                      local.get 21
                                                      i64.add
                                                      local.tee 18
                                                      i64.xor
                                                      local.tee 19
                                                      i64.const 16
                                                      i64.rotl
                                                      local.get 17
                                                      local.get 16
                                                      i64.const 13
                                                      i64.rotl
                                                      i64.xor
                                                      local.tee 16
                                                      local.get 20
                                                      i64.add
                                                      local.tee 17
                                                      i64.const 32
                                                      i64.rotl
                                                      local.get 19
                                                      i64.add
                                                      local.tee 19
                                                      i64.xor
                                                      local.tee 20
                                                      i64.const 21
                                                      i64.rotl
                                                      local.get 16
                                                      i64.const 17
                                                      i64.rotl
                                                      local.get 17
                                                      i64.xor
                                                      local.tee 16
                                                      local.get 18
                                                      i64.add
                                                      local.tee 17
                                                      i64.const 32
                                                      i64.rotl
                                                      local.get 20
                                                      i64.add
                                                      local.tee 18
                                                      i64.xor
                                                      local.tee 20
                                                      i64.const 16
                                                      i64.rotl
                                                      local.get 16
                                                      i64.const 13
                                                      i64.rotl
                                                      local.get 17
                                                      i64.xor
                                                      local.tee 16
                                                      local.get 19
                                                      i64.add
                                                      local.tee 17
                                                      i64.const 32
                                                      i64.rotl
                                                      local.get 20
                                                      i64.add
                                                      local.tee 19
                                                      i64.xor
                                                      i64.const 21
                                                      i64.rotl
                                                      local.get 16
                                                      i64.const 17
                                                      i64.rotl
                                                      local.get 17
                                                      i64.xor
                                                      local.tee 16
                                                      i64.const 13
                                                      i64.rotl
                                                      local.get 16
                                                      local.get 18
                                                      i64.add
                                                      i64.xor
                                                      local.tee 16
                                                      i64.const 17
                                                      i64.rotl
                                                      i64.xor
                                                      local.get 16
                                                      local.get 19
                                                      i64.add
                                                      local.tee 16
                                                      i64.const 32
                                                      i64.rotl
                                                      i64.xor
                                                      local.get 16
                                                      i64.xor
                                                      local.tee 16
                                                      i64.store32 offset=252
                                                      local.get 8
                                                      local.get 16
                                                      i64.const 48
                                                      i64.shr_u
                                                      i64.store8
                                                      local.get 9
                                                      local.get 16
                                                      i64.const 32
                                                      i64.shr_u
                                                      i64.store16
                                                      i32.const 7
                                                      local.set 29
                                                      local.get 2
                                                      i32.const 7
                                                      i32.store offset=248
                                                      local.get 16
                                                      i64.const 56
                                                      i64.shr_u
                                                      i32.wrap_i64
                                                      local.set 1
                                                    end
                                                    local.get 1
                                                    i32.const 1
                                                    i32.and
                                                    i32.const 1049932
                                                    i32.add
                                                    i32.load8_u
                                                    local.set 15
                                                    br 2 (;@22;)
                                                  end
                                                  i32.const 10
                                                  local.set 15
                                                  br 1 (;@22;)
                                                end
                                                block  ;; label = @23
                                                  local.get 1
                                                  i32.const 3
                                                  i32.rem_u
                                                  br_if 0 (;@23;)
                                                  i32.const 0
                                                  local.set 15
                                                  br 1 (;@22;)
                                                end
                                                block  ;; label = @23
                                                  block  ;; label = @24
                                                    local.get 29
                                                    i32.eqz
                                                    br_if 0 (;@24;)
                                                    local.get 2
                                                    local.get 29
                                                    i32.const -1
                                                    i32.add
                                                    local.tee 29
                                                    i32.store offset=248
                                                    local.get 5
                                                    local.get 29
                                                    i32.add
                                                    i32.load8_u
                                                    local.set 1
                                                    br 1 (;@23;)
                                                  end
                                                  local.get 27
                                                  local.get 27
                                                  i64.load offset=32
                                                  local.tee 16
                                                  i64.const 1
                                                  i64.add
                                                  i64.store offset=32
                                                  local.get 2
                                                  local.get 16
                                                  local.get 27
                                                  i64.load offset=24
                                                  i64.xor
                                                  local.tee 17
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 17
                                                  local.get 27
                                                  i64.load offset=16
                                                  i64.add
                                                  local.tee 17
                                                  i64.xor
                                                  local.tee 18
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 18
                                                  local.get 27
                                                  i64.load offset=8
                                                  local.tee 19
                                                  local.get 27
                                                  i64.load
                                                  i64.add
                                                  local.tee 20
                                                  i64.const 32
                                                  i64.rotl
                                                  i64.add
                                                  local.tee 18
                                                  i64.xor
                                                  local.tee 21
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 17
                                                  local.get 19
                                                  i64.const 13
                                                  i64.rotl
                                                  local.get 20
                                                  i64.xor
                                                  local.tee 19
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  i64.const 255
                                                  i64.xor
                                                  local.get 21
                                                  i64.add
                                                  local.tee 20
                                                  i64.xor
                                                  local.tee 21
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 18
                                                  local.get 16
                                                  i64.xor
                                                  local.get 17
                                                  local.get 19
                                                  i64.const 17
                                                  i64.rotl
                                                  i64.xor
                                                  local.tee 16
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 21
                                                  i64.add
                                                  local.tee 18
                                                  i64.xor
                                                  local.tee 19
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 17
                                                  local.get 16
                                                  i64.const 13
                                                  i64.rotl
                                                  i64.xor
                                                  local.tee 16
                                                  local.get 20
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 19
                                                  i64.add
                                                  local.tee 19
                                                  i64.xor
                                                  local.tee 20
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 16
                                                  i64.const 17
                                                  i64.rotl
                                                  local.get 17
                                                  i64.xor
                                                  local.tee 16
                                                  local.get 18
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 20
                                                  i64.add
                                                  local.tee 18
                                                  i64.xor
                                                  local.tee 20
                                                  i64.const 16
                                                  i64.rotl
                                                  local.get 16
                                                  i64.const 13
                                                  i64.rotl
                                                  local.get 17
                                                  i64.xor
                                                  local.tee 16
                                                  local.get 19
                                                  i64.add
                                                  local.tee 17
                                                  i64.const 32
                                                  i64.rotl
                                                  local.get 20
                                                  i64.add
                                                  local.tee 19
                                                  i64.xor
                                                  i64.const 21
                                                  i64.rotl
                                                  local.get 16
                                                  i64.const 17
                                                  i64.rotl
                                                  local.get 17
                                                  i64.xor
                                                  local.tee 16
                                                  i64.const 13
                                                  i64.rotl
                                                  local.get 16
                                                  local.get 18
                                                  i64.add
                                                  i64.xor
                                                  local.tee 16
                                                  i64.const 17
                                                  i64.rotl
                                                  i64.xor
                                                  local.get 16
                                                  local.get 19
                                                  i64.add
                                                  local.tee 16
                                                  i64.const 32
                                                  i64.rotl
                                                  i64.xor
                                                  local.get 16
                                                  i64.xor
                                                  local.tee 16
                                                  i64.store32 offset=252
                                                  local.get 8
                                                  local.get 16
                                                  i64.const 48
                                                  i64.shr_u
                                                  i64.store8
                                                  local.get 9
                                                  local.get 16
                                                  i64.const 32
                                                  i64.shr_u
                                                  i64.store16
                                                  i32.const 7
                                                  local.set 29
                                                  local.get 2
                                                  i32.const 7
                                                  i32.store offset=248
                                                  local.get 16
                                                  i64.const 56
                                                  i64.shr_u
                                                  i32.wrap_i64
                                                  local.set 1
                                                end
                                                local.get 1
                                                i32.const 3
                                                i32.and
                                                i32.const 1049928
                                                i32.add
                                                i32.load8_u
                                                local.set 15
                                              end
                                              block  ;; label = @22
                                                local.get 2
                                                i32.load8_u offset=424
                                                local.tee 1
                                                i32.const 11
                                                i32.eq
                                                br_if 0 (;@22;)
                                                block  ;; label = @23
                                                  i32.const 1
                                                  local.get 15
                                                  i32.const 255
                                                  i32.and
                                                  local.tee 14
                                                  i32.shl
                                                  local.tee 22
                                                  i32.const 464
                                                  i32.and
                                                  br_if 0 (;@23;)
                                                  local.get 22
                                                  i32.const 40
                                                  i32.and
                                                  i32.eqz
                                                  br_if 1 (;@22;)
                                                  local.get 1
                                                  i32.const -3
                                                  i32.add
                                                  br_table 2 (;@21;) 1 (;@22;) 2 (;@21;) 1 (;@22;)
                                                end
                                                local.get 1
                                                local.get 14
                                                i32.eq
                                                br_if 1 (;@21;)
                                              end
                                            end
                                            local.get 2
                                            local.get 15
                                            i32.store8 offset=424
                                            i32.const 1
                                            local.set 13
                                            i32.const 4
                                            local.set 1
                                            i32.const 0
                                            local.set 24
                                            local.get 2
                                            i32.load8_u offset=422
                                            local.set 14
                                            block  ;; label = @21
                                              block  ;; label = @22
                                                block  ;; label = @23
                                                  block  ;; label = @24
                                                    block  ;; label = @25
                                                      block  ;; label = @26
                                                        block  ;; label = @27
                                                          block  ;; label = @28
                                                            local.get 15
                                                            i32.const 255
                                                            i32.and
                                                            local.tee 49
                                                            br_table 3 (;@25;) 5 (;@23;) 5 (;@23;) 2 (;@26;) 0 (;@28;) 0 (;@28;) 0 (;@28;) 0 (;@28;) 1 (;@27;) 4 (;@24;) 4 (;@24;) 3 (;@25;)
                                                          end
                                                          i32.const 7
                                                          local.set 1
                                                          br 2 (;@25;)
                                                        end
                                                        i32.const 3
                                                        local.set 1
                                                        i32.const 1
                                                        local.set 24
                                                        br 1 (;@25;)
                                                      end
                                                      i32.const 6
                                                      local.set 1
                                                    end
                                                    block  ;; label = @25
                                                      local.get 14
                                                      i32.const 5
                                                      i32.shr_u
                                                      local.tee 22
                                                      i32.const 7
                                                      i32.eq
                                                      br_if 0 (;@25;)
                                                      local.get 14
                                                      i32.const 195
                                                      local.get 14
                                                      i32.const 195
                                                      i32.gt_u
                                                      select
                                                      local.set 30
                                                      local.get 1
                                                      i32.const 2
                                                      i32.and
                                                      local.set 26
                                                      local.get 1
                                                      i32.const 1
                                                      i32.and
                                                      local.set 13
                                                      local.get 14
                                                      i32.const 16
                                                      i32.shl
                                                      local.set 23
                                                      loop  ;; label = @26
                                                        i32.const 1
                                                        local.get 14
                                                        i32.shl
                                                        local.set 1
                                                        local.get 4
                                                        local.get 22
                                                        i32.const 2
                                                        i32.shl
                                                        i32.add
                                                        local.set 22
                                                        block  ;; label = @27
                                                          block  ;; label = @28
                                                            block  ;; label = @29
                                                              local.get 13
                                                              i32.eqz
                                                              br_if 0 (;@29;)
                                                              local.get 22
                                                              i32.load
                                                              local.get 1
                                                              i32.and
                                                              br_if 0 (;@29;)
                                                              i32.const 0
                                                              local.set 1
                                                              br 1 (;@28;)
                                                            end
                                                            block  ;; label = @29
                                                              local.get 26
                                                              i32.eqz
                                                              br_if 0 (;@29;)
                                                              local.get 22
                                                              i32.load offset=28
                                                              local.get 1
                                                              i32.and
                                                              br_if 0 (;@29;)
                                                              i32.const 16777216
                                                              local.set 1
                                                              br 1 (;@28;)
                                                            end
                                                            local.get 24
                                                            br_if 1 (;@27;)
                                                            local.get 22
                                                            i32.load offset=56
                                                            local.get 1
                                                            i32.and
                                                            br_if 1 (;@27;)
                                                            i32.const 33554432
                                                            local.set 1
                                                          end
                                                          local.get 1
                                                          local.get 23
                                                          i32.const 16711680
                                                          i32.and
                                                          i32.or
                                                          local.set 13
                                                          br 5 (;@22;)
                                                        end
                                                        block  ;; label = @27
                                                          local.get 30
                                                          local.get 14
                                                          i32.ne
                                                          br_if 0 (;@27;)
                                                          local.get 50
                                                          local.set 16
                                                          br 6 (;@21;)
                                                        end
                                                        local.get 23
                                                        i32.const 65536
                                                        i32.add
                                                        local.set 23
                                                        local.get 14
                                                        i32.const 1
                                                        i32.add
                                                        local.tee 14
                                                        i32.const 5
                                                        i32.shr_u
                                                        local.tee 22
                                                        i32.const 7
                                                        i32.ne
                                                        br_if 0 (;@26;)
                                                      end
                                                    end
                                                    i32.const 7
                                                    i32.const 7
                                                    i32.const 1049936
                                                    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                                    unreachable
                                                  end
                                                  i32.const 0
                                                  local.set 13
                                                  i32.const 7
                                                  local.set 1
                                                end
                                                local.get 1
                                                i32.const 2
                                                i32.and
                                                local.set 30
                                                local.get 1
                                                i32.const 1
                                                i32.and
                                                local.set 51
                                                local.get 14
                                                local.set 31
                                                block  ;; label = @23
                                                  loop  ;; label = @24
                                                    block  ;; label = @25
                                                      block  ;; label = @26
                                                        block  ;; label = @27
                                                          block  ;; label = @28
                                                            local.get 14
                                                            i32.const 5
                                                            i32.shr_u
                                                            local.tee 24
                                                            i32.const 7
                                                            i32.eq
                                                            br_if 0 (;@28;)
                                                            local.get 14
                                                            i32.const 195
                                                            local.get 14
                                                            i32.const 195
                                                            i32.gt_u
                                                            select
                                                            local.set 32
                                                            i32.const 1
                                                            local.get 14
                                                            i32.shl
                                                            local.set 1
                                                            local.get 4
                                                            local.get 24
                                                            i32.const 2
                                                            i32.shl
                                                            local.tee 28
                                                            i32.add
                                                            local.set 23
                                                            block  ;; label = @29
                                                              local.get 51
                                                              i32.eqz
                                                              br_if 0 (;@29;)
                                                              i32.const 1
                                                              local.set 26
                                                              local.get 14
                                                              local.set 22
                                                              local.get 23
                                                              i32.load
                                                              local.get 1
                                                              i32.and
                                                              br_if 2 (;@27;)
                                                              i32.const 0
                                                              local.set 25
                                                              local.get 14
                                                              local.set 32
                                                              br 4 (;@25;)
                                                            end
                                                            block  ;; label = @29
                                                              block  ;; label = @30
                                                                local.get 30
                                                                i32.eqz
                                                                br_if 0 (;@30;)
                                                                i32.const 1
                                                                local.set 26
                                                                i32.const 16777216
                                                                local.set 25
                                                                local.get 14
                                                                local.set 22
                                                                local.get 23
                                                                i32.load offset=28
                                                                local.get 1
                                                                i32.and
                                                                br_if 1 (;@29;)
                                                                local.get 14
                                                                local.set 32
                                                                br 5 (;@25;)
                                                              end
                                                              i32.const 33554432
                                                              local.set 25
                                                              block  ;; label = @30
                                                                local.get 4
                                                                local.get 28
                                                                i32.add
                                                                i32.load offset=56
                                                                local.get 1
                                                                i32.and
                                                                br_if 0 (;@30;)
                                                                i32.const 1
                                                                local.set 26
                                                                local.get 14
                                                                local.set 32
                                                                br 5 (;@25;)
                                                              end
                                                              local.get 14
                                                              local.set 1
                                                              loop  ;; label = @30
                                                                local.get 1
                                                                i32.const 195
                                                                i32.lt_u
                                                                local.set 26
                                                                block  ;; label = @31
                                                                  local.get 1
                                                                  i32.const 194
                                                                  i32.le_u
                                                                  br_if 0 (;@31;)
                                                                  i32.const 50331648
                                                                  local.set 25
                                                                  br 6 (;@25;)
                                                                end
                                                                local.get 4
                                                                local.get 1
                                                                i32.const 1
                                                                i32.add
                                                                local.tee 1
                                                                i32.const 3
                                                                i32.shr_u
                                                                i32.const 536870908
                                                                i32.and
                                                                i32.add
                                                                i32.load offset=56
                                                                local.get 1
                                                                i32.shr_u
                                                                i32.const 1
                                                                i32.and
                                                                br_if 0 (;@30;)
                                                              end
                                                              local.get 1
                                                              local.set 32
                                                              br 4 (;@25;)
                                                            end
                                                            loop  ;; label = @29
                                                              block  ;; label = @30
                                                                local.get 4
                                                                local.get 24
                                                                i32.const 2
                                                                i32.shl
                                                                i32.add
                                                                i32.load offset=56
                                                                local.get 1
                                                                i32.and
                                                                br_if 0 (;@30;)
                                                                i32.const 33554432
                                                                local.set 25
                                                                local.get 22
                                                                local.set 32
                                                                br 5 (;@25;)
                                                              end
                                                              local.get 22
                                                              i32.const 194
                                                              i32.gt_u
                                                              br_if 3 (;@26;)
                                                              local.get 4
                                                              local.get 22
                                                              i32.const 1
                                                              i32.add
                                                              local.tee 22
                                                              i32.const 5
                                                              i32.shr_u
                                                              local.tee 24
                                                              i32.const 2
                                                              i32.shl
                                                              i32.add
                                                              i32.load offset=28
                                                              i32.const 1
                                                              local.get 22
                                                              i32.shl
                                                              local.tee 1
                                                              i32.and
                                                              br_if 0 (;@29;)
                                                            end
                                                            local.get 22
                                                            local.set 32
                                                            i32.const 1
                                                            local.set 26
                                                            br 3 (;@25;)
                                                          end
                                                          i32.const 7
                                                          i32.const 7
                                                          i32.const 1049936
                                                          call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                                          unreachable
                                                        end
                                                        block  ;; label = @27
                                                          block  ;; label = @28
                                                            loop  ;; label = @29
                                                              block  ;; label = @30
                                                                local.get 30
                                                                i32.eqz
                                                                br_if 0 (;@30;)
                                                                local.get 23
                                                                i32.load offset=28
                                                                local.get 1
                                                                i32.and
                                                                br_if 0 (;@30;)
                                                                i32.const 16777216
                                                                local.set 25
                                                                br 3 (;@27;)
                                                              end
                                                              local.get 23
                                                              i32.load offset=56
                                                              local.get 1
                                                              i32.and
                                                              i32.eqz
                                                              br_if 1 (;@28;)
                                                              local.get 22
                                                              i32.const 194
                                                              i32.gt_u
                                                              br_if 3 (;@26;)
                                                              local.get 4
                                                              local.get 22
                                                              i32.const 1
                                                              i32.add
                                                              local.tee 22
                                                              i32.const 3
                                                              i32.shr_u
                                                              i32.const 536870908
                                                              i32.and
                                                              i32.add
                                                              local.tee 23
                                                              i32.load
                                                              i32.const 1
                                                              local.get 22
                                                              i32.shl
                                                              local.tee 1
                                                              i32.and
                                                              br_if 0 (;@29;)
                                                            end
                                                            i32.const 0
                                                            local.set 25
                                                            local.get 22
                                                            local.set 32
                                                            i32.const 1
                                                            local.set 26
                                                            br 3 (;@25;)
                                                          end
                                                          i32.const 33554432
                                                          local.set 25
                                                        end
                                                        local.get 22
                                                        local.set 32
                                                        br 1 (;@25;)
                                                      end
                                                      i32.const 0
                                                      local.set 26
                                                      i32.const 50331648
                                                      local.set 25
                                                    end
                                                    i32.const 0
                                                    local.set 24
                                                    local.get 14
                                                    local.set 1
                                                    block  ;; label = @25
                                                      block  ;; label = @26
                                                        block  ;; label = @27
                                                          local.get 4
                                                          local.get 28
                                                          i32.add
                                                          local.tee 22
                                                          i32.load
                                                          i32.const 1
                                                          local.get 14
                                                          i32.shl
                                                          local.tee 23
                                                          i32.and
                                                          br_if 0 (;@27;)
                                                          local.get 14
                                                          local.set 1
                                                          br 1 (;@26;)
                                                        end
                                                        block  ;; label = @27
                                                          loop  ;; label = @28
                                                            block  ;; label = @29
                                                              local.get 13
                                                              br_if 0 (;@29;)
                                                              block  ;; label = @30
                                                                local.get 22
                                                                i32.load offset=28
                                                                local.get 23
                                                                i32.and
                                                                br_if 0 (;@30;)
                                                                i32.const 256
                                                                local.set 24
                                                                br 4 (;@26;)
                                                              end
                                                              local.get 22
                                                              i32.load offset=56
                                                              local.get 23
                                                              i32.and
                                                              i32.eqz
                                                              br_if 2 (;@27;)
                                                            end
                                                            local.get 1
                                                            i32.const 194
                                                            i32.gt_u
                                                            br_if 3 (;@25;)
                                                            local.get 4
                                                            local.get 1
                                                            i32.const 1
                                                            i32.add
                                                            local.tee 1
                                                            i32.const 3
                                                            i32.shr_u
                                                            i32.const 536870908
                                                            i32.and
                                                            i32.add
                                                            local.tee 22
                                                            i32.load
                                                            i32.const 1
                                                            local.get 1
                                                            i32.shl
                                                            local.tee 23
                                                            i32.and
                                                            i32.eqz
                                                            br_if 2 (;@26;)
                                                            br 0 (;@28;)
                                                          end
                                                        end
                                                        i32.const 512
                                                        local.set 24
                                                      end
                                                      local.get 26
                                                      local.get 32
                                                      i32.const 255
                                                      i32.and
                                                      local.get 1
                                                      i32.const 255
                                                      i32.and
                                                      i32.eq
                                                      i32.and
                                                      br_if 2 (;@23;)
                                                    end
                                                    local.get 14
                                                    i32.const 1
                                                    i32.add
                                                    local.set 14
                                                    local.get 31
                                                    i32.const 255
                                                    i32.and
                                                    local.set 1
                                                    local.get 31
                                                    i32.const 1
                                                    i32.add
                                                    local.set 31
                                                    local.get 1
                                                    i32.const 195
                                                    i32.lt_u
                                                    br_if 0 (;@24;)
                                                  end
                                                  local.get 50
                                                  local.set 16
                                                  br 2 (;@21;)
                                                end
                                                local.get 32
                                                i32.const 16
                                                i32.shl
                                                i32.const 16711680
                                                i32.and
                                                local.get 25
                                                i32.or
                                                local.get 24
                                                i32.or
                                                i32.const 1
                                                i32.or
                                                local.set 13
                                              end
                                              local.get 2
                                              local.get 13
                                              i32.store offset=428
                                              i64.const 0
                                              local.set 11
                                              i64.const 0
                                              local.set 16
                                              block  ;; label = @22
                                                block  ;; label = @23
                                                  block  ;; label = @24
                                                    block  ;; label = @25
                                                      block  ;; label = @26
                                                        block  ;; label = @27
                                                          block  ;; label = @28
                                                            block  ;; label = @29
                                                              block  ;; label = @30
                                                                block  ;; label = @31
                                                                  block  ;; label = @32
                                                                    block  ;; label = @33
                                                                      block  ;; label = @34
                                                                        block  ;; label = @35
                                                                          block  ;; label = @36
                                                                            block  ;; label = @37
                                                                              block  ;; label = @38
                                                                                block  ;; label = @39
                                                                                  block  ;; label = @40
                                                                                    block  ;; label = @41
                                                                                      block  ;; label = @42
                                                                                        local.get 49
                                                                                        br_table 0 (;@42;) 3 (;@39;) 4 (;@38;) 5 (;@37;) 6 (;@36;) 1 (;@41;) 2 (;@40;) 9 (;@33;) 10 (;@32;) 19 (;@23;) 11 (;@31;) 0 (;@42;)
                                                                                      end
                                                                                      local.get 2
                                                                                      i32.const 8
                                                                                      i32.add
                                                                                      local.get 2
                                                                                      i32.const 236
                                                                                      i32.add
                                                                                      i32.const 0
                                                                                      local.get 2
                                                                                      i32.const 428
                                                                                      i32.add
                                                                                      call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                                                                                      block  ;; label = @42
                                                                                        block  ;; label = @43
                                                                                          local.get 2
                                                                                          i32.load8_u offset=8
                                                                                          i32.eqz
                                                                                          br_if 0 (;@43;)
                                                                                          local.get 52
                                                                                          i64.const -256
                                                                                          i64.and
                                                                                          i64.const 9
                                                                                          i64.or
                                                                                          local.set 17
                                                                                          br 1 (;@42;)
                                                                                        end
                                                                                        local.get 2
                                                                                        local.get 2
                                                                                        i32.load8_u offset=9
                                                                                        local.tee 1
                                                                                        i64.extend_i32_u
                                                                                        i64.const 255
                                                                                        i64.and
                                                                                        i64.const 8
                                                                                        i64.shl
                                                                                        i64.const 1
                                                                                        i64.or
                                                                                        local.tee 17
                                                                                        i64.store offset=432
                                                                                        local.get 2
                                                                                        local.get 2
                                                                                        i32.const 236
                                                                                        i32.add
                                                                                        i32.const 0
                                                                                        i32.const 1
                                                                                        local.get 2
                                                                                        i32.const 432
                                                                                        i32.add
                                                                                        i32.const 1
                                                                                        local.get 1
                                                                                        local.get 2
                                                                                        i32.const 428
                                                                                        i32.add
                                                                                        call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                                                        block  ;; label = @43
                                                                                          local.get 2
                                                                                          i32.load8_u
                                                                                          i32.eqz
                                                                                          br_if 0 (;@43;)
                                                                                          local.get 52
                                                                                          i64.const -256
                                                                                          i64.and
                                                                                          i64.const 9
                                                                                          i64.or
                                                                                          local.set 17
                                                                                          br 1 (;@42;)
                                                                                        end
                                                                                        local.get 2
                                                                                        i32.load8_u offset=1
                                                                                        local.set 53
                                                                                        local.get 1
                                                                                        local.set 54
                                                                                      end
                                                                                      local.get 17
                                                                                      i64.const 255
                                                                                      i64.and
                                                                                      i64.const 9
                                                                                      i64.eq
                                                                                      br_if 11 (;@30;)
                                                                                      local.get 17
                                                                                      local.set 52
                                                                                      local.get 53
                                                                                      local.set 22
                                                                                      local.get 54
                                                                                      local.set 14
                                                                                      local.get 17
                                                                                      local.set 16
                                                                                      br 18 (;@23;)
                                                                                    end
                                                                                    local.get 2
                                                                                    i32.const 80
                                                                                    i32.add
                                                                                    local.get 2
                                                                                    i32.const 236
                                                                                    i32.add
                                                                                    i32.const 5
                                                                                    local.get 2
                                                                                    i32.const 428
                                                                                    i32.add
                                                                                    call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                                                                                    block  ;; label = @41
                                                                                      block  ;; label = @42
                                                                                        local.get 2
                                                                                        i32.load8_u offset=80
                                                                                        i32.eqz
                                                                                        br_if 0 (;@42;)
                                                                                        local.get 55
                                                                                        i64.const -256
                                                                                        i64.and
                                                                                        i64.const 9
                                                                                        i64.or
                                                                                        local.set 17
                                                                                        br 1 (;@41;)
                                                                                      end
                                                                                      local.get 2
                                                                                      local.get 2
                                                                                      i32.load8_u offset=81
                                                                                      local.tee 1
                                                                                      i64.extend_i32_u
                                                                                      i64.const 255
                                                                                      i64.and
                                                                                      i64.const 8
                                                                                      i64.shl
                                                                                      i64.const 4
                                                                                      i64.or
                                                                                      local.tee 17
                                                                                      i64.store offset=432
                                                                                      local.get 2
                                                                                      i32.const 72
                                                                                      i32.add
                                                                                      local.get 2
                                                                                      i32.const 236
                                                                                      i32.add
                                                                                      i32.const 5
                                                                                      i32.const 1
                                                                                      local.get 2
                                                                                      i32.const 432
                                                                                      i32.add
                                                                                      i32.const 1
                                                                                      local.get 1
                                                                                      local.get 2
                                                                                      i32.const 428
                                                                                      i32.add
                                                                                      call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                                                      block  ;; label = @42
                                                                                        local.get 2
                                                                                        i32.load8_u offset=72
                                                                                        i32.eqz
                                                                                        br_if 0 (;@42;)
                                                                                        local.get 55
                                                                                        i64.const -256
                                                                                        i64.and
                                                                                        i64.const 9
                                                                                        i64.or
                                                                                        local.set 17
                                                                                        br 1 (;@41;)
                                                                                      end
                                                                                      local.get 2
                                                                                      i32.load8_u offset=73
                                                                                      local.set 56
                                                                                      local.get 1
                                                                                      local.set 57
                                                                                    end
                                                                                    local.get 17
                                                                                    i64.const 255
                                                                                    i64.and
                                                                                    i64.const 9
                                                                                    i64.eq
                                                                                    br_if 5 (;@35;)
                                                                                    local.get 17
                                                                                    local.set 55
                                                                                    local.get 56
                                                                                    local.set 22
                                                                                    local.get 57
                                                                                    local.set 14
                                                                                    local.get 17
                                                                                    local.set 16
                                                                                    br 17 (;@23;)
                                                                                  end
                                                                                  local.get 2
                                                                                  i32.const 96
                                                                                  i32.add
                                                                                  local.get 2
                                                                                  i32.const 236
                                                                                  i32.add
                                                                                  i32.const 6
                                                                                  local.get 2
                                                                                  i32.const 428
                                                                                  i32.add
                                                                                  call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                                                                                  block  ;; label = @40
                                                                                    block  ;; label = @41
                                                                                      local.get 2
                                                                                      i32.load8_u offset=96
                                                                                      i32.eqz
                                                                                      br_if 0 (;@41;)
                                                                                      local.get 58
                                                                                      i64.const -256
                                                                                      i64.and
                                                                                      i64.const 9
                                                                                      i64.or
                                                                                      local.set 17
                                                                                      br 1 (;@40;)
                                                                                    end
                                                                                    local.get 2
                                                                                    local.get 2
                                                                                    i32.load8_u offset=97
                                                                                    local.tee 1
                                                                                    i64.extend_i32_u
                                                                                    i64.const 255
                                                                                    i64.and
                                                                                    i64.const 8
                                                                                    i64.shl
                                                                                    i64.const 6
                                                                                    i64.or
                                                                                    local.tee 17
                                                                                    i64.store offset=432
                                                                                    local.get 2
                                                                                    i32.const 88
                                                                                    i32.add
                                                                                    local.get 2
                                                                                    i32.const 236
                                                                                    i32.add
                                                                                    i32.const 6
                                                                                    i32.const 1
                                                                                    local.get 2
                                                                                    i32.const 432
                                                                                    i32.add
                                                                                    i32.const 1
                                                                                    local.get 1
                                                                                    local.get 2
                                                                                    i32.const 428
                                                                                    i32.add
                                                                                    call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                                                    block  ;; label = @41
                                                                                      local.get 2
                                                                                      i32.load8_u offset=88
                                                                                      i32.eqz
                                                                                      br_if 0 (;@41;)
                                                                                      local.get 58
                                                                                      i64.const -256
                                                                                      i64.and
                                                                                      i64.const 9
                                                                                      i64.or
                                                                                      local.set 17
                                                                                      br 1 (;@40;)
                                                                                    end
                                                                                    local.get 2
                                                                                    i32.load8_u offset=89
                                                                                    local.set 59
                                                                                    local.get 1
                                                                                    local.set 60
                                                                                  end
                                                                                  local.get 17
                                                                                  i64.const 255
                                                                                  i64.and
                                                                                  i64.const 9
                                                                                  i64.eq
                                                                                  br_if 5 (;@34;)
                                                                                  local.get 17
                                                                                  local.set 58
                                                                                  local.get 59
                                                                                  local.set 22
                                                                                  local.get 60
                                                                                  local.set 14
                                                                                  local.get 17
                                                                                  local.set 16
                                                                                  br 16 (;@23;)
                                                                                end
                                                                                local.get 2
                                                                                i32.load offset=236
                                                                                br_if 13 (;@25;)
                                                                                local.get 27
                                                                                local.get 27
                                                                                i64.load offset=32
                                                                                local.tee 16
                                                                                i64.const 1
                                                                                i64.add
                                                                                i64.store offset=32
                                                                                local.get 2
                                                                                local.get 16
                                                                                local.get 27
                                                                                i64.load offset=24
                                                                                i64.xor
                                                                                local.tee 17
                                                                                i64.const 16
                                                                                i64.rotl
                                                                                local.get 17
                                                                                local.get 27
                                                                                i64.load offset=16
                                                                                i64.add
                                                                                local.tee 17
                                                                                i64.xor
                                                                                local.tee 18
                                                                                i64.const 21
                                                                                i64.rotl
                                                                                local.get 18
                                                                                local.get 27
                                                                                i64.load offset=8
                                                                                local.tee 19
                                                                                local.get 27
                                                                                i64.load
                                                                                i64.add
                                                                                local.tee 20
                                                                                i64.const 32
                                                                                i64.rotl
                                                                                i64.add
                                                                                local.tee 18
                                                                                i64.xor
                                                                                local.tee 21
                                                                                i64.const 16
                                                                                i64.rotl
                                                                                local.get 17
                                                                                local.get 19
                                                                                i64.const 13
                                                                                i64.rotl
                                                                                local.get 20
                                                                                i64.xor
                                                                                local.tee 19
                                                                                i64.add
                                                                                local.tee 17
                                                                                i64.const 32
                                                                                i64.rotl
                                                                                i64.const 255
                                                                                i64.xor
                                                                                local.get 21
                                                                                i64.add
                                                                                local.tee 20
                                                                                i64.xor
                                                                                local.tee 21
                                                                                i64.const 21
                                                                                i64.rotl
                                                                                local.get 18
                                                                                local.get 16
                                                                                i64.xor
                                                                                local.get 17
                                                                                local.get 19
                                                                                i64.const 17
                                                                                i64.rotl
                                                                                i64.xor
                                                                                local.tee 16
                                                                                i64.add
                                                                                local.tee 17
                                                                                i64.const 32
                                                                                i64.rotl
                                                                                local.get 21
                                                                                i64.add
                                                                                local.tee 18
                                                                                i64.xor
                                                                                local.tee 19
                                                                                i64.const 16
                                                                                i64.rotl
                                                                                local.get 17
                                                                                local.get 16
                                                                                i64.const 13
                                                                                i64.rotl
                                                                                i64.xor
                                                                                local.tee 16
                                                                                local.get 20
                                                                                i64.add
                                                                                local.tee 17
                                                                                i64.const 32
                                                                                i64.rotl
                                                                                local.get 19
                                                                                i64.add
                                                                                local.tee 19
                                                                                i64.xor
                                                                                local.tee 20
                                                                                i64.const 21
                                                                                i64.rotl
                                                                                local.get 16
                                                                                i64.const 17
                                                                                i64.rotl
                                                                                local.get 17
                                                                                i64.xor
                                                                                local.tee 16
                                                                                local.get 18
                                                                                i64.add
                                                                                local.tee 17
                                                                                i64.const 32
                                                                                i64.rotl
                                                                                local.get 20
                                                                                i64.add
                                                                                local.tee 18
                                                                                i64.xor
                                                                                local.tee 20
                                                                                i64.const 16
                                                                                i64.rotl
                                                                                local.get 16
                                                                                i64.const 13
                                                                                i64.rotl
                                                                                local.get 17
                                                                                i64.xor
                                                                                local.tee 16
                                                                                local.get 19
                                                                                i64.add
                                                                                local.tee 17
                                                                                i64.const 32
                                                                                i64.rotl
                                                                                local.get 20
                                                                                i64.add
                                                                                local.tee 19
                                                                                i64.xor
                                                                                i64.const 21
                                                                                i64.rotl
                                                                                local.get 16
                                                                                i64.const 17
                                                                                i64.rotl
                                                                                local.get 17
                                                                                i64.xor
                                                                                local.tee 16
                                                                                i64.const 13
                                                                                i64.rotl
                                                                                local.get 16
                                                                                local.get 18
                                                                                i64.add
                                                                                i64.xor
                                                                                local.tee 16
                                                                                i64.const 17
                                                                                i64.rotl
                                                                                i64.xor
                                                                                local.get 16
                                                                                local.get 19
                                                                                i64.add
                                                                                local.tee 16
                                                                                i64.const 32
                                                                                i64.rotl
                                                                                i64.xor
                                                                                local.get 16
                                                                                i64.xor
                                                                                local.tee 16
                                                                                i64.store32 offset=240
                                                                                local.get 16
                                                                                i64.const 32
                                                                                i64.shr_u
                                                                                i32.wrap_i64
                                                                                local.set 1
                                                                                i32.const 1
                                                                                local.set 14
                                                                                br 14 (;@24;)
                                                                              end
                                                                              local.get 2
                                                                              i32.load offset=236
                                                                              br_if 10 (;@27;)
                                                                              local.get 27
                                                                              local.get 27
                                                                              i64.load offset=32
                                                                              local.tee 16
                                                                              i64.const 1
                                                                              i64.add
                                                                              i64.store offset=32
                                                                              local.get 2
                                                                              local.get 16
                                                                              local.get 27
                                                                              i64.load offset=24
                                                                              i64.xor
                                                                              local.tee 17
                                                                              i64.const 16
                                                                              i64.rotl
                                                                              local.get 17
                                                                              local.get 27
                                                                              i64.load offset=16
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.xor
                                                                              local.tee 18
                                                                              i64.const 21
                                                                              i64.rotl
                                                                              local.get 18
                                                                              local.get 27
                                                                              i64.load offset=8
                                                                              local.tee 19
                                                                              local.get 27
                                                                              i64.load
                                                                              i64.add
                                                                              local.tee 20
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              i64.add
                                                                              local.tee 18
                                                                              i64.xor
                                                                              local.tee 21
                                                                              i64.const 16
                                                                              i64.rotl
                                                                              local.get 17
                                                                              local.get 19
                                                                              i64.const 13
                                                                              i64.rotl
                                                                              local.get 20
                                                                              i64.xor
                                                                              local.tee 19
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              i64.const 255
                                                                              i64.xor
                                                                              local.get 21
                                                                              i64.add
                                                                              local.tee 20
                                                                              i64.xor
                                                                              local.tee 21
                                                                              i64.const 21
                                                                              i64.rotl
                                                                              local.get 18
                                                                              local.get 16
                                                                              i64.xor
                                                                              local.get 17
                                                                              local.get 19
                                                                              i64.const 17
                                                                              i64.rotl
                                                                              i64.xor
                                                                              local.tee 16
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              local.get 21
                                                                              i64.add
                                                                              local.tee 18
                                                                              i64.xor
                                                                              local.tee 19
                                                                              i64.const 16
                                                                              i64.rotl
                                                                              local.get 17
                                                                              local.get 16
                                                                              i64.const 13
                                                                              i64.rotl
                                                                              i64.xor
                                                                              local.tee 16
                                                                              local.get 20
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              local.get 19
                                                                              i64.add
                                                                              local.tee 19
                                                                              i64.xor
                                                                              local.tee 20
                                                                              i64.const 21
                                                                              i64.rotl
                                                                              local.get 16
                                                                              i64.const 17
                                                                              i64.rotl
                                                                              local.get 17
                                                                              i64.xor
                                                                              local.tee 16
                                                                              local.get 18
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              local.get 20
                                                                              i64.add
                                                                              local.tee 18
                                                                              i64.xor
                                                                              local.tee 20
                                                                              i64.const 16
                                                                              i64.rotl
                                                                              local.get 16
                                                                              i64.const 13
                                                                              i64.rotl
                                                                              local.get 17
                                                                              i64.xor
                                                                              local.tee 16
                                                                              local.get 19
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              local.get 20
                                                                              i64.add
                                                                              local.tee 19
                                                                              i64.xor
                                                                              i64.const 21
                                                                              i64.rotl
                                                                              local.get 16
                                                                              i64.const 17
                                                                              i64.rotl
                                                                              local.get 17
                                                                              i64.xor
                                                                              local.tee 16
                                                                              i64.const 13
                                                                              i64.rotl
                                                                              local.get 16
                                                                              local.get 18
                                                                              i64.add
                                                                              i64.xor
                                                                              local.tee 16
                                                                              i64.const 17
                                                                              i64.rotl
                                                                              i64.xor
                                                                              local.get 16
                                                                              local.get 19
                                                                              i64.add
                                                                              local.tee 16
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              i64.xor
                                                                              local.get 16
                                                                              i64.xor
                                                                              local.tee 16
                                                                              i64.store32 offset=240
                                                                              local.get 16
                                                                              i64.const 32
                                                                              i64.shr_u
                                                                              i32.wrap_i64
                                                                              local.set 1
                                                                              i32.const 1
                                                                              local.set 14
                                                                              br 11 (;@26;)
                                                                            end
                                                                            local.get 2
                                                                            i32.load offset=236
                                                                            br_if 7 (;@29;)
                                                                            local.get 27
                                                                            local.get 27
                                                                            i64.load offset=32
                                                                            local.tee 16
                                                                            i64.const 1
                                                                            i64.add
                                                                            i64.store offset=32
                                                                            local.get 2
                                                                            local.get 16
                                                                            local.get 27
                                                                            i64.load offset=24
                                                                            i64.xor
                                                                            local.tee 17
                                                                            i64.const 16
                                                                            i64.rotl
                                                                            local.get 17
                                                                            local.get 27
                                                                            i64.load offset=16
                                                                            i64.add
                                                                            local.tee 17
                                                                            i64.xor
                                                                            local.tee 18
                                                                            i64.const 21
                                                                            i64.rotl
                                                                            local.get 18
                                                                            local.get 27
                                                                            i64.load offset=8
                                                                            local.tee 19
                                                                            local.get 27
                                                                            i64.load
                                                                            i64.add
                                                                            local.tee 20
                                                                            i64.const 32
                                                                            i64.rotl
                                                                            i64.add
                                                                            local.tee 18
                                                                            i64.xor
                                                                            local.tee 21
                                                                            i64.const 16
                                                                            i64.rotl
                                                                            local.get 17
                                                                            local.get 19
                                                                            i64.const 13
                                                                            i64.rotl
                                                                            local.get 20
                                                                            i64.xor
                                                                            local.tee 19
                                                                            i64.add
                                                                            local.tee 17
                                                                            i64.const 32
                                                                            i64.rotl
                                                                            i64.const 255
                                                                            i64.xor
                                                                            local.get 21
                                                                            i64.add
                                                                            local.tee 20
                                                                            i64.xor
                                                                            local.tee 21
                                                                            i64.const 21
                                                                            i64.rotl
                                                                            local.get 18
                                                                            local.get 16
                                                                            i64.xor
                                                                            local.get 17
                                                                            local.get 19
                                                                            i64.const 17
                                                                            i64.rotl
                                                                            i64.xor
                                                                            local.tee 16
                                                                            i64.add
                                                                            local.tee 17
                                                                            i64.const 32
                                                                            i64.rotl
                                                                            local.get 21
                                                                            i64.add
                                                                            local.tee 18
                                                                            i64.xor
                                                                            local.tee 19
                                                                            i64.const 16
                                                                            i64.rotl
                                                                            local.get 17
                                                                            local.get 16
                                                                            i64.const 13
                                                                            i64.rotl
                                                                            i64.xor
                                                                            local.tee 16
                                                                            local.get 20
                                                                            i64.add
                                                                            local.tee 17
                                                                            i64.const 32
                                                                            i64.rotl
                                                                            local.get 19
                                                                            i64.add
                                                                            local.tee 19
                                                                            i64.xor
                                                                            local.tee 20
                                                                            i64.const 21
                                                                            i64.rotl
                                                                            local.get 16
                                                                            i64.const 17
                                                                            i64.rotl
                                                                            local.get 17
                                                                            i64.xor
                                                                            local.tee 16
                                                                            local.get 18
                                                                            i64.add
                                                                            local.tee 17
                                                                            i64.const 32
                                                                            i64.rotl
                                                                            local.get 20
                                                                            i64.add
                                                                            local.tee 18
                                                                            i64.xor
                                                                            local.tee 20
                                                                            i64.const 16
                                                                            i64.rotl
                                                                            local.get 16
                                                                            i64.const 13
                                                                            i64.rotl
                                                                            local.get 17
                                                                            i64.xor
                                                                            local.tee 16
                                                                            local.get 19
                                                                            i64.add
                                                                            local.tee 17
                                                                            i64.const 32
                                                                            i64.rotl
                                                                            local.get 20
                                                                            i64.add
                                                                            local.tee 19
                                                                            i64.xor
                                                                            i64.const 21
                                                                            i64.rotl
                                                                            local.get 16
                                                                            i64.const 17
                                                                            i64.rotl
                                                                            local.get 17
                                                                            i64.xor
                                                                            local.tee 16
                                                                            i64.const 13
                                                                            i64.rotl
                                                                            local.get 16
                                                                            local.get 18
                                                                            i64.add
                                                                            i64.xor
                                                                            local.tee 16
                                                                            i64.const 17
                                                                            i64.rotl
                                                                            i64.xor
                                                                            local.get 16
                                                                            local.get 19
                                                                            i64.add
                                                                            local.tee 16
                                                                            i64.const 32
                                                                            i64.rotl
                                                                            i64.xor
                                                                            local.get 16
                                                                            i64.xor
                                                                            local.tee 16
                                                                            i64.store32 offset=240
                                                                            local.get 16
                                                                            i64.const 32
                                                                            i64.shr_u
                                                                            i32.wrap_i64
                                                                            local.set 14
                                                                            i32.const 1
                                                                            local.set 1
                                                                            br 8 (;@28;)
                                                                          end
                                                                          local.get 2
                                                                          i32.load offset=240
                                                                          local.set 23
                                                                          local.get 2
                                                                          i32.load offset=236
                                                                          local.set 14
                                                                          loop  ;; label = @36
                                                                            local.get 14
                                                                            i32.const 1
                                                                            i32.and
                                                                            local.set 22
                                                                            i32.const 0
                                                                            local.set 14
                                                                            local.get 23
                                                                            local.set 1
                                                                            block  ;; label = @37
                                                                              local.get 22
                                                                              br_if 0 (;@37;)
                                                                              local.get 27
                                                                              local.get 27
                                                                              i64.load offset=32
                                                                              local.tee 16
                                                                              i64.const 1
                                                                              i64.add
                                                                              i64.store offset=32
                                                                              local.get 16
                                                                              local.get 27
                                                                              i64.load offset=24
                                                                              i64.xor
                                                                              local.tee 17
                                                                              i64.const 16
                                                                              i64.rotl
                                                                              local.get 17
                                                                              local.get 27
                                                                              i64.load offset=16
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.xor
                                                                              local.tee 18
                                                                              i64.const 21
                                                                              i64.rotl
                                                                              local.get 18
                                                                              local.get 27
                                                                              i64.load offset=8
                                                                              local.tee 19
                                                                              local.get 27
                                                                              i64.load
                                                                              i64.add
                                                                              local.tee 20
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              i64.add
                                                                              local.tee 18
                                                                              i64.xor
                                                                              local.tee 21
                                                                              i64.const 16
                                                                              i64.rotl
                                                                              local.get 17
                                                                              local.get 19
                                                                              i64.const 13
                                                                              i64.rotl
                                                                              local.get 20
                                                                              i64.xor
                                                                              local.tee 19
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              i64.const 255
                                                                              i64.xor
                                                                              local.get 21
                                                                              i64.add
                                                                              local.tee 20
                                                                              i64.xor
                                                                              local.tee 21
                                                                              i64.const 21
                                                                              i64.rotl
                                                                              local.get 18
                                                                              local.get 16
                                                                              i64.xor
                                                                              local.get 17
                                                                              local.get 19
                                                                              i64.const 17
                                                                              i64.rotl
                                                                              i64.xor
                                                                              local.tee 16
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              local.get 21
                                                                              i64.add
                                                                              local.tee 18
                                                                              i64.xor
                                                                              local.tee 19
                                                                              i64.const 16
                                                                              i64.rotl
                                                                              local.get 17
                                                                              local.get 16
                                                                              i64.const 13
                                                                              i64.rotl
                                                                              i64.xor
                                                                              local.tee 16
                                                                              local.get 20
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              local.get 19
                                                                              i64.add
                                                                              local.tee 19
                                                                              i64.xor
                                                                              local.tee 20
                                                                              i64.const 21
                                                                              i64.rotl
                                                                              local.get 16
                                                                              i64.const 17
                                                                              i64.rotl
                                                                              local.get 17
                                                                              i64.xor
                                                                              local.tee 16
                                                                              local.get 18
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              local.get 20
                                                                              i64.add
                                                                              local.tee 18
                                                                              i64.xor
                                                                              local.tee 20
                                                                              i64.const 16
                                                                              i64.rotl
                                                                              local.get 16
                                                                              i64.const 13
                                                                              i64.rotl
                                                                              local.get 17
                                                                              i64.xor
                                                                              local.tee 16
                                                                              local.get 19
                                                                              i64.add
                                                                              local.tee 17
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              local.get 20
                                                                              i64.add
                                                                              local.tee 19
                                                                              i64.xor
                                                                              i64.const 21
                                                                              i64.rotl
                                                                              local.get 16
                                                                              i64.const 17
                                                                              i64.rotl
                                                                              local.get 17
                                                                              i64.xor
                                                                              local.tee 16
                                                                              i64.const 13
                                                                              i64.rotl
                                                                              local.get 16
                                                                              local.get 18
                                                                              i64.add
                                                                              i64.xor
                                                                              local.tee 16
                                                                              i64.const 17
                                                                              i64.rotl
                                                                              i64.xor
                                                                              local.get 16
                                                                              local.get 19
                                                                              i64.add
                                                                              local.tee 16
                                                                              i64.const 32
                                                                              i64.rotl
                                                                              i64.xor
                                                                              local.get 16
                                                                              i64.xor
                                                                              local.tee 16
                                                                              i32.wrap_i64
                                                                              local.set 23
                                                                              local.get 16
                                                                              i64.const 32
                                                                              i64.shr_u
                                                                              i32.wrap_i64
                                                                              local.set 1
                                                                              i32.const 1
                                                                              local.set 14
                                                                            end
                                                                            local.get 1
                                                                            i32.eqz
                                                                            br_if 0 (;@36;)
                                                                          end
                                                                          local.get 2
                                                                          local.get 23
                                                                          i32.store offset=240
                                                                          local.get 2
                                                                          local.get 14
                                                                          i32.store offset=236
                                                                          local.get 2
                                                                          i32.const 5
                                                                          i32.store8 offset=432
                                                                          local.get 2
                                                                          i32.const 64
                                                                          i32.add
                                                                          local.get 2
                                                                          i32.const 236
                                                                          i32.add
                                                                          i32.const 4
                                                                          i32.const 1
                                                                          local.get 2
                                                                          i32.const 432
                                                                          i32.add
                                                                          i32.const 0
                                                                          local.get 4
                                                                          local.get 2
                                                                          i32.const 428
                                                                          i32.add
                                                                          call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                                          block  ;; label = @36
                                                                            local.get 2
                                                                            i32.load8_u offset=64
                                                                            i32.eqz
                                                                            br_if 0 (;@36;)
                                                                            local.get 50
                                                                            i64.const -256
                                                                            i64.and
                                                                            i64.const 9
                                                                            i64.or
                                                                            local.set 16
                                                                            br 14 (;@22;)
                                                                          end
                                                                          local.get 2
                                                                          i32.load8_u offset=65
                                                                          local.set 22
                                                                          i64.const 5
                                                                          local.set 16
                                                                          br 12 (;@23;)
                                                                        end
                                                                        local.get 50
                                                                        i64.const -256
                                                                        i64.and
                                                                        i64.const 9
                                                                        i64.or
                                                                        local.set 16
                                                                        local.get 17
                                                                        local.set 55
                                                                        br 12 (;@22;)
                                                                      end
                                                                      local.get 50
                                                                      i64.const -256
                                                                      i64.and
                                                                      i64.const 9
                                                                      i64.or
                                                                      local.set 16
                                                                      local.get 17
                                                                      local.set 58
                                                                      br 11 (;@22;)
                                                                    end
                                                                    local.get 2
                                                                    i32.load offset=240
                                                                    local.set 23
                                                                    local.get 2
                                                                    i32.load offset=236
                                                                    local.set 14
                                                                    loop  ;; label = @33
                                                                      local.get 14
                                                                      i32.const 1
                                                                      i32.and
                                                                      local.set 22
                                                                      i32.const 0
                                                                      local.set 14
                                                                      local.get 23
                                                                      local.set 1
                                                                      block  ;; label = @34
                                                                        local.get 22
                                                                        br_if 0 (;@34;)
                                                                        local.get 27
                                                                        local.get 27
                                                                        i64.load offset=32
                                                                        local.tee 16
                                                                        i64.const 1
                                                                        i64.add
                                                                        i64.store offset=32
                                                                        local.get 16
                                                                        local.get 27
                                                                        i64.load offset=24
                                                                        i64.xor
                                                                        local.tee 17
                                                                        i64.const 16
                                                                        i64.rotl
                                                                        local.get 17
                                                                        local.get 27
                                                                        i64.load offset=16
                                                                        i64.add
                                                                        local.tee 17
                                                                        i64.xor
                                                                        local.tee 18
                                                                        i64.const 21
                                                                        i64.rotl
                                                                        local.get 18
                                                                        local.get 27
                                                                        i64.load offset=8
                                                                        local.tee 19
                                                                        local.get 27
                                                                        i64.load
                                                                        i64.add
                                                                        local.tee 20
                                                                        i64.const 32
                                                                        i64.rotl
                                                                        i64.add
                                                                        local.tee 18
                                                                        i64.xor
                                                                        local.tee 21
                                                                        i64.const 16
                                                                        i64.rotl
                                                                        local.get 17
                                                                        local.get 19
                                                                        i64.const 13
                                                                        i64.rotl
                                                                        local.get 20
                                                                        i64.xor
                                                                        local.tee 19
                                                                        i64.add
                                                                        local.tee 17
                                                                        i64.const 32
                                                                        i64.rotl
                                                                        i64.const 255
                                                                        i64.xor
                                                                        local.get 21
                                                                        i64.add
                                                                        local.tee 20
                                                                        i64.xor
                                                                        local.tee 21
                                                                        i64.const 21
                                                                        i64.rotl
                                                                        local.get 18
                                                                        local.get 16
                                                                        i64.xor
                                                                        local.get 17
                                                                        local.get 19
                                                                        i64.const 17
                                                                        i64.rotl
                                                                        i64.xor
                                                                        local.tee 16
                                                                        i64.add
                                                                        local.tee 17
                                                                        i64.const 32
                                                                        i64.rotl
                                                                        local.get 21
                                                                        i64.add
                                                                        local.tee 18
                                                                        i64.xor
                                                                        local.tee 19
                                                                        i64.const 16
                                                                        i64.rotl
                                                                        local.get 17
                                                                        local.get 16
                                                                        i64.const 13
                                                                        i64.rotl
                                                                        i64.xor
                                                                        local.tee 16
                                                                        local.get 20
                                                                        i64.add
                                                                        local.tee 17
                                                                        i64.const 32
                                                                        i64.rotl
                                                                        local.get 19
                                                                        i64.add
                                                                        local.tee 19
                                                                        i64.xor
                                                                        local.tee 20
                                                                        i64.const 21
                                                                        i64.rotl
                                                                        local.get 16
                                                                        i64.const 17
                                                                        i64.rotl
                                                                        local.get 17
                                                                        i64.xor
                                                                        local.tee 16
                                                                        local.get 18
                                                                        i64.add
                                                                        local.tee 17
                                                                        i64.const 32
                                                                        i64.rotl
                                                                        local.get 20
                                                                        i64.add
                                                                        local.tee 18
                                                                        i64.xor
                                                                        local.tee 20
                                                                        i64.const 16
                                                                        i64.rotl
                                                                        local.get 16
                                                                        i64.const 13
                                                                        i64.rotl
                                                                        local.get 17
                                                                        i64.xor
                                                                        local.tee 16
                                                                        local.get 19
                                                                        i64.add
                                                                        local.tee 17
                                                                        i64.const 32
                                                                        i64.rotl
                                                                        local.get 20
                                                                        i64.add
                                                                        local.tee 19
                                                                        i64.xor
                                                                        i64.const 21
                                                                        i64.rotl
                                                                        local.get 16
                                                                        i64.const 17
                                                                        i64.rotl
                                                                        local.get 17
                                                                        i64.xor
                                                                        local.tee 16
                                                                        i64.const 13
                                                                        i64.rotl
                                                                        local.get 16
                                                                        local.get 18
                                                                        i64.add
                                                                        i64.xor
                                                                        local.tee 16
                                                                        i64.const 17
                                                                        i64.rotl
                                                                        i64.xor
                                                                        local.get 16
                                                                        local.get 19
                                                                        i64.add
                                                                        local.tee 16
                                                                        i64.const 32
                                                                        i64.rotl
                                                                        i64.xor
                                                                        local.get 16
                                                                        i64.xor
                                                                        local.tee 16
                                                                        i32.wrap_i64
                                                                        local.set 23
                                                                        local.get 16
                                                                        i64.const 32
                                                                        i64.shr_u
                                                                        i32.wrap_i64
                                                                        local.set 1
                                                                        i32.const 1
                                                                        local.set 14
                                                                      end
                                                                      local.get 1
                                                                      i32.eqz
                                                                      br_if 0 (;@33;)
                                                                    end
                                                                    local.get 2
                                                                    local.get 23
                                                                    i32.store offset=240
                                                                    local.get 2
                                                                    local.get 14
                                                                    i32.store offset=236
                                                                    local.get 2
                                                                    i32.const 7
                                                                    i32.store8 offset=432
                                                                    local.get 2
                                                                    i32.const 104
                                                                    i32.add
                                                                    local.get 2
                                                                    i32.const 236
                                                                    i32.add
                                                                    i32.const 7
                                                                    i32.const 1
                                                                    local.get 2
                                                                    i32.const 432
                                                                    i32.add
                                                                    i32.const 0
                                                                    local.get 4
                                                                    local.get 2
                                                                    i32.const 428
                                                                    i32.add
                                                                    call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                                    block  ;; label = @33
                                                                      local.get 2
                                                                      i32.load8_u offset=104
                                                                      i32.eqz
                                                                      br_if 0 (;@33;)
                                                                      local.get 50
                                                                      i64.const -256
                                                                      i64.and
                                                                      i64.const 9
                                                                      i64.or
                                                                      local.set 16
                                                                      br 11 (;@22;)
                                                                    end
                                                                    local.get 2
                                                                    i32.load8_u offset=105
                                                                    local.set 22
                                                                    i64.const 7
                                                                    local.set 16
                                                                    br 9 (;@23;)
                                                                  end
                                                                  local.get 2
                                                                  i32.load offset=240
                                                                  local.set 23
                                                                  local.get 2
                                                                  i32.load offset=236
                                                                  local.set 1
                                                                  loop  ;; label = @32
                                                                    local.get 1
                                                                    i32.const 1
                                                                    i32.and
                                                                    local.set 22
                                                                    i32.const 0
                                                                    local.set 1
                                                                    local.get 23
                                                                    local.set 14
                                                                    block  ;; label = @33
                                                                      local.get 22
                                                                      br_if 0 (;@33;)
                                                                      local.get 27
                                                                      local.get 27
                                                                      i64.load offset=32
                                                                      local.tee 16
                                                                      i64.const 1
                                                                      i64.add
                                                                      i64.store offset=32
                                                                      local.get 16
                                                                      local.get 27
                                                                      i64.load offset=24
                                                                      i64.xor
                                                                      local.tee 17
                                                                      i64.const 16
                                                                      i64.rotl
                                                                      local.get 17
                                                                      local.get 27
                                                                      i64.load offset=16
                                                                      i64.add
                                                                      local.tee 17
                                                                      i64.xor
                                                                      local.tee 18
                                                                      i64.const 21
                                                                      i64.rotl
                                                                      local.get 18
                                                                      local.get 27
                                                                      i64.load offset=8
                                                                      local.tee 19
                                                                      local.get 27
                                                                      i64.load
                                                                      i64.add
                                                                      local.tee 20
                                                                      i64.const 32
                                                                      i64.rotl
                                                                      i64.add
                                                                      local.tee 18
                                                                      i64.xor
                                                                      local.tee 21
                                                                      i64.const 16
                                                                      i64.rotl
                                                                      local.get 17
                                                                      local.get 19
                                                                      i64.const 13
                                                                      i64.rotl
                                                                      local.get 20
                                                                      i64.xor
                                                                      local.tee 19
                                                                      i64.add
                                                                      local.tee 17
                                                                      i64.const 32
                                                                      i64.rotl
                                                                      i64.const 255
                                                                      i64.xor
                                                                      local.get 21
                                                                      i64.add
                                                                      local.tee 20
                                                                      i64.xor
                                                                      local.tee 21
                                                                      i64.const 21
                                                                      i64.rotl
                                                                      local.get 18
                                                                      local.get 16
                                                                      i64.xor
                                                                      local.get 17
                                                                      local.get 19
                                                                      i64.const 17
                                                                      i64.rotl
                                                                      i64.xor
                                                                      local.tee 16
                                                                      i64.add
                                                                      local.tee 17
                                                                      i64.const 32
                                                                      i64.rotl
                                                                      local.get 21
                                                                      i64.add
                                                                      local.tee 18
                                                                      i64.xor
                                                                      local.tee 19
                                                                      i64.const 16
                                                                      i64.rotl
                                                                      local.get 17
                                                                      local.get 16
                                                                      i64.const 13
                                                                      i64.rotl
                                                                      i64.xor
                                                                      local.tee 16
                                                                      local.get 20
                                                                      i64.add
                                                                      local.tee 17
                                                                      i64.const 32
                                                                      i64.rotl
                                                                      local.get 19
                                                                      i64.add
                                                                      local.tee 19
                                                                      i64.xor
                                                                      local.tee 20
                                                                      i64.const 21
                                                                      i64.rotl
                                                                      local.get 16
                                                                      i64.const 17
                                                                      i64.rotl
                                                                      local.get 17
                                                                      i64.xor
                                                                      local.tee 16
                                                                      local.get 18
                                                                      i64.add
                                                                      local.tee 17
                                                                      i64.const 32
                                                                      i64.rotl
                                                                      local.get 20
                                                                      i64.add
                                                                      local.tee 18
                                                                      i64.xor
                                                                      local.tee 20
                                                                      i64.const 16
                                                                      i64.rotl
                                                                      local.get 16
                                                                      i64.const 13
                                                                      i64.rotl
                                                                      local.get 17
                                                                      i64.xor
                                                                      local.tee 16
                                                                      local.get 19
                                                                      i64.add
                                                                      local.tee 17
                                                                      i64.const 32
                                                                      i64.rotl
                                                                      local.get 20
                                                                      i64.add
                                                                      local.tee 19
                                                                      i64.xor
                                                                      i64.const 21
                                                                      i64.rotl
                                                                      local.get 16
                                                                      i64.const 17
                                                                      i64.rotl
                                                                      local.get 17
                                                                      i64.xor
                                                                      local.tee 16
                                                                      i64.const 13
                                                                      i64.rotl
                                                                      local.get 16
                                                                      local.get 18
                                                                      i64.add
                                                                      i64.xor
                                                                      local.tee 16
                                                                      i64.const 17
                                                                      i64.rotl
                                                                      i64.xor
                                                                      local.get 16
                                                                      local.get 19
                                                                      i64.add
                                                                      local.tee 16
                                                                      i64.const 32
                                                                      i64.rotl
                                                                      i64.xor
                                                                      local.get 16
                                                                      i64.xor
                                                                      local.tee 16
                                                                      i32.wrap_i64
                                                                      local.set 23
                                                                      local.get 16
                                                                      i64.const 32
                                                                      i64.shr_u
                                                                      i32.wrap_i64
                                                                      local.set 14
                                                                      i32.const 1
                                                                      local.set 1
                                                                    end
                                                                    local.get 14
                                                                    i32.const 63
                                                                    i32.and
                                                                    local.tee 14
                                                                    i32.eqz
                                                                    br_if 0 (;@32;)
                                                                  end
                                                                  local.get 2
                                                                  local.get 23
                                                                  i32.store offset=240
                                                                  local.get 2
                                                                  local.get 1
                                                                  i32.store offset=236
                                                                  local.get 2
                                                                  i32.const 8
                                                                  i32.store8 offset=432
                                                                  local.get 2
                                                                  i32.const 112
                                                                  i32.add
                                                                  local.get 2
                                                                  i32.const 236
                                                                  i32.add
                                                                  i32.const 8
                                                                  i32.const 1
                                                                  local.get 2
                                                                  i32.const 432
                                                                  i32.add
                                                                  i32.const 0
                                                                  local.get 4
                                                                  local.get 2
                                                                  i32.const 428
                                                                  i32.add
                                                                  call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                                  block  ;; label = @32
                                                                    local.get 2
                                                                    i32.load8_u offset=112
                                                                    i32.eqz
                                                                    br_if 0 (;@32;)
                                                                    local.get 50
                                                                    i64.const -256
                                                                    i64.and
                                                                    i64.const 9
                                                                    i64.or
                                                                    local.set 16
                                                                    br 10 (;@22;)
                                                                  end
                                                                  local.get 2
                                                                  i32.load8_u offset=113
                                                                  local.set 22
                                                                  i64.const 8
                                                                  local.set 16
                                                                  br 8 (;@23;)
                                                                end
                                                                i32.const 0
                                                                local.set 1
                                                                i32.const 0
                                                                local.set 14
                                                                loop  ;; label = @31
                                                                  block  ;; label = @32
                                                                    block  ;; label = @33
                                                                      local.get 29
                                                                      i32.eqz
                                                                      br_if 0 (;@33;)
                                                                      local.get 2
                                                                      local.get 29
                                                                      i32.const -1
                                                                      i32.add
                                                                      local.tee 29
                                                                      i32.store offset=248
                                                                      local.get 5
                                                                      local.get 29
                                                                      i32.add
                                                                      i32.load8_u
                                                                      local.set 22
                                                                      br 1 (;@32;)
                                                                    end
                                                                    local.get 27
                                                                    local.get 27
                                                                    i64.load offset=32
                                                                    local.tee 16
                                                                    i64.const 1
                                                                    i64.add
                                                                    i64.store offset=32
                                                                    local.get 2
                                                                    local.get 16
                                                                    local.get 27
                                                                    i64.load offset=24
                                                                    i64.xor
                                                                    local.tee 17
                                                                    i64.const 16
                                                                    i64.rotl
                                                                    local.get 17
                                                                    local.get 27
                                                                    i64.load offset=16
                                                                    i64.add
                                                                    local.tee 17
                                                                    i64.xor
                                                                    local.tee 18
                                                                    i64.const 21
                                                                    i64.rotl
                                                                    local.get 18
                                                                    local.get 27
                                                                    i64.load offset=8
                                                                    local.tee 19
                                                                    local.get 27
                                                                    i64.load
                                                                    i64.add
                                                                    local.tee 20
                                                                    i64.const 32
                                                                    i64.rotl
                                                                    i64.add
                                                                    local.tee 18
                                                                    i64.xor
                                                                    local.tee 21
                                                                    i64.const 16
                                                                    i64.rotl
                                                                    local.get 17
                                                                    local.get 19
                                                                    i64.const 13
                                                                    i64.rotl
                                                                    local.get 20
                                                                    i64.xor
                                                                    local.tee 19
                                                                    i64.add
                                                                    local.tee 17
                                                                    i64.const 32
                                                                    i64.rotl
                                                                    i64.const 255
                                                                    i64.xor
                                                                    local.get 21
                                                                    i64.add
                                                                    local.tee 20
                                                                    i64.xor
                                                                    local.tee 21
                                                                    i64.const 21
                                                                    i64.rotl
                                                                    local.get 18
                                                                    local.get 16
                                                                    i64.xor
                                                                    local.get 17
                                                                    local.get 19
                                                                    i64.const 17
                                                                    i64.rotl
                                                                    i64.xor
                                                                    local.tee 16
                                                                    i64.add
                                                                    local.tee 17
                                                                    i64.const 32
                                                                    i64.rotl
                                                                    local.get 21
                                                                    i64.add
                                                                    local.tee 18
                                                                    i64.xor
                                                                    local.tee 19
                                                                    i64.const 16
                                                                    i64.rotl
                                                                    local.get 17
                                                                    local.get 16
                                                                    i64.const 13
                                                                    i64.rotl
                                                                    i64.xor
                                                                    local.tee 16
                                                                    local.get 20
                                                                    i64.add
                                                                    local.tee 17
                                                                    i64.const 32
                                                                    i64.rotl
                                                                    local.get 19
                                                                    i64.add
                                                                    local.tee 19
                                                                    i64.xor
                                                                    local.tee 20
                                                                    i64.const 21
                                                                    i64.rotl
                                                                    local.get 16
                                                                    i64.const 17
                                                                    i64.rotl
                                                                    local.get 17
                                                                    i64.xor
                                                                    local.tee 16
                                                                    local.get 18
                                                                    i64.add
                                                                    local.tee 17
                                                                    i64.const 32
                                                                    i64.rotl
                                                                    local.get 20
                                                                    i64.add
                                                                    local.tee 18
                                                                    i64.xor
                                                                    local.tee 20
                                                                    i64.const 16
                                                                    i64.rotl
                                                                    local.get 16
                                                                    i64.const 13
                                                                    i64.rotl
                                                                    local.get 17
                                                                    i64.xor
                                                                    local.tee 16
                                                                    local.get 19
                                                                    i64.add
                                                                    local.tee 17
                                                                    i64.const 32
                                                                    i64.rotl
                                                                    local.get 20
                                                                    i64.add
                                                                    local.tee 19
                                                                    i64.xor
                                                                    i64.const 21
                                                                    i64.rotl
                                                                    local.get 16
                                                                    i64.const 17
                                                                    i64.rotl
                                                                    local.get 17
                                                                    i64.xor
                                                                    local.tee 16
                                                                    i64.const 13
                                                                    i64.rotl
                                                                    local.get 16
                                                                    local.get 18
                                                                    i64.add
                                                                    i64.xor
                                                                    local.tee 16
                                                                    i64.const 17
                                                                    i64.rotl
                                                                    i64.xor
                                                                    local.get 16
                                                                    local.get 19
                                                                    i64.add
                                                                    local.tee 16
                                                                    i64.const 32
                                                                    i64.rotl
                                                                    i64.xor
                                                                    local.get 16
                                                                    i64.xor
                                                                    local.tee 16
                                                                    i64.store32 offset=252
                                                                    local.get 8
                                                                    local.get 16
                                                                    i64.const 48
                                                                    i64.shr_u
                                                                    i64.store8
                                                                    local.get 9
                                                                    local.get 16
                                                                    i64.const 32
                                                                    i64.shr_u
                                                                    i64.store16
                                                                    i32.const 7
                                                                    local.set 29
                                                                    local.get 2
                                                                    i32.const 7
                                                                    i32.store offset=248
                                                                    local.get 16
                                                                    i64.const 56
                                                                    i64.shr_u
                                                                    i32.wrap_i64
                                                                    local.set 22
                                                                  end
                                                                  i32.const 0
                                                                  i32.const 1
                                                                  local.get 22
                                                                  i32.shl
                                                                  local.tee 22
                                                                  local.get 22
                                                                  local.get 1
                                                                  i32.and
                                                                  local.tee 22
                                                                  select
                                                                  local.get 1
                                                                  i32.or
                                                                  local.set 1
                                                                  local.get 14
                                                                  local.get 22
                                                                  i32.eqz
                                                                  i32.add
                                                                  local.tee 14
                                                                  i32.const 4
                                                                  i32.lt_u
                                                                  br_if 0 (;@31;)
                                                                end
                                                                i64.const 0
                                                                local.set 16
                                                                br 7 (;@23;)
                                                              end
                                                              local.get 50
                                                              i64.const -256
                                                              i64.and
                                                              i64.const 9
                                                              i64.or
                                                              local.set 16
                                                              local.get 17
                                                              local.set 52
                                                              br 7 (;@22;)
                                                            end
                                                            i32.const 0
                                                            local.set 1
                                                            local.get 2
                                                            i32.load offset=240
                                                            local.set 14
                                                          end
                                                          local.get 2
                                                          local.get 1
                                                          i32.store offset=236
                                                          local.get 2
                                                          i32.const 56
                                                          i32.add
                                                          local.get 2
                                                          i32.const 236
                                                          i32.add
                                                          i32.const 3
                                                          local.get 2
                                                          i32.const 428
                                                          i32.add
                                                          call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                                                          block  ;; label = @28
                                                            block  ;; label = @29
                                                              local.get 2
                                                              i32.load8_u offset=56
                                                              i32.eqz
                                                              br_if 0 (;@29;)
                                                              local.get 61
                                                              i64.const -256
                                                              i64.and
                                                              i64.const 9
                                                              i64.or
                                                              local.set 17
                                                              br 1 (;@28;)
                                                            end
                                                            local.get 2
                                                            local.get 2
                                                            i32.load8_u offset=57
                                                            local.tee 1
                                                            i64.extend_i32_u
                                                            i64.const 255
                                                            i64.and
                                                            i64.const 8
                                                            i64.shl
                                                            i64.const 4
                                                            i64.or
                                                            local.tee 17
                                                            i64.store offset=432
                                                            local.get 2
                                                            i32.const 48
                                                            i32.add
                                                            local.get 2
                                                            i32.const 236
                                                            i32.add
                                                            i32.const 3
                                                            i32.const 1
                                                            local.get 2
                                                            i32.const 432
                                                            i32.add
                                                            i32.const 1
                                                            local.get 1
                                                            local.get 2
                                                            i32.const 428
                                                            i32.add
                                                            call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                            block  ;; label = @29
                                                              local.get 2
                                                              i32.load8_u offset=48
                                                              i32.eqz
                                                              br_if 0 (;@29;)
                                                              local.get 61
                                                              i64.const -256
                                                              i64.and
                                                              i64.const 9
                                                              i64.or
                                                              local.set 17
                                                              br 1 (;@28;)
                                                            end
                                                            local.get 2
                                                            i32.load8_u offset=49
                                                            local.set 62
                                                            local.get 1
                                                            local.set 63
                                                          end
                                                          block  ;; label = @28
                                                            local.get 17
                                                            i64.const 255
                                                            i64.and
                                                            i64.const 9
                                                            i64.ne
                                                            br_if 0 (;@28;)
                                                            local.get 50
                                                            i64.const -256
                                                            i64.and
                                                            i64.const 9
                                                            i64.or
                                                            local.set 16
                                                            local.get 17
                                                            local.set 61
                                                            br 6 (;@22;)
                                                          end
                                                          local.get 14
                                                          i32.const 24
                                                          i32.shl
                                                          i32.const 50331648
                                                          i32.and
                                                          i64.extend_i32_u
                                                          local.set 11
                                                          local.get 17
                                                          local.set 61
                                                          local.get 62
                                                          local.set 22
                                                          local.get 63
                                                          local.set 14
                                                          local.get 17
                                                          local.set 16
                                                          br 4 (;@23;)
                                                        end
                                                        i32.const 0
                                                        local.set 14
                                                        local.get 2
                                                        i32.load offset=240
                                                        local.set 1
                                                      end
                                                      local.get 2
                                                      local.get 14
                                                      i32.store offset=236
                                                      local.get 2
                                                      local.get 1
                                                      i32.store offset=436
                                                      local.get 2
                                                      i32.const 3
                                                      i32.store8 offset=432
                                                      local.get 2
                                                      i32.const 40
                                                      i32.add
                                                      local.get 2
                                                      i32.const 236
                                                      i32.add
                                                      i32.const 2
                                                      local.get 2
                                                      i32.const 428
                                                      i32.add
                                                      call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                                                      block  ;; label = @26
                                                        block  ;; label = @27
                                                          local.get 2
                                                          i32.load8_u offset=40
                                                          br_if 0 (;@27;)
                                                          local.get 2
                                                          i32.const 32
                                                          i32.add
                                                          local.get 2
                                                          i32.const 236
                                                          i32.add
                                                          i32.const 2
                                                          i32.const 1
                                                          local.get 2
                                                          i32.const 432
                                                          i32.add
                                                          i32.const 1
                                                          local.get 2
                                                          i32.load8_u offset=41
                                                          local.tee 14
                                                          local.get 2
                                                          i32.const 428
                                                          i32.add
                                                          call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                          local.get 2
                                                          i32.load8_u offset=32
                                                          i32.eqz
                                                          br_if 1 (;@26;)
                                                        end
                                                        local.get 50
                                                        i64.const -256
                                                        i64.and
                                                        i64.const 9
                                                        i64.or
                                                        local.set 16
                                                        br 4 (;@22;)
                                                      end
                                                      local.get 2
                                                      i32.load8_u offset=33
                                                      local.set 22
                                                      local.get 1
                                                      i64.extend_i32_u
                                                      i64.const 32
                                                      i64.shl
                                                      i64.const 3
                                                      i64.or
                                                      local.set 16
                                                      br 2 (;@23;)
                                                    end
                                                    i32.const 0
                                                    local.set 14
                                                    local.get 2
                                                    i32.load offset=240
                                                    local.set 1
                                                  end
                                                  local.get 2
                                                  local.get 14
                                                  i32.store offset=236
                                                  local.get 2
                                                  local.get 1
                                                  i32.store offset=436
                                                  local.get 2
                                                  i32.const 2
                                                  i32.store8 offset=432
                                                  local.get 2
                                                  i32.const 24
                                                  i32.add
                                                  local.get 2
                                                  i32.const 236
                                                  i32.add
                                                  i32.const 1
                                                  local.get 2
                                                  i32.const 428
                                                  i32.add
                                                  call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_src_reg17h01d36aca73eb0b5eE
                                                  block  ;; label = @24
                                                    block  ;; label = @25
                                                      local.get 2
                                                      i32.load8_u offset=24
                                                      br_if 0 (;@25;)
                                                      local.get 2
                                                      i32.const 16
                                                      i32.add
                                                      local.get 2
                                                      i32.const 236
                                                      i32.add
                                                      i32.const 1
                                                      i32.const 1
                                                      local.get 2
                                                      i32.const 432
                                                      i32.add
                                                      i32.const 1
                                                      local.get 2
                                                      i32.load8_u offset=25
                                                      local.tee 14
                                                      local.get 2
                                                      i32.const 428
                                                      i32.add
                                                      call $_ZN5hashx9generator18Generator$LT$R$GT$14choose_dst_reg17ha98dce9648929bf2E
                                                      local.get 2
                                                      i32.load8_u offset=16
                                                      i32.eqz
                                                      br_if 1 (;@24;)
                                                    end
                                                    local.get 50
                                                    i64.const -256
                                                    i64.and
                                                    i64.const 9
                                                    i64.or
                                                    local.set 16
                                                    br 2 (;@22;)
                                                  end
                                                  local.get 2
                                                  i32.load8_u offset=17
                                                  local.set 22
                                                  local.get 1
                                                  i64.extend_i32_u
                                                  i64.const 32
                                                  i64.shl
                                                  i64.const 2
                                                  i64.or
                                                  local.set 16
                                                end
                                                local.get 11
                                                local.get 22
                                                i64.extend_i32_u
                                                i64.const 255
                                                i64.and
                                                i64.const 8
                                                i64.shl
                                                local.get 15
                                                i64.extend_i32_u
                                                i64.const 255
                                                i64.and
                                                i64.or
                                                local.get 14
                                                i64.extend_i32_u
                                                i64.const 255
                                                i64.and
                                                i64.const 16
                                                i64.shl
                                                i64.or
                                                i64.or
                                                local.get 1
                                                i64.extend_i32_u
                                                i64.const 32
                                                i64.shl
                                                i64.or
                                                local.set 64
                                              end
                                              local.get 16
                                              i64.const 255
                                              i64.and
                                              i64.const 9
                                              i64.eq
                                              br_if 0 (;@21;)
                                              local.get 13
                                              i32.const 24
                                              i32.shr_u
                                              local.set 1
                                              local.get 13
                                              i32.const 50331647
                                              i32.gt_u
                                              br_if 6 (;@15;)
                                              local.get 13
                                              i32.const 16
                                              i32.shr_u
                                              local.tee 14
                                              i32.const 255
                                              i32.and
                                              local.tee 23
                                              i32.const 5
                                              i32.shr_u
                                              local.tee 22
                                              i32.const 7
                                              i32.eq
                                              br_if 7 (;@14;)
                                              local.get 4
                                              local.get 1
                                              i32.const 28
                                              i32.mul
                                              i32.add
                                              local.get 22
                                              i32.const 2
                                              i32.shl
                                              local.tee 22
                                              i32.add
                                              local.tee 1
                                              local.get 1
                                              i32.load
                                              i32.const 1
                                              local.get 14
                                              i32.shl
                                              local.tee 1
                                              i32.or
                                              i32.store
                                              block  ;; label = @22
                                                local.get 13
                                                i32.const 1
                                                i32.and
                                                i32.eqz
                                                br_if 0 (;@22;)
                                                local.get 13
                                                i32.const 8
                                                i32.shr_u
                                                i32.const 255
                                                i32.and
                                                local.tee 14
                                                i32.const 3
                                                i32.ge_u
                                                br_if 9 (;@13;)
                                                local.get 4
                                                local.get 14
                                                i32.const 28
                                                i32.mul
                                                i32.add
                                                local.get 22
                                                i32.add
                                                local.tee 14
                                                local.get 14
                                                i32.load
                                                local.get 1
                                                i32.or
                                                i32.store
                                              end
                                              i32.const 3
                                              local.set 1
                                              local.get 16
                                              local.set 47
                                              local.get 64
                                              local.set 48
                                              block  ;; label = @22
                                                block  ;; label = @23
                                                  block  ;; label = @24
                                                    local.get 64
                                                    i32.wrap_i64
                                                    local.tee 14
                                                    i32.const 255
                                                    i32.and
                                                    br_table 2 (;@22;) 0 (;@24;) 0 (;@24;) 1 (;@23;) 1 (;@23;) 1 (;@23;) 1 (;@23;) 1 (;@23;) 1 (;@23;) 3 (;@21;) 3 (;@21;) 2 (;@22;)
                                                  end
                                                  i32.const 4
                                                  local.set 1
                                                  br 1 (;@22;)
                                                end
                                                i32.const 1
                                                local.set 1
                                              end
                                              local.get 1
                                              local.get 23
                                              i32.add
                                              local.tee 1
                                              i32.const 196
                                              i32.ge_u
                                              br_if 9 (;@12;)
                                              local.get 14
                                              i32.const 8
                                              i32.shr_u
                                              i32.const 255
                                              i32.and
                                              local.set 14
                                              local.get 64
                                              i64.const 63488
                                              i64.and
                                              i64.const 0
                                              i64.ne
                                              br_if 10 (;@11;)
                                              local.get 6
                                              local.get 14
                                              i32.add
                                              local.get 1
                                              i32.store8
                                              local.get 16
                                              local.set 47
                                              local.get 64
                                              local.set 48
                                            end
                                            local.get 47
                                            i64.const 255
                                            i64.and
                                            i64.const 9
                                            i64.eq
                                            br_if 1 (;@19;)
                                            local.get 16
                                            local.set 50
                                          end
                                          local.get 48
                                          i32.wrap_i64
                                          local.tee 1
                                          i32.const 255
                                          i32.and
                                          local.tee 14
                                          i32.const 2
                                          i32.gt_u
                                          br_if 1 (;@18;)
                                          local.get 2
                                          local.get 2
                                          i32.load offset=324
                                          i32.const 1
                                          i32.add
                                          i32.store offset=324
                                          br 2 (;@17;)
                                        end
                                        local.get 2
                                        i32.load16_u offset=420
                                        local.tee 1
                                        i32.const 583
                                        i32.gt_u
                                        br_if 11 (;@7;)
                                        local.get 1
                                        i32.const 3
                                        i32.add
                                        local.tee 1
                                        i32.const 65535
                                        i32.and
                                        i32.const 3
                                        i32.div_u
                                        local.tee 14
                                        i32.const 255
                                        i32.and
                                        i32.const 191
                                        i32.gt_u
                                        br_if 11 (;@7;)
                                        local.get 2
                                        local.get 14
                                        i32.store8 offset=422
                                        local.get 2
                                        local.get 1
                                        i32.store16 offset=420
                                        local.get 12
                                        local.set 11
                                        local.get 16
                                        local.set 50
                                        br 15 (;@3;)
                                      end
                                      local.get 14
                                      i32.const 8
                                      i32.gt_u
                                      br_if 1 (;@16;)
                                    end
                                    local.get 1
                                    i32.const 8
                                    i32.shr_u
                                    i32.const 255
                                    i32.and
                                    local.set 1
                                    local.get 48
                                    i64.const 63488
                                    i64.and
                                    i64.const 0
                                    i64.ne
                                    br_if 6 (;@10;)
                                    local.get 7
                                    local.get 1
                                    i32.const 3
                                    i32.shl
                                    i32.add
                                    local.get 47
                                    i64.store align=4
                                  end
                                  local.get 2
                                  i32.load16_u offset=420
                                  local.set 1
                                  block  ;; label = @16
                                    block  ;; label = @17
                                      i32.const 1
                                      local.get 14
                                      i32.shl
                                      i32.const 505
                                      i32.and
                                      i32.eqz
                                      br_if 0 (;@17;)
                                      local.get 1
                                      i32.const 65535
                                      i32.and
                                      i32.const 585
                                      i32.gt_u
                                      br_if 1 (;@16;)
                                      local.get 1
                                      i32.const 1
                                      i32.add
                                      local.tee 1
                                      i32.const 65535
                                      i32.and
                                      i32.const 3
                                      i32.div_u
                                      local.tee 14
                                      i32.const 255
                                      i32.and
                                      i32.const 192
                                      i32.ge_u
                                      br_if 1 (;@16;)
                                      br 8 (;@9;)
                                    end
                                    local.get 1
                                    i32.const 65535
                                    i32.and
                                    i32.const 584
                                    i32.gt_u
                                    br_if 0 (;@16;)
                                    local.get 1
                                    i32.const 2
                                    i32.add
                                    local.tee 1
                                    i32.const 65535
                                    i32.and
                                    i32.const 3
                                    i32.div_u
                                    local.tee 14
                                    i32.const 255
                                    i32.and
                                    i32.const 192
                                    i32.lt_u
                                    br_if 7 (;@9;)
                                  end
                                  local.get 3
                                  local.get 10
                                  i32.const 3
                                  i32.shl
                                  i32.add
                                  local.get 48
                                  i64.store align=4
                                  local.get 10
                                  i32.const 511
                                  i32.eq
                                  br_if 7 (;@8;)
                                  br 8 (;@7;)
                                end
                                local.get 1
                                i32.const 3
                                i32.const 1049952
                                call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                unreachable
                              end
                              i32.const 7
                              i32.const 7
                              i32.const 1049968
                              call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                              unreachable
                            end
                            local.get 14
                            i32.const 3
                            i32.const 1049952
                            call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                            unreachable
                          end
                          i32.const 1049984
                          i32.const 44
                          local.get 2
                          i32.const 447
                          i32.add
                          i32.const 1049816
                          i32.const 1050028
                          call $_RNvNtCsgXGp5Oqx2Ny_4core6result13unwrap_failed
                          unreachable
                        end
                        local.get 14
                        i32.const 8
                        i32.const 1050044
                        call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                        unreachable
                      end
                      local.get 1
                      i32.const 8
                      i32.const 1049800
                      call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                      unreachable
                    end
                    local.get 3
                    local.get 10
                    i32.const 3
                    i32.shl
                    i32.add
                    local.get 48
                    i64.store align=4
                    local.get 2
                    local.get 14
                    i32.store8 offset=422
                    local.get 2
                    local.get 1
                    i32.store16 offset=420
                    local.get 10
                    i32.const 1
                    i32.add
                    local.tee 10
                    i32.const 512
                    i32.ne
                    br_if 6 (;@2;)
                  end
                  local.get 2
                  i32.load8_u offset=328
                  local.tee 4
                  local.get 2
                  i32.load8_u offset=329
                  local.tee 1
                  local.get 4
                  local.get 1
                  i32.gt_u
                  select
                  local.tee 4
                  local.get 2
                  i32.load8_u offset=330
                  local.tee 1
                  local.get 4
                  local.get 1
                  i32.gt_u
                  select
                  local.tee 4
                  local.get 2
                  i32.load8_u offset=331
                  local.tee 1
                  local.get 4
                  local.get 1
                  i32.gt_u
                  select
                  local.tee 4
                  local.get 2
                  i32.load8_u offset=332
                  local.tee 1
                  local.get 4
                  local.get 1
                  i32.gt_u
                  select
                  local.tee 4
                  local.get 2
                  i32.load8_u offset=333
                  local.tee 1
                  local.get 4
                  local.get 1
                  i32.gt_u
                  select
                  local.tee 4
                  local.get 2
                  i32.load8_u offset=334
                  local.tee 1
                  local.get 4
                  local.get 1
                  i32.gt_u
                  select
                  local.tee 4
                  local.get 2
                  i32.load8_u offset=335
                  local.tee 1
                  local.get 4
                  local.get 1
                  i32.gt_u
                  select
                  i32.const 194
                  i32.ne
                  br_if 0 (;@7;)
                  local.get 2
                  i32.load offset=324
                  i32.const 192
                  i32.ne
                  br_if 0 (;@7;)
                  local.get 0
                  i32.const 2
                  i32.store
                  local.get 0
                  local.get 3
                  i32.store offset=4
                  br 1 (;@6;)
                end
                local.get 0
                i32.const 0
                i32.store
                local.get 3
                i32.const 4096
                i32.const 4
                call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
              end
              local.get 2
              i32.const 448
              i32.add
              global.set $__stack_pointer
              return
            end
            local.get 2
            i32.load16_u offset=420
            local.set 1
            br 0 (;@4;)
          end
        end
      end
    end
    i32.const 4
    i32.const 4096
    call $_RNvNtCs5cOc02OMXlo_5alloc5alloc18handle_alloc_error
    unreachable)
  (func $_ZN5hashx7program7Program9interpret17h0055f1d8eb66940cE (type 1) (param i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64)
    global.get $__stack_pointer
    i32.const 32
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    local.get 0
    i32.load
    local.set 3
    i32.const 0
    local.set 0
    i32.const 1
    local.set 4
    i32.const 0
    local.set 5
    i32.const 0
    local.set 6
    loop  ;; label = @1
      local.get 8
      local.set 7
      local.get 6
      local.set 9
      local.get 0
      i32.const 3
      i32.shl
      local.set 10
      local.get 0
      local.set 8
      i32.const 1
      local.set 6
      local.get 0
      i32.const 1
      i32.add
      local.tee 11
      local.set 0
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          block  ;; label = @12
                            block  ;; label = @13
                              block  ;; label = @14
                                block  ;; label = @15
                                  block  ;; label = @16
                                    block  ;; label = @17
                                      block  ;; label = @18
                                        block  ;; label = @19
                                          block  ;; label = @20
                                            block  ;; label = @21
                                              block  ;; label = @22
                                                block  ;; label = @23
                                                  local.get 3
                                                  local.get 10
                                                  i32.add
                                                  local.tee 10
                                                  i32.load8_u
                                                  br_table 1 (;@22;) 2 (;@21;) 3 (;@20;) 4 (;@19;) 5 (;@18;) 6 (;@17;) 7 (;@16;) 8 (;@15;) 9 (;@14;) 21 (;@2;) 0 (;@23;) 1 (;@22;)
                                                end
                                                local.get 4
                                                i32.const 1
                                                i32.and
                                                local.set 0
                                                i32.const 0
                                                local.set 4
                                                local.get 0
                                                br_if 9 (;@13;)
                                                br 19 (;@3;)
                                              end
                                              block  ;; label = @22
                                                local.get 10
                                                i32.load8_u offset=1
                                                local.tee 0
                                                i32.const 8
                                                i32.lt_u
                                                br_if 0 (;@22;)
                                                local.get 0
                                                i32.const 8
                                                i32.const 1049832
                                                call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                                unreachable
                                              end
                                              local.get 10
                                              i32.load8_u offset=2
                                              local.tee 8
                                              i32.const 8
                                              i32.lt_u
                                              br_if 17 (;@4;)
                                              local.get 8
                                              i32.const 8
                                              i32.const 1049832
                                              call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                              unreachable
                                            end
                                            block  ;; label = @21
                                              local.get 10
                                              i32.load8_u offset=1
                                              local.tee 0
                                              i32.const 8
                                              i32.lt_u
                                              br_if 0 (;@21;)
                                              local.get 0
                                              i32.const 8
                                              i32.const 1049832
                                              call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                              unreachable
                                            end
                                            local.get 10
                                            i32.load8_u offset=2
                                            local.tee 8
                                            i32.const 8
                                            i32.lt_u
                                            br_if 15 (;@5;)
                                            local.get 8
                                            i32.const 8
                                            i32.const 1049832
                                            call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                            unreachable
                                          end
                                          block  ;; label = @20
                                            local.get 10
                                            i32.load8_u offset=1
                                            local.tee 0
                                            i32.const 8
                                            i32.lt_u
                                            br_if 0 (;@20;)
                                            local.get 0
                                            i32.const 8
                                            i32.const 1049832
                                            call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                            unreachable
                                          end
                                          local.get 10
                                          i32.load8_u offset=2
                                          local.tee 8
                                          i32.const 8
                                          i32.lt_u
                                          br_if 13 (;@6;)
                                          local.get 8
                                          i32.const 8
                                          i32.const 1049832
                                          call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                          unreachable
                                        end
                                        block  ;; label = @19
                                          local.get 10
                                          i32.load8_u offset=1
                                          local.tee 0
                                          i32.const 8
                                          i32.lt_u
                                          br_if 0 (;@19;)
                                          local.get 0
                                          i32.const 8
                                          i32.const 1049832
                                          call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                          unreachable
                                        end
                                        local.get 10
                                        i32.load8_u offset=2
                                        local.tee 8
                                        i32.const 8
                                        i32.lt_u
                                        br_if 11 (;@7;)
                                        local.get 8
                                        i32.const 8
                                        i32.const 1049832
                                        call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                        unreachable
                                      end
                                      local.get 10
                                      i32.load8_u offset=1
                                      local.tee 0
                                      i32.const 8
                                      i32.lt_u
                                      br_if 9 (;@8;)
                                      local.get 0
                                      i32.const 8
                                      i32.const 1049832
                                      call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                      unreachable
                                    end
                                    block  ;; label = @17
                                      local.get 10
                                      i32.load8_u offset=1
                                      local.tee 0
                                      i32.const 8
                                      i32.lt_u
                                      br_if 0 (;@17;)
                                      local.get 0
                                      i32.const 8
                                      i32.const 1049832
                                      call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                      unreachable
                                    end
                                    local.get 10
                                    i32.load8_u offset=2
                                    local.tee 8
                                    i32.const 8
                                    i32.lt_u
                                    br_if 7 (;@9;)
                                    local.get 8
                                    i32.const 8
                                    i32.const 1049832
                                    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                    unreachable
                                  end
                                  block  ;; label = @16
                                    local.get 10
                                    i32.load8_u offset=1
                                    local.tee 0
                                    i32.const 8
                                    i32.lt_u
                                    br_if 0 (;@16;)
                                    local.get 0
                                    i32.const 8
                                    i32.const 1049832
                                    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                    unreachable
                                  end
                                  local.get 10
                                  i32.load8_u offset=2
                                  local.tee 8
                                  i32.const 8
                                  i32.lt_u
                                  br_if 5 (;@10;)
                                  local.get 8
                                  i32.const 8
                                  i32.const 1049832
                                  call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                  unreachable
                                end
                                local.get 10
                                i32.load8_u offset=1
                                local.tee 0
                                i32.const 8
                                i32.lt_u
                                br_if 3 (;@11;)
                                local.get 0
                                i32.const 8
                                i32.const 1049832
                                call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                                unreachable
                              end
                              local.get 10
                              i32.load8_u offset=1
                              local.tee 0
                              i32.const 8
                              i32.lt_u
                              br_if 1 (;@12;)
                              local.get 0
                              i32.const 8
                              i32.const 1049832
                              call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check
                              unreachable
                            end
                            block  ;; label = @13
                              local.get 10
                              i32.load offset=4
                              local.get 5
                              i32.and
                              i32.eqz
                              br_if 0 (;@13;)
                              local.get 7
                              local.set 8
                              local.get 9
                              local.set 6
                              local.get 11
                              local.set 0
                              i32.const 1
                              local.set 4
                              br 11 (;@2;)
                            end
                            i32.const 1
                            local.set 6
                            local.get 7
                            local.set 8
                            local.get 7
                            local.set 0
                            local.get 9
                            i32.const 1
                            i32.and
                            br_if 10 (;@2;)
                            i32.const 1049848
                            i32.const 53
                            i32.const 1049904
                            call $_RNvNtCsgXGp5Oqx2Ny_4core6option13expect_failed
                            unreachable
                          end
                          local.get 1
                          local.get 0
                          i32.const 3
                          i32.shl
                          i32.add
                          local.tee 0
                          local.get 0
                          i64.load
                          local.get 10
                          i64.load8_u offset=2
                          i64.rotr
                          i64.store
                          br 8 (;@3;)
                        end
                        local.get 1
                        local.get 0
                        i32.const 3
                        i32.shl
                        i32.add
                        local.tee 0
                        local.get 0
                        i64.load
                        local.get 10
                        i64.load32_s offset=4
                        i64.xor
                        i64.store
                        br 7 (;@3;)
                      end
                      local.get 1
                      local.get 0
                      i32.const 3
                      i32.shl
                      i32.add
                      local.tee 0
                      local.get 1
                      local.get 8
                      i32.const 3
                      i32.shl
                      i32.add
                      i64.load
                      local.get 0
                      i64.load
                      i64.xor
                      i64.store
                      br 6 (;@3;)
                    end
                    local.get 1
                    local.get 0
                    i32.const 3
                    i32.shl
                    i32.add
                    local.tee 0
                    local.get 0
                    i64.load
                    local.get 1
                    local.get 8
                    i32.const 3
                    i32.shl
                    i32.add
                    i64.load
                    i64.sub
                    i64.store
                    br 5 (;@3;)
                  end
                  local.get 1
                  local.get 0
                  i32.const 3
                  i32.shl
                  i32.add
                  local.tee 0
                  local.get 0
                  i64.load
                  local.get 10
                  i64.load32_s offset=4
                  i64.add
                  i64.store
                  br 4 (;@3;)
                end
                local.get 1
                local.get 0
                i32.const 3
                i32.shl
                i32.add
                local.tee 0
                local.get 1
                local.get 8
                i32.const 3
                i32.shl
                i32.add
                i64.load
                local.get 10
                i64.load8_u offset=3
                i64.shl
                local.get 0
                i64.load
                i64.add
                i64.store
                br 3 (;@3;)
              end
              local.get 2
              i32.const 16
              i32.add
              local.get 1
              local.get 8
              i32.const 3
              i32.shl
              i32.add
              i64.load
              local.tee 12
              local.get 12
              i64.const 63
              i64.shr_s
              local.get 1
              local.get 0
              i32.const 3
              i32.shl
              i32.add
              local.tee 0
              i64.load
              local.tee 12
              local.get 12
              i64.const 63
              i64.shr_s
              call $__multi3
              local.get 0
              local.get 2
              i64.load offset=24
              local.tee 12
              i64.store
              local.get 12
              i32.wrap_i64
              local.set 5
              br 2 (;@3;)
            end
            local.get 2
            local.get 1
            local.get 8
            i32.const 3
            i32.shl
            i32.add
            i64.load
            i64.const 0
            local.get 1
            local.get 0
            i32.const 3
            i32.shl
            i32.add
            local.tee 0
            i64.load
            i64.const 0
            call $__multi3
            local.get 0
            local.get 2
            i64.load offset=8
            local.tee 12
            i64.store
            local.get 12
            i32.wrap_i64
            local.set 5
            br 1 (;@3;)
          end
          local.get 1
          local.get 0
          i32.const 3
          i32.shl
          i32.add
          local.tee 0
          local.get 1
          local.get 8
          i32.const 3
          i32.shl
          i32.add
          i64.load
          local.get 0
          i64.load
          i64.mul
          i64.store
        end
        local.get 7
        local.set 8
        local.get 9
        local.set 6
        local.get 11
        local.set 0
      end
      local.get 0
      i32.const 512
      i32.lt_u
      br_if 0 (;@1;)
    end
    local.get 2
    i32.const 32
    i32.add
    global.set $__stack_pointer)
  (func $_ZN5alloc4sync16Arc$LT$T$C$A$GT$9drop_slow17h543bbdef373111b6E (type 0) (param i32)
    (local i32 i32 i32 i32)
    block  ;; label = @1
      local.get 0
      i32.load
      local.tee 0
      i32.load8_u offset=8
      i32.const 3
      i32.ne
      br_if 0 (;@1;)
      local.get 0
      i32.const 12
      i32.add
      i32.load
      local.tee 1
      i32.load
      local.set 2
      block  ;; label = @2
        local.get 1
        i32.const 4
        i32.add
        i32.load
        local.tee 3
        i32.load
        local.tee 4
        i32.eqz
        br_if 0 (;@2;)
        local.get 2
        local.get 4
        call_indirect (type 0)
      end
      block  ;; label = @2
        local.get 3
        i32.load offset=4
        local.tee 4
        i32.eqz
        br_if 0 (;@2;)
        local.get 2
        local.get 4
        local.get 3
        i32.load offset=8
        call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
      end
      local.get 1
      i32.const 12
      i32.const 4
      call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
    end
    block  ;; label = @1
      local.get 0
      i32.const -1
      i32.eq
      br_if 0 (;@1;)
      local.get 0
      local.get 0
      i32.load offset=4
      local.tee 1
      i32.const -1
      i32.add
      i32.store offset=4
      local.get 1
      i32.const 1
      i32.ne
      br_if 0 (;@1;)
      local.get 0
      i32.const 16
      i32.const 4
      call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
    end)
  (func $_ZN5hashx12HashXBuilder5build17h2fb587a851cf5b39E (type 12) (param i32 i32 i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 144
    i32.sub
    local.tee 4
    global.set $__stack_pointer
    local.get 4
    i32.const 40
    i32.add
    local.get 2
    local.get 3
    call $_ZN5hashx7siphash8SipState14pair_from_seed17h57d87d0a4da54b55E
    local.get 4
    local.get 4
    i64.load offset=64
    i64.store offset=128
    local.get 4
    local.get 4
    i64.load offset=56
    i64.store offset=120
    local.get 4
    local.get 4
    i64.load offset=48
    i64.store offset=112
    local.get 4
    local.get 4
    i64.load offset=40
    i64.store offset=104
    local.get 4
    local.get 4
    i64.load offset=72
    i64.store offset=8
    local.get 4
    local.get 4
    i64.load offset=80
    i64.store offset=16
    local.get 4
    local.get 4
    i64.load offset=88
    i64.store offset=24
    local.get 4
    local.get 4
    i64.load offset=96
    i64.store offset=32
    local.get 4
    i64.const 0
    i64.store offset=136
    local.get 1
    i32.load8_u
    local.set 1
    local.get 4
    i32.const 40
    i32.add
    local.get 4
    i32.const 104
    i32.add
    call $_ZN5hashx7program7Program8generate17h6731d6e09f18a3ccE
    local.get 4
    i32.load offset=44
    local.set 2
    block  ;; label = @1
      block  ;; label = @2
        local.get 4
        i32.load offset=40
        local.tee 3
        i32.const 2
        i32.eq
        br_if 0 (;@2;)
        local.get 0
        local.get 2
        i32.store offset=8
        local.get 0
        local.get 3
        i32.store offset=4
        i32.const 1
        local.set 3
        br 1 (;@1;)
      end
      i32.const 1
      local.set 3
      block  ;; label = @2
        local.get 1
        i32.const 255
        i32.and
        i32.const 1
        i32.ne
        br_if 0 (;@2;)
        local.get 0
        i64.const 1
        i64.store offset=4 align=4
        local.get 2
        i32.const 4096
        i32.const 4
        call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
        br 1 (;@1;)
      end
      local.get 0
      local.get 4
      i64.load offset=32
      i64.store offset=32
      local.get 0
      local.get 4
      i64.load offset=24
      i64.store offset=24
      local.get 0
      local.get 4
      i64.load offset=16
      i64.store offset=16
      local.get 0
      local.get 4
      i64.load offset=8
      i64.store offset=8
      local.get 0
      local.get 2
      i32.store offset=40
      i32.const 0
      local.set 3
    end
    local.get 0
    local.get 3
    i32.store
    local.get 4
    i32.const 144
    i32.add
    global.set $__stack_pointer)
  (func $_ZN5hashx7siphash8SipState14pair_from_seed17h57d87d0a4da54b55E (type 9) (param i32 i32 i32)
    (local i32 i32 i32 i32)
    global.get $__stack_pointer
    i32.const 272
    i32.sub
    local.tee 3
    global.set $__stack_pointer
    local.get 3
    i32.const 7
    i32.add
    i32.const 0
    i32.const 128
    memory.fill
    local.get 3
    i32.const 0
    i32.store8 offset=135
    local.get 3
    i32.const 136
    i32.add
    i32.const 1050060
    i32.const 8
    i32.const 1
    i32.const 0
    i32.const 0
    i32.const 64
    call $_ZN6blake214Blake2bVarCore15new_with_params17hbddb78c2cf65de78E
    local.get 3
    i64.const 0
    i64.store offset=264
    local.get 3
    i64.const 0
    i64.store offset=256
    local.get 3
    i64.const 0
    i64.store offset=248
    local.get 3
    i64.const 0
    i64.store offset=240
    local.get 3
    i64.const 0
    i64.store offset=232
    local.get 3
    i64.const 0
    i64.store offset=224
    local.get 3
    i64.const 0
    i64.store offset=216
    local.get 3
    i64.const 0
    i64.store offset=208
    block  ;; label = @1
      block  ;; label = @2
        local.get 2
        i32.const 129
        i32.lt_u
        br_if 0 (;@2;)
        local.get 2
        i32.const 7
        i32.shr_u
        local.get 2
        i32.const 127
        i32.and
        local.tee 4
        i32.eqz
        i32.sub
        local.tee 5
        i32.const 7
        i32.shl
        local.set 6
        local.get 4
        i32.const 128
        local.get 4
        select
        local.set 2
        block  ;; label = @3
          local.get 5
          i32.eqz
          br_if 0 (;@3;)
          local.get 6
          local.set 5
          local.get 1
          local.set 4
          loop  ;; label = @4
            local.get 3
            local.get 3
            i64.load offset=200
            i64.const 128
            i64.add
            i64.store offset=200
            local.get 3
            i32.const 136
            i32.add
            local.get 4
            i64.const 0
            i64.const 0
            call $_ZN6blake214Blake2bVarCore8compress17h51445756c46d1d0aE
            local.get 4
            i32.const 128
            i32.add
            local.set 4
            local.get 5
            i32.const -128
            i32.add
            local.tee 5
            br_if 0 (;@4;)
          end
        end
        local.get 2
        i32.eqz
        br_if 1 (;@1;)
        local.get 3
        i32.const 7
        i32.add
        local.get 1
        local.get 6
        i32.add
        local.get 2
        memory.copy
        br 1 (;@1;)
      end
      local.get 2
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.const 7
      i32.add
      local.get 1
      local.get 2
      memory.copy
    end
    local.get 3
    local.get 3
    i64.load offset=200
    local.get 2
    i64.extend_i32_u
    i64.add
    i64.store offset=200
    block  ;; label = @1
      local.get 2
      i32.const 128
      i32.eq
      br_if 0 (;@1;)
      i32.const 128
      local.get 2
      i32.sub
      local.tee 4
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.const 7
      i32.add
      local.get 2
      i32.add
      i32.const 0
      local.get 4
      memory.fill
    end
    local.get 3
    i32.const 0
    i32.store8 offset=135
    local.get 3
    i32.const 136
    i32.add
    local.get 3
    i32.const 7
    i32.add
    i64.const 0
    local.get 3
    i32.const 208
    i32.add
    call $_ZN6blake214Blake2bVarCore18finalize_with_flag17h8280626bb9c29f91E
    local.get 0
    local.get 3
    i64.load offset=264
    i64.store offset=56
    local.get 0
    local.get 3
    i64.load offset=256
    i64.store offset=48
    local.get 0
    local.get 3
    i64.load offset=248
    i64.store offset=40
    local.get 0
    local.get 3
    i64.load offset=240
    i64.store offset=32
    local.get 0
    local.get 3
    i64.load offset=232
    i64.store offset=24
    local.get 0
    local.get 3
    i64.load offset=224
    i64.store offset=16
    local.get 0
    local.get 3
    i64.load offset=216
    i64.store offset=8
    local.get 0
    local.get 3
    i64.load offset=208
    i64.store
    local.get 3
    i32.const 272
    i32.add
    global.set $__stack_pointer)
  (func $_ZN5hashx5HashX11hash_to_u6417h4c9db866b9cd67d7E (type 14) (param i32 i64) (result i64)
    (local i32 i32 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)
    global.get $__stack_pointer
    i32.const 64
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    local.get 2
    local.get 0
    local.get 1
    call $_ZN5hashx7siphash13siphash24_ctr17h3f4e30402d2baf48E
    local.get 0
    i32.const 32
    i32.add
    local.set 3
    block  ;; label = @1
      local.get 0
      i32.load offset=32
      br_if 0 (;@1;)
      local.get 3
      local.get 2
      call $_ZN77_$LT$hashx..compiler..Executable$u20$as$u20$hashx..compiler..Architecture$GT$6invoke17hfb0f54ba72cfdb13E
      unreachable
    end
    local.get 3
    local.get 2
    call $_ZN5hashx7program7Program9interpret17h0055f1d8eb66940cE
    local.get 2
    i64.load offset=16
    local.set 4
    local.get 2
    i64.load offset=24
    local.set 1
    local.get 2
    i64.load
    local.set 5
    local.get 2
    i64.load offset=8
    local.set 6
    local.get 2
    i64.load offset=40
    local.set 7
    local.get 2
    i64.load offset=32
    local.set 8
    local.get 2
    i64.load offset=48
    local.set 9
    local.get 2
    i64.load offset=56
    local.set 10
    local.get 0
    i64.load
    local.set 11
    local.get 0
    i64.load offset=8
    local.set 12
    local.get 0
    i64.load offset=24
    local.set 13
    local.get 0
    i64.load offset=16
    local.set 14
    local.get 2
    i32.const 64
    i32.add
    global.set $__stack_pointer
    local.get 13
    local.get 10
    i64.add
    local.tee 10
    i64.const 16
    i64.rotl
    local.get 14
    local.get 9
    i64.add
    local.get 10
    i64.add
    i64.xor
    local.get 7
    local.get 8
    i64.add
    i64.const 32
    i64.rotl
    i64.add
    local.get 12
    local.get 11
    local.get 6
    local.get 5
    i64.add
    i64.add
    i64.add
    i64.const 32
    i64.rotl
    local.get 1
    i64.const 16
    i64.rotl
    local.get 1
    local.get 4
    i64.add
    i64.xor
    i64.add
    i64.xor)
  (func $_ZN5hashx7siphash13siphash24_ctr17h3f4e30402d2baf48E (type 15) (param i32 i32 i64)
    (local i64 i64 i64 i64 i64)
    local.get 0
    local.get 1
    i64.load offset=24
    local.get 2
    i64.xor
    local.tee 3
    i64.const 16
    i64.rotl
    local.get 3
    local.get 1
    i64.load offset=16
    i64.add
    local.tee 3
    i64.xor
    local.tee 4
    i64.const 21
    i64.rotl
    local.get 4
    local.get 1
    i64.load offset=8
    i64.const 238
    i64.xor
    local.tee 5
    local.get 1
    i64.load
    i64.add
    local.tee 6
    i64.const 32
    i64.rotl
    i64.add
    local.tee 4
    i64.xor
    local.tee 7
    i64.const 16
    i64.rotl
    local.get 7
    local.get 3
    local.get 5
    i64.const 13
    i64.rotl
    local.get 6
    i64.xor
    local.tee 5
    i64.add
    local.tee 3
    i64.const 32
    i64.rotl
    i64.add
    local.tee 6
    i64.xor
    local.tee 7
    local.get 4
    local.get 3
    local.get 5
    i64.const 17
    i64.rotl
    i64.xor
    local.tee 3
    i64.add
    local.tee 4
    i64.const 32
    i64.rotl
    i64.add
    local.tee 5
    local.get 2
    i64.xor
    local.get 3
    i64.const 13
    i64.rotl
    local.get 4
    i64.xor
    local.tee 2
    i64.const 17
    i64.rotl
    local.get 6
    local.get 2
    i64.add
    local.tee 2
    i64.xor
    local.tee 3
    i64.add
    local.tee 4
    local.get 3
    i64.const 13
    i64.rotl
    i64.xor
    local.tee 3
    i64.const 17
    i64.rotl
    local.get 3
    local.get 2
    i64.const 32
    i64.rotl
    i64.const 238
    i64.xor
    local.get 7
    i64.const 21
    i64.rotl
    local.get 5
    i64.xor
    local.tee 2
    i64.add
    local.tee 5
    i64.add
    local.tee 3
    i64.xor
    local.tee 6
    i64.const 13
    i64.rotl
    local.get 6
    local.get 4
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 16
    i64.rotl
    local.get 5
    i64.xor
    local.tee 2
    i64.add
    local.tee 4
    i64.add
    local.tee 5
    i64.xor
    local.tee 6
    i64.const 17
    i64.rotl
    local.get 6
    local.get 3
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 21
    i64.rotl
    local.get 4
    i64.xor
    local.tee 2
    i64.add
    local.tee 3
    i64.add
    local.tee 4
    i64.xor
    local.tee 6
    i64.const 13
    i64.rotl
    local.get 6
    local.get 5
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 16
    i64.rotl
    local.get 3
    i64.xor
    local.tee 2
    i64.add
    local.tee 3
    i64.add
    local.tee 5
    i64.xor
    local.tee 6
    i64.const 17
    i64.rotl
    local.get 6
    local.get 4
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 21
    i64.rotl
    local.get 3
    i64.xor
    local.tee 2
    i64.add
    local.tee 3
    i64.add
    local.tee 4
    i64.xor
    local.tee 6
    i64.const 13
    i64.rotl
    local.get 6
    local.get 5
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 16
    i64.rotl
    local.get 3
    i64.xor
    local.tee 2
    i64.add
    local.tee 3
    i64.add
    local.tee 5
    i64.xor
    local.tee 6
    local.get 4
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 21
    i64.rotl
    local.get 3
    i64.xor
    local.tee 2
    i64.add
    local.tee 3
    i64.add
    local.tee 4
    i64.const 32
    i64.rotl
    local.tee 7
    i64.store offset=16
    local.get 0
    local.get 6
    i64.const 17
    i64.rotl
    local.get 4
    i64.xor
    local.tee 4
    i64.store offset=8
    local.get 0
    local.get 5
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 16
    i64.rotl
    local.get 3
    i64.xor
    local.tee 3
    i64.add
    local.tee 2
    i64.store
    local.get 0
    local.get 3
    i64.const 21
    i64.rotl
    local.get 2
    i64.xor
    local.tee 3
    i64.store offset=24
    local.get 0
    local.get 4
    i64.const 221
    i64.xor
    local.tee 4
    i64.const 13
    i64.rotl
    local.get 4
    local.get 2
    i64.add
    local.tee 2
    i64.xor
    local.tee 4
    i64.const 17
    i64.rotl
    local.get 4
    local.get 7
    local.get 3
    i64.add
    local.tee 5
    i64.add
    local.tee 4
    i64.xor
    local.tee 6
    i64.const 13
    i64.rotl
    local.get 6
    local.get 2
    i64.const 32
    i64.rotl
    local.get 3
    i64.const 16
    i64.rotl
    local.get 5
    i64.xor
    local.tee 2
    i64.add
    local.tee 3
    i64.add
    local.tee 5
    i64.xor
    local.tee 6
    i64.const 17
    i64.rotl
    local.get 6
    local.get 4
    i64.const 32
    i64.rotl
    local.get 3
    local.get 2
    i64.const 21
    i64.rotl
    i64.xor
    local.tee 2
    i64.add
    local.tee 3
    i64.add
    local.tee 4
    i64.xor
    local.tee 6
    i64.const 13
    i64.rotl
    local.get 6
    local.get 5
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 16
    i64.rotl
    local.get 3
    i64.xor
    local.tee 2
    i64.add
    local.tee 3
    i64.add
    local.tee 5
    i64.xor
    local.tee 6
    i64.const 17
    i64.rotl
    local.get 6
    local.get 4
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 21
    i64.rotl
    local.get 3
    i64.xor
    local.tee 2
    i64.add
    local.tee 3
    i64.add
    local.tee 4
    i64.xor
    local.tee 6
    local.get 5
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 16
    i64.rotl
    local.get 3
    i64.xor
    local.tee 2
    i64.add
    local.tee 3
    i64.add
    local.tee 5
    i64.const 32
    i64.rotl
    local.get 2
    i64.const 21
    i64.rotl
    local.get 3
    i64.xor
    local.tee 2
    i64.const 16
    i64.rotl
    local.get 4
    i64.const 32
    i64.rotl
    local.get 2
    i64.add
    local.tee 2
    i64.xor
    local.tee 3
    i64.add
    local.tee 4
    i64.store offset=32
    local.get 0
    local.get 3
    i64.const 21
    i64.rotl
    local.get 4
    i64.xor
    i64.store offset=56
    local.get 0
    local.get 6
    i64.const 13
    i64.rotl
    local.get 5
    i64.xor
    local.tee 3
    local.get 2
    i64.add
    local.tee 2
    i64.const 32
    i64.rotl
    i64.store offset=48
    local.get 0
    local.get 3
    i64.const 17
    i64.rotl
    local.get 2
    i64.xor
    i64.store offset=40)
  (func $_ZN6blake214Blake2bVarCore15new_with_params17hbddb78c2cf65de78E (type 16) (param i32 i32 i32 i32 i32 i32 i32)
    (local i32 i64 i64 i64 i64)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 7
    global.set $__stack_pointer
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 5
            i32.const 65
            i32.ge_u
            br_if 0 (;@4;)
            local.get 6
            i32.const 65
            i32.ge_u
            br_if 1 (;@3;)
            local.get 2
            i32.const 17
            i32.ge_u
            br_if 2 (;@2;)
            local.get 4
            i32.const 17
            i32.ge_u
            br_if 3 (;@1;)
            block  ;; label = @5
              block  ;; label = @6
                local.get 2
                i32.const 16
                i32.eq
                br_if 0 (;@6;)
                i64.const 0
                local.set 8
                local.get 7
                i64.const 0
                i64.store offset=8
                local.get 7
                i64.const 0
                i64.store
                i64.const 0
                local.set 9
                local.get 2
                i32.eqz
                br_if 1 (;@5;)
                block  ;; label = @7
                  local.get 2
                  i32.eqz
                  br_if 0 (;@7;)
                  local.get 7
                  local.get 1
                  local.get 2
                  memory.copy
                end
                local.get 7
                i64.load offset=8
                local.set 8
                local.get 7
                i64.load
                local.set 9
                br 1 (;@5;)
              end
              local.get 1
              i64.load offset=8 align=1
              local.set 8
              local.get 1
              i64.load align=1
              local.set 9
            end
            block  ;; label = @5
              block  ;; label = @6
                local.get 4
                i32.const 16
                i32.eq
                br_if 0 (;@6;)
                i64.const 0
                local.set 10
                local.get 7
                i64.const 0
                i64.store offset=8
                local.get 7
                i64.const 0
                i64.store
                i64.const 0
                local.set 11
                local.get 4
                i32.eqz
                br_if 1 (;@5;)
                block  ;; label = @7
                  local.get 4
                  i32.eqz
                  br_if 0 (;@7;)
                  local.get 7
                  local.get 3
                  local.get 4
                  memory.copy
                end
                local.get 7
                i64.load offset=8
                local.set 10
                local.get 7
                i64.load
                local.set 11
                br 1 (;@5;)
              end
              local.get 3
              i64.load offset=8 align=1
              local.set 10
              local.get 3
              i64.load align=1
              local.set 11
            end
            local.get 0
            i64.const 0
            i64.store offset=64
            local.get 0
            i64.const -6534734903238641935
            i64.store offset=24
            local.get 0
            i64.const 4354685564936845355
            i64.store offset=16
            local.get 0
            i64.const -4942790177534073029
            i64.store offset=8
            local.get 0
            local.get 10
            i64.const 6620516959819538809
            i64.xor
            i64.store offset=56
            local.get 0
            local.get 11
            i64.const 2270897969802886507
            i64.xor
            i64.store offset=48
            local.get 0
            local.get 8
            i64.const -7276294671716946913
            i64.xor
            i64.store offset=40
            local.get 0
            local.get 9
            i64.const 5840696475078001361
            i64.xor
            i64.store offset=32
            local.get 0
            local.get 5
            i32.const 8
            i32.shl
            local.get 6
            i32.or
            i32.const 16842752
            i32.or
            i64.extend_i32_u
            i64.const 7640891576956012808
            i64.xor
            i64.store
            local.get 7
            i32.const 16
            i32.add
            global.set $__stack_pointer
            return
          end
          i32.const 1050068
          i32.const 45
          i32.const 1050116
          call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
          unreachable
        end
        i32.const 1050132
        i32.const 48
        i32.const 1050116
        call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
        unreachable
      end
      i32.const 1050180
      i32.const 38
      i32.const 1050116
      call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
      unreachable
    end
    i32.const 1050218
    i32.const 41
    i32.const 1050116
    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
    unreachable)
  (func $_ZN6blake214Blake2bVarCore18finalize_with_flag17h8280626bb9c29f91E (type 17) (param i32 i32 i64 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 64
    i32.sub
    local.tee 4
    global.set $__stack_pointer
    local.get 0
    local.get 1
    i64.const -1
    local.get 2
    call $_ZN6blake214Blake2bVarCore8compress17h51445756c46d1d0aE
    local.get 3
    local.get 0
    i64.load
    i64.store align=1
    local.get 3
    local.get 0
    i64.load offset=8
    i64.store offset=8 align=1
    local.get 3
    local.get 0
    i64.load offset=16
    i64.store offset=16 align=1
    local.get 3
    local.get 0
    i64.load offset=24
    i64.store offset=24 align=1
    local.get 3
    local.get 0
    i64.load offset=32
    i64.store offset=32 align=1
    local.get 3
    local.get 0
    i64.load offset=40
    i64.store offset=40 align=1
    local.get 3
    local.get 0
    i64.load offset=48
    i64.store offset=48 align=1
    local.get 3
    local.get 0
    i64.load offset=56
    i64.store offset=56 align=1
    local.get 4
    i32.const 64
    i32.add
    global.set $__stack_pointer)
  (func $_ZN6blake214Blake2bVarCore8compress17h51445756c46d1d0aE (type 18) (param i32 i32 i64 i64)
    (local i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)
    local.get 0
    local.get 0
    i64.load offset=16
    local.tee 4
    local.get 1
    i64.load offset=32 align=1
    local.tee 5
    i64.add
    local.get 0
    i64.load offset=48
    local.tee 6
    i64.add
    local.tee 7
    local.get 1
    i64.load offset=40 align=1
    local.tee 8
    i64.add
    local.get 2
    local.get 7
    i64.xor
    i64.const 2270897969802886507
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 9
    i64.const 4354685564936845355
    i64.add
    local.tee 10
    local.get 6
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 11
    i64.add
    local.tee 12
    local.get 1
    i64.load offset=96 align=1
    local.tee 2
    i64.add
    local.get 0
    i64.load offset=24
    local.tee 13
    local.get 1
    i64.load offset=48 align=1
    local.tee 7
    i64.add
    local.get 0
    i64.load offset=56
    local.tee 14
    i64.add
    local.tee 15
    local.get 1
    i64.load offset=56 align=1
    local.tee 16
    i64.add
    local.get 3
    local.get 15
    i64.xor
    i64.const 6620516959819538809
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 3
    i64.const -6534734903238641935
    i64.add
    local.tee 15
    local.get 14
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 17
    i64.add
    local.tee 18
    local.get 3
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 15
    i64.add
    local.tee 20
    local.get 17
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 21
    i64.add
    local.tee 22
    local.get 1
    i64.load offset=104 align=1
    local.tee 3
    i64.add
    local.get 22
    local.get 0
    i64.load offset=8
    local.tee 23
    local.get 1
    i64.load offset=16 align=1
    local.tee 15
    i64.add
    local.get 0
    i64.load offset=40
    local.tee 24
    i64.add
    local.tee 25
    local.get 1
    i64.load offset=24 align=1
    local.tee 17
    i64.add
    local.get 25
    i64.const -7276294671716946913
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 25
    i64.const -4942790177534073029
    i64.add
    local.tee 26
    local.get 24
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 27
    i64.add
    local.tee 28
    local.get 25
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 29
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 30
    local.get 0
    i64.load
    local.tee 31
    local.get 1
    i64.load align=1
    local.tee 22
    i64.add
    local.get 0
    i64.load offset=32
    local.tee 32
    i64.add
    local.tee 33
    local.get 1
    i64.load offset=8 align=1
    local.tee 25
    i64.add
    local.get 0
    i64.load offset=64
    local.get 33
    i64.xor
    i64.const 5840696475078001361
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 33
    i64.const 7640891576956012808
    i64.add
    local.tee 34
    local.get 32
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 35
    i64.add
    local.tee 36
    local.get 33
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 37
    local.get 34
    i64.add
    local.tee 34
    i64.add
    local.tee 38
    local.get 21
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 40
    local.get 1
    i64.load offset=72 align=1
    local.tee 21
    i64.add
    local.get 28
    local.get 1
    i64.load offset=80 align=1
    local.tee 33
    i64.add
    local.get 12
    local.get 9
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 12
    local.get 10
    i64.add
    local.tee 28
    local.get 11
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 10
    i64.add
    local.tee 11
    local.get 1
    i64.load offset=88 align=1
    local.tee 9
    i64.add
    local.get 11
    local.get 37
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 11
    local.get 20
    i64.add
    local.tee 20
    local.get 10
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 10
    i64.add
    local.tee 37
    local.get 11
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 41
    local.get 20
    i64.add
    local.tee 20
    local.get 10
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 42
    i64.add
    local.tee 43
    local.get 1
    i64.load offset=120 align=1
    local.tee 10
    i64.add
    local.get 43
    local.get 18
    local.get 1
    i64.load offset=112 align=1
    local.tee 11
    i64.add
    local.get 34
    local.get 35
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 10
    i64.add
    local.get 34
    local.get 12
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 12
    local.get 29
    local.get 26
    i64.add
    local.tee 26
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 12
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 43
    local.get 36
    local.get 1
    i64.load offset=64 align=1
    local.tee 12
    i64.add
    local.get 26
    local.get 27
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 21
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 28
    i64.add
    local.tee 27
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 28
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 27
    i64.add
    local.tee 27
    i64.add
    local.tee 36
    local.get 42
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 42
    i64.add
    local.tee 44
    local.get 9
    i64.add
    local.get 34
    local.get 3
    i64.add
    local.get 40
    local.get 30
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 30
    local.get 38
    i64.add
    local.tee 34
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 39
    local.get 7
    i64.add
    local.get 39
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 39
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 40
    local.get 16
    i64.add
    local.get 40
    local.get 37
    local.get 5
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 12
    i64.add
    local.get 27
    local.get 30
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 27
    local.get 35
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 30
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 35
    local.get 27
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 27
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 37
    local.get 28
    local.get 11
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 28
    local.get 33
    i64.add
    local.get 28
    local.get 41
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 34
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 40
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 41
    local.get 8
    i64.add
    local.get 35
    local.get 22
    i64.add
    local.get 44
    local.get 43
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 36
    i64.add
    local.tee 36
    local.get 42
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 42
    i64.add
    local.tee 43
    local.get 15
    i64.add
    local.get 43
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 42
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 42
    i64.add
    local.tee 43
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 42
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 42
    i64.add
    local.tee 44
    local.get 15
    i64.add
    local.get 44
    local.get 39
    local.get 8
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 17
    i64.add
    local.get 29
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 29
    local.get 27
    local.get 30
    i64.add
    local.tee 27
    i64.add
    local.tee 30
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 35
    local.get 29
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 29
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 39
    local.get 34
    local.get 25
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 2
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 36
    i64.add
    local.tee 27
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 34
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 27
    i64.add
    local.tee 27
    i64.add
    local.tee 36
    local.get 42
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 42
    i64.add
    local.tee 44
    local.get 16
    i64.add
    local.get 35
    local.get 10
    i64.add
    local.get 41
    local.get 37
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 40
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 40
    local.get 3
    i64.add
    local.get 40
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 40
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 41
    local.get 25
    i64.add
    local.get 41
    local.get 43
    local.get 2
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 22
    i64.add
    local.get 27
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 27
    local.get 29
    local.get 30
    i64.add
    local.tee 29
    i64.add
    local.tee 30
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 35
    local.get 27
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 27
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 41
    local.get 34
    local.get 9
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 12
    i64.add
    local.get 29
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 37
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 3
    i64.add
    local.get 35
    local.get 17
    i64.add
    local.get 44
    local.get 39
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 36
    i64.add
    local.tee 36
    local.get 42
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 42
    local.get 7
    i64.add
    local.get 42
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 42
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 2
    i64.add
    local.get 44
    local.get 40
    local.get 21
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 5
    i64.add
    local.get 29
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 29
    local.get 27
    local.get 30
    i64.add
    local.tee 27
    i64.add
    local.tee 30
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 35
    local.get 29
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 29
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 40
    local.get 34
    local.get 33
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 11
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 36
    i64.add
    local.tee 27
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 34
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 27
    i64.add
    local.tee 27
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 5
    i64.add
    local.get 35
    local.get 9
    i64.add
    local.get 43
    local.get 41
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 37
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 41
    local.get 11
    i64.add
    local.get 41
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 41
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 22
    i64.add
    local.get 43
    local.get 42
    local.get 17
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 25
    i64.add
    local.get 27
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 27
    local.get 29
    local.get 30
    i64.add
    local.tee 29
    i64.add
    local.tee 30
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 35
    local.get 27
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 27
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 42
    local.get 34
    local.get 16
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 21
    i64.add
    local.get 29
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 37
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 15
    i64.add
    local.get 35
    local.get 8
    i64.add
    local.get 44
    local.get 40
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 36
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 40
    local.get 33
    i64.add
    local.get 40
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 40
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 5
    i64.add
    local.get 44
    local.get 41
    local.get 10
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 12
    i64.add
    local.get 29
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 29
    local.get 27
    local.get 30
    i64.add
    local.tee 27
    i64.add
    local.tee 30
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 35
    local.get 29
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 29
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 41
    local.get 34
    local.get 15
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 7
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 36
    i64.add
    local.tee 27
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 34
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 27
    i64.add
    local.tee 27
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 7
    i64.add
    local.get 35
    local.get 33
    i64.add
    local.get 43
    local.get 42
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 37
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 42
    local.get 10
    i64.add
    local.get 42
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 42
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 12
    i64.add
    local.get 43
    local.get 40
    local.get 8
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 16
    i64.add
    local.get 27
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 27
    local.get 29
    local.get 30
    i64.add
    local.tee 29
    i64.add
    local.tee 30
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 35
    local.get 27
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 27
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 40
    local.get 34
    local.get 21
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 22
    i64.add
    local.get 29
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 37
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 22
    i64.add
    local.get 35
    local.get 9
    i64.add
    local.get 44
    local.get 41
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 36
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 41
    local.get 2
    i64.add
    local.get 41
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 41
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 9
    i64.add
    local.get 44
    local.get 42
    local.get 17
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 3
    i64.add
    local.get 29
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 29
    local.get 27
    local.get 30
    i64.add
    local.tee 27
    i64.add
    local.tee 30
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 35
    local.get 29
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 29
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 42
    local.get 34
    local.get 11
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 25
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 36
    i64.add
    local.tee 27
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 34
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 27
    i64.add
    local.tee 27
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 10
    i64.add
    local.get 35
    local.get 12
    i64.add
    local.get 43
    local.get 40
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 37
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 40
    local.get 17
    i64.add
    local.get 40
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 40
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 11
    i64.add
    local.get 43
    local.get 41
    local.get 7
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 33
    i64.add
    local.get 27
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 27
    local.get 29
    local.get 30
    i64.add
    local.tee 29
    i64.add
    local.tee 30
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 35
    local.get 27
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 27
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 41
    local.get 34
    local.get 15
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 2
    i64.add
    local.get 29
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 37
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 11
    i64.add
    local.get 35
    local.get 16
    i64.add
    local.get 44
    local.get 42
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 36
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 42
    local.get 8
    i64.add
    local.get 42
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 42
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 3
    i64.add
    local.get 44
    local.get 40
    local.get 25
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 21
    i64.add
    local.get 29
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 29
    local.get 27
    local.get 30
    i64.add
    local.tee 27
    i64.add
    local.tee 30
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 35
    local.get 29
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 29
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 40
    local.get 34
    local.get 5
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 3
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 36
    i64.add
    local.tee 27
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 34
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 27
    i64.add
    local.tee 27
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 21
    i64.add
    local.get 35
    local.get 5
    i64.add
    local.get 43
    local.get 41
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 37
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 41
    local.get 33
    i64.add
    local.get 41
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 41
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 15
    i64.add
    local.get 43
    local.get 42
    local.get 25
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 10
    i64.add
    local.get 27
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 27
    local.get 29
    local.get 30
    i64.add
    local.tee 29
    i64.add
    local.tee 30
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 35
    local.get 27
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 27
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 42
    local.get 34
    local.get 2
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 8
    i64.add
    local.get 29
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 37
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 2
    i64.add
    local.get 35
    local.get 7
    i64.add
    local.get 44
    local.get 40
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 36
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 40
    local.get 17
    i64.add
    local.get 40
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 40
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 25
    i64.add
    local.get 44
    local.get 41
    local.get 12
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 9
    i64.add
    local.get 29
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 29
    local.get 27
    local.get 30
    i64.add
    local.tee 27
    i64.add
    local.tee 30
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 35
    local.get 29
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 29
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 41
    local.get 34
    local.get 22
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 16
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 36
    i64.add
    local.tee 27
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 34
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 27
    i64.add
    local.tee 27
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 12
    i64.add
    local.get 35
    local.get 17
    i64.add
    local.get 43
    local.get 42
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 37
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 42
    local.get 21
    i64.add
    local.get 42
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 42
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 7
    i64.add
    local.get 43
    local.get 40
    local.get 16
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 11
    i64.add
    local.get 27
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 27
    local.get 29
    local.get 30
    i64.add
    local.tee 29
    i64.add
    local.tee 30
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 35
    local.get 27
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 27
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 40
    local.get 34
    local.get 3
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 9
    i64.add
    local.get 29
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 37
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 9
    i64.add
    local.get 35
    local.get 10
    i64.add
    local.get 44
    local.get 41
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 36
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 41
    local.get 5
    i64.add
    local.get 41
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 41
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 17
    i64.add
    local.get 44
    local.get 42
    local.get 15
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 33
    i64.add
    local.get 29
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 29
    local.get 27
    local.get 30
    i64.add
    local.tee 27
    i64.add
    local.tee 30
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 35
    local.get 29
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 29
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 42
    local.get 34
    local.get 8
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 22
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 36
    i64.add
    local.tee 27
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 34
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 27
    i64.add
    local.tee 27
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 25
    i64.add
    local.get 35
    local.get 22
    i64.add
    local.get 43
    local.get 40
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 37
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 40
    local.get 12
    i64.add
    local.get 40
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 40
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 5
    i64.add
    local.get 43
    local.get 41
    local.get 11
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 21
    i64.add
    local.get 27
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 27
    local.get 29
    local.get 30
    i64.add
    local.tee 29
    i64.add
    local.tee 30
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 35
    local.get 27
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 27
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 41
    local.get 34
    local.get 7
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 10
    i64.add
    local.get 29
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 37
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 16
    i64.add
    local.get 35
    local.get 3
    i64.add
    local.get 44
    local.get 42
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 36
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 42
    local.get 16
    i64.add
    local.get 42
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 42
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 7
    i64.add
    local.get 44
    local.get 40
    local.get 33
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 8
    i64.add
    local.get 29
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 29
    local.get 27
    local.get 30
    i64.add
    local.tee 27
    i64.add
    local.tee 30
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 35
    local.get 29
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 29
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 40
    local.get 34
    local.get 2
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 15
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 36
    i64.add
    local.tee 27
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 34
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 27
    i64.add
    local.tee 27
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 17
    i64.add
    local.get 35
    local.get 25
    i64.add
    local.get 43
    local.get 41
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 37
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 41
    local.get 8
    i64.add
    local.get 41
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 41
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 2
    i64.add
    local.get 43
    local.get 42
    local.get 12
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 5
    i64.add
    local.get 27
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 27
    local.get 29
    local.get 30
    i64.add
    local.tee 29
    i64.add
    local.tee 30
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 35
    local.get 27
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 27
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 42
    local.get 34
    local.get 33
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 15
    i64.add
    local.get 29
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 37
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 5
    i64.add
    local.get 35
    local.get 21
    i64.add
    local.get 44
    local.get 40
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 36
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 40
    local.get 11
    i64.add
    local.get 40
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 40
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 8
    i64.add
    local.get 44
    local.get 41
    local.get 3
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 22
    i64.add
    local.get 29
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 29
    local.get 27
    local.get 30
    i64.add
    local.tee 27
    i64.add
    local.tee 30
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 35
    local.get 29
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 29
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 41
    local.get 34
    local.get 10
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 9
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 36
    i64.add
    local.tee 27
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 34
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 27
    i64.add
    local.tee 27
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 2
    i64.add
    local.get 35
    local.get 7
    i64.add
    local.get 43
    local.get 42
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 37
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 42
    local.get 16
    i64.add
    local.get 42
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 42
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 19
    local.get 20
    i64.add
    local.tee 20
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 3
    i64.add
    local.get 43
    local.get 40
    local.get 15
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 17
    i64.add
    local.get 27
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 27
    local.get 29
    local.get 30
    i64.add
    local.tee 29
    i64.add
    local.tee 30
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 35
    local.get 27
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 27
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 40
    local.get 34
    local.get 22
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 25
    i64.add
    local.get 29
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 37
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 34
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 29
    i64.add
    local.tee 29
    i64.add
    local.tee 37
    local.get 38
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 38
    i64.add
    local.tee 43
    local.get 21
    i64.add
    local.get 35
    local.get 33
    i64.add
    local.get 44
    local.get 41
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 35
    local.get 36
    i64.add
    local.tee 36
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 41
    local.get 9
    i64.add
    local.get 41
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 39
    i64.add
    local.tee 41
    local.get 28
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 28
    local.get 20
    i64.add
    local.tee 20
    local.get 39
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 39
    i64.add
    local.tee 44
    local.get 10
    i64.add
    local.get 44
    local.get 42
    local.get 11
    i64.add
    local.get 29
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 18
    i64.add
    local.tee 29
    local.get 10
    i64.add
    local.get 29
    local.get 35
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 10
    local.get 27
    local.get 30
    i64.add
    local.tee 27
    i64.add
    local.tee 29
    local.get 18
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 18
    i64.add
    local.tee 30
    local.get 10
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 10
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 35
    local.get 34
    local.get 12
    i64.add
    local.get 27
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 21
    i64.add
    local.get 27
    local.get 19
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 21
    local.get 36
    i64.add
    local.tee 19
    local.get 26
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 26
    i64.add
    local.tee 27
    local.get 21
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 21
    local.get 19
    i64.add
    local.tee 19
    i64.add
    local.tee 34
    local.get 39
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 36
    i64.add
    local.tee 39
    local.get 9
    i64.add
    local.get 30
    local.get 3
    i64.add
    local.get 43
    local.get 40
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 3
    local.get 37
    i64.add
    local.tee 9
    local.get 38
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 30
    i64.add
    local.tee 37
    local.get 7
    i64.add
    local.get 37
    local.get 21
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 7
    local.get 20
    i64.add
    local.tee 21
    local.get 30
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 20
    i64.add
    local.tee 30
    local.get 7
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 7
    local.get 21
    i64.add
    local.tee 21
    local.get 20
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 20
    i64.add
    local.tee 37
    local.get 16
    i64.add
    local.get 37
    local.get 41
    local.get 5
    i64.add
    local.get 19
    local.get 26
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 5
    i64.add
    local.tee 16
    local.get 12
    i64.add
    local.get 16
    local.get 3
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 16
    local.get 10
    local.get 29
    i64.add
    local.tee 3
    i64.add
    local.tee 10
    local.get 5
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 5
    i64.add
    local.tee 12
    local.get 16
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 16
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 19
    local.get 27
    local.get 11
    i64.add
    local.get 3
    local.get 18
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 3
    i64.add
    local.tee 11
    local.get 33
    i64.add
    local.get 11
    local.get 28
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 33
    local.get 9
    i64.add
    local.tee 9
    local.get 3
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 3
    i64.add
    local.tee 11
    local.get 33
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 33
    local.get 9
    i64.add
    local.tee 9
    i64.add
    local.tee 18
    local.get 20
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 20
    i64.add
    local.tee 26
    local.get 4
    i64.xor
    local.get 11
    local.get 25
    i64.add
    local.get 16
    local.get 10
    i64.add
    local.tee 16
    local.get 5
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 5
    i64.add
    local.tee 25
    local.get 2
    i64.add
    local.get 25
    local.get 7
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 2
    local.get 39
    local.get 35
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 7
    local.get 34
    i64.add
    local.tee 25
    i64.add
    local.tee 10
    local.get 5
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 5
    i64.add
    local.tee 11
    local.get 2
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 2
    local.get 10
    i64.add
    local.tee 10
    i64.xor
    i64.store offset=16
    local.get 0
    local.get 13
    local.get 17
    local.get 30
    local.get 8
    i64.add
    local.get 9
    local.get 3
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 8
    i64.add
    local.tee 3
    i64.add
    local.get 3
    local.get 7
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 7
    local.get 16
    i64.add
    local.tee 16
    local.get 8
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 8
    i64.add
    local.tee 3
    i64.xor
    local.get 15
    local.get 12
    local.get 22
    i64.add
    local.get 25
    local.get 36
    i64.xor
    i64.const 1
    i64.rotl
    local.tee 17
    i64.add
    local.tee 22
    i64.add
    local.get 22
    local.get 33
    i64.xor
    i64.const 32
    i64.rotl
    local.tee 15
    local.get 21
    i64.add
    local.tee 22
    local.get 17
    i64.xor
    i64.const 40
    i64.rotl
    local.tee 17
    i64.add
    local.tee 25
    local.get 15
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 15
    local.get 22
    i64.add
    local.tee 22
    i64.xor
    i64.store offset=24
    local.get 0
    local.get 25
    local.get 23
    i64.xor
    local.get 3
    local.get 7
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 7
    local.get 16
    i64.add
    local.tee 16
    i64.xor
    i64.store offset=8
    local.get 0
    local.get 11
    local.get 31
    i64.xor
    local.get 26
    local.get 19
    i64.xor
    i64.const 48
    i64.rotl
    local.tee 3
    local.get 18
    i64.add
    local.tee 25
    i64.xor
    i64.store
    local.get 0
    local.get 24
    local.get 10
    local.get 5
    i64.xor
    i64.const 1
    i64.rotl
    i64.xor
    local.get 3
    i64.xor
    i64.store offset=40
    local.get 0
    local.get 14
    local.get 25
    local.get 20
    i64.xor
    i64.const 1
    i64.rotl
    i64.xor
    local.get 2
    i64.xor
    i64.store offset=56
    local.get 0
    local.get 6
    local.get 22
    local.get 17
    i64.xor
    i64.const 1
    i64.rotl
    i64.xor
    local.get 7
    i64.xor
    i64.store offset=48
    local.get 0
    local.get 32
    local.get 16
    local.get 8
    i64.xor
    i64.const 1
    i64.rotl
    i64.xor
    local.get 15
    i64.xor
    i64.store offset=32)
  (func $_RNvCsfLfy6EI15iL_7___rustc18___rust_start_panic (type 3) (param i32 i32) (result i32)
    call $_RNvCsfLfy6EI15iL_7___rustc12___rust_abort
    unreachable)
  (func $_RINvNvMs2_NtCs5cOc02OMXlo_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsebHcaeoSrxy_3std (type 11) (param i32 i32 i32 i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 5
    global.set $__stack_pointer
    block  ;; label = @1
      local.get 2
      local.get 1
      i32.add
      local.tee 1
      local.get 2
      i32.ge_u
      br_if 0 (;@1;)
      i32.const 0
      i32.const 0
      call $_RNvNtCs5cOc02OMXlo_5alloc7raw_vec12handle_error
      unreachable
    end
    local.get 5
    i32.const 4
    i32.add
    local.get 0
    i32.load
    local.tee 2
    local.get 0
    i32.load offset=4
    local.get 1
    local.get 2
    i32.const 1
    i32.shl
    local.tee 2
    local.get 1
    local.get 2
    i32.gt_u
    select
    local.tee 2
    i32.const 8
    i32.const 4
    local.get 4
    i32.const 1
    i32.eq
    select
    local.tee 1
    local.get 2
    local.get 1
    i32.gt_u
    select
    local.tee 2
    local.get 3
    local.get 4
    call $_RNvMs4_NtCs5cOc02OMXlo_5alloc7raw_vecNtB5_11RawVecInner11finish_growCsebHcaeoSrxy_3std
    block  ;; label = @1
      local.get 5
      i32.load offset=4
      i32.const 1
      i32.ne
      br_if 0 (;@1;)
      local.get 5
      i32.load offset=8
      local.get 5
      i32.load offset=12
      call $_RNvNtCs5cOc02OMXlo_5alloc7raw_vec12handle_error
      unreachable
    end
    local.get 5
    i32.load offset=8
    local.set 4
    local.get 0
    local.get 2
    i32.store
    local.get 0
    local.get 4
    i32.store offset=4
    local.get 5
    i32.const 16
    i32.add
    global.set $__stack_pointer)
  (func $_RINvNtCsgXGp5Oqx2Ny_4core3ptr13drop_in_placeINtNtB4_6option6OptionINtNtCs5cOc02OMXlo_5alloc3vec3VechEEECsebHcaeoSrxy_3std (type 1) (param i32 i32)
    block  ;; label = @1
      local.get 0
      i32.const -2147483648
      i32.or
      i32.const -2147483648
      i32.eq
      br_if 0 (;@1;)
      local.get 1
      local.get 0
      i32.const 1
      call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
    end)
  (func $_RINvNtCsgXGp5Oqx2Ny_4core3ptr13drop_in_placeNtNtCs5cOc02OMXlo_5alloc6string6StringECsebHcaeoSrxy_3std (type 0) (param i32)
    (local i32)
    block  ;; label = @1
      local.get 0
      i32.load
      local.tee 1
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      i32.load offset=4
      local.get 1
      i32.const 1
      call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
    end)
  (func $_RINvNtCsgXGp5Oqx2Ny_4core3ptr13drop_in_placeNtNvNtCsebHcaeoSrxy_3std9panicking13panic_handler19FormatStringPayloadEBM_ (type 0) (param i32)
    (local i32)
    block  ;; label = @1
      local.get 0
      i32.load
      local.tee 1
      i32.const 1
      i32.lt_s
      br_if 0 (;@1;)
      local.get 0
      i32.load offset=4
      local.get 1
      i32.const 1
      call $_RNvCsfLfy6EI15iL_7___rustc14___rust_dealloc
    end)
  (func $_RINvNtNtCsebHcaeoSrxy_3std3sys9backtrace26___rust_end_short_backtraceNCNvNtB6_5alloc8rust_oom0zEB6_ (type 0) (param i32)
    local.get 0
    call $_RNCNvNtCsebHcaeoSrxy_3std5alloc8rust_oom0B5_
    unreachable)
  (func $_RNCNvNtCsebHcaeoSrxy_3std5alloc8rust_oom0B5_ (type 0) (param i32)
    local.get 0
    i32.load
    local.get 0
    i32.load offset=4
    i32.const 0
    i32.load offset=1056160
    local.tee 0
    i32.const 2
    local.get 0
    select
    call_indirect (type 1)
    unreachable)
  (func $_RINvNtNtCsebHcaeoSrxy_3std3sys9backtrace26___rust_end_short_backtraceNCNvNtB6_9panicking13panic_handler0zEB6_ (type 0) (param i32)
    local.get 0
    call $_RNCNvNtCsebHcaeoSrxy_3std9panicking13panic_handler0B5_
    unreachable)
  (func $_RNCNvNtCsebHcaeoSrxy_3std9panicking13panic_handler0B5_ (type 0) (param i32)
    (local i32 i32 i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 1
    global.set $__stack_pointer
    block  ;; label = @1
      local.get 0
      i32.load
      local.tee 2
      i32.load offset=4
      local.tee 3
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 2
      i32.load
      local.set 2
      local.get 1
      local.get 3
      i32.const 1
      i32.shr_u
      i32.store offset=4
      local.get 1
      local.get 2
      i32.store
      local.get 1
      i32.const 1050284
      local.get 0
      i32.load offset=4
      local.get 0
      i32.load offset=8
      local.tee 0
      i32.load8_u offset=8
      local.get 0
      i32.load8_u offset=9
      call $_RNvNtCsebHcaeoSrxy_3std9panicking15panic_with_hook
      unreachable
    end
    local.get 1
    i32.const -2147483648
    i32.store
    local.get 1
    local.get 0
    i32.store offset=12
    local.get 1
    i32.const 1050312
    local.get 0
    i32.load offset=4
    local.get 0
    i32.load offset=8
    local.tee 0
    i32.load8_u offset=8
    local.get 0
    i32.load8_u offset=9
    call $_RNvNtCsebHcaeoSrxy_3std9panicking15panic_with_hook
    unreachable)
  (func $_RNvMs4_NtCs5cOc02OMXlo_5alloc7raw_vecNtB5_11RawVecInner11finish_growCsebHcaeoSrxy_3std (type 19) (param i32 i32 i32 i32 i32 i32)
    (local i32 i32 i64)
    i32.const 1
    local.set 6
    i32.const 4
    local.set 7
    block  ;; label = @1
      block  ;; label = @2
        local.get 5
        i64.extend_i32_u
        local.get 3
        i64.extend_i32_u
        i64.mul
        local.tee 8
        i64.const 32
        i64.shr_u
        i32.wrap_i64
        i32.eqz
        br_if 0 (;@2;)
        i32.const 0
        local.set 3
        br 1 (;@1;)
      end
      block  ;; label = @2
        local.get 8
        i32.wrap_i64
        local.tee 3
        i32.const -2147483648
        local.get 4
        i32.sub
        i32.le_u
        br_if 0 (;@2;)
        i32.const 0
        local.set 3
        br 1 (;@1;)
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 1
              i32.eqz
              br_if 0 (;@5;)
              local.get 2
              local.get 5
              local.get 1
              i32.mul
              local.get 4
              local.get 3
              call $_RNvCsfLfy6EI15iL_7___rustc14___rust_realloc
              local.set 7
              br 1 (;@4;)
            end
            block  ;; label = @5
              local.get 3
              br_if 0 (;@5;)
              local.get 4
              local.set 7
              br 2 (;@3;)
            end
            call $_RNvCsfLfy6EI15iL_7___rustc35___rust_no_alloc_shim_is_unstable_v2
            local.get 3
            local.get 4
            call $_RNvCsfLfy6EI15iL_7___rustc12___rust_alloc
            local.set 7
          end
          local.get 7
          br_if 0 (;@3;)
          local.get 0
          local.get 4
          i32.store offset=4
          br 1 (;@2;)
        end
        local.get 0
        local.get 7
        i32.store offset=4
        i32.const 0
        local.set 6
      end
      i32.const 8
      local.set 7
    end
    local.get 0
    local.get 7
    i32.add
    local.get 3
    i32.store
    local.get 0
    local.get 6
    i32.store)
  (func $_RNvNtCsebHcaeoSrxy_3std9panicking15panic_with_hook (type 11) (param i32 i32 i32 i32 i32)
    (local i32 i32)
    global.get $__stack_pointer
    i32.const 32
    i32.sub
    local.tee 5
    global.set $__stack_pointer
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              i32.const 1
              call $_RNvNtNtCsebHcaeoSrxy_3std9panicking11panic_count8increase
              i32.const 255
              i32.and
              br_table 4 (;@1;) 1 (;@4;) 0 (;@5;) 1 (;@4;)
            end
            i32.const 0
            i32.load offset=1056164
            local.tee 6
            i32.const -1
            i32.le_s
            br_if 3 (;@1;)
            i32.const 0
            local.get 6
            i32.const 1
            i32.add
            i32.store offset=1056164
            i32.const 0
            i32.load offset=1056168
            i32.eqz
            br_if 1 (;@3;)
            local.get 5
            i32.const 8
            i32.add
            local.get 0
            local.get 1
            i32.load offset=20
            call_indirect (type 1)
            local.get 5
            local.get 4
            i32.store8 offset=29
            local.get 5
            local.get 3
            i32.store8 offset=28
            local.get 5
            local.get 2
            i32.store offset=24
            local.get 5
            local.get 5
            i64.load offset=8
            i64.store offset=16 align=4
            i32.const 0
            i32.load offset=1056168
            local.get 5
            i32.const 16
            i32.add
            i32.const 0
            i32.load offset=1056172
            i32.load offset=20
            call_indirect (type 1)
            br 2 (;@2;)
          end
          local.get 5
          local.get 0
          local.get 1
          i32.load offset=24
          call_indirect (type 1)
          br 2 (;@1;)
        end
        i32.const -2147483648
        local.get 5
        call $_RINvNtCsgXGp5Oqx2Ny_4core3ptr13drop_in_placeINtNtB4_6option6OptionINtNtCs5cOc02OMXlo_5alloc3vec3VechEEECsebHcaeoSrxy_3std
      end
      i32.const 0
      i32.const 0
      i32.load offset=1056164
      i32.const -1
      i32.add
      i32.store offset=1056164
      i32.const 0
      i32.const 0
      i32.store8 offset=1056156
      local.get 3
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      local.get 1
      call $_RNvCsfLfy6EI15iL_7___rustc10rust_panic
      unreachable
    end
    unreachable)
  (func $_RNvNtCsebHcaeoSrxy_3std5alloc24default_alloc_error_hook (type 1) (param i32 i32)
    i32.const 0
    i32.const 1
    i32.store8 offset=1056632)
  (func $_RNvCsfLfy6EI15iL_7___rustc10rust_panic (type 1) (param i32 i32)
    local.get 0
    local.get 1
    call $_RNvCsfLfy6EI15iL_7___rustc18___rust_start_panic
    drop
    unreachable)
  (func $_RNvCsfLfy6EI15iL_7___rustc11___rdl_alloc (type 3) (param i32 i32) (result i32)
    block  ;; label = @1
      local.get 1
      i32.const 9
      i32.lt_u
      br_if 0 (;@1;)
      local.get 1
      local.get 0
      call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE8memalignCsebHcaeoSrxy_3std
      return
    end
    local.get 0
    call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE6mallocCsebHcaeoSrxy_3std)
  (func $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE8memalignCsebHcaeoSrxy_3std (type 3) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32)
    i32.const 0
    local.set 2
    block  ;; label = @1
      local.get 1
      i32.const -65587
      local.get 0
      i32.const 16
      local.get 0
      i32.const 16
      i32.gt_u
      select
      local.tee 0
      i32.sub
      i32.ge_u
      br_if 0 (;@1;)
      local.get 0
      i32.const 16
      local.get 1
      i32.const 11
      i32.add
      i32.const -8
      i32.and
      local.get 1
      i32.const 11
      i32.lt_u
      select
      local.tee 3
      i32.add
      i32.const 12
      i32.add
      call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE6mallocCsebHcaeoSrxy_3std
      local.tee 1
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      i32.const -8
      i32.add
      local.set 2
      block  ;; label = @2
        block  ;; label = @3
          local.get 0
          i32.const -1
          i32.add
          local.tee 4
          local.get 1
          i32.and
          br_if 0 (;@3;)
          local.get 2
          local.set 0
          br 1 (;@2;)
        end
        local.get 1
        i32.const -4
        i32.add
        local.tee 5
        i32.load
        local.tee 6
        i32.const -8
        i32.and
        local.get 4
        local.get 1
        i32.add
        i32.const 0
        local.get 0
        i32.sub
        i32.and
        i32.const -8
        i32.add
        local.tee 1
        i32.const 0
        local.get 0
        local.get 1
        local.get 2
        i32.sub
        i32.const 16
        i32.gt_u
        select
        i32.add
        local.tee 0
        local.get 2
        i32.sub
        local.tee 1
        i32.sub
        local.set 4
        block  ;; label = @3
          local.get 6
          i32.const 3
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          local.get 4
          local.get 0
          i32.load offset=4
          i32.const 1
          i32.and
          i32.or
          i32.const 2
          i32.or
          i32.store offset=4
          local.get 0
          local.get 4
          i32.add
          local.tee 4
          local.get 4
          i32.load offset=4
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 5
          local.get 1
          local.get 5
          i32.load
          i32.const 1
          i32.and
          i32.or
          i32.const 2
          i32.or
          i32.store
          local.get 2
          local.get 1
          i32.add
          local.tee 4
          local.get 4
          i32.load offset=4
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 2
          local.get 1
          call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE13dispose_chunkCsebHcaeoSrxy_3std
          br 1 (;@2;)
        end
        local.get 2
        i32.load
        local.set 2
        local.get 0
        local.get 4
        i32.store offset=4
        local.get 0
        local.get 2
        local.get 1
        i32.add
        i32.store
      end
      block  ;; label = @2
        local.get 0
        i32.load offset=4
        local.tee 1
        i32.const 3
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 1
        i32.const -8
        i32.and
        local.tee 2
        local.get 3
        i32.const 16
        i32.add
        i32.le_u
        br_if 0 (;@2;)
        local.get 0
        local.get 3
        local.get 1
        i32.const 1
        i32.and
        i32.or
        i32.const 2
        i32.or
        i32.store offset=4
        local.get 0
        local.get 3
        i32.add
        local.tee 1
        local.get 2
        local.get 3
        i32.sub
        local.tee 3
        i32.const 3
        i32.or
        i32.store offset=4
        local.get 0
        local.get 2
        i32.add
        local.tee 2
        local.get 2
        i32.load offset=4
        i32.const 1
        i32.or
        i32.store offset=4
        local.get 1
        local.get 3
        call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE13dispose_chunkCsebHcaeoSrxy_3std
      end
      local.get 0
      i32.const 8
      i32.add
      local.set 2
    end
    local.get 2)
  (func $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE6mallocCsebHcaeoSrxy_3std (type 20) (param i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i64)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 1
    global.set $__stack_pointer
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 0
            i32.const 245
            i32.lt_u
            br_if 0 (;@4;)
            block  ;; label = @5
              local.get 0
              i32.const -65588
              i32.le_u
              br_if 0 (;@5;)
              i32.const 0
              local.set 0
              br 4 (;@1;)
            end
            local.get 0
            i32.const 11
            i32.add
            local.tee 2
            i32.const -8
            i32.and
            local.set 3
            i32.const 0
            i32.load offset=1056592
            local.tee 4
            i32.eqz
            br_if 2 (;@2;)
            i32.const 31
            local.set 5
            local.get 0
            i32.const 16777205
            i32.ge_u
            br_if 1 (;@3;)
            local.get 3
            i32.const 38
            local.get 2
            i32.const 8
            i32.shr_u
            i32.clz
            local.tee 0
            i32.sub
            i32.shr_u
            i32.const 1
            i32.and
            local.get 0
            i32.const 1
            i32.shl
            i32.sub
            i32.const 62
            i32.add
            local.set 5
            br 1 (;@3;)
          end
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      i32.const 0
                      i32.load offset=1056588
                      local.tee 6
                      i32.const 16
                      local.get 0
                      i32.const 11
                      i32.add
                      i32.const 504
                      i32.and
                      local.get 0
                      i32.const 11
                      i32.lt_u
                      select
                      local.tee 3
                      i32.const 3
                      i32.shr_u
                      local.tee 2
                      i32.shr_u
                      local.tee 0
                      i32.const 3
                      i32.and
                      i32.eqz
                      br_if 0 (;@9;)
                      local.get 0
                      i32.const -1
                      i32.xor
                      i32.const 1
                      i32.and
                      local.get 2
                      i32.add
                      local.tee 7
                      i32.const 3
                      i32.shl
                      local.tee 3
                      i32.const 1056324
                      i32.add
                      local.tee 0
                      local.get 3
                      i32.const 1056332
                      i32.add
                      i32.load
                      local.tee 2
                      i32.load offset=8
                      local.tee 8
                      i32.eq
                      br_if 1 (;@8;)
                      local.get 8
                      local.get 0
                      i32.store offset=12
                      local.get 0
                      local.get 8
                      i32.store offset=8
                      br 2 (;@7;)
                    end
                    local.get 3
                    i32.const 0
                    i32.load offset=1056596
                    i32.le_u
                    br_if 6 (;@2;)
                    local.get 0
                    br_if 2 (;@6;)
                    i32.const 0
                    i32.load offset=1056592
                    local.tee 0
                    i32.eqz
                    br_if 6 (;@2;)
                    local.get 0
                    i32.ctz
                    i32.const 2
                    i32.shl
                    i32.const 1056180
                    i32.add
                    i32.load
                    local.tee 8
                    i32.load offset=4
                    i32.const -8
                    i32.and
                    local.get 3
                    i32.sub
                    local.set 2
                    local.get 8
                    local.set 6
                    loop  ;; label = @9
                      block  ;; label = @10
                        local.get 8
                        i32.load offset=16
                        local.tee 0
                        br_if 0 (;@10;)
                        local.get 8
                        i32.load offset=20
                        local.tee 0
                        br_if 0 (;@10;)
                        local.get 6
                        i32.load offset=24
                        local.set 5
                        block  ;; label = @11
                          block  ;; label = @12
                            block  ;; label = @13
                              local.get 6
                              i32.load offset=12
                              local.tee 0
                              local.get 6
                              i32.ne
                              br_if 0 (;@13;)
                              local.get 6
                              i32.const 20
                              i32.const 16
                              local.get 6
                              i32.load offset=20
                              local.tee 0
                              select
                              i32.add
                              i32.load
                              local.tee 8
                              br_if 1 (;@12;)
                              i32.const 0
                              local.set 0
                              br 2 (;@11;)
                            end
                            local.get 6
                            i32.load offset=8
                            local.tee 8
                            local.get 0
                            i32.store offset=12
                            local.get 0
                            local.get 8
                            i32.store offset=8
                            br 1 (;@11;)
                          end
                          local.get 6
                          i32.const 20
                          i32.add
                          local.get 6
                          i32.const 16
                          i32.add
                          local.get 0
                          select
                          local.set 7
                          loop  ;; label = @12
                            local.get 7
                            local.set 9
                            local.get 8
                            local.tee 0
                            i32.const 20
                            i32.add
                            local.get 0
                            i32.const 16
                            i32.add
                            local.get 0
                            i32.load offset=20
                            local.tee 8
                            select
                            local.set 7
                            local.get 0
                            i32.const 20
                            i32.const 16
                            local.get 8
                            select
                            i32.add
                            i32.load
                            local.tee 8
                            br_if 0 (;@12;)
                          end
                          local.get 9
                          i32.const 0
                          i32.store
                        end
                        local.get 5
                        i32.eqz
                        br_if 6 (;@4;)
                        block  ;; label = @11
                          block  ;; label = @12
                            local.get 6
                            local.get 6
                            i32.load offset=28
                            i32.const 2
                            i32.shl
                            i32.const 1056180
                            i32.add
                            local.tee 8
                            i32.load
                            i32.eq
                            br_if 0 (;@12;)
                            block  ;; label = @13
                              local.get 5
                              i32.load offset=16
                              local.get 6
                              i32.eq
                              br_if 0 (;@13;)
                              local.get 5
                              local.get 0
                              i32.store offset=20
                              local.get 0
                              br_if 2 (;@11;)
                              br 9 (;@4;)
                            end
                            local.get 5
                            local.get 0
                            i32.store offset=16
                            local.get 0
                            br_if 1 (;@11;)
                            br 8 (;@4;)
                          end
                          local.get 8
                          local.get 0
                          i32.store
                          local.get 0
                          i32.eqz
                          br_if 6 (;@5;)
                        end
                        local.get 0
                        local.get 5
                        i32.store offset=24
                        block  ;; label = @11
                          local.get 6
                          i32.load offset=16
                          local.tee 8
                          i32.eqz
                          br_if 0 (;@11;)
                          local.get 0
                          local.get 8
                          i32.store offset=16
                          local.get 8
                          local.get 0
                          i32.store offset=24
                        end
                        local.get 6
                        i32.load offset=20
                        local.tee 8
                        i32.eqz
                        br_if 6 (;@4;)
                        local.get 0
                        local.get 8
                        i32.store offset=20
                        local.get 8
                        local.get 0
                        i32.store offset=24
                        br 6 (;@4;)
                      end
                      local.get 0
                      i32.load offset=4
                      i32.const -8
                      i32.and
                      local.get 3
                      i32.sub
                      local.tee 8
                      local.get 2
                      local.get 8
                      local.get 2
                      i32.lt_u
                      local.tee 8
                      select
                      local.set 2
                      local.get 0
                      local.get 6
                      local.get 8
                      select
                      local.set 6
                      local.get 0
                      local.set 8
                      br 0 (;@9;)
                    end
                  end
                  i32.const 0
                  local.get 6
                  i32.const -2
                  local.get 7
                  i32.rotl
                  i32.and
                  i32.store offset=1056588
                end
                local.get 2
                i32.const 8
                i32.add
                local.set 0
                local.get 2
                local.get 3
                i32.const 3
                i32.or
                i32.store offset=4
                local.get 2
                local.get 3
                i32.add
                local.tee 3
                local.get 3
                i32.load offset=4
                i32.const 1
                i32.or
                i32.store offset=4
                br 5 (;@1;)
              end
              block  ;; label = @6
                block  ;; label = @7
                  local.get 0
                  local.get 2
                  i32.shl
                  i32.const 2
                  local.get 2
                  i32.shl
                  local.tee 0
                  i32.const 0
                  local.get 0
                  i32.sub
                  i32.or
                  i32.and
                  i32.ctz
                  local.tee 9
                  i32.const 3
                  i32.shl
                  local.tee 2
                  i32.const 1056324
                  i32.add
                  local.tee 8
                  local.get 2
                  i32.const 1056332
                  i32.add
                  i32.load
                  local.tee 0
                  i32.load offset=8
                  local.tee 7
                  i32.eq
                  br_if 0 (;@7;)
                  local.get 7
                  local.get 8
                  i32.store offset=12
                  local.get 8
                  local.get 7
                  i32.store offset=8
                  br 1 (;@6;)
                end
                i32.const 0
                local.get 6
                i32.const -2
                local.get 9
                i32.rotl
                i32.and
                i32.store offset=1056588
              end
              local.get 0
              local.get 3
              i32.const 3
              i32.or
              i32.store offset=4
              local.get 0
              local.get 3
              i32.add
              local.tee 6
              local.get 2
              local.get 3
              i32.sub
              local.tee 8
              i32.const 1
              i32.or
              i32.store offset=4
              local.get 0
              local.get 2
              i32.add
              local.get 8
              i32.store
              block  ;; label = @6
                i32.const 0
                i32.load offset=1056596
                local.tee 2
                i32.eqz
                br_if 0 (;@6;)
                i32.const 0
                i32.load offset=1056604
                local.set 3
                block  ;; label = @7
                  block  ;; label = @8
                    i32.const 0
                    i32.load offset=1056588
                    local.tee 7
                    i32.const 1
                    local.get 2
                    i32.const 3
                    i32.shr_u
                    i32.shl
                    local.tee 9
                    i32.and
                    br_if 0 (;@8;)
                    i32.const 0
                    local.get 7
                    local.get 9
                    i32.or
                    i32.store offset=1056588
                    local.get 2
                    i32.const -8
                    i32.and
                    i32.const 1056324
                    i32.add
                    local.tee 2
                    local.set 7
                    br 1 (;@7;)
                  end
                  local.get 2
                  i32.const -8
                  i32.and
                  local.tee 2
                  i32.const 1056324
                  i32.add
                  local.set 7
                  local.get 2
                  i32.const 1056332
                  i32.add
                  i32.load
                  local.set 2
                end
                local.get 7
                local.get 3
                i32.store offset=8
                local.get 2
                local.get 3
                i32.store offset=12
                local.get 3
                local.get 7
                i32.store offset=12
                local.get 3
                local.get 2
                i32.store offset=8
              end
              local.get 0
              i32.const 8
              i32.add
              local.set 0
              i32.const 0
              local.get 6
              i32.store offset=1056604
              i32.const 0
              local.get 8
              i32.store offset=1056596
              br 4 (;@1;)
            end
            i32.const 0
            i32.const 0
            i32.load offset=1056592
            i32.const -2
            local.get 6
            i32.load offset=28
            i32.rotl
            i32.and
            i32.store offset=1056592
          end
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 2
                i32.const 16
                i32.lt_u
                br_if 0 (;@6;)
                local.get 6
                local.get 3
                i32.const 3
                i32.or
                i32.store offset=4
                local.get 6
                local.get 3
                i32.add
                local.tee 8
                local.get 2
                i32.const 1
                i32.or
                i32.store offset=4
                local.get 8
                local.get 2
                i32.add
                local.get 2
                i32.store
                i32.const 0
                i32.load offset=1056596
                local.tee 7
                i32.eqz
                br_if 1 (;@5;)
                i32.const 0
                i32.load offset=1056604
                local.set 0
                block  ;; label = @7
                  block  ;; label = @8
                    i32.const 0
                    i32.load offset=1056588
                    local.tee 9
                    i32.const 1
                    local.get 7
                    i32.const 3
                    i32.shr_u
                    i32.shl
                    local.tee 5
                    i32.and
                    br_if 0 (;@8;)
                    i32.const 0
                    local.get 9
                    local.get 5
                    i32.or
                    i32.store offset=1056588
                    local.get 7
                    i32.const -8
                    i32.and
                    i32.const 1056324
                    i32.add
                    local.tee 7
                    local.set 9
                    br 1 (;@7;)
                  end
                  local.get 7
                  i32.const -8
                  i32.and
                  local.tee 7
                  i32.const 1056324
                  i32.add
                  local.set 9
                  local.get 7
                  i32.const 1056332
                  i32.add
                  i32.load
                  local.set 7
                end
                local.get 9
                local.get 0
                i32.store offset=8
                local.get 7
                local.get 0
                i32.store offset=12
                local.get 0
                local.get 9
                i32.store offset=12
                local.get 0
                local.get 7
                i32.store offset=8
                br 1 (;@5;)
              end
              local.get 6
              local.get 2
              local.get 3
              i32.add
              local.tee 0
              i32.const 3
              i32.or
              i32.store offset=4
              local.get 6
              local.get 0
              i32.add
              local.tee 0
              local.get 0
              i32.load offset=4
              i32.const 1
              i32.or
              i32.store offset=4
              br 1 (;@4;)
            end
            i32.const 0
            local.get 8
            i32.store offset=1056604
            i32.const 0
            local.get 2
            i32.store offset=1056596
          end
          local.get 6
          i32.const 8
          i32.add
          local.tee 0
          i32.eqz
          br_if 1 (;@2;)
          br 2 (;@1;)
        end
        i32.const 0
        local.get 3
        i32.sub
        local.set 2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 5
                i32.const 2
                i32.shl
                i32.const 1056180
                i32.add
                i32.load
                local.tee 6
                br_if 0 (;@6;)
                i32.const 0
                local.set 8
                i32.const 0
                local.set 0
                br 1 (;@5;)
              end
              i32.const 0
              local.set 8
              local.get 3
              i32.const 0
              i32.const 25
              local.get 5
              i32.const 1
              i32.shr_u
              i32.sub
              local.get 5
              i32.const 31
              i32.eq
              select
              i32.shl
              local.set 7
              i32.const 0
              local.set 0
              loop  ;; label = @6
                block  ;; label = @7
                  local.get 6
                  local.tee 6
                  i32.load offset=4
                  i32.const -8
                  i32.and
                  local.tee 9
                  local.get 3
                  i32.lt_u
                  br_if 0 (;@7;)
                  local.get 9
                  local.get 3
                  i32.sub
                  local.tee 9
                  local.get 2
                  i32.ge_u
                  br_if 0 (;@7;)
                  local.get 6
                  local.set 8
                  local.get 9
                  local.set 2
                  local.get 9
                  br_if 0 (;@7;)
                  i32.const 0
                  local.set 2
                  local.get 6
                  local.set 0
                  local.get 6
                  local.set 8
                  br 3 (;@4;)
                end
                local.get 6
                i32.load offset=20
                local.tee 9
                local.get 0
                local.get 9
                local.get 6
                local.get 7
                i32.const 29
                i32.shr_u
                i32.const 4
                i32.and
                i32.add
                i32.load offset=16
                local.tee 6
                i32.ne
                select
                local.get 0
                local.get 9
                select
                local.set 0
                local.get 7
                i32.const 1
                i32.shl
                local.set 7
                local.get 6
                br_if 0 (;@6;)
              end
            end
            block  ;; label = @5
              local.get 0
              local.get 8
              i32.or
              br_if 0 (;@5;)
              i32.const 0
              local.set 8
              i32.const 2
              local.get 5
              i32.shl
              local.tee 0
              i32.const 0
              local.get 0
              i32.sub
              i32.or
              local.get 4
              i32.and
              local.tee 0
              i32.eqz
              br_if 3 (;@2;)
              local.get 0
              i32.ctz
              i32.const 2
              i32.shl
              i32.const 1056180
              i32.add
              i32.load
              local.set 0
            end
            local.get 0
            i32.eqz
            br_if 1 (;@3;)
          end
          loop  ;; label = @4
            local.get 0
            i32.load offset=4
            i32.const -8
            i32.and
            local.tee 6
            local.get 3
            i32.sub
            local.tee 7
            local.get 2
            local.get 7
            local.get 2
            i32.lt_u
            local.tee 9
            select
            local.set 5
            local.get 6
            local.get 3
            i32.lt_u
            local.set 7
            local.get 0
            local.get 8
            local.get 9
            select
            local.set 9
            block  ;; label = @5
              local.get 0
              i32.load offset=16
              local.tee 6
              br_if 0 (;@5;)
              local.get 0
              i32.load offset=20
              local.set 6
            end
            local.get 2
            local.get 5
            local.get 7
            select
            local.set 2
            local.get 8
            local.get 9
            local.get 7
            select
            local.set 8
            local.get 6
            local.set 0
            local.get 6
            br_if 0 (;@4;)
          end
        end
        local.get 8
        i32.eqz
        br_if 0 (;@2;)
        block  ;; label = @3
          i32.const 0
          i32.load offset=1056596
          local.tee 0
          local.get 3
          i32.lt_u
          br_if 0 (;@3;)
          local.get 2
          local.get 0
          local.get 3
          i32.sub
          i32.ge_u
          br_if 1 (;@2;)
        end
        local.get 8
        i32.load offset=24
        local.set 5
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 8
              i32.load offset=12
              local.tee 0
              local.get 8
              i32.ne
              br_if 0 (;@5;)
              local.get 8
              i32.const 20
              i32.const 16
              local.get 8
              i32.load offset=20
              local.tee 0
              select
              i32.add
              i32.load
              local.tee 6
              br_if 1 (;@4;)
              i32.const 0
              local.set 0
              br 2 (;@3;)
            end
            local.get 8
            i32.load offset=8
            local.tee 6
            local.get 0
            i32.store offset=12
            local.get 0
            local.get 6
            i32.store offset=8
            br 1 (;@3;)
          end
          local.get 8
          i32.const 20
          i32.add
          local.get 8
          i32.const 16
          i32.add
          local.get 0
          select
          local.set 7
          loop  ;; label = @4
            local.get 7
            local.set 9
            local.get 6
            local.tee 0
            i32.const 20
            i32.add
            local.get 0
            i32.const 16
            i32.add
            local.get 0
            i32.load offset=20
            local.tee 6
            select
            local.set 7
            local.get 0
            i32.const 20
            i32.const 16
            local.get 6
            select
            i32.add
            i32.load
            local.tee 6
            br_if 0 (;@4;)
          end
          local.get 9
          i32.const 0
          i32.store
        end
        block  ;; label = @3
          local.get 5
          i32.eqz
          br_if 0 (;@3;)
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 8
                local.get 8
                i32.load offset=28
                i32.const 2
                i32.shl
                i32.const 1056180
                i32.add
                local.tee 6
                i32.load
                i32.eq
                br_if 0 (;@6;)
                block  ;; label = @7
                  local.get 5
                  i32.load offset=16
                  local.get 8
                  i32.eq
                  br_if 0 (;@7;)
                  local.get 5
                  local.get 0
                  i32.store offset=20
                  local.get 0
                  br_if 2 (;@5;)
                  br 4 (;@3;)
                end
                local.get 5
                local.get 0
                i32.store offset=16
                local.get 0
                br_if 1 (;@5;)
                br 3 (;@3;)
              end
              local.get 6
              local.get 0
              i32.store
              local.get 0
              i32.eqz
              br_if 1 (;@4;)
            end
            local.get 0
            local.get 5
            i32.store offset=24
            block  ;; label = @5
              local.get 8
              i32.load offset=16
              local.tee 6
              i32.eqz
              br_if 0 (;@5;)
              local.get 0
              local.get 6
              i32.store offset=16
              local.get 6
              local.get 0
              i32.store offset=24
            end
            local.get 8
            i32.load offset=20
            local.tee 6
            i32.eqz
            br_if 1 (;@3;)
            local.get 0
            local.get 6
            i32.store offset=20
            local.get 6
            local.get 0
            i32.store offset=24
            br 1 (;@3;)
          end
          i32.const 0
          i32.const 0
          i32.load offset=1056592
          i32.const -2
          local.get 8
          i32.load offset=28
          i32.rotl
          i32.and
          i32.store offset=1056592
        end
        block  ;; label = @3
          block  ;; label = @4
            local.get 2
            i32.const 16
            i32.lt_u
            br_if 0 (;@4;)
            local.get 8
            local.get 3
            i32.const 3
            i32.or
            i32.store offset=4
            local.get 8
            local.get 3
            i32.add
            local.tee 0
            local.get 2
            i32.const 1
            i32.or
            i32.store offset=4
            local.get 0
            local.get 2
            i32.add
            local.get 2
            i32.store
            block  ;; label = @5
              local.get 2
              i32.const 256
              i32.lt_u
              br_if 0 (;@5;)
              local.get 0
              local.get 2
              call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE18insert_large_chunkCsebHcaeoSrxy_3std
              br 2 (;@3;)
            end
            block  ;; label = @5
              block  ;; label = @6
                i32.const 0
                i32.load offset=1056588
                local.tee 6
                i32.const 1
                local.get 2
                i32.const 3
                i32.shr_u
                i32.shl
                local.tee 7
                i32.and
                br_if 0 (;@6;)
                i32.const 0
                local.get 6
                local.get 7
                i32.or
                i32.store offset=1056588
                local.get 2
                i32.const 248
                i32.and
                i32.const 1056324
                i32.add
                local.tee 2
                local.set 6
                br 1 (;@5;)
              end
              local.get 2
              i32.const 248
              i32.and
              local.tee 2
              i32.const 1056324
              i32.add
              local.set 6
              local.get 2
              i32.const 1056332
              i32.add
              i32.load
              local.set 2
            end
            local.get 6
            local.get 0
            i32.store offset=8
            local.get 2
            local.get 0
            i32.store offset=12
            local.get 0
            local.get 6
            i32.store offset=12
            local.get 0
            local.get 2
            i32.store offset=8
            br 1 (;@3;)
          end
          local.get 8
          local.get 2
          local.get 3
          i32.add
          local.tee 0
          i32.const 3
          i32.or
          i32.store offset=4
          local.get 8
          local.get 0
          i32.add
          local.tee 0
          local.get 0
          i32.load offset=4
          i32.const 1
          i32.or
          i32.store offset=4
        end
        local.get 8
        i32.const 8
        i32.add
        local.tee 0
        br_if 1 (;@1;)
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  i32.const 0
                  i32.load offset=1056596
                  local.tee 0
                  local.get 3
                  i32.ge_u
                  br_if 0 (;@7;)
                  block  ;; label = @8
                    i32.const 0
                    i32.load offset=1056600
                    local.tee 0
                    local.get 3
                    i32.gt_u
                    br_if 0 (;@8;)
                    local.get 1
                    i32.const 4
                    i32.add
                    i32.const 1056632
                    local.get 3
                    i32.const 65583
                    i32.add
                    i32.const -65536
                    i32.and
                    call $_RNvXs_NtCsjqx8TIyZbP9_8dlmalloc3sysNtB4_6SystemNtB6_9Allocator5alloc
                    block  ;; label = @9
                      local.get 1
                      i32.load offset=4
                      local.tee 6
                      br_if 0 (;@9;)
                      i32.const 0
                      local.set 0
                      br 8 (;@1;)
                    end
                    local.get 1
                    i32.load offset=12
                    local.set 5
                    i32.const 0
                    i32.const 0
                    i32.load offset=1056612
                    local.get 1
                    i32.load offset=8
                    local.tee 9
                    i32.add
                    local.tee 0
                    i32.store offset=1056612
                    i32.const 0
                    local.get 0
                    i32.const 0
                    i32.load offset=1056616
                    local.tee 2
                    local.get 0
                    local.get 2
                    i32.gt_u
                    select
                    i32.store offset=1056616
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          i32.const 0
                          i32.load offset=1056608
                          local.tee 2
                          i32.eqz
                          br_if 0 (;@11;)
                          i32.const 1056308
                          local.set 0
                          loop  ;; label = @12
                            local.get 6
                            local.get 0
                            i32.load
                            local.tee 8
                            local.get 0
                            i32.load offset=4
                            local.tee 7
                            i32.add
                            i32.eq
                            br_if 2 (;@10;)
                            local.get 0
                            i32.load offset=8
                            local.tee 0
                            br_if 0 (;@12;)
                            br 3 (;@9;)
                          end
                        end
                        block  ;; label = @11
                          block  ;; label = @12
                            i32.const 0
                            i32.load offset=1056624
                            local.tee 0
                            i32.eqz
                            br_if 0 (;@12;)
                            local.get 6
                            local.get 0
                            i32.ge_u
                            br_if 1 (;@11;)
                          end
                          i32.const 0
                          local.get 6
                          i32.store offset=1056624
                        end
                        i32.const 0
                        i32.const 4095
                        i32.store offset=1056628
                        i32.const 0
                        local.get 5
                        i32.store offset=1056320
                        i32.const 0
                        local.get 9
                        i32.store offset=1056312
                        i32.const 0
                        local.get 6
                        i32.store offset=1056308
                        i32.const 0
                        i32.const 1056324
                        i32.store offset=1056336
                        i32.const 0
                        i32.const 1056332
                        i32.store offset=1056344
                        i32.const 0
                        i32.const 1056324
                        i32.store offset=1056332
                        i32.const 0
                        i32.const 1056340
                        i32.store offset=1056352
                        i32.const 0
                        i32.const 1056332
                        i32.store offset=1056340
                        i32.const 0
                        i32.const 1056348
                        i32.store offset=1056360
                        i32.const 0
                        i32.const 1056340
                        i32.store offset=1056348
                        i32.const 0
                        i32.const 1056356
                        i32.store offset=1056368
                        i32.const 0
                        i32.const 1056348
                        i32.store offset=1056356
                        i32.const 0
                        i32.const 1056364
                        i32.store offset=1056376
                        i32.const 0
                        i32.const 1056356
                        i32.store offset=1056364
                        i32.const 0
                        i32.const 1056372
                        i32.store offset=1056384
                        i32.const 0
                        i32.const 1056364
                        i32.store offset=1056372
                        i32.const 0
                        i32.const 1056380
                        i32.store offset=1056392
                        i32.const 0
                        i32.const 1056372
                        i32.store offset=1056380
                        i32.const 0
                        i32.const 1056388
                        i32.store offset=1056400
                        i32.const 0
                        i32.const 1056380
                        i32.store offset=1056388
                        i32.const 0
                        i32.const 1056388
                        i32.store offset=1056396
                        i32.const 0
                        i32.const 1056396
                        i32.store offset=1056408
                        i32.const 0
                        i32.const 1056396
                        i32.store offset=1056404
                        i32.const 0
                        i32.const 1056404
                        i32.store offset=1056416
                        i32.const 0
                        i32.const 1056404
                        i32.store offset=1056412
                        i32.const 0
                        i32.const 1056412
                        i32.store offset=1056424
                        i32.const 0
                        i32.const 1056412
                        i32.store offset=1056420
                        i32.const 0
                        i32.const 1056420
                        i32.store offset=1056432
                        i32.const 0
                        i32.const 1056420
                        i32.store offset=1056428
                        i32.const 0
                        i32.const 1056428
                        i32.store offset=1056440
                        i32.const 0
                        i32.const 1056428
                        i32.store offset=1056436
                        i32.const 0
                        i32.const 1056436
                        i32.store offset=1056448
                        i32.const 0
                        i32.const 1056436
                        i32.store offset=1056444
                        i32.const 0
                        i32.const 1056444
                        i32.store offset=1056456
                        i32.const 0
                        i32.const 1056444
                        i32.store offset=1056452
                        i32.const 0
                        i32.const 1056452
                        i32.store offset=1056464
                        i32.const 0
                        i32.const 1056460
                        i32.store offset=1056472
                        i32.const 0
                        i32.const 1056452
                        i32.store offset=1056460
                        i32.const 0
                        i32.const 1056468
                        i32.store offset=1056480
                        i32.const 0
                        i32.const 1056460
                        i32.store offset=1056468
                        i32.const 0
                        i32.const 1056476
                        i32.store offset=1056488
                        i32.const 0
                        i32.const 1056468
                        i32.store offset=1056476
                        i32.const 0
                        i32.const 1056484
                        i32.store offset=1056496
                        i32.const 0
                        i32.const 1056476
                        i32.store offset=1056484
                        i32.const 0
                        i32.const 1056492
                        i32.store offset=1056504
                        i32.const 0
                        i32.const 1056484
                        i32.store offset=1056492
                        i32.const 0
                        i32.const 1056500
                        i32.store offset=1056512
                        i32.const 0
                        i32.const 1056492
                        i32.store offset=1056500
                        i32.const 0
                        i32.const 1056508
                        i32.store offset=1056520
                        i32.const 0
                        i32.const 1056500
                        i32.store offset=1056508
                        i32.const 0
                        i32.const 1056516
                        i32.store offset=1056528
                        i32.const 0
                        i32.const 1056508
                        i32.store offset=1056516
                        i32.const 0
                        i32.const 1056524
                        i32.store offset=1056536
                        i32.const 0
                        i32.const 1056516
                        i32.store offset=1056524
                        i32.const 0
                        i32.const 1056532
                        i32.store offset=1056544
                        i32.const 0
                        i32.const 1056524
                        i32.store offset=1056532
                        i32.const 0
                        i32.const 1056540
                        i32.store offset=1056552
                        i32.const 0
                        i32.const 1056532
                        i32.store offset=1056540
                        i32.const 0
                        i32.const 1056548
                        i32.store offset=1056560
                        i32.const 0
                        i32.const 1056540
                        i32.store offset=1056548
                        i32.const 0
                        i32.const 1056556
                        i32.store offset=1056568
                        i32.const 0
                        i32.const 1056548
                        i32.store offset=1056556
                        i32.const 0
                        i32.const 1056564
                        i32.store offset=1056576
                        i32.const 0
                        i32.const 1056556
                        i32.store offset=1056564
                        i32.const 0
                        i32.const 1056572
                        i32.store offset=1056584
                        i32.const 0
                        i32.const 1056564
                        i32.store offset=1056572
                        i32.const 0
                        local.get 6
                        i32.const 15
                        i32.add
                        i32.const -8
                        i32.and
                        local.tee 0
                        i32.const -8
                        i32.add
                        local.tee 2
                        i32.store offset=1056608
                        i32.const 0
                        i32.const 1056572
                        i32.store offset=1056580
                        i32.const 0
                        local.get 6
                        local.get 0
                        i32.sub
                        local.get 9
                        i32.const -40
                        i32.add
                        local.tee 0
                        i32.add
                        i32.const 8
                        i32.add
                        local.tee 8
                        i32.store offset=1056600
                        local.get 2
                        local.get 8
                        i32.const 1
                        i32.or
                        i32.store offset=4
                        local.get 6
                        local.get 0
                        i32.add
                        i32.const 40
                        i32.store offset=4
                        i32.const 0
                        i32.const 2097152
                        i32.store offset=1056620
                        br 8 (;@2;)
                      end
                      local.get 2
                      local.get 6
                      i32.ge_u
                      br_if 0 (;@9;)
                      local.get 8
                      local.get 2
                      i32.gt_u
                      br_if 0 (;@9;)
                      local.get 0
                      i32.load offset=12
                      local.tee 8
                      i32.const 1
                      i32.and
                      br_if 0 (;@9;)
                      local.get 8
                      i32.const 1
                      i32.shr_u
                      local.get 5
                      i32.eq
                      br_if 3 (;@6;)
                    end
                    i32.const 0
                    i32.const 0
                    i32.load offset=1056624
                    local.tee 0
                    local.get 6
                    local.get 0
                    local.get 6
                    i32.lt_u
                    select
                    i32.store offset=1056624
                    local.get 6
                    local.get 9
                    i32.add
                    local.set 8
                    i32.const 1056308
                    local.set 0
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          loop  ;; label = @12
                            local.get 0
                            i32.load
                            local.tee 7
                            local.get 8
                            i32.eq
                            br_if 1 (;@11;)
                            local.get 0
                            i32.load offset=8
                            local.tee 0
                            br_if 0 (;@12;)
                            br 2 (;@10;)
                          end
                        end
                        local.get 0
                        i32.load offset=12
                        local.tee 8
                        i32.const 1
                        i32.and
                        br_if 0 (;@10;)
                        local.get 8
                        i32.const 1
                        i32.shr_u
                        local.get 5
                        i32.eq
                        br_if 1 (;@9;)
                      end
                      i32.const 1056308
                      local.set 0
                      block  ;; label = @10
                        loop  ;; label = @11
                          block  ;; label = @12
                            local.get 0
                            i32.load
                            local.tee 8
                            local.get 2
                            i32.gt_u
                            br_if 0 (;@12;)
                            local.get 2
                            local.get 8
                            local.get 0
                            i32.load offset=4
                            i32.add
                            local.tee 8
                            i32.lt_u
                            br_if 2 (;@10;)
                          end
                          local.get 0
                          i32.load offset=8
                          local.set 0
                          br 0 (;@11;)
                        end
                      end
                      i32.const 0
                      local.get 6
                      i32.const 15
                      i32.add
                      i32.const -8
                      i32.and
                      local.tee 0
                      i32.const -8
                      i32.add
                      local.tee 7
                      i32.store offset=1056608
                      i32.const 0
                      local.get 6
                      local.get 0
                      i32.sub
                      local.get 9
                      i32.const -40
                      i32.add
                      local.tee 0
                      i32.add
                      i32.const 8
                      i32.add
                      local.tee 4
                      i32.store offset=1056600
                      local.get 7
                      local.get 4
                      i32.const 1
                      i32.or
                      i32.store offset=4
                      local.get 6
                      local.get 0
                      i32.add
                      i32.const 40
                      i32.store offset=4
                      i32.const 0
                      i32.const 2097152
                      i32.store offset=1056620
                      local.get 2
                      local.get 8
                      i32.const -32
                      i32.add
                      i32.const -8
                      i32.and
                      i32.const -8
                      i32.add
                      local.tee 0
                      local.get 0
                      local.get 2
                      i32.const 16
                      i32.add
                      i32.lt_u
                      select
                      local.tee 7
                      i32.const 27
                      i32.store offset=4
                      i32.const 0
                      i64.load offset=1056308 align=4
                      local.set 10
                      local.get 7
                      i32.const 16
                      i32.add
                      i32.const 0
                      i64.load offset=1056316 align=4
                      i64.store align=4
                      local.get 7
                      i32.const 8
                      i32.add
                      local.tee 0
                      local.get 10
                      i64.store align=4
                      i32.const 0
                      local.get 5
                      i32.store offset=1056320
                      i32.const 0
                      local.get 9
                      i32.store offset=1056312
                      i32.const 0
                      local.get 6
                      i32.store offset=1056308
                      i32.const 0
                      local.get 0
                      i32.store offset=1056316
                      local.get 7
                      i32.const 28
                      i32.add
                      local.set 0
                      loop  ;; label = @10
                        local.get 0
                        i32.const 7
                        i32.store
                        local.get 0
                        i32.const 4
                        i32.add
                        local.tee 0
                        local.get 8
                        i32.lt_u
                        br_if 0 (;@10;)
                      end
                      local.get 7
                      local.get 2
                      i32.eq
                      br_if 7 (;@2;)
                      local.get 7
                      local.get 7
                      i32.load offset=4
                      i32.const -2
                      i32.and
                      i32.store offset=4
                      local.get 2
                      local.get 7
                      local.get 2
                      i32.sub
                      local.tee 0
                      i32.const 1
                      i32.or
                      i32.store offset=4
                      local.get 7
                      local.get 0
                      i32.store
                      block  ;; label = @10
                        local.get 0
                        i32.const 256
                        i32.lt_u
                        br_if 0 (;@10;)
                        local.get 2
                        local.get 0
                        call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE18insert_large_chunkCsebHcaeoSrxy_3std
                        br 8 (;@2;)
                      end
                      block  ;; label = @10
                        block  ;; label = @11
                          i32.const 0
                          i32.load offset=1056588
                          local.tee 8
                          i32.const 1
                          local.get 0
                          i32.const 3
                          i32.shr_u
                          i32.shl
                          local.tee 6
                          i32.and
                          br_if 0 (;@11;)
                          i32.const 0
                          local.get 8
                          local.get 6
                          i32.or
                          i32.store offset=1056588
                          local.get 0
                          i32.const 248
                          i32.and
                          i32.const 1056324
                          i32.add
                          local.tee 0
                          local.set 8
                          br 1 (;@10;)
                        end
                        local.get 0
                        i32.const 248
                        i32.and
                        local.tee 0
                        i32.const 1056324
                        i32.add
                        local.set 8
                        local.get 0
                        i32.const 1056332
                        i32.add
                        i32.load
                        local.set 0
                      end
                      local.get 8
                      local.get 2
                      i32.store offset=8
                      local.get 0
                      local.get 2
                      i32.store offset=12
                      local.get 2
                      local.get 8
                      i32.store offset=12
                      local.get 2
                      local.get 0
                      i32.store offset=8
                      br 7 (;@2;)
                    end
                    local.get 0
                    local.get 6
                    i32.store
                    local.get 0
                    local.get 0
                    i32.load offset=4
                    local.get 9
                    i32.add
                    i32.store offset=4
                    local.get 6
                    i32.const 15
                    i32.add
                    i32.const -8
                    i32.and
                    i32.const -8
                    i32.add
                    local.tee 8
                    local.get 3
                    i32.const 3
                    i32.or
                    i32.store offset=4
                    local.get 7
                    i32.const 15
                    i32.add
                    i32.const -8
                    i32.and
                    i32.const -8
                    i32.add
                    local.tee 2
                    local.get 8
                    local.get 3
                    i32.add
                    local.tee 0
                    i32.sub
                    local.set 3
                    local.get 2
                    i32.const 0
                    i32.load offset=1056608
                    i32.eq
                    br_if 3 (;@5;)
                    local.get 2
                    i32.const 0
                    i32.load offset=1056604
                    i32.eq
                    br_if 4 (;@4;)
                    block  ;; label = @9
                      local.get 2
                      i32.load offset=4
                      local.tee 6
                      i32.const 3
                      i32.and
                      i32.const 1
                      i32.ne
                      br_if 0 (;@9;)
                      local.get 2
                      local.get 6
                      i32.const -8
                      i32.and
                      local.tee 6
                      call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE12unlink_chunkCsebHcaeoSrxy_3std
                      local.get 6
                      local.get 3
                      i32.add
                      local.set 3
                      local.get 2
                      local.get 6
                      i32.add
                      local.tee 2
                      i32.load offset=4
                      local.set 6
                    end
                    local.get 2
                    local.get 6
                    i32.const -2
                    i32.and
                    i32.store offset=4
                    local.get 0
                    local.get 3
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    local.get 0
                    local.get 3
                    i32.add
                    local.get 3
                    i32.store
                    block  ;; label = @9
                      local.get 3
                      i32.const 256
                      i32.lt_u
                      br_if 0 (;@9;)
                      local.get 0
                      local.get 3
                      call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE18insert_large_chunkCsebHcaeoSrxy_3std
                      br 6 (;@3;)
                    end
                    block  ;; label = @9
                      block  ;; label = @10
                        i32.const 0
                        i32.load offset=1056588
                        local.tee 2
                        i32.const 1
                        local.get 3
                        i32.const 3
                        i32.shr_u
                        i32.shl
                        local.tee 6
                        i32.and
                        br_if 0 (;@10;)
                        i32.const 0
                        local.get 2
                        local.get 6
                        i32.or
                        i32.store offset=1056588
                        local.get 3
                        i32.const 248
                        i32.and
                        i32.const 1056324
                        i32.add
                        local.tee 3
                        local.set 2
                        br 1 (;@9;)
                      end
                      local.get 3
                      i32.const 248
                      i32.and
                      local.tee 3
                      i32.const 1056324
                      i32.add
                      local.set 2
                      local.get 3
                      i32.const 1056332
                      i32.add
                      i32.load
                      local.set 3
                    end
                    local.get 2
                    local.get 0
                    i32.store offset=8
                    local.get 3
                    local.get 0
                    i32.store offset=12
                    local.get 0
                    local.get 2
                    i32.store offset=12
                    local.get 0
                    local.get 3
                    i32.store offset=8
                    br 5 (;@3;)
                  end
                  i32.const 0
                  local.get 0
                  local.get 3
                  i32.sub
                  local.tee 2
                  i32.store offset=1056600
                  i32.const 0
                  i32.const 0
                  i32.load offset=1056608
                  local.tee 0
                  local.get 3
                  i32.add
                  local.tee 8
                  i32.store offset=1056608
                  local.get 8
                  local.get 2
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  local.get 0
                  local.get 3
                  i32.const 3
                  i32.or
                  i32.store offset=4
                  local.get 0
                  i32.const 8
                  i32.add
                  local.set 0
                  br 6 (;@1;)
                end
                i32.const 0
                i32.load offset=1056604
                local.set 2
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 0
                    local.get 3
                    i32.sub
                    local.tee 8
                    i32.const 15
                    i32.gt_u
                    br_if 0 (;@8;)
                    i32.const 0
                    i32.const 0
                    i32.store offset=1056604
                    i32.const 0
                    i32.const 0
                    i32.store offset=1056596
                    local.get 2
                    local.get 0
                    i32.const 3
                    i32.or
                    i32.store offset=4
                    local.get 2
                    local.get 0
                    i32.add
                    local.tee 0
                    local.get 0
                    i32.load offset=4
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    br 1 (;@7;)
                  end
                  i32.const 0
                  local.get 8
                  i32.store offset=1056596
                  i32.const 0
                  local.get 2
                  local.get 3
                  i32.add
                  local.tee 6
                  i32.store offset=1056604
                  local.get 6
                  local.get 8
                  i32.const 1
                  i32.or
                  i32.store offset=4
                  local.get 2
                  local.get 0
                  i32.add
                  local.get 8
                  i32.store
                  local.get 2
                  local.get 3
                  i32.const 3
                  i32.or
                  i32.store offset=4
                end
                local.get 2
                i32.const 8
                i32.add
                local.set 0
                br 5 (;@1;)
              end
              local.get 0
              local.get 7
              local.get 9
              i32.add
              i32.store offset=4
              i32.const 0
              i32.const 0
              i32.load offset=1056608
              local.tee 0
              i32.const 15
              i32.add
              i32.const -8
              i32.and
              local.tee 2
              i32.const -8
              i32.add
              local.tee 8
              i32.store offset=1056608
              i32.const 0
              local.get 0
              local.get 2
              i32.sub
              i32.const 0
              i32.load offset=1056600
              local.get 9
              i32.add
              local.tee 2
              i32.add
              i32.const 8
              i32.add
              local.tee 6
              i32.store offset=1056600
              local.get 8
              local.get 6
              i32.const 1
              i32.or
              i32.store offset=4
              local.get 0
              local.get 2
              i32.add
              i32.const 40
              i32.store offset=4
              i32.const 0
              i32.const 2097152
              i32.store offset=1056620
              br 3 (;@2;)
            end
            i32.const 0
            local.get 0
            i32.store offset=1056608
            i32.const 0
            i32.const 0
            i32.load offset=1056600
            local.get 3
            i32.add
            local.tee 3
            i32.store offset=1056600
            local.get 0
            local.get 3
            i32.const 1
            i32.or
            i32.store offset=4
            br 1 (;@3;)
          end
          i32.const 0
          local.get 0
          i32.store offset=1056604
          i32.const 0
          i32.const 0
          i32.load offset=1056596
          local.get 3
          i32.add
          local.tee 3
          i32.store offset=1056596
          local.get 0
          local.get 3
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 0
          local.get 3
          i32.add
          local.get 3
          i32.store
        end
        local.get 8
        i32.const 8
        i32.add
        local.set 0
        br 1 (;@1;)
      end
      i32.const 0
      local.set 0
      i32.const 0
      i32.load offset=1056600
      local.tee 2
      local.get 3
      i32.le_u
      br_if 0 (;@1;)
      i32.const 0
      local.get 2
      local.get 3
      i32.sub
      local.tee 2
      i32.store offset=1056600
      i32.const 0
      i32.const 0
      i32.load offset=1056608
      local.tee 0
      local.get 3
      i32.add
      local.tee 8
      i32.store offset=1056608
      local.get 8
      local.get 2
      i32.const 1
      i32.or
      i32.store offset=4
      local.get 0
      local.get 3
      i32.const 3
      i32.or
      i32.store offset=4
      local.get 0
      i32.const 8
      i32.add
      local.set 0
    end
    local.get 1
    i32.const 16
    i32.add
    global.set $__stack_pointer
    local.get 0)
  (func $_RNvCsfLfy6EI15iL_7___rustc12___rust_abort (type 10)
    unreachable)
  (func $_RNvCsfLfy6EI15iL_7___rustc13___rdl_dealloc (type 9) (param i32 i32 i32)
    (local i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 0
        i32.const -4
        i32.add
        i32.load
        local.tee 3
        i32.const -8
        i32.and
        local.tee 4
        i32.const 4
        i32.const 8
        local.get 3
        i32.const 3
        i32.and
        local.tee 3
        select
        local.get 1
        i32.add
        i32.lt_u
        br_if 0 (;@2;)
        block  ;; label = @3
          local.get 3
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          local.get 1
          i32.const 39
          i32.add
          i32.gt_u
          br_if 2 (;@1;)
        end
        local.get 0
        call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE4freeCsebHcaeoSrxy_3std
        return
      end
      i32.const 1050372
      i32.const 46
      i32.const 1050420
      call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
      unreachable
    end
    i32.const 1050436
    i32.const 46
    i32.const 1050484
    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
    unreachable)
  (func $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE4freeCsebHcaeoSrxy_3std (type 0) (param i32)
    (local i32 i32 i32 i32)
    local.get 0
    i32.const -8
    i32.add
    local.tee 1
    local.get 0
    i32.const -4
    i32.add
    i32.load
    local.tee 2
    i32.const -8
    i32.and
    local.tee 0
    i32.add
    local.set 3
    block  ;; label = @1
      block  ;; label = @2
        local.get 2
        i32.const 1
        i32.and
        br_if 0 (;@2;)
        local.get 2
        i32.const 2
        i32.and
        i32.eqz
        br_if 1 (;@1;)
        local.get 1
        i32.load
        local.tee 2
        local.get 0
        i32.add
        local.set 0
        block  ;; label = @3
          local.get 1
          local.get 2
          i32.sub
          local.tee 1
          i32.const 0
          i32.load offset=1056604
          i32.ne
          br_if 0 (;@3;)
          local.get 3
          i32.load offset=4
          i32.const 3
          i32.and
          i32.const 3
          i32.ne
          br_if 1 (;@2;)
          i32.const 0
          local.get 0
          i32.store offset=1056596
          local.get 3
          local.get 3
          i32.load offset=4
          i32.const -2
          i32.and
          i32.store offset=4
          local.get 1
          local.get 0
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 3
          local.get 0
          i32.store
          return
        end
        local.get 1
        local.get 2
        call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE12unlink_chunkCsebHcaeoSrxy_3std
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 3
                      i32.load offset=4
                      local.tee 2
                      i32.const 2
                      i32.and
                      br_if 0 (;@9;)
                      local.get 3
                      i32.const 0
                      i32.load offset=1056608
                      i32.eq
                      br_if 2 (;@7;)
                      local.get 3
                      i32.const 0
                      i32.load offset=1056604
                      i32.eq
                      br_if 3 (;@6;)
                      local.get 3
                      local.get 2
                      i32.const -8
                      i32.and
                      local.tee 2
                      call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE12unlink_chunkCsebHcaeoSrxy_3std
                      local.get 1
                      local.get 2
                      local.get 0
                      i32.add
                      local.tee 0
                      i32.const 1
                      i32.or
                      i32.store offset=4
                      local.get 1
                      local.get 0
                      i32.add
                      local.get 0
                      i32.store
                      local.get 1
                      i32.const 0
                      i32.load offset=1056604
                      i32.ne
                      br_if 1 (;@8;)
                      i32.const 0
                      local.get 0
                      i32.store offset=1056596
                      return
                    end
                    local.get 3
                    local.get 2
                    i32.const -2
                    i32.and
                    i32.store offset=4
                    local.get 1
                    local.get 0
                    i32.const 1
                    i32.or
                    i32.store offset=4
                    local.get 1
                    local.get 0
                    i32.add
                    local.get 0
                    i32.store
                  end
                  local.get 0
                  i32.const 256
                  i32.lt_u
                  br_if 4 (;@3;)
                  local.get 1
                  local.get 0
                  call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE18insert_large_chunkCsebHcaeoSrxy_3std
                  i32.const 0
                  i32.const 0
                  i32.load offset=1056628
                  i32.const -1
                  i32.add
                  local.tee 1
                  i32.store offset=1056628
                  local.get 1
                  br_if 6 (;@1;)
                  i32.const 0
                  i32.load offset=1056316
                  local.tee 0
                  br_if 2 (;@5;)
                  i32.const 4095
                  local.set 1
                  br 3 (;@4;)
                end
                i32.const 0
                local.get 1
                i32.store offset=1056608
                i32.const 0
                i32.const 0
                i32.load offset=1056600
                local.get 0
                i32.add
                local.tee 0
                i32.store offset=1056600
                local.get 1
                local.get 0
                i32.const 1
                i32.or
                i32.store offset=4
                block  ;; label = @7
                  local.get 1
                  i32.const 0
                  i32.load offset=1056604
                  i32.ne
                  br_if 0 (;@7;)
                  i32.const 0
                  i32.const 0
                  i32.store offset=1056596
                  i32.const 0
                  i32.const 0
                  i32.store offset=1056604
                end
                local.get 0
                i32.const 0
                i32.load offset=1056620
                local.tee 2
                i32.le_u
                br_if 5 (;@1;)
                i32.const 0
                i32.load offset=1056608
                local.tee 0
                i32.eqz
                br_if 5 (;@1;)
                i32.const 0
                i32.load offset=1056600
                local.tee 4
                i32.const 41
                i32.lt_u
                br_if 4 (;@2;)
                i32.const 1056308
                local.set 1
                loop  ;; label = @7
                  block  ;; label = @8
                    local.get 1
                    i32.load
                    local.tee 3
                    local.get 0
                    i32.gt_u
                    br_if 0 (;@8;)
                    local.get 0
                    local.get 3
                    local.get 1
                    i32.load offset=4
                    i32.add
                    i32.lt_u
                    br_if 6 (;@2;)
                  end
                  local.get 1
                  i32.load offset=8
                  local.set 1
                  br 0 (;@7;)
                end
              end
              i32.const 0
              local.get 1
              i32.store offset=1056604
              i32.const 0
              i32.const 0
              i32.load offset=1056596
              local.get 0
              i32.add
              local.tee 0
              i32.store offset=1056596
              local.get 1
              local.get 0
              i32.const 1
              i32.or
              i32.store offset=4
              local.get 1
              local.get 0
              i32.add
              local.get 0
              i32.store
              return
            end
            i32.const 0
            local.set 1
            loop  ;; label = @5
              local.get 1
              i32.const 1
              i32.add
              local.set 1
              local.get 0
              i32.load offset=8
              local.tee 0
              br_if 0 (;@5;)
            end
            local.get 1
            i32.const 4095
            local.get 1
            i32.const 4095
            i32.gt_u
            select
            local.set 1
          end
          i32.const 0
          local.get 1
          i32.store offset=1056628
          return
        end
        block  ;; label = @3
          block  ;; label = @4
            i32.const 0
            i32.load offset=1056588
            local.tee 3
            i32.const 1
            local.get 0
            i32.const 3
            i32.shr_u
            i32.shl
            local.tee 2
            i32.and
            br_if 0 (;@4;)
            i32.const 0
            local.get 3
            local.get 2
            i32.or
            i32.store offset=1056588
            local.get 0
            i32.const 248
            i32.and
            i32.const 1056324
            i32.add
            local.tee 0
            local.set 3
            br 1 (;@3;)
          end
          local.get 0
          i32.const 248
          i32.and
          local.tee 0
          i32.const 1056324
          i32.add
          local.set 3
          local.get 0
          i32.const 1056332
          i32.add
          i32.load
          local.set 0
        end
        local.get 3
        local.get 1
        i32.store offset=8
        local.get 0
        local.get 1
        i32.store offset=12
        local.get 1
        local.get 3
        i32.store offset=12
        local.get 1
        local.get 0
        i32.store offset=8
        return
      end
      block  ;; label = @2
        block  ;; label = @3
          i32.const 0
          i32.load offset=1056316
          local.tee 0
          br_if 0 (;@3;)
          i32.const 4095
          local.set 1
          br 1 (;@2;)
        end
        i32.const 0
        local.set 1
        loop  ;; label = @3
          local.get 1
          i32.const 1
          i32.add
          local.set 1
          local.get 0
          i32.load offset=8
          local.tee 0
          br_if 0 (;@3;)
        end
        local.get 1
        i32.const 4095
        local.get 1
        i32.const 4095
        i32.gt_u
        select
        local.set 1
      end
      i32.const 0
      local.get 1
      i32.store offset=1056628
      local.get 4
      local.get 2
      i32.le_u
      br_if 0 (;@1;)
      i32.const 0
      i32.const -1
      i32.store offset=1056620
    end)
  (func $_RNvCsfLfy6EI15iL_7___rustc13___rdl_realloc (type 5) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 0
                    i32.const -4
                    i32.add
                    local.tee 4
                    i32.load
                    local.tee 5
                    i32.const -8
                    i32.and
                    local.tee 6
                    i32.const 4
                    i32.const 8
                    local.get 5
                    i32.const 3
                    i32.and
                    local.tee 7
                    select
                    local.get 1
                    i32.add
                    i32.lt_u
                    br_if 0 (;@8;)
                    local.get 1
                    i32.const 39
                    i32.add
                    local.set 8
                    block  ;; label = @9
                      local.get 7
                      i32.eqz
                      br_if 0 (;@9;)
                      local.get 6
                      local.get 8
                      i32.gt_u
                      br_if 2 (;@7;)
                    end
                    block  ;; label = @9
                      block  ;; label = @10
                        local.get 2
                        i32.const 9
                        i32.lt_u
                        br_if 0 (;@10;)
                        local.get 2
                        local.get 3
                        call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE8memalignCsebHcaeoSrxy_3std
                        local.tee 2
                        br_if 1 (;@9;)
                        i32.const 0
                        return
                      end
                      i32.const 0
                      local.set 2
                      local.get 3
                      i32.const -65588
                      i32.gt_u
                      br_if 8 (;@1;)
                      i32.const 16
                      local.get 3
                      i32.const 11
                      i32.add
                      i32.const -8
                      i32.and
                      local.get 3
                      i32.const 11
                      i32.lt_u
                      select
                      local.set 1
                      local.get 0
                      i32.const -8
                      i32.add
                      local.set 8
                      block  ;; label = @10
                        local.get 7
                        br_if 0 (;@10;)
                        local.get 1
                        i32.const 256
                        i32.lt_u
                        br_if 7 (;@3;)
                        local.get 8
                        i32.eqz
                        br_if 7 (;@3;)
                        local.get 6
                        local.get 1
                        i32.le_u
                        br_if 7 (;@3;)
                        local.get 6
                        local.get 1
                        i32.sub
                        i32.const 131072
                        i32.gt_u
                        br_if 7 (;@3;)
                        local.get 0
                        return
                      end
                      local.get 8
                      local.get 6
                      i32.add
                      local.set 7
                      block  ;; label = @10
                        block  ;; label = @11
                          local.get 6
                          local.get 1
                          i32.ge_u
                          br_if 0 (;@11;)
                          local.get 7
                          i32.const 0
                          i32.load offset=1056608
                          i32.eq
                          br_if 1 (;@10;)
                          block  ;; label = @12
                            local.get 7
                            i32.const 0
                            i32.load offset=1056604
                            i32.eq
                            br_if 0 (;@12;)
                            local.get 7
                            i32.load offset=4
                            local.tee 5
                            i32.const 2
                            i32.and
                            br_if 9 (;@3;)
                            local.get 5
                            i32.const -8
                            i32.and
                            local.tee 9
                            local.get 6
                            i32.add
                            local.tee 5
                            local.get 1
                            i32.lt_u
                            br_if 9 (;@3;)
                            local.get 7
                            local.get 9
                            call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE12unlink_chunkCsebHcaeoSrxy_3std
                            block  ;; label = @13
                              local.get 5
                              local.get 1
                              i32.sub
                              local.tee 7
                              i32.const 16
                              i32.lt_u
                              br_if 0 (;@13;)
                              local.get 4
                              local.get 1
                              local.get 4
                              i32.load
                              i32.const 1
                              i32.and
                              i32.or
                              i32.const 2
                              i32.or
                              i32.store
                              local.get 8
                              local.get 1
                              i32.add
                              local.tee 1
                              local.get 7
                              i32.const 3
                              i32.or
                              i32.store offset=4
                              local.get 8
                              local.get 5
                              i32.add
                              local.tee 5
                              local.get 5
                              i32.load offset=4
                              i32.const 1
                              i32.or
                              i32.store offset=4
                              local.get 1
                              local.get 7
                              call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE13dispose_chunkCsebHcaeoSrxy_3std
                              br 9 (;@4;)
                            end
                            local.get 4
                            local.get 5
                            local.get 4
                            i32.load
                            i32.const 1
                            i32.and
                            i32.or
                            i32.const 2
                            i32.or
                            i32.store
                            local.get 8
                            local.get 5
                            i32.add
                            local.tee 1
                            local.get 1
                            i32.load offset=4
                            i32.const 1
                            i32.or
                            i32.store offset=4
                            br 8 (;@4;)
                          end
                          i32.const 0
                          i32.load offset=1056596
                          local.get 6
                          i32.add
                          local.tee 7
                          local.get 1
                          i32.lt_u
                          br_if 8 (;@3;)
                          block  ;; label = @12
                            block  ;; label = @13
                              local.get 7
                              local.get 1
                              i32.sub
                              local.tee 6
                              i32.const 15
                              i32.gt_u
                              br_if 0 (;@13;)
                              local.get 4
                              local.get 5
                              i32.const 1
                              i32.and
                              local.get 7
                              i32.or
                              i32.const 2
                              i32.or
                              i32.store
                              local.get 8
                              local.get 7
                              i32.add
                              local.tee 1
                              local.get 1
                              i32.load offset=4
                              i32.const 1
                              i32.or
                              i32.store offset=4
                              i32.const 0
                              local.set 6
                              i32.const 0
                              local.set 1
                              br 1 (;@12;)
                            end
                            local.get 4
                            local.get 1
                            local.get 5
                            i32.const 1
                            i32.and
                            i32.or
                            i32.const 2
                            i32.or
                            i32.store
                            local.get 8
                            local.get 1
                            i32.add
                            local.tee 1
                            local.get 6
                            i32.const 1
                            i32.or
                            i32.store offset=4
                            local.get 8
                            local.get 7
                            i32.add
                            local.tee 7
                            local.get 6
                            i32.store
                            local.get 7
                            local.get 7
                            i32.load offset=4
                            i32.const -2
                            i32.and
                            i32.store offset=4
                          end
                          i32.const 0
                          local.get 1
                          i32.store offset=1056604
                          i32.const 0
                          local.get 6
                          i32.store offset=1056596
                          br 7 (;@4;)
                        end
                        local.get 6
                        local.get 1
                        i32.sub
                        local.tee 6
                        i32.const 15
                        i32.le_u
                        br_if 6 (;@4;)
                        local.get 4
                        local.get 1
                        local.get 5
                        i32.const 1
                        i32.and
                        i32.or
                        i32.const 2
                        i32.or
                        i32.store
                        local.get 8
                        local.get 1
                        i32.add
                        local.tee 1
                        local.get 6
                        i32.const 3
                        i32.or
                        i32.store offset=4
                        local.get 7
                        local.get 7
                        i32.load offset=4
                        i32.const 1
                        i32.or
                        i32.store offset=4
                        local.get 1
                        local.get 6
                        call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE13dispose_chunkCsebHcaeoSrxy_3std
                        br 6 (;@4;)
                      end
                      i32.const 0
                      i32.load offset=1056600
                      local.get 6
                      i32.add
                      local.tee 7
                      local.get 1
                      i32.gt_u
                      br_if 4 (;@5;)
                      br 6 (;@3;)
                    end
                    block  ;; label = @9
                      local.get 3
                      local.get 1
                      local.get 3
                      local.get 1
                      i32.lt_u
                      select
                      local.tee 3
                      i32.eqz
                      br_if 0 (;@9;)
                      local.get 2
                      local.get 0
                      local.get 3
                      memory.copy
                    end
                    local.get 4
                    i32.load
                    local.tee 3
                    i32.const -8
                    i32.and
                    local.tee 7
                    i32.const 4
                    i32.const 8
                    local.get 3
                    i32.const 3
                    i32.and
                    local.tee 3
                    select
                    local.get 1
                    i32.add
                    i32.lt_u
                    br_if 2 (;@6;)
                    local.get 3
                    i32.eqz
                    br_if 6 (;@2;)
                    local.get 7
                    local.get 8
                    i32.le_u
                    br_if 6 (;@2;)
                    i32.const 1050436
                    i32.const 46
                    i32.const 1050484
                    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
                    unreachable
                  end
                  i32.const 1050372
                  i32.const 46
                  i32.const 1050420
                  call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
                  unreachable
                end
                i32.const 1050436
                i32.const 46
                i32.const 1050484
                call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
                unreachable
              end
              i32.const 1050372
              i32.const 46
              i32.const 1050420
              call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic
              unreachable
            end
            local.get 4
            local.get 1
            local.get 5
            i32.const 1
            i32.and
            i32.or
            i32.const 2
            i32.or
            i32.store
            local.get 8
            local.get 1
            i32.add
            local.tee 5
            local.get 7
            local.get 1
            i32.sub
            local.tee 1
            i32.const 1
            i32.or
            i32.store offset=4
            i32.const 0
            local.get 1
            i32.store offset=1056600
            i32.const 0
            local.get 5
            i32.store offset=1056608
          end
          local.get 8
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          return
        end
        local.get 3
        call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE6mallocCsebHcaeoSrxy_3std
        local.tee 1
        i32.eqz
        br_if 1 (;@1;)
        block  ;; label = @3
          local.get 3
          i32.const -4
          i32.const -8
          local.get 4
          i32.load
          local.tee 2
          i32.const 3
          i32.and
          select
          local.get 2
          i32.const -8
          i32.and
          i32.add
          local.tee 2
          local.get 3
          local.get 2
          i32.lt_u
          select
          local.tee 3
          i32.eqz
          br_if 0 (;@3;)
          local.get 1
          local.get 0
          local.get 3
          memory.copy
        end
        local.get 1
        local.set 2
      end
      local.get 0
      call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE4freeCsebHcaeoSrxy_3std
    end
    local.get 2)
  (func $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE12unlink_chunkCsebHcaeoSrxy_3std (type 1) (param i32 i32)
    (local i32 i32 i32 i32)
    local.get 0
    i32.load offset=12
    local.set 2
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 1
            i32.const 256
            i32.lt_u
            br_if 0 (;@4;)
            local.get 0
            i32.load offset=24
            local.set 3
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  local.get 2
                  local.get 0
                  i32.ne
                  br_if 0 (;@7;)
                  local.get 0
                  i32.const 20
                  i32.const 16
                  local.get 0
                  i32.load offset=20
                  local.tee 2
                  select
                  i32.add
                  i32.load
                  local.tee 1
                  br_if 1 (;@6;)
                  i32.const 0
                  local.set 2
                  br 2 (;@5;)
                end
                local.get 0
                i32.load offset=8
                local.tee 1
                local.get 2
                i32.store offset=12
                local.get 2
                local.get 1
                i32.store offset=8
                br 1 (;@5;)
              end
              local.get 0
              i32.const 20
              i32.add
              local.get 0
              i32.const 16
              i32.add
              local.get 2
              select
              local.set 4
              loop  ;; label = @6
                local.get 4
                local.set 5
                local.get 1
                local.tee 2
                i32.const 20
                i32.add
                local.get 2
                i32.const 16
                i32.add
                local.get 2
                i32.load offset=20
                local.tee 1
                select
                local.set 4
                local.get 2
                i32.const 20
                i32.const 16
                local.get 1
                select
                i32.add
                i32.load
                local.tee 1
                br_if 0 (;@6;)
              end
              local.get 5
              i32.const 0
              i32.store
            end
            local.get 3
            i32.eqz
            br_if 2 (;@2;)
            block  ;; label = @5
              block  ;; label = @6
                local.get 0
                local.get 0
                i32.load offset=28
                i32.const 2
                i32.shl
                i32.const 1056180
                i32.add
                local.tee 1
                i32.load
                i32.eq
                br_if 0 (;@6;)
                local.get 3
                i32.load offset=16
                local.get 0
                i32.eq
                br_if 1 (;@5;)
                local.get 3
                local.get 2
                i32.store offset=20
                local.get 2
                br_if 3 (;@3;)
                br 4 (;@2;)
              end
              local.get 1
              local.get 2
              i32.store
              local.get 2
              i32.eqz
              br_if 4 (;@1;)
              br 2 (;@3;)
            end
            local.get 3
            local.get 2
            i32.store offset=16
            local.get 2
            br_if 1 (;@3;)
            br 2 (;@2;)
          end
          block  ;; label = @4
            local.get 2
            local.get 0
            i32.load offset=8
            local.tee 4
            i32.eq
            br_if 0 (;@4;)
            local.get 4
            local.get 2
            i32.store offset=12
            local.get 2
            local.get 4
            i32.store offset=8
            return
          end
          i32.const 0
          i32.const 0
          i32.load offset=1056588
          i32.const -2
          local.get 1
          i32.const 3
          i32.shr_u
          i32.rotl
          i32.and
          i32.store offset=1056588
          return
        end
        local.get 2
        local.get 3
        i32.store offset=24
        block  ;; label = @3
          local.get 0
          i32.load offset=16
          local.tee 1
          i32.eqz
          br_if 0 (;@3;)
          local.get 2
          local.get 1
          i32.store offset=16
          local.get 1
          local.get 2
          i32.store offset=24
        end
        local.get 0
        i32.load offset=20
        local.tee 1
        i32.eqz
        br_if 0 (;@2;)
        local.get 2
        local.get 1
        i32.store offset=20
        local.get 1
        local.get 2
        i32.store offset=24
        return
      end
      return
    end
    i32.const 0
    i32.const 0
    i32.load offset=1056592
    i32.const -2
    local.get 0
    i32.load offset=28
    i32.rotl
    i32.and
    i32.store offset=1056592)
  (func $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE13dispose_chunkCsebHcaeoSrxy_3std (type 1) (param i32 i32)
    (local i32 i32)
    local.get 0
    local.get 1
    i32.add
    local.set 2
    block  ;; label = @1
      block  ;; label = @2
        local.get 0
        i32.load offset=4
        local.tee 3
        i32.const 1
        i32.and
        br_if 0 (;@2;)
        local.get 3
        i32.const 2
        i32.and
        i32.eqz
        br_if 1 (;@1;)
        local.get 0
        i32.load
        local.tee 3
        local.get 1
        i32.add
        local.set 1
        block  ;; label = @3
          local.get 0
          local.get 3
          i32.sub
          local.tee 0
          i32.const 0
          i32.load offset=1056604
          i32.ne
          br_if 0 (;@3;)
          local.get 2
          i32.load offset=4
          i32.const 3
          i32.and
          i32.const 3
          i32.ne
          br_if 1 (;@2;)
          i32.const 0
          local.get 1
          i32.store offset=1056596
          local.get 2
          local.get 2
          i32.load offset=4
          i32.const -2
          i32.and
          i32.store offset=4
          local.get 0
          local.get 1
          i32.const 1
          i32.or
          i32.store offset=4
          local.get 2
          local.get 1
          i32.store
          br 2 (;@1;)
        end
        local.get 0
        local.get 3
        call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE12unlink_chunkCsebHcaeoSrxy_3std
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 2
              i32.load offset=4
              local.tee 3
              i32.const 2
              i32.and
              br_if 0 (;@5;)
              local.get 2
              i32.const 0
              i32.load offset=1056608
              i32.eq
              br_if 2 (;@3;)
              local.get 2
              i32.const 0
              i32.load offset=1056604
              i32.eq
              br_if 3 (;@2;)
              local.get 2
              local.get 3
              i32.const -8
              i32.and
              local.tee 3
              call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE12unlink_chunkCsebHcaeoSrxy_3std
              local.get 0
              local.get 3
              local.get 1
              i32.add
              local.tee 1
              i32.const 1
              i32.or
              i32.store offset=4
              local.get 0
              local.get 1
              i32.add
              local.get 1
              i32.store
              local.get 0
              i32.const 0
              i32.load offset=1056604
              i32.ne
              br_if 1 (;@4;)
              i32.const 0
              local.get 1
              i32.store offset=1056596
              return
            end
            local.get 2
            local.get 3
            i32.const -2
            i32.and
            i32.store offset=4
            local.get 0
            local.get 1
            i32.const 1
            i32.or
            i32.store offset=4
            local.get 0
            local.get 1
            i32.add
            local.get 1
            i32.store
          end
          block  ;; label = @4
            local.get 1
            i32.const 256
            i32.lt_u
            br_if 0 (;@4;)
            local.get 0
            local.get 1
            call $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE18insert_large_chunkCsebHcaeoSrxy_3std
            return
          end
          block  ;; label = @4
            block  ;; label = @5
              i32.const 0
              i32.load offset=1056588
              local.tee 2
              i32.const 1
              local.get 1
              i32.const 3
              i32.shr_u
              i32.shl
              local.tee 3
              i32.and
              br_if 0 (;@5;)
              i32.const 0
              local.get 2
              local.get 3
              i32.or
              i32.store offset=1056588
              local.get 1
              i32.const 248
              i32.and
              i32.const 1056324
              i32.add
              local.tee 1
              local.set 2
              br 1 (;@4;)
            end
            local.get 1
            i32.const 248
            i32.and
            local.tee 1
            i32.const 1056324
            i32.add
            local.set 2
            local.get 1
            i32.const 1056332
            i32.add
            i32.load
            local.set 1
          end
          local.get 2
          local.get 0
          i32.store offset=8
          local.get 1
          local.get 0
          i32.store offset=12
          local.get 0
          local.get 2
          i32.store offset=12
          local.get 0
          local.get 1
          i32.store offset=8
          return
        end
        i32.const 0
        local.get 0
        i32.store offset=1056608
        i32.const 0
        i32.const 0
        i32.load offset=1056600
        local.get 1
        i32.add
        local.tee 1
        i32.store offset=1056600
        local.get 0
        local.get 1
        i32.const 1
        i32.or
        i32.store offset=4
        local.get 0
        i32.const 0
        i32.load offset=1056604
        i32.ne
        br_if 1 (;@1;)
        i32.const 0
        i32.const 0
        i32.store offset=1056596
        i32.const 0
        i32.const 0
        i32.store offset=1056604
        return
      end
      i32.const 0
      local.get 0
      i32.store offset=1056604
      i32.const 0
      i32.const 0
      i32.load offset=1056596
      local.get 1
      i32.add
      local.tee 1
      i32.store offset=1056596
      local.get 0
      local.get 1
      i32.const 1
      i32.or
      i32.store offset=4
      local.get 0
      local.get 1
      i32.add
      local.get 1
      i32.store
      return
    end)
  (func $_RNvCsfLfy6EI15iL_7___rustc17rust_begin_unwind (type 0) (param i32)
    (local i32 i64)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 1
    global.set $__stack_pointer
    local.get 0
    i64.load align=4
    local.set 2
    local.get 1
    local.get 0
    i32.store offset=12
    local.get 1
    local.get 2
    i64.store offset=4 align=4
    local.get 1
    i32.const 4
    i32.add
    call $_RINvNtNtCsebHcaeoSrxy_3std3sys9backtrace26___rust_end_short_backtraceNCNvNtB6_9panicking13panic_handler0zEB6_
    unreachable)
  (func $_RNvCsfLfy6EI15iL_7___rustc26___rust_alloc_error_handler (type 1) (param i32 i32)
    local.get 1
    local.get 0
    call $_RNvNtCsebHcaeoSrxy_3std5alloc8rust_oom
    unreachable)
  (func $_RNvNtCsebHcaeoSrxy_3std5alloc8rust_oom (type 1) (param i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    local.get 2
    local.get 1
    i32.store offset=12
    local.get 2
    local.get 0
    i32.store offset=8
    local.get 2
    i32.const 8
    i32.add
    call $_RINvNtNtCsebHcaeoSrxy_3std3sys9backtrace26___rust_end_short_backtraceNCNvNtB6_5alloc8rust_oom0zEB6_
    unreachable)
  (func $_RNvMs0_NtCsjqx8TIyZbP9_8dlmalloc8dlmallocINtB5_8DlmallocNtNtB7_3sys6SystemE18insert_large_chunkCsebHcaeoSrxy_3std (type 1) (param i32 i32)
    (local i32 i32 i32 i32)
    i32.const 0
    local.set 2
    block  ;; label = @1
      local.get 1
      i32.const 8
      i32.shr_u
      local.tee 3
      i32.eqz
      br_if 0 (;@1;)
      i32.const 31
      local.set 2
      local.get 1
      i32.const 16777216
      i32.ge_u
      br_if 0 (;@1;)
      local.get 1
      i32.const 38
      local.get 3
      i32.clz
      local.tee 2
      i32.sub
      i32.shr_u
      i32.const 1
      i32.and
      local.get 2
      i32.const 1
      i32.shl
      i32.or
      i32.const 62
      i32.xor
      local.set 2
    end
    local.get 0
    i64.const 0
    i64.store offset=16 align=4
    local.get 0
    local.get 2
    i32.store offset=28
    local.get 2
    i32.const 2
    i32.shl
    i32.const 1056180
    i32.add
    local.set 3
    block  ;; label = @1
      i32.const 0
      i32.load offset=1056592
      i32.const 1
      local.get 2
      i32.shl
      local.tee 4
      i32.and
      br_if 0 (;@1;)
      local.get 3
      local.get 0
      i32.store
      local.get 0
      local.get 3
      i32.store offset=24
      local.get 0
      local.get 0
      i32.store offset=12
      local.get 0
      local.get 0
      i32.store offset=8
      i32.const 0
      i32.const 0
      i32.load offset=1056592
      local.get 4
      i32.or
      i32.store offset=1056592
      return
    end
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          local.get 3
          i32.load
          local.tee 4
          i32.load offset=4
          i32.const -8
          i32.and
          local.get 1
          i32.ne
          br_if 0 (;@3;)
          local.get 4
          local.set 2
          br 1 (;@2;)
        end
        local.get 1
        i32.const 0
        i32.const 25
        local.get 2
        i32.const 1
        i32.shr_u
        i32.sub
        local.get 2
        i32.const 31
        i32.eq
        select
        i32.shl
        local.set 3
        loop  ;; label = @3
          local.get 4
          local.get 3
          i32.const 29
          i32.shr_u
          i32.const 4
          i32.and
          i32.add
          local.tee 5
          i32.load offset=16
          local.tee 2
          i32.eqz
          br_if 2 (;@1;)
          local.get 3
          i32.const 1
          i32.shl
          local.set 3
          local.get 2
          local.set 4
          local.get 2
          i32.load offset=4
          i32.const -8
          i32.and
          local.get 1
          i32.ne
          br_if 0 (;@3;)
        end
      end
      local.get 2
      i32.load offset=8
      local.tee 3
      local.get 0
      i32.store offset=12
      local.get 2
      local.get 0
      i32.store offset=8
      local.get 0
      i32.const 0
      i32.store offset=24
      local.get 0
      local.get 2
      i32.store offset=12
      local.get 0
      local.get 3
      i32.store offset=8
      return
    end
    local.get 5
    i32.const 16
    i32.add
    local.get 0
    i32.store
    local.get 0
    local.get 4
    i32.store offset=24
    local.get 0
    local.get 0
    i32.store offset=12
    local.get 0
    local.get 0
    i32.store offset=8)
  (func $_RNvNtNtCsebHcaeoSrxy_3std9panicking11panic_count8increase (type 20) (param i32) (result i32)
    (local i32 i32)
    i32.const 0
    local.set 1
    i32.const 0
    i32.const 0
    i32.load offset=1056176
    local.tee 2
    i32.const 1
    i32.add
    i32.store offset=1056176
    block  ;; label = @1
      local.get 2
      i32.const 0
      i32.lt_s
      br_if 0 (;@1;)
      i32.const 1
      local.set 1
      i32.const 0
      i32.load8_u offset=1056156
      br_if 0 (;@1;)
      i32.const 0
      local.get 0
      i32.store8 offset=1056156
      i32.const 0
      i32.const 0
      i32.load offset=1056152
      i32.const 1
      i32.add
      i32.store offset=1056152
      i32.const 2
      local.set 1
    end
    local.get 1)
  (func $_RNvXNtCsgXGp5Oqx2Ny_4core3anyNtNtCs5cOc02OMXlo_5alloc6string6StringNtB2_3Any7type_idCsebHcaeoSrxy_3std (type 1) (param i32 i32)
    local.get 0
    i32.const 0
    i64.load offset=1050364 align=4
    i64.store offset=8 align=4
    local.get 0
    i32.const 0
    i64.load offset=1050356 align=4
    i64.store align=4)
  (func $_RNvXNtCsgXGp5Oqx2Ny_4core3anyReNtB2_3Any7type_idCsebHcaeoSrxy_3std (type 1) (param i32 i32)
    local.get 0
    i32.const 0
    i64.load offset=1050348 align=4
    i64.store offset=8 align=4
    local.get 0
    i32.const 0
    i64.load offset=1050340 align=4
    i64.store align=4)
  (func $_RNvXs0_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB5_19FormatStringPayloadNtNtCsgXGp5Oqx2Ny_4core3fmt7Display3fmt (type 3) (param i32 i32) (result i32)
    block  ;; label = @1
      local.get 0
      i32.load
      i32.const -2147483648
      i32.eq
      br_if 0 (;@1;)
      local.get 1
      local.get 0
      i32.load offset=4
      local.get 0
      i32.load offset=8
      call $_RNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB5_9Formatter9write_str
      return
    end
    local.get 1
    i32.load
    local.get 1
    i32.load offset=4
    local.get 0
    i32.load offset=12
    i32.load
    local.tee 0
    i32.load
    local.get 0
    i32.load offset=4
    call $_RNvNtCsgXGp5Oqx2Ny_4core3fmt5write)
  (func $_RNvXs1_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB5_16StaticStrPayloadNtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload3get (type 1) (param i32 i32)
    local.get 0
    i32.const 1050500
    i32.store offset=4
    local.get 0
    local.get 1
    i32.store)
  (func $_RNvXs1_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB5_16StaticStrPayloadNtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload6as_str (type 1) (param i32 i32)
    local.get 0
    local.get 1
    i64.load align=4
    i64.store)
  (func $_RNvXs1_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB5_16StaticStrPayloadNtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload8take_box (type 1) (param i32 i32)
    (local i32 i32)
    local.get 1
    i32.load offset=4
    local.set 2
    local.get 1
    i32.load
    local.set 3
    call $_RNvCsfLfy6EI15iL_7___rustc35___rust_no_alloc_shim_is_unstable_v2
    block  ;; label = @1
      i32.const 8
      i32.const 4
      call $_RNvCsfLfy6EI15iL_7___rustc12___rust_alloc
      local.tee 1
      br_if 0 (;@1;)
      i32.const 4
      i32.const 8
      call $_RNvNtCs5cOc02OMXlo_5alloc5alloc18handle_alloc_error
      unreachable
    end
    local.get 1
    local.get 2
    i32.store offset=4
    local.get 1
    local.get 3
    i32.store
    local.get 0
    i32.const 1050500
    i32.store offset=4
    local.get 0
    local.get 1
    i32.store)
  (func $_RNvXs2_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB5_16StaticStrPayloadNtNtCsgXGp5Oqx2Ny_4core3fmt7Display3fmt (type 3) (param i32 i32) (result i32)
    local.get 1
    local.get 0
    i32.load
    local.get 0
    i32.load offset=4
    call $_RNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB5_9Formatter9write_str)
  (func $_RNvXsZ_NtCs5cOc02OMXlo_5alloc6stringNtB5_6StringNtNtCsgXGp5Oqx2Ny_4core3fmt5Write10write_char (type 3) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32)
    local.get 0
    i32.load offset=8
    local.set 2
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        i32.const 128
        i32.ge_u
        br_if 0 (;@2;)
        i32.const 1
        local.set 3
        br 1 (;@1;)
      end
      block  ;; label = @2
        local.get 1
        i32.const 2048
        i32.ge_u
        br_if 0 (;@2;)
        i32.const 2
        local.set 3
        br 1 (;@1;)
      end
      i32.const 3
      i32.const 4
      local.get 1
      i32.const 65536
      i32.lt_u
      select
      local.set 3
    end
    local.get 2
    local.set 4
    block  ;; label = @1
      local.get 3
      local.get 0
      i32.load
      local.get 2
      i32.sub
      i32.le_u
      br_if 0 (;@1;)
      local.get 0
      local.get 2
      local.get 3
      i32.const 1
      i32.const 1
      call $_RINvNvMs2_NtCs5cOc02OMXlo_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsebHcaeoSrxy_3std
      local.get 0
      i32.load offset=8
      local.set 4
    end
    local.get 0
    i32.load offset=4
    local.get 4
    i32.add
    local.set 4
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        i32.const 128
        i32.lt_u
        br_if 0 (;@2;)
        local.get 1
        i32.const 63
        i32.and
        i32.const -128
        i32.or
        local.set 5
        local.get 1
        i32.const 6
        i32.shr_u
        local.set 6
        block  ;; label = @3
          local.get 1
          i32.const 2048
          i32.ge_u
          br_if 0 (;@3;)
          local.get 4
          local.get 5
          i32.store8 offset=1
          local.get 4
          local.get 6
          i32.const 192
          i32.or
          i32.store8
          br 2 (;@1;)
        end
        local.get 1
        i32.const 12
        i32.shr_u
        local.set 7
        local.get 6
        i32.const 63
        i32.and
        i32.const -128
        i32.or
        local.set 6
        block  ;; label = @3
          local.get 1
          i32.const 65535
          i32.gt_u
          br_if 0 (;@3;)
          local.get 4
          local.get 5
          i32.store8 offset=2
          local.get 4
          local.get 6
          i32.store8 offset=1
          local.get 4
          local.get 7
          i32.const 224
          i32.or
          i32.store8
          br 2 (;@1;)
        end
        local.get 4
        local.get 5
        i32.store8 offset=3
        local.get 4
        local.get 6
        i32.store8 offset=2
        local.get 4
        local.get 7
        i32.const 63
        i32.and
        i32.const -128
        i32.or
        i32.store8 offset=1
        local.get 4
        local.get 1
        i32.const 18
        i32.shr_u
        i32.const -16
        i32.or
        i32.store8
        br 1 (;@1;)
      end
      local.get 4
      local.get 1
      i32.store8
    end
    local.get 0
    local.get 3
    local.get 2
    i32.add
    i32.store offset=8
    i32.const 0)
  (func $_RNvXsZ_NtCs5cOc02OMXlo_5alloc6stringNtB5_6StringNtNtCsgXGp5Oqx2Ny_4core3fmt5Write9write_str (type 2) (param i32 i32 i32) (result i32)
    (local i32)
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          local.get 2
          local.get 0
          i32.load
          local.get 0
          i32.load offset=8
          local.tee 3
          i32.sub
          i32.le_u
          br_if 0 (;@3;)
          local.get 0
          local.get 3
          local.get 2
          i32.const 1
          i32.const 1
          call $_RINvNvMs2_NtCs5cOc02OMXlo_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECsebHcaeoSrxy_3std
          local.get 0
          i32.load offset=8
          local.set 3
          br 1 (;@2;)
        end
        local.get 2
        i32.eqz
        br_if 1 (;@1;)
      end
      local.get 2
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      i32.load offset=4
      local.get 3
      i32.add
      local.get 1
      local.get 2
      memory.copy
    end
    local.get 0
    local.get 3
    local.get 2
    i32.add
    i32.store offset=8
    i32.const 0)
  (func $_RNvXs_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB4_19FormatStringPayloadNtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload3get (type 1) (param i32 i32)
    (local i32 i32 i64)
    global.get $__stack_pointer
    i32.const 32
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    block  ;; label = @1
      local.get 1
      i32.load
      i32.const -2147483648
      i32.ne
      br_if 0 (;@1;)
      local.get 1
      i32.load offset=12
      local.set 3
      local.get 2
      i32.const 0
      i32.store offset=28
      local.get 2
      i64.const 4294967296
      i64.store offset=20 align=4
      local.get 2
      i32.const 20
      i32.add
      i32.const 1050260
      local.get 3
      i32.load
      local.tee 3
      i32.load
      local.get 3
      i32.load offset=4
      call $_RNvNtCsgXGp5Oqx2Ny_4core3fmt5write
      drop
      local.get 2
      local.get 2
      i32.load offset=28
      local.tee 3
      i32.store offset=16
      local.get 2
      local.get 2
      i64.load offset=20 align=4
      local.tee 4
      i64.store offset=8
      local.get 1
      local.get 3
      i32.store offset=8
      local.get 1
      local.get 4
      i64.store align=4
    end
    local.get 0
    i32.const 1050516
    i32.store offset=4
    local.get 0
    local.get 1
    i32.store
    local.get 2
    i32.const 32
    i32.add
    global.set $__stack_pointer)
  (func $_RNvXs_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB4_19FormatStringPayloadNtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload8take_box (type 1) (param i32 i32)
    (local i32 i32 i64)
    global.get $__stack_pointer
    i32.const 48
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    block  ;; label = @1
      local.get 1
      i32.load
      i32.const -2147483648
      i32.ne
      br_if 0 (;@1;)
      local.get 1
      i32.load offset=12
      local.set 3
      local.get 2
      i32.const 0
      i32.store offset=44
      local.get 2
      i64.const 4294967296
      i64.store offset=36 align=4
      local.get 2
      i32.const 36
      i32.add
      i32.const 1050260
      local.get 3
      i32.load
      local.tee 3
      i32.load
      local.get 3
      i32.load offset=4
      call $_RNvNtCsgXGp5Oqx2Ny_4core3fmt5write
      drop
      local.get 2
      local.get 2
      i32.load offset=44
      local.tee 3
      i32.store offset=32
      local.get 2
      local.get 2
      i64.load offset=36 align=4
      local.tee 4
      i64.store offset=24
      local.get 1
      local.get 3
      i32.store offset=8
      local.get 1
      local.get 4
      i64.store align=4
    end
    local.get 1
    i32.load offset=8
    local.set 3
    local.get 1
    i32.const 0
    i32.store offset=8
    local.get 1
    i64.load align=4
    local.set 4
    local.get 1
    i64.const 4294967296
    i64.store align=4
    local.get 2
    local.get 3
    i32.store offset=16
    local.get 2
    local.get 4
    i64.store offset=8
    call $_RNvCsfLfy6EI15iL_7___rustc35___rust_no_alloc_shim_is_unstable_v2
    block  ;; label = @1
      i32.const 12
      i32.const 4
      call $_RNvCsfLfy6EI15iL_7___rustc12___rust_alloc
      local.tee 1
      br_if 0 (;@1;)
      i32.const 4
      i32.const 12
      call $_RNvNtCs5cOc02OMXlo_5alloc5alloc18handle_alloc_error
      unreachable
    end
    local.get 1
    local.get 2
    i32.load offset=16
    i32.store offset=8
    local.get 1
    local.get 2
    i64.load offset=8
    i64.store align=4
    local.get 0
    i32.const 1050516
    i32.store offset=4
    local.get 0
    local.get 1
    i32.store
    local.get 2
    i32.const 48
    i32.add
    global.set $__stack_pointer)
  (func $_RNvYINtNvNtCsebHcaeoSrxy_3std9panicking11begin_panic7PayloadReENtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload6as_strB9_ (type 1) (param i32 i32)
    local.get 0
    i32.const 0
    i32.store)
  (func $_RNvYNtNtCs5cOc02OMXlo_5alloc6string6StringNtNtCsgXGp5Oqx2Ny_4core3fmt5Write9write_fmtCsebHcaeoSrxy_3std (type 2) (param i32 i32 i32) (result i32)
    local.get 0
    i32.const 1050260
    local.get 1
    local.get 2
    call $_RNvNtCsgXGp5Oqx2Ny_4core3fmt5write)
  (func $_RNvXs_NtCsjqx8TIyZbP9_8dlmalloc3sysNtB4_6SystemNtB6_9Allocator5alloc (type 9) (param i32 i32 i32)
    (local i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 2
        i32.const 16
        i32.shr_u
        local.get 2
        i32.const 65535
        i32.and
        i32.const 0
        i32.ne
        i32.add
        local.tee 2
        memory.grow
        local.tee 3
        i32.const -1
        i32.ne
        br_if 0 (;@2;)
        i32.const 0
        local.set 2
        i32.const 0
        local.set 4
        br 1 (;@1;)
      end
      local.get 2
      i32.const 16
      i32.shl
      local.tee 4
      i32.const -16
      i32.add
      local.get 4
      local.get 3
      i32.const 16
      i32.shl
      local.tee 2
      i32.const 0
      local.get 4
      i32.sub
      i32.eq
      select
      local.set 4
    end
    local.get 0
    i32.const 0
    i32.store offset=8
    local.get 0
    local.get 4
    i32.store offset=4
    local.get 0
    local.get 2
    i32.store)
  (func $_RNvNtCs5cOc02OMXlo_5alloc7raw_vec12handle_error (type 1) (param i32 i32)
    block  ;; label = @1
      local.get 0
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      local.get 1
      call $_RNvNtCs5cOc02OMXlo_5alloc5alloc18handle_alloc_error
      unreachable
    end
    call $_RNvNtCs5cOc02OMXlo_5alloc7raw_vec17capacity_overflow
    unreachable)
  (func $_RNvNtCs5cOc02OMXlo_5alloc5alloc18handle_alloc_error (type 1) (param i32 i32)
    local.get 1
    local.get 0
    call $_RNvCsfLfy6EI15iL_7___rustc26___rust_alloc_error_handler
    unreachable)
  (func $_RNvNtCs5cOc02OMXlo_5alloc7raw_vec17capacity_overflow (type 10)
    i32.const 1050532
    i32.const 35
    i32.const 1050552
    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking9panic_fmt
    unreachable)
  (func $_RNvNtCsgXGp5Oqx2Ny_4core9panicking5panic (type 9) (param i32 i32 i32)
    local.get 0
    local.get 1
    i32.const 1
    i32.shl
    i32.const 1
    i32.or
    local.get 2
    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking9panic_fmt
    unreachable)
  (func $_RNvNtCsgXGp5Oqx2Ny_4core9panicking9panic_fmt (type 9) (param i32 i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 32
    i32.sub
    local.tee 3
    global.set $__stack_pointer
    local.get 3
    local.get 1
    i32.store offset=16
    local.get 3
    local.get 0
    i32.store offset=12
    local.get 3
    i32.const 1
    i32.store16 offset=28
    local.get 3
    local.get 2
    i32.store offset=24
    local.get 3
    local.get 3
    i32.const 12
    i32.add
    i32.store offset=20
    local.get 3
    i32.const 20
    i32.add
    call $_RNvCsfLfy6EI15iL_7___rustc17rust_begin_unwind
    unreachable)
  (func $_RNvXs1i_NtCsgXGp5Oqx2Ny_4core3fmtReNtB6_7Display3fmtB8_ (type 3) (param i32 i32) (result i32)
    local.get 1
    local.get 0
    i32.load
    local.get 0
    i32.load offset=4
    call $_RNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB5_9Formatter3pad)
  (func $_RNvNtCsgXGp5Oqx2Ny_4core3fmt5write (type 5) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 4
    global.set $__stack_pointer
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          local.get 3
          i32.const 1
          i32.and
          br_if 0 (;@3;)
          local.get 2
          i32.load8_u
          local.tee 5
          br_if 1 (;@2;)
          i32.const 0
          local.set 5
          br 2 (;@1;)
        end
        local.get 0
        local.get 2
        local.get 3
        i32.const 1
        i32.shr_u
        local.get 1
        i32.load offset=12
        call_indirect (type 2)
        local.set 5
        br 1 (;@1;)
      end
      local.get 1
      i32.load offset=12
      local.set 6
      i32.const 0
      local.set 7
      loop  ;; label = @2
        local.get 2
        i32.const 1
        i32.add
        local.set 8
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  local.get 5
                  i32.extend8_s
                  i32.const -1
                  i32.gt_s
                  br_if 0 (;@7;)
                  local.get 5
                  i32.const 255
                  i32.and
                  local.tee 9
                  i32.const 128
                  i32.eq
                  br_if 1 (;@6;)
                  local.get 9
                  i32.const 192
                  i32.ne
                  br_if 3 (;@4;)
                  local.get 4
                  local.get 1
                  i32.store offset=4
                  local.get 4
                  local.get 0
                  i32.store
                  local.get 4
                  i64.const 1610612768
                  i64.store offset=8 align=4
                  local.get 3
                  local.get 7
                  i32.const 3
                  i32.shl
                  i32.add
                  local.tee 5
                  i32.load
                  local.get 4
                  local.get 5
                  i32.load offset=4
                  call_indirect (type 3)
                  i32.eqz
                  br_if 2 (;@5;)
                  i32.const 1
                  local.set 5
                  br 6 (;@1;)
                end
                block  ;; label = @7
                  local.get 0
                  local.get 8
                  local.get 5
                  i32.const 255
                  i32.and
                  local.tee 5
                  local.get 6
                  call_indirect (type 2)
                  br_if 0 (;@7;)
                  local.get 8
                  local.get 5
                  i32.add
                  local.set 2
                  br 4 (;@3;)
                end
                i32.const 1
                local.set 5
                br 5 (;@1;)
              end
              block  ;; label = @6
                local.get 0
                local.get 2
                i32.const 3
                i32.add
                local.tee 5
                local.get 2
                i32.load16_u offset=1 align=1
                local.tee 2
                local.get 6
                call_indirect (type 2)
                br_if 0 (;@6;)
                local.get 5
                local.get 2
                i32.add
                local.set 2
                br 3 (;@3;)
              end
              i32.const 1
              local.set 5
              br 4 (;@1;)
            end
            local.get 7
            i32.const 1
            i32.add
            local.set 7
            local.get 8
            local.set 2
            br 1 (;@3;)
          end
          i32.const 1610612768
          local.set 10
          block  ;; label = @4
            local.get 5
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 2
            i32.const 5
            i32.add
            local.set 8
            local.get 2
            i32.load offset=1 align=1
            local.set 10
          end
          i32.const 0
          local.set 9
          block  ;; label = @4
            block  ;; label = @5
              local.get 5
              i32.const 2
              i32.and
              br_if 0 (;@5;)
              i32.const 0
              local.set 11
              local.get 8
              local.set 2
              br 1 (;@4;)
            end
            local.get 8
            i32.const 2
            i32.add
            local.set 2
            local.get 8
            i32.load16_u align=1
            local.set 11
          end
          block  ;; label = @4
            block  ;; label = @5
              local.get 5
              i32.const 4
              i32.and
              br_if 0 (;@5;)
              local.get 2
              local.set 8
              br 1 (;@4;)
            end
            local.get 2
            i32.const 2
            i32.add
            local.set 8
            local.get 2
            i32.load16_u align=1
            local.set 9
          end
          block  ;; label = @4
            block  ;; label = @5
              local.get 5
              i32.const 8
              i32.and
              br_if 0 (;@5;)
              local.get 8
              local.set 2
              br 1 (;@4;)
            end
            local.get 8
            i32.const 2
            i32.add
            local.set 2
            local.get 8
            i32.load16_u align=1
            local.set 7
          end
          block  ;; label = @4
            local.get 5
            i32.const 16
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 3
            local.get 11
            i32.const 65535
            i32.and
            i32.const 3
            i32.shl
            i32.add
            i32.load16_u offset=4
            local.set 11
          end
          block  ;; label = @4
            local.get 5
            i32.const 32
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 3
            local.get 9
            i32.const 65535
            i32.and
            i32.const 3
            i32.shl
            i32.add
            i32.load16_u offset=4
            local.set 9
          end
          local.get 4
          local.get 9
          i32.store16 offset=14
          local.get 4
          local.get 11
          i32.store16 offset=12
          local.get 4
          local.get 10
          i32.store offset=8
          local.get 4
          local.get 1
          i32.store offset=4
          local.get 4
          local.get 0
          i32.store
          block  ;; label = @4
            local.get 3
            local.get 7
            i32.const 3
            i32.shl
            i32.add
            local.tee 5
            i32.load
            local.get 4
            local.get 5
            i32.load offset=4
            call_indirect (type 3)
            i32.eqz
            br_if 0 (;@4;)
            i32.const 1
            local.set 5
            br 3 (;@1;)
          end
          local.get 7
          i32.const 1
          i32.add
          local.set 7
        end
        local.get 2
        i32.load8_u
        local.tee 5
        br_if 0 (;@2;)
      end
      i32.const 0
      local.set 5
    end
    local.get 4
    i32.const 16
    i32.add
    global.set $__stack_pointer
    local.get 5)
  (func $_RNvNtCsgXGp5Oqx2Ny_4core9panicking18panic_bounds_check (type 9) (param i32 i32 i32)
    (local i32 i64)
    global.get $__stack_pointer
    i32.const 32
    i32.sub
    local.tee 3
    global.set $__stack_pointer
    local.get 3
    local.get 1
    i32.store offset=12
    local.get 3
    local.get 0
    i32.store offset=8
    local.get 3
    i32.const 18
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.tee 4
    local.get 3
    i32.const 8
    i32.add
    i64.extend_i32_u
    i64.or
    i64.store offset=24
    local.get 3
    local.get 4
    local.get 3
    i32.const 12
    i32.add
    i64.extend_i32_u
    i64.or
    i64.store offset=16
    i32.const 1048576
    local.get 3
    i32.const 16
    i32.add
    local.get 2
    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking9panic_fmt
    unreachable)
  (func $_RNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB5_9Formatter12pad_integral (type 7) (param i32 i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i64)
    i32.const 43
    i32.const 1114112
    local.get 0
    i32.load offset=8
    local.tee 6
    i32.const 2097152
    i32.and
    local.tee 7
    select
    local.set 8
    local.get 7
    i32.const 21
    i32.shr_u
    i32.const 1
    local.get 1
    select
    local.get 5
    i32.add
    local.set 9
    block  ;; label = @1
      block  ;; label = @2
        local.get 6
        i32.const 8388608
        i32.and
        br_if 0 (;@2;)
        i32.const 0
        local.set 2
        br 1 (;@1;)
      end
      block  ;; label = @2
        block  ;; label = @3
          local.get 3
          i32.const 16
          i32.lt_u
          br_if 0 (;@3;)
          local.get 2
          local.get 3
          call $_RNvNtNtCsgXGp5Oqx2Ny_4core3str5count14do_count_chars
          local.set 7
          br 1 (;@2;)
        end
        block  ;; label = @3
          local.get 3
          br_if 0 (;@3;)
          i32.const 0
          local.set 7
          br 1 (;@2;)
        end
        local.get 3
        i32.const 3
        i32.and
        local.set 10
        i32.const 0
        local.set 11
        i32.const 0
        local.set 7
        block  ;; label = @3
          local.get 3
          i32.const 4
          i32.lt_u
          br_if 0 (;@3;)
          local.get 3
          i32.const 12
          i32.and
          local.set 12
          i32.const 0
          local.set 11
          i32.const 0
          local.set 7
          loop  ;; label = @4
            local.get 7
            local.get 2
            local.get 11
            i32.add
            local.tee 13
            i32.load8_s
            i32.const -65
            i32.gt_s
            i32.add
            local.get 13
            i32.const 1
            i32.add
            i32.load8_s
            i32.const -65
            i32.gt_s
            i32.add
            local.get 13
            i32.const 2
            i32.add
            i32.load8_s
            i32.const -65
            i32.gt_s
            i32.add
            local.get 13
            i32.const 3
            i32.add
            i32.load8_s
            i32.const -65
            i32.gt_s
            i32.add
            local.set 7
            local.get 12
            local.get 11
            i32.const 4
            i32.add
            local.tee 11
            i32.ne
            br_if 0 (;@4;)
          end
          local.get 10
          i32.eqz
          br_if 1 (;@2;)
        end
        local.get 2
        local.get 11
        i32.add
        local.set 13
        loop  ;; label = @3
          local.get 7
          local.get 13
          i32.load8_s
          i32.const -65
          i32.gt_s
          i32.add
          local.set 7
          local.get 13
          i32.const 1
          i32.add
          local.set 13
          local.get 10
          i32.const -1
          i32.add
          local.tee 10
          br_if 0 (;@3;)
        end
      end
      local.get 7
      local.get 9
      i32.add
      local.set 9
    end
    local.get 8
    i32.const 45
    local.get 1
    select
    local.set 12
    block  ;; label = @1
      block  ;; label = @2
        local.get 9
        local.get 0
        i32.load16_u offset=12
        local.tee 1
        i32.ge_u
        br_if 0 (;@2;)
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 6
              i32.const 16777216
              i32.and
              br_if 0 (;@5;)
              local.get 1
              local.get 9
              i32.sub
              local.set 8
              i32.const 0
              local.set 7
              i32.const 0
              local.set 1
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 6
                    i32.const 29
                    i32.shr_u
                    i32.const 3
                    i32.and
                    br_table 2 (;@6;) 0 (;@8;) 1 (;@7;) 0 (;@8;) 2 (;@6;)
                  end
                  local.get 8
                  local.set 1
                  br 1 (;@6;)
                end
                local.get 8
                i32.const 65534
                i32.and
                i32.const 1
                i32.shr_u
                local.set 1
              end
              local.get 6
              i32.const 2097151
              i32.and
              local.set 9
              local.get 0
              i32.load offset=4
              local.set 11
              local.get 0
              i32.load
              local.set 10
              loop  ;; label = @6
                local.get 7
                i32.const 65535
                i32.and
                local.get 1
                i32.const 65535
                i32.and
                i32.ge_u
                br_if 2 (;@4;)
                i32.const 1
                local.set 13
                local.get 7
                i32.const 1
                i32.add
                local.set 7
                local.get 10
                local.get 9
                local.get 11
                i32.load offset=16
                call_indirect (type 3)
                i32.eqz
                br_if 0 (;@6;)
                br 5 (;@1;)
              end
            end
            local.get 0
            local.get 0
            i64.load offset=8 align=4
            local.tee 14
            i32.wrap_i64
            i32.const -1612709888
            i32.and
            i32.const 536870960
            i32.or
            i32.store offset=8
            i32.const 1
            local.set 13
            local.get 0
            i32.load
            local.tee 10
            local.get 0
            i32.load offset=4
            local.tee 11
            local.get 12
            local.get 2
            local.get 3
            call $_RNvNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB7_9Formatter12pad_integral12write_prefix
            br_if 3 (;@1;)
            i32.const 0
            local.set 7
            local.get 1
            local.get 9
            i32.sub
            i32.const 65535
            i32.and
            local.set 2
            loop  ;; label = @5
              local.get 7
              i32.const 65535
              i32.and
              local.get 2
              i32.ge_u
              br_if 2 (;@3;)
              i32.const 1
              local.set 13
              local.get 7
              i32.const 1
              i32.add
              local.set 7
              local.get 10
              i32.const 48
              local.get 11
              i32.load offset=16
              call_indirect (type 3)
              i32.eqz
              br_if 0 (;@5;)
              br 4 (;@1;)
            end
          end
          i32.const 1
          local.set 13
          local.get 10
          local.get 11
          local.get 12
          local.get 2
          local.get 3
          call $_RNvNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB7_9Formatter12pad_integral12write_prefix
          br_if 2 (;@1;)
          local.get 10
          local.get 4
          local.get 5
          local.get 11
          i32.load offset=12
          call_indirect (type 2)
          br_if 2 (;@1;)
          i32.const 0
          local.set 7
          local.get 8
          local.get 1
          i32.sub
          i32.const 65535
          i32.and
          local.set 0
          loop  ;; label = @4
            local.get 7
            i32.const 65535
            i32.and
            local.tee 2
            local.get 0
            i32.lt_u
            local.set 13
            local.get 2
            local.get 0
            i32.ge_u
            br_if 3 (;@1;)
            local.get 7
            i32.const 1
            i32.add
            local.set 7
            local.get 10
            local.get 9
            local.get 11
            i32.load offset=16
            call_indirect (type 3)
            i32.eqz
            br_if 0 (;@4;)
            br 3 (;@1;)
          end
        end
        i32.const 1
        local.set 13
        local.get 10
        local.get 4
        local.get 5
        local.get 11
        i32.load offset=12
        call_indirect (type 2)
        br_if 1 (;@1;)
        local.get 0
        local.get 14
        i64.store offset=8 align=4
        i32.const 0
        return
      end
      i32.const 1
      local.set 13
      local.get 0
      i32.load
      local.tee 7
      local.get 0
      i32.load offset=4
      local.tee 10
      local.get 12
      local.get 2
      local.get 3
      call $_RNvNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB7_9Formatter12pad_integral12write_prefix
      br_if 0 (;@1;)
      local.get 7
      local.get 4
      local.get 5
      local.get 10
      i32.load offset=12
      call_indirect (type 2)
      local.set 13
    end
    local.get 13)
  (func $_RNvNtNtCsgXGp5Oqx2Ny_4core3str5count14do_count_chars (type 3) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        local.get 0
        i32.const 3
        i32.add
        i32.const -4
        i32.and
        local.tee 2
        local.get 0
        i32.sub
        local.tee 3
        i32.lt_u
        br_if 0 (;@2;)
        local.get 1
        local.get 3
        i32.sub
        local.tee 4
        i32.const 2
        i32.shr_u
        local.tee 5
        i32.eqz
        br_if 0 (;@2;)
        local.get 4
        i32.const 3
        i32.and
        local.set 6
        i32.const 0
        local.set 7
        i32.const 0
        local.set 1
        block  ;; label = @3
          local.get 2
          local.get 0
          i32.eq
          br_if 0 (;@3;)
          i32.const 0
          local.set 8
          i32.const 0
          local.set 1
          block  ;; label = @4
            local.get 0
            local.get 2
            i32.sub
            local.tee 9
            i32.const -4
            i32.gt_u
            br_if 0 (;@4;)
            i32.const 0
            local.set 8
            i32.const 0
            local.set 1
            loop  ;; label = @5
              local.get 1
              local.get 0
              local.get 8
              i32.add
              local.tee 2
              i32.load8_s
              i32.const -65
              i32.gt_s
              i32.add
              local.get 2
              i32.const 1
              i32.add
              i32.load8_s
              i32.const -65
              i32.gt_s
              i32.add
              local.get 2
              i32.const 2
              i32.add
              i32.load8_s
              i32.const -65
              i32.gt_s
              i32.add
              local.get 2
              i32.const 3
              i32.add
              i32.load8_s
              i32.const -65
              i32.gt_s
              i32.add
              local.set 1
              local.get 8
              i32.const 4
              i32.add
              local.tee 8
              br_if 0 (;@5;)
            end
          end
          local.get 0
          local.get 8
          i32.add
          local.set 2
          loop  ;; label = @4
            local.get 1
            local.get 2
            i32.load8_s
            i32.const -65
            i32.gt_s
            i32.add
            local.set 1
            local.get 2
            i32.const 1
            i32.add
            local.set 2
            local.get 9
            i32.const 1
            i32.add
            local.tee 9
            br_if 0 (;@4;)
          end
        end
        local.get 0
        local.get 3
        i32.add
        local.set 9
        block  ;; label = @3
          local.get 6
          i32.eqz
          br_if 0 (;@3;)
          local.get 9
          local.get 4
          i32.const 2147483644
          i32.and
          i32.add
          local.tee 2
          i32.load8_s
          i32.const -65
          i32.gt_s
          local.set 7
          local.get 6
          i32.const 1
          i32.eq
          br_if 0 (;@3;)
          local.get 7
          local.get 2
          i32.load8_s offset=1
          i32.const -65
          i32.gt_s
          i32.add
          local.set 7
          local.get 6
          i32.const 2
          i32.eq
          br_if 0 (;@3;)
          local.get 7
          local.get 2
          i32.load8_s offset=2
          i32.const -65
          i32.gt_s
          i32.add
          local.set 7
        end
        local.get 7
        local.get 1
        i32.add
        local.set 8
        loop  ;; label = @3
          local.get 9
          local.set 3
          local.get 5
          i32.eqz
          br_if 2 (;@1;)
          local.get 5
          i32.const 192
          local.get 5
          i32.const 192
          i32.lt_u
          select
          local.tee 7
          i32.const 3
          i32.and
          local.set 6
          block  ;; label = @4
            block  ;; label = @5
              local.get 7
              i32.const 2
              i32.shl
              local.tee 4
              i32.const 1008
              i32.and
              local.tee 1
              br_if 0 (;@5;)
              i32.const 0
              local.set 2
              br 1 (;@4;)
            end
            local.get 3
            local.get 1
            i32.add
            local.set 0
            i32.const 0
            local.set 2
            local.get 3
            local.set 1
            loop  ;; label = @5
              local.get 1
              i32.const 12
              i32.add
              i32.load
              local.tee 9
              i32.const -1
              i32.xor
              i32.const 7
              i32.shr_u
              local.get 9
              i32.const 6
              i32.shr_u
              i32.or
              i32.const 16843009
              i32.and
              local.get 1
              i32.const 8
              i32.add
              i32.load
              local.tee 9
              i32.const -1
              i32.xor
              i32.const 7
              i32.shr_u
              local.get 9
              i32.const 6
              i32.shr_u
              i32.or
              i32.const 16843009
              i32.and
              local.get 1
              i32.const 4
              i32.add
              i32.load
              local.tee 9
              i32.const -1
              i32.xor
              i32.const 7
              i32.shr_u
              local.get 9
              i32.const 6
              i32.shr_u
              i32.or
              i32.const 16843009
              i32.and
              local.get 1
              i32.load
              local.tee 9
              i32.const -1
              i32.xor
              i32.const 7
              i32.shr_u
              local.get 9
              i32.const 6
              i32.shr_u
              i32.or
              i32.const 16843009
              i32.and
              local.get 2
              i32.add
              i32.add
              i32.add
              i32.add
              local.set 2
              local.get 1
              i32.const 16
              i32.add
              local.tee 1
              local.get 0
              i32.ne
              br_if 0 (;@5;)
            end
          end
          local.get 5
          local.get 7
          i32.sub
          local.set 5
          local.get 3
          local.get 4
          i32.add
          local.set 9
          local.get 2
          i32.const 8
          i32.shr_u
          i32.const 16711935
          i32.and
          local.get 2
          i32.const 16711935
          i32.and
          i32.add
          i32.const 65537
          i32.mul
          i32.const 16
          i32.shr_u
          local.get 8
          i32.add
          local.set 8
          local.get 6
          i32.eqz
          br_if 0 (;@3;)
        end
        local.get 3
        local.get 7
        i32.const 252
        i32.and
        i32.const 2
        i32.shl
        i32.add
        local.tee 2
        i32.load
        local.tee 1
        i32.const -1
        i32.xor
        i32.const 7
        i32.shr_u
        local.get 1
        i32.const 6
        i32.shr_u
        i32.or
        i32.const 16843009
        i32.and
        local.set 1
        block  ;; label = @3
          local.get 6
          i32.const 1
          i32.eq
          br_if 0 (;@3;)
          local.get 2
          i32.load offset=4
          local.tee 9
          i32.const -1
          i32.xor
          i32.const 7
          i32.shr_u
          local.get 9
          i32.const 6
          i32.shr_u
          i32.or
          i32.const 16843009
          i32.and
          local.get 1
          i32.add
          local.set 1
          local.get 6
          i32.const 2
          i32.eq
          br_if 0 (;@3;)
          local.get 2
          i32.load offset=8
          local.tee 2
          i32.const -1
          i32.xor
          i32.const 7
          i32.shr_u
          local.get 2
          i32.const 6
          i32.shr_u
          i32.or
          i32.const 16843009
          i32.and
          local.get 1
          i32.add
          local.set 1
        end
        local.get 1
        i32.const 8
        i32.shr_u
        i32.const 459007
        i32.and
        local.get 1
        i32.const 16711935
        i32.and
        i32.add
        i32.const 65537
        i32.mul
        i32.const 16
        i32.shr_u
        local.get 8
        i32.add
        local.set 8
        br 1 (;@1;)
      end
      block  ;; label = @2
        local.get 1
        br_if 0 (;@2;)
        i32.const 0
        return
      end
      local.get 1
      i32.const 3
      i32.and
      local.set 2
      i32.const 0
      local.set 9
      i32.const 0
      local.set 8
      block  ;; label = @2
        local.get 1
        i32.const 4
        i32.lt_u
        br_if 0 (;@2;)
        local.get 1
        i32.const -4
        i32.and
        local.set 5
        i32.const 0
        local.set 8
        i32.const 0
        local.set 9
        loop  ;; label = @3
          local.get 8
          local.get 0
          local.get 9
          i32.add
          local.tee 1
          i32.load8_s
          i32.const -65
          i32.gt_s
          i32.add
          local.get 1
          i32.const 1
          i32.add
          i32.load8_s
          i32.const -65
          i32.gt_s
          i32.add
          local.get 1
          i32.const 2
          i32.add
          i32.load8_s
          i32.const -65
          i32.gt_s
          i32.add
          local.get 1
          i32.const 3
          i32.add
          i32.load8_s
          i32.const -65
          i32.gt_s
          i32.add
          local.set 8
          local.get 5
          local.get 9
          i32.const 4
          i32.add
          local.tee 9
          i32.ne
          br_if 0 (;@3;)
        end
        local.get 2
        i32.eqz
        br_if 1 (;@1;)
      end
      local.get 0
      local.get 9
      i32.add
      local.set 1
      loop  ;; label = @2
        local.get 8
        local.get 1
        i32.load8_s
        i32.const -65
        i32.gt_s
        i32.add
        local.set 8
        local.get 1
        i32.const 1
        i32.add
        local.set 1
        local.get 2
        i32.const -1
        i32.add
        local.tee 2
        br_if 0 (;@2;)
      end
    end
    local.get 8)
  (func $_RNvNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB7_9Formatter12pad_integral12write_prefix (type 6) (param i32 i32 i32 i32 i32) (result i32)
    block  ;; label = @1
      local.get 2
      i32.const 1114112
      i32.eq
      br_if 0 (;@1;)
      local.get 0
      local.get 2
      local.get 1
      i32.load offset=16
      call_indirect (type 3)
      i32.eqz
      br_if 0 (;@1;)
      i32.const 1
      return
    end
    block  ;; label = @1
      local.get 3
      br_if 0 (;@1;)
      i32.const 0
      return
    end
    local.get 0
    local.get 3
    local.get 4
    local.get 1
    i32.load offset=12
    call_indirect (type 2))
  (func $_RNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB5_9Formatter3pad (type 2) (param i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 0
        i32.load offset=8
        local.tee 3
        i32.const 402653184
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  local.get 3
                  i32.const 268435456
                  i32.and
                  i32.eqz
                  br_if 0 (;@7;)
                  local.get 0
                  i32.load16_u offset=14
                  local.tee 4
                  br_if 1 (;@6;)
                  i32.const 0
                  local.set 2
                  br 2 (;@5;)
                end
                block  ;; label = @7
                  local.get 2
                  i32.const 16
                  i32.lt_u
                  br_if 0 (;@7;)
                  local.get 1
                  local.get 2
                  call $_RNvNtNtCsgXGp5Oqx2Ny_4core3str5count14do_count_chars
                  local.set 5
                  br 4 (;@3;)
                end
                block  ;; label = @7
                  local.get 2
                  br_if 0 (;@7;)
                  i32.const 0
                  local.set 5
                  br 4 (;@3;)
                end
                local.get 2
                i32.const 3
                i32.and
                local.set 6
                i32.const 0
                local.set 7
                i32.const 0
                local.set 5
                block  ;; label = @7
                  local.get 2
                  i32.const 4
                  i32.lt_u
                  br_if 0 (;@7;)
                  local.get 2
                  i32.const 12
                  i32.and
                  local.set 4
                  i32.const 0
                  local.set 5
                  i32.const 0
                  local.set 7
                  loop  ;; label = @8
                    local.get 5
                    local.get 1
                    local.get 7
                    i32.add
                    local.tee 8
                    i32.load8_s
                    i32.const -65
                    i32.gt_s
                    i32.add
                    local.get 8
                    i32.const 1
                    i32.add
                    i32.load8_s
                    i32.const -65
                    i32.gt_s
                    i32.add
                    local.get 8
                    i32.const 2
                    i32.add
                    i32.load8_s
                    i32.const -65
                    i32.gt_s
                    i32.add
                    local.get 8
                    i32.const 3
                    i32.add
                    i32.load8_s
                    i32.const -65
                    i32.gt_s
                    i32.add
                    local.set 5
                    local.get 4
                    local.get 7
                    i32.const 4
                    i32.add
                    local.tee 7
                    i32.ne
                    br_if 0 (;@8;)
                  end
                  local.get 6
                  i32.eqz
                  br_if 4 (;@3;)
                end
                local.get 1
                local.get 7
                i32.add
                local.set 8
                loop  ;; label = @7
                  local.get 5
                  local.get 8
                  i32.load8_s
                  i32.const -65
                  i32.gt_s
                  i32.add
                  local.set 5
                  local.get 8
                  i32.const 1
                  i32.add
                  local.set 8
                  local.get 6
                  i32.const -1
                  i32.add
                  local.tee 6
                  br_if 0 (;@7;)
                  br 4 (;@3;)
                end
              end
              local.get 1
              local.get 2
              i32.add
              local.set 7
              i32.const 0
              local.set 2
              local.get 1
              local.set 8
              local.get 4
              local.set 6
              loop  ;; label = @6
                local.get 8
                local.tee 5
                local.get 7
                i32.eq
                br_if 2 (;@4;)
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 5
                    i32.load8_s
                    local.tee 8
                    i32.const -1
                    i32.le_s
                    br_if 0 (;@8;)
                    local.get 5
                    i32.const 1
                    i32.add
                    local.set 8
                    br 1 (;@7;)
                  end
                  block  ;; label = @8
                    local.get 8
                    i32.const -32
                    i32.ge_u
                    br_if 0 (;@8;)
                    local.get 5
                    i32.const 2
                    i32.add
                    local.set 8
                    br 1 (;@7;)
                  end
                  local.get 5
                  i32.const 4
                  i32.const 3
                  local.get 8
                  i32.const -17
                  i32.gt_u
                  select
                  i32.add
                  local.set 8
                end
                local.get 8
                local.get 5
                i32.sub
                local.get 2
                i32.add
                local.set 2
                local.get 6
                i32.const -1
                i32.add
                local.tee 6
                br_if 0 (;@6;)
              end
            end
            i32.const 0
            local.set 6
          end
          local.get 4
          local.get 6
          i32.sub
          local.set 5
        end
        local.get 5
        local.get 0
        i32.load16_u offset=12
        local.tee 8
        i32.ge_u
        br_if 0 (;@2;)
        local.get 8
        local.get 5
        i32.sub
        local.set 9
        i32.const 0
        local.set 5
        i32.const 0
        local.set 4
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 3
              i32.const 29
              i32.shr_u
              i32.const 3
              i32.and
              br_table 2 (;@3;) 0 (;@5;) 1 (;@4;) 2 (;@3;) 2 (;@3;)
            end
            local.get 9
            local.set 4
            br 1 (;@3;)
          end
          local.get 9
          i32.const 65534
          i32.and
          i32.const 1
          i32.shr_u
          local.set 4
        end
        local.get 3
        i32.const 2097151
        i32.and
        local.set 7
        local.get 0
        i32.load offset=4
        local.set 6
        local.get 0
        i32.load
        local.set 0
        block  ;; label = @3
          loop  ;; label = @4
            local.get 5
            i32.const 65535
            i32.and
            local.get 4
            i32.const 65535
            i32.and
            i32.ge_u
            br_if 1 (;@3;)
            i32.const 1
            local.set 8
            local.get 5
            i32.const 1
            i32.add
            local.set 5
            local.get 0
            local.get 7
            local.get 6
            i32.load offset=16
            call_indirect (type 3)
            br_if 3 (;@1;)
            br 0 (;@4;)
          end
        end
        i32.const 1
        local.set 8
        local.get 0
        local.get 1
        local.get 2
        local.get 6
        i32.load offset=12
        call_indirect (type 2)
        br_if 1 (;@1;)
        i32.const 0
        local.set 5
        local.get 9
        local.get 4
        i32.sub
        i32.const 65535
        i32.and
        local.set 2
        loop  ;; label = @3
          local.get 5
          i32.const 65535
          i32.and
          local.tee 4
          local.get 2
          i32.lt_u
          local.set 8
          local.get 4
          local.get 2
          i32.ge_u
          br_if 2 (;@1;)
          local.get 5
          i32.const 1
          i32.add
          local.set 5
          local.get 0
          local.get 7
          local.get 6
          i32.load offset=16
          call_indirect (type 3)
          br_if 2 (;@1;)
          br 0 (;@3;)
        end
      end
      local.get 0
      i32.load
      local.get 1
      local.get 2
      local.get 0
      i32.load offset=4
      i32.load offset=12
      call_indirect (type 2)
      local.set 8
    end
    local.get 8)
  (func $_RNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB5_9Formatter9write_str (type 2) (param i32 i32 i32) (result i32)
    local.get 0
    i32.load
    local.get 1
    local.get 2
    local.get 0
    i32.load offset=4
    i32.load offset=12
    call_indirect (type 2))
  (func $_RNvXs8_NtNtNtCsgXGp5Oqx2Ny_4core3fmt3num3impmNtB9_7Display3fmt (type 3) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 2
    global.set $__stack_pointer
    i32.const 10
    local.set 3
    local.get 0
    i32.load
    local.tee 4
    local.set 5
    block  ;; label = @1
      local.get 4
      i32.const 1000
      i32.lt_u
      br_if 0 (;@1;)
      i32.const 10
      local.set 3
      local.get 4
      local.set 5
      loop  ;; label = @2
        local.get 2
        i32.const 6
        i32.add
        local.get 3
        i32.add
        local.tee 6
        i32.const -4
        i32.add
        local.get 5
        local.tee 0
        local.get 0
        i32.const 10000
        i32.div_u
        local.tee 5
        i32.const 10000
        i32.mul
        i32.sub
        local.tee 7
        i32.const 65535
        i32.and
        i32.const 100
        i32.div_u
        local.tee 8
        i32.const 1
        i32.shl
        i32.load16_u offset=1050568 align=1
        i32.store16 align=1
        local.get 6
        i32.const -2
        i32.add
        local.get 7
        local.get 8
        i32.const 100
        i32.mul
        i32.sub
        i32.const 65535
        i32.and
        i32.const 1
        i32.shl
        i32.load16_u offset=1050568 align=1
        i32.store16 align=1
        local.get 3
        i32.const -4
        i32.add
        local.set 3
        local.get 0
        i32.const 9999999
        i32.gt_u
        br_if 0 (;@2;)
      end
    end
    block  ;; label = @1
      block  ;; label = @2
        local.get 5
        i32.const 9
        i32.gt_u
        br_if 0 (;@2;)
        local.get 5
        local.set 0
        br 1 (;@1;)
      end
      local.get 2
      i32.const 6
      i32.add
      local.get 3
      i32.const -2
      i32.add
      local.tee 3
      i32.add
      local.get 5
      local.get 5
      i32.const 65535
      i32.and
      i32.const 100
      i32.div_u
      local.tee 0
      i32.const 100
      i32.mul
      i32.sub
      i32.const 65535
      i32.and
      i32.const 1
      i32.shl
      i32.load16_u offset=1050568 align=1
      i32.store16 align=1
    end
    block  ;; label = @1
      block  ;; label = @2
        local.get 4
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        i32.eqz
        br_if 1 (;@1;)
      end
      local.get 2
      i32.const 6
      i32.add
      local.get 3
      i32.const -1
      i32.add
      local.tee 3
      i32.add
      local.get 0
      i32.const 1
      i32.shl
      i32.load8_u offset=1050569
      i32.store8
    end
    local.get 1
    i32.const 1
    i32.const 1
    i32.const 0
    local.get 2
    i32.const 6
    i32.add
    local.get 3
    i32.add
    i32.const 10
    local.get 3
    i32.sub
    call $_RNvMsa_NtCsgXGp5Oqx2Ny_4core3fmtNtB5_9Formatter12pad_integral
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $__stack_pointer
    local.get 3)
  (func $_RNvNtCsgXGp5Oqx2Ny_4core6option13expect_failed (type 9) (param i32 i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 3
    global.set $__stack_pointer
    local.get 3
    local.get 1
    i32.store offset=4
    local.get 3
    local.get 0
    i32.store
    local.get 3
    i32.const 19
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get 3
    i64.extend_i32_u
    i64.or
    i64.store offset=8
    i32.const 1048635
    local.get 3
    i32.const 8
    i32.add
    local.get 2
    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking9panic_fmt
    unreachable)
  (func $_RNvNtCsgXGp5Oqx2Ny_4core6result13unwrap_failed (type 11) (param i32 i32 i32 i32 i32)
    (local i32)
    global.get $__stack_pointer
    i32.const 32
    i32.sub
    local.tee 5
    global.set $__stack_pointer
    local.get 5
    local.get 1
    i32.store offset=4
    local.get 5
    local.get 0
    i32.store
    local.get 5
    local.get 3
    i32.store offset=12
    local.get 5
    local.get 2
    i32.store offset=8
    local.get 5
    i32.const 20
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get 5
    i32.const 8
    i32.add
    i64.extend_i32_u
    i64.or
    i64.store offset=24
    local.get 5
    i32.const 19
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get 5
    i64.extend_i32_u
    i64.or
    i64.store offset=16
    i32.const 1048631
    local.get 5
    i32.const 16
    i32.add
    local.get 4
    call $_RNvNtCsgXGp5Oqx2Ny_4core9panicking9panic_fmt
    unreachable)
  (func $_RNvXs1g_NtCsgXGp5Oqx2Ny_4core3fmtRDNtB6_5DebugEL_Bx_3fmtB8_ (type 3) (param i32 i32) (result i32)
    local.get 0
    i32.load
    local.get 1
    local.get 0
    i32.load offset=4
    i32.load offset=12
    call_indirect (type 3))
  (func $__multi3 (type 21) (param i32 i64 i64 i64 i64)
    (local i64 i64 i64 i64 i64 i64)
    local.get 0
    local.get 3
    i64.const 4294967295
    i64.and
    local.tee 5
    local.get 1
    i64.const 4294967295
    i64.and
    local.tee 6
    i64.mul
    local.tee 7
    local.get 3
    i64.const 32
    i64.shr_u
    local.tee 8
    local.get 6
    i64.mul
    local.tee 6
    local.get 5
    local.get 1
    i64.const 32
    i64.shr_u
    local.tee 9
    i64.mul
    i64.add
    local.tee 5
    i64.const 32
    i64.shl
    i64.add
    local.tee 10
    i64.store
    local.get 0
    local.get 8
    local.get 9
    i64.mul
    local.get 5
    local.get 6
    i64.lt_u
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get 5
    i64.const 32
    i64.shr_u
    i64.or
    i64.add
    local.get 10
    local.get 7
    i64.lt_u
    i64.extend_i32_u
    i64.add
    local.get 4
    local.get 1
    i64.mul
    local.get 3
    local.get 2
    i64.mul
    i64.add
    i64.add
    i64.store offset=8)
  (table (;0;) 21 21 funcref)
  (memory (;0;) 17)
  (global $__stack_pointer (mut i32) (i32.const 1048576))
  (global (;1;) i32 (i32.const 1056633))
  (global (;2;) i32 (i32.const 1056640))
  (export "memory" (memory 0))
  (export "proto_abi_version" (func $proto_abi_version))
  (export "proto_standard_id" (func $proto_standard_id))
  (export "tor_hs_pow_v1_blake2b_result" (func $tor_hs_pow_v1_blake2b_result))
  (export "tor_hs_pow_v1_challenge" (func $tor_hs_pow_v1_challenge))
  (export "tor_hs_pow_v1_challenge_len" (func $tor_hs_pow_v1_challenge_len))
  (export "tor_hs_pow_v1_equix_verify" (func $tor_hs_pow_v1_equix_verify))
  (export "tor_hs_pow_v1_replay_capacity" (func $tor_hs_pow_v1_replay_capacity))
  (export "tor_hs_pow_v1_replay_count" (func $tor_hs_pow_v1_replay_count))
  (export "tor_hs_pow_v1_replay_reset" (func $tor_hs_pow_v1_replay_reset))
  (export "tor_hs_pow_v1_solution_len" (func $tor_hs_pow_v1_solution_len))
  (export "tor_hs_pow_v1_solve_first" (func $tor_hs_pow_v1_solve_first))
  (export "tor_hs_pow_v1_verify" (func $tor_hs_pow_v1_verify))
  (export "__data_end" (global 1))
  (export "__heap_base" (global 2))
  (elem (;0;) (i32.const 1) func $_RNvXss_NtCsgXGp5Oqx2Ny_4core3fmtuNtB5_5Debug3fmt $_RNvNtCsebHcaeoSrxy_3std5alloc24default_alloc_error_hook $_RINvNtCsgXGp5Oqx2Ny_4core3ptr13drop_in_placeNtNtCs5cOc02OMXlo_5alloc6string6StringECsebHcaeoSrxy_3std $_RNvXsZ_NtCs5cOc02OMXlo_5alloc6stringNtB5_6StringNtNtCsgXGp5Oqx2Ny_4core3fmt5Write9write_str $_RNvXsZ_NtCs5cOc02OMXlo_5alloc6stringNtB5_6StringNtNtCsgXGp5Oqx2Ny_4core3fmt5Write10write_char $_RNvYNtNtCs5cOc02OMXlo_5alloc6string6StringNtNtCsgXGp5Oqx2Ny_4core3fmt5Write9write_fmtCsebHcaeoSrxy_3std $_RNvXs2_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB5_16StaticStrPayloadNtNtCsgXGp5Oqx2Ny_4core3fmt7Display3fmt $_RNvXs1_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB5_16StaticStrPayloadNtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload8take_box $_RNvXs1_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB5_16StaticStrPayloadNtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload3get $_RNvXs1_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB5_16StaticStrPayloadNtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload6as_str $_RINvNtCsgXGp5Oqx2Ny_4core3ptr13drop_in_placeNtNvNtCsebHcaeoSrxy_3std9panicking13panic_handler19FormatStringPayloadEBM_ $_RNvXs0_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB5_19FormatStringPayloadNtNtCsgXGp5Oqx2Ny_4core3fmt7Display3fmt $_RNvXs_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB4_19FormatStringPayloadNtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload8take_box $_RNvXs_NvNtCsebHcaeoSrxy_3std9panicking13panic_handlerNtB4_19FormatStringPayloadNtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload3get $_RNvYINtNvNtCsebHcaeoSrxy_3std9panicking11begin_panic7PayloadReENtNtCsgXGp5Oqx2Ny_4core5panic12PanicPayload6as_strB9_ $_RNvXNtCsgXGp5Oqx2Ny_4core3anyReNtB2_3Any7type_idCsebHcaeoSrxy_3std $_RNvXNtCsgXGp5Oqx2Ny_4core3anyNtNtCs5cOc02OMXlo_5alloc6string6StringNtB2_3Any7type_idCsebHcaeoSrxy_3std $_RNvXs8_NtNtNtCsgXGp5Oqx2Ny_4core3fmt3num3impmNtB9_7Display3fmt $_RNvXs1i_NtCsgXGp5Oqx2Ny_4core3fmtReNtB6_7Display3fmtB8_ $_RNvXs1g_NtCsgXGp5Oqx2Ny_4core3fmtRDNtB6_5DebugEL_Bx_3fmtB8_)
  (data $.rodata (i32.const 1048576) " index out of bounds: the len is \c0\12 but the index is \c0\00\c0\02: \c0\00/home/ken/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashx-0.8.0/src/constraints.rs\00/home/ken/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashx-0.8.0/src/register.rs\00/home/ken/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashx-0.8.0/src/scheduler.rs\00/home/ken/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashx-0.8.0/src/compiler.rs\00/home/ken/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/equix-0.6.2/src/solution.rs\00/home/ken/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/equix-0.6.2/src/bucket_array/mem.rs\00/home/ken/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hashx-0.8.0/src/program.rs\00/rustc/59807616e1fa2540724bfbac14d7976d7e4a3860/library/alloc/src/raw_vec/mod.rs\00/rust/deps/dlmalloc-0.2.11/src/dlmalloc.rs\00/home/ken/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/blake2-0.10.6/src/lib.rs\00Tor hs intro v1\00assertion failed: self.state.item_range(bucket).contains(&item)\08\02\10\00b\00\00\00\d6\00\00\00\09\00\00\00\08\02\10\00b\00\00\00\d9\00\00\00\12\00\00\00\08\02\10\00b\00\00\00%\01\00\00\09\00\00\00\08\02\10\00b\00\00\00(\01\00\00\12\00\00\00\08\02\10\00b\00\00\001\01\00\00\09\00\00\00\08\02\10\00b\00\00\004\01\00\00\12\00\00\00destination and source slices have different lengths\ad\01\10\00Z\00\00\00\a6\00\00\00\0e\00\00\00internal error: entered unreachable codeR\01\10\00Z\00\00\001\00\00\00\09\00\00\00()\00\00=\00\10\00]\00\00\00h\01\00\00\09\00\00\00\00\00\00\00\00\00\00\00\01\00\00\00\01\00\00\00\9b\00\10\00Z\00\00\00|\00\00\00\09\00\00\00generated programs always have a target before branch\00\00\00k\02\10\00Y\00\00\00\17\01\00\00\1e\00\00\00\08\07\04\04\05\06\07\03\08\07\04\04\02\01\00\00\f6\00\10\00[\00\00\00*\02\00\00\0f\00\00\00\f6\00\10\00[\00\00\00|\01\00\00\09\00\00\00\f6\00\10\00[\00\00\00\1c\02\00\00\0d\00\00\00instruction retired prior to end of schedule\f6\00\10\00[\00\00\00\d5\01\00\00\0e\00\00\00\f6\00\10\00[\00\00\00C\01\00\00\09\00\00\00HashX v1assertion failed: key_size <= U64::to_usize()\00\00\00A\03\10\00W\00\00\00r\00\00\00\01\00\00\00assertion failed: output_size <= U64::to_usize()assertion failed: salt.len() <= lengthassertion failed: persona.len() <= length\00\03\00\00\00\0c\00\00\00\04\00\00\00\04\00\00\00\05\00\00\00\06\00\00\00\00\00\00\00\08\00\00\00\04\00\00\00\07\00\00\00\08\00\00\00\09\00\00\00\0a\00\00\00\0b\00\00\00\10\00\00\00\04\00\00\00\0c\00\00\00\0d\00\00\00\0e\00\00\00\0f\00\00\00m]\cb\d6,P\ebcxA\a6Wq\1b\8b\b9\15\a2\5cU4U\07\d4Sx\ad\81Q\f0\a3\f7assertion failed: psize >= size + min_overhead\00\00\16\03\10\00*\00\00\00\b1\04\00\00\09\00\00\00assertion failed: psize <= size + max_overhead\00\00\16\03\10\00*\00\00\00\b7\04\00\00\0d\00\00\00\00\00\00\00\08\00\00\00\04\00\00\00\10\00\00\00\03\00\00\00\0c\00\00\00\04\00\00\00\11\00\00\00capacity overflow\00\00\00\c5\02\10\00P\00\00\00\1c\00\00\00\05\00\00\0000010203040506070809101112131415161718192021222324252627282930313233343536373839404142434445464748495051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899"))
