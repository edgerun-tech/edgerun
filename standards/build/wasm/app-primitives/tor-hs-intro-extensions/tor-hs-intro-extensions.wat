(module
  (type (;0;) (func (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (type (;1;) (func (param i32 i32) (result i32)))
  (type (;2;) (func (param i32 i32 i32 i32 i32) (result i32)))
  (type (;3;) (func (param i32 i32 i32 i32 i32 i32) (result i32)))
  (type (;4;) (func (param i32 i32 i32 i32) (result i32)))
  (type (;5;) (func (param i32) (result i32)))
  (type (;6;) (func (result i32)))
  (func (;0;) (type 0) (param i32 i32 i32 i32 i32 i32 i32) (result i32)
    (local i32)
    i32.const -3
    local.set 7
    block  ;; label = @1
      local.get 1
      local.get 4
      call 1
      br_if 0 (;@1;)
      local.get 6
      local.get 0
      local.get 1
      local.get 2
      local.get 3
      local.get 5
      call 2
      local.tee 1
      i32.store
      i32.const -3
      i32.const 0
      local.get 1
      i64.extend_i32_u
      local.get 3
      i64.extend_i32_u
      i64.mul
      i64.const 32
      i64.shr_u
      i32.wrap_i64
      select
      local.set 7
    end
    local.get 7)
  (func (;1;) (type 1) (param i32 i32) (result i32)
    (local i32 i32)
    i32.const 0
    local.set 2
    i32.const 0
    local.set 3
    block  ;; label = @1
      loop  ;; label = @2
        local.get 3
        i32.const 4
        i32.eq
        br_if 1 (;@1;)
        local.get 1
        local.get 3
        i32.add
        i32.load8_u
        local.get 0
        local.get 3
        i32.add
        i32.load8_u
        i32.xor
        local.get 2
        i32.or
        local.set 2
        local.get 3
        i32.const 1
        i32.add
        local.set 3
        br 0 (;@2;)
      end
    end
    i32.const -3
    i32.const 0
    local.get 2
    i32.const 255
    i32.and
    select)
  (func (;2;) (type 2) (param i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)
    global.get 0
    i32.const 576
    i32.sub
    local.tee 5
    global.set 0
    i32.const 0
    local.set 6
    local.get 5
    i32.const 8
    i32.add
    i32.const 0
    i64.load offset=1048584 align=1
    i64.store
    local.get 5
    i32.const 24
    i32.add
    local.get 0
    i32.const 8
    i32.add
    i64.load align=1
    i64.store
    local.get 5
    i32.const 32
    i32.add
    local.get 0
    i32.const 16
    i32.add
    i64.load align=1
    i64.store
    local.get 5
    i32.const 40
    i32.add
    local.get 0
    i32.const 24
    i32.add
    i64.load align=1
    i64.store
    local.get 5
    i32.const 56
    i32.add
    local.get 1
    i32.const 8
    i32.add
    i64.load align=1
    i64.store
    local.get 5
    i32.const 64
    i32.add
    local.get 1
    i32.const 16
    i32.add
    i64.load align=1
    i64.store
    local.get 5
    i32.const 72
    i32.add
    local.get 1
    i32.const 24
    i32.add
    i64.load align=1
    i64.store
    local.get 5
    i32.const 0
    i64.load offset=1048576 align=1
    i64.store
    local.get 5
    local.get 0
    i64.load align=1
    i64.store offset=16
    local.get 5
    local.get 1
    i64.load align=1
    i64.store offset=48
    local.get 5
    i32.const 88
    i32.add
    local.get 2
    i32.const 8
    i32.add
    i64.load align=1
    i64.store
    local.get 5
    local.get 2
    i64.load align=1
    i64.store offset=80
    local.get 5
    i32.const 284
    i32.add
    local.get 4
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=4
    local.get 5
    i64.const 7640891576939301132
    i64.store offset=112
    local.get 5
    local.get 4
    i64.load align=1
    i64.store offset=276 align=4
    block  ;; label = @1
      i32.const 56
      i32.eqz
      br_if 0 (;@1;)
      local.get 5
      i32.const 96
      i32.add
      i32.const 24
      i32.add
      i32.const 1048608
      i32.const 56
      memory.copy
    end
    block  ;; label = @1
      i32.const 96
      i32.eqz
      br_if 0 (;@1;)
      local.get 5
      i32.const 96
      i32.add
      i32.const 80
      i32.add
      local.get 5
      i32.const 96
      memory.copy
    end
    local.get 5
    i32.const 300
    i32.add
    i32.const 0
    i32.store
    local.get 5
    i32.const 116
    i32.store8 offset=304
    local.get 5
    i64.const 0
    i64.store offset=292 align=4
    local.get 5
    local.get 3
    i32.store8 offset=275
    local.get 5
    local.get 3
    i32.const 8
    i32.shr_u
    i32.store8 offset=274
    local.get 5
    local.get 3
    i32.const 16
    i32.shr_u
    i32.store8 offset=273
    local.get 5
    local.get 3
    i32.const 24
    i32.shr_u
    i32.store8 offset=272
    loop (result i32)  ;; label = @1
      block  ;; label = @2
        local.get 6
        i32.const 128
        i32.ne
        br_if 0 (;@2;)
        i32.const 0
        local.set 6
        block  ;; label = @3
          loop  ;; label = @4
            local.get 6
            i32.const 64
            i32.eq
            br_if 1 (;@3;)
            local.get 5
            i32.const 448
            i32.add
            local.get 6
            i32.add
            local.tee 0
            i32.const 64
            i32.add
            local.get 6
            i32.const 1048600
            i32.add
            i64.load
            i64.store
            local.get 0
            local.get 5
            i32.const 96
            i32.add
            local.get 6
            i32.add
            i32.const 16
            i32.add
            i64.load
            i64.store
            local.get 6
            i32.const 8
            i32.add
            local.set 6
            br 0 (;@4;)
          end
        end
        local.get 5
        local.get 5
        i64.load offset=400
        local.tee 7
        local.get 5
        i64.load offset=344
        local.tee 8
        local.get 5
        i64.load offset=488
        local.tee 9
        local.get 5
        i64.load offset=456
        i64.add
        local.get 5
        i64.load offset=336
        local.tee 10
        i64.add
        local.tee 11
        i64.add
        local.get 11
        local.get 5
        i64.load offset=552
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 12
        local.get 5
        i64.load offset=520
        i64.add
        local.tee 13
        local.get 9
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 15
        i64.add
        local.get 5
        i64.load offset=360
        local.tee 9
        local.get 5
        i64.load offset=496
        local.tee 16
        local.get 5
        i64.load offset=464
        i64.add
        local.get 5
        i64.load offset=352
        local.tee 11
        i64.add
        local.tee 17
        i64.add
        local.get 5
        i64.load offset=560
        local.get 17
        i64.xor
        i64.const -1
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 17
        local.get 5
        i64.load offset=528
        i64.add
        local.tee 18
        local.get 16
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 16
        i64.add
        local.tee 19
        local.get 17
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 20
        local.get 18
        i64.add
        local.tee 21
        local.get 16
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 22
        i64.add
        local.tee 23
        local.get 5
        i64.load offset=408
        local.tee 16
        i64.add
        local.get 23
        local.get 5
        i64.load offset=328
        local.tee 17
        local.get 5
        i64.load offset=480
        local.tee 24
        local.get 5
        i64.load offset=448
        i64.add
        local.get 5
        i64.load offset=320
        local.tee 18
        i64.add
        local.tee 25
        i64.add
        local.get 5
        i64.load offset=544
        local.get 25
        i64.xor
        i64.const 116
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 25
        local.get 5
        i64.load offset=512
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
        local.tee 25
        local.get 5
        i64.load offset=376
        local.tee 23
        local.get 5
        i64.load offset=504
        local.tee 30
        local.get 5
        i64.load offset=472
        i64.add
        local.get 5
        i64.load offset=368
        local.tee 24
        i64.add
        local.tee 31
        i64.add
        local.get 5
        i64.load offset=568
        local.get 31
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 31
        local.get 5
        i64.load offset=536
        i64.add
        local.tee 32
        local.get 30
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 30
        i64.add
        local.tee 33
        local.get 31
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 31
        local.get 32
        i64.add
        local.tee 32
        i64.add
        local.tee 34
        local.get 22
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 22
        i64.add
        local.tee 35
        local.get 25
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 36
        local.get 34
        i64.add
        local.tee 34
        local.get 22
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 37
        local.get 5
        i64.load offset=392
        local.tee 22
        i64.add
        local.get 5
        i64.load offset=416
        local.tee 25
        local.get 19
        i64.add
        local.get 32
        local.get 30
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 30
        i64.add
        local.tee 32
        local.get 5
        i64.load offset=424
        local.tee 19
        i64.add
        local.get 32
        local.get 15
        local.get 12
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 38
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 32
        local.get 29
        local.get 26
        i64.add
        local.tee 15
        i64.add
        local.tee 26
        local.get 30
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 30
        i64.add
        local.tee 39
        local.get 5
        i64.load offset=440
        local.tee 12
        i64.add
        local.get 39
        local.get 12
        local.get 33
        local.get 15
        local.get 27
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 27
        i64.add
        local.get 5
        i64.load offset=432
        local.tee 15
        i64.add
        local.tee 33
        i64.add
        local.get 33
        local.get 20
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 20
        local.get 38
        local.get 13
        i64.add
        local.tee 13
        i64.add
        local.tee 33
        local.get 27
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 27
        i64.add
        local.tee 38
        local.get 20
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 20
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 39
        local.get 22
        local.get 13
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        local.get 28
        i64.add
        local.get 5
        i64.load offset=384
        local.tee 13
        i64.add
        local.tee 28
        i64.add
        local.get 31
        local.get 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 21
        i64.add
        local.tee 21
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 31
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        local.get 21
        i64.add
        local.tee 21
        i64.add
        local.tee 40
        local.get 37
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 37
        i64.add
        local.tee 41
        local.get 16
        i64.add
        local.get 38
        local.get 19
        i64.add
        local.get 30
        local.get 32
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 30
        local.get 26
        i64.add
        local.tee 26
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 32
        local.get 24
        i64.add
        local.get 32
        local.get 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 34
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 34
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 38
        local.get 23
        i64.add
        local.get 38
        local.get 35
        local.get 11
        i64.add
        local.get 21
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 21
        local.get 13
        i64.add
        local.get 30
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 20
        local.get 33
        i64.add
        local.tee 20
        i64.add
        local.tee 30
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 33
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 35
        local.get 31
        local.get 15
        i64.add
        local.get 20
        local.get 27
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 7
        i64.add
        local.get 27
        local.get 36
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 27
        local.get 26
        i64.add
        local.tee 26
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 31
        local.get 27
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 27
        local.get 26
        i64.add
        local.tee 26
        i64.add
        local.tee 36
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 38
        local.get 9
        i64.add
        local.get 33
        local.get 18
        i64.add
        local.get 41
        local.get 39
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 40
        i64.add
        local.tee 39
        local.get 37
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 37
        i64.add
        local.tee 40
        local.get 10
        i64.add
        local.get 40
        local.get 27
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 27
        local.get 32
        i64.add
        local.tee 32
        local.get 37
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 37
        i64.add
        local.tee 40
        local.get 27
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 27
        local.get 32
        i64.add
        local.tee 32
        local.get 37
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 37
        i64.add
        local.tee 41
        local.get 10
        i64.add
        local.get 41
        local.get 34
        local.get 9
        i64.add
        local.get 26
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 26
        local.get 8
        i64.add
        local.get 26
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 21
        local.get 30
        i64.add
        local.tee 21
        i64.add
        local.tee 30
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 33
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 34
        local.get 31
        local.get 17
        i64.add
        local.get 21
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 21
        local.get 25
        i64.add
        local.get 21
        local.get 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 39
        i64.add
        local.tee 28
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 31
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 28
        i64.add
        local.tee 28
        i64.add
        local.tee 39
        local.get 37
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 37
        i64.add
        local.tee 41
        local.get 23
        i64.add
        local.get 33
        local.get 12
        i64.add
        local.get 38
        local.get 35
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 36
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 36
        local.get 19
        i64.add
        local.get 36
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 36
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 38
        local.get 17
        i64.add
        local.get 38
        local.get 40
        local.get 25
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 18
        i64.add
        local.get 28
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 26
        local.get 30
        i64.add
        local.tee 26
        i64.add
        local.tee 30
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 33
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 38
        local.get 31
        local.get 16
        i64.add
        local.get 26
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 26
        local.get 13
        i64.add
        local.get 26
        local.get 27
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 35
        i64.add
        local.tee 27
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 31
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 27
        i64.add
        local.tee 27
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 19
        i64.add
        local.get 33
        local.get 8
        i64.add
        local.get 41
        local.get 34
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 39
        i64.add
        local.tee 34
        local.get 37
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 37
        i64.add
        local.tee 39
        local.get 24
        i64.add
        local.get 39
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 37
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 37
        i64.add
        local.tee 39
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 37
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 37
        i64.add
        local.tee 41
        local.get 25
        i64.add
        local.get 41
        local.get 36
        local.get 22
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 11
        i64.add
        local.get 27
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 27
        local.get 28
        local.get 30
        i64.add
        local.tee 28
        i64.add
        local.tee 30
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 33
        local.get 27
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 27
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 36
        local.get 31
        local.get 7
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 15
        i64.add
        local.get 28
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 34
        i64.add
        local.tee 28
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 31
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 28
        i64.add
        local.tee 28
        i64.add
        local.tee 34
        local.get 37
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 37
        i64.add
        local.tee 41
        local.get 11
        i64.add
        local.get 33
        local.get 16
        i64.add
        local.get 40
        local.get 38
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 35
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 38
        local.get 15
        i64.add
        local.get 38
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 38
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 18
        i64.add
        local.get 40
        local.get 39
        local.get 8
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 17
        i64.add
        local.get 33
        local.get 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 27
        local.get 30
        i64.add
        local.tee 27
        i64.add
        local.tee 30
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 33
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 39
        local.get 31
        local.get 23
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 22
        i64.add
        local.get 27
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 35
        i64.add
        local.tee 27
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 31
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 27
        i64.add
        local.tee 27
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 10
        i64.add
        local.get 33
        local.get 9
        i64.add
        local.get 41
        local.get 36
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 34
        i64.add
        local.tee 34
        local.get 37
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 37
        local.get 7
        i64.add
        local.get 37
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 37
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 11
        i64.add
        local.get 41
        local.get 38
        local.get 12
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 13
        i64.add
        local.get 27
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 27
        local.get 28
        local.get 30
        i64.add
        local.tee 28
        i64.add
        local.tee 30
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 33
        local.get 27
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 27
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 38
        local.get 31
        local.get 10
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 24
        i64.add
        local.get 21
        local.get 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 34
        i64.add
        local.tee 28
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 31
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 28
        i64.add
        local.tee 28
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 24
        i64.add
        local.get 33
        local.get 7
        i64.add
        local.get 40
        local.get 39
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 35
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 39
        local.get 12
        i64.add
        local.get 39
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 39
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 13
        i64.add
        local.get 40
        local.get 37
        local.get 9
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 23
        i64.add
        local.get 33
        local.get 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 27
        local.get 30
        i64.add
        local.tee 27
        i64.add
        local.tee 30
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 33
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 37
        local.get 31
        local.get 22
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 18
        i64.add
        local.get 27
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 35
        i64.add
        local.tee 27
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 31
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 27
        i64.add
        local.tee 27
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 18
        i64.add
        local.get 33
        local.get 16
        i64.add
        local.get 41
        local.get 38
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 34
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 38
        local.get 25
        i64.add
        local.get 38
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 38
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 16
        i64.add
        local.get 41
        local.get 39
        local.get 8
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 19
        i64.add
        local.get 27
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 27
        local.get 28
        local.get 30
        i64.add
        local.tee 28
        i64.add
        local.tee 30
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 33
        local.get 27
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 27
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 39
        local.get 31
        local.get 15
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 17
        i64.add
        local.get 21
        local.get 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 34
        i64.add
        local.tee 28
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 31
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 28
        i64.add
        local.tee 28
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 12
        i64.add
        local.get 33
        local.get 13
        i64.add
        local.get 40
        local.get 37
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 35
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 37
        local.get 8
        i64.add
        local.get 37
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 37
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 15
        i64.add
        local.get 40
        local.get 38
        local.get 24
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 7
        i64.add
        local.get 33
        local.get 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 27
        local.get 30
        i64.add
        local.tee 27
        i64.add
        local.tee 30
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 33
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 38
        local.get 31
        local.get 10
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 25
        i64.add
        local.get 27
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 35
        i64.add
        local.tee 27
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 31
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 27
        i64.add
        local.tee 27
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 15
        i64.add
        local.get 33
        local.get 23
        i64.add
        local.get 41
        local.get 39
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 34
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 39
        local.get 9
        i64.add
        local.get 39
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 39
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 19
        i64.add
        local.get 41
        local.get 37
        local.get 17
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 22
        i64.add
        local.get 27
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 27
        local.get 28
        local.get 30
        i64.add
        local.tee 28
        i64.add
        local.tee 30
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 33
        local.get 27
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 27
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 37
        local.get 31
        local.get 11
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 19
        i64.add
        local.get 28
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 34
        i64.add
        local.tee 28
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 31
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 28
        i64.add
        local.tee 28
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 22
        i64.add
        local.get 33
        local.get 11
        i64.add
        local.get 40
        local.get 38
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 35
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 38
        local.get 7
        i64.add
        local.get 38
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 38
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 10
        i64.add
        local.get 40
        local.get 39
        local.get 17
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 12
        i64.add
        local.get 28
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 27
        local.get 30
        i64.add
        local.tee 27
        i64.add
        local.tee 30
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 33
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 39
        local.get 31
        local.get 25
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 9
        i64.add
        local.get 27
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 35
        i64.add
        local.tee 27
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 31
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 27
        i64.add
        local.tee 27
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 25
        i64.add
        local.get 33
        local.get 24
        i64.add
        local.get 41
        local.get 37
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 34
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 37
        local.get 8
        i64.add
        local.get 37
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 37
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 17
        i64.add
        local.get 38
        local.get 13
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 16
        i64.add
        local.get 27
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 27
        local.get 28
        local.get 30
        i64.add
        local.tee 28
        i64.add
        local.tee 30
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 33
        local.get 27
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 27
        local.get 41
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 38
        local.get 31
        local.get 18
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 23
        i64.add
        local.get 28
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 34
        i64.add
        local.tee 28
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 31
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 28
        i64.add
        local.tee 28
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 13
        i64.add
        local.get 33
        local.get 8
        i64.add
        local.get 40
        local.get 39
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 35
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 39
        local.get 22
        i64.add
        local.get 39
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 39
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 24
        i64.add
        local.get 40
        local.get 37
        local.get 23
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 15
        i64.add
        local.get 28
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 27
        local.get 30
        i64.add
        local.tee 27
        i64.add
        local.tee 30
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 33
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 37
        local.get 31
        local.get 19
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 16
        i64.add
        local.get 27
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 35
        i64.add
        local.tee 27
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 31
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 27
        i64.add
        local.tee 27
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 16
        i64.add
        local.get 33
        local.get 12
        i64.add
        local.get 41
        local.get 38
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 34
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 38
        local.get 11
        i64.add
        local.get 38
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 38
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 8
        i64.add
        local.get 41
        local.get 39
        local.get 10
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 7
        i64.add
        local.get 27
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 27
        local.get 28
        local.get 30
        i64.add
        local.tee 28
        i64.add
        local.tee 30
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 33
        local.get 27
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 27
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 39
        local.get 31
        local.get 9
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 18
        i64.add
        local.get 28
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 34
        i64.add
        local.tee 28
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 31
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 28
        i64.add
        local.tee 28
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 17
        i64.add
        local.get 33
        local.get 18
        i64.add
        local.get 40
        local.get 37
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 35
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 37
        local.get 13
        i64.add
        local.get 37
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 37
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 11
        i64.add
        local.get 40
        local.get 38
        local.get 15
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 22
        i64.add
        local.get 28
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 27
        local.get 30
        i64.add
        local.tee 27
        i64.add
        local.tee 30
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 33
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 38
        local.get 31
        local.get 24
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 12
        i64.add
        local.get 27
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 35
        i64.add
        local.tee 27
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 31
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 27
        i64.add
        local.tee 27
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 23
        i64.add
        local.get 33
        local.get 19
        i64.add
        local.get 41
        local.get 39
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 34
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 39
        local.get 23
        i64.add
        local.get 39
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 39
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 24
        i64.add
        local.get 41
        local.get 37
        local.get 7
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 9
        i64.add
        local.get 27
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 27
        local.get 28
        local.get 30
        i64.add
        local.tee 28
        i64.add
        local.tee 30
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 33
        local.get 27
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 27
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 37
        local.get 31
        local.get 25
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 10
        i64.add
        local.get 28
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 34
        i64.add
        local.tee 28
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 31
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 28
        i64.add
        local.tee 28
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 8
        i64.add
        local.get 33
        local.get 17
        i64.add
        local.get 40
        local.get 38
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 35
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 38
        local.get 9
        i64.add
        local.get 38
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 38
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 25
        i64.add
        local.get 40
        local.get 39
        local.get 13
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 11
        i64.add
        local.get 28
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 27
        local.get 30
        i64.add
        local.tee 27
        i64.add
        local.tee 30
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 33
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 39
        local.get 31
        local.get 7
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 10
        i64.add
        local.get 27
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 35
        i64.add
        local.tee 27
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 31
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 27
        i64.add
        local.tee 27
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 11
        i64.add
        local.get 33
        local.get 22
        i64.add
        local.get 41
        local.get 37
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 34
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 37
        local.get 15
        i64.add
        local.get 37
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 37
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 9
        i64.add
        local.get 41
        local.get 38
        local.get 19
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 18
        i64.add
        local.get 27
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 27
        local.get 28
        local.get 30
        i64.add
        local.tee 28
        i64.add
        local.tee 30
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 33
        local.get 27
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 27
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 38
        local.get 31
        local.get 12
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 16
        i64.add
        local.get 28
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 34
        i64.add
        local.tee 28
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 31
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 28
        i64.add
        local.tee 28
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 25
        i64.add
        local.get 33
        local.get 24
        i64.add
        local.get 40
        local.get 39
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 35
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 39
        local.get 23
        i64.add
        local.get 39
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 39
        local.get 21
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 21
        local.get 32
        i64.add
        local.tee 32
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 19
        i64.add
        local.get 40
        local.get 37
        local.get 10
        i64.add
        local.get 28
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 28
        local.get 8
        i64.add
        local.get 28
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 28
        local.get 27
        local.get 30
        i64.add
        local.tee 27
        i64.add
        local.tee 30
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 33
        local.get 28
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 28
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 37
        local.get 31
        local.get 18
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 17
        i64.add
        local.get 27
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 35
        i64.add
        local.tee 27
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 31
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 27
        i64.add
        local.tee 27
        i64.add
        local.tee 35
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 40
        local.get 22
        i64.add
        local.get 33
        local.get 7
        i64.add
        local.get 41
        local.get 38
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 33
        local.get 34
        i64.add
        local.tee 34
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 38
        local.get 16
        i64.add
        local.get 38
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 36
        i64.add
        local.tee 38
        local.get 26
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 26
        local.get 32
        i64.add
        local.tee 32
        local.get 36
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 36
        i64.add
        local.tee 41
        local.get 12
        i64.add
        local.get 41
        local.get 39
        local.get 15
        i64.add
        local.get 27
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.add
        local.tee 27
        local.get 12
        i64.add
        local.get 27
        local.get 33
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 12
        local.get 28
        local.get 30
        i64.add
        local.tee 27
        i64.add
        local.tee 28
        local.get 20
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 20
        i64.add
        local.tee 30
        local.get 12
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 12
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 33
        local.get 31
        local.get 13
        i64.add
        local.get 27
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 14
        i64.add
        local.tee 27
        local.get 22
        i64.add
        local.get 27
        local.get 21
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 22
        local.get 34
        i64.add
        local.tee 21
        local.get 14
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 14
        i64.add
        local.tee 27
        local.get 22
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 22
        local.get 21
        i64.add
        local.tee 21
        i64.add
        local.tee 31
        local.get 36
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 34
        i64.add
        local.tee 36
        local.get 16
        i64.add
        local.get 30
        local.get 19
        i64.add
        local.get 40
        local.get 37
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 16
        local.get 35
        i64.add
        local.tee 19
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 30
        local.get 24
        i64.add
        local.get 30
        local.get 22
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 24
        local.get 32
        i64.add
        local.tee 22
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 29
        i64.add
        local.tee 30
        local.get 24
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 24
        local.get 22
        i64.add
        local.tee 22
        local.get 29
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 29
        i64.add
        local.tee 32
        local.get 23
        i64.add
        local.get 32
        local.get 38
        local.get 11
        i64.add
        local.get 21
        local.get 14
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 11
        i64.add
        local.tee 23
        local.get 13
        i64.add
        local.get 23
        local.get 16
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 16
        local.get 12
        local.get 28
        i64.add
        local.tee 23
        i64.add
        local.tee 12
        local.get 11
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 11
        i64.add
        local.tee 13
        local.get 16
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 16
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 14
        local.get 27
        local.get 15
        i64.add
        local.get 23
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 23
        i64.add
        local.tee 15
        local.get 7
        i64.add
        local.get 15
        local.get 26
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 7
        local.get 19
        i64.add
        local.tee 19
        local.get 23
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 23
        i64.add
        local.tee 15
        local.get 7
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 7
        local.get 19
        i64.add
        local.tee 19
        i64.add
        local.tee 20
        local.get 29
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 21
        i64.add
        local.tee 26
        i64.store offset=464
        local.get 5
        local.get 26
        local.get 14
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 14
        i64.store offset=552
        local.get 5
        local.get 14
        local.get 20
        i64.add
        local.tee 14
        i64.store offset=512
        local.get 5
        local.get 14
        local.get 21
        i64.xor
        i64.const 1
        i64.rotl
        i64.store offset=504
        local.get 5
        local.get 25
        local.get 15
        local.get 17
        i64.add
        local.get 16
        local.get 12
        i64.add
        local.tee 16
        local.get 11
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 11
        i64.add
        local.tee 17
        i64.add
        local.get 17
        local.get 24
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 17
        local.get 36
        local.get 33
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 24
        local.get 31
        i64.add
        local.tee 25
        i64.add
        local.tee 12
        local.get 11
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 11
        i64.add
        local.tee 15
        i64.store offset=448
        local.get 5
        local.get 15
        local.get 17
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 17
        i64.store offset=568
        local.get 5
        local.get 17
        local.get 12
        i64.add
        local.tee 17
        i64.store offset=528
        local.get 5
        local.get 17
        local.get 11
        i64.xor
        i64.const 1
        i64.rotl
        i64.store offset=488
        local.get 5
        local.get 10
        local.get 13
        local.get 18
        i64.add
        local.get 25
        local.get 34
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 11
        i64.add
        local.tee 17
        i64.add
        local.get 17
        local.get 7
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 7
        local.get 22
        i64.add
        local.tee 10
        local.get 11
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 11
        i64.add
        local.tee 17
        i64.store offset=456
        local.get 5
        local.get 17
        local.get 7
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 7
        i64.store offset=544
        local.get 5
        local.get 7
        local.get 10
        i64.add
        local.tee 7
        i64.store offset=536
        local.get 5
        local.get 7
        local.get 11
        i64.xor
        i64.const 1
        i64.rotl
        i64.store offset=496
        local.get 5
        local.get 8
        local.get 30
        local.get 9
        i64.add
        local.get 19
        local.get 23
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 7
        i64.add
        local.tee 10
        i64.add
        local.get 10
        local.get 24
        i64.xor
        i64.const 32
        i64.rotl
        local.tee 8
        local.get 16
        i64.add
        local.tee 10
        local.get 7
        i64.xor
        i64.const 40
        i64.rotl
        local.tee 7
        i64.add
        local.tee 9
        i64.store offset=472
        local.get 5
        local.get 9
        local.get 8
        i64.xor
        i64.const 48
        i64.rotl
        local.tee 8
        i64.store offset=560
        local.get 5
        local.get 8
        local.get 10
        i64.add
        local.tee 8
        i64.store offset=520
        local.get 5
        local.get 8
        local.get 7
        i64.xor
        i64.const 1
        i64.rotl
        i64.store offset=480
        i32.const 0
        local.set 6
        block  ;; label = @3
          loop  ;; label = @4
            local.get 6
            i32.const 64
            i32.eq
            br_if 1 (;@3;)
            local.get 5
            i32.const 96
            i32.add
            local.get 6
            i32.add
            i32.const 16
            i32.add
            local.tee 0
            local.get 5
            i32.const 448
            i32.add
            local.get 6
            i32.add
            local.tee 1
            i64.load
            local.get 0
            i64.load
            i64.xor
            local.get 1
            i32.const 64
            i32.add
            i64.load
            i64.xor
            i64.store
            local.get 6
            i32.const 8
            i32.add
            local.set 6
            br 0 (;@4;)
          end
        end
        local.get 5
        i32.load offset=112
        local.set 6
        local.get 5
        i32.const 576
        i32.add
        global.set 0
        local.get 6
        i32.const 24
        i32.shl
        local.get 6
        i32.const 65280
        i32.and
        i32.const 8
        i32.shl
        i32.or
        local.get 6
        i32.const 8
        i32.shr_u
        i32.const 65280
        i32.and
        local.get 6
        i32.const 24
        i32.shr_u
        i32.or
        i32.or
        return
      end
      local.get 5
      i32.const 320
      i32.add
      local.get 6
      i32.add
      local.get 5
      i32.const 96
      i32.add
      local.get 6
      i32.add
      i32.const 80
      i32.add
      i64.load
      i64.store
      local.get 6
      i32.const 8
      i32.add
      local.set 6
      br 0 (;@1;)
    end)
  (func (;3;) (type 2) (param i32 i32 i32 i32 i32) (result i32)
    local.get 4
    i32.const 8
    i32.add
    i32.const 0
    i64.load offset=1048584 align=1
    i64.store align=1
    local.get 4
    i32.const 0
    i64.load offset=1048576 align=1
    i64.store align=1
    local.get 4
    i32.const 40
    i32.add
    local.get 0
    i32.const 24
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 4
    i32.const 32
    i32.add
    local.get 0
    i32.const 16
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 4
    i32.const 24
    i32.add
    local.get 0
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 4
    local.get 0
    i64.load align=1
    i64.store offset=16 align=1
    local.get 4
    local.get 1
    i64.load align=1
    i64.store offset=48 align=1
    local.get 4
    i32.const 56
    i32.add
    local.get 1
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 4
    i32.const 64
    i32.add
    local.get 1
    i32.const 16
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 4
    i32.const 72
    i32.add
    local.get 1
    i32.const 24
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 4
    local.get 2
    i64.load align=1
    i64.store offset=80 align=1
    local.get 4
    i32.const 88
    i32.add
    local.get 2
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 4
    local.get 3
    i32.const 24
    i32.shl
    local.get 3
    i32.const 65280
    i32.and
    i32.const 8
    i32.shl
    i32.or
    local.get 3
    i32.const 8
    i32.shr_u
    i32.const 65280
    i32.and
    local.get 3
    i32.const 24
    i32.shr_u
    i32.or
    i32.or
    i32.store offset=96 align=1
    i32.const 100)
  (func (;4;) (type 3) (param i32 i32 i32 i32 i32 i32) (result i32)
    block  ;; label = @1
      local.get 1
      i32.const 41
      i32.eq
      br_if 0 (;@1;)
      i32.const -1
      return
    end
    block  ;; label = @1
      local.get 0
      i32.load8_u
      i32.const 1
      i32.eq
      br_if 0 (;@1;)
      i32.const -4
      return
    end
    local.get 2
    local.get 0
    i64.load offset=1 align=1
    i64.store align=1
    local.get 2
    i32.const 8
    i32.add
    local.get 0
    i32.const 9
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 3
    local.get 0
    i32.load offset=17 align=1
    local.tee 1
    i32.const 24
    i32.shl
    local.get 1
    i32.const 65280
    i32.and
    i32.const 8
    i32.shl
    i32.or
    local.get 1
    i32.const 8
    i32.shr_u
    i32.const 65280
    i32.and
    local.get 1
    i32.const 24
    i32.shr_u
    i32.or
    i32.or
    i32.store
    local.get 4
    local.get 0
    i32.load offset=21 align=1
    i32.store align=1
    local.get 5
    i32.const 8
    i32.add
    local.get 0
    i32.const 33
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 5
    local.get 0
    i64.load offset=25 align=1
    i64.store align=1
    i32.const 0)
  (func (;5;) (type 4) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32)
    i32.const -1
    local.set 4
    block  ;; label = @1
      local.get 2
      i32.const 256
      i32.ge_u
      br_if 0 (;@1;)
      local.get 0
      local.get 1
      call 6
      local.tee 4
      br_if 0 (;@1;)
      local.get 0
      i32.load8_u
      local.set 1
      i32.const 1
      local.set 4
      local.get 2
      i32.const 255
      i32.and
      local.set 5
      i32.const 1
      local.set 6
      loop  ;; label = @2
        local.get 1
        i32.eqz
        br_if 1 (;@1;)
        local.get 0
        local.get 6
        i32.add
        local.tee 7
        i32.const 1
        i32.add
        i32.load8_u
        local.set 8
        block  ;; label = @3
          local.get 7
          i32.load8_u
          local.get 5
          i32.ne
          br_if 0 (;@3;)
          local.get 3
          local.get 8
          i32.store offset=8
          local.get 3
          local.get 2
          i32.store
          local.get 3
          local.get 6
          i32.const 2
          i32.add
          local.tee 1
          i32.store offset=4
          local.get 3
          local.get 1
          local.get 8
          i32.add
          i32.store offset=12
          i32.const 0
          local.set 4
          br 2 (;@1;)
        end
        local.get 1
        i32.const -1
        i32.add
        local.set 1
        local.get 6
        local.get 8
        i32.add
        i32.const 2
        i32.add
        local.set 6
        br 0 (;@2;)
      end
    end
    local.get 4)
  (func (;6;) (type 1) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        i32.const -427
        i32.add
        i32.const -427
        i32.gt_u
        br_if 0 (;@2;)
        i32.const -2
        local.set 2
        br 1 (;@1;)
      end
      local.get 0
      i32.load8_u
      local.set 3
      i32.const 1
      local.set 4
      loop  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 3
            i32.eqz
            br_if 0 (;@4;)
            i32.const -1
            local.set 2
            local.get 4
            i32.const 2
            i32.add
            local.tee 5
            local.get 1
            i32.gt_u
            br_if 3 (;@1;)
            local.get 5
            local.get 0
            local.get 4
            i32.add
            local.tee 6
            i32.load8_u offset=1
            local.tee 7
            i32.add
            local.tee 4
            local.get 1
            i32.gt_u
            br_if 3 (;@1;)
            block  ;; label = @5
              local.get 6
              i32.load8_u
              i32.const 255
              i32.and
              local.tee 6
              i32.const 1
              i32.ne
              br_if 0 (;@5;)
              local.get 7
              br_if 4 (;@1;)
            end
            local.get 6
            i32.const 2
            i32.ne
            br_if 1 (;@3;)
            local.get 7
            i32.const 41
            i32.ne
            br_if 3 (;@1;)
            local.get 0
            local.get 5
            i32.add
            i32.load8_u
            i32.const 1
            i32.eq
            br_if 1 (;@3;)
            i32.const -4
            return
          end
          i32.const -1
          i32.const 0
          local.get 4
          local.get 1
          i32.ne
          select
          return
        end
        local.get 3
        i32.const -1
        i32.add
        local.set 3
        br 0 (;@2;)
      end
    end
    local.get 2)
  (func (;7;) (type 4) (param i32 i32 i32 i32) (result i32)
    (local i32)
    block  ;; label = @1
      local.get 0
      local.get 1
      call 6
      local.tee 1
      br_if 0 (;@1;)
      i32.const 1
      local.set 1
      local.get 2
      local.get 0
      i32.load8_u
      i32.ge_u
      br_if 0 (;@1;)
      i32.const 1
      local.set 1
      local.get 0
      i32.const 1
      i32.add
      local.set 4
      block  ;; label = @2
        loop  ;; label = @3
          local.get 2
          i32.eqz
          br_if 1 (;@2;)
          local.get 2
          i32.const -1
          i32.add
          local.set 2
          local.get 1
          local.get 4
          local.get 1
          i32.add
          i32.load8_u
          i32.add
          i32.const 2
          i32.add
          local.set 1
          br 0 (;@3;)
        end
      end
      local.get 0
      local.get 1
      i32.add
      local.tee 2
      i32.load8_u
      local.set 4
      local.get 3
      local.get 1
      i32.const 2
      i32.add
      local.tee 1
      i32.store offset=4
      local.get 3
      local.get 4
      i32.store
      local.get 3
      local.get 2
      i32.const 1
      i32.add
      local.tee 2
      i32.load8_u
      i32.store offset=8
      local.get 3
      local.get 1
      local.get 2
      i32.load8_u
      i32.add
      i32.store offset=12
      i32.const 0
      local.set 1
    end
    local.get 1)
  (func (;8;) (type 1) (param i32 i32) (result i32)
    local.get 0
    local.get 1
    call 6)
  (func (;9;) (type 2) (param i32 i32 i32 i32 i32) (result i32)
    local.get 0
    i32.const 297
    i32.store16 offset=4 align=1
    local.get 0
    i32.const 33554690
    i32.store align=1
    local.get 0
    local.get 1
    i64.load align=1
    i64.store offset=6 align=1
    local.get 0
    i32.const 14
    i32.add
    local.get 1
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 0
    local.get 2
    i32.const 24
    i32.shl
    local.get 2
    i32.const 65280
    i32.and
    i32.const 8
    i32.shl
    i32.or
    local.get 2
    i32.const 8
    i32.shr_u
    i32.const 65280
    i32.and
    local.get 2
    i32.const 24
    i32.shr_u
    i32.or
    i32.or
    i32.store offset=22 align=1
    local.get 0
    local.get 3
    i32.load align=1
    i32.store offset=26 align=1
    local.get 0
    local.get 4
    i64.load align=1
    i64.store offset=30 align=1
    local.get 0
    i32.const 38
    i32.add
    local.get 4
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 46)
  (func (;10;) (type 2) (param i32 i32 i32 i32 i32) (result i32)
    local.get 0
    i32.const 19464705
    i32.store align=1
    local.get 0
    local.get 1
    i64.load align=1
    i64.store offset=4 align=1
    local.get 0
    i32.const 12
    i32.add
    local.get 1
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 0
    local.get 2
    i32.const 24
    i32.shl
    local.get 2
    i32.const 65280
    i32.and
    i32.const 8
    i32.shl
    i32.or
    local.get 2
    i32.const 8
    i32.shr_u
    i32.const 65280
    i32.and
    local.get 2
    i32.const 24
    i32.shr_u
    i32.or
    i32.or
    i32.store offset=20 align=1
    local.get 0
    local.get 3
    i32.load align=1
    i32.store offset=24 align=1
    local.get 0
    local.get 4
    i64.load align=1
    i64.store offset=28 align=1
    local.get 0
    i32.const 36
    i32.add
    local.get 4
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 44)
  (func (;11;) (type 5) (param i32) (result i32)
    local.get 0
    i32.const 0
    i32.store8 offset=2
    local.get 0
    i32.const 257
    i32.store16 align=1
    i32.const 3)
  (func (;12;) (type 5) (param i32) (result i32)
    local.get 0
    i32.const 0
    i32.store8
    i32.const 1)
  (func (;13;) (type 6) (result i32)
    i32.const 100)
  (func (;14;) (type 6) (result i32)
    i32.const 41)
  (func (;15;) (type 6) (result i32)
    i32.const 1)
  (func (;16;) (type 6) (result i32)
    i32.const 300216)
  (table (;0;) 1 1 funcref)
  (memory (;0;) 17)
  (global (;0;) (mut i32) (i32.const 1048576))
  (export "memory" (memory 0))
  (export "tor_hs_intro_pow_effort_gate" (func 0))
  (export "tor_hs_intro_pow_seed_prefix_ok" (func 1))
  (export "tor_hs_intro_pow_blake2b_result" (func 2))
  (export "tor_hs_intro_pow_challenge" (func 3))
  (export "tor_hs_intro_ext_parse_pow" (func 4))
  (export "tor_hs_intro_ext_find" (func 5))
  (export "tor_hs_intro_ext_record" (func 7))
  (export "tor_hs_intro_ext_validate" (func 8))
  (export "tor_hs_intro_ext_build_cc_pow" (func 9))
  (export "tor_hs_intro_ext_build_pow" (func 10))
  (export "tor_hs_intro_ext_build_cc" (func 11))
  (export "tor_hs_intro_ext_empty" (func 12))
  (export "tor_hs_intro_ext_pow_challenge_len" (func 13))
  (export "tor_hs_intro_ext_pow_len" (func 14))
  (export "proto_abi_version" (func 15))
  (export "proto_standard_id" (func 16))
  (data (;0;) (i32.const 1048576) "Tor hs intro v1\00\00\00\00\00\00\00\00\00\08\c9\bc\f3g\e6\09j;\a7\ca\84\85\aeg\bb+\f8\94\fer\f3n<\f16\1d_:\f5O\a5\d1\82\e6\ad\7fR\0eQ\1fl>+\8ch\05\9bk\bdA\fb\ab\d9\83\1fy!~\13\19\cd\e0["))
