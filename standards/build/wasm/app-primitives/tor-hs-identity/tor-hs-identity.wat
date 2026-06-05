(module
  (type (;0;) (func (param i32 i32 i32) (result i32)))
  (type (;1;) (func (param i32 i32) (result i32)))
  (type (;2;) (func (param i32 i32 i32)))
  (type (;3;) (func (param i32)))
  (type (;4;) (func (param i32 i32)))
  (type (;5;) (func (result i32)))
  (func (;0;) (type 0) (param i32 i32 i32) (result i32)
    (local i32)
    global.get 0
    i32.const 112
    i32.sub
    local.tee 3
    global.set 0
    local.get 0
    local.get 3
    i32.const 13
    i32.add
    call 1
    drop
    local.get 3
    i32.const 53
    i32.add
    local.get 1
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 3
    i32.const 61
    i32.add
    local.get 1
    i32.const 16
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 3
    i32.const 69
    i32.add
    local.get 1
    i32.const 24
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 3
    i32.const 0
    i64.load offset=1048604 align=1
    i64.store offset=5 align=1
    local.get 3
    i32.const 0
    i64.load offset=1048599 align=1
    i64.store
    local.get 3
    local.get 1
    i64.load align=1
    i64.store offset=45 align=1
    local.get 3
    i32.const 77
    local.get 3
    i32.const 80
    i32.add
    call 2
    local.get 2
    i32.const 24
    i32.add
    local.get 3
    i32.const 80
    i32.add
    i32.const 24
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 2
    i32.const 16
    i32.add
    local.get 3
    i32.const 80
    i32.add
    i32.const 16
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 2
    i32.const 8
    i32.add
    local.get 3
    i32.const 80
    i32.add
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 2
    local.get 3
    i64.load offset=80 align=1
    i64.store align=1
    local.get 3
    i32.const 112
    i32.add
    global.set 0
    i32.const 0)
  (func (;1;) (type 1) (param i32 i32) (result i32)
    (local i32)
    global.get 0
    i32.const 80
    i32.sub
    local.tee 2
    global.set 0
    local.get 2
    i32.const 8
    i32.add
    i32.const 0
    i32.load16_u offset=1048610 align=1
    i32.store16
    local.get 2
    i32.const 18
    i32.add
    local.get 0
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=2
    local.get 2
    i32.const 26
    i32.add
    local.get 0
    i32.const 16
    i32.add
    i64.load align=1
    i64.store align=2
    local.get 2
    i32.const 34
    i32.add
    local.get 0
    i32.const 24
    i32.add
    i64.load align=1
    i64.store align=2
    local.get 2
    i32.const 0
    i64.load offset=1048602 align=1
    i64.store
    local.get 2
    local.get 0
    i64.load align=1
    i64.store offset=10 align=2
    local.get 2
    i32.const 42
    local.get 2
    i32.const 48
    i32.add
    call 2
    local.get 1
    i32.const 24
    i32.add
    local.get 2
    i32.const 48
    i32.add
    i32.const 24
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 1
    i32.const 16
    i32.add
    local.get 2
    i32.const 48
    i32.add
    i32.const 16
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 1
    i32.const 8
    i32.add
    local.get 2
    i32.const 48
    i32.add
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 1
    local.get 2
    i64.load offset=48 align=1
    i64.store align=1
    local.get 2
    i32.const 80
    i32.add
    global.set 0
    i32.const 0)
  (func (;2;) (type 2) (param i32 i32 i32)
    (local i32 i32 i32 i32 i32)
    global.get 0
    i32.const 352
    i32.sub
    local.tee 3
    global.set 0
    i32.const 0
    local.set 4
    block  ;; label = @1
      i32.const 204
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.const 8
      i32.add
      i32.const 0
      i32.const 204
      memory.fill
    end
    local.get 3
    i32.const 6
    i32.store8 offset=212
    local.get 3
    i32.const 213
    i32.add
    local.set 5
    block  ;; label = @1
      block  ;; label = @2
        local.get 3
        i32.load offset=208
        local.tee 6
        i32.eqz
        br_if 0 (;@2;)
        block  ;; label = @3
          i32.const 136
          local.get 6
          i32.sub
          local.tee 7
          local.get 1
          local.get 7
          local.get 1
          i32.lt_u
          select
          local.tee 4
          i32.eqz
          br_if 0 (;@3;)
          local.get 5
          local.get 6
          i32.add
          local.get 0
          local.get 4
          memory.copy
        end
        local.get 3
        local.get 3
        i32.load offset=208
        local.get 4
        i32.add
        local.tee 6
        i32.store offset=208
        block  ;; label = @3
          local.get 1
          local.get 7
          i32.gt_u
          br_if 0 (;@3;)
          local.get 6
          local.set 1
          br 2 (;@1;)
        end
        block  ;; label = @3
          local.get 6
          i32.const 136
          i32.ne
          br_if 0 (;@3;)
          local.get 3
          i32.const 8
          i32.add
          local.get 5
          i32.const 136
          call 3
          local.get 3
          i32.const 8
          i32.add
          call 4
        end
        local.get 1
        local.get 4
        i32.sub
        local.set 1
      end
      block  ;; label = @2
        local.get 1
        i32.eqz
        br_if 0 (;@2;)
        local.get 5
        local.get 0
        local.get 4
        i32.add
        local.get 1
        memory.copy
      end
      local.get 3
      local.get 1
      i32.store offset=208
    end
    local.get 3
    i32.const 8
    i32.add
    local.get 5
    local.get 1
    call 3
    block  ;; label = @1
      local.get 3
      i32.load offset=208
      local.tee 1
      i32.const 136
      i32.ne
      br_if 0 (;@1;)
      local.get 3
      i32.const 8
      i32.add
      call 4
      i32.const 0
      local.set 1
      local.get 3
      i32.const 0
      i32.store offset=208
    end
    local.get 3
    i32.const 8
    i32.add
    local.get 1
    i32.const -8
    i32.and
    i32.add
    local.tee 4
    local.get 3
    i64.load8_u offset=212
    local.get 1
    i32.const 3
    i32.shl
    i32.const 56
    i32.and
    i64.extend_i32_u
    i64.shl
    local.get 4
    i64.load
    i64.xor
    i64.store
    local.get 3
    local.get 3
    i64.load offset=136
    i64.const -9223372036854775808
    i64.xor
    i64.store offset=136
    local.get 3
    i32.const 8
    i32.add
    call 4
    i32.const 0
    local.set 1
    local.get 3
    i32.const 0
    i32.store offset=208
    block  ;; label = @1
      loop  ;; label = @2
        local.get 1
        i32.const 31
        i32.gt_u
        br_if 1 (;@1;)
        local.get 2
        local.get 1
        i32.add
        local.get 3
        i32.const 8
        i32.add
        local.get 1
        i32.add
        i64.load
        i64.store align=1
        local.get 1
        i32.const 8
        i32.add
        local.set 1
        br 0 (;@2;)
      end
    end
    local.get 3
    i32.const 352
    i32.add
    global.set 0)
  (func (;3;) (type 2) (param i32 i32 i32)
    (local i32 i32 i32 i32)
    global.get 0
    i32.const 16
    i32.sub
    local.tee 3
    global.set 0
    i32.const 0
    local.set 4
    block  ;; label = @1
      loop  ;; label = @2
        local.get 4
        i32.const 8
        i32.add
        local.tee 5
        local.get 2
        i32.gt_u
        br_if 1 (;@1;)
        local.get 0
        local.get 4
        i32.add
        local.tee 6
        local.get 1
        local.get 4
        i32.add
        i64.load align=1
        local.get 6
        i64.load
        i64.xor
        i64.store
        local.get 5
        local.set 4
        br 0 (;@2;)
      end
    end
    block  ;; label = @1
      local.get 2
      local.get 4
      i32.le_u
      br_if 0 (;@1;)
      local.get 3
      i64.const 0
      i64.store offset=8
      block  ;; label = @2
        local.get 2
        local.get 4
        i32.sub
        local.tee 5
        i32.eqz
        br_if 0 (;@2;)
        local.get 3
        i32.const 8
        i32.add
        local.get 1
        local.get 4
        i32.add
        local.get 5
        memory.copy
      end
      local.get 0
      local.get 4
      i32.add
      local.tee 4
      local.get 3
      i64.load offset=8
      local.get 4
      i64.load
      i64.xor
      i64.store
    end
    local.get 3
    i32.const 16
    i32.add
    global.set 0)
  (func (;4;) (type 3) (param i32)
    (local i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i32 i32 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)
    local.get 0
    i64.load offset=192
    local.set 1
    local.get 0
    i64.load offset=152
    local.set 2
    local.get 0
    i64.load offset=112
    local.set 3
    local.get 0
    i64.load offset=72
    local.set 4
    local.get 0
    i64.load offset=32
    local.set 5
    local.get 0
    i64.load offset=184
    local.set 6
    local.get 0
    i64.load offset=144
    local.set 7
    local.get 0
    i64.load offset=104
    local.set 8
    local.get 0
    i64.load offset=64
    local.set 9
    local.get 0
    i64.load offset=24
    local.set 10
    local.get 0
    i64.load offset=176
    local.set 11
    local.get 0
    i64.load offset=136
    local.set 12
    local.get 0
    i64.load offset=96
    local.set 13
    local.get 0
    i64.load offset=56
    local.set 14
    local.get 0
    i64.load offset=16
    local.set 15
    local.get 0
    i64.load offset=168
    local.set 16
    local.get 0
    i64.load offset=128
    local.set 17
    local.get 0
    i64.load offset=88
    local.set 18
    local.get 0
    i64.load offset=48
    local.set 19
    local.get 0
    i64.load offset=8
    local.set 20
    local.get 0
    i64.load offset=160
    local.set 21
    local.get 0
    i64.load offset=120
    local.set 22
    local.get 0
    i64.load offset=80
    local.set 23
    local.get 0
    i64.load offset=40
    local.set 24
    local.get 0
    i64.load
    local.set 25
    i32.const 0
    local.set 26
    i32.const 1048648
    local.set 27
    block  ;; label = @1
      loop  ;; label = @2
        local.get 26
        i32.const 23
        i32.gt_u
        br_if 1 (;@1;)
        local.get 9
        local.get 10
        i64.xor
        local.get 8
        i64.xor
        local.get 7
        i64.xor
        local.get 6
        i64.xor
        local.tee 28
        i64.const 1
        i64.rotl
        local.get 19
        local.get 20
        i64.xor
        local.get 18
        i64.xor
        local.get 17
        i64.xor
        local.get 16
        i64.xor
        local.tee 29
        i64.xor
        local.tee 30
        local.get 12
        i64.xor
        i64.const 15
        i64.rotl
        local.tee 31
        local.get 14
        local.get 15
        i64.xor
        local.get 13
        i64.xor
        local.get 12
        i64.xor
        local.get 11
        i64.xor
        local.tee 32
        i64.const 1
        i64.rotl
        local.get 24
        local.get 25
        i64.xor
        local.get 23
        i64.xor
        local.get 22
        i64.xor
        local.get 21
        i64.xor
        local.tee 33
        i64.xor
        local.tee 12
        local.get 18
        i64.xor
        i64.const 10
        i64.rotl
        local.tee 18
        i64.const -1
        i64.xor
        i64.and
        local.get 4
        local.get 5
        i64.xor
        local.get 3
        i64.xor
        local.get 2
        i64.xor
        local.get 1
        i64.xor
        local.tee 34
        local.get 29
        i64.const 1
        i64.rotl
        i64.xor
        local.tee 29
        local.get 24
        i64.xor
        i64.const 36
        i64.rotl
        local.tee 35
        i64.xor
        local.tee 36
        local.get 28
        local.get 33
        i64.const 1
        i64.rotl
        i64.xor
        local.tee 24
        local.get 4
        i64.xor
        i64.const 20
        i64.rotl
        local.tee 4
        local.get 12
        local.get 17
        i64.xor
        i64.const 45
        i64.rotl
        local.tee 17
        local.get 29
        local.get 23
        i64.xor
        i64.const 3
        i64.rotl
        local.tee 28
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.tee 33
        i64.xor
        local.get 34
        i64.const 1
        i64.rotl
        local.get 32
        i64.xor
        local.tee 23
        local.get 9
        i64.xor
        i64.const 55
        i64.rotl
        local.tee 9
        local.get 29
        local.get 22
        i64.xor
        i64.const 41
        i64.rotl
        local.tee 22
        local.get 24
        local.get 3
        i64.xor
        i64.const 39
        i64.rotl
        local.tee 3
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.tee 32
        i64.xor
        local.get 23
        local.get 7
        i64.xor
        i64.const 21
        i64.rotl
        local.tee 7
        local.get 30
        local.get 13
        i64.xor
        i64.const 43
        i64.rotl
        local.tee 13
        i64.const -1
        i64.xor
        i64.and
        local.get 12
        local.get 19
        i64.xor
        i64.const 44
        i64.rotl
        local.tee 19
        i64.xor
        local.tee 34
        i64.xor
        local.get 24
        local.get 2
        i64.xor
        i64.const 8
        i64.rotl
        local.tee 2
        local.get 23
        local.get 8
        i64.xor
        i64.const 25
        i64.rotl
        local.tee 8
        i64.const -1
        i64.xor
        i64.and
        local.get 30
        local.get 14
        i64.xor
        i64.const 6
        i64.rotl
        local.tee 14
        i64.xor
        local.tee 37
        i64.xor
        local.tee 38
        i64.const 1
        i64.rotl
        local.get 19
        local.get 29
        local.get 25
        i64.xor
        local.tee 25
        i64.const -1
        i64.xor
        i64.and
        local.get 24
        local.get 1
        i64.xor
        i64.const 14
        i64.rotl
        local.tee 1
        i64.xor
        local.tee 39
        local.get 14
        local.get 12
        local.get 20
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 20
        i64.const -1
        i64.xor
        i64.and
        local.get 29
        local.get 21
        i64.xor
        i64.const 18
        i64.rotl
        local.tee 29
        i64.xor
        local.tee 21
        i64.xor
        local.get 23
        local.get 6
        i64.xor
        i64.const 56
        i64.rotl
        local.tee 6
        local.get 35
        local.get 24
        local.get 5
        i64.xor
        i64.const 27
        i64.rotl
        local.tee 24
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.tee 40
        i64.xor
        local.get 4
        local.get 23
        local.get 10
        i64.xor
        i64.const 28
        i64.rotl
        local.tee 23
        i64.const -1
        i64.xor
        i64.and
        local.get 30
        local.get 11
        i64.xor
        i64.const 61
        i64.rotl
        local.tee 5
        i64.xor
        local.tee 41
        i64.xor
        local.get 9
        local.get 30
        local.get 15
        i64.xor
        i64.const 62
        i64.rotl
        local.tee 10
        i64.const -1
        i64.xor
        i64.and
        local.get 12
        local.get 16
        i64.xor
        i64.const 2
        i64.rotl
        local.tee 11
        i64.xor
        local.tee 42
        i64.xor
        local.tee 15
        i64.xor
        local.tee 30
        local.get 8
        local.get 14
        i64.const -1
        i64.xor
        i64.and
        local.get 20
        i64.xor
        local.tee 12
        i64.xor
        i64.const 3
        i64.rotl
        local.tee 14
        local.get 13
        local.get 19
        i64.const -1
        i64.xor
        i64.and
        local.get 27
        i64.load
        i64.xor
        local.get 25
        i64.xor
        local.tee 16
        local.get 24
        local.get 18
        local.get 35
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.tee 35
        i64.xor
        local.get 23
        local.get 28
        local.get 4
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.tee 19
        i64.xor
        local.get 12
        i64.xor
        local.get 3
        local.get 9
        i64.const -1
        i64.xor
        i64.and
        local.get 10
        i64.xor
        local.tee 43
        i64.xor
        local.tee 9
        i64.const 1
        i64.rotl
        local.get 2
        local.get 20
        local.get 29
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.tee 20
        local.get 10
        local.get 11
        i64.const -1
        i64.xor
        i64.and
        local.get 22
        i64.xor
        local.tee 44
        i64.xor
        local.get 7
        local.get 25
        local.get 1
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.tee 10
        i64.xor
        local.get 23
        local.get 5
        i64.const -1
        i64.xor
        i64.and
        local.get 17
        i64.xor
        local.tee 25
        i64.xor
        local.get 24
        local.get 6
        i64.const -1
        i64.xor
        i64.and
        local.get 31
        i64.xor
        local.tee 45
        i64.xor
        local.tee 24
        i64.xor
        local.tee 12
        local.get 41
        i64.xor
        i64.const 20
        i64.rotl
        local.tee 4
        i64.const -1
        i64.xor
        i64.and
        local.get 15
        i64.const 1
        i64.rotl
        local.get 3
        local.get 11
        local.get 22
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.tee 22
        local.get 5
        local.get 17
        i64.const -1
        i64.xor
        i64.and
        local.get 28
        i64.xor
        local.tee 23
        i64.xor
        local.get 8
        local.get 29
        local.get 2
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.tee 15
        i64.xor
        local.get 1
        local.get 7
        i64.const -1
        i64.xor
        i64.and
        local.get 13
        i64.xor
        local.tee 11
        i64.xor
        local.get 6
        local.get 31
        i64.const -1
        i64.xor
        i64.and
        local.get 18
        i64.xor
        local.tee 31
        i64.xor
        local.tee 5
        i64.xor
        local.tee 29
        local.get 10
        i64.xor
        i64.const 28
        i64.rotl
        local.tee 1
        i64.xor
        local.tee 41
        local.get 29
        local.get 20
        i64.xor
        i64.const 25
        i64.rotl
        local.tee 2
        local.get 24
        i64.const 1
        i64.rotl
        local.get 38
        i64.xor
        local.tee 24
        local.get 23
        i64.xor
        i64.const 6
        i64.rotl
        local.tee 3
        i64.const -1
        i64.xor
        i64.and
        local.get 5
        i64.const 1
        i64.rotl
        local.get 9
        i64.xor
        local.tee 23
        local.get 34
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 5
        i64.xor
        local.tee 46
        i64.xor
        local.get 23
        local.get 37
        i64.xor
        i64.const 10
        i64.rotl
        local.tee 6
        local.get 30
        local.get 19
        i64.xor
        i64.const 36
        i64.rotl
        local.tee 7
        i64.const -1
        i64.xor
        i64.and
        local.get 12
        local.get 39
        i64.xor
        i64.const 27
        i64.rotl
        local.tee 8
        i64.xor
        local.tee 28
        i64.xor
        local.get 12
        local.get 21
        i64.xor
        i64.const 39
        i64.rotl
        local.tee 9
        local.get 29
        local.get 25
        i64.xor
        i64.const 55
        i64.rotl
        local.tee 10
        i64.const -1
        i64.xor
        i64.and
        local.get 24
        local.get 11
        i64.xor
        i64.const 62
        i64.rotl
        local.tee 11
        i64.xor
        local.tee 25
        i64.xor
        local.get 24
        local.get 15
        i64.xor
        i64.const 43
        i64.rotl
        local.tee 13
        local.get 23
        local.get 33
        i64.xor
        i64.const 44
        i64.rotl
        local.tee 15
        i64.const -1
        i64.xor
        i64.and
        local.get 27
        i32.const 8
        i32.add
        i64.load
        i64.xor
        local.get 30
        local.get 16
        i64.xor
        local.tee 16
        i64.xor
        local.tee 33
        i64.xor
        local.tee 34
        i64.const 1
        i64.rotl
        local.get 16
        local.get 12
        local.get 42
        i64.xor
        i64.const 14
        i64.rotl
        local.tee 17
        i64.const -1
        i64.xor
        i64.and
        local.get 29
        local.get 45
        i64.xor
        i64.const 21
        i64.rotl
        local.tee 18
        i64.xor
        local.tee 42
        local.get 1
        local.get 24
        local.get 22
        i64.xor
        i64.const 61
        i64.rotl
        local.tee 19
        i64.const -1
        i64.xor
        i64.and
        local.get 23
        local.get 36
        i64.xor
        i64.const 45
        i64.rotl
        local.tee 20
        i64.xor
        local.tee 36
        i64.xor
        local.get 5
        local.get 30
        local.get 43
        i64.xor
        i64.const 18
        i64.rotl
        local.tee 21
        i64.const -1
        i64.xor
        i64.and
        local.get 12
        local.get 40
        i64.xor
        i64.const 8
        i64.rotl
        local.tee 22
        i64.xor
        local.tee 40
        i64.xor
        local.get 8
        local.get 29
        local.get 44
        i64.xor
        i64.const 56
        i64.rotl
        local.tee 29
        i64.const -1
        i64.xor
        i64.and
        local.get 24
        local.get 31
        i64.xor
        i64.const 15
        i64.rotl
        local.tee 24
        i64.xor
        local.tee 44
        i64.xor
        local.get 11
        local.get 23
        local.get 32
        i64.xor
        i64.const 2
        i64.rotl
        local.tee 12
        i64.const -1
        i64.xor
        i64.and
        local.get 30
        local.get 35
        i64.xor
        i64.const 41
        i64.rotl
        local.tee 23
        i64.xor
        local.tee 32
        i64.xor
        local.tee 35
        i64.xor
        local.tee 30
        local.get 10
        local.get 11
        i64.const -1
        i64.xor
        i64.and
        local.get 12
        i64.xor
        local.tee 11
        i64.xor
        i64.const 14
        i64.rotl
        local.tee 31
        local.get 34
        local.get 17
        local.get 18
        i64.const -1
        i64.xor
        i64.and
        local.get 13
        i64.xor
        local.tee 37
        local.get 19
        local.get 20
        i64.const -1
        i64.xor
        i64.and
        local.get 14
        i64.xor
        local.tee 38
        i64.xor
        local.get 21
        local.get 22
        i64.const -1
        i64.xor
        i64.and
        local.get 2
        i64.xor
        local.tee 45
        i64.xor
        local.get 29
        local.get 24
        i64.const -1
        i64.xor
        i64.and
        local.get 6
        i64.xor
        local.tee 39
        i64.xor
        local.get 12
        local.get 23
        i64.const -1
        i64.xor
        i64.and
        local.get 9
        i64.xor
        local.tee 43
        i64.xor
        local.tee 47
        i64.const 1
        i64.rotl
        i64.xor
        local.tee 12
        local.get 20
        local.get 14
        i64.const -1
        i64.xor
        i64.and
        local.get 4
        i64.xor
        local.tee 14
        i64.xor
        i64.const 44
        i64.rotl
        local.tee 20
        local.get 18
        local.get 13
        i64.const -1
        i64.xor
        i64.and
        local.get 15
        i64.xor
        local.tee 18
        local.get 14
        i64.xor
        local.get 22
        local.get 2
        i64.const -1
        i64.xor
        i64.and
        local.get 3
        i64.xor
        local.tee 14
        i64.xor
        local.get 24
        local.get 6
        i64.const -1
        i64.xor
        i64.and
        local.get 7
        i64.xor
        local.tee 48
        i64.xor
        local.get 23
        local.get 9
        i64.const -1
        i64.xor
        i64.and
        local.get 10
        i64.xor
        local.tee 23
        i64.xor
        local.tee 24
        i64.const 1
        i64.rotl
        local.get 15
        local.get 16
        i64.const -1
        i64.xor
        i64.and
        local.get 17
        i64.xor
        local.tee 15
        local.get 4
        local.get 1
        i64.const -1
        i64.xor
        i64.and
        local.get 19
        i64.xor
        local.tee 4
        i64.xor
        local.get 3
        local.get 5
        i64.const -1
        i64.xor
        i64.and
        local.get 21
        i64.xor
        local.tee 3
        i64.xor
        local.get 7
        local.get 8
        i64.const -1
        i64.xor
        i64.and
        local.get 29
        i64.xor
        local.tee 8
        i64.xor
        local.get 11
        i64.xor
        local.tee 7
        i64.xor
        local.tee 29
        local.get 33
        i64.xor
        local.tee 10
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.set 5
        local.get 35
        i64.const 1
        i64.rotl
        local.get 24
        i64.xor
        local.tee 24
        local.get 37
        i64.xor
        i64.const 62
        i64.rotl
        local.tee 1
        local.get 12
        local.get 23
        i64.xor
        i64.const 2
        i64.rotl
        local.tee 2
        i64.const -1
        i64.xor
        i64.and
        local.get 29
        local.get 28
        i64.xor
        i64.const 41
        i64.rotl
        local.tee 9
        i64.xor
        local.set 6
        local.get 30
        local.get 3
        i64.xor
        i64.const 39
        i64.rotl
        local.tee 13
        local.get 2
        local.get 9
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.set 11
        local.get 13
        local.get 7
        i64.const 1
        i64.rotl
        local.get 47
        i64.xor
        local.tee 23
        local.get 36
        i64.xor
        i64.const 55
        i64.rotl
        local.tee 16
        i64.const -1
        i64.xor
        i64.and
        local.get 1
        i64.xor
        local.set 21
        local.get 30
        local.get 15
        i64.xor
        i64.const 27
        i64.rotl
        local.tee 35
        local.get 23
        local.get 32
        i64.xor
        i64.const 56
        i64.rotl
        local.tee 28
        i64.const -1
        i64.xor
        i64.and
        local.get 24
        local.get 39
        i64.xor
        i64.const 15
        i64.rotl
        local.tee 32
        i64.xor
        local.set 7
        local.get 32
        local.get 12
        local.get 14
        i64.xor
        i64.const 10
        i64.rotl
        local.tee 33
        i64.const -1
        i64.xor
        i64.and
        local.get 29
        local.get 41
        i64.xor
        i64.const 36
        i64.rotl
        local.tee 34
        i64.xor
        local.set 17
        local.get 35
        local.get 33
        local.get 34
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.set 22
        local.get 24
        local.get 38
        i64.xor
        i64.const 6
        i64.rotl
        local.tee 36
        local.get 12
        local.get 18
        i64.xor
        i64.const 1
        i64.rotl
        local.tee 37
        i64.const -1
        i64.xor
        i64.and
        local.get 29
        local.get 25
        i64.xor
        i64.const 18
        i64.rotl
        local.tee 38
        i64.xor
        local.set 3
        local.get 30
        local.get 8
        i64.xor
        i64.const 8
        i64.rotl
        local.tee 39
        local.get 37
        local.get 38
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.set 8
        local.get 39
        local.get 23
        local.get 40
        i64.xor
        i64.const 25
        i64.rotl
        local.tee 40
        i64.const -1
        i64.xor
        i64.and
        local.get 36
        i64.xor
        local.set 18
        local.get 30
        local.get 4
        i64.xor
        i64.const 20
        i64.rotl
        local.tee 30
        local.get 23
        local.get 42
        i64.xor
        i64.const 28
        i64.rotl
        local.tee 41
        i64.const -1
        i64.xor
        i64.and
        local.get 24
        local.get 43
        i64.xor
        i64.const 61
        i64.rotl
        local.tee 42
        i64.xor
        local.set 4
        local.get 42
        local.get 12
        local.get 48
        i64.xor
        i64.const 45
        i64.rotl
        local.tee 43
        i64.const -1
        i64.xor
        i64.and
        local.get 29
        local.get 46
        i64.xor
        i64.const 3
        i64.rotl
        local.tee 29
        i64.xor
        local.set 14
        local.get 30
        local.get 43
        local.get 29
        i64.const -1
        i64.xor
        i64.and
        i64.xor
        local.set 19
        local.get 31
        local.get 23
        local.get 44
        i64.xor
        i64.const 21
        i64.rotl
        local.tee 44
        i64.const -1
        i64.xor
        i64.and
        local.get 24
        local.get 45
        i64.xor
        i64.const 43
        i64.rotl
        local.tee 45
        i64.xor
        local.set 15
        local.get 45
        local.get 20
        i64.const -1
        i64.xor
        i64.and
        local.get 27
        i32.const 16
        i32.add
        i64.load
        i64.xor
        local.get 10
        i64.xor
        local.set 25
        local.get 16
        local.get 1
        i64.const -1
        i64.xor
        i64.and
        local.get 2
        i64.xor
        local.set 1
        local.get 9
        local.get 13
        i64.const -1
        i64.xor
        i64.and
        local.get 16
        i64.xor
        local.set 16
        local.get 34
        local.get 35
        i64.const -1
        i64.xor
        i64.and
        local.get 28
        i64.xor
        local.set 2
        local.get 28
        local.get 32
        i64.const -1
        i64.xor
        i64.and
        local.get 33
        i64.xor
        local.set 12
        local.get 38
        local.get 39
        i64.const -1
        i64.xor
        i64.and
        local.get 40
        i64.xor
        local.set 13
        local.get 40
        local.get 36
        i64.const -1
        i64.xor
        i64.and
        local.get 37
        i64.xor
        local.set 23
        local.get 41
        local.get 42
        i64.const -1
        i64.xor
        i64.and
        local.get 43
        i64.xor
        local.set 9
        local.get 29
        local.get 30
        i64.const -1
        i64.xor
        i64.and
        local.get 41
        i64.xor
        local.set 24
        local.get 10
        local.get 31
        i64.const -1
        i64.xor
        i64.and
        local.get 44
        i64.xor
        local.set 10
        local.get 44
        local.get 45
        i64.const -1
        i64.xor
        i64.and
        local.get 20
        i64.xor
        local.set 20
        local.get 27
        i32.const 24
        i32.add
        local.set 27
        local.get 26
        i32.const 3
        i32.add
        local.set 26
        br 0 (;@2;)
      end
    end
    local.get 0
    local.get 21
    i64.store offset=160
    local.get 0
    local.get 22
    i64.store offset=120
    local.get 0
    local.get 23
    i64.store offset=80
    local.get 0
    local.get 24
    i64.store offset=40
    local.get 0
    local.get 25
    i64.store
    local.get 0
    local.get 16
    i64.store offset=168
    local.get 0
    local.get 17
    i64.store offset=128
    local.get 0
    local.get 18
    i64.store offset=88
    local.get 0
    local.get 19
    i64.store offset=48
    local.get 0
    local.get 20
    i64.store offset=8
    local.get 0
    local.get 11
    i64.store offset=176
    local.get 0
    local.get 12
    i64.store offset=136
    local.get 0
    local.get 13
    i64.store offset=96
    local.get 0
    local.get 14
    i64.store offset=56
    local.get 0
    local.get 15
    i64.store offset=16
    local.get 0
    local.get 6
    i64.store offset=184
    local.get 0
    local.get 7
    i64.store offset=144
    local.get 0
    local.get 8
    i64.store offset=104
    local.get 0
    local.get 9
    i64.store offset=64
    local.get 0
    local.get 10
    i64.store offset=24
    local.get 0
    local.get 1
    i64.store offset=192
    local.get 0
    local.get 2
    i64.store offset=152
    local.get 0
    local.get 3
    i64.store offset=112
    local.get 0
    local.get 4
    i64.store offset=72
    local.get 0
    local.get 5
    i64.store offset=32)
  (func (;5;) (type 0) (param i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32)
    global.get 0
    i32.const 48
    i32.sub
    local.tee 3
    global.set 0
    i32.const -1
    local.set 4
    block  ;; label = @1
      local.get 1
      i32.const 62
      i32.ne
      br_if 0 (;@1;)
      local.get 0
      i32.load8_u offset=56
      i32.const 46
      i32.ne
      br_if 0 (;@1;)
      local.get 0
      i32.load8_u offset=57
      i32.const 111
      i32.ne
      br_if 0 (;@1;)
      local.get 0
      i32.load8_u offset=58
      i32.const 110
      i32.ne
      br_if 0 (;@1;)
      local.get 0
      i32.load8_u offset=59
      i32.const 105
      i32.ne
      br_if 0 (;@1;)
      local.get 0
      i32.load8_u offset=60
      i32.const 111
      i32.ne
      br_if 0 (;@1;)
      local.get 0
      i32.load8_u offset=61
      i32.const 110
      i32.ne
      br_if 0 (;@1;)
      i32.const 0
      local.set 5
      local.get 3
      i32.const 0
      i32.store8 offset=47
      i32.const 0
      local.set 6
      i32.const 0
      local.set 1
      loop  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 1
            i32.const 56
            i32.eq
            br_if 0 (;@4;)
            block  ;; label = @5
              local.get 0
              local.get 1
              i32.add
              i32.load8_u
              local.tee 7
              i32.const -97
              i32.add
              local.tee 8
              i32.const 255
              i32.and
              i32.const 26
              i32.lt_u
              br_if 0 (;@5;)
              local.get 7
              i32.const -65
              i32.add
              local.tee 8
              i32.const 255
              i32.and
              i32.const 26
              i32.lt_u
              br_if 0 (;@5;)
              local.get 7
              i32.const -50
              i32.add
              i32.const 255
              i32.and
              i32.const 5
              i32.gt_u
              br_if 4 (;@1;)
              local.get 7
              i32.const -24
              i32.add
              local.set 8
            end
            local.get 3
            local.get 3
            i32.load8_u offset=47
            i32.const 5
            i32.add
            i32.const 31
            i32.and
            local.tee 7
            i32.store8 offset=47
            local.get 5
            i32.const 5
            i32.shl
            local.get 8
            i32.const 255
            i32.and
            i32.or
            local.set 5
            local.get 7
            i32.const 7
            i32.le_u
            br_if 1 (;@3;)
            local.get 3
            local.get 7
            i32.const 24
            i32.add
            i32.const 31
            i32.and
            i32.store8 offset=47
            local.get 6
            i32.const 34
            i32.gt_u
            br_if 3 (;@1;)
            local.get 3
            i32.const 10
            i32.add
            local.get 6
            i32.add
            local.get 5
            local.get 3
            i32.load8_u offset=47
            i32.shr_u
            i32.store8
            local.get 6
            i32.const 1
            i32.add
            local.set 6
            br 1 (;@3;)
          end
          local.get 6
          i32.const 35
          i32.ne
          br_if 2 (;@1;)
          local.get 3
          i32.load8_u offset=47
          i32.const 31
          i32.and
          br_if 2 (;@1;)
          local.get 3
          i32.load8_u offset=44
          i32.const 255
          i32.and
          i32.const 3
          i32.ne
          br_if 2 (;@1;)
          local.get 3
          i32.const 10
          i32.add
          local.get 3
          i32.const 45
          i32.add
          call 6
          local.get 3
          i32.load8_u offset=46
          local.get 3
          i32.load8_u offset=43
          i32.ne
          br_if 2 (;@1;)
          local.get 3
          i32.load8_u offset=45
          i32.const 255
          i32.and
          local.get 3
          i32.load8_u offset=42
          i32.const 255
          i32.and
          i32.ne
          br_if 2 (;@1;)
          block  ;; label = @4
            i32.const 236
            local.get 3
            i32.load8_u offset=10
            i32.sub
            local.get 3
            i32.load8_u offset=41
            i32.const -1
            i32.xor
            i32.const 127
            i32.and
            local.get 3
            i32.load8_u offset=12
            local.get 3
            i32.load8_u offset=11
            i32.and
            local.get 3
            i32.load8_u offset=13
            i32.and
            local.get 3
            i32.load8_u offset=14
            i32.and
            local.get 3
            i32.load8_u offset=15
            i32.and
            local.get 3
            i32.load8_u offset=16
            i32.and
            local.get 3
            i32.load8_u offset=17
            i32.and
            local.get 3
            i32.load8_u offset=18
            i32.and
            local.get 3
            i32.load8_u offset=19
            i32.and
            local.get 3
            i32.load8_u offset=20
            i32.and
            local.get 3
            i32.load8_u offset=21
            i32.and
            local.get 3
            i32.load8_u offset=22
            i32.and
            local.get 3
            i32.load8_u offset=23
            i32.and
            local.get 3
            i32.load8_u offset=24
            i32.and
            local.get 3
            i32.load8_u offset=25
            i32.and
            local.get 3
            i32.load8_u offset=26
            i32.and
            local.get 3
            i32.load8_u offset=27
            i32.and
            local.get 3
            i32.load8_u offset=28
            i32.and
            local.get 3
            i32.load8_u offset=29
            i32.and
            local.get 3
            i32.load8_u offset=30
            i32.and
            local.get 3
            i32.load8_u offset=31
            i32.and
            local.get 3
            i32.load8_u offset=32
            i32.and
            local.get 3
            i32.load8_u offset=33
            i32.and
            local.get 3
            i32.load8_u offset=34
            i32.and
            local.get 3
            i32.load8_u offset=35
            i32.and
            local.get 3
            i32.load8_u offset=36
            i32.and
            local.get 3
            i32.load8_u offset=37
            i32.and
            local.get 3
            i32.load8_u offset=38
            i32.and
            local.get 3
            i32.load8_u offset=39
            i32.and
            local.get 3
            i32.load8_u offset=40
            i32.and
            i32.const 255
            i32.xor
            i32.or
            i32.const -1
            i32.add
            i32.and
            i32.const 256
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            i32.const -3
            local.set 4
            br 3 (;@1;)
          end
          local.get 2
          local.get 3
          i64.load offset=10 align=1
          i64.store align=1
          local.get 2
          i32.const 24
          i32.add
          local.get 3
          i32.const 10
          i32.add
          i32.const 24
          i32.add
          i64.load align=1
          i64.store align=1
          local.get 2
          i32.const 16
          i32.add
          local.get 3
          i32.const 10
          i32.add
          i32.const 16
          i32.add
          i64.load align=1
          i64.store align=1
          local.get 2
          i32.const 8
          i32.add
          local.get 3
          i32.const 10
          i32.add
          i32.const 8
          i32.add
          i64.load align=1
          i64.store align=1
          i32.const 0
          local.set 4
          br 2 (;@1;)
        end
        local.get 1
        i32.const 1
        i32.add
        local.set 1
        br 0 (;@2;)
      end
    end
    local.get 3
    i32.const 48
    i32.add
    global.set 0
    local.get 4)
  (func (;6;) (type 4) (param i32 i32)
    (local i32)
    global.get 0
    i32.const 80
    i32.sub
    local.tee 2
    global.set 0
    local.get 2
    i32.const 23
    i32.add
    local.get 0
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 2
    i32.const 31
    i32.add
    local.get 0
    i32.const 16
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 2
    i32.const 39
    i32.add
    local.get 0
    i32.const 24
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 2
    i32.const 3
    i32.store8 offset=47
    local.get 2
    i32.const 0
    i64.load offset=1048590 align=1
    i64.store offset=7 align=1
    local.get 2
    i32.const 0
    i64.load offset=1048583 align=1
    i64.store
    local.get 2
    local.get 0
    i64.load align=1
    i64.store offset=15 align=1
    local.get 2
    i32.const 48
    local.get 2
    i32.const 48
    i32.add
    call 2
    local.get 1
    local.get 2
    i32.load16_u offset=48 align=1
    i32.store16 align=1
    local.get 2
    i32.const 80
    i32.add
    global.set 0)
  (func (;7;) (type 1) (param i32 i32) (result i32)
    (local i32 i32 i32 i32)
    global.get 0
    i32.const 48
    i32.sub
    local.tee 2
    global.set 0
    local.get 2
    i32.const 8
    i32.add
    i32.const 24
    i32.add
    local.get 0
    i32.const 24
    i32.add
    i64.load align=1
    i64.store
    local.get 2
    i32.const 8
    i32.add
    i32.const 16
    i32.add
    local.get 0
    i32.const 16
    i32.add
    i64.load align=1
    i64.store
    local.get 2
    i32.const 8
    i32.add
    i32.const 8
    i32.add
    local.get 0
    i32.const 8
    i32.add
    i64.load align=1
    i64.store
    local.get 2
    local.get 0
    i64.load align=1
    i64.store offset=8
    local.get 0
    local.get 2
    i32.const 40
    i32.add
    call 6
    local.get 2
    i32.const 3
    i32.store8 offset=42
    i32.const 0
    local.set 3
    local.get 2
    i32.const 0
    i32.store8 offset=47
    i32.const 0
    local.set 4
    i32.const 0
    local.set 5
    block  ;; label = @1
      loop  ;; label = @2
        local.get 5
        i32.const 35
        i32.eq
        br_if 1 (;@1;)
        local.get 2
        local.get 2
        i32.load8_u offset=47
        i32.const 8
        i32.add
        i32.const 31
        i32.and
        local.tee 0
        i32.store8 offset=47
        local.get 3
        i32.const 8
        i32.shl
        local.get 2
        i32.const 8
        i32.add
        local.get 5
        i32.add
        i32.load8_u
        i32.or
        local.set 3
        block  ;; label = @3
          loop  ;; label = @4
            local.get 0
            i32.const 31
            i32.and
            i32.const 5
            i32.lt_u
            br_if 1 (;@3;)
            local.get 1
            local.get 4
            i32.add
            local.get 3
            local.get 0
            i32.const 27
            i32.add
            local.tee 0
            i32.shr_u
            i32.const 31
            i32.and
            i32.load8_u offset=1048613
            i32.store8
            local.get 2
            local.get 0
            i32.const 31
            i32.and
            local.tee 0
            i32.store8 offset=47
            local.get 4
            i32.const 1
            i32.add
            local.set 4
            br 0 (;@4;)
          end
        end
        local.get 5
        i32.const 1
        i32.add
        local.set 5
        br 0 (;@2;)
      end
    end
    local.get 1
    i32.const 60
    i32.add
    i32.const 0
    i32.load16_u offset=1048580 align=1
    i32.store16 align=1
    local.get 1
    i32.const 0
    i32.load offset=1048576 align=1
    i32.store offset=56 align=1
    local.get 2
    i32.const 48
    i32.add
    global.set 0
    i32.const 0)
  (func (;8;) (type 5) (result i32)
    i32.const 35)
  (func (;9;) (type 5) (result i32)
    i32.const 62)
  (func (;10;) (type 5) (result i32)
    i32.const 1)
  (func (;11;) (type 5) (result i32)
    i32.const 300211)
  (table (;0;) 1 1 funcref)
  (memory (;0;) 17)
  (global (;0;) (mut i32) (i32.const 1048576))
  (export "memory" (memory 0))
  (export "tor_hs_identity_subcredential" (func 0))
  (export "tor_hs_identity_credential" (func 1))
  (export "tor_hs_identity_validate_onion" (func 5))
  (export "tor_hs_identity_build_onion" (func 7))
  (export "tor_hs_identity_raw_len" (func 8))
  (export "tor_hs_identity_onion_len" (func 9))
  (export "proto_abi_version" (func 10))
  (export "proto_standard_id" (func 11))
  (data (;0;) (i32.const 1048576) ".onion\00.onion checksum\00subcredential\00abcdefghijklmnopqrstuvwxyz234567\00\00\00\01\00\00\00\00\00\00\00\82\80\00\00\00\00\00\00\8a\80\00\00\00\00\00\80\00\80\00\80\00\00\00\80\8b\80\00\00\00\00\00\00\01\00\00\80\00\00\00\00\81\80\00\80\00\00\00\80\09\80\00\00\00\00\00\80\8a\00\00\00\00\00\00\00\88\00\00\00\00\00\00\00\09\80\00\80\00\00\00\00\0a\00\00\80\00\00\00\00\8b\80\00\80\00\00\00\00\8b\00\00\00\00\00\00\80\89\80\00\00\00\00\00\80\03\80\00\00\00\00\00\80\02\80\00\00\00\00\00\80\80\00\00\00\00\00\00\80\0a\80\00\00\00\00\00\00\0a\00\00\80\00\00\00\80\81\80\00\80\00\00\00\80\80\80\00\00\00\00\00\80\01\00\00\80\00\00\00\00\08\80\00\80\00\00\00\80"))
