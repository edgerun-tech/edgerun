(module
  (type (;0;) (func (param i32 i32 i32) (result i32)))
  (type (;1;) (func (param i32 i32 i32 i32) (result i32)))
  (type (;2;) (func (param i32 i32) (result i32)))
  (type (;3;) (func (param i32) (result i32)))
  (type (;4;) (func (param i32 i32 i32 i32 i32) (result i32)))
  (type (;5;) (func (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (type (;6;) (func (param i32 i32 i32 i32 i32 i32) (result i32)))
  (type (;7;) (func (result i32)))
  (func (;0;) (type 0) (param i32 i32 i32) (result i32)
    (local i32)
    i32.const -2
    local.set 3
    block  ;; label = @1
      local.get 1
      i32.const 22
      i32.le_u
      br_if 0 (;@1;)
      i32.const -4
      local.set 3
      local.get 0
      i32.load8_u
      i32.const 1
      i32.ne
      br_if 0 (;@1;)
      i32.const -2
      local.set 3
      local.get 0
      i32.load16_u offset=1 align=1
      local.tee 1
      i32.const 8
      i32.shl
      local.get 1
      i32.const 8
      i32.shr_u
      i32.or
      i32.const 65535
      i32.and
      i32.const 20
      i32.lt_u
      br_if 0 (;@1;)
      local.get 0
      i32.const 3
      i32.add
      local.set 0
      i32.const 0
      local.set 1
      i32.const 0
      local.set 3
      block  ;; label = @2
        loop  ;; label = @3
          local.get 3
          i32.const 20
          i32.eq
          br_if 1 (;@2;)
          local.get 2
          local.get 3
          i32.add
          i32.load8_u
          local.get 0
          local.get 3
          i32.add
          i32.load8_u
          i32.xor
          local.get 1
          i32.or
          local.set 1
          local.get 3
          i32.const 1
          i32.add
          local.set 3
          br 0 (;@3;)
        end
      end
      i32.const -3
      i32.const 0
      local.get 1
      i32.const 255
      i32.and
      select
      local.set 3
    end
    local.get 3)
  (func (;1;) (type 1) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        i32.const 2
        i32.le_u
        br_if 0 (;@2;)
        i32.const -4
        local.set 4
        local.get 2
        local.get 0
        i32.load8_u
        local.tee 5
        i32.gt_u
        br_if 1 (;@1;)
        local.get 1
        local.get 0
        i32.load16_u offset=1 align=1
        local.tee 0
        i32.const 8
        i32.shl
        local.get 0
        i32.const 8
        i32.shr_u
        i32.or
        local.tee 6
        i32.const 65535
        i32.and
        local.tee 0
        i32.const 3
        i32.add
        local.tee 2
        i32.lt_u
        br_if 0 (;@2;)
        block  ;; label = @3
          block  ;; label = @4
            local.get 5
            br_table 1 (;@3;) 0 (;@4;) 3 (;@1;)
          end
          i32.const -2
          local.set 4
          local.get 6
          i32.const 65535
          i32.and
          i32.const 20
          i32.lt_u
          br_if 2 (;@1;)
        end
        local.get 3
        local.get 2
        i32.store offset=12
        local.get 3
        local.get 0
        i32.store offset=8
        local.get 3
        i32.const 3
        i32.store offset=4
        local.get 3
        local.get 5
        i32.store
        i32.const 0
        return
      end
      i32.const -2
      local.set 4
    end
    local.get 4)
  (func (;2;) (type 2) (param i32 i32) (result i32)
    local.get 0
    i32.const 20
    i32.store8 offset=2
    local.get 0
    i32.const 1
    i32.store16 align=1
    local.get 0
    local.get 1
    i64.load align=1
    i64.store offset=3 align=1
    local.get 0
    i32.const 11
    i32.add
    local.get 1
    i32.const 8
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 0
    i32.const 19
    i32.add
    local.get 1
    i32.const 16
    i32.add
    i32.load align=1
    i32.store align=1
    i32.const 23)
  (func (;3;) (type 3) (param i32) (result i32)
    local.get 0
    i32.const 0
    i32.store8 offset=2
    local.get 0
    i32.const 0
    i32.store16 align=1
    i32.const 3)
  (func (;4;) (type 1) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32)
    i32.const -2
    local.set 4
    block  ;; label = @1
      local.get 1
      i32.const 6
      i32.lt_u
      br_if 0 (;@1;)
      i32.const -4
      local.set 4
      local.get 0
      i32.load8_u
      br_if 0 (;@1;)
      local.get 0
      i32.load8_u offset=1
      local.tee 1
      i32.const -1
      i32.add
      i32.const 255
      i32.and
      i32.const 1
      i32.gt_u
      br_if 0 (;@1;)
      local.get 3
      local.get 1
      i32.store offset=4
      i32.const 0
      local.set 4
      local.get 3
      i32.const 0
      i32.store
      i32.const 0
      local.set 5
      i32.const 0
      local.set 6
      block  ;; label = @2
        local.get 1
        i32.const 1
        i32.eq
        br_if 0 (;@2;)
        local.get 2
        local.get 0
        i32.load8_u offset=2
        i32.const 8
        i32.shl
        local.get 0
        i32.load8_u offset=3
        i32.or
        local.tee 1
        local.get 2
        local.get 1
        i32.gt_u
        select
        local.tee 1
        i32.const 65535
        local.get 1
        i32.const 65535
        i32.lt_u
        select
        local.tee 5
        local.get 0
        i32.load8_u offset=4
        i32.const 8
        i32.shl
        local.get 0
        i32.load8_u offset=5
        i32.or
        local.tee 1
        local.get 5
        local.get 1
        i32.gt_u
        select
        local.set 6
      end
      local.get 3
      i32.const 6
      i32.store offset=16
      local.get 3
      local.get 6
      i32.store offset=12
      local.get 3
      local.get 5
      i32.store offset=8
    end
    local.get 4)
  (func (;5;) (type 4) (param i32 i32 i32 i32 i32) (result i32)
    (local i32)
    i32.const -4
    local.set 5
    block  ;; label = @1
      local.get 1
      i32.const -1
      i32.add
      i32.const 2
      i32.ge_u
      br_if 0 (;@1;)
      local.get 0
      local.get 1
      i32.store8 offset=1
      local.get 0
      i32.const 0
      i32.store8
      block  ;; label = @2
        block  ;; label = @3
          local.get 1
          i32.const 1
          i32.ne
          br_if 0 (;@3;)
          local.get 0
          i32.const 0
          i32.store offset=2 align=1
          br 1 (;@2;)
        end
        local.get 0
        local.get 2
        local.get 4
        local.get 2
        local.get 4
        i32.gt_u
        select
        local.tee 1
        i32.const 65535
        local.get 1
        i32.const 65535
        i32.lt_u
        select
        local.tee 1
        i32.store8 offset=3
        local.get 0
        local.get 1
        i32.const 8
        i32.shr_u
        i32.store8 offset=2
        local.get 0
        local.get 3
        local.get 1
        local.get 3
        local.get 1
        i32.gt_u
        select
        local.tee 1
        i32.const 65535
        local.get 1
        i32.const 65535
        i32.lt_u
        select
        local.tee 1
        i32.store8 offset=5
        local.get 0
        local.get 1
        i32.const 8
        i32.shr_u
        i32.store8 offset=4
      end
      i32.const 6
      local.set 5
    end
    local.get 5)
  (func (;6;) (type 2) (param i32 i32) (result i32)
    block  ;; label = @1
      local.get 1
      br_if 0 (;@1;)
      i32.const -2
      return
    end
    local.get 0
    i32.load8_u)
  (func (;7;) (type 2) (param i32 i32) (result i32)
    (local i32)
    i32.const -1
    local.set 2
    block  ;; label = @1
      local.get 1
      i32.const 256
      i32.ge_u
      br_if 0 (;@1;)
      i32.const 1
      local.set 2
      block  ;; label = @2
        i32.const 508
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        i32.const 1
        i32.add
        i32.const 0
        i32.const 508
        memory.fill
      end
      local.get 0
      local.get 1
      i32.store8
    end
    local.get 2)
  (func (;8;) (type 0) (param i32 i32 i32) (result i32)
    (local i32 i32)
    i32.const -2
    local.set 3
    block  ;; label = @1
      local.get 1
      i32.const 1
      i32.le_u
      br_if 0 (;@1;)
      local.get 0
      i32.load16_u align=1
      local.tee 0
      i32.const 8
      i32.shl
      local.get 0
      i32.const 8
      i32.shr_u
      i32.or
      i32.const 65535
      i32.and
      local.tee 0
      i32.const 507
      i32.gt_u
      br_if 0 (;@1;)
      local.get 0
      i32.const 2
      i32.add
      local.tee 4
      local.get 1
      i32.gt_u
      br_if 0 (;@1;)
      local.get 2
      local.get 4
      i32.store offset=8
      local.get 2
      local.get 0
      i32.store offset=4
      local.get 2
      i32.const 2
      i32.store
      i32.const 0
      local.set 3
    end
    local.get 3)
  (func (;9;) (type 0) (param i32 i32 i32) (result i32)
    (local i32)
    i32.const -2
    local.set 3
    block  ;; label = @1
      local.get 2
      i32.const 508
      i32.ge_u
      br_if 0 (;@1;)
      block  ;; label = @2
        i32.const 507
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        i32.const 2
        i32.add
        i32.const 0
        i32.const 507
        memory.fill
      end
      local.get 0
      local.get 2
      i32.const 8
      i32.shl
      local.get 2
      i32.const 65280
      i32.and
      i32.const 8
      i32.shr_u
      i32.or
      i32.store16 align=1
      block  ;; label = @2
        local.get 2
        i32.eqz
        br_if 0 (;@2;)
        local.get 2
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        i32.const 2
        i32.add
        local.get 1
        local.get 2
        memory.copy
      end
      local.get 2
      i32.const 2
      i32.add
      local.set 3
    end
    local.get 3)
  (func (;10;) (type 0) (param i32 i32 i32) (result i32)
    (local i32 i32 i32)
    i32.const -2
    local.set 3
    block  ;; label = @1
      local.get 1
      i32.const 3
      i32.le_u
      br_if 0 (;@1;)
      local.get 0
      i32.load16_u offset=2 align=1
      local.tee 4
      i32.const 8
      i32.shl
      local.get 4
      i32.const 8
      i32.shr_u
      i32.or
      i32.const 65535
      i32.and
      local.tee 4
      i32.const 505
      i32.gt_u
      br_if 0 (;@1;)
      local.get 4
      i32.const 4
      i32.add
      local.tee 5
      local.get 1
      i32.gt_u
      br_if 0 (;@1;)
      local.get 0
      i32.load8_u offset=1
      local.set 1
      local.get 0
      i32.load8_u
      local.set 3
      local.get 2
      local.get 5
      i32.store offset=12
      local.get 2
      local.get 4
      i32.store offset=8
      local.get 2
      i32.const 4
      i32.store offset=4
      local.get 2
      local.get 1
      local.get 3
      i32.const 8
      i32.shl
      i32.or
      i32.store
      i32.const 0
      local.set 3
    end
    local.get 3)
  (func (;11;) (type 1) (param i32 i32 i32 i32) (result i32)
    (local i32)
    i32.const -2
    local.set 4
    block  ;; label = @1
      local.get 1
      i32.const 65535
      i32.gt_u
      br_if 0 (;@1;)
      local.get 3
      i32.const 506
      i32.ge_u
      br_if 0 (;@1;)
      block  ;; label = @2
        i32.const 505
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        i32.const 4
        i32.add
        i32.const 0
        i32.const 505
        memory.fill
      end
      local.get 0
      local.get 3
      i32.store8 offset=3
      local.get 0
      local.get 3
      i32.const 8
      i32.shr_u
      i32.store8 offset=2
      local.get 0
      local.get 1
      i32.store8 offset=1
      local.get 0
      local.get 1
      i32.const 8
      i32.shr_u
      i32.store8
      block  ;; label = @2
        local.get 3
        i32.eqz
        br_if 0 (;@2;)
        local.get 3
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        i32.const 4
        i32.add
        local.get 2
        local.get 3
        memory.copy
      end
      local.get 3
      i32.const 4
      i32.add
      local.set 4
    end
    local.get 4)
  (func (;12;) (type 0) (param i32 i32 i32) (result i32)
    (local i32 i32)
    i32.const -2
    local.set 3
    block  ;; label = @1
      local.get 1
      i32.const 10
      i32.le_u
      br_if 0 (;@1;)
      local.get 0
      i32.load16_u offset=9 align=1
      local.tee 4
      i32.const 8
      i32.shl
      local.get 4
      i32.const 8
      i32.shr_u
      i32.or
      i32.const 65535
      i32.and
      local.tee 4
      i32.const 498
      i32.gt_u
      br_if 0 (;@1;)
      local.get 4
      i32.const 11
      i32.add
      local.get 1
      i32.gt_u
      br_if 0 (;@1;)
      local.get 2
      local.get 0
      i32.load8_u
      i32.store
      local.get 2
      local.get 0
      i32.load8_u offset=1
      i32.const 8
      i32.shl
      local.get 0
      i32.load8_u offset=2
      i32.or
      i32.store offset=4
      local.get 2
      local.get 0
      i32.load8_u offset=3
      i32.const 8
      i32.shl
      local.get 0
      i32.load8_u offset=4
      i32.or
      local.tee 1
      i32.store offset=8
      local.get 0
      i32.load offset=5 align=1
      local.set 3
      local.get 2
      i32.const 11
      i32.store offset=20
      local.get 2
      local.get 4
      i32.store offset=16
      local.get 2
      local.get 3
      i32.store offset=12
      local.get 0
      i32.load8_u
      local.get 1
      call 13
      local.set 3
    end
    local.get 3)
  (func (;13;) (type 2) (param i32 i32) (result i32)
    (local i32 i32)
    i32.const -1
    local.set 2
    block  ;; label = @1
      local.get 0
      i32.const 255
      i32.gt_u
      br_if 0 (;@1;)
      local.get 1
      i32.const 65536
      i32.ge_u
      br_if 0 (;@1;)
      i32.const 0
      local.set 3
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 0
            call 14
            local.tee 2
            i32.const 4
            i32.add
            br_table 3 (;@1;) 2 (;@2;) 2 (;@2;) 2 (;@2;) 0 (;@4;) 1 (;@3;) 2 (;@2;)
          end
          i32.const -1
          local.set 2
          local.get 1
          br_if 2 (;@1;)
          br 1 (;@2;)
        end
        i32.const 0
        i32.const -1
        local.get 1
        select
        local.set 3
      end
      local.get 3
      local.set 2
    end
    local.get 2)
  (func (;14;) (type 3) (param i32) (result i32)
    (local i32)
    i32.const -4
    local.set 1
    block  ;; label = @1
      local.get 0
      i32.const -1
      i32.add
      i32.const 255
      i32.and
      local.tee 0
      i32.const 43
      i32.gt_u
      br_if 0 (;@1;)
      local.get 0
      i32.const 2
      i32.shl
      i32.load offset=1048576
      local.set 1
    end
    local.get 1)
  (func (;15;) (type 5) (param i32 i32 i32 i32 i32 i32 i32) (result i32)
    (local i32)
    i32.const -1
    local.set 7
    block  ;; label = @1
      local.get 1
      i32.const 255
      i32.gt_u
      br_if 0 (;@1;)
      local.get 2
      i32.const 65535
      i32.gt_u
      br_if 0 (;@1;)
      local.get 3
      i32.const 65536
      i32.ge_u
      br_if 0 (;@1;)
      i32.const -2
      local.set 7
      local.get 6
      i32.const 498
      i32.gt_u
      br_if 0 (;@1;)
      local.get 1
      local.get 3
      call 13
      local.tee 7
      br_if 0 (;@1;)
      block  ;; label = @2
        i32.const 498
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        i32.const 11
        i32.add
        i32.const 0
        i32.const 498
        memory.fill
      end
      local.get 0
      local.get 6
      i32.store8 offset=10
      local.get 0
      local.get 6
      i32.const 8
      i32.shr_u
      i32.store8 offset=9
      local.get 0
      local.get 4
      i32.store offset=5 align=1
      local.get 0
      local.get 3
      i32.store8 offset=4
      local.get 0
      local.get 3
      i32.const 8
      i32.shr_u
      i32.store8 offset=3
      local.get 0
      local.get 2
      i32.store8 offset=2
      local.get 0
      local.get 2
      i32.const 8
      i32.shr_u
      i32.store8 offset=1
      local.get 0
      local.get 1
      i32.store8
      block  ;; label = @2
        local.get 6
        i32.eqz
        br_if 0 (;@2;)
        local.get 6
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        i32.const 11
        i32.add
        local.get 5
        local.get 6
        memory.copy
      end
      local.get 6
      i32.const 11
      i32.add
      local.set 7
    end
    local.get 7)
  (func (;16;) (type 1) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32)
    i32.const -1
    local.set 4
    block  ;; label = @1
      local.get 1
      i32.const 1
      i32.and
      br_if 0 (;@1;)
      local.get 1
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.const 1
      i32.and
      br_if 0 (;@1;)
      i32.const 0
      local.set 5
      i32.const 0
      local.set 4
      block  ;; label = @2
        loop  ;; label = @3
          local.get 4
          local.get 1
          i32.ge_u
          br_if 1 (;@2;)
          local.get 0
          local.get 4
          i32.add
          local.set 6
          local.get 4
          i32.const 2
          i32.add
          local.set 4
          local.get 6
          i32.load16_u align=1
          local.tee 6
          i32.const 8
          i32.shl
          local.get 6
          i32.const 8
          i32.shr_u
          i32.or
          local.tee 7
          i32.const -6
          i32.add
          i32.const 65535
          i32.and
          i32.const 65533
          i32.lt_u
          br_if 0 (;@3;)
          i32.const 0
          local.set 6
          loop  ;; label = @4
            local.get 6
            local.get 3
            i32.ge_u
            br_if 1 (;@3;)
            local.get 7
            i32.const 65535
            i32.and
            local.tee 8
            local.get 5
            i32.const 65535
            i32.and
            local.tee 9
            local.get 8
            local.get 9
            i32.gt_u
            select
            local.get 5
            local.get 8
            local.get 2
            local.get 6
            i32.add
            i32.load16_u align=1
            local.tee 9
            i32.const 8
            i32.shl
            local.get 9
            i32.const 8
            i32.shr_u
            i32.or
            i32.const 65535
            i32.and
            i32.eq
            select
            local.set 5
            local.get 6
            i32.const 2
            i32.add
            local.set 6
            br 0 (;@4;)
          end
        end
      end
      local.get 5
      i32.const 65535
      i32.and
      local.tee 4
      i32.const -4
      local.get 4
      select
      local.set 4
    end
    local.get 4)
  (func (;17;) (type 1) (param i32 i32 i32 i32) (result i32)
    (local i32 i32)
    i32.const -1
    local.set 4
    block  ;; label = @1
      local.get 1
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      i32.const 1
      i32.and
      br_if 0 (;@1;)
      i32.const -2
      local.set 4
      local.get 1
      i32.const 64
      i32.gt_u
      br_if 0 (;@1;)
      local.get 1
      i32.const 1
      i32.shr_u
      local.tee 5
      local.get 3
      i32.gt_u
      br_if 0 (;@1;)
      local.get 5
      local.set 1
      loop  ;; label = @2
        block  ;; label = @3
          local.get 1
          br_if 0 (;@3;)
          local.get 5
          return
        end
        block  ;; label = @3
          local.get 0
          i32.load16_u align=1
          local.tee 4
          i32.const 8
          i32.shl
          local.get 4
          i32.const 8
          i32.shr_u
          i32.or
          local.tee 4
          i32.const -1
          i32.add
          i32.const 65535
          i32.and
          i32.const 2
          i32.ge_u
          br_if 0 (;@3;)
          i32.const -4
          local.set 4
          br 2 (;@1;)
        end
        local.get 2
        local.get 4
        i32.store16
        local.get 2
        i32.const 2
        i32.add
        local.set 2
        local.get 0
        i32.const 2
        i32.add
        local.set 0
        local.get 1
        i32.const -1
        i32.add
        local.set 1
        br 0 (;@2;)
      end
    end
    local.get 4)
  (func (;18;) (type 0) (param i32 i32 i32) (result i32)
    (local i32 i32)
    block  ;; label = @1
      local.get 2
      i32.const -33
      i32.add
      i32.const -32
      i32.ge_u
      br_if 0 (;@1;)
      i32.const -2
      return
    end
    local.get 2
    local.set 3
    loop (result i32)  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          local.get 3
          i32.eqz
          br_if 0 (;@3;)
          local.get 1
          i32.load16_u
          local.tee 4
          i32.const -1
          i32.add
          i32.const 65535
          i32.and
          i32.const 2
          i32.ge_u
          br_if 1 (;@2;)
          i32.const -4
          return
        end
        local.get 2
        i32.const 1
        i32.shl
        return
      end
      local.get 0
      local.get 4
      i32.const 8
      i32.shl
      local.get 4
      i32.const 65280
      i32.and
      i32.const 8
      i32.shr_u
      i32.or
      i32.store16 align=1
      local.get 1
      i32.const 2
      i32.add
      local.set 1
      local.get 0
      i32.const 2
      i32.add
      local.set 0
      local.get 3
      i32.const -1
      i32.add
      local.set 3
      br 0 (;@1;)
    end)
  (func (;19;) (type 1) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32)
    global.get 0
    i32.const 16
    i32.sub
    local.tee 4
    global.set 0
    block  ;; label = @1
      local.get 1
      local.get 2
      local.get 0
      local.get 4
      i32.const 8
      i32.add
      call 20
      local.tee 0
      i32.const -1
      i32.le_s
      br_if 0 (;@1;)
      block  ;; label = @2
        local.get 2
        local.get 0
        i32.const 3
        i32.add
        local.tee 5
        i32.ge_u
        br_if 0 (;@2;)
        i32.const -2
        local.set 0
        br 1 (;@1;)
      end
      local.get 1
      local.get 0
      i32.add
      local.tee 6
      i32.load8_u
      local.tee 7
      i32.extend8_s
      local.set 1
      block  ;; label = @2
        local.get 7
        i32.const 7
        i32.eq
        br_if 0 (;@2;)
        i32.const -1
        local.set 0
        local.get 1
        i32.const -1
        i32.gt_s
        br_if 1 (;@1;)
      end
      local.get 1
      local.get 4
      i32.load offset=8
      local.tee 8
      call 21
      local.tee 0
      br_if 0 (;@1;)
      i32.const -2
      local.set 0
      local.get 2
      local.get 6
      i32.load8_u offset=1
      i32.const 8
      i32.shl
      local.get 6
      i32.load8_u offset=2
      i32.or
      local.tee 1
      local.get 5
      i32.add
      local.tee 6
      i32.lt_u
      br_if 0 (;@1;)
      local.get 3
      local.get 6
      i32.store offset=16
      local.get 3
      local.get 1
      i32.store offset=12
      local.get 3
      local.get 5
      i32.store offset=8
      local.get 3
      local.get 7
      i32.store offset=4
      local.get 3
      local.get 8
      i32.store
      i32.const 0
      local.set 0
    end
    local.get 4
    i32.const 16
    i32.add
    global.set 0
    local.get 0)
  (func (;20;) (type 1) (param i32 i32 i32 i32) (result i32)
    (local i32 i32)
    i32.const -4
    local.set 4
    block  ;; label = @1
      local.get 2
      i32.const -6
      i32.add
      i32.const -4
      i32.le_u
      br_if 0 (;@1;)
      i32.const -2
      local.set 4
      local.get 1
      i32.const 2
      i32.const 4
      local.get 2
      i32.const 4
      i32.lt_u
      select
      local.tee 5
      i32.le_u
      br_if 0 (;@1;)
      local.get 0
      i32.load8_u offset=1
      local.set 4
      local.get 0
      i32.load8_u
      local.set 1
      block  ;; label = @2
        block  ;; label = @3
          local.get 2
          i32.const 4
          i32.ge_u
          br_if 0 (;@3;)
          local.get 1
          i32.const 8
          i32.shl
          local.get 4
          i32.or
          local.set 2
          br 1 (;@2;)
        end
        local.get 4
        i32.const 16
        i32.shl
        local.get 1
        i32.const 24
        i32.shl
        i32.or
        local.get 0
        i32.load8_u offset=2
        i32.const 8
        i32.shl
        i32.or
        local.get 0
        i32.load8_u offset=3
        i32.or
        local.set 2
      end
      local.get 3
      local.get 5
      i32.store offset=4
      local.get 3
      local.get 2
      i32.store
      local.get 5
      local.set 4
    end
    local.get 4)
  (func (;21;) (type 2) (param i32 i32) (result i32)
    (local i32)
    i32.const -1
    local.set 2
    block  ;; label = @1
      block  ;; label = @2
        local.get 0
        call 22
        local.tee 0
        i32.const -1
        i32.gt_s
        br_if 0 (;@2;)
        i32.const -4
        local.set 2
        br 1 (;@1;)
      end
      block  ;; label = @2
        local.get 0
        br_if 0 (;@2;)
        local.get 1
        br_if 1 (;@1;)
      end
      i32.const 0
      local.get 0
      i32.const 1
      i32.eq
      local.get 1
      i32.eqz
      i32.and
      i32.sub
      return
    end
    local.get 2)
  (func (;22;) (type 3) (param i32) (result i32)
    (local i32)
    i32.const 0
    local.set 1
    block  ;; label = @1
      block  ;; label = @2
        local.get 0
        i32.const 255
        i32.and
        local.tee 0
        i32.const 12
        i32.le_u
        br_if 0 (;@2;)
        local.get 0
        i32.const -128
        i32.add
        i32.const 4
        i32.lt_u
        br_if 1 (;@1;)
        i32.const -1
        return
      end
      i32.const 1
      local.get 0
      i32.shl
      i32.const 8062
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      i32.const 1
      local.set 1
    end
    local.get 1)
  (func (;23;) (type 6) (param i32 i32 i32 i32 i32 i32) (result i32)
    (local i32)
    i32.const -1
    local.set 6
    block  ;; label = @1
      local.get 3
      i32.const 256
      i32.ge_u
      br_if 0 (;@1;)
      block  ;; label = @2
        local.get 3
        i32.const 7
        i32.eq
        br_if 0 (;@2;)
        local.get 3
        i32.extend8_s
        i32.const -1
        i32.gt_s
        br_if 1 (;@1;)
      end
      i32.const -2
      local.set 6
      local.get 5
      i32.const 65535
      i32.gt_u
      br_if 0 (;@1;)
      local.get 3
      local.get 2
      call 21
      local.tee 6
      br_if 0 (;@1;)
      local.get 1
      local.get 0
      local.get 2
      call 24
      local.tee 6
      i32.const 0
      i32.lt_s
      br_if 0 (;@1;)
      local.get 1
      local.get 6
      i32.add
      local.tee 2
      local.get 5
      i32.store8 offset=2
      local.get 2
      local.get 5
      i32.const 8
      i32.shr_u
      i32.store8 offset=1
      local.get 2
      local.get 3
      i32.store8
      local.get 6
      i32.const 3
      i32.add
      local.set 3
      block  ;; label = @2
        local.get 5
        i32.eqz
        br_if 0 (;@2;)
        local.get 5
        i32.eqz
        br_if 0 (;@2;)
        local.get 1
        local.get 3
        i32.add
        local.get 4
        local.get 5
        memory.copy
      end
      local.get 3
      local.get 5
      i32.add
      local.set 6
    end
    local.get 6)
  (func (;24;) (type 0) (param i32 i32 i32) (result i32)
    (local i32)
    block  ;; label = @1
      local.get 1
      i32.const -6
      i32.add
      i32.const -3
      i32.ge_u
      br_if 0 (;@1;)
      i32.const -4
      return
    end
    i32.const 3
    local.set 3
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        i32.const 3
        i32.gt_u
        br_if 0 (;@2;)
        block  ;; label = @3
          local.get 2
          i32.const 65535
          i32.le_u
          br_if 0 (;@3;)
          i32.const -2
          return
        end
        local.get 0
        local.get 2
        i32.const 8
        i32.shr_u
        i32.store8
        i32.const 2
        local.set 1
        i32.const 1
        local.set 3
        br 1 (;@1;)
      end
      local.get 0
      local.get 2
      i32.const 8
      i32.shr_u
      i32.store8 offset=2
      local.get 0
      local.get 2
      i32.const 16
      i32.shr_u
      i32.store8 offset=1
      local.get 0
      local.get 2
      i32.const 24
      i32.shr_u
      i32.store8
      i32.const 4
      local.set 1
    end
    local.get 0
    local.get 3
    i32.add
    local.get 2
    i32.store8
    local.get 1)
  (func (;25;) (type 1) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32)
    global.get 0
    i32.const 16
    i32.sub
    local.tee 4
    global.set 0
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        local.get 2
        local.get 0
        local.get 4
        i32.const 8
        i32.add
        call 20
        local.tee 5
        i32.const -1
        i32.gt_s
        br_if 0 (;@2;)
        local.get 5
        local.set 0
        br 1 (;@1;)
      end
      i32.const -2
      local.set 0
      local.get 2
      local.get 5
      i32.const 510
      i32.add
      local.tee 6
      i32.lt_u
      br_if 0 (;@1;)
      i32.const -1
      local.set 0
      local.get 1
      local.get 5
      i32.add
      i32.load8_s
      local.tee 2
      i32.const 7
      i32.eq
      br_if 0 (;@1;)
      local.get 2
      i32.const 0
      i32.lt_s
      br_if 0 (;@1;)
      local.get 2
      local.get 4
      i32.load offset=8
      local.tee 1
      call 21
      local.tee 0
      br_if 0 (;@1;)
      local.get 3
      local.get 6
      i32.store offset=16
      local.get 3
      i32.const 509
      i32.store offset=12
      local.get 3
      local.get 5
      i32.const 1
      i32.add
      i32.store offset=8
      local.get 3
      local.get 2
      i32.store offset=4
      local.get 3
      local.get 1
      i32.store
      i32.const 0
      local.set 0
    end
    local.get 4
    i32.const 16
    i32.add
    global.set 0
    local.get 0)
  (func (;26;) (type 6) (param i32 i32 i32 i32 i32 i32) (result i32)
    (local i32)
    i32.const -1
    local.set 6
    block  ;; label = @1
      local.get 3
      i32.const 256
      i32.ge_u
      br_if 0 (;@1;)
      local.get 3
      i32.const 7
      i32.eq
      br_if 0 (;@1;)
      local.get 3
      i32.extend8_s
      i32.const 0
      i32.lt_s
      br_if 0 (;@1;)
      i32.const -2
      local.set 6
      local.get 5
      i32.const 509
      i32.gt_u
      br_if 0 (;@1;)
      local.get 3
      local.get 2
      call 21
      local.tee 6
      br_if 0 (;@1;)
      local.get 1
      local.get 0
      local.get 2
      call 24
      local.tee 6
      i32.const 0
      i32.lt_s
      br_if 0 (;@1;)
      local.get 1
      local.get 6
      i32.add
      local.tee 2
      local.get 3
      i32.store8
      local.get 2
      i32.const 1
      i32.add
      local.set 3
      block  ;; label = @2
        i32.const 509
        i32.eqz
        br_if 0 (;@2;)
        local.get 3
        i32.const 0
        i32.const 509
        memory.fill
      end
      local.get 6
      i32.const 510
      i32.add
      local.set 6
      local.get 5
      i32.eqz
      br_if 0 (;@1;)
      local.get 5
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      local.get 4
      local.get 5
      memory.copy
    end
    local.get 6)
  (func (;27;) (type 1) (param i32 i32 i32 i32) (result i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        i32.const 256
        i32.lt_u
        br_if 0 (;@2;)
        i32.const -1
        local.set 0
        br 1 (;@1;)
      end
      block  ;; label = @2
        local.get 0
        i32.const -6
        i32.add
        i32.const -3
        i32.ge_u
        br_if 0 (;@2;)
        i32.const -4
        return
      end
      block  ;; label = @2
        local.get 0
        i32.const 3
        i32.gt_u
        br_if 0 (;@2;)
        local.get 2
        i32.const 65535
        i32.le_u
        br_if 0 (;@2;)
        i32.const -2
        return
      end
      local.get 1
      local.get 2
      call 21
      local.tee 0
      br_if 0 (;@1;)
      block  ;; label = @2
        block  ;; label = @3
          local.get 1
          i32.const 7
          i32.eq
          br_if 0 (;@3;)
          local.get 1
          i32.extend8_s
          i32.const -1
          i32.gt_s
          br_if 1 (;@2;)
        end
        i32.const -2
        i32.const 0
        local.get 3
        i32.const 65535
        i32.gt_u
        select
        return
      end
      i32.const -2
      i32.const 0
      local.get 3
      i32.const 509
      i32.gt_u
      select
      return
    end
    local.get 0)
  (func (;28;) (type 3) (param i32) (result i32)
    (local i32)
    i32.const -1
    local.set 1
    block  ;; label = @1
      local.get 0
      i32.const 256
      i32.ge_u
      br_if 0 (;@1;)
      local.get 0
      call 14
      local.set 1
    end
    local.get 1)
  (func (;29;) (type 7) (result i32)
    i32.const 2)
  (func (;30;) (type 7) (result i32)
    i32.const 0)
  (func (;31;) (type 3) (param i32) (result i32)
    (local i32)
    i32.const -1
    local.set 1
    block  ;; label = @1
      local.get 0
      i32.const 256
      i32.ge_u
      br_if 0 (;@1;)
      local.get 0
      call 22
      local.set 1
    end
    local.get 1)
  (func (;32;) (type 3) (param i32) (result i32)
    i32.const -1
    local.get 0
    i32.const 7
    i32.eq
    local.get 0
    i32.const 128
    i32.and
    i32.const 7
    i32.shr_u
    i32.or
    local.get 0
    i32.const 255
    i32.gt_u
    select)
  (func (;33;) (type 3) (param i32) (result i32)
    i32.const -4
    i32.const 5
    i32.const 7
    local.get 0
    i32.const 4
    i32.lt_u
    select
    local.get 0
    i32.const -6
    i32.add
    i32.const -3
    i32.lt_u
    select)
  (func (;34;) (type 3) (param i32) (result i32)
    i32.const -4
    i32.const 512
    i32.const 514
    local.get 0
    i32.const 4
    i32.lt_u
    select
    local.get 0
    i32.const -6
    i32.add
    i32.const -3
    i32.lt_u
    select)
  (func (;35;) (type 3) (param i32) (result i32)
    i32.const -4
    i32.const 2
    i32.const 4
    local.get 0
    i32.const 4
    i32.lt_u
    select
    local.get 0
    i32.const -6
    i32.add
    i32.const -3
    i32.lt_u
    select)
  (func (;36;) (type 7) (result i32)
    i32.const 498)
  (func (;37;) (type 7) (result i32)
    i32.const 11)
  (func (;38;) (type 7) (result i32)
    i32.const 509)
  (func (;39;) (type 7) (result i32)
    i32.const 1)
  (func (;40;) (type 7) (result i32)
    i32.const 300224)
  (table (;0;) 1 1 funcref)
  (memory (;0;) 17)
  (global (;0;) (mut i32) (i32.const 1048576))
  (export "memory" (memory 0))
  (export "tor_cell_sendme_v1_digest_matches" (func 0))
  (export "tor_cell_parse_sendme_body" (func 1))
  (export "tor_cell_build_sendme_v1" (func 2))
  (export "tor_cell_build_sendme_v0" (func 3))
  (export "tor_cell_parse_padding_negotiate_body" (func 4))
  (export "tor_cell_build_padding_negotiate_body" (func 5))
  (export "tor_cell_parse_destroy_body" (func 6))
  (export "tor_cell_build_destroy_body" (func 7))
  (export "tor_cell_parse_created2_body" (func 8))
  (export "tor_cell_build_created2_body" (func 9))
  (export "tor_cell_parse_create2_body" (func 10))
  (export "tor_cell_build_create2_body" (func 11))
  (export "tor_cell_parse_relay_payload" (func 12))
  (export "tor_cell_validate_relay_stream_id" (func 13))
  (export "tor_cell_build_relay_payload" (func 15))
  (export "tor_cell_versions_negotiate" (func 16))
  (export "tor_cell_parse_versions_body" (func 17))
  (export "tor_cell_build_versions_body" (func 18))
  (export "tor_cell_parse_var" (func 19))
  (export "tor_cell_build_var" (func 23))
  (export "tor_cell_parse_fixed" (func 25))
  (export "tor_cell_build_fixed" (func 26))
  (export "tor_cell_validate_command" (func 27))
  (export "tor_cell_relay_stream_requirement" (func 28))
  (export "tor_cell_sendme_version_1" (func 39))
  (export "tor_cell_sendme_version_0" (func 30))
  (export "tor_cell_padding_command_start" (func 29))
  (export "tor_cell_padding_command_stop" (func 39))
  (export "tor_cell_destroy_reason_protocol" (func 29))
  (export "tor_cell_destroy_reason_none" (func 30))
  (export "tor_cell_command_circ_requirement" (func 31))
  (export "tor_cell_is_variable_command" (func 32))
  (export "tor_cell_var_header_len" (func 33))
  (export "tor_cell_fixed_len" (func 34))
  (export "tor_cell_circ_id_len" (func 35))
  (export "tor_cell_relay_data_max" (func 36))
  (export "tor_cell_relay_header_len" (func 37))
  (export "tor_cell_body_len" (func 38))
  (export "proto_abi_version" (func 39))
  (export "proto_standard_id" (func 40))
  (data (;0;) (i32.const 1048576) "\01\00\00\00\01\00\00\00\01\00\00\00\01\00\00\00\fe\ff\ff\ff\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\01\00\00\00\01\00\00\00\01\00\00\00\00\00\00\00\00\00\00\00\fc\ff\ff\ff\fc\ff\ff\ff\fc\ff\ff\ff\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\fc\ff\ff\ff\fc\ff\ff\ff\fc\ff\ff\ff\fc\ff\ff\ff\fc\ff\ff\ff\fc\ff\ff\ff\fc\ff\ff\ff\fc\ff\ff\ff\fc\ff\ff\ff\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\01\00\00\00\01\00\00\00"))
