$tor-device.wasm
  (type $t_6_0 (func (param i32 i32 i32 i32 i32 i32)))
  (type $t_6_1 (func (param i32 i32 i32 i32)))
  (type $t_6_2 (func (param i32 i32 i32)))
  (type $t_6_3 (func (param i32)))
  (type $t_6_4 (func (param i32 i32 i32 i32) (result i32)))
  (type $t_6_5 (func (param i32 i32 i32 i32 i32)))
  (type $t_6_6 (func (param i32 i32 i32) (result i32)))
  (type $t_6_7 (func (param i32 i32 i32 i32 i32) (result i32)))
  (type $t_6_8 (func (param i32 i32 i32 i32 i32 i32) (result i32)))
  (type $t_6_9 (func (param i32 i32)))
  (type $t_6_10 (func (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (type $t_6_11 (func (param i32 i32) (result i32)))
  (type $t_6_12 (func (param i32 i64) (result i64)))
  (type $t_6_13 (func (param i32) (result i32)))
  (type $t_6_14 (func (param i32 i32 i32 i32 i32 i32 i32 i32)))
  (type $t_6_15 (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type $t_6_16 (func (result i32)))
  (func $Io.Writer.fixed (type $t_6_1) (param i32 i32 i32 i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 3
    i32.store offset=12
    local.get 4
    local.get 2
    i32.store offset=8
    local.get 0
    i32.const 1049368
    i32.store
    local.get 0
    local.get 3
    i32.store offset=8
    local.get 0
    local.get 2
    i32.store offset=4
    local.get 0
    i32.const 0
    i32.store offset=12
    local.get 4
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $Io.Writer.buffered (type $t_6_2) (param i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 2
    i32.store offset=8
    local.get 3
    local.get 2
    i32.store offset=12
    local.get 3
    i32.load offset=12
    local.set 4
    local.get 4
    i32.load offset=12
    local.set 5
    local.get 4
    i32.load offset=8
    local.set 6
    local.get 4
    i32.load offset=4
    local.set 7
    local.get 5
    i32.const 0
    i32.sub
    local.set 8
    block  ;; label = @1
      block  ;; label = @2
        local.get 5
        local.get 6
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      local.get 5
      local.get 6
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 0
    local.get 8
    i32.store offset=4
    local.get 0
    local.get 7
    i32.store
    local.get 3
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $debug.FullPanic__function_'defaultPanic'__.memcpyAlias (type $t_6_3) (param i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 1
    local.get 1
    global.set $m6__stack_pointer
    local.get 1
    i32.const 0
    i32.store offset=8
    local.get 1
    i32.const 1
    i32.store8 offset=12
    local.get 0
    i32.const 1048649
    i32.const 23
    local.get 1
    i32.const 8
    i32.add
    call $debug.defaultPanic
    unreachable)
  (func $debug.defaultPanic (type $t_6_1) (param i32 i32 i32 i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 2
    i32.store offset=4
    local.get 4
    local.get 1
    i32.store
    local.get 4
    local.get 2
    i32.store offset=12
    local.get 4
    local.get 1
    i32.store offset=8
    unreachable)
  (func $Io.Writer.writeAll (type $t_6_4) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 48
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 1
    i32.store offset=12
    local.get 4
    local.get 3
    i32.store offset=20
    local.get 4
    local.get 2
    i32.store offset=16
    local.get 4
    local.get 1
    i32.store offset=24
    local.get 4
    local.get 3
    i32.store offset=32
    local.get 4
    local.get 2
    i32.store offset=28
    local.get 4
    i32.const 0
    i32.store offset=36
    block  ;; label = @1
      loop  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 4
                i32.load offset=36
                local.get 4
                i32.load offset=32
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 4
                i32.load offset=36
                local.set 5
                local.get 5
                local.set 6
                local.get 4
                i32.load offset=24
                local.set 7
                local.get 4
                i32.load offset=32
                local.set 8
                local.get 5
                local.get 4
                i32.load offset=28
                i32.add
                local.set 9
                local.get 5
                local.get 8
                i32.le_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 0
          local.get 5
          local.get 8
          call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
          unreachable
        end
        local.get 8
        local.get 5
        i32.sub
        local.set 10
        block  ;; label = @3
          block  ;; label = @4
            local.get 8
            local.get 8
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 8
          local.get 8
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 10
        local.set 11
        local.get 9
        local.set 12
        local.get 4
        i32.const 40
        i32.add
        local.get 0
        local.get 7
        local.get 12
        local.get 11
        call $Io.Writer.write
        local.get 4
        i32.load16_u offset=44
        local.set 13
        i32.const 0
        local.set 14
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      local.get 13
                      i32.const 65535
                      i32.and
                      local.get 14
                      i32.const 65535
                      i32.and
                      i32.ne
                      i32.const 1
                      i32.and
                      i32.eqz
                      br_if 0 (;@9;)
                      local.get 4
                      i32.load16_u offset=44
                      local.set 15
                      i32.const 0
                      local.set 16
                      local.get 15
                      i32.const 65535
                      i32.and
                      local.get 16
                      i32.const 65535
                      i32.and
                      i32.eq
                      i32.const 1
                      i32.and
                      br_if 1 (;@8;)
                      br 2 (;@7;)
                    end
                    local.get 6
                    local.get 4
                    i32.load offset=40
                    i32.add
                    local.set 17
                    local.get 17
                    local.get 6
                    i32.lt_u
                    i32.const 1
                    i32.and
                    br_if 2 (;@6;)
                    br 3 (;@5;)
                  end
                  br 3 (;@4;)
                end
                local.get 0
                call $builtin.returnError
                br 2 (;@4;)
              end
              local.get 0
              call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
              unreachable
            end
            local.get 4
            local.get 17
            i32.store offset=36
            br 1 (;@3;)
          end
          local.get 4
          i32.const 48
          i32.add
          global.set $m6__stack_pointer
          local.get 15
          return
        end
        br 0 (;@2;)
      end
    end
    i32.const 0
    local.set 18
    local.get 4
    i32.const 48
    i32.add
    global.set $m6__stack_pointer
    local.get 18
    return)
  (func $builtin.returnError (type $t_6_3) (param i32)
    (local i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 1
    local.get 1
    global.set $m6__stack_pointer
    local.get 1
    local.get 0
    i32.store offset=8
    local.get 1
    local.get 0
    i32.store offset=12
    block  ;; label = @1
      block  ;; label = @2
        local.get 1
        i32.load offset=8
        i32.load
        local.get 1
        i32.load offset=8
        i32.load offset=8
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 1
        i32.load offset=8
        local.set 2
        local.get 2
        i32.load
        local.set 3
        local.get 2
        i32.load offset=8
        drop
        local.get 2
        i32.load offset=4
        local.get 3
        i32.const 2
        i32.shl
        i32.add
        i32.const 0
        i32.store
        br 1 (;@1;)
      end
    end
    local.get 1
    i32.load offset=8
    local.set 4
    local.get 4
    local.get 4
    i32.load
    i32.const 1
    i32.add
    i32.store
    local.get 1
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd (type $t_6_2) (param i32 i32 i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 32
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 1
    i32.store offset=8
    local.get 3
    local.get 2
    i32.store offset=12
    local.get 3
    i32.const 0
    i32.store offset=16
    local.get 3
    i32.const 1
    i32.store8 offset=20
    local.get 3
    local.get 1
    i32.store offset=24
    local.get 3
    local.get 2
    i32.store offset=28
    local.get 0
    local.get 3
    i32.const 16
    i32.add
    local.get 3
    i32.const 24
    i32.add
    call $debug.panicExtra__anon_2170
    unreachable)
  (func $debug.FullPanic__function_'defaultPanic'__.outOfBounds (type $t_6_2) (param i32 i32 i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 32
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 1
    i32.store offset=8
    local.get 3
    local.get 2
    i32.store offset=12
    local.get 3
    i32.const 0
    i32.store offset=16
    local.get 3
    i32.const 1
    i32.store8 offset=20
    local.get 3
    local.get 1
    i32.store offset=24
    local.get 3
    local.get 2
    i32.store offset=28
    local.get 0
    local.get 3
    i32.const 16
    i32.add
    local.get 3
    i32.const 24
    i32.add
    call $debug.panicExtra__anon_2155
    unreachable)
  (func $Io.Writer.write (type $t_6_5) (param i32 i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 48
    i32.sub
    local.set 5
    local.get 5
    global.set $m6__stack_pointer
    local.get 4
    local.set 6
    local.get 3
    local.set 7
    local.get 5
    local.get 2
    i32.store
    local.get 5
    local.get 4
    i32.store offset=8
    local.get 5
    local.get 3
    i32.store offset=4
    local.get 5
    local.get 2
    i32.store offset=12
    local.get 5
    local.get 4
    i32.store offset=20
    local.get 5
    local.get 3
    i32.store offset=16
    local.get 5
    i32.load offset=12
    i32.load offset=12
    local.set 8
    local.get 8
    local.get 5
    i32.load offset=20
    i32.add
    local.set 9
    block  ;; label = @1
      local.get 9
      local.get 8
      i32.lt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 9
              local.get 5
              i32.load offset=12
              i32.load offset=8
              i32.le_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              local.get 5
              i32.load offset=12
              local.set 10
              local.get 10
              i32.load offset=12
              local.set 11
              local.get 5
              i32.load offset=20
              local.set 12
              local.get 10
              i32.load offset=8
              local.set 13
              local.get 11
              local.get 10
              i32.load offset=4
              i32.add
              local.set 14
              local.get 11
              local.get 12
              i32.add
              local.set 15
              local.get 15
              local.get 13
              i32.le_u
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            br 3 (;@1;)
          end
          br 1 (;@2;)
        end
        local.get 1
        local.get 15
        local.get 13
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 12
      local.set 16
      local.get 14
      local.set 17
      block  ;; label = @2
        block  ;; label = @3
          local.get 16
          local.get 6
          i32.eq
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 1
        call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
        unreachable
      end
      local.get 7
      local.get 16
      i32.add
      local.set 18
      local.get 17
      local.get 16
      i32.add
      local.set 19
      block  ;; label = @2
        block  ;; label = @3
          local.get 17
          local.get 18
          i32.ge_u
          local.get 7
          local.get 19
          i32.ge_u
          i32.or
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 1
        call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
        unreachable
      end
      block  ;; label = @2
        local.get 16
        i32.eqz
        br_if 0 (;@2;)
        local.get 17
        local.get 7
        local.get 16
        memory.copy
      end
      local.get 5
      i32.load offset=12
      local.set 20
      local.get 20
      i32.const 12
      i32.add
      local.set 21
      local.get 20
      i32.load offset=12
      local.set 22
      local.get 22
      local.get 5
      i32.load offset=20
      i32.add
      local.set 23
      block  ;; label = @2
        local.get 23
        local.get 22
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 1
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      local.get 21
      local.get 23
      i32.store
      local.get 5
      i32.load offset=20
      local.set 24
      local.get 5
      i32.const 0
      i32.store16 offset=28
      local.get 5
      local.get 24
      i32.store offset=24
      local.get 5
      i32.load16_u offset=28
      local.set 25
      i32.const 0
      local.set 26
      block  ;; label = @2
        block  ;; label = @3
          local.get 25
          i32.const 65535
          i32.and
          local.get 26
          i32.const 65535
          i32.and
          i32.eq
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 1
        call $builtin.returnError
      end
      local.get 0
      local.get 5
      i64.load offset=24 align=4
      i64.store align=4
      local.get 5
      i32.const 48
      i32.add
      global.set $m6__stack_pointer
      return
    end
    local.get 5
    i32.load offset=12
    i32.load
    i32.load
    local.set 27
    local.get 5
    local.get 6
    i32.store offset=36
    local.get 5
    local.get 7
    i32.store offset=32
    i32.const 1
    local.set 28
    local.get 5
    i32.const 32
    i32.add
    local.set 29
    local.get 5
    i32.const 40
    i32.add
    local.get 1
    local.get 2
    local.get 29
    local.get 28
    i32.const 1
    local.get 27
    call_indirect (type $t_6_0)
    local.get 5
    i32.load16_u offset=44
    local.set 30
    i32.const 0
    local.set 31
    block  ;; label = @1
      block  ;; label = @2
        local.get 30
        i32.const 65535
        i32.and
        local.get 31
        i32.const 65535
        i32.and
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      call $builtin.returnError
    end
    local.get 0
    local.get 5
    i64.load offset=40 align=4
    i64.store align=4
    local.get 5
    i32.const 48
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $debug.FullPanic__function_'defaultPanic'__.integerOverflow (type $t_6_3) (param i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 1
    local.get 1
    global.set $m6__stack_pointer
    local.get 1
    i32.const 0
    i32.store offset=8
    local.get 1
    i32.const 1
    i32.store8 offset=12
    local.get 0
    i32.const 1048576
    i32.const 16
    local.get 1
    i32.const 8
    i32.add
    call $debug.defaultPanic
    unreachable)
  (func $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch (type $t_6_3) (param i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 1
    local.get 1
    global.set $m6__stack_pointer
    local.get 1
    i32.const 0
    i32.store offset=8
    local.get 1
    i32.const 1
    i32.store8 offset=12
    local.get 0
    i32.const 1048593
    i32.const 55
    local.get 1
    i32.const 8
    i32.add
    call $debug.defaultPanic
    unreachable)
  (func $debug.panicExtra__anon_2155 (type $t_6_2) (param i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 4176
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 0
    i32.load
    local.set 4
    local.get 3
    i32.const 1049203
    i32.store offset=12
    i32.const 4111
    local.set 5
    i32.const 170
    local.set 6
    block  ;; label = @1
      local.get 5
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.const 17
      i32.add
      local.get 6
      local.get 5
      memory.fill
    end
    i32.const 4096
    local.set 7
    local.get 3
    i32.const 17
    i32.add
    local.set 8
    local.get 3
    i32.const 4144
    i32.add
    local.get 0
    local.get 8
    local.get 7
    call $Io.Writer.fixed
    i32.const 8
    local.set 9
    local.get 9
    local.get 3
    i32.const 4128
    i32.add
    i32.add
    local.get 9
    local.get 3
    i32.const 4144
    i32.add
    i32.add
    i64.load align=4
    i64.store
    local.get 3
    local.get 3
    i64.load offset=4144 align=4
    i64.store offset=4128
    local.get 0
    local.get 3
    i32.const 4128
    i32.add
    local.get 2
    call $Io.Writer.print__anon_2163
    local.set 10
    i32.const 0
    local.set 11
    block  ;; label = @1
      block  ;; label = @2
        local.get 10
        i32.const 65535
        i32.and
        local.get 11
        i32.const 65535
        i32.and
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 3
        local.get 0
        local.get 3
        i32.const 4128
        i32.add
        call $Io.Writer.buffered
        local.get 3
        i32.load offset=4
        local.set 12
        local.get 3
        i32.load
        local.set 13
        local.get 12
        local.set 14
        br 1 (;@1;)
      end
      local.get 3
      i32.const 17
      i32.add
      i32.const 4096
      i32.add
      local.set 15
      local.get 15
      i32.const 15
      i32.add
      local.set 16
      block  ;; label = @2
        block  ;; label = @3
          local.get 15
          i32.const 1049203
          i32.const 15
          i32.add
          i32.ge_u
          i32.const 1049203
          local.get 16
          i32.ge_u
          i32.or
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
        unreachable
      end
      local.get 15
      i32.const 7
      i32.add
      local.set 17
      i32.const 0
      local.set 18
      local.get 17
      local.get 18
      i64.load offset=1049210 align=1
      i64.store align=1
      local.get 15
      local.get 18
      i64.load offset=1049203 align=1
      i64.store align=1
      local.get 0
      local.get 4
      i32.store
      i32.const 4111
      local.set 19
      local.get 3
      i32.const 17
      i32.add
      local.set 13
      local.get 19
      local.set 14
    end
    local.get 14
    local.set 20
    local.get 13
    local.set 21
    local.get 3
    local.get 21
    i32.store offset=4160
    local.get 3
    local.get 20
    i32.store offset=4164
    local.get 3
    local.get 20
    i32.store offset=4172
    local.get 3
    local.get 21
    i32.store offset=4168
    local.get 3
    i32.load offset=4172
    local.set 22
    local.get 0
    local.get 3
    i32.load offset=4168
    local.get 22
    local.get 1
    call $debug.defaultPanic
    unreachable)
  (func $Io.Writer.print__anon_2163 (type $t_6_6) (param i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 48
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 1
    i32.store offset=8
    local.get 3
    local.get 1
    i32.store offset=12
    local.get 3
    local.get 2
    i64.load align=4
    i64.store offset=16
    local.get 3
    i32.const 2
    i32.store offset=32
    local.get 3
    i32.const 1049220
    i32.store offset=28
    local.get 3
    i32.const 32
    i32.store16 offset=38
    local.get 0
    local.get 3
    i32.load offset=12
    i32.const 1049268
    i32.const 27
    call $Io.Writer.writeAll
    local.set 4
    i32.const 0
    local.set 5
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        local.get 4
                        i32.const 65535
                        i32.and
                        local.get 5
                        i32.const 65535
                        i32.and
                        i32.ne
                        i32.const 1
                        i32.and
                        i32.eqz
                        br_if 0 (;@10;)
                        i32.const 0
                        local.set 6
                        local.get 4
                        i32.const 65535
                        i32.and
                        local.get 6
                        i32.const 65535
                        i32.and
                        i32.eq
                        i32.const 1
                        i32.and
                        br_if 1 (;@9;)
                        br 2 (;@8;)
                      end
                      local.get 3
                      i32.const 0
                      i32.store offset=40
                      local.get 3
                      i32.load offset=12
                      local.set 7
                      local.get 3
                      i32.load offset=16
                      local.set 8
                      local.get 0
                      local.get 7
                      i32.const 1049296
                      local.get 8
                      i32.const 3
                      call $Io.Writer.printValue__anon_2477
                      local.set 9
                      i32.const 0
                      local.set 10
                      local.get 9
                      i32.const 65535
                      i32.and
                      local.get 10
                      i32.const 65535
                      i32.and
                      i32.ne
                      i32.const 1
                      i32.and
                      br_if 2 (;@7;)
                      br 3 (;@6;)
                    end
                    br 6 (;@2;)
                  end
                  local.get 0
                  call $builtin.returnError
                  br 5 (;@2;)
                end
                i32.const 0
                local.set 11
                local.get 9
                i32.const 65535
                i32.and
                local.get 11
                i32.const 65535
                i32.and
                i32.eq
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 0
          call $builtin.returnError
        end
        local.get 3
        i32.const 48
        i32.add
        global.set $m6__stack_pointer
        local.get 9
        return
      end
      local.get 3
      i32.const 48
      i32.add
      global.set $m6__stack_pointer
      local.get 4
      return
    end
    local.get 0
    local.get 3
    i32.load offset=12
    i32.const 1049316
    i32.const 6
    call $Io.Writer.writeAll
    local.set 12
    i32.const 0
    local.set 13
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        local.get 12
                        i32.const 65535
                        i32.and
                        local.get 13
                        i32.const 65535
                        i32.and
                        i32.ne
                        i32.const 1
                        i32.and
                        i32.eqz
                        br_if 0 (;@10;)
                        i32.const 0
                        local.set 14
                        local.get 12
                        i32.const 65535
                        i32.and
                        local.get 14
                        i32.const 65535
                        i32.and
                        i32.eq
                        i32.const 1
                        i32.and
                        br_if 1 (;@9;)
                        br 2 (;@8;)
                      end
                      local.get 3
                      i32.const 1
                      i32.store offset=44
                      local.get 3
                      i32.load offset=12
                      local.set 15
                      local.get 3
                      i32.load offset=20
                      local.set 16
                      local.get 0
                      local.get 15
                      i32.const 1049296
                      local.get 16
                      i32.const 3
                      call $Io.Writer.printValue__anon_2477
                      local.set 17
                      i32.const 0
                      local.set 18
                      local.get 17
                      i32.const 65535
                      i32.and
                      local.get 18
                      i32.const 65535
                      i32.and
                      i32.ne
                      i32.const 1
                      i32.and
                      br_if 2 (;@7;)
                      br 3 (;@6;)
                    end
                    br 6 (;@2;)
                  end
                  local.get 0
                  call $builtin.returnError
                  br 5 (;@2;)
                end
                i32.const 0
                local.set 19
                local.get 17
                i32.const 65535
                i32.and
                local.get 19
                i32.const 65535
                i32.and
                i32.eq
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 0
          call $builtin.returnError
        end
        local.get 3
        i32.const 48
        i32.add
        global.set $m6__stack_pointer
        local.get 17
        return
      end
      local.get 3
      i32.const 48
      i32.add
      global.set $m6__stack_pointer
      local.get 12
      return
    end
    i32.const 0
    local.set 20
    local.get 3
    i32.const 48
    i32.add
    global.set $m6__stack_pointer
    local.get 20
    return)
  (func $debug.panicExtra__anon_2170 (type $t_6_2) (param i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 4176
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 0
    i32.load
    local.set 4
    local.get 3
    i32.const 1049203
    i32.store offset=12
    i32.const 4111
    local.set 5
    i32.const 170
    local.set 6
    block  ;; label = @1
      local.get 5
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.const 17
      i32.add
      local.get 6
      local.get 5
      memory.fill
    end
    i32.const 4096
    local.set 7
    local.get 3
    i32.const 17
    i32.add
    local.set 8
    local.get 3
    i32.const 4144
    i32.add
    local.get 0
    local.get 8
    local.get 7
    call $Io.Writer.fixed
    i32.const 8
    local.set 9
    local.get 9
    local.get 3
    i32.const 4128
    i32.add
    i32.add
    local.get 9
    local.get 3
    i32.const 4144
    i32.add
    i32.add
    i64.load align=4
    i64.store
    local.get 3
    local.get 3
    i64.load offset=4144 align=4
    i64.store offset=4128
    local.get 0
    local.get 3
    i32.const 4128
    i32.add
    local.get 2
    call $Io.Writer.print__anon_2519
    local.set 10
    i32.const 0
    local.set 11
    block  ;; label = @1
      block  ;; label = @2
        local.get 10
        i32.const 65535
        i32.and
        local.get 11
        i32.const 65535
        i32.and
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 3
        local.get 0
        local.get 3
        i32.const 4128
        i32.add
        call $Io.Writer.buffered
        local.get 3
        i32.load offset=4
        local.set 12
        local.get 3
        i32.load
        local.set 13
        local.get 12
        local.set 14
        br 1 (;@1;)
      end
      local.get 3
      i32.const 17
      i32.add
      i32.const 4096
      i32.add
      local.set 15
      local.get 15
      i32.const 15
      i32.add
      local.set 16
      block  ;; label = @2
        block  ;; label = @3
          local.get 15
          i32.const 1049203
          i32.const 15
          i32.add
          i32.ge_u
          i32.const 1049203
          local.get 16
          i32.ge_u
          i32.or
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
        unreachable
      end
      local.get 15
      i32.const 7
      i32.add
      local.set 17
      i32.const 0
      local.set 18
      local.get 17
      local.get 18
      i64.load offset=1049210 align=1
      i64.store align=1
      local.get 15
      local.get 18
      i64.load offset=1049203 align=1
      i64.store align=1
      local.get 0
      local.get 4
      i32.store
      i32.const 4111
      local.set 19
      local.get 3
      i32.const 17
      i32.add
      local.set 13
      local.get 19
      local.set 14
    end
    local.get 14
    local.set 20
    local.get 13
    local.set 21
    local.get 3
    local.get 21
    i32.store offset=4160
    local.get 3
    local.get 20
    i32.store offset=4164
    local.get 3
    local.get 20
    i32.store offset=4172
    local.get 3
    local.get 21
    i32.store offset=4168
    local.get 3
    i32.load offset=4172
    local.set 22
    local.get 0
    local.get 3
    i32.load offset=4168
    local.get 22
    local.get 1
    call $debug.defaultPanic
    unreachable)
  (func $Io.Writer.printValue__anon_2477 (type $t_6_7) (param i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i64 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 64
    i32.sub
    local.set 5
    local.get 5
    global.set $m6__stack_pointer
    local.get 5
    local.get 1
    i32.store offset=8
    local.get 5
    local.get 3
    i32.store offset=12
    local.get 5
    local.get 4
    i32.store offset=16
    local.get 5
    local.get 1
    i32.store offset=20
    i32.const 16
    local.set 6
    local.get 2
    local.get 6
    i32.add
    i32.load
    local.set 7
    local.get 6
    local.get 5
    i32.const 24
    i32.add
    i32.add
    local.get 7
    i32.store
    i32.const 8
    local.set 8
    local.get 2
    local.get 8
    i32.add
    i64.load align=4
    local.set 9
    local.get 8
    local.get 5
    i32.const 24
    i32.add
    i32.add
    local.get 9
    i64.store
    local.get 5
    local.get 2
    i64.load align=4
    i64.store offset=24
    local.get 5
    local.get 3
    i32.store offset=48
    local.get 5
    local.get 1
    i32.store offset=52
    local.get 5
    local.get 3
    i32.store offset=56
    local.get 5
    i32.const 10
    i32.store8 offset=62
    local.get 5
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=63
    local.get 0
    local.get 1
    local.get 3
    i32.const 10
    i32.const 0
    local.get 2
    call $Io.Writer.printIntAny__anon_2550
    local.set 10
    i32.const 0
    local.set 11
    block  ;; label = @1
      block  ;; label = @2
        local.get 10
        i32.const 65535
        i32.and
        local.get 11
        i32.const 65535
        i32.and
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $builtin.returnError
    end
    local.get 10
    local.set 12
    local.get 12
    local.set 13
    i32.const 0
    local.set 14
    block  ;; label = @1
      block  ;; label = @2
        local.get 13
        i32.const 65535
        i32.and
        local.get 14
        i32.const 65535
        i32.and
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $builtin.returnError
    end
    local.get 5
    i32.const 64
    i32.add
    global.set $m6__stack_pointer
    local.get 13
    return)
  (func $Io.Writer.print__anon_2519 (type $t_6_6) (param i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 48
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 1
    i32.store offset=8
    local.get 3
    local.get 1
    i32.store offset=12
    local.get 3
    local.get 2
    i64.load align=4
    i64.store offset=16
    local.get 3
    i32.const 2
    i32.store offset=32
    local.get 3
    i32.const 1049220
    i32.store offset=28
    local.get 3
    i32.const 32
    i32.store16 offset=38
    local.get 0
    local.get 3
    i32.load offset=12
    i32.const 1049322
    i32.const 12
    call $Io.Writer.writeAll
    local.set 4
    i32.const 0
    local.set 5
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        local.get 4
                        i32.const 65535
                        i32.and
                        local.get 5
                        i32.const 65535
                        i32.and
                        i32.ne
                        i32.const 1
                        i32.and
                        i32.eqz
                        br_if 0 (;@10;)
                        i32.const 0
                        local.set 6
                        local.get 4
                        i32.const 65535
                        i32.and
                        local.get 6
                        i32.const 65535
                        i32.and
                        i32.eq
                        i32.const 1
                        i32.and
                        br_if 1 (;@9;)
                        br 2 (;@8;)
                      end
                      local.get 3
                      i32.const 0
                      i32.store offset=40
                      local.get 3
                      i32.load offset=12
                      local.set 7
                      local.get 3
                      i32.load offset=16
                      local.set 8
                      local.get 0
                      local.get 7
                      i32.const 1049296
                      local.get 8
                      i32.const 3
                      call $Io.Writer.printValue__anon_2477
                      local.set 9
                      i32.const 0
                      local.set 10
                      local.get 9
                      i32.const 65535
                      i32.and
                      local.get 10
                      i32.const 65535
                      i32.and
                      i32.ne
                      i32.const 1
                      i32.and
                      br_if 2 (;@7;)
                      br 3 (;@6;)
                    end
                    br 6 (;@2;)
                  end
                  local.get 0
                  call $builtin.returnError
                  br 5 (;@2;)
                end
                i32.const 0
                local.set 11
                local.get 9
                i32.const 65535
                i32.and
                local.get 11
                i32.const 65535
                i32.and
                i32.eq
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 0
          call $builtin.returnError
        end
        local.get 3
        i32.const 48
        i32.add
        global.set $m6__stack_pointer
        local.get 9
        return
      end
      local.get 3
      i32.const 48
      i32.add
      global.set $m6__stack_pointer
      local.get 4
      return
    end
    local.get 0
    local.get 3
    i32.load offset=12
    i32.const 1049334
    i32.const 26
    call $Io.Writer.writeAll
    local.set 12
    i32.const 0
    local.set 13
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        local.get 12
                        i32.const 65535
                        i32.and
                        local.get 13
                        i32.const 65535
                        i32.and
                        i32.ne
                        i32.const 1
                        i32.and
                        i32.eqz
                        br_if 0 (;@10;)
                        i32.const 0
                        local.set 14
                        local.get 12
                        i32.const 65535
                        i32.and
                        local.get 14
                        i32.const 65535
                        i32.and
                        i32.eq
                        i32.const 1
                        i32.and
                        br_if 1 (;@9;)
                        br 2 (;@8;)
                      end
                      local.get 3
                      i32.const 1
                      i32.store offset=44
                      local.get 3
                      i32.load offset=12
                      local.set 15
                      local.get 3
                      i32.load offset=20
                      local.set 16
                      local.get 0
                      local.get 15
                      i32.const 1049296
                      local.get 16
                      i32.const 3
                      call $Io.Writer.printValue__anon_2477
                      local.set 17
                      i32.const 0
                      local.set 18
                      local.get 17
                      i32.const 65535
                      i32.and
                      local.get 18
                      i32.const 65535
                      i32.and
                      i32.ne
                      i32.const 1
                      i32.and
                      br_if 2 (;@7;)
                      br 3 (;@6;)
                    end
                    br 6 (;@2;)
                  end
                  local.get 0
                  call $builtin.returnError
                  br 5 (;@2;)
                end
                i32.const 0
                local.set 19
                local.get 17
                i32.const 65535
                i32.and
                local.get 19
                i32.const 65535
                i32.and
                i32.eq
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 0
          call $builtin.returnError
        end
        local.get 3
        i32.const 48
        i32.add
        global.set $m6__stack_pointer
        local.get 17
        return
      end
      local.get 3
      i32.const 48
      i32.add
      global.set $m6__stack_pointer
      local.get 12
      return
    end
    i32.const 0
    local.set 20
    local.get 3
    i32.const 48
    i32.add
    global.set $m6__stack_pointer
    local.get 20
    return)
  (func $Io.Writer.printIntAny__anon_2550 (type $t_6_8) (param i32 i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i64 i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 112
    i32.sub
    local.set 6
    local.get 6
    global.set $m6__stack_pointer
    local.get 6
    local.get 1
    i32.store
    local.get 6
    local.get 2
    i32.store offset=4
    local.get 6
    local.get 3
    i32.store8 offset=10
    local.get 3
    i32.const 255
    i32.and
    local.set 7
    i32.const 1
    local.set 8
    local.get 6
    local.get 4
    local.get 8
    i32.and
    i32.store8 offset=11
    local.get 6
    local.get 1
    i32.store offset=12
    i32.const 16
    local.set 9
    local.get 5
    local.get 9
    i32.add
    i32.load
    local.set 10
    local.get 9
    local.get 6
    i32.const 16
    i32.add
    i32.add
    local.get 10
    i32.store
    i32.const 8
    local.set 11
    local.get 5
    local.get 11
    i32.add
    i64.load align=4
    local.set 12
    local.get 11
    local.get 6
    i32.const 16
    i32.add
    i32.add
    local.get 12
    i64.store
    local.get 6
    local.get 5
    i64.load align=4
    i64.store offset=16
    local.get 0
    local.get 7
    local.get 8
    i32.gt_u
    call $debug.assert
    local.get 6
    i32.const 32
    i32.store8 offset=43
    local.get 6
    local.get 2
    i32.store offset=44
    local.get 6
    i32.const 80
    i32.add
    i32.const -86
    i32.store8
    local.get 6
    i32.const 72
    i32.add
    local.set 13
    i64.const -6148914691236517206
    local.set 14
    local.get 13
    local.get 14
    i64.store
    local.get 6
    i32.const 64
    i32.add
    local.get 14
    i64.store
    local.get 6
    i32.const 56
    i32.add
    local.get 14
    i64.store
    local.get 6
    local.get 14
    i64.store offset=48
    local.get 6
    local.get 2
    i32.store offset=88
    local.get 6
    i32.const 33
    i32.store offset=92
    i32.const 10
    local.set 15
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 3
              i32.const 255
              i32.and
              local.get 15
              i32.const 255
              i32.and
              i32.eq
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            br 1 (;@3;)
          end
          loop  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  local.get 6
                  i32.load offset=88
                  i32.const 100
                  i32.ge_u
                  i32.const 1
                  i32.and
                  i32.eqz
                  br_if 0 (;@7;)
                  local.get 6
                  i32.load offset=92
                  local.set 16
                  local.get 16
                  i32.const -2
                  i32.add
                  local.set 17
                  local.get 17
                  local.get 16
                  i32.gt_u
                  i32.const 1
                  i32.and
                  br_if 1 (;@6;)
                  br 2 (;@5;)
                end
                br 4 (;@2;)
              end
              local.get 0
              call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
              unreachable
            end
            local.get 6
            local.get 17
            i32.store offset=92
            local.get 6
            i32.load offset=92
            local.set 18
            local.get 18
            local.get 6
            i32.const 48
            i32.add
            i32.add
            local.set 19
            local.get 18
            i32.const 2
            i32.add
            local.set 20
            block  ;; label = @5
              block  ;; label = @6
                local.get 20
                i32.const 33
                i32.le_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                br 1 (;@5;)
              end
              local.get 0
              local.get 20
              i32.const 33
              call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
              unreachable
            end
            local.get 6
            i32.load offset=88
            i32.const 100
            i32.rem_u
            local.set 21
            block  ;; label = @5
              local.get 21
              i32.const 255
              i32.le_u
              i32.const 1
              i32.and
              br_if 0 (;@5;)
              local.get 0
              call $debug.FullPanic__function_'defaultPanic'__.integerOutOfBounds
              unreachable
            end
            local.get 6
            i32.const 96
            i32.add
            local.get 0
            local.get 21
            call $fmt.digits2
            local.get 19
            local.get 6
            i32.load16_u offset=96 align=1
            i32.store16 align=1
            local.get 6
            local.get 6
            i32.load offset=88
            i32.const 100
            i32.div_u
            i32.store offset=88
            br 0 (;@4;)
          end
        end
        loop  ;; label = @3
          local.get 6
          i32.load offset=88
          local.set 22
          local.get 3
          i32.const 255
          i32.and
          local.set 23
          block  ;; label = @4
            block  ;; label = @5
              local.get 23
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 0
            call $debug.FullPanic__function_'defaultPanic'__.divideByZero
            unreachable
          end
          local.get 22
          local.get 23
          i32.rem_u
          local.set 24
          local.get 6
          local.get 24
          i32.store offset=100
          local.get 6
          i32.load offset=92
          local.set 25
          local.get 25
          i32.const -1
          i32.add
          local.set 26
          block  ;; label = @4
            local.get 26
            local.get 25
            i32.gt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 0
            call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
            unreachable
          end
          local.get 6
          local.get 26
          i32.store offset=92
          local.get 6
          i32.load offset=92
          local.set 27
          block  ;; label = @4
            block  ;; label = @5
              local.get 27
              i32.const 33
              i32.lt_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 0
            local.get 27
            i32.const 33
            call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
            unreachable
          end
          local.get 6
          i32.const 48
          i32.add
          local.get 27
          i32.add
          local.set 28
          block  ;; label = @4
            local.get 24
            i32.const 255
            i32.le_u
            i32.const 1
            i32.and
            br_if 0 (;@4;)
            local.get 0
            call $debug.FullPanic__function_'defaultPanic'__.integerOutOfBounds
            unreachable
          end
          local.get 28
          local.get 0
          local.get 24
          local.get 4
          call $fmt.digitToChar
          i32.store8
          local.get 6
          i32.load offset=88
          local.set 29
          local.get 3
          i32.const 255
          i32.and
          local.set 30
          block  ;; label = @4
            block  ;; label = @5
              local.get 30
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 0
            call $debug.FullPanic__function_'defaultPanic'__.divideByZero
            unreachable
          end
          local.get 6
          local.get 29
          local.get 30
          i32.div_u
          i32.store offset=88
          block  ;; label = @4
            block  ;; label = @5
              local.get 6
              i32.load offset=88
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            br 1 (;@3;)
          end
        end
        br 1 (;@1;)
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 6
                    i32.load offset=88
                    i32.const 10
                    i32.lt_u
                    i32.const 1
                    i32.and
                    i32.eqz
                    br_if 0 (;@8;)
                    local.get 6
                    i32.load offset=92
                    local.set 31
                    local.get 31
                    i32.const -1
                    i32.add
                    local.set 32
                    local.get 32
                    local.get 31
                    i32.gt_u
                    i32.const 1
                    i32.and
                    br_if 1 (;@7;)
                    br 2 (;@6;)
                  end
                  local.get 6
                  i32.load offset=92
                  local.set 33
                  local.get 33
                  i32.const -2
                  i32.add
                  local.set 34
                  local.get 34
                  local.get 33
                  i32.gt_u
                  i32.const 1
                  i32.and
                  br_if 2 (;@5;)
                  br 3 (;@4;)
                end
                local.get 0
                call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
                unreachable
              end
              local.get 6
              local.get 32
              i32.store offset=92
              local.get 6
              i32.load offset=92
              local.set 35
              block  ;; label = @6
                local.get 35
                i32.const 33
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                br 3 (;@3;)
              end
              local.get 0
              local.get 35
              i32.const 33
              call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
              unreachable
            end
            local.get 0
            call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
            unreachable
          end
          local.get 6
          local.get 34
          i32.store offset=92
          local.get 6
          i32.load offset=92
          local.set 36
          local.get 36
          local.get 6
          i32.const 48
          i32.add
          i32.add
          local.set 37
          local.get 36
          i32.const 2
          i32.add
          local.set 38
          block  ;; label = @4
            block  ;; label = @5
              local.get 38
              i32.const 33
              i32.le_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 0
            local.get 38
            i32.const 33
            call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
            unreachable
          end
          local.get 6
          i32.load offset=88
          local.set 39
          block  ;; label = @4
            local.get 39
            i32.const 255
            i32.le_u
            i32.const 1
            i32.and
            br_if 0 (;@4;)
            local.get 0
            call $debug.FullPanic__function_'defaultPanic'__.integerOutOfBounds
            unreachable
          end
          local.get 6
          i32.const 98
          i32.add
          local.get 0
          local.get 39
          call $fmt.digits2
          local.get 37
          local.get 6
          i32.load16_u offset=98 align=1
          i32.store16 align=1
          br 1 (;@2;)
        end
        local.get 6
        i32.const 48
        i32.add
        local.get 35
        i32.add
        local.set 40
        local.get 6
        i32.load offset=88
        local.set 41
        block  ;; label = @3
          local.get 41
          i32.const 255
          i32.le_u
          i32.const 1
          i32.and
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOutOfBounds
          unreachable
        end
        i32.const 255
        local.set 42
        local.get 41
        local.get 42
        i32.and
        i32.const 48
        i32.add
        local.set 43
        block  ;; label = @3
          local.get 43
          local.get 43
          local.get 42
          i32.and
          i32.ne
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 40
        local.get 43
        i32.store8
      end
    end
    local.get 6
    i32.load offset=12
    local.set 44
    local.get 6
    i32.load offset=92
    local.set 45
    local.get 45
    local.get 6
    i32.const 48
    i32.add
    i32.add
    local.set 46
    block  ;; label = @1
      block  ;; label = @2
        local.get 45
        i32.const 33
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 45
      i32.const 33
      call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
      unreachable
    end
    i32.const 33
    local.get 45
    i32.sub
    local.set 47
    i32.const 33
    local.set 48
    block  ;; label = @1
      block  ;; label = @2
        local.get 48
        local.get 48
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      i32.const 33
      local.set 49
      local.get 0
      local.get 49
      local.get 49
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 6
    local.get 47
    i32.store offset=108
    local.get 6
    local.get 46
    i32.store offset=104
    local.get 6
    i32.load offset=108
    local.set 50
    local.get 0
    local.get 44
    local.get 6
    i32.load offset=104
    local.get 50
    local.get 5
    call $Io.Writer.alignBufferOptions
    local.set 51
    i32.const 0
    local.set 52
    block  ;; label = @1
      block  ;; label = @2
        local.get 51
        i32.const 65535
        i32.and
        local.get 52
        i32.const 65535
        i32.and
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $builtin.returnError
    end
    local.get 6
    i32.const 112
    i32.add
    global.set $m6__stack_pointer
    local.get 51
    return)
  (func $debug.assert (type $t_6_9) (param i32 i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i32.const 1
    i32.and
    i32.store8 offset=15
    block  ;; label = @1
      local.get 1
      i32.const -1
      i32.xor
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.reachedUnreachable
      unreachable
    end
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $debug.FullPanic__function_'defaultPanic'__.integerOutOfBounds (type $t_6_3) (param i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 1
    local.get 1
    global.set $m6__stack_pointer
    local.get 1
    i32.const 0
    i32.store offset=8
    local.get 1
    i32.const 1
    i32.store8 offset=12
    local.get 0
    i32.const 1048897
    i32.const 40
    local.get 1
    i32.const 8
    i32.add
    call $debug.defaultPanic
    unreachable)
  (func $fmt.digits2 (type $t_6_2) (param i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 2
    i32.store8 offset=13
    i32.const 255
    local.set 4
    local.get 2
    local.get 4
    i32.and
    local.set 5
    local.get 5
    local.get 5
    i32.add
    local.set 6
    block  ;; label = @1
      local.get 6
      local.get 6
      local.get 4
      i32.and
      i32.ne
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 6
    i32.const 255
    i32.and
    local.set 7
    local.get 7
    i32.const 1048998
    i32.add
    local.set 8
    local.get 7
    i32.const 2
    i32.add
    local.set 9
    block  ;; label = @1
      block  ;; label = @2
        local.get 9
        i32.const 201
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      local.get 9
      i32.const 201
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 3
    local.get 8
    i32.load16_u align=1
    i32.store16 offset=14
    local.get 0
    local.get 3
    i32.load16_u offset=14 align=1
    i32.store16 align=1
    local.get 3
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $debug.FullPanic__function_'defaultPanic'__.divideByZero (type $t_6_3) (param i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 1
    local.get 1
    global.set $m6__stack_pointer
    local.get 1
    i32.const 0
    i32.store offset=8
    local.get 1
    i32.const 1
    i32.store8 offset=12
    local.get 0
    i32.const 1048673
    i32.const 16
    local.get 1
    i32.const 8
    i32.add
    call $debug.defaultPanic
    unreachable)
  (func $fmt.digitToChar (type $t_6_6) (param i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 1
    i32.store8 offset=14
    local.get 3
    local.get 2
    i32.const 1
    i32.and
    i32.store8 offset=15
    i32.const 0
    local.set 4
    local.get 1
    i32.const 255
    i32.and
    local.get 4
    i32.const 255
    i32.and
    i32.ge_u
    local.set 5
    i32.const 9
    local.set 6
    block  ;; label = @1
      block  ;; label = @2
        local.get 5
        local.get 1
        i32.const 255
        i32.and
        local.get 6
        i32.const 255
        i32.and
        i32.le_u
        i32.and
        i32.const 1
        i32.and
        br_if 0 (;@2;)
        i32.const 10
        local.set 7
        local.get 1
        i32.const 255
        i32.and
        local.get 7
        i32.const 255
        i32.and
        i32.ge_u
        local.set 8
        i32.const 35
        local.set 9
        block  ;; label = @3
          local.get 8
          local.get 1
          i32.const 255
          i32.and
          local.get 9
          i32.const 255
          i32.and
          i32.le_u
          i32.and
          i32.const 1
          i32.and
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.reachedUnreachable
          unreachable
        end
        i32.const 1
        local.set 10
        block  ;; label = @3
          block  ;; label = @4
            local.get 2
            i32.const 1
            i32.and
            local.get 10
            i32.const 1
            i32.and
            i32.eq
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            i32.const 65
            local.set 11
            br 1 (;@3;)
          end
          i32.const 97
          local.set 11
        end
        local.get 11
        local.set 12
        i32.const 255
        local.set 13
        local.get 12
        local.get 13
        i32.and
        i32.const -10
        i32.add
        local.set 14
        block  ;; label = @3
          local.get 14
          local.get 14
          local.get 13
          i32.and
          i32.ne
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 14
        local.set 15
        i32.const 255
        local.set 16
        local.get 15
        local.get 16
        i32.and
        local.get 1
        local.get 16
        i32.and
        i32.add
        local.set 17
        block  ;; label = @3
          local.get 17
          local.get 17
          local.get 16
          i32.and
          i32.ne
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 17
        local.set 18
        br 1 (;@1;)
      end
      i32.const 255
      local.set 19
      local.get 1
      local.get 19
      i32.and
      i32.const 48
      i32.add
      local.set 20
      block  ;; label = @2
        local.get 20
        local.get 20
        local.get 19
        i32.and
        i32.ne
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      local.get 20
      local.set 18
    end
    local.get 18
    local.set 21
    local.get 3
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 21
    return)
  (func $Io.Writer.alignBufferOptions (type $t_6_7) (param i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i64 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 64
    i32.sub
    local.set 5
    local.get 5
    global.set $m6__stack_pointer
    local.get 3
    local.set 6
    local.get 2
    local.set 7
    local.get 5
    local.get 1
    i32.store offset=8
    local.get 5
    local.get 3
    i32.store offset=16
    local.get 5
    local.get 2
    i32.store offset=12
    local.get 5
    local.get 1
    i32.store offset=20
    local.get 5
    local.get 3
    i32.store offset=28
    local.get 5
    local.get 2
    i32.store offset=24
    i32.const 16
    local.set 8
    local.get 4
    local.get 8
    i32.add
    i32.load
    local.set 9
    local.get 8
    local.get 5
    i32.const 32
    i32.add
    i32.add
    local.get 9
    i32.store
    i32.const 8
    local.set 10
    local.get 4
    local.get 10
    i32.add
    i64.load align=4
    local.set 11
    local.get 10
    local.get 5
    i32.const 32
    i32.add
    i32.add
    local.get 11
    i64.store
    local.get 5
    local.get 4
    i64.load align=4
    i64.store offset=32
    local.get 5
    i32.load offset=20
    local.set 12
    local.get 5
    local.get 5
    i32.const 32
    i32.add
    i32.const 8
    i32.add
    i64.load align=4
    i64.store offset=56
    local.get 5
    i32.load8_u offset=60
    local.set 13
    i32.const 0
    local.set 14
    block  ;; label = @1
      block  ;; label = @2
        local.get 13
        i32.const 255
        i32.and
        local.get 14
        i32.const 255
        i32.and
        i32.ne
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 5
        i32.load offset=56
        local.set 15
        br 1 (;@1;)
      end
      local.get 5
      i32.load offset=28
      local.set 15
    end
    local.get 0
    local.get 12
    local.get 7
    local.get 6
    local.get 15
    local.get 5
    i32.load8_u offset=48
    local.get 5
    i32.load8_u offset=49
    call $Io.Writer.alignBuffer
    local.set 16
    i32.const 0
    local.set 17
    block  ;; label = @1
      block  ;; label = @2
        local.get 16
        i32.const 65535
        i32.and
        local.get 17
        i32.const 65535
        i32.and
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $builtin.returnError
    end
    local.get 5
    i32.const 64
    i32.add
    global.set $m6__stack_pointer
    local.get 16
    return)
  (func $Io.Writer.alignBuffer (type $t_6_10) (param i32 i32 i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 48
    i32.sub
    local.set 7
    local.get 7
    global.set $m6__stack_pointer
    local.get 3
    local.set 8
    local.get 2
    local.set 9
    local.get 7
    local.get 1
    i32.store offset=4
    local.get 7
    local.get 3
    i32.store offset=12
    local.get 7
    local.get 2
    i32.store offset=8
    local.get 7
    local.get 4
    i32.store offset=16
    local.get 7
    local.get 5
    i32.const 3
    i32.and
    i32.store8 offset=22
    local.get 7
    local.get 6
    i32.store8 offset=23
    local.get 7
    local.get 1
    i32.store offset=24
    local.get 7
    local.get 3
    i32.store offset=32
    local.get 7
    local.get 2
    i32.store offset=28
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 7
            i32.load offset=32
            local.get 4
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 4
            local.get 7
            i32.load offset=32
            i32.sub
            local.set 10
            local.get 10
            local.get 4
            i32.gt_u
            i32.const 1
            i32.and
            br_if 1 (;@3;)
            br 2 (;@2;)
          end
          i32.const 0
          local.set 11
          br 2 (;@1;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      local.get 10
      local.set 11
    end
    local.get 11
    local.set 12
    local.get 7
    local.get 12
    i32.store offset=36
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 12
              br_if 0 (;@5;)
              local.get 0
              local.get 7
              i32.load offset=24
              local.get 9
              local.get 8
              call $Io.Writer.writeAll
              local.set 13
              i32.const 0
              local.set 14
              local.get 13
              i32.const 65535
              i32.and
              local.get 14
              i32.const 65535
              i32.and
              i32.eq
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            br 3 (;@1;)
          end
          br 1 (;@2;)
        end
        local.get 0
        call $builtin.returnError
      end
      local.get 7
      i32.const 48
      i32.add
      global.set $m6__stack_pointer
      local.get 13
      return
    end
    local.get 5
    i32.const 2
    i32.add
    i32.const 3
    i32.and
    local.set 15
    block  ;; label = @1
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
                                                  block  ;; label = @24
                                                    block  ;; label = @25
                                                      block  ;; label = @26
                                                        block  ;; label = @27
                                                          i32.const 0
                                                          br_if 0 (;@27;)
                                                          block  ;; label = @28
                                                            block  ;; label = @29
                                                              block  ;; label = @30
                                                                local.get 15
                                                                br_table 2 (;@28;) 3 (;@27;) 0 (;@30;) 1 (;@29;) 2 (;@28;)
                                                              end
                                                              local.get 0
                                                              local.get 7
                                                              i32.load offset=24
                                                              local.get 9
                                                              local.get 8
                                                              call $Io.Writer.writeAll
                                                              local.set 16
                                                              i32.const 0
                                                              local.set 17
                                                              local.get 16
                                                              i32.const 65535
                                                              i32.and
                                                              local.get 17
                                                              i32.const 65535
                                                              i32.and
                                                              i32.ne
                                                              i32.const 1
                                                              i32.and
                                                              br_if 3 (;@26;)
                                                              br 4 (;@25;)
                                                            end
                                                            local.get 12
                                                            i32.const 1
                                                            i32.shr_u
                                                            local.set 18
                                                            local.get 7
                                                            local.get 18
                                                            i32.store offset=40
                                                            local.get 12
                                                            i32.const 1
                                                            i32.add
                                                            local.set 19
                                                            local.get 19
                                                            i32.eqz
                                                            i32.const 1
                                                            i32.and
                                                            br_if 10 (;@18;)
                                                            br 11 (;@17;)
                                                          end
                                                          local.get 0
                                                          local.get 7
                                                          i32.load offset=24
                                                          local.get 6
                                                          local.get 12
                                                          call $Io.Writer.splatByteAll
                                                          local.set 20
                                                          i32.const 0
                                                          local.set 21
                                                          local.get 20
                                                          i32.const 65535
                                                          i32.and
                                                          local.get 21
                                                          i32.const 65535
                                                          i32.and
                                                          i32.ne
                                                          i32.const 1
                                                          i32.and
                                                          br_if 11 (;@16;)
                                                          br 12 (;@15;)
                                                        end
                                                        local.get 0
                                                        call $debug.FullPanic__function_'defaultPanic'__.corruptSwitch
                                                        unreachable
                                                      end
                                                      i32.const 0
                                                      local.set 22
                                                      local.get 16
                                                      i32.const 65535
                                                      i32.and
                                                      local.get 22
                                                      i32.const 65535
                                                      i32.and
                                                      i32.eq
                                                      i32.const 1
                                                      i32.and
                                                      br_if 1 (;@24;)
                                                      br 2 (;@23;)
                                                    end
                                                    local.get 0
                                                    local.get 7
                                                    i32.load offset=24
                                                    local.get 6
                                                    local.get 12
                                                    call $Io.Writer.splatByteAll
                                                    local.set 23
                                                    i32.const 0
                                                    local.set 24
                                                    local.get 23
                                                    i32.const 65535
                                                    i32.and
                                                    local.get 24
                                                    i32.const 65535
                                                    i32.and
                                                    i32.ne
                                                    i32.const 1
                                                    i32.and
                                                    br_if 2 (;@22;)
                                                    br 3 (;@21;)
                                                  end
                                                  br 21 (;@2;)
                                                end
                                                local.get 0
                                                call $builtin.returnError
                                                br 20 (;@2;)
                                              end
                                              i32.const 0
                                              local.set 25
                                              local.get 23
                                              i32.const 65535
                                              i32.and
                                              local.get 25
                                              i32.const 65535
                                              i32.and
                                              i32.eq
                                              i32.const 1
                                              i32.and
                                              br_if 1 (;@20;)
                                              br 2 (;@19;)
                                            end
                                            br 19 (;@1;)
                                          end
                                          br 16 (;@3;)
                                        end
                                        local.get 0
                                        call $builtin.returnError
                                        br 15 (;@3;)
                                      end
                                      local.get 0
                                      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
                                      unreachable
                                    end
                                    local.get 19
                                    i32.const 1
                                    i32.shr_u
                                    local.set 26
                                    local.get 7
                                    local.get 26
                                    i32.store offset=44
                                    local.get 0
                                    local.get 7
                                    i32.load offset=24
                                    local.get 6
                                    local.get 18
                                    call $Io.Writer.splatByteAll
                                    local.set 27
                                    i32.const 0
                                    local.set 28
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
                                                          local.get 27
                                                          i32.const 65535
                                                          i32.and
                                                          local.get 28
                                                          i32.const 65535
                                                          i32.and
                                                          i32.ne
                                                          i32.const 1
                                                          i32.and
                                                          i32.eqz
                                                          br_if 0 (;@27;)
                                                          i32.const 0
                                                          local.set 29
                                                          local.get 27
                                                          i32.const 65535
                                                          i32.and
                                                          local.get 29
                                                          i32.const 65535
                                                          i32.and
                                                          i32.eq
                                                          i32.const 1
                                                          i32.and
                                                          br_if 1 (;@26;)
                                                          br 2 (;@25;)
                                                        end
                                                        local.get 0
                                                        local.get 7
                                                        i32.load offset=24
                                                        local.get 9
                                                        local.get 8
                                                        call $Io.Writer.writeAll
                                                        local.set 30
                                                        i32.const 0
                                                        local.set 31
                                                        local.get 30
                                                        i32.const 65535
                                                        i32.and
                                                        local.get 31
                                                        i32.const 65535
                                                        i32.and
                                                        i32.ne
                                                        i32.const 1
                                                        i32.and
                                                        br_if 2 (;@24;)
                                                        br 3 (;@23;)
                                                      end
                                                      br 21 (;@4;)
                                                    end
                                                    local.get 0
                                                    call $builtin.returnError
                                                    br 20 (;@4;)
                                                  end
                                                  i32.const 0
                                                  local.set 32
                                                  local.get 30
                                                  i32.const 65535
                                                  i32.and
                                                  local.get 32
                                                  i32.const 65535
                                                  i32.and
                                                  i32.eq
                                                  i32.const 1
                                                  i32.and
                                                  br_if 1 (;@22;)
                                                  br 2 (;@21;)
                                                end
                                                local.get 0
                                                local.get 7
                                                i32.load offset=24
                                                local.get 6
                                                local.get 26
                                                call $Io.Writer.splatByteAll
                                                local.set 33
                                                i32.const 0
                                                local.set 34
                                                local.get 33
                                                i32.const 65535
                                                i32.and
                                                local.get 34
                                                i32.const 65535
                                                i32.and
                                                i32.ne
                                                i32.const 1
                                                i32.and
                                                br_if 2 (;@20;)
                                                br 3 (;@19;)
                                              end
                                              br 16 (;@5;)
                                            end
                                            local.get 0
                                            call $builtin.returnError
                                            br 15 (;@5;)
                                          end
                                          i32.const 0
                                          local.set 35
                                          local.get 33
                                          i32.const 65535
                                          i32.and
                                          local.get 35
                                          i32.const 65535
                                          i32.and
                                          i32.eq
                                          i32.const 1
                                          i32.and
                                          br_if 1 (;@18;)
                                          br 2 (;@17;)
                                        end
                                        br 17 (;@1;)
                                      end
                                      br 11 (;@6;)
                                    end
                                    local.get 0
                                    call $builtin.returnError
                                    br 10 (;@6;)
                                  end
                                  i32.const 0
                                  local.set 36
                                  local.get 20
                                  i32.const 65535
                                  i32.and
                                  local.get 36
                                  i32.const 65535
                                  i32.and
                                  i32.eq
                                  i32.const 1
                                  i32.and
                                  br_if 1 (;@14;)
                                  br 2 (;@13;)
                                end
                                local.get 0
                                local.get 7
                                i32.load offset=24
                                local.get 9
                                local.get 8
                                call $Io.Writer.writeAll
                                local.set 37
                                i32.const 0
                                local.set 38
                                local.get 37
                                i32.const 65535
                                i32.and
                                local.get 38
                                i32.const 65535
                                i32.and
                                i32.ne
                                i32.const 1
                                i32.and
                                br_if 2 (;@12;)
                                br 3 (;@11;)
                              end
                              br 6 (;@7;)
                            end
                            local.get 0
                            call $builtin.returnError
                            br 5 (;@7;)
                          end
                          i32.const 0
                          local.set 39
                          local.get 37
                          i32.const 65535
                          i32.and
                          local.get 39
                          i32.const 65535
                          i32.and
                          i32.eq
                          i32.const 1
                          i32.and
                          br_if 1 (;@10;)
                          br 2 (;@9;)
                        end
                        br 9 (;@1;)
                      end
                      br 1 (;@8;)
                    end
                    local.get 0
                    call $builtin.returnError
                  end
                  local.get 7
                  i32.const 48
                  i32.add
                  global.set $m6__stack_pointer
                  local.get 37
                  return
                end
                local.get 7
                i32.const 48
                i32.add
                global.set $m6__stack_pointer
                local.get 20
                return
              end
              local.get 7
              i32.const 48
              i32.add
              global.set $m6__stack_pointer
              local.get 33
              return
            end
            local.get 7
            i32.const 48
            i32.add
            global.set $m6__stack_pointer
            local.get 30
            return
          end
          local.get 7
          i32.const 48
          i32.add
          global.set $m6__stack_pointer
          local.get 27
          return
        end
        local.get 7
        i32.const 48
        i32.add
        global.set $m6__stack_pointer
        local.get 23
        return
      end
      local.get 7
      i32.const 48
      i32.add
      global.set $m6__stack_pointer
      local.get 16
      return
    end
    i32.const 0
    local.set 40
    local.get 7
    i32.const 48
    i32.add
    global.set $m6__stack_pointer
    local.get 40
    return)
  (func $Io.Writer.splatByteAll (type $t_6_4) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 32
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 1
    i32.store offset=4
    local.get 4
    local.get 2
    i32.store8 offset=11
    local.get 4
    local.get 3
    i32.store offset=12
    local.get 4
    local.get 1
    i32.store offset=16
    local.get 4
    local.get 3
    i32.store offset=20
    block  ;; label = @1
      loop  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    block  ;; label = @9
                      block  ;; label = @10
                        block  ;; label = @11
                          local.get 4
                          i32.load offset=20
                          i32.const 0
                          i32.gt_u
                          i32.const 1
                          i32.and
                          i32.eqz
                          br_if 0 (;@11;)
                          local.get 4
                          i32.load offset=20
                          local.set 5
                          local.get 4
                          i32.load offset=16
                          local.set 6
                          local.get 4
                          i32.load offset=20
                          local.set 7
                          local.get 4
                          i32.const 24
                          i32.add
                          local.get 0
                          local.get 6
                          local.get 2
                          local.get 7
                          call $Io.Writer.splatByte
                          local.get 4
                          i32.load16_u offset=28
                          local.set 8
                          i32.const 0
                          local.set 9
                          local.get 8
                          i32.const 65535
                          i32.and
                          local.get 9
                          i32.const 65535
                          i32.and
                          i32.ne
                          i32.const 1
                          i32.and
                          br_if 1 (;@10;)
                          br 2 (;@9;)
                        end
                        br 9 (;@1;)
                      end
                      local.get 4
                      i32.load16_u offset=28
                      local.set 10
                      i32.const 0
                      local.set 11
                      local.get 10
                      i32.const 65535
                      i32.and
                      local.get 11
                      i32.const 65535
                      i32.and
                      i32.eq
                      i32.const 1
                      i32.and
                      br_if 1 (;@8;)
                      br 2 (;@7;)
                    end
                    local.get 5
                    local.get 4
                    i32.load offset=24
                    i32.sub
                    local.set 12
                    local.get 12
                    local.get 5
                    i32.gt_u
                    i32.const 1
                    i32.and
                    br_if 2 (;@6;)
                    br 3 (;@5;)
                  end
                  br 3 (;@4;)
                end
                local.get 0
                call $builtin.returnError
                br 2 (;@4;)
              end
              local.get 0
              call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
              unreachable
            end
            local.get 4
            local.get 12
            i32.store offset=20
            br 1 (;@3;)
          end
          local.get 4
          i32.const 32
          i32.add
          global.set $m6__stack_pointer
          local.get 10
          return
        end
        br 0 (;@2;)
      end
    end
    i32.const 0
    local.set 13
    local.get 4
    i32.const 32
    i32.add
    global.set $m6__stack_pointer
    local.get 13
    return)
  (func $debug.FullPanic__function_'defaultPanic'__.corruptSwitch (type $t_6_3) (param i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 1
    local.get 1
    global.set $m6__stack_pointer
    local.get 1
    i32.const 0
    i32.store offset=8
    local.get 1
    i32.const 1
    i32.store8 offset=12
    local.get 0
    i32.const 1048873
    i32.const 23
    local.get 1
    i32.const 8
    i32.add
    call $debug.defaultPanic
    unreachable)
  (func $debug.FullPanic__function_'defaultPanic'__.reachedUnreachable (type $t_6_3) (param i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 1
    local.get 1
    global.set $m6__stack_pointer
    local.get 1
    i32.const 0
    i32.store offset=8
    local.get 1
    i32.const 1
    i32.store8 offset=12
    local.get 0
    i32.const 1048938
    i32.const 24
    local.get 1
    i32.const 8
    i32.add
    call $debug.defaultPanic
    unreachable)
  (func $Io.Writer.splatByte (type $t_6_5) (param i32 i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 48
    i32.sub
    local.set 5
    local.get 5
    global.set $m6__stack_pointer
    local.get 5
    local.get 2
    i32.store offset=4
    local.get 5
    local.get 3
    i32.store8 offset=11
    local.get 5
    local.get 4
    i32.store offset=12
    local.get 5
    local.get 2
    i32.store offset=16
    local.get 5
    i32.load offset=16
    i32.load offset=12
    local.set 6
    local.get 6
    local.get 4
    i32.add
    local.set 7
    block  ;; label = @1
      local.get 7
      local.get 6
      i32.lt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 7
              local.get 5
              i32.load offset=16
              i32.load offset=8
              i32.le_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              local.get 5
              i32.load offset=16
              local.set 8
              local.get 8
              i32.load offset=12
              local.set 9
              local.get 8
              i32.load offset=8
              local.set 10
              local.get 9
              local.get 8
              i32.load offset=4
              i32.add
              local.set 11
              local.get 9
              local.get 4
              i32.add
              local.set 12
              local.get 12
              local.get 10
              i32.le_u
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            br 3 (;@1;)
          end
          br 1 (;@2;)
        end
        local.get 1
        local.get 12
        local.get 10
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 4
      local.set 13
      local.get 11
      local.set 14
      block  ;; label = @2
        local.get 13
        i32.eqz
        br_if 0 (;@2;)
        local.get 14
        local.get 3
        local.get 13
        memory.fill
      end
      local.get 5
      i32.load offset=16
      local.set 15
      local.get 15
      i32.const 12
      i32.add
      local.set 16
      local.get 15
      i32.load offset=12
      local.set 17
      local.get 17
      local.get 4
      i32.add
      local.set 18
      block  ;; label = @2
        local.get 18
        local.get 17
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 1
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      local.get 16
      local.get 18
      i32.store
      local.get 5
      i32.const 0
      i32.store16 offset=24
      local.get 5
      local.get 4
      i32.store offset=20
      local.get 5
      i32.load16_u offset=24
      local.set 19
      i32.const 0
      local.set 20
      block  ;; label = @2
        block  ;; label = @3
          local.get 19
          i32.const 65535
          i32.and
          local.get 20
          i32.const 65535
          i32.and
          i32.eq
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 1
        call $builtin.returnError
      end
      local.get 0
      local.get 5
      i64.load offset=20 align=4
      i64.store align=4
      local.get 5
      i32.const 48
      i32.add
      global.set $m6__stack_pointer
      return
    end
    local.get 5
    local.get 3
    i32.store8 offset=31
    i32.const 1
    local.set 21
    local.get 5
    local.get 21
    i32.store offset=36
    local.get 5
    local.get 5
    i32.const 31
    i32.add
    i32.store offset=32
    local.get 5
    i32.const 32
    i32.add
    local.set 22
    local.get 5
    i32.const 40
    i32.add
    local.get 1
    local.get 2
    local.get 22
    local.get 21
    local.get 4
    call $Io.Writer.writeSplat
    local.get 5
    i32.load16_u offset=44
    local.set 23
    i32.const 0
    local.set 24
    block  ;; label = @1
      block  ;; label = @2
        local.get 23
        i32.const 65535
        i32.and
        local.get 24
        i32.const 65535
        i32.and
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      call $builtin.returnError
    end
    local.get 0
    local.get 5
    i64.load offset=40 align=4
    i64.store align=4
    local.get 5
    i32.const 48
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $Io.Writer.writeSplat (type $t_6_0) (param i32 i32 i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 112
    i32.sub
    local.set 6
    local.get 6
    global.set $m6__stack_pointer
    local.get 4
    local.set 7
    local.get 3
    local.set 8
    local.get 6
    local.get 2
    i32.store offset=8
    local.get 6
    local.get 4
    i32.store offset=16
    local.get 6
    local.get 3
    i32.store offset=12
    local.get 6
    local.get 5
    i32.store offset=20
    local.get 6
    local.get 2
    i32.store offset=24
    local.get 6
    local.get 4
    i32.store offset=32
    local.get 6
    local.get 3
    i32.store offset=28
    local.get 1
    local.get 6
    i32.load offset=32
    i32.const 0
    i32.ne
    call $debug.assert
    local.get 6
    i32.load offset=24
    local.set 9
    local.get 9
    i32.load offset=4
    local.set 10
    local.get 9
    i32.load offset=8
    local.set 11
    local.get 6
    local.get 11
    i32.store offset=40
    local.get 6
    local.get 10
    i32.store offset=36
    local.get 6
    local.get 11
    i32.store offset=48
    local.get 6
    local.get 10
    i32.store offset=44
    local.get 1
    local.get 8
    local.get 7
    local.get 5
    call $Io.Writer.countSplat
    local.set 12
    local.get 6
    local.get 12
    i32.store offset=52
    local.get 6
    i32.load offset=24
    i32.load offset=12
    local.set 13
    local.get 13
    local.get 12
    i32.add
    local.set 14
    block  ;; label = @1
      local.get 14
      local.get 13
      i32.lt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 14
              local.get 6
              i32.load offset=40
              i32.gt_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              local.get 6
              i32.load offset=24
              i32.load
              i32.load
              local.set 15
              local.get 6
              i32.const 56
              i32.add
              local.get 1
              local.get 2
              local.get 8
              local.get 7
              local.get 5
              local.get 15
              call_indirect (type $t_6_0)
              local.get 6
              i32.load16_u offset=60
              local.set 16
              i32.const 0
              local.set 17
              local.get 16
              i32.const 65535
              i32.and
              local.get 17
              i32.const 65535
              i32.and
              i32.eq
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            br 3 (;@1;)
          end
          br 1 (;@2;)
        end
        local.get 1
        call $builtin.returnError
      end
      local.get 0
      local.get 6
      i64.load offset=56 align=4
      i64.store align=4
      local.get 6
      i32.const 112
      i32.add
      global.set $m6__stack_pointer
      return
    end
    local.get 6
    i32.const 0
    i32.store offset=64
    local.get 6
    i32.load offset=32
    local.set 18
    local.get 18
    i32.const -1
    i32.add
    local.set 19
    block  ;; label = @1
      local.get 19
      local.get 18
      i32.gt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 19
    local.set 20
    local.get 6
    i32.load offset=32
    local.set 21
    local.get 6
    i32.load offset=28
    local.set 22
    local.get 20
    i32.const 0
    i32.sub
    local.set 23
    block  ;; label = @1
      block  ;; label = @2
        local.get 20
        local.get 21
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      local.get 20
      local.get 21
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 23
    local.set 24
    local.get 22
    local.set 25
    block  ;; label = @1
      loop  ;; label = @2
        local.get 6
        i32.load offset=64
        local.set 26
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 26
                local.get 24
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 25
                local.get 26
                i32.const 3
                i32.shl
                i32.add
                local.set 27
                local.get 27
                i32.load
                local.set 28
                local.get 27
                i32.load offset=4
                local.set 29
                local.get 6
                local.get 29
                i32.store offset=72
                local.get 6
                local.get 28
                i32.store offset=68
                local.get 6
                local.get 29
                i32.store offset=80
                local.get 6
                local.get 28
                i32.store offset=76
                local.get 6
                i32.load offset=24
                i32.load offset=12
                local.set 30
                local.get 6
                i32.load offset=72
                local.set 31
                local.get 6
                i32.load offset=40
                local.set 32
                local.get 30
                local.get 6
                i32.load offset=36
                i32.add
                local.set 33
                local.get 30
                local.get 31
                i32.add
                local.set 34
                local.get 34
                local.get 32
                i32.le_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 1
          local.get 34
          local.get 32
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 31
        local.set 35
        local.get 33
        local.set 36
        block  ;; label = @3
          block  ;; label = @4
            local.get 35
            local.get 29
            i32.eq
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 1
          call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
          unreachable
        end
        local.get 28
        local.get 35
        i32.add
        local.set 37
        local.get 36
        local.get 35
        i32.add
        local.set 38
        block  ;; label = @3
          block  ;; label = @4
            local.get 36
            local.get 37
            i32.ge_u
            local.get 28
            local.get 38
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 1
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        block  ;; label = @3
          local.get 35
          i32.eqz
          br_if 0 (;@3;)
          local.get 36
          local.get 28
          local.get 35
          memory.copy
        end
        local.get 6
        i32.load offset=24
        local.set 39
        local.get 39
        i32.const 12
        i32.add
        local.set 40
        local.get 39
        i32.load offset=12
        local.set 41
        local.get 41
        local.get 6
        i32.load offset=72
        i32.add
        local.set 42
        block  ;; label = @3
          local.get 42
          local.get 41
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 1
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 40
        local.get 42
        i32.store
        local.get 6
        local.get 26
        i32.const 1
        i32.add
        i32.store offset=64
        br 0 (;@2;)
      end
    end
    local.get 6
    i32.load offset=32
    local.set 43
    local.get 43
    i32.const -1
    i32.add
    local.set 44
    block  ;; label = @1
      local.get 44
      local.get 43
      i32.gt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 44
    local.set 45
    local.get 6
    i32.load offset=32
    local.set 46
    local.get 6
    i32.load offset=28
    local.set 47
    block  ;; label = @1
      block  ;; label = @2
        local.get 45
        local.get 46
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      local.get 45
      local.get 46
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 47
    local.get 45
    i32.const 3
    i32.shl
    i32.add
    local.set 48
    local.get 48
    i32.load
    local.set 49
    local.get 48
    i32.load offset=4
    local.set 50
    local.get 6
    local.get 50
    i32.store offset=88
    local.get 6
    local.get 49
    i32.store offset=84
    local.get 6
    local.get 50
    i32.store offset=96
    local.get 6
    local.get 49
    i32.store offset=92
    local.get 6
    i32.load offset=88
    local.set 51
    local.get 51
    i32.const 1
    i32.gt_u
    drop
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 51
                    br_table 0 (;@8;) 1 (;@7;) 2 (;@6;)
                  end
                  br 6 (;@1;)
                end
                local.get 6
                i32.load offset=24
                i32.load offset=12
                local.set 52
                local.get 6
                i32.load offset=40
                local.set 53
                local.get 52
                local.get 6
                i32.load offset=36
                i32.add
                local.set 54
                local.get 52
                local.get 5
                i32.add
                local.set 55
                local.get 55
                local.get 53
                i32.le_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              local.get 6
              i32.const 0
              i32.store offset=100
              br 2 (;@3;)
            end
            br 2 (;@2;)
          end
          local.get 1
          local.get 55
          local.get 53
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        block  ;; label = @3
          loop  ;; label = @4
            local.get 6
            i32.load offset=100
            local.set 56
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  block  ;; label = @8
                    local.get 56
                    local.get 5
                    i32.lt_u
                    i32.const 1
                    i32.and
                    i32.eqz
                    br_if 0 (;@8;)
                    local.get 6
                    i32.load offset=24
                    i32.load offset=12
                    local.set 57
                    local.get 6
                    i32.load offset=88
                    local.set 58
                    local.get 6
                    i32.load offset=40
                    local.set 59
                    local.get 57
                    local.get 6
                    i32.load offset=36
                    i32.add
                    local.set 60
                    local.get 57
                    local.get 58
                    i32.add
                    local.set 61
                    local.get 61
                    local.get 59
                    i32.le_u
                    i32.const 1
                    i32.and
                    br_if 1 (;@7;)
                    br 2 (;@6;)
                  end
                  br 4 (;@3;)
                end
                br 1 (;@5;)
              end
              local.get 1
              local.get 61
              local.get 59
              call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
              unreachable
            end
            local.get 58
            local.set 62
            local.get 60
            local.set 63
            block  ;; label = @5
              block  ;; label = @6
                local.get 62
                local.get 50
                i32.eq
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                br 1 (;@5;)
              end
              local.get 1
              call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
              unreachable
            end
            local.get 49
            local.get 62
            i32.add
            local.set 64
            local.get 63
            local.get 62
            i32.add
            local.set 65
            block  ;; label = @5
              block  ;; label = @6
                local.get 63
                local.get 64
                i32.ge_u
                local.get 49
                local.get 65
                i32.ge_u
                i32.or
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                br 1 (;@5;)
              end
              local.get 1
              call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
              unreachable
            end
            block  ;; label = @5
              local.get 62
              i32.eqz
              br_if 0 (;@5;)
              local.get 63
              local.get 49
              local.get 62
              memory.copy
            end
            local.get 6
            i32.load offset=24
            local.set 66
            local.get 66
            i32.const 12
            i32.add
            local.set 67
            local.get 66
            i32.load offset=12
            local.set 68
            local.get 68
            local.get 6
            i32.load offset=88
            i32.add
            local.set 69
            block  ;; label = @5
              local.get 69
              local.get 68
              i32.lt_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              local.get 1
              call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
              unreachable
            end
            local.get 67
            local.get 69
            i32.store
            local.get 6
            local.get 56
            i32.const 1
            i32.add
            i32.store offset=100
            br 0 (;@4;)
          end
        end
        br 1 (;@1;)
      end
      local.get 5
      local.set 70
      local.get 54
      local.set 71
      local.get 6
      i32.load offset=88
      local.set 72
      local.get 6
      i32.load offset=84
      local.set 73
      block  ;; label = @2
        block  ;; label = @3
          i32.const 0
          local.get 72
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 1
        i32.const 0
        local.get 72
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 73
      i32.load8_u
      local.set 74
      block  ;; label = @2
        local.get 70
        i32.eqz
        br_if 0 (;@2;)
        local.get 71
        local.get 74
        local.get 70
        memory.fill
      end
      local.get 6
      i32.load offset=24
      local.set 75
      local.get 75
      i32.const 12
      i32.add
      local.set 76
      local.get 75
      i32.load offset=12
      local.set 77
      local.get 77
      local.get 5
      i32.add
      local.set 78
      block  ;; label = @2
        local.get 78
        local.get 77
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 1
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      local.get 76
      local.get 78
      i32.store
    end
    local.get 6
    i32.const 0
    i32.store16 offset=108
    local.get 6
    local.get 12
    i32.store offset=104
    local.get 6
    i32.load16_u offset=108
    local.set 79
    i32.const 0
    local.set 80
    block  ;; label = @1
      block  ;; label = @2
        local.get 79
        i32.const 65535
        i32.and
        local.get 80
        i32.const 65535
        i32.and
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      call $builtin.returnError
    end
    local.get 0
    local.get 6
    i64.load offset=104 align=4
    i64.store align=4
    local.get 6
    i32.const 112
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $Io.Writer.countSplat (type $t_6_4) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 48
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 2
    i32.store offset=8
    local.get 4
    local.get 1
    i32.store offset=4
    local.get 4
    local.get 3
    i32.store offset=12
    local.get 4
    local.get 2
    i32.store offset=20
    local.get 4
    local.get 1
    i32.store offset=16
    local.get 4
    i32.const 0
    i32.store offset=24
    local.get 4
    i32.const 0
    i32.store offset=28
    local.get 4
    i32.load offset=20
    local.set 5
    local.get 5
    i32.const -1
    i32.add
    local.set 6
    block  ;; label = @1
      local.get 6
      local.get 5
      i32.gt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 6
    local.set 7
    local.get 4
    i32.load offset=20
    local.set 8
    local.get 4
    i32.load offset=16
    local.set 9
    local.get 7
    i32.const 0
    i32.sub
    local.set 10
    block  ;; label = @1
      block  ;; label = @2
        local.get 7
        local.get 8
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 7
      local.get 8
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 10
    local.set 11
    local.get 9
    local.set 12
    block  ;; label = @1
      loop  ;; label = @2
        local.get 4
        i32.load offset=28
        local.set 13
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 13
              local.get 11
              i32.lt_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              local.get 12
              local.get 13
              i32.const 3
              i32.shl
              i32.add
              local.set 14
              local.get 14
              i32.load
              local.set 15
              local.get 14
              i32.load offset=4
              local.set 16
              local.get 4
              local.get 16
              i32.store offset=36
              local.get 4
              local.get 15
              i32.store offset=32
              local.get 4
              local.get 16
              i32.store offset=44
              local.get 4
              local.get 15
              i32.store offset=40
              local.get 4
              i32.load offset=24
              local.set 17
              local.get 17
              local.get 4
              i32.load offset=36
              i32.add
              local.set 18
              local.get 18
              local.get 17
              i32.lt_u
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            br 3 (;@1;)
          end
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 18
        i32.store offset=24
        local.get 4
        local.get 13
        i32.const 1
        i32.add
        i32.store offset=28
        br 0 (;@2;)
      end
    end
    local.get 4
    i32.load offset=24
    local.set 19
    local.get 4
    i32.load offset=20
    local.set 20
    local.get 20
    i32.const -1
    i32.add
    local.set 21
    block  ;; label = @1
      local.get 21
      local.get 20
      i32.gt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 21
    local.set 22
    local.get 4
    i32.load offset=20
    local.set 23
    local.get 4
    i32.load offset=16
    local.set 24
    block  ;; label = @1
      block  ;; label = @2
        local.get 22
        local.get 23
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 22
      local.get 23
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 24
    local.get 22
    i32.const 3
    i32.shl
    i32.add
    i32.load offset=4
    local.set 25
    local.get 3
    i64.extend_i32_u
    local.get 25
    i64.extend_i32_u
    i64.mul
    local.set 26
    local.get 26
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    i32.const 0
    i32.ne
    local.set 27
    local.get 26
    i32.wrap_i64
    local.set 28
    block  ;; label = @1
      local.get 27
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 19
    local.get 28
    i32.add
    local.set 29
    block  ;; label = @1
      local.get 29
      local.get 19
      i32.lt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 4
    local.get 29
    i32.store offset=24
    local.get 4
    i32.load offset=24
    local.set 30
    local.get 4
    i32.const 48
    i32.add
    global.set $m6__stack_pointer
    local.get 30
    return)
  (func $Io.Writer.unimplementedSendFile (type $t_6_5) (param i32 i32 i32 i32 i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 5
    local.get 5
    global.set $m6__stack_pointer
    local.get 5
    local.get 2
    i32.store offset=4
    local.get 5
    local.get 3
    i32.store offset=8
    local.get 5
    local.get 4
    i32.store offset=12
    local.get 1
    call $builtin.returnError
    local.get 0
    i32.const 0
    i64.load offset=1049384 align=4
    i64.store align=4
    local.get 5
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $Io.Writer.failingRebase (type $t_6_4) (param i32 i32 i32 i32) (result i32)
    (local i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 1
    i32.store offset=4
    local.get 4
    local.get 2
    i32.store offset=8
    local.get 4
    local.get 3
    i32.store offset=12
    local.get 0
    call $builtin.returnError
    i32.const 1
    local.set 5
    local.get 4
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 5
    return)
  (func $Io.Writer.noopFlush (type $t_6_11) (param i32 i32) (result i32)
    (local i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i32.store offset=12
    i32.const 0
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $Io.Writer.fixedDrain (type $t_6_0) (param i32 i32 i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 128
    i32.sub
    local.set 6
    local.get 6
    global.set $m6__stack_pointer
    local.get 6
    local.get 2
    i32.store
    local.get 6
    local.get 4
    i32.store offset=8
    local.get 6
    local.get 3
    i32.store offset=4
    local.get 6
    local.get 5
    i32.store offset=12
    local.get 6
    local.get 2
    i32.store offset=16
    local.get 6
    local.get 4
    i32.store offset=24
    local.get 6
    local.get 3
    i32.store offset=20
    block  ;; label = @1
      local.get 6
      i32.load offset=24
      br_if 0 (;@1;)
      local.get 0
      i64.const 0
      i64.store align=4
      local.get 6
      i32.const 128
      i32.add
      global.set $m6__stack_pointer
      return
    end
    local.get 6
    i32.const 0
    i32.store offset=28
    local.get 6
    i32.load offset=24
    local.set 7
    local.get 7
    i32.const -1
    i32.add
    local.set 8
    block  ;; label = @1
      local.get 8
      local.get 7
      i32.gt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 8
    local.set 9
    local.get 6
    i32.load offset=24
    local.set 10
    local.get 6
    i32.load offset=20
    local.set 11
    local.get 9
    i32.const 0
    i32.sub
    local.set 12
    block  ;; label = @1
      block  ;; label = @2
        local.get 9
        local.get 10
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      local.get 9
      local.get 10
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 12
    local.set 13
    local.get 11
    local.set 14
    block  ;; label = @1
      loop  ;; label = @2
        local.get 6
        i32.load offset=28
        local.set 15
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 15
                local.get 13
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 14
                local.get 15
                i32.const 3
                i32.shl
                i32.add
                local.set 16
                local.get 16
                i32.load
                local.set 17
                local.get 16
                i32.load offset=4
                local.set 18
                local.get 6
                local.get 18
                i32.store offset=36
                local.get 6
                local.get 17
                i32.store offset=32
                local.get 6
                local.get 18
                i32.store offset=44
                local.get 6
                local.get 17
                i32.store offset=40
                local.get 6
                i32.load offset=16
                local.set 19
                local.get 19
                i32.load offset=12
                local.set 20
                local.get 19
                i32.load offset=8
                local.set 21
                local.get 20
                local.get 19
                i32.load offset=4
                i32.add
                local.set 22
                local.get 20
                local.get 21
                i32.le_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 1
          local.get 20
          local.get 21
          call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
          unreachable
        end
        local.get 21
        local.get 20
        i32.sub
        local.set 23
        block  ;; label = @3
          block  ;; label = @4
            local.get 21
            local.get 21
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 1
          local.get 21
          local.get 21
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 6
        local.get 23
        i32.store offset=52
        local.get 6
        local.get 22
        i32.store offset=48
        local.get 6
        local.get 23
        i32.store offset=60
        local.get 6
        local.get 22
        i32.store offset=56
        local.get 6
        i32.load offset=36
        local.set 24
        local.get 6
        i32.load offset=52
        local.set 25
        local.get 24
        local.get 25
        local.get 24
        local.get 25
        i32.lt_u
        select
        local.set 26
        local.get 6
        local.get 26
        i32.store offset=64
        local.get 6
        i32.load offset=52
        local.set 27
        local.get 6
        i32.load offset=48
        local.set 28
        local.get 26
        i32.const 0
        i32.sub
        local.set 29
        block  ;; label = @3
          block  ;; label = @4
            local.get 26
            local.get 27
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 1
          local.get 26
          local.get 27
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 29
        local.set 30
        local.get 28
        local.set 31
        local.get 6
        i32.load offset=36
        local.set 32
        local.get 6
        i32.load offset=32
        local.set 33
        local.get 26
        i32.const 0
        i32.sub
        local.set 34
        block  ;; label = @3
          block  ;; label = @4
            local.get 26
            local.get 32
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 1
          local.get 26
          local.get 32
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 34
        local.set 35
        local.get 33
        local.set 36
        block  ;; label = @3
          block  ;; label = @4
            local.get 30
            local.get 35
            i32.eq
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 1
          call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
          unreachable
        end
        local.get 36
        local.get 30
        i32.add
        local.set 37
        local.get 31
        local.get 30
        i32.add
        local.set 38
        block  ;; label = @3
          block  ;; label = @4
            local.get 31
            local.get 37
            i32.ge_u
            local.get 36
            local.get 38
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 1
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        block  ;; label = @3
          local.get 30
          i32.eqz
          br_if 0 (;@3;)
          local.get 31
          local.get 36
          local.get 30
          memory.copy
        end
        local.get 6
        i32.load offset=16
        local.set 39
        local.get 39
        i32.const 12
        i32.add
        local.set 40
        local.get 39
        i32.load offset=12
        local.set 41
        local.get 41
        local.get 26
        i32.add
        local.set 42
        block  ;; label = @3
          local.get 42
          local.get 41
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 1
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 40
        local.get 42
        i32.store
        block  ;; label = @3
          local.get 6
          i32.load offset=36
          local.get 6
          i32.load offset=52
          i32.gt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 1
          call $builtin.returnError
          local.get 0
          i32.const 0
          i64.load offset=1049392 align=4
          i64.store align=4
          local.get 6
          i32.const 128
          i32.add
          global.set $m6__stack_pointer
          return
        end
        local.get 6
        local.get 15
        i32.const 1
        i32.add
        i32.store offset=28
        br 0 (;@2;)
      end
    end
    local.get 6
    i32.load offset=24
    local.set 43
    local.get 43
    i32.const -1
    i32.add
    local.set 44
    block  ;; label = @1
      local.get 44
      local.get 43
      i32.gt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 44
    local.set 45
    local.get 6
    i32.load offset=24
    local.set 46
    local.get 6
    i32.load offset=20
    local.set 47
    block  ;; label = @1
      block  ;; label = @2
        local.get 45
        local.get 46
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      local.get 45
      local.get 46
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 47
    local.get 45
    i32.const 3
    i32.shl
    i32.add
    local.set 48
    local.get 48
    i32.load
    local.set 49
    local.get 48
    i32.load offset=4
    local.set 50
    local.get 6
    local.get 50
    i32.store offset=72
    local.get 6
    local.get 49
    i32.store offset=68
    local.get 6
    local.get 50
    i32.store offset=80
    local.get 6
    local.get 49
    i32.store offset=76
    local.get 6
    i32.load offset=16
    local.set 51
    local.get 51
    i32.load offset=12
    local.set 52
    local.get 51
    i32.load offset=8
    local.set 53
    local.get 52
    local.get 51
    i32.load offset=4
    i32.add
    local.set 54
    block  ;; label = @1
      block  ;; label = @2
        local.get 52
        local.get 53
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      local.get 52
      local.get 53
      call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
      unreachable
    end
    local.get 53
    local.get 52
    i32.sub
    local.set 55
    block  ;; label = @1
      block  ;; label = @2
        local.get 53
        local.get 53
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      local.get 53
      local.get 53
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 55
    local.set 56
    local.get 54
    local.set 57
    local.get 6
    local.get 55
    i32.store offset=88
    local.get 6
    local.get 54
    i32.store offset=84
    local.get 6
    local.get 55
    i32.store offset=96
    local.get 6
    local.get 54
    i32.store offset=92
    local.get 6
    i32.load offset=72
    local.set 58
    local.get 58
    i32.const 1
    i32.gt_u
    drop
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                block  ;; label = @7
                  local.get 58
                  br_table 0 (;@7;) 1 (;@6;) 2 (;@5;)
                end
                local.get 0
                i64.const 0
                i64.store align=4
                local.get 6
                i32.const 128
                i32.add
                global.set $m6__stack_pointer
                return
              end
              local.get 1
              local.get 5
              local.get 6
              i32.load offset=88
              i32.ge_u
              call $debug.assert
              local.get 6
              i32.load offset=72
              local.set 59
              local.get 6
              i32.load offset=68
              local.set 60
              i32.const 0
              local.get 59
              i32.lt_u
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            local.get 6
            i32.const 0
            i32.store offset=100
            br 2 (;@2;)
          end
          br 2 (;@1;)
        end
        local.get 1
        i32.const 0
        local.get 59
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      block  ;; label = @2
        loop  ;; label = @3
          local.get 6
          i32.load offset=100
          local.set 61
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 61
                local.get 5
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 6
                local.get 61
                i32.store offset=104
                local.get 6
                i32.load offset=72
                i64.extend_i32_u
                local.get 61
                i64.extend_i32_u
                i64.mul
                local.set 62
                local.get 62
                i64.const 32
                i64.shr_u
                i32.wrap_i64
                i32.const 0
                i32.ne
                local.set 63
                local.get 62
                i32.wrap_i64
                local.set 64
                local.get 63
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 3 (;@2;)
            end
            local.get 1
            call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
            unreachable
          end
          local.get 64
          local.set 65
          local.get 6
          i32.load offset=88
          local.set 66
          local.get 65
          local.get 6
          i32.load offset=84
          i32.add
          local.set 67
          block  ;; label = @4
            block  ;; label = @5
              local.get 65
              local.get 66
              i32.le_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 1
            local.get 65
            local.get 66
            call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
            unreachable
          end
          local.get 66
          local.get 65
          i32.sub
          local.set 68
          block  ;; label = @4
            block  ;; label = @5
              local.get 66
              local.get 66
              i32.le_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 1
            local.get 66
            local.get 66
            call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
            unreachable
          end
          local.get 6
          local.get 68
          i32.store offset=112
          local.get 6
          local.get 67
          i32.store offset=108
          local.get 6
          local.get 68
          i32.store offset=120
          local.get 6
          local.get 67
          i32.store offset=116
          local.get 6
          i32.load offset=72
          local.set 69
          local.get 6
          i32.load offset=112
          local.set 70
          local.get 69
          local.get 70
          local.get 69
          local.get 70
          i32.lt_u
          select
          local.set 71
          local.get 6
          local.get 71
          i32.store offset=124
          local.get 6
          i32.load offset=112
          local.set 72
          local.get 6
          i32.load offset=108
          local.set 73
          local.get 71
          i32.const 0
          i32.sub
          local.set 74
          block  ;; label = @4
            block  ;; label = @5
              local.get 71
              local.get 72
              i32.le_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 1
            local.get 71
            local.get 72
            call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
            unreachable
          end
          local.get 74
          local.set 75
          local.get 73
          local.set 76
          local.get 6
          i32.load offset=72
          local.set 77
          local.get 6
          i32.load offset=68
          local.set 78
          local.get 71
          i32.const 0
          i32.sub
          local.set 79
          block  ;; label = @4
            block  ;; label = @5
              local.get 71
              local.get 77
              i32.le_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 1
            local.get 71
            local.get 77
            call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
            unreachable
          end
          local.get 79
          local.set 80
          local.get 78
          local.set 81
          block  ;; label = @4
            block  ;; label = @5
              local.get 75
              local.get 80
              i32.eq
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 1
            call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
            unreachable
          end
          local.get 81
          local.get 75
          i32.add
          local.set 82
          local.get 76
          local.get 75
          i32.add
          local.set 83
          block  ;; label = @4
            block  ;; label = @5
              local.get 76
              local.get 82
              i32.ge_u
              local.get 81
              local.get 83
              i32.ge_u
              i32.or
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 1
            call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
            unreachable
          end
          block  ;; label = @4
            local.get 75
            i32.eqz
            br_if 0 (;@4;)
            local.get 76
            local.get 81
            local.get 75
            memory.copy
          end
          local.get 6
          i32.load offset=16
          local.set 84
          local.get 84
          i32.const 12
          i32.add
          local.set 85
          local.get 84
          i32.load offset=12
          local.set 86
          local.get 86
          local.get 71
          i32.add
          local.set 87
          block  ;; label = @4
            local.get 87
            local.get 86
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 1
            call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
            unreachable
          end
          local.get 85
          local.get 87
          i32.store
          block  ;; label = @4
            local.get 6
            i32.load offset=72
            local.get 6
            i32.load offset=112
            i32.gt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 1
            call $builtin.returnError
            local.get 0
            i32.const 0
            i64.load offset=1049392 align=4
            i64.store align=4
            local.get 6
            i32.const 128
            i32.add
            global.set $m6__stack_pointer
            return
          end
          local.get 6
          local.get 61
          i32.const 1
          i32.add
          i32.store offset=100
          br 0 (;@3;)
        end
      end
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.reachedUnreachable
      unreachable
    end
    local.get 60
    i32.load8_u
    local.set 88
    block  ;; label = @1
      local.get 56
      i32.eqz
      br_if 0 (;@1;)
      local.get 57
      local.get 88
      local.get 56
      memory.fill
    end
    local.get 6
    i32.load offset=16
    local.set 89
    local.get 89
    i32.const 12
    i32.add
    local.set 90
    local.get 89
    i32.load offset=12
    local.set 91
    local.get 91
    local.get 6
    i32.load offset=88
    i32.add
    local.set 92
    block  ;; label = @1
      local.get 92
      local.get 91
      i32.lt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 90
    local.get 92
    i32.store
    local.get 1
    call $builtin.returnError
    local.get 0
    i32.const 0
    i64.load offset=1049392 align=4
    i64.store align=4
    local.get 6
    i32.const 128
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $root.identity_from_pubkey (type $t_6_9) (param i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 224
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 0
    i32.store offset=4
    local.get 2
    local.get 1
    i32.store offset=8
    i32.const 32
    local.set 3
    local.get 2
    local.get 3
    i32.store offset=148
    local.get 2
    local.get 2
    i32.const 12
    i32.add
    i32.store offset=144
    local.get 2
    i32.const 0
    i32.store offset=140
    local.get 2
    local.get 0
    i32.store offset=152
    local.get 2
    local.get 1
    i32.store offset=156
    local.get 2
    i32.load offset=152
    local.set 4
    local.get 2
    i32.const 160
    i32.add
    local.get 2
    i32.const 140
    i32.add
    local.get 4
    local.get 3
    call $identity.Identity.fromPubKey
    i32.const 24
    local.set 5
    local.get 5
    local.get 2
    i32.const 192
    i32.add
    i32.add
    local.get 5
    local.get 2
    i32.const 160
    i32.add
    i32.add
    i64.load align=1
    i64.store
    i32.const 16
    local.set 6
    local.get 6
    local.get 2
    i32.const 192
    i32.add
    i32.add
    local.get 6
    local.get 2
    i32.const 160
    i32.add
    i32.add
    i64.load align=1
    i64.store
    i32.const 8
    local.set 7
    local.get 7
    local.get 2
    i32.const 192
    i32.add
    i32.add
    local.get 7
    local.get 2
    i32.const 160
    i32.add
    i32.add
    i64.load align=1
    i64.store
    local.get 2
    local.get 2
    i64.load offset=160 align=1
    i64.store offset=192
    local.get 2
    i32.load offset=156
    local.set 8
    local.get 2
    i32.const 192
    i32.add
    local.set 9
    local.get 9
    i32.const 32
    i32.add
    local.set 10
    local.get 8
    i32.const 32
    i32.add
    local.set 11
    block  ;; label = @1
      block  ;; label = @2
        local.get 8
        local.get 10
        i32.ge_u
        local.get 9
        local.get 11
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 2
      i32.const 140
      i32.add
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    local.get 8
    local.get 9
    i64.load align=1
    i64.store align=1
    i32.const 24
    local.set 12
    local.get 8
    local.get 12
    i32.add
    local.get 9
    local.get 12
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 16
    local.set 13
    local.get 8
    local.get 13
    i32.add
    local.get 9
    local.get 13
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 8
    local.set 14
    local.get 8
    local.get 14
    i32.add
    local.get 9
    local.get 14
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 2
    i32.const 224
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $identity.Identity.fromPubKey (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i64 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 80
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 3
    i32.store offset=12
    local.get 4
    local.get 2
    i32.store offset=8
    local.get 4
    i32.const 40
    i32.add
    local.set 5
    i64.const -6148914691236517206
    local.set 6
    local.get 5
    local.get 6
    i64.store
    local.get 4
    i32.const 32
    i32.add
    local.get 6
    i64.store
    local.get 4
    i32.const 24
    i32.add
    local.get 6
    i64.store
    local.get 4
    local.get 6
    i64.store offset=16
    i32.const 1049400
    local.set 7
    local.get 1
    local.get 2
    local.get 3
    local.get 4
    i32.const 16
    i32.add
    local.get 7
    call $crypto.sha3.Keccak_1600_256_6_24_.hash
    i32.const 32
    local.set 8
    local.get 4
    i32.const 16
    i32.add
    local.set 9
    local.get 4
    i32.const 48
    i32.add
    local.get 1
    local.get 9
    local.get 8
    call $identity.Identity.fromBytes
    local.get 0
    local.get 4
    i64.load offset=48 align=1
    i64.store align=1
    i32.const 24
    local.set 10
    local.get 0
    local.get 10
    i32.add
    local.get 10
    local.get 4
    i32.const 48
    i32.add
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 16
    local.set 11
    local.get 0
    local.get 11
    i32.add
    local.get 11
    local.get 4
    i32.const 48
    i32.add
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 8
    local.set 12
    local.get 0
    local.get 12
    i32.add
    local.get 12
    local.get 4
    i32.const 48
    i32.add
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 4
    i32.const 80
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.sha3.Keccak_1600_256_6_24_.hash (type $t_6_5) (param i32 i32 i32 i32 i32)
    (local i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 704
    i32.sub
    local.set 5
    local.get 5
    global.set $m6__stack_pointer
    local.get 2
    local.set 6
    local.get 1
    local.set 7
    local.get 5
    local.get 2
    i32.store offset=8
    local.get 5
    local.get 1
    i32.store offset=4
    local.get 5
    local.get 3
    i32.store offset=12
    local.get 5
    i32.const 360
    i32.add
    local.get 0
    local.get 4
    call $crypto.sha3.Keccak_1600_256_6_24_.init
    i32.const 344
    local.set 8
    block  ;; label = @1
      local.get 8
      i32.eqz
      br_if 0 (;@1;)
      local.get 5
      i32.const 16
      i32.add
      local.get 5
      i32.const 360
      i32.add
      local.get 8
      memory.copy
    end
    local.get 0
    local.get 5
    i32.const 16
    i32.add
    local.get 7
    local.get 6
    call $crypto.sha3.Keccak_1600_256_6_24_.update
    local.get 0
    local.get 5
    i32.const 16
    i32.add
    local.get 3
    call $crypto.sha3.Keccak_1600_256_6_24_.final
    local.get 5
    i32.const 704
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $identity.Identity.fromBytes (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 80
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 3
    i32.store offset=4
    local.get 4
    local.get 2
    i32.store
    local.get 4
    local.get 3
    i32.store offset=12
    local.get 4
    local.get 2
    i32.store offset=8
    local.get 4
    i32.const 40
    i32.add
    local.set 5
    i64.const -6148914691236517206
    local.set 6
    local.get 5
    local.get 6
    i64.store
    local.get 4
    i32.const 32
    i32.add
    local.get 6
    i64.store
    local.get 4
    i32.const 24
    i32.add
    local.get 6
    i64.store
    local.get 4
    local.get 6
    i64.store offset=16
    local.get 4
    i32.const 16
    i32.add
    local.set 7
    local.get 4
    i32.load offset=12
    local.set 8
    i32.const 32
    local.set 9
    local.get 8
    local.get 9
    local.get 8
    local.get 9
    i32.lt_u
    select
    local.set 10
    local.get 4
    i32.load offset=8
    local.set 11
    local.get 10
    i32.const 0
    i32.sub
    local.set 12
    block  ;; label = @1
      block  ;; label = @2
        local.get 10
        local.get 8
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      local.get 10
      local.get 8
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 12
    local.set 13
    local.get 11
    local.set 14
    block  ;; label = @1
      block  ;; label = @2
        local.get 13
        i32.const 32
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
      unreachable
    end
    local.get 14
    i32.const 32
    i32.add
    local.set 15
    local.get 7
    i32.const 32
    i32.add
    local.set 16
    block  ;; label = @1
      block  ;; label = @2
        local.get 7
        local.get 15
        i32.ge_u
        local.get 14
        local.get 16
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    local.get 7
    local.get 14
    i64.load align=1
    i64.store align=1
    i32.const 24
    local.set 17
    local.get 7
    local.get 17
    i32.add
    local.get 14
    local.get 17
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 16
    local.set 18
    local.get 7
    local.get 18
    i32.add
    local.get 14
    local.get 18
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 8
    local.set 19
    local.get 7
    local.get 19
    i32.add
    local.get 14
    local.get 19
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 24
    local.set 20
    local.get 20
    local.get 4
    i32.const 48
    i32.add
    i32.add
    local.get 20
    local.get 4
    i32.const 16
    i32.add
    i32.add
    i64.load
    i64.store
    i32.const 16
    local.set 21
    local.get 21
    local.get 4
    i32.const 48
    i32.add
    i32.add
    local.get 21
    local.get 4
    i32.const 16
    i32.add
    i32.add
    i64.load
    i64.store
    i32.const 8
    local.set 22
    local.get 22
    local.get 4
    i32.const 48
    i32.add
    i32.add
    local.get 22
    local.get 4
    i32.const 16
    i32.add
    i32.add
    i64.load
    i64.store
    local.get 4
    local.get 4
    i64.load offset=16
    i64.store offset=48
    local.get 0
    local.get 4
    i64.load offset=48 align=1
    i64.store align=1
    i32.const 24
    local.set 23
    local.get 0
    local.get 23
    i32.add
    local.get 23
    local.get 4
    i32.const 48
    i32.add
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 16
    local.set 24
    local.get 0
    local.get 24
    i32.add
    local.get 24
    local.get 4
    i32.const 48
    i32.add
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 8
    local.set 25
    local.get 0
    local.get 25
    i32.add
    local.get 25
    local.get 4
    i32.const 48
    i32.add
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 4
    i32.const 80
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.sha3.Keccak_1600_256_6_24_.init (type $t_6_2) (param i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 704
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 2
    i32.load8_u
    i32.store8 offset=15
    local.get 3
    local.get 3
    i32.load8_u offset=15
    i32.store8 offset=220
    local.get 3
    i32.const 0
    i32.store offset=216
    local.get 3
    i32.const 16
    i32.add
    i32.const 205
    i32.add
    local.set 4
    i32.const 1049600
    local.set 5
    i32.const 136
    local.set 6
    block  ;; label = @1
      local.get 6
      i32.eqz
      br_if 0 (;@1;)
      local.get 4
      local.get 5
      local.get 6
      memory.copy
    end
    i32.const 1049736
    local.set 7
    i32.const 200
    local.set 8
    block  ;; label = @1
      local.get 8
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.const 16
      i32.add
      local.get 7
      local.get 8
      memory.copy
    end
    local.get 3
    i32.const 16
    i32.add
    i32.const 341
    i32.add
    i32.const 0
    i32.store8
    i32.const 344
    local.set 9
    block  ;; label = @1
      local.get 9
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.const 360
      i32.add
      local.get 3
      i32.const 16
      i32.add
      local.get 9
      memory.copy
    end
    i32.const 344
    local.set 10
    block  ;; label = @1
      local.get 10
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      local.get 3
      i32.const 360
      i32.add
      local.get 10
      memory.copy
    end
    local.get 3
    i32.const 704
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.sha3.Keccak_1600_256_6_24_.update (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 3
    local.set 5
    local.get 2
    local.set 6
    local.get 4
    local.get 1
    i32.store
    local.get 4
    local.get 3
    i32.store offset=8
    local.get 4
    local.get 2
    i32.store offset=4
    local.get 4
    local.get 1
    i32.store offset=12
    local.get 0
    local.get 4
    i32.load offset=12
    local.get 6
    local.get 5
    call $crypto.keccak_p.State_1600_512_24_.absorb
    local.get 4
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.sha3.Keccak_1600_256_6_24_.final (type $t_6_2) (param i32 i32 i32)
    (local i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 1
    i32.store
    local.get 3
    local.get 2
    i32.store offset=4
    local.get 3
    local.get 1
    i32.store offset=8
    local.get 3
    local.get 2
    i32.store offset=12
    local.get 0
    local.get 3
    i32.load offset=8
    call $crypto.keccak_p.State_1600_512_24_.pad
    local.get 0
    local.get 3
    i32.load offset=8
    local.get 3
    i32.load offset=12
    i32.const 32
    call $crypto.keccak_p.State_1600_512_24_.squeeze
    local.get 3
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.keccak_p.State_1600_512_24_.pad (type $t_6_9) (param i32 i32)
    (local i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i32.store
    local.get 2
    local.get 1
    i32.store offset=4
    local.get 0
    local.get 2
    i32.load offset=4
    i32.const 341
    i32.add
    i32.const 3
    call $crypto.keccak_p.State__struct_8347.to
    local.get 2
    i32.load offset=4
    local.set 3
    local.get 2
    i32.load offset=4
    i32.const 205
    i32.add
    local.set 4
    local.get 2
    i32.load offset=4
    i32.load offset=200
    local.set 5
    local.get 5
    i32.const 0
    i32.sub
    local.set 6
    block  ;; label = @1
      block  ;; label = @2
        local.get 5
        i32.const 136
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 5
      i32.const 136
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 2
    local.get 6
    i32.store offset=12
    local.get 2
    local.get 4
    i32.store offset=8
    local.get 2
    i32.load offset=12
    local.set 7
    local.get 0
    local.get 3
    local.get 2
    i32.load offset=8
    local.get 7
    call $crypto.keccak_p.KeccakF_1600_.addBytes
    block  ;; label = @1
      block  ;; label = @2
        local.get 2
        i32.load offset=4
        i32.load offset=200
        i32.const 136
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        local.get 2
        i32.load offset=4
        call $crypto.keccak_p.KeccakF_1600_.permuteR__anon_8442
        local.get 2
        i32.load offset=4
        i32.const 0
        i32.store offset=200
        br 1 (;@1;)
      end
    end
    local.get 0
    local.get 2
    i32.load offset=4
    local.get 2
    i32.load offset=4
    i32.load8_u offset=204
    local.get 2
    i32.load offset=4
    i32.load offset=200
    call $crypto.keccak_p.KeccakF_1600_.addByte
    local.get 0
    local.get 2
    i32.load offset=4
    i32.const 128
    i32.const 135
    call $crypto.keccak_p.KeccakF_1600_.addByte
    local.get 0
    local.get 2
    i32.load offset=4
    call $crypto.keccak_p.KeccakF_1600_.permuteR__anon_8442
    local.get 2
    i32.load offset=4
    i32.const 0
    i32.store offset=200
    local.get 0
    local.get 2
    i32.load offset=4
    i32.const 341
    i32.add
    i32.const 2
    call $crypto.keccak_p.State__struct_8347.to
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.keccak_p.State_1600_512_24_.squeeze (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 176
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 1
    i32.store offset=4
    local.get 4
    local.get 3
    i32.store offset=12
    local.get 4
    local.get 2
    i32.store offset=8
    local.get 4
    local.get 1
    i32.store offset=16
    local.get 4
    local.get 3
    i32.store offset=24
    local.get 4
    local.get 2
    i32.store offset=20
    local.get 0
    local.get 4
    i32.load offset=16
    i32.const 341
    i32.add
    i32.const 4
    call $crypto.keccak_p.State__struct_8347.to
    local.get 4
    i32.const 0
    i32.store offset=28
    block  ;; label = @1
      block  ;; label = @2
        local.get 4
        i32.load offset=16
        i32.load offset=200
        i32.const 136
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        local.get 4
        i32.load offset=16
        call $crypto.keccak_p.KeccakF_1600_.permuteR__anon_8442
        br 1 (;@1;)
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 4
              i32.load offset=16
              i32.load offset=200
              i32.const 0
              i32.gt_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              i32.const 136
              local.set 5
              i32.const 170
              local.set 6
              block  ;; label = @6
                local.get 5
                i32.eqz
                br_if 0 (;@6;)
                local.get 4
                i32.const 32
                i32.add
                local.get 6
                local.get 5
                memory.fill
              end
              local.get 0
              local.get 4
              i32.load offset=16
              local.get 4
              i32.const 32
              i32.add
              local.get 5
              call $crypto.keccak_p.KeccakF_1600_.extractBytes
              local.get 4
              i32.load offset=16
              i32.load offset=200
              local.set 7
              i32.const 136
              local.set 8
              local.get 8
              local.get 7
              i32.sub
              local.set 9
              local.get 9
              local.get 8
              i32.gt_u
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            br 2 (;@2;)
          end
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        i32.load offset=24
        local.set 10
        local.get 9
        local.get 10
        local.get 9
        local.get 10
        i32.lt_u
        select
        local.set 11
        local.get 4
        local.get 11
        i32.store offset=168
        local.get 4
        i32.load offset=24
        local.set 12
        local.get 4
        i32.load offset=20
        local.set 13
        local.get 11
        i32.const 0
        i32.sub
        local.set 14
        block  ;; label = @3
          block  ;; label = @4
            local.get 11
            local.get 12
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 11
          local.get 12
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 14
        local.set 15
        local.get 13
        local.set 16
        local.get 4
        i32.load offset=16
        i32.load offset=200
        local.set 17
        local.get 17
        local.get 4
        i32.const 32
        i32.add
        i32.add
        local.set 18
        local.get 17
        local.get 11
        i32.add
        local.set 19
        block  ;; label = @3
          block  ;; label = @4
            local.get 19
            i32.const 136
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 19
          i32.const 136
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 11
        local.set 20
        local.get 18
        local.set 21
        block  ;; label = @3
          block  ;; label = @4
            local.get 15
            local.get 20
            i32.eq
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
          unreachable
        end
        local.get 21
        local.get 15
        i32.add
        local.set 22
        local.get 16
        local.get 15
        i32.add
        local.set 23
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            local.get 22
            i32.ge_u
            local.get 21
            local.get 23
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        block  ;; label = @3
          local.get 15
          i32.eqz
          br_if 0 (;@3;)
          local.get 16
          local.get 21
          local.get 15
          memory.copy
        end
        local.get 4
        i32.load offset=16
        local.set 24
        local.get 24
        i32.const 200
        i32.add
        local.set 25
        local.get 24
        i32.load offset=200
        local.set 26
        local.get 26
        local.get 11
        i32.add
        local.set 27
        block  ;; label = @3
          local.get 27
          local.get 26
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 25
        local.get 27
        i32.store
        block  ;; label = @3
          local.get 11
          local.get 4
          i32.load offset=24
          i32.eq
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 176
          i32.add
          global.set $m6__stack_pointer
          return
        end
        block  ;; label = @3
          block  ;; label = @4
            local.get 4
            i32.load offset=16
            i32.load offset=200
            i32.const 136
            i32.eq
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 4
            i32.load offset=16
            i32.const 0
            i32.store offset=200
            local.get 0
            local.get 4
            i32.load offset=16
            call $crypto.keccak_p.KeccakF_1600_.permuteR__anon_8442
            br 1 (;@3;)
          end
        end
        local.get 4
        local.get 11
        i32.store offset=28
      end
    end
    loop  ;; label = @1
      local.get 4
      i32.load offset=28
      local.set 28
      local.get 28
      i32.const 136
      i32.add
      local.set 29
      block  ;; label = @2
        local.get 29
        local.get 28
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 29
                local.get 4
                i32.load offset=24
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 4
                i32.load offset=16
                local.set 30
                local.get 4
                i32.load offset=28
                local.set 31
                local.get 4
                i32.load offset=24
                local.set 32
                local.get 31
                local.get 4
                i32.load offset=20
                i32.add
                local.set 33
                local.get 31
                i32.const 136
                i32.add
                local.set 34
                local.get 34
                local.get 32
                i32.le_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 3 (;@2;)
            end
            br 1 (;@3;)
          end
          local.get 0
          local.get 34
          local.get 32
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        i32.const 136
        local.set 35
        local.get 0
        local.get 30
        local.get 33
        local.get 35
        call $crypto.keccak_p.KeccakF_1600_.extractBytes
        local.get 0
        local.get 4
        i32.load offset=16
        call $crypto.keccak_p.KeccakF_1600_.permuteR__anon_8442
        local.get 4
        i32.load offset=28
        local.set 36
        local.get 36
        i32.const 136
        i32.add
        local.set 37
        block  ;; label = @3
          local.get 37
          local.get 36
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 37
        i32.store offset=28
        br 1 (;@1;)
      end
    end
    local.get 4
    i32.load offset=24
    local.set 38
    local.get 38
    local.get 4
    i32.load offset=28
    i32.sub
    local.set 39
    block  ;; label = @1
      local.get 39
      local.get 38
      i32.gt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 4
    local.get 39
    i32.store offset=172
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 39
              i32.const 0
              i32.gt_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              local.get 4
              i32.load offset=16
              local.set 40
              local.get 4
              i32.load offset=28
              local.set 41
              local.get 4
              i32.load offset=24
              local.set 42
              local.get 41
              local.get 4
              i32.load offset=20
              i32.add
              local.set 43
              local.get 41
              local.get 39
              i32.add
              local.set 44
              local.get 44
              local.get 42
              i32.le_u
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            br 3 (;@1;)
          end
          br 1 (;@2;)
        end
        local.get 0
        local.get 44
        local.get 42
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 39
      local.set 45
      local.get 0
      local.get 40
      local.get 43
      local.get 45
      call $crypto.keccak_p.KeccakF_1600_.extractBytes
    end
    local.get 4
    i32.load offset=16
    local.get 39
    i32.store offset=200
    local.get 4
    i32.const 176
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.keccak_p.State_1600_512_24_.absorb (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 48
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 1
    i32.store offset=12
    local.get 4
    local.get 3
    i32.store offset=20
    local.get 4
    local.get 2
    i32.store offset=16
    local.get 4
    local.get 1
    i32.store offset=24
    local.get 4
    local.get 3
    i32.store offset=32
    local.get 4
    local.get 2
    i32.store offset=28
    local.get 0
    local.get 4
    i32.load offset=24
    i32.const 341
    i32.add
    i32.const 3
    call $crypto.keccak_p.State__struct_8347.to
    local.get 4
    i32.const 0
    i32.store offset=36
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 4
            i32.load offset=24
            i32.load offset=200
            i32.const 0
            i32.gt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 4
            i32.load offset=24
            i32.load offset=200
            local.set 5
            i32.const 136
            local.set 6
            local.get 6
            local.get 5
            i32.sub
            local.set 7
            local.get 7
            local.get 6
            i32.gt_u
            i32.const 1
            i32.and
            br_if 1 (;@3;)
            br 2 (;@2;)
          end
          br 2 (;@1;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      local.get 4
      i32.load offset=32
      local.set 8
      local.get 7
      local.get 8
      local.get 7
      local.get 8
      i32.lt_u
      select
      local.set 9
      local.get 4
      local.get 9
      i32.store offset=40
      local.get 4
      i32.load offset=24
      i32.const 205
      i32.add
      local.set 10
      local.get 4
      i32.load offset=24
      i32.load offset=200
      local.set 11
      local.get 10
      local.get 11
      i32.add
      local.set 12
      local.get 11
      local.get 9
      i32.add
      local.set 13
      block  ;; label = @2
        block  ;; label = @3
          local.get 13
          i32.const 136
          i32.le_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        local.get 13
        i32.const 136
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 9
      local.set 14
      local.get 12
      local.set 15
      local.get 4
      i32.load offset=32
      local.set 16
      local.get 4
      i32.load offset=28
      local.set 17
      local.get 9
      i32.const 0
      i32.sub
      local.set 18
      block  ;; label = @2
        block  ;; label = @3
          local.get 9
          local.get 16
          i32.le_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        local.get 9
        local.get 16
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 18
      local.set 19
      local.get 17
      local.set 20
      block  ;; label = @2
        block  ;; label = @3
          local.get 14
          local.get 19
          i32.eq
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
        unreachable
      end
      local.get 20
      local.get 14
      i32.add
      local.set 21
      local.get 15
      local.get 14
      i32.add
      local.set 22
      block  ;; label = @2
        block  ;; label = @3
          local.get 15
          local.get 21
          i32.ge_u
          local.get 20
          local.get 22
          i32.ge_u
          i32.or
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
        unreachable
      end
      block  ;; label = @2
        local.get 14
        i32.eqz
        br_if 0 (;@2;)
        local.get 15
        local.get 20
        local.get 14
        memory.copy
      end
      local.get 4
      i32.load offset=24
      local.set 23
      local.get 23
      i32.const 200
      i32.add
      local.set 24
      local.get 23
      i32.load offset=200
      local.set 25
      local.get 25
      local.get 9
      i32.add
      local.set 26
      block  ;; label = @2
        local.get 26
        local.get 25
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      local.get 24
      local.get 26
      i32.store
      block  ;; label = @2
        local.get 9
        local.get 4
        i32.load offset=32
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 4
        i32.const 48
        i32.add
        global.set $m6__stack_pointer
        return
      end
      block  ;; label = @2
        block  ;; label = @3
          local.get 4
          i32.load offset=24
          i32.load offset=200
          i32.const 136
          i32.eq
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.load offset=24
          local.set 27
          local.get 0
          local.get 27
          local.get 27
          i32.const 205
          i32.add
          i32.const 136
          call $crypto.keccak_p.KeccakF_1600_.addBytes
          local.get 0
          local.get 4
          i32.load offset=24
          call $crypto.keccak_p.KeccakF_1600_.permuteR__anon_8442
          local.get 4
          i32.load offset=24
          i32.const 0
          i32.store offset=200
          br 1 (;@2;)
        end
      end
      local.get 4
      local.get 9
      i32.store offset=36
    end
    loop  ;; label = @1
      local.get 4
      i32.load offset=36
      local.set 28
      local.get 28
      i32.const 136
      i32.add
      local.set 29
      block  ;; label = @2
        local.get 29
        local.get 28
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 29
                local.get 4
                i32.load offset=32
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 4
                i32.load offset=24
                local.set 30
                local.get 4
                i32.load offset=36
                local.set 31
                local.get 4
                i32.load offset=32
                local.set 32
                local.get 31
                local.get 4
                i32.load offset=28
                i32.add
                local.set 33
                local.get 31
                i32.const 136
                i32.add
                local.set 34
                local.get 34
                local.get 32
                i32.le_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 3 (;@2;)
            end
            br 1 (;@3;)
          end
          local.get 0
          local.get 34
          local.get 32
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        i32.const 136
        local.set 35
        local.get 0
        local.get 30
        local.get 33
        local.get 35
        call $crypto.keccak_p.KeccakF_1600_.addBytes
        local.get 0
        local.get 4
        i32.load offset=24
        call $crypto.keccak_p.KeccakF_1600_.permuteR__anon_8442
        local.get 4
        i32.load offset=36
        local.set 36
        local.get 36
        i32.const 136
        i32.add
        local.set 37
        block  ;; label = @3
          local.get 37
          local.get 36
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 37
        i32.store offset=36
        br 1 (;@1;)
      end
    end
    local.get 4
    i32.load offset=32
    local.set 38
    local.get 38
    local.get 4
    i32.load offset=36
    i32.sub
    local.set 39
    block  ;; label = @1
      local.get 39
      local.get 38
      i32.gt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 4
    local.get 39
    i32.store offset=44
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 39
              i32.const 0
              i32.gt_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              local.get 4
              i32.load offset=24
              i32.const 205
              i32.add
              local.set 40
              local.get 39
              i32.const 0
              i32.sub
              local.set 41
              local.get 39
              i32.const 136
              i32.le_u
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            br 3 (;@1;)
          end
          br 1 (;@2;)
        end
        local.get 0
        local.get 39
        i32.const 136
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 41
      local.set 42
      local.get 40
      local.set 43
      local.get 4
      i32.load offset=36
      local.set 44
      local.get 4
      i32.load offset=32
      local.set 45
      local.get 44
      local.get 4
      i32.load offset=28
      i32.add
      local.set 46
      local.get 44
      local.get 39
      i32.add
      local.set 47
      block  ;; label = @2
        block  ;; label = @3
          local.get 47
          local.get 45
          i32.le_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        local.get 47
        local.get 45
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 39
      local.set 48
      local.get 46
      local.set 49
      block  ;; label = @2
        block  ;; label = @3
          local.get 42
          local.get 48
          i32.eq
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
        unreachable
      end
      local.get 49
      local.get 42
      i32.add
      local.set 50
      local.get 43
      local.get 42
      i32.add
      local.set 51
      block  ;; label = @2
        block  ;; label = @3
          local.get 43
          local.get 50
          i32.ge_u
          local.get 49
          local.get 51
          i32.ge_u
          i32.or
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
        unreachable
      end
      block  ;; label = @2
        local.get 42
        i32.eqz
        br_if 0 (;@2;)
        local.get 43
        local.get 49
        local.get 42
        memory.copy
      end
    end
    local.get 4
    i32.load offset=24
    local.get 39
    i32.store offset=200
    local.get 4
    i32.const 48
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.keccak_p.State__struct_8347.to (type $t_6_2) (param i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 1
    i32.store offset=4
    i32.const 7
    local.set 4
    local.get 3
    local.get 2
    local.get 4
    i32.and
    i32.store8 offset=11
    local.get 3
    local.get 1
    i32.store offset=12
    local.get 4
    local.get 2
    i32.const 4
    i32.add
    i32.and
    local.set 5
    block  ;; label = @1
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
                            i32.const 0
                            br_if 0 (;@12;)
                            block  ;; label = @13
                              block  ;; label = @14
                                block  ;; label = @15
                                  block  ;; label = @16
                                    block  ;; label = @17
                                      local.get 5
                                      br_table 2 (;@15;) 5 (;@12;) 5 (;@12;) 5 (;@12;) 3 (;@14;) 4 (;@13;) 0 (;@17;) 1 (;@16;) 2 (;@15;)
                                    end
                                    local.get 3
                                    i32.load offset=12
                                    i32.load8_u
                                    local.set 6
                                    local.get 6
                                    i32.const 7
                                    i32.and
                                    i32.eqz
                                    br_if 5 (;@11;)
                                    br 6 (;@10;)
                                  end
                                  local.get 3
                                  i32.load offset=12
                                  i32.load8_u
                                  local.set 7
                                  local.get 7
                                  i32.const 7
                                  i32.and
                                  i32.const 4
                                  i32.eq
                                  br_if 6 (;@9;)
                                  br 7 (;@8;)
                                end
                                local.get 3
                                i32.load offset=12
                                i32.load8_u
                                local.set 8
                                i32.const 29
                                local.set 9
                                local.get 8
                                local.get 9
                                i32.shl
                                local.get 9
                                i32.shr_s
                                i32.const 0
                                i32.lt_s
                                local.set 10
                                local.get 8
                                i32.const 7
                                i32.and
                                local.set 11
                                local.get 10
                                br_if 10 (;@4;)
                                local.get 11
                                br_table 7 (;@7;) 8 (;@6;) 10 (;@4;) 9 (;@5;) 7 (;@7;)
                              end
                              local.get 0
                              i32.const 1048963
                              i32.const 34
                              i32.const 1049360
                              call $debug.defaultPanic
                              unreachable
                            end
                            br 11 (;@1;)
                          end
                          local.get 0
                          call $debug.FullPanic__function_'defaultPanic'__.corruptSwitch
                          unreachable
                        end
                        local.get 0
                        i32.const 1048765
                        i32.const 34
                        i32.const 1049360
                        call $debug.defaultPanic
                        unreachable
                      end
                      block  ;; label = @10
                        local.get 6
                        call $__zig_is_named_enum_value_crypto.keccak_p.State.Op
                        i32.const 1
                        i32.and
                        i32.eqz
                        br_if 0 (;@10;)
                        br 8 (;@2;)
                      end
                      local.get 0
                      call $debug.FullPanic__function_'defaultPanic'__.corruptSwitch
                      unreachable
                    end
                    local.get 0
                    i32.const 1048800
                    i32.const 35
                    i32.const 1049360
                    call $debug.defaultPanic
                    unreachable
                  end
                  block  ;; label = @8
                    local.get 7
                    call $__zig_is_named_enum_value_crypto.keccak_p.State.Op
                    i32.const 1
                    i32.and
                    i32.eqz
                    br_if 0 (;@8;)
                    br 5 (;@3;)
                  end
                  local.get 0
                  call $debug.FullPanic__function_'defaultPanic'__.corruptSwitch
                  unreachable
                end
                local.get 0
                i32.const 1048730
                i32.const 34
                i32.const 1049360
                call $debug.defaultPanic
                unreachable
              end
              local.get 0
              i32.const 1048690
              i32.const 39
              i32.const 1049360
              call $debug.defaultPanic
              unreachable
            end
            local.get 0
            i32.const 1048836
            i32.const 36
            i32.const 1049360
            call $debug.defaultPanic
            unreachable
          end
          block  ;; label = @4
            block  ;; label = @5
              local.get 8
              call $__zig_is_named_enum_value_crypto.keccak_p.State.Op
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              br 1 (;@4;)
            end
            local.get 0
            call $debug.FullPanic__function_'defaultPanic'__.corruptSwitch
            unreachable
          end
          br 2 (;@1;)
        end
        br 1 (;@1;)
      end
    end
    local.get 3
    i32.load offset=12
    local.get 2
    i32.const 7
    i32.and
    i32.store8
    local.get 3
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.keccak_p.KeccakF_1600_.permuteR__anon_8442 (type $t_6_9) (param i32 i32)
    (local i32 i32 i32 i64 i32 i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i64 i64 i64 i64 i64 i64 i64 i32 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i32 i32 i32 i64 i32 i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i64 i64 i64 i64 i64 i64 i64 i32 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i32 i32 i32 i32 i64 i32 i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i64 i64 i64 i64 i64 i64 i64 i32 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i32 i32 i32 i32 i32 i64 i32 i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i64 i64 i64 i64 i64 i64 i64 i32 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i64 i32 i32)
    global.get $m6__stack_pointer
    i32.const 3696
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i32.store
    local.get 2
    local.get 1
    i32.store offset=4
    local.get 2
    i32.const 0
    i32.store offset=8
    block  ;; label = @1
      loop  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 2
                i32.load offset=8
                i32.const 24
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 2
                i32.load offset=4
                local.set 3
                local.get 2
                i32.load offset=8
                local.set 4
                local.get 4
                i32.const 24
                i32.lt_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 0
          local.get 4
          i32.const 24
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        i32.const 1049408
        local.get 4
        i32.const 3
        i32.shl
        i32.add
        i64.load
        local.set 5
        local.get 2
        local.get 3
        i32.store offset=12
        local.get 2
        local.get 5
        i64.store offset=16
        local.get 2
        local.get 3
        i32.store offset=28
        local.get 2
        i32.load offset=28
        local.set 6
        local.get 2
        local.get 6
        i32.store offset=32
        local.get 2
        local.get 6
        i32.store offset=36
        local.get 2
        i32.const 72
        i32.add
        local.set 7
        i64.const 0
        local.set 8
        local.get 7
        local.get 8
        i64.store
        local.get 2
        i32.const 64
        i32.add
        local.get 8
        i64.store
        local.get 2
        i32.const 56
        i32.add
        local.get 8
        i64.store
        local.get 2
        i32.const 48
        i32.add
        local.get 8
        i64.store
        local.get 2
        local.get 8
        i64.store offset=40
        local.get 2
        i32.const 0
        i32.store offset=80
        local.get 2
        i32.const 0
        i32.store offset=84
        local.get 2
        local.get 2
        i64.load offset=40
        local.get 2
        i32.load offset=32
        i64.load
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=88
        local.get 2
        local.get 2
        i64.load offset=40
        local.get 2
        i32.load offset=32
        i64.load offset=40
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 2
        i32.store offset=92
        local.get 2
        local.get 2
        i64.load offset=40
        local.get 2
        i32.load offset=32
        i64.load offset=80
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 3
        i32.store offset=96
        local.get 2
        local.get 2
        i64.load offset=40
        local.get 2
        i32.load offset=32
        i64.load offset=120
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 4
        i32.store offset=100
        local.get 2
        local.get 2
        i64.load offset=40
        local.get 2
        i32.load offset=32
        i64.load offset=160
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=104
        local.get 2
        i32.const 0
        i32.store offset=108
        local.get 2
        local.get 2
        i64.load offset=48
        local.get 2
        i32.load offset=32
        i64.load offset=8
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 1
        i32.store offset=112
        local.get 2
        local.get 2
        i64.load offset=48
        local.get 2
        i32.load offset=32
        i64.load offset=48
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=116
        local.get 2
        local.get 2
        i64.load offset=48
        local.get 2
        i32.load offset=32
        i64.load offset=88
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 3
        i32.store offset=120
        local.get 2
        local.get 2
        i64.load offset=48
        local.get 2
        i32.load offset=32
        i64.load offset=128
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 4
        i32.store offset=124
        local.get 2
        local.get 2
        i64.load offset=48
        local.get 2
        i32.load offset=32
        i64.load offset=168
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=128
        local.get 2
        i32.const 0
        i32.store offset=132
        local.get 2
        local.get 2
        i64.load offset=56
        local.get 2
        i32.load offset=32
        i64.load offset=16
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 1
        i32.store offset=136
        local.get 2
        local.get 2
        i64.load offset=56
        local.get 2
        i32.load offset=32
        i64.load offset=56
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 2
        i32.store offset=140
        local.get 2
        local.get 2
        i64.load offset=56
        local.get 2
        i32.load offset=32
        i64.load offset=96
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=144
        local.get 2
        local.get 2
        i64.load offset=56
        local.get 2
        i32.load offset=32
        i64.load offset=136
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 4
        i32.store offset=148
        local.get 2
        local.get 2
        i64.load offset=56
        local.get 2
        i32.load offset=32
        i64.load offset=176
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=152
        local.get 2
        i32.const 0
        i32.store offset=156
        local.get 2
        local.get 2
        i64.load offset=64
        local.get 2
        i32.load offset=32
        i64.load offset=24
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 1
        i32.store offset=160
        local.get 2
        local.get 2
        i64.load offset=64
        local.get 2
        i32.load offset=32
        i64.load offset=64
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 2
        i32.store offset=164
        local.get 2
        local.get 2
        i64.load offset=64
        local.get 2
        i32.load offset=32
        i64.load offset=104
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 3
        i32.store offset=168
        local.get 2
        local.get 2
        i64.load offset=64
        local.get 2
        i32.load offset=32
        i64.load offset=144
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=172
        local.get 2
        local.get 2
        i64.load offset=64
        local.get 2
        i32.load offset=32
        i64.load offset=184
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=176
        local.get 2
        i32.const 0
        i32.store offset=180
        local.get 2
        local.get 2
        i64.load offset=72
        local.get 2
        i32.load offset=32
        i64.load offset=32
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 1
        i32.store offset=184
        local.get 2
        local.get 2
        i64.load offset=72
        local.get 2
        i32.load offset=32
        i64.load offset=72
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 2
        i32.store offset=188
        local.get 2
        local.get 2
        i64.load offset=72
        local.get 2
        i32.load offset=32
        i64.load offset=112
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 3
        i32.store offset=192
        local.get 2
        local.get 2
        i64.load offset=72
        local.get 2
        i32.load offset=32
        i64.load offset=152
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 4
        i32.store offset=196
        local.get 2
        local.get 2
        i64.load offset=72
        local.get 2
        i32.load offset=32
        i64.load offset=192
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 0
        i32.store offset=200
        local.get 2
        i32.const 0
        i32.store offset=204
        local.get 2
        i32.load offset=32
        local.set 9
        local.get 9
        local.get 9
        i64.load
        local.get 2
        i64.load offset=72
        local.get 0
        local.get 2
        i64.load offset=48
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store
        local.get 2
        i32.const 1
        i32.store offset=208
        local.get 2
        i32.load offset=32
        local.set 10
        local.get 10
        local.get 10
        i64.load offset=40
        local.get 2
        i64.load offset=72
        local.get 0
        local.get 2
        i64.load offset=48
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 2
        i32.store offset=212
        local.get 2
        i32.load offset=32
        local.set 11
        local.get 11
        local.get 11
        i64.load offset=80
        local.get 2
        i64.load offset=72
        local.get 0
        local.get 2
        i64.load offset=48
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=80
        local.get 2
        i32.const 3
        i32.store offset=216
        local.get 2
        i32.load offset=32
        local.set 12
        local.get 12
        local.get 12
        i64.load offset=120
        local.get 2
        i64.load offset=72
        local.get 0
        local.get 2
        i64.load offset=48
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=120
        local.get 2
        i32.const 4
        i32.store offset=220
        local.get 2
        i32.load offset=32
        local.set 13
        local.get 13
        local.get 13
        i64.load offset=160
        local.get 2
        i64.load offset=72
        local.get 0
        local.get 2
        i64.load offset=48
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=160
        local.get 2
        i32.const 1
        i32.store offset=224
        local.get 2
        i32.const 0
        i32.store offset=228
        local.get 2
        i32.load offset=32
        local.set 14
        local.get 14
        local.get 14
        i64.load offset=8
        local.get 2
        i64.load offset=40
        local.get 0
        local.get 2
        i64.load offset=56
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=8
        local.get 2
        i32.const 1
        i32.store offset=232
        local.get 2
        i32.load offset=32
        local.set 15
        local.get 15
        local.get 15
        i64.load offset=48
        local.get 2
        i64.load offset=40
        local.get 0
        local.get 2
        i64.load offset=56
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=236
        local.get 2
        i32.load offset=32
        local.set 16
        local.get 16
        local.get 16
        i64.load offset=88
        local.get 2
        i64.load offset=40
        local.get 0
        local.get 2
        i64.load offset=56
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=88
        local.get 2
        i32.const 3
        i32.store offset=240
        local.get 2
        i32.load offset=32
        local.set 17
        local.get 17
        local.get 17
        i64.load offset=128
        local.get 2
        i64.load offset=40
        local.get 0
        local.get 2
        i64.load offset=56
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=128
        local.get 2
        i32.const 4
        i32.store offset=244
        local.get 2
        i32.load offset=32
        local.set 18
        local.get 18
        local.get 18
        i64.load offset=168
        local.get 2
        i64.load offset=40
        local.get 0
        local.get 2
        i64.load offset=56
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=168
        local.get 2
        i32.const 2
        i32.store offset=248
        local.get 2
        i32.const 0
        i32.store offset=252
        local.get 2
        i32.load offset=32
        local.set 19
        local.get 19
        local.get 19
        i64.load offset=16
        local.get 2
        i64.load offset=48
        local.get 0
        local.get 2
        i64.load offset=64
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=16
        local.get 2
        i32.const 1
        i32.store offset=256
        local.get 2
        i32.load offset=32
        local.set 20
        local.get 20
        local.get 20
        i64.load offset=56
        local.get 2
        i64.load offset=48
        local.get 0
        local.get 2
        i64.load offset=64
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 2
        i32.store offset=260
        local.get 2
        i32.load offset=32
        local.set 21
        local.get 21
        local.get 21
        i64.load offset=96
        local.get 2
        i64.load offset=48
        local.get 0
        local.get 2
        i64.load offset=64
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=96
        local.get 2
        i32.const 3
        i32.store offset=264
        local.get 2
        i32.load offset=32
        local.set 22
        local.get 22
        local.get 22
        i64.load offset=136
        local.get 2
        i64.load offset=48
        local.get 0
        local.get 2
        i64.load offset=64
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=136
        local.get 2
        i32.const 4
        i32.store offset=268
        local.get 2
        i32.load offset=32
        local.set 23
        local.get 23
        local.get 23
        i64.load offset=176
        local.get 2
        i64.load offset=48
        local.get 0
        local.get 2
        i64.load offset=64
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=176
        local.get 2
        i32.const 3
        i32.store offset=272
        local.get 2
        i32.const 0
        i32.store offset=276
        local.get 2
        i32.load offset=32
        local.set 24
        local.get 24
        local.get 24
        i64.load offset=24
        local.get 2
        i64.load offset=56
        local.get 0
        local.get 2
        i64.load offset=72
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=24
        local.get 2
        i32.const 1
        i32.store offset=280
        local.get 2
        i32.load offset=32
        local.set 25
        local.get 25
        local.get 25
        i64.load offset=64
        local.get 2
        i64.load offset=56
        local.get 0
        local.get 2
        i64.load offset=72
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 2
        i32.store offset=284
        local.get 2
        i32.load offset=32
        local.set 26
        local.get 26
        local.get 26
        i64.load offset=104
        local.get 2
        i64.load offset=56
        local.get 0
        local.get 2
        i64.load offset=72
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=104
        local.get 2
        i32.const 3
        i32.store offset=288
        local.get 2
        i32.load offset=32
        local.set 27
        local.get 27
        local.get 27
        i64.load offset=144
        local.get 2
        i64.load offset=56
        local.get 0
        local.get 2
        i64.load offset=72
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=144
        local.get 2
        i32.const 4
        i32.store offset=292
        local.get 2
        i32.load offset=32
        local.set 28
        local.get 28
        local.get 28
        i64.load offset=184
        local.get 2
        i64.load offset=56
        local.get 0
        local.get 2
        i64.load offset=72
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=184
        local.get 2
        i32.const 4
        i32.store offset=296
        local.get 2
        i32.const 0
        i32.store offset=300
        local.get 2
        i32.load offset=32
        local.set 29
        local.get 29
        local.get 29
        i64.load offset=32
        local.get 2
        i64.load offset=64
        local.get 0
        local.get 2
        i64.load offset=40
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=32
        local.get 2
        i32.const 1
        i32.store offset=304
        local.get 2
        i32.load offset=32
        local.set 30
        local.get 30
        local.get 30
        i64.load offset=72
        local.get 2
        i64.load offset=64
        local.get 0
        local.get 2
        i64.load offset=40
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 2
        i32.store offset=308
        local.get 2
        i32.load offset=32
        local.set 31
        local.get 31
        local.get 31
        i64.load offset=112
        local.get 2
        i64.load offset=64
        local.get 0
        local.get 2
        i64.load offset=40
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=112
        local.get 2
        i32.const 3
        i32.store offset=312
        local.get 2
        i32.load offset=32
        local.set 32
        local.get 32
        local.get 32
        i64.load offset=152
        local.get 2
        i64.load offset=64
        local.get 0
        local.get 2
        i64.load offset=40
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=152
        local.get 2
        i32.const 4
        i32.store offset=316
        local.get 2
        i32.load offset=32
        local.set 33
        local.get 33
        local.get 33
        i64.load offset=192
        local.get 2
        i64.load offset=64
        local.get 0
        local.get 2
        i64.load offset=40
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=192
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=8
        i64.store offset=320
        local.get 2
        i32.const 0
        i32.store offset=328
        local.get 2
        i32.const 10
        i32.store8 offset=335
        local.get 2
        i32.load offset=32
        i64.load offset=80
        local.set 34
        local.get 2
        local.get 34
        i64.store offset=336
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8631
        i64.store offset=80
        local.get 2
        local.get 34
        i64.store offset=320
        local.get 2
        i32.const 1
        i32.store offset=344
        local.get 2
        i32.const 7
        i32.store8 offset=351
        local.get 2
        i32.load offset=32
        i64.load offset=56
        local.set 35
        local.get 2
        local.get 35
        i64.store offset=352
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8636
        i64.store offset=56
        local.get 2
        local.get 35
        i64.store offset=320
        local.get 2
        i32.const 2
        i32.store offset=360
        local.get 2
        i32.const 11
        i32.store8 offset=367
        local.get 2
        i32.load offset=32
        i64.load offset=88
        local.set 36
        local.get 2
        local.get 36
        i64.store offset=368
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8637
        i64.store offset=88
        local.get 2
        local.get 36
        i64.store offset=320
        local.get 2
        i32.const 3
        i32.store offset=376
        local.get 2
        i32.const 17
        i32.store8 offset=383
        local.get 2
        i32.load offset=32
        i64.load offset=136
        local.set 37
        local.get 2
        local.get 37
        i64.store offset=384
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8638
        i64.store offset=136
        local.get 2
        local.get 37
        i64.store offset=320
        local.get 2
        i32.const 4
        i32.store offset=392
        local.get 2
        i32.const 18
        i32.store8 offset=399
        local.get 2
        i32.load offset=32
        i64.load offset=144
        local.set 38
        local.get 2
        local.get 38
        i64.store offset=400
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8639
        i64.store offset=144
        local.get 2
        local.get 38
        i64.store offset=320
        local.get 2
        i32.const 5
        i32.store offset=408
        local.get 2
        i32.const 3
        i32.store8 offset=415
        local.get 2
        i32.load offset=32
        i64.load offset=24
        local.set 39
        local.get 2
        local.get 39
        i64.store offset=416
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8640
        i64.store offset=24
        local.get 2
        local.get 39
        i64.store offset=320
        local.get 2
        i32.const 6
        i32.store offset=424
        local.get 2
        i32.const 5
        i32.store8 offset=431
        local.get 2
        i32.load offset=32
        i64.load offset=40
        local.set 40
        local.get 2
        local.get 40
        i64.store offset=432
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8641
        i64.store offset=40
        local.get 2
        local.get 40
        i64.store offset=320
        local.get 2
        i32.const 7
        i32.store offset=440
        local.get 2
        i32.const 16
        i32.store8 offset=447
        local.get 2
        i32.load offset=32
        i64.load offset=128
        local.set 41
        local.get 2
        local.get 41
        i64.store offset=448
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8642
        i64.store offset=128
        local.get 2
        local.get 41
        i64.store offset=320
        i32.const 8
        local.set 42
        local.get 2
        local.get 42
        i32.store offset=456
        local.get 2
        local.get 42
        i32.store8 offset=463
        local.get 2
        i32.load offset=32
        i64.load offset=64
        local.set 43
        local.get 2
        local.get 43
        i64.store offset=464
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8643
        i64.store offset=64
        local.get 2
        local.get 43
        i64.store offset=320
        local.get 2
        i32.const 9
        i32.store offset=472
        local.get 2
        i32.const 21
        i32.store8 offset=479
        local.get 2
        i32.load offset=32
        i64.load offset=168
        local.set 44
        local.get 2
        local.get 44
        i64.store offset=480
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8644
        i64.store offset=168
        local.get 2
        local.get 44
        i64.store offset=320
        local.get 2
        i32.const 10
        i32.store offset=488
        local.get 2
        i32.const 24
        i32.store8 offset=495
        local.get 2
        i32.load offset=32
        i64.load offset=192
        local.set 45
        local.get 2
        local.get 45
        i64.store offset=496
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8645
        i64.store offset=192
        local.get 2
        local.get 45
        i64.store offset=320
        local.get 2
        i32.const 11
        i32.store offset=504
        local.get 2
        i32.const 4
        i32.store8 offset=511
        local.get 2
        i32.load offset=32
        i64.load offset=32
        local.set 46
        local.get 2
        local.get 46
        i64.store offset=512
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8646
        i64.store offset=32
        local.get 2
        local.get 46
        i64.store offset=320
        local.get 2
        i32.const 12
        i32.store offset=520
        local.get 2
        i32.const 15
        i32.store8 offset=527
        local.get 2
        i32.load offset=32
        i64.load offset=120
        local.set 47
        local.get 2
        local.get 47
        i64.store offset=528
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8647
        i64.store offset=120
        local.get 2
        local.get 47
        i64.store offset=320
        local.get 2
        i32.const 13
        i32.store offset=536
        local.get 2
        i32.const 23
        i32.store8 offset=543
        local.get 2
        i32.load offset=32
        i64.load offset=184
        local.set 48
        local.get 2
        local.get 48
        i64.store offset=544
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8648
        i64.store offset=184
        local.get 2
        local.get 48
        i64.store offset=320
        local.get 2
        i32.const 14
        i32.store offset=552
        local.get 2
        i32.const 19
        i32.store8 offset=559
        local.get 2
        i32.load offset=32
        i64.load offset=152
        local.set 49
        local.get 2
        local.get 49
        i64.store offset=560
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8649
        i64.store offset=152
        local.get 2
        local.get 49
        i64.store offset=320
        local.get 2
        i32.const 15
        i32.store offset=568
        local.get 2
        i32.const 13
        i32.store8 offset=575
        local.get 2
        i32.load offset=32
        i64.load offset=104
        local.set 50
        local.get 2
        local.get 50
        i64.store offset=576
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8650
        i64.store offset=104
        local.get 2
        local.get 50
        i64.store offset=320
        local.get 2
        i32.const 16
        i32.store offset=584
        local.get 2
        i32.const 12
        i32.store8 offset=591
        local.get 2
        i32.load offset=32
        i64.load offset=96
        local.set 51
        local.get 2
        local.get 51
        i64.store offset=592
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8651
        i64.store offset=96
        local.get 2
        local.get 51
        i64.store offset=320
        local.get 2
        i32.const 17
        i32.store offset=600
        local.get 2
        i32.const 2
        i32.store8 offset=607
        local.get 2
        i32.load offset=32
        i64.load offset=16
        local.set 52
        local.get 2
        local.get 52
        i64.store offset=608
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8652
        i64.store offset=16
        local.get 2
        local.get 52
        i64.store offset=320
        local.get 2
        i32.const 18
        i32.store offset=616
        local.get 2
        i32.const 20
        i32.store8 offset=623
        local.get 2
        i32.load offset=32
        i64.load offset=160
        local.set 53
        local.get 2
        local.get 53
        i64.store offset=624
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8653
        i64.store offset=160
        local.get 2
        local.get 53
        i64.store offset=320
        local.get 2
        i32.const 19
        i32.store offset=632
        local.get 2
        i32.const 14
        i32.store8 offset=639
        local.get 2
        i32.load offset=32
        i64.load offset=112
        local.set 54
        local.get 2
        local.get 54
        i64.store offset=640
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8654
        i64.store offset=112
        local.get 2
        local.get 54
        i64.store offset=320
        local.get 2
        i32.const 20
        i32.store offset=648
        local.get 2
        i32.const 22
        i32.store8 offset=655
        local.get 2
        i32.load offset=32
        i64.load offset=176
        local.set 55
        local.get 2
        local.get 55
        i64.store offset=656
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8655
        i64.store offset=176
        local.get 2
        local.get 55
        i64.store offset=320
        local.get 2
        i32.const 21
        i32.store offset=664
        local.get 2
        i32.const 9
        i32.store8 offset=671
        local.get 2
        i32.load offset=32
        i64.load offset=72
        local.set 56
        local.get 2
        local.get 56
        i64.store offset=672
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8656
        i64.store offset=72
        local.get 2
        local.get 56
        i64.store offset=320
        local.get 2
        i32.const 22
        i32.store offset=680
        local.get 2
        i32.const 6
        i32.store8 offset=687
        local.get 2
        i32.load offset=32
        i64.load offset=48
        local.set 57
        local.get 2
        local.get 57
        i64.store offset=688
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8657
        i64.store offset=48
        local.get 2
        local.get 57
        i64.store offset=320
        local.get 2
        i32.const 23
        i32.store offset=696
        local.get 2
        i32.const 1
        i32.store8 offset=703
        local.get 2
        i32.load offset=32
        i64.load offset=8
        local.set 58
        local.get 2
        local.get 58
        i64.store offset=704
        local.get 2
        i32.load offset=32
        local.get 0
        local.get 2
        i64.load offset=320
        call $math.rotl__anon_8658
        i64.store offset=8
        local.get 2
        local.get 58
        i64.store offset=320
        local.get 2
        i32.const 0
        i32.store offset=712
        local.get 2
        i32.const 0
        i32.store offset=716
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=720
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=8
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=724
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=16
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=728
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=24
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=732
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=32
        i64.store offset=72
        local.get 2
        i32.const 0
        i32.store offset=736
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=40
        local.get 2
        i64.load offset=48
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=56
        i64.and
        i64.xor
        i64.store
        local.get 2
        i32.const 1
        i32.store offset=740
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=48
        local.get 2
        i64.load offset=56
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=64
        i64.and
        i64.xor
        i64.store offset=8
        local.get 2
        i32.const 2
        i32.store offset=744
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=56
        local.get 2
        i64.load offset=64
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=72
        i64.and
        i64.xor
        i64.store offset=16
        local.get 2
        i32.const 3
        i32.store offset=748
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=64
        local.get 2
        i64.load offset=72
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=40
        i64.and
        i64.xor
        i64.store offset=24
        local.get 2
        i32.const 4
        i32.store offset=752
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=72
        local.get 2
        i64.load offset=40
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=48
        i64.and
        i64.xor
        i64.store offset=32
        local.get 2
        i32.const 1
        i32.store offset=756
        local.get 2
        i32.const 0
        i32.store offset=760
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=40
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=764
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=48
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=768
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=56
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=772
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=64
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=776
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=72
        i64.store offset=72
        local.get 2
        i32.const 0
        i32.store offset=780
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=40
        local.get 2
        i64.load offset=48
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=56
        i64.and
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=784
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=48
        local.get 2
        i64.load offset=56
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=64
        i64.and
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=788
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=56
        local.get 2
        i64.load offset=64
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=72
        i64.and
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=792
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=64
        local.get 2
        i64.load offset=72
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=40
        i64.and
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=796
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=72
        local.get 2
        i64.load offset=40
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=48
        i64.and
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 2
        i32.store offset=800
        local.get 2
        i32.const 0
        i32.store offset=804
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=80
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=808
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=88
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=812
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=96
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=816
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=104
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=820
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=112
        i64.store offset=72
        local.get 2
        i32.const 0
        i32.store offset=824
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=40
        local.get 2
        i64.load offset=48
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=56
        i64.and
        i64.xor
        i64.store offset=80
        local.get 2
        i32.const 1
        i32.store offset=828
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=48
        local.get 2
        i64.load offset=56
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=64
        i64.and
        i64.xor
        i64.store offset=88
        local.get 2
        i32.const 2
        i32.store offset=832
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=56
        local.get 2
        i64.load offset=64
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=72
        i64.and
        i64.xor
        i64.store offset=96
        local.get 2
        i32.const 3
        i32.store offset=836
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=64
        local.get 2
        i64.load offset=72
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=40
        i64.and
        i64.xor
        i64.store offset=104
        local.get 2
        i32.const 4
        i32.store offset=840
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=72
        local.get 2
        i64.load offset=40
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=48
        i64.and
        i64.xor
        i64.store offset=112
        local.get 2
        i32.const 3
        i32.store offset=844
        local.get 2
        i32.const 0
        i32.store offset=848
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=120
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=852
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=128
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=856
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=136
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=860
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=144
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=864
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=152
        i64.store offset=72
        local.get 2
        i32.const 0
        i32.store offset=868
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=40
        local.get 2
        i64.load offset=48
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=56
        i64.and
        i64.xor
        i64.store offset=120
        local.get 2
        i32.const 1
        i32.store offset=872
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=48
        local.get 2
        i64.load offset=56
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=64
        i64.and
        i64.xor
        i64.store offset=128
        local.get 2
        i32.const 2
        i32.store offset=876
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=56
        local.get 2
        i64.load offset=64
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=72
        i64.and
        i64.xor
        i64.store offset=136
        local.get 2
        i32.const 3
        i32.store offset=880
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=64
        local.get 2
        i64.load offset=72
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=40
        i64.and
        i64.xor
        i64.store offset=144
        local.get 2
        i32.const 4
        i32.store offset=884
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=72
        local.get 2
        i64.load offset=40
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=48
        i64.and
        i64.xor
        i64.store offset=152
        local.get 2
        i32.const 4
        i32.store offset=888
        local.get 2
        i32.const 0
        i32.store offset=892
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=160
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=896
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=168
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=900
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=176
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=904
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=184
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=908
        local.get 2
        local.get 2
        i32.load offset=32
        i64.load offset=192
        i64.store offset=72
        local.get 2
        i32.const 0
        i32.store offset=912
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=40
        local.get 2
        i64.load offset=48
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=56
        i64.and
        i64.xor
        i64.store offset=160
        local.get 2
        i32.const 1
        i32.store offset=916
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=48
        local.get 2
        i64.load offset=56
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=64
        i64.and
        i64.xor
        i64.store offset=168
        local.get 2
        i32.const 2
        i32.store offset=920
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=56
        local.get 2
        i64.load offset=64
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=72
        i64.and
        i64.xor
        i64.store offset=176
        local.get 2
        i32.const 3
        i32.store offset=924
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=64
        local.get 2
        i64.load offset=72
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=40
        i64.and
        i64.xor
        i64.store offset=184
        local.get 2
        i32.const 4
        i32.store offset=928
        local.get 2
        i32.load offset=32
        local.get 2
        i64.load offset=72
        local.get 2
        i64.load offset=40
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=48
        i64.and
        i64.xor
        i64.store offset=192
        local.get 2
        i32.load offset=32
        local.set 59
        local.get 59
        local.get 59
        i64.load
        local.get 5
        i64.xor
        i64.store
        local.get 2
        i32.load offset=4
        local.set 60
        local.get 2
        i32.load offset=8
        i32.const 1
        i32.add
        local.set 61
        block  ;; label = @3
          local.get 61
          i32.eqz
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        block  ;; label = @3
          block  ;; label = @4
            local.get 61
            i32.const 24
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 61
          i32.const 24
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        i32.const 1049408
        local.get 61
        i32.const 3
        i32.shl
        i32.add
        i64.load
        local.set 62
        local.get 2
        local.get 60
        i32.store offset=932
        local.get 2
        local.get 62
        i64.store offset=936
        local.get 2
        local.get 60
        i32.store offset=948
        local.get 2
        i32.load offset=948
        local.set 63
        local.get 2
        local.get 63
        i32.store offset=952
        local.get 2
        local.get 63
        i32.store offset=956
        local.get 2
        i32.const 992
        i32.add
        local.set 64
        i64.const 0
        local.set 65
        local.get 64
        local.get 65
        i64.store
        local.get 2
        i32.const 984
        i32.add
        local.get 65
        i64.store
        local.get 2
        i32.const 976
        i32.add
        local.get 65
        i64.store
        local.get 2
        i32.const 968
        i32.add
        local.get 65
        i64.store
        local.get 2
        local.get 65
        i64.store offset=960
        local.get 2
        i32.const 0
        i32.store offset=1000
        local.get 2
        i32.const 0
        i32.store offset=1004
        local.get 2
        local.get 2
        i64.load offset=960
        local.get 2
        i32.load offset=952
        i64.load
        i64.xor
        i64.store offset=960
        local.get 2
        i32.const 1
        i32.store offset=1008
        local.get 2
        local.get 2
        i64.load offset=960
        local.get 2
        i32.load offset=952
        i64.load offset=40
        i64.xor
        i64.store offset=960
        local.get 2
        i32.const 2
        i32.store offset=1012
        local.get 2
        local.get 2
        i64.load offset=960
        local.get 2
        i32.load offset=952
        i64.load offset=80
        i64.xor
        i64.store offset=960
        local.get 2
        i32.const 3
        i32.store offset=1016
        local.get 2
        local.get 2
        i64.load offset=960
        local.get 2
        i32.load offset=952
        i64.load offset=120
        i64.xor
        i64.store offset=960
        local.get 2
        i32.const 4
        i32.store offset=1020
        local.get 2
        local.get 2
        i64.load offset=960
        local.get 2
        i32.load offset=952
        i64.load offset=160
        i64.xor
        i64.store offset=960
        local.get 2
        i32.const 1
        i32.store offset=1024
        local.get 2
        i32.const 0
        i32.store offset=1028
        local.get 2
        local.get 2
        i64.load offset=968
        local.get 2
        i32.load offset=952
        i64.load offset=8
        i64.xor
        i64.store offset=968
        local.get 2
        i32.const 1
        i32.store offset=1032
        local.get 2
        local.get 2
        i64.load offset=968
        local.get 2
        i32.load offset=952
        i64.load offset=48
        i64.xor
        i64.store offset=968
        local.get 2
        i32.const 2
        i32.store offset=1036
        local.get 2
        local.get 2
        i64.load offset=968
        local.get 2
        i32.load offset=952
        i64.load offset=88
        i64.xor
        i64.store offset=968
        local.get 2
        i32.const 3
        i32.store offset=1040
        local.get 2
        local.get 2
        i64.load offset=968
        local.get 2
        i32.load offset=952
        i64.load offset=128
        i64.xor
        i64.store offset=968
        local.get 2
        i32.const 4
        i32.store offset=1044
        local.get 2
        local.get 2
        i64.load offset=968
        local.get 2
        i32.load offset=952
        i64.load offset=168
        i64.xor
        i64.store offset=968
        local.get 2
        i32.const 2
        i32.store offset=1048
        local.get 2
        i32.const 0
        i32.store offset=1052
        local.get 2
        local.get 2
        i64.load offset=976
        local.get 2
        i32.load offset=952
        i64.load offset=16
        i64.xor
        i64.store offset=976
        local.get 2
        i32.const 1
        i32.store offset=1056
        local.get 2
        local.get 2
        i64.load offset=976
        local.get 2
        i32.load offset=952
        i64.load offset=56
        i64.xor
        i64.store offset=976
        local.get 2
        i32.const 2
        i32.store offset=1060
        local.get 2
        local.get 2
        i64.load offset=976
        local.get 2
        i32.load offset=952
        i64.load offset=96
        i64.xor
        i64.store offset=976
        local.get 2
        i32.const 3
        i32.store offset=1064
        local.get 2
        local.get 2
        i64.load offset=976
        local.get 2
        i32.load offset=952
        i64.load offset=136
        i64.xor
        i64.store offset=976
        local.get 2
        i32.const 4
        i32.store offset=1068
        local.get 2
        local.get 2
        i64.load offset=976
        local.get 2
        i32.load offset=952
        i64.load offset=176
        i64.xor
        i64.store offset=976
        local.get 2
        i32.const 3
        i32.store offset=1072
        local.get 2
        i32.const 0
        i32.store offset=1076
        local.get 2
        local.get 2
        i64.load offset=984
        local.get 2
        i32.load offset=952
        i64.load offset=24
        i64.xor
        i64.store offset=984
        local.get 2
        i32.const 1
        i32.store offset=1080
        local.get 2
        local.get 2
        i64.load offset=984
        local.get 2
        i32.load offset=952
        i64.load offset=64
        i64.xor
        i64.store offset=984
        local.get 2
        i32.const 2
        i32.store offset=1084
        local.get 2
        local.get 2
        i64.load offset=984
        local.get 2
        i32.load offset=952
        i64.load offset=104
        i64.xor
        i64.store offset=984
        local.get 2
        i32.const 3
        i32.store offset=1088
        local.get 2
        local.get 2
        i64.load offset=984
        local.get 2
        i32.load offset=952
        i64.load offset=144
        i64.xor
        i64.store offset=984
        local.get 2
        i32.const 4
        i32.store offset=1092
        local.get 2
        local.get 2
        i64.load offset=984
        local.get 2
        i32.load offset=952
        i64.load offset=184
        i64.xor
        i64.store offset=984
        local.get 2
        i32.const 4
        i32.store offset=1096
        local.get 2
        i32.const 0
        i32.store offset=1100
        local.get 2
        local.get 2
        i64.load offset=992
        local.get 2
        i32.load offset=952
        i64.load offset=32
        i64.xor
        i64.store offset=992
        local.get 2
        i32.const 1
        i32.store offset=1104
        local.get 2
        local.get 2
        i64.load offset=992
        local.get 2
        i32.load offset=952
        i64.load offset=72
        i64.xor
        i64.store offset=992
        local.get 2
        i32.const 2
        i32.store offset=1108
        local.get 2
        local.get 2
        i64.load offset=992
        local.get 2
        i32.load offset=952
        i64.load offset=112
        i64.xor
        i64.store offset=992
        local.get 2
        i32.const 3
        i32.store offset=1112
        local.get 2
        local.get 2
        i64.load offset=992
        local.get 2
        i32.load offset=952
        i64.load offset=152
        i64.xor
        i64.store offset=992
        local.get 2
        i32.const 4
        i32.store offset=1116
        local.get 2
        local.get 2
        i64.load offset=992
        local.get 2
        i32.load offset=952
        i64.load offset=192
        i64.xor
        i64.store offset=992
        local.get 2
        i32.const 0
        i32.store offset=1120
        local.get 2
        i32.const 0
        i32.store offset=1124
        local.get 2
        i32.load offset=952
        local.set 66
        local.get 66
        local.get 66
        i64.load
        local.get 2
        i64.load offset=992
        local.get 0
        local.get 2
        i64.load offset=968
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store
        local.get 2
        i32.const 1
        i32.store offset=1128
        local.get 2
        i32.load offset=952
        local.set 67
        local.get 67
        local.get 67
        i64.load offset=40
        local.get 2
        i64.load offset=992
        local.get 0
        local.get 2
        i64.load offset=968
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 2
        i32.store offset=1132
        local.get 2
        i32.load offset=952
        local.set 68
        local.get 68
        local.get 68
        i64.load offset=80
        local.get 2
        i64.load offset=992
        local.get 0
        local.get 2
        i64.load offset=968
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=80
        local.get 2
        i32.const 3
        i32.store offset=1136
        local.get 2
        i32.load offset=952
        local.set 69
        local.get 69
        local.get 69
        i64.load offset=120
        local.get 2
        i64.load offset=992
        local.get 0
        local.get 2
        i64.load offset=968
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=120
        local.get 2
        i32.const 4
        i32.store offset=1140
        local.get 2
        i32.load offset=952
        local.set 70
        local.get 70
        local.get 70
        i64.load offset=160
        local.get 2
        i64.load offset=992
        local.get 0
        local.get 2
        i64.load offset=968
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=160
        local.get 2
        i32.const 1
        i32.store offset=1144
        local.get 2
        i32.const 0
        i32.store offset=1148
        local.get 2
        i32.load offset=952
        local.set 71
        local.get 71
        local.get 71
        i64.load offset=8
        local.get 2
        i64.load offset=960
        local.get 0
        local.get 2
        i64.load offset=976
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=8
        local.get 2
        i32.const 1
        i32.store offset=1152
        local.get 2
        i32.load offset=952
        local.set 72
        local.get 72
        local.get 72
        i64.load offset=48
        local.get 2
        i64.load offset=960
        local.get 0
        local.get 2
        i64.load offset=976
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=1156
        local.get 2
        i32.load offset=952
        local.set 73
        local.get 73
        local.get 73
        i64.load offset=88
        local.get 2
        i64.load offset=960
        local.get 0
        local.get 2
        i64.load offset=976
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=88
        local.get 2
        i32.const 3
        i32.store offset=1160
        local.get 2
        i32.load offset=952
        local.set 74
        local.get 74
        local.get 74
        i64.load offset=128
        local.get 2
        i64.load offset=960
        local.get 0
        local.get 2
        i64.load offset=976
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=128
        local.get 2
        i32.const 4
        i32.store offset=1164
        local.get 2
        i32.load offset=952
        local.set 75
        local.get 75
        local.get 75
        i64.load offset=168
        local.get 2
        i64.load offset=960
        local.get 0
        local.get 2
        i64.load offset=976
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=168
        local.get 2
        i32.const 2
        i32.store offset=1168
        local.get 2
        i32.const 0
        i32.store offset=1172
        local.get 2
        i32.load offset=952
        local.set 76
        local.get 76
        local.get 76
        i64.load offset=16
        local.get 2
        i64.load offset=968
        local.get 0
        local.get 2
        i64.load offset=984
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=16
        local.get 2
        i32.const 1
        i32.store offset=1176
        local.get 2
        i32.load offset=952
        local.set 77
        local.get 77
        local.get 77
        i64.load offset=56
        local.get 2
        i64.load offset=968
        local.get 0
        local.get 2
        i64.load offset=984
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 2
        i32.store offset=1180
        local.get 2
        i32.load offset=952
        local.set 78
        local.get 78
        local.get 78
        i64.load offset=96
        local.get 2
        i64.load offset=968
        local.get 0
        local.get 2
        i64.load offset=984
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=96
        local.get 2
        i32.const 3
        i32.store offset=1184
        local.get 2
        i32.load offset=952
        local.set 79
        local.get 79
        local.get 79
        i64.load offset=136
        local.get 2
        i64.load offset=968
        local.get 0
        local.get 2
        i64.load offset=984
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=136
        local.get 2
        i32.const 4
        i32.store offset=1188
        local.get 2
        i32.load offset=952
        local.set 80
        local.get 80
        local.get 80
        i64.load offset=176
        local.get 2
        i64.load offset=968
        local.get 0
        local.get 2
        i64.load offset=984
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=176
        local.get 2
        i32.const 3
        i32.store offset=1192
        local.get 2
        i32.const 0
        i32.store offset=1196
        local.get 2
        i32.load offset=952
        local.set 81
        local.get 81
        local.get 81
        i64.load offset=24
        local.get 2
        i64.load offset=976
        local.get 0
        local.get 2
        i64.load offset=992
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=24
        local.get 2
        i32.const 1
        i32.store offset=1200
        local.get 2
        i32.load offset=952
        local.set 82
        local.get 82
        local.get 82
        i64.load offset=64
        local.get 2
        i64.load offset=976
        local.get 0
        local.get 2
        i64.load offset=992
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 2
        i32.store offset=1204
        local.get 2
        i32.load offset=952
        local.set 83
        local.get 83
        local.get 83
        i64.load offset=104
        local.get 2
        i64.load offset=976
        local.get 0
        local.get 2
        i64.load offset=992
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=104
        local.get 2
        i32.const 3
        i32.store offset=1208
        local.get 2
        i32.load offset=952
        local.set 84
        local.get 84
        local.get 84
        i64.load offset=144
        local.get 2
        i64.load offset=976
        local.get 0
        local.get 2
        i64.load offset=992
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=144
        local.get 2
        i32.const 4
        i32.store offset=1212
        local.get 2
        i32.load offset=952
        local.set 85
        local.get 85
        local.get 85
        i64.load offset=184
        local.get 2
        i64.load offset=976
        local.get 0
        local.get 2
        i64.load offset=992
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=184
        local.get 2
        i32.const 4
        i32.store offset=1216
        local.get 2
        i32.const 0
        i32.store offset=1220
        local.get 2
        i32.load offset=952
        local.set 86
        local.get 86
        local.get 86
        i64.load offset=32
        local.get 2
        i64.load offset=984
        local.get 0
        local.get 2
        i64.load offset=960
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=32
        local.get 2
        i32.const 1
        i32.store offset=1224
        local.get 2
        i32.load offset=952
        local.set 87
        local.get 87
        local.get 87
        i64.load offset=72
        local.get 2
        i64.load offset=984
        local.get 0
        local.get 2
        i64.load offset=960
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 2
        i32.store offset=1228
        local.get 2
        i32.load offset=952
        local.set 88
        local.get 88
        local.get 88
        i64.load offset=112
        local.get 2
        i64.load offset=984
        local.get 0
        local.get 2
        i64.load offset=960
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=112
        local.get 2
        i32.const 3
        i32.store offset=1232
        local.get 2
        i32.load offset=952
        local.set 89
        local.get 89
        local.get 89
        i64.load offset=152
        local.get 2
        i64.load offset=984
        local.get 0
        local.get 2
        i64.load offset=960
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=152
        local.get 2
        i32.const 4
        i32.store offset=1236
        local.get 2
        i32.load offset=952
        local.set 90
        local.get 90
        local.get 90
        i64.load offset=192
        local.get 2
        i64.load offset=984
        local.get 0
        local.get 2
        i64.load offset=960
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=192
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=8
        i64.store offset=1240
        local.get 2
        i32.const 0
        i32.store offset=1248
        local.get 2
        i32.const 10
        i32.store8 offset=1255
        local.get 2
        i32.load offset=952
        i64.load offset=80
        local.set 91
        local.get 2
        local.get 91
        i64.store offset=1256
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8631
        i64.store offset=80
        local.get 2
        local.get 91
        i64.store offset=1240
        local.get 2
        i32.const 1
        i32.store offset=1264
        local.get 2
        i32.const 7
        i32.store8 offset=1271
        local.get 2
        i32.load offset=952
        i64.load offset=56
        local.set 92
        local.get 2
        local.get 92
        i64.store offset=1272
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8636
        i64.store offset=56
        local.get 2
        local.get 92
        i64.store offset=1240
        local.get 2
        i32.const 2
        i32.store offset=1280
        local.get 2
        i32.const 11
        i32.store8 offset=1287
        local.get 2
        i32.load offset=952
        i64.load offset=88
        local.set 93
        local.get 2
        local.get 93
        i64.store offset=1288
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8637
        i64.store offset=88
        local.get 2
        local.get 93
        i64.store offset=1240
        local.get 2
        i32.const 3
        i32.store offset=1296
        local.get 2
        i32.const 17
        i32.store8 offset=1303
        local.get 2
        i32.load offset=952
        i64.load offset=136
        local.set 94
        local.get 2
        local.get 94
        i64.store offset=1304
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8638
        i64.store offset=136
        local.get 2
        local.get 94
        i64.store offset=1240
        local.get 2
        i32.const 4
        i32.store offset=1312
        local.get 2
        i32.const 18
        i32.store8 offset=1319
        local.get 2
        i32.load offset=952
        i64.load offset=144
        local.set 95
        local.get 2
        local.get 95
        i64.store offset=1320
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8639
        i64.store offset=144
        local.get 2
        local.get 95
        i64.store offset=1240
        local.get 2
        i32.const 5
        i32.store offset=1328
        local.get 2
        i32.const 3
        i32.store8 offset=1335
        local.get 2
        i32.load offset=952
        i64.load offset=24
        local.set 96
        local.get 2
        local.get 96
        i64.store offset=1336
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8640
        i64.store offset=24
        local.get 2
        local.get 96
        i64.store offset=1240
        local.get 2
        i32.const 6
        i32.store offset=1344
        local.get 2
        i32.const 5
        i32.store8 offset=1351
        local.get 2
        i32.load offset=952
        i64.load offset=40
        local.set 97
        local.get 2
        local.get 97
        i64.store offset=1352
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8641
        i64.store offset=40
        local.get 2
        local.get 97
        i64.store offset=1240
        local.get 2
        i32.const 7
        i32.store offset=1360
        local.get 2
        i32.const 16
        i32.store8 offset=1367
        local.get 2
        i32.load offset=952
        i64.load offset=128
        local.set 98
        local.get 2
        local.get 98
        i64.store offset=1368
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8642
        i64.store offset=128
        local.get 2
        local.get 98
        i64.store offset=1240
        i32.const 8
        local.set 99
        local.get 2
        local.get 99
        i32.store offset=1376
        local.get 2
        local.get 99
        i32.store8 offset=1383
        local.get 2
        i32.load offset=952
        i64.load offset=64
        local.set 100
        local.get 2
        local.get 100
        i64.store offset=1384
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8643
        i64.store offset=64
        local.get 2
        local.get 100
        i64.store offset=1240
        local.get 2
        i32.const 9
        i32.store offset=1392
        local.get 2
        i32.const 21
        i32.store8 offset=1399
        local.get 2
        i32.load offset=952
        i64.load offset=168
        local.set 101
        local.get 2
        local.get 101
        i64.store offset=1400
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8644
        i64.store offset=168
        local.get 2
        local.get 101
        i64.store offset=1240
        local.get 2
        i32.const 10
        i32.store offset=1408
        local.get 2
        i32.const 24
        i32.store8 offset=1415
        local.get 2
        i32.load offset=952
        i64.load offset=192
        local.set 102
        local.get 2
        local.get 102
        i64.store offset=1416
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8645
        i64.store offset=192
        local.get 2
        local.get 102
        i64.store offset=1240
        local.get 2
        i32.const 11
        i32.store offset=1424
        local.get 2
        i32.const 4
        i32.store8 offset=1431
        local.get 2
        i32.load offset=952
        i64.load offset=32
        local.set 103
        local.get 2
        local.get 103
        i64.store offset=1432
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8646
        i64.store offset=32
        local.get 2
        local.get 103
        i64.store offset=1240
        local.get 2
        i32.const 12
        i32.store offset=1440
        local.get 2
        i32.const 15
        i32.store8 offset=1447
        local.get 2
        i32.load offset=952
        i64.load offset=120
        local.set 104
        local.get 2
        local.get 104
        i64.store offset=1448
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8647
        i64.store offset=120
        local.get 2
        local.get 104
        i64.store offset=1240
        local.get 2
        i32.const 13
        i32.store offset=1456
        local.get 2
        i32.const 23
        i32.store8 offset=1463
        local.get 2
        i32.load offset=952
        i64.load offset=184
        local.set 105
        local.get 2
        local.get 105
        i64.store offset=1464
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8648
        i64.store offset=184
        local.get 2
        local.get 105
        i64.store offset=1240
        local.get 2
        i32.const 14
        i32.store offset=1472
        local.get 2
        i32.const 19
        i32.store8 offset=1479
        local.get 2
        i32.load offset=952
        i64.load offset=152
        local.set 106
        local.get 2
        local.get 106
        i64.store offset=1480
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8649
        i64.store offset=152
        local.get 2
        local.get 106
        i64.store offset=1240
        local.get 2
        i32.const 15
        i32.store offset=1488
        local.get 2
        i32.const 13
        i32.store8 offset=1495
        local.get 2
        i32.load offset=952
        i64.load offset=104
        local.set 107
        local.get 2
        local.get 107
        i64.store offset=1496
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8650
        i64.store offset=104
        local.get 2
        local.get 107
        i64.store offset=1240
        local.get 2
        i32.const 16
        i32.store offset=1504
        local.get 2
        i32.const 12
        i32.store8 offset=1511
        local.get 2
        i32.load offset=952
        i64.load offset=96
        local.set 108
        local.get 2
        local.get 108
        i64.store offset=1512
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8651
        i64.store offset=96
        local.get 2
        local.get 108
        i64.store offset=1240
        local.get 2
        i32.const 17
        i32.store offset=1520
        local.get 2
        i32.const 2
        i32.store8 offset=1527
        local.get 2
        i32.load offset=952
        i64.load offset=16
        local.set 109
        local.get 2
        local.get 109
        i64.store offset=1528
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8652
        i64.store offset=16
        local.get 2
        local.get 109
        i64.store offset=1240
        local.get 2
        i32.const 18
        i32.store offset=1536
        local.get 2
        i32.const 20
        i32.store8 offset=1543
        local.get 2
        i32.load offset=952
        i64.load offset=160
        local.set 110
        local.get 2
        local.get 110
        i64.store offset=1544
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8653
        i64.store offset=160
        local.get 2
        local.get 110
        i64.store offset=1240
        local.get 2
        i32.const 19
        i32.store offset=1552
        local.get 2
        i32.const 14
        i32.store8 offset=1559
        local.get 2
        i32.load offset=952
        i64.load offset=112
        local.set 111
        local.get 2
        local.get 111
        i64.store offset=1560
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8654
        i64.store offset=112
        local.get 2
        local.get 111
        i64.store offset=1240
        local.get 2
        i32.const 20
        i32.store offset=1568
        local.get 2
        i32.const 22
        i32.store8 offset=1575
        local.get 2
        i32.load offset=952
        i64.load offset=176
        local.set 112
        local.get 2
        local.get 112
        i64.store offset=1576
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8655
        i64.store offset=176
        local.get 2
        local.get 112
        i64.store offset=1240
        local.get 2
        i32.const 21
        i32.store offset=1584
        local.get 2
        i32.const 9
        i32.store8 offset=1591
        local.get 2
        i32.load offset=952
        i64.load offset=72
        local.set 113
        local.get 2
        local.get 113
        i64.store offset=1592
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8656
        i64.store offset=72
        local.get 2
        local.get 113
        i64.store offset=1240
        local.get 2
        i32.const 22
        i32.store offset=1600
        local.get 2
        i32.const 6
        i32.store8 offset=1607
        local.get 2
        i32.load offset=952
        i64.load offset=48
        local.set 114
        local.get 2
        local.get 114
        i64.store offset=1608
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8657
        i64.store offset=48
        local.get 2
        local.get 114
        i64.store offset=1240
        local.get 2
        i32.const 23
        i32.store offset=1616
        local.get 2
        i32.const 1
        i32.store8 offset=1623
        local.get 2
        i32.load offset=952
        i64.load offset=8
        local.set 115
        local.get 2
        local.get 115
        i64.store offset=1624
        local.get 2
        i32.load offset=952
        local.get 0
        local.get 2
        i64.load offset=1240
        call $math.rotl__anon_8658
        i64.store offset=8
        local.get 2
        local.get 115
        i64.store offset=1240
        local.get 2
        i32.const 0
        i32.store offset=1632
        local.get 2
        i32.const 0
        i32.store offset=1636
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load
        i64.store offset=960
        local.get 2
        i32.const 1
        i32.store offset=1640
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=8
        i64.store offset=968
        local.get 2
        i32.const 2
        i32.store offset=1644
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=16
        i64.store offset=976
        local.get 2
        i32.const 3
        i32.store offset=1648
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=24
        i64.store offset=984
        local.get 2
        i32.const 4
        i32.store offset=1652
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=32
        i64.store offset=992
        local.get 2
        i32.const 0
        i32.store offset=1656
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=960
        local.get 2
        i64.load offset=968
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=976
        i64.and
        i64.xor
        i64.store
        local.get 2
        i32.const 1
        i32.store offset=1660
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=968
        local.get 2
        i64.load offset=976
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=984
        i64.and
        i64.xor
        i64.store offset=8
        local.get 2
        i32.const 2
        i32.store offset=1664
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=976
        local.get 2
        i64.load offset=984
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=992
        i64.and
        i64.xor
        i64.store offset=16
        local.get 2
        i32.const 3
        i32.store offset=1668
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=984
        local.get 2
        i64.load offset=992
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=960
        i64.and
        i64.xor
        i64.store offset=24
        local.get 2
        i32.const 4
        i32.store offset=1672
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=992
        local.get 2
        i64.load offset=960
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=968
        i64.and
        i64.xor
        i64.store offset=32
        local.get 2
        i32.const 1
        i32.store offset=1676
        local.get 2
        i32.const 0
        i32.store offset=1680
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=40
        i64.store offset=960
        local.get 2
        i32.const 1
        i32.store offset=1684
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=48
        i64.store offset=968
        local.get 2
        i32.const 2
        i32.store offset=1688
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=56
        i64.store offset=976
        local.get 2
        i32.const 3
        i32.store offset=1692
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=64
        i64.store offset=984
        local.get 2
        i32.const 4
        i32.store offset=1696
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=72
        i64.store offset=992
        local.get 2
        i32.const 0
        i32.store offset=1700
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=960
        local.get 2
        i64.load offset=968
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=976
        i64.and
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=1704
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=968
        local.get 2
        i64.load offset=976
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=984
        i64.and
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=1708
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=976
        local.get 2
        i64.load offset=984
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=992
        i64.and
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=1712
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=984
        local.get 2
        i64.load offset=992
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=960
        i64.and
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=1716
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=992
        local.get 2
        i64.load offset=960
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=968
        i64.and
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 2
        i32.store offset=1720
        local.get 2
        i32.const 0
        i32.store offset=1724
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=80
        i64.store offset=960
        local.get 2
        i32.const 1
        i32.store offset=1728
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=88
        i64.store offset=968
        local.get 2
        i32.const 2
        i32.store offset=1732
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=96
        i64.store offset=976
        local.get 2
        i32.const 3
        i32.store offset=1736
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=104
        i64.store offset=984
        local.get 2
        i32.const 4
        i32.store offset=1740
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=112
        i64.store offset=992
        local.get 2
        i32.const 0
        i32.store offset=1744
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=960
        local.get 2
        i64.load offset=968
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=976
        i64.and
        i64.xor
        i64.store offset=80
        local.get 2
        i32.const 1
        i32.store offset=1748
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=968
        local.get 2
        i64.load offset=976
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=984
        i64.and
        i64.xor
        i64.store offset=88
        local.get 2
        i32.const 2
        i32.store offset=1752
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=976
        local.get 2
        i64.load offset=984
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=992
        i64.and
        i64.xor
        i64.store offset=96
        local.get 2
        i32.const 3
        i32.store offset=1756
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=984
        local.get 2
        i64.load offset=992
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=960
        i64.and
        i64.xor
        i64.store offset=104
        local.get 2
        i32.const 4
        i32.store offset=1760
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=992
        local.get 2
        i64.load offset=960
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=968
        i64.and
        i64.xor
        i64.store offset=112
        local.get 2
        i32.const 3
        i32.store offset=1764
        local.get 2
        i32.const 0
        i32.store offset=1768
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=120
        i64.store offset=960
        local.get 2
        i32.const 1
        i32.store offset=1772
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=128
        i64.store offset=968
        local.get 2
        i32.const 2
        i32.store offset=1776
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=136
        i64.store offset=976
        local.get 2
        i32.const 3
        i32.store offset=1780
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=144
        i64.store offset=984
        local.get 2
        i32.const 4
        i32.store offset=1784
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=152
        i64.store offset=992
        local.get 2
        i32.const 0
        i32.store offset=1788
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=960
        local.get 2
        i64.load offset=968
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=976
        i64.and
        i64.xor
        i64.store offset=120
        local.get 2
        i32.const 1
        i32.store offset=1792
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=968
        local.get 2
        i64.load offset=976
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=984
        i64.and
        i64.xor
        i64.store offset=128
        local.get 2
        i32.const 2
        i32.store offset=1796
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=976
        local.get 2
        i64.load offset=984
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=992
        i64.and
        i64.xor
        i64.store offset=136
        local.get 2
        i32.const 3
        i32.store offset=1800
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=984
        local.get 2
        i64.load offset=992
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=960
        i64.and
        i64.xor
        i64.store offset=144
        local.get 2
        i32.const 4
        i32.store offset=1804
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=992
        local.get 2
        i64.load offset=960
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=968
        i64.and
        i64.xor
        i64.store offset=152
        local.get 2
        i32.const 4
        i32.store offset=1808
        local.get 2
        i32.const 0
        i32.store offset=1812
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=160
        i64.store offset=960
        local.get 2
        i32.const 1
        i32.store offset=1816
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=168
        i64.store offset=968
        local.get 2
        i32.const 2
        i32.store offset=1820
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=176
        i64.store offset=976
        local.get 2
        i32.const 3
        i32.store offset=1824
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=184
        i64.store offset=984
        local.get 2
        i32.const 4
        i32.store offset=1828
        local.get 2
        local.get 2
        i32.load offset=952
        i64.load offset=192
        i64.store offset=992
        local.get 2
        i32.const 0
        i32.store offset=1832
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=960
        local.get 2
        i64.load offset=968
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=976
        i64.and
        i64.xor
        i64.store offset=160
        local.get 2
        i32.const 1
        i32.store offset=1836
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=968
        local.get 2
        i64.load offset=976
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=984
        i64.and
        i64.xor
        i64.store offset=168
        local.get 2
        i32.const 2
        i32.store offset=1840
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=976
        local.get 2
        i64.load offset=984
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=992
        i64.and
        i64.xor
        i64.store offset=176
        local.get 2
        i32.const 3
        i32.store offset=1844
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=984
        local.get 2
        i64.load offset=992
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=960
        i64.and
        i64.xor
        i64.store offset=184
        local.get 2
        i32.const 4
        i32.store offset=1848
        local.get 2
        i32.load offset=952
        local.get 2
        i64.load offset=992
        local.get 2
        i64.load offset=960
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=968
        i64.and
        i64.xor
        i64.store offset=192
        local.get 2
        i32.load offset=952
        local.set 116
        local.get 116
        local.get 116
        i64.load
        local.get 62
        i64.xor
        i64.store
        local.get 2
        i32.load offset=4
        local.set 117
        local.get 2
        i32.load offset=8
        local.set 118
        local.get 118
        i32.const 2
        i32.add
        local.set 119
        block  ;; label = @3
          local.get 119
          local.get 118
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        block  ;; label = @3
          block  ;; label = @4
            local.get 119
            i32.const 24
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 119
          i32.const 24
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        i32.const 1049408
        local.get 119
        i32.const 3
        i32.shl
        i32.add
        i64.load
        local.set 120
        local.get 2
        local.get 117
        i32.store offset=1852
        local.get 2
        local.get 120
        i64.store offset=1856
        local.get 2
        local.get 117
        i32.store offset=1868
        local.get 2
        i32.load offset=1868
        local.set 121
        local.get 2
        local.get 121
        i32.store offset=1872
        local.get 2
        local.get 121
        i32.store offset=1876
        local.get 2
        i32.const 1912
        i32.add
        local.set 122
        i64.const 0
        local.set 123
        local.get 122
        local.get 123
        i64.store
        local.get 2
        i32.const 1904
        i32.add
        local.get 123
        i64.store
        local.get 2
        i32.const 1896
        i32.add
        local.get 123
        i64.store
        local.get 2
        i32.const 1888
        i32.add
        local.get 123
        i64.store
        local.get 2
        local.get 123
        i64.store offset=1880
        local.get 2
        i32.const 0
        i32.store offset=1920
        local.get 2
        i32.const 0
        i32.store offset=1924
        local.get 2
        local.get 2
        i64.load offset=1880
        local.get 2
        i32.load offset=1872
        i64.load
        i64.xor
        i64.store offset=1880
        local.get 2
        i32.const 1
        i32.store offset=1928
        local.get 2
        local.get 2
        i64.load offset=1880
        local.get 2
        i32.load offset=1872
        i64.load offset=40
        i64.xor
        i64.store offset=1880
        local.get 2
        i32.const 2
        i32.store offset=1932
        local.get 2
        local.get 2
        i64.load offset=1880
        local.get 2
        i32.load offset=1872
        i64.load offset=80
        i64.xor
        i64.store offset=1880
        local.get 2
        i32.const 3
        i32.store offset=1936
        local.get 2
        local.get 2
        i64.load offset=1880
        local.get 2
        i32.load offset=1872
        i64.load offset=120
        i64.xor
        i64.store offset=1880
        local.get 2
        i32.const 4
        i32.store offset=1940
        local.get 2
        local.get 2
        i64.load offset=1880
        local.get 2
        i32.load offset=1872
        i64.load offset=160
        i64.xor
        i64.store offset=1880
        local.get 2
        i32.const 1
        i32.store offset=1944
        local.get 2
        i32.const 0
        i32.store offset=1948
        local.get 2
        local.get 2
        i64.load offset=1888
        local.get 2
        i32.load offset=1872
        i64.load offset=8
        i64.xor
        i64.store offset=1888
        local.get 2
        i32.const 1
        i32.store offset=1952
        local.get 2
        local.get 2
        i64.load offset=1888
        local.get 2
        i32.load offset=1872
        i64.load offset=48
        i64.xor
        i64.store offset=1888
        local.get 2
        i32.const 2
        i32.store offset=1956
        local.get 2
        local.get 2
        i64.load offset=1888
        local.get 2
        i32.load offset=1872
        i64.load offset=88
        i64.xor
        i64.store offset=1888
        local.get 2
        i32.const 3
        i32.store offset=1960
        local.get 2
        local.get 2
        i64.load offset=1888
        local.get 2
        i32.load offset=1872
        i64.load offset=128
        i64.xor
        i64.store offset=1888
        local.get 2
        i32.const 4
        i32.store offset=1964
        local.get 2
        local.get 2
        i64.load offset=1888
        local.get 2
        i32.load offset=1872
        i64.load offset=168
        i64.xor
        i64.store offset=1888
        local.get 2
        i32.const 2
        i32.store offset=1968
        local.get 2
        i32.const 0
        i32.store offset=1972
        local.get 2
        local.get 2
        i64.load offset=1896
        local.get 2
        i32.load offset=1872
        i64.load offset=16
        i64.xor
        i64.store offset=1896
        local.get 2
        i32.const 1
        i32.store offset=1976
        local.get 2
        local.get 2
        i64.load offset=1896
        local.get 2
        i32.load offset=1872
        i64.load offset=56
        i64.xor
        i64.store offset=1896
        local.get 2
        i32.const 2
        i32.store offset=1980
        local.get 2
        local.get 2
        i64.load offset=1896
        local.get 2
        i32.load offset=1872
        i64.load offset=96
        i64.xor
        i64.store offset=1896
        local.get 2
        i32.const 3
        i32.store offset=1984
        local.get 2
        local.get 2
        i64.load offset=1896
        local.get 2
        i32.load offset=1872
        i64.load offset=136
        i64.xor
        i64.store offset=1896
        local.get 2
        i32.const 4
        i32.store offset=1988
        local.get 2
        local.get 2
        i64.load offset=1896
        local.get 2
        i32.load offset=1872
        i64.load offset=176
        i64.xor
        i64.store offset=1896
        local.get 2
        i32.const 3
        i32.store offset=1992
        local.get 2
        i32.const 0
        i32.store offset=1996
        local.get 2
        local.get 2
        i64.load offset=1904
        local.get 2
        i32.load offset=1872
        i64.load offset=24
        i64.xor
        i64.store offset=1904
        local.get 2
        i32.const 1
        i32.store offset=2000
        local.get 2
        local.get 2
        i64.load offset=1904
        local.get 2
        i32.load offset=1872
        i64.load offset=64
        i64.xor
        i64.store offset=1904
        local.get 2
        i32.const 2
        i32.store offset=2004
        local.get 2
        local.get 2
        i64.load offset=1904
        local.get 2
        i32.load offset=1872
        i64.load offset=104
        i64.xor
        i64.store offset=1904
        local.get 2
        i32.const 3
        i32.store offset=2008
        local.get 2
        local.get 2
        i64.load offset=1904
        local.get 2
        i32.load offset=1872
        i64.load offset=144
        i64.xor
        i64.store offset=1904
        local.get 2
        i32.const 4
        i32.store offset=2012
        local.get 2
        local.get 2
        i64.load offset=1904
        local.get 2
        i32.load offset=1872
        i64.load offset=184
        i64.xor
        i64.store offset=1904
        local.get 2
        i32.const 4
        i32.store offset=2016
        local.get 2
        i32.const 0
        i32.store offset=2020
        local.get 2
        local.get 2
        i64.load offset=1912
        local.get 2
        i32.load offset=1872
        i64.load offset=32
        i64.xor
        i64.store offset=1912
        local.get 2
        i32.const 1
        i32.store offset=2024
        local.get 2
        local.get 2
        i64.load offset=1912
        local.get 2
        i32.load offset=1872
        i64.load offset=72
        i64.xor
        i64.store offset=1912
        local.get 2
        i32.const 2
        i32.store offset=2028
        local.get 2
        local.get 2
        i64.load offset=1912
        local.get 2
        i32.load offset=1872
        i64.load offset=112
        i64.xor
        i64.store offset=1912
        local.get 2
        i32.const 3
        i32.store offset=2032
        local.get 2
        local.get 2
        i64.load offset=1912
        local.get 2
        i32.load offset=1872
        i64.load offset=152
        i64.xor
        i64.store offset=1912
        local.get 2
        i32.const 4
        i32.store offset=2036
        local.get 2
        local.get 2
        i64.load offset=1912
        local.get 2
        i32.load offset=1872
        i64.load offset=192
        i64.xor
        i64.store offset=1912
        local.get 2
        i32.const 0
        i32.store offset=2040
        local.get 2
        i32.const 0
        i32.store offset=2044
        local.get 2
        i32.load offset=1872
        local.set 124
        local.get 124
        local.get 124
        i64.load
        local.get 2
        i64.load offset=1912
        local.get 0
        local.get 2
        i64.load offset=1888
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store
        local.get 2
        i32.const 1
        i32.store offset=2048
        local.get 2
        i32.load offset=1872
        local.set 125
        local.get 125
        local.get 125
        i64.load offset=40
        local.get 2
        i64.load offset=1912
        local.get 0
        local.get 2
        i64.load offset=1888
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 2
        i32.store offset=2052
        local.get 2
        i32.load offset=1872
        local.set 126
        local.get 126
        local.get 126
        i64.load offset=80
        local.get 2
        i64.load offset=1912
        local.get 0
        local.get 2
        i64.load offset=1888
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=80
        local.get 2
        i32.const 3
        i32.store offset=2056
        local.get 2
        i32.load offset=1872
        local.set 127
        local.get 127
        local.get 127
        i64.load offset=120
        local.get 2
        i64.load offset=1912
        local.get 0
        local.get 2
        i64.load offset=1888
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=120
        local.get 2
        i32.const 4
        i32.store offset=2060
        local.get 2
        i32.load offset=1872
        local.set 128
        local.get 128
        local.get 128
        i64.load offset=160
        local.get 2
        i64.load offset=1912
        local.get 0
        local.get 2
        i64.load offset=1888
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=160
        local.get 2
        i32.const 1
        i32.store offset=2064
        local.get 2
        i32.const 0
        i32.store offset=2068
        local.get 2
        i32.load offset=1872
        local.set 129
        local.get 129
        local.get 129
        i64.load offset=8
        local.get 2
        i64.load offset=1880
        local.get 0
        local.get 2
        i64.load offset=1896
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=8
        local.get 2
        i32.const 1
        i32.store offset=2072
        local.get 2
        i32.load offset=1872
        local.set 130
        local.get 130
        local.get 130
        i64.load offset=48
        local.get 2
        i64.load offset=1880
        local.get 0
        local.get 2
        i64.load offset=1896
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=2076
        local.get 2
        i32.load offset=1872
        local.set 131
        local.get 131
        local.get 131
        i64.load offset=88
        local.get 2
        i64.load offset=1880
        local.get 0
        local.get 2
        i64.load offset=1896
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=88
        local.get 2
        i32.const 3
        i32.store offset=2080
        local.get 2
        i32.load offset=1872
        local.set 132
        local.get 132
        local.get 132
        i64.load offset=128
        local.get 2
        i64.load offset=1880
        local.get 0
        local.get 2
        i64.load offset=1896
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=128
        local.get 2
        i32.const 4
        i32.store offset=2084
        local.get 2
        i32.load offset=1872
        local.set 133
        local.get 133
        local.get 133
        i64.load offset=168
        local.get 2
        i64.load offset=1880
        local.get 0
        local.get 2
        i64.load offset=1896
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=168
        local.get 2
        i32.const 2
        i32.store offset=2088
        local.get 2
        i32.const 0
        i32.store offset=2092
        local.get 2
        i32.load offset=1872
        local.set 134
        local.get 134
        local.get 134
        i64.load offset=16
        local.get 2
        i64.load offset=1888
        local.get 0
        local.get 2
        i64.load offset=1904
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=16
        local.get 2
        i32.const 1
        i32.store offset=2096
        local.get 2
        i32.load offset=1872
        local.set 135
        local.get 135
        local.get 135
        i64.load offset=56
        local.get 2
        i64.load offset=1888
        local.get 0
        local.get 2
        i64.load offset=1904
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 2
        i32.store offset=2100
        local.get 2
        i32.load offset=1872
        local.set 136
        local.get 136
        local.get 136
        i64.load offset=96
        local.get 2
        i64.load offset=1888
        local.get 0
        local.get 2
        i64.load offset=1904
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=96
        local.get 2
        i32.const 3
        i32.store offset=2104
        local.get 2
        i32.load offset=1872
        local.set 137
        local.get 137
        local.get 137
        i64.load offset=136
        local.get 2
        i64.load offset=1888
        local.get 0
        local.get 2
        i64.load offset=1904
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=136
        local.get 2
        i32.const 4
        i32.store offset=2108
        local.get 2
        i32.load offset=1872
        local.set 138
        local.get 138
        local.get 138
        i64.load offset=176
        local.get 2
        i64.load offset=1888
        local.get 0
        local.get 2
        i64.load offset=1904
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=176
        local.get 2
        i32.const 3
        i32.store offset=2112
        local.get 2
        i32.const 0
        i32.store offset=2116
        local.get 2
        i32.load offset=1872
        local.set 139
        local.get 139
        local.get 139
        i64.load offset=24
        local.get 2
        i64.load offset=1896
        local.get 0
        local.get 2
        i64.load offset=1912
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=24
        local.get 2
        i32.const 1
        i32.store offset=2120
        local.get 2
        i32.load offset=1872
        local.set 140
        local.get 140
        local.get 140
        i64.load offset=64
        local.get 2
        i64.load offset=1896
        local.get 0
        local.get 2
        i64.load offset=1912
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 2
        i32.store offset=2124
        local.get 2
        i32.load offset=1872
        local.set 141
        local.get 141
        local.get 141
        i64.load offset=104
        local.get 2
        i64.load offset=1896
        local.get 0
        local.get 2
        i64.load offset=1912
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=104
        local.get 2
        i32.const 3
        i32.store offset=2128
        local.get 2
        i32.load offset=1872
        local.set 142
        local.get 142
        local.get 142
        i64.load offset=144
        local.get 2
        i64.load offset=1896
        local.get 0
        local.get 2
        i64.load offset=1912
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=144
        local.get 2
        i32.const 4
        i32.store offset=2132
        local.get 2
        i32.load offset=1872
        local.set 143
        local.get 143
        local.get 143
        i64.load offset=184
        local.get 2
        i64.load offset=1896
        local.get 0
        local.get 2
        i64.load offset=1912
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=184
        local.get 2
        i32.const 4
        i32.store offset=2136
        local.get 2
        i32.const 0
        i32.store offset=2140
        local.get 2
        i32.load offset=1872
        local.set 144
        local.get 144
        local.get 144
        i64.load offset=32
        local.get 2
        i64.load offset=1904
        local.get 0
        local.get 2
        i64.load offset=1880
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=32
        local.get 2
        i32.const 1
        i32.store offset=2144
        local.get 2
        i32.load offset=1872
        local.set 145
        local.get 145
        local.get 145
        i64.load offset=72
        local.get 2
        i64.load offset=1904
        local.get 0
        local.get 2
        i64.load offset=1880
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 2
        i32.store offset=2148
        local.get 2
        i32.load offset=1872
        local.set 146
        local.get 146
        local.get 146
        i64.load offset=112
        local.get 2
        i64.load offset=1904
        local.get 0
        local.get 2
        i64.load offset=1880
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=112
        local.get 2
        i32.const 3
        i32.store offset=2152
        local.get 2
        i32.load offset=1872
        local.set 147
        local.get 147
        local.get 147
        i64.load offset=152
        local.get 2
        i64.load offset=1904
        local.get 0
        local.get 2
        i64.load offset=1880
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=152
        local.get 2
        i32.const 4
        i32.store offset=2156
        local.get 2
        i32.load offset=1872
        local.set 148
        local.get 148
        local.get 148
        i64.load offset=192
        local.get 2
        i64.load offset=1904
        local.get 0
        local.get 2
        i64.load offset=1880
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=192
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=8
        i64.store offset=2160
        local.get 2
        i32.const 0
        i32.store offset=2168
        local.get 2
        i32.const 10
        i32.store8 offset=2175
        local.get 2
        i32.load offset=1872
        i64.load offset=80
        local.set 149
        local.get 2
        local.get 149
        i64.store offset=2176
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8631
        i64.store offset=80
        local.get 2
        local.get 149
        i64.store offset=2160
        local.get 2
        i32.const 1
        i32.store offset=2184
        local.get 2
        i32.const 7
        i32.store8 offset=2191
        local.get 2
        i32.load offset=1872
        i64.load offset=56
        local.set 150
        local.get 2
        local.get 150
        i64.store offset=2192
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8636
        i64.store offset=56
        local.get 2
        local.get 150
        i64.store offset=2160
        local.get 2
        i32.const 2
        i32.store offset=2200
        local.get 2
        i32.const 11
        i32.store8 offset=2207
        local.get 2
        i32.load offset=1872
        i64.load offset=88
        local.set 151
        local.get 2
        local.get 151
        i64.store offset=2208
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8637
        i64.store offset=88
        local.get 2
        local.get 151
        i64.store offset=2160
        local.get 2
        i32.const 3
        i32.store offset=2216
        local.get 2
        i32.const 17
        i32.store8 offset=2223
        local.get 2
        i32.load offset=1872
        i64.load offset=136
        local.set 152
        local.get 2
        local.get 152
        i64.store offset=2224
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8638
        i64.store offset=136
        local.get 2
        local.get 152
        i64.store offset=2160
        local.get 2
        i32.const 4
        i32.store offset=2232
        local.get 2
        i32.const 18
        i32.store8 offset=2239
        local.get 2
        i32.load offset=1872
        i64.load offset=144
        local.set 153
        local.get 2
        local.get 153
        i64.store offset=2240
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8639
        i64.store offset=144
        local.get 2
        local.get 153
        i64.store offset=2160
        local.get 2
        i32.const 5
        i32.store offset=2248
        local.get 2
        i32.const 3
        i32.store8 offset=2255
        local.get 2
        i32.load offset=1872
        i64.load offset=24
        local.set 154
        local.get 2
        local.get 154
        i64.store offset=2256
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8640
        i64.store offset=24
        local.get 2
        local.get 154
        i64.store offset=2160
        local.get 2
        i32.const 6
        i32.store offset=2264
        local.get 2
        i32.const 5
        i32.store8 offset=2271
        local.get 2
        i32.load offset=1872
        i64.load offset=40
        local.set 155
        local.get 2
        local.get 155
        i64.store offset=2272
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8641
        i64.store offset=40
        local.get 2
        local.get 155
        i64.store offset=2160
        local.get 2
        i32.const 7
        i32.store offset=2280
        local.get 2
        i32.const 16
        i32.store8 offset=2287
        local.get 2
        i32.load offset=1872
        i64.load offset=128
        local.set 156
        local.get 2
        local.get 156
        i64.store offset=2288
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8642
        i64.store offset=128
        local.get 2
        local.get 156
        i64.store offset=2160
        i32.const 8
        local.set 157
        local.get 2
        local.get 157
        i32.store offset=2296
        local.get 2
        local.get 157
        i32.store8 offset=2303
        local.get 2
        i32.load offset=1872
        i64.load offset=64
        local.set 158
        local.get 2
        local.get 158
        i64.store offset=2304
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8643
        i64.store offset=64
        local.get 2
        local.get 158
        i64.store offset=2160
        local.get 2
        i32.const 9
        i32.store offset=2312
        local.get 2
        i32.const 21
        i32.store8 offset=2319
        local.get 2
        i32.load offset=1872
        i64.load offset=168
        local.set 159
        local.get 2
        local.get 159
        i64.store offset=2320
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8644
        i64.store offset=168
        local.get 2
        local.get 159
        i64.store offset=2160
        local.get 2
        i32.const 10
        i32.store offset=2328
        local.get 2
        i32.const 24
        i32.store8 offset=2335
        local.get 2
        i32.load offset=1872
        i64.load offset=192
        local.set 160
        local.get 2
        local.get 160
        i64.store offset=2336
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8645
        i64.store offset=192
        local.get 2
        local.get 160
        i64.store offset=2160
        local.get 2
        i32.const 11
        i32.store offset=2344
        local.get 2
        i32.const 4
        i32.store8 offset=2351
        local.get 2
        i32.load offset=1872
        i64.load offset=32
        local.set 161
        local.get 2
        local.get 161
        i64.store offset=2352
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8646
        i64.store offset=32
        local.get 2
        local.get 161
        i64.store offset=2160
        local.get 2
        i32.const 12
        i32.store offset=2360
        local.get 2
        i32.const 15
        i32.store8 offset=2367
        local.get 2
        i32.load offset=1872
        i64.load offset=120
        local.set 162
        local.get 2
        local.get 162
        i64.store offset=2368
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8647
        i64.store offset=120
        local.get 2
        local.get 162
        i64.store offset=2160
        local.get 2
        i32.const 13
        i32.store offset=2376
        local.get 2
        i32.const 23
        i32.store8 offset=2383
        local.get 2
        i32.load offset=1872
        i64.load offset=184
        local.set 163
        local.get 2
        local.get 163
        i64.store offset=2384
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8648
        i64.store offset=184
        local.get 2
        local.get 163
        i64.store offset=2160
        local.get 2
        i32.const 14
        i32.store offset=2392
        local.get 2
        i32.const 19
        i32.store8 offset=2399
        local.get 2
        i32.load offset=1872
        i64.load offset=152
        local.set 164
        local.get 2
        local.get 164
        i64.store offset=2400
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8649
        i64.store offset=152
        local.get 2
        local.get 164
        i64.store offset=2160
        local.get 2
        i32.const 15
        i32.store offset=2408
        local.get 2
        i32.const 13
        i32.store8 offset=2415
        local.get 2
        i32.load offset=1872
        i64.load offset=104
        local.set 165
        local.get 2
        local.get 165
        i64.store offset=2416
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8650
        i64.store offset=104
        local.get 2
        local.get 165
        i64.store offset=2160
        local.get 2
        i32.const 16
        i32.store offset=2424
        local.get 2
        i32.const 12
        i32.store8 offset=2431
        local.get 2
        i32.load offset=1872
        i64.load offset=96
        local.set 166
        local.get 2
        local.get 166
        i64.store offset=2432
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8651
        i64.store offset=96
        local.get 2
        local.get 166
        i64.store offset=2160
        local.get 2
        i32.const 17
        i32.store offset=2440
        local.get 2
        i32.const 2
        i32.store8 offset=2447
        local.get 2
        i32.load offset=1872
        i64.load offset=16
        local.set 167
        local.get 2
        local.get 167
        i64.store offset=2448
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8652
        i64.store offset=16
        local.get 2
        local.get 167
        i64.store offset=2160
        local.get 2
        i32.const 18
        i32.store offset=2456
        local.get 2
        i32.const 20
        i32.store8 offset=2463
        local.get 2
        i32.load offset=1872
        i64.load offset=160
        local.set 168
        local.get 2
        local.get 168
        i64.store offset=2464
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8653
        i64.store offset=160
        local.get 2
        local.get 168
        i64.store offset=2160
        local.get 2
        i32.const 19
        i32.store offset=2472
        local.get 2
        i32.const 14
        i32.store8 offset=2479
        local.get 2
        i32.load offset=1872
        i64.load offset=112
        local.set 169
        local.get 2
        local.get 169
        i64.store offset=2480
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8654
        i64.store offset=112
        local.get 2
        local.get 169
        i64.store offset=2160
        local.get 2
        i32.const 20
        i32.store offset=2488
        local.get 2
        i32.const 22
        i32.store8 offset=2495
        local.get 2
        i32.load offset=1872
        i64.load offset=176
        local.set 170
        local.get 2
        local.get 170
        i64.store offset=2496
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8655
        i64.store offset=176
        local.get 2
        local.get 170
        i64.store offset=2160
        local.get 2
        i32.const 21
        i32.store offset=2504
        local.get 2
        i32.const 9
        i32.store8 offset=2511
        local.get 2
        i32.load offset=1872
        i64.load offset=72
        local.set 171
        local.get 2
        local.get 171
        i64.store offset=2512
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8656
        i64.store offset=72
        local.get 2
        local.get 171
        i64.store offset=2160
        local.get 2
        i32.const 22
        i32.store offset=2520
        local.get 2
        i32.const 6
        i32.store8 offset=2527
        local.get 2
        i32.load offset=1872
        i64.load offset=48
        local.set 172
        local.get 2
        local.get 172
        i64.store offset=2528
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8657
        i64.store offset=48
        local.get 2
        local.get 172
        i64.store offset=2160
        local.get 2
        i32.const 23
        i32.store offset=2536
        local.get 2
        i32.const 1
        i32.store8 offset=2543
        local.get 2
        i32.load offset=1872
        i64.load offset=8
        local.set 173
        local.get 2
        local.get 173
        i64.store offset=2544
        local.get 2
        i32.load offset=1872
        local.get 0
        local.get 2
        i64.load offset=2160
        call $math.rotl__anon_8658
        i64.store offset=8
        local.get 2
        local.get 173
        i64.store offset=2160
        local.get 2
        i32.const 0
        i32.store offset=2552
        local.get 2
        i32.const 0
        i32.store offset=2556
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load
        i64.store offset=1880
        local.get 2
        i32.const 1
        i32.store offset=2560
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=8
        i64.store offset=1888
        local.get 2
        i32.const 2
        i32.store offset=2564
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=16
        i64.store offset=1896
        local.get 2
        i32.const 3
        i32.store offset=2568
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=24
        i64.store offset=1904
        local.get 2
        i32.const 4
        i32.store offset=2572
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=32
        i64.store offset=1912
        local.get 2
        i32.const 0
        i32.store offset=2576
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1880
        local.get 2
        i64.load offset=1888
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1896
        i64.and
        i64.xor
        i64.store
        local.get 2
        i32.const 1
        i32.store offset=2580
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1888
        local.get 2
        i64.load offset=1896
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1904
        i64.and
        i64.xor
        i64.store offset=8
        local.get 2
        i32.const 2
        i32.store offset=2584
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1896
        local.get 2
        i64.load offset=1904
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1912
        i64.and
        i64.xor
        i64.store offset=16
        local.get 2
        i32.const 3
        i32.store offset=2588
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1904
        local.get 2
        i64.load offset=1912
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1880
        i64.and
        i64.xor
        i64.store offset=24
        local.get 2
        i32.const 4
        i32.store offset=2592
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1912
        local.get 2
        i64.load offset=1880
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1888
        i64.and
        i64.xor
        i64.store offset=32
        local.get 2
        i32.const 1
        i32.store offset=2596
        local.get 2
        i32.const 0
        i32.store offset=2600
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=40
        i64.store offset=1880
        local.get 2
        i32.const 1
        i32.store offset=2604
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=48
        i64.store offset=1888
        local.get 2
        i32.const 2
        i32.store offset=2608
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=56
        i64.store offset=1896
        local.get 2
        i32.const 3
        i32.store offset=2612
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=64
        i64.store offset=1904
        local.get 2
        i32.const 4
        i32.store offset=2616
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=72
        i64.store offset=1912
        local.get 2
        i32.const 0
        i32.store offset=2620
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1880
        local.get 2
        i64.load offset=1888
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1896
        i64.and
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=2624
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1888
        local.get 2
        i64.load offset=1896
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1904
        i64.and
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=2628
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1896
        local.get 2
        i64.load offset=1904
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1912
        i64.and
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=2632
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1904
        local.get 2
        i64.load offset=1912
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1880
        i64.and
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=2636
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1912
        local.get 2
        i64.load offset=1880
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1888
        i64.and
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 2
        i32.store offset=2640
        local.get 2
        i32.const 0
        i32.store offset=2644
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=80
        i64.store offset=1880
        local.get 2
        i32.const 1
        i32.store offset=2648
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=88
        i64.store offset=1888
        local.get 2
        i32.const 2
        i32.store offset=2652
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=96
        i64.store offset=1896
        local.get 2
        i32.const 3
        i32.store offset=2656
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=104
        i64.store offset=1904
        local.get 2
        i32.const 4
        i32.store offset=2660
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=112
        i64.store offset=1912
        local.get 2
        i32.const 0
        i32.store offset=2664
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1880
        local.get 2
        i64.load offset=1888
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1896
        i64.and
        i64.xor
        i64.store offset=80
        local.get 2
        i32.const 1
        i32.store offset=2668
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1888
        local.get 2
        i64.load offset=1896
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1904
        i64.and
        i64.xor
        i64.store offset=88
        local.get 2
        i32.const 2
        i32.store offset=2672
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1896
        local.get 2
        i64.load offset=1904
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1912
        i64.and
        i64.xor
        i64.store offset=96
        local.get 2
        i32.const 3
        i32.store offset=2676
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1904
        local.get 2
        i64.load offset=1912
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1880
        i64.and
        i64.xor
        i64.store offset=104
        local.get 2
        i32.const 4
        i32.store offset=2680
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1912
        local.get 2
        i64.load offset=1880
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1888
        i64.and
        i64.xor
        i64.store offset=112
        local.get 2
        i32.const 3
        i32.store offset=2684
        local.get 2
        i32.const 0
        i32.store offset=2688
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=120
        i64.store offset=1880
        local.get 2
        i32.const 1
        i32.store offset=2692
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=128
        i64.store offset=1888
        local.get 2
        i32.const 2
        i32.store offset=2696
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=136
        i64.store offset=1896
        local.get 2
        i32.const 3
        i32.store offset=2700
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=144
        i64.store offset=1904
        local.get 2
        i32.const 4
        i32.store offset=2704
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=152
        i64.store offset=1912
        local.get 2
        i32.const 0
        i32.store offset=2708
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1880
        local.get 2
        i64.load offset=1888
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1896
        i64.and
        i64.xor
        i64.store offset=120
        local.get 2
        i32.const 1
        i32.store offset=2712
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1888
        local.get 2
        i64.load offset=1896
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1904
        i64.and
        i64.xor
        i64.store offset=128
        local.get 2
        i32.const 2
        i32.store offset=2716
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1896
        local.get 2
        i64.load offset=1904
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1912
        i64.and
        i64.xor
        i64.store offset=136
        local.get 2
        i32.const 3
        i32.store offset=2720
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1904
        local.get 2
        i64.load offset=1912
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1880
        i64.and
        i64.xor
        i64.store offset=144
        local.get 2
        i32.const 4
        i32.store offset=2724
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1912
        local.get 2
        i64.load offset=1880
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1888
        i64.and
        i64.xor
        i64.store offset=152
        local.get 2
        i32.const 4
        i32.store offset=2728
        local.get 2
        i32.const 0
        i32.store offset=2732
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=160
        i64.store offset=1880
        local.get 2
        i32.const 1
        i32.store offset=2736
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=168
        i64.store offset=1888
        local.get 2
        i32.const 2
        i32.store offset=2740
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=176
        i64.store offset=1896
        local.get 2
        i32.const 3
        i32.store offset=2744
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=184
        i64.store offset=1904
        local.get 2
        i32.const 4
        i32.store offset=2748
        local.get 2
        local.get 2
        i32.load offset=1872
        i64.load offset=192
        i64.store offset=1912
        local.get 2
        i32.const 0
        i32.store offset=2752
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1880
        local.get 2
        i64.load offset=1888
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1896
        i64.and
        i64.xor
        i64.store offset=160
        local.get 2
        i32.const 1
        i32.store offset=2756
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1888
        local.get 2
        i64.load offset=1896
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1904
        i64.and
        i64.xor
        i64.store offset=168
        local.get 2
        i32.const 2
        i32.store offset=2760
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1896
        local.get 2
        i64.load offset=1904
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1912
        i64.and
        i64.xor
        i64.store offset=176
        local.get 2
        i32.const 3
        i32.store offset=2764
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1904
        local.get 2
        i64.load offset=1912
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1880
        i64.and
        i64.xor
        i64.store offset=184
        local.get 2
        i32.const 4
        i32.store offset=2768
        local.get 2
        i32.load offset=1872
        local.get 2
        i64.load offset=1912
        local.get 2
        i64.load offset=1880
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=1888
        i64.and
        i64.xor
        i64.store offset=192
        local.get 2
        i32.load offset=1872
        local.set 174
        local.get 174
        local.get 174
        i64.load
        local.get 120
        i64.xor
        i64.store
        local.get 2
        i32.load offset=8
        local.set 175
        local.get 175
        i32.const 3
        i32.add
        local.set 176
        block  ;; label = @3
          local.get 176
          local.get 175
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 2
        local.get 176
        i32.store offset=8
        br 0 (;@2;)
      end
    end
    block  ;; label = @1
      loop  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 2
                i32.load offset=8
                i32.const 24
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 2
                i32.load offset=4
                local.set 177
                local.get 2
                i32.load offset=8
                local.set 178
                local.get 178
                i32.const 24
                i32.lt_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 0
          local.get 178
          i32.const 24
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        i32.const 1049408
        local.get 178
        i32.const 3
        i32.shl
        i32.add
        i64.load
        local.set 179
        local.get 2
        local.get 177
        i32.store offset=2772
        local.get 2
        local.get 179
        i64.store offset=2776
        local.get 2
        local.get 177
        i32.store offset=2788
        local.get 2
        i32.load offset=2788
        local.set 180
        local.get 2
        local.get 180
        i32.store offset=2792
        local.get 2
        local.get 180
        i32.store offset=2796
        local.get 2
        i32.const 2832
        i32.add
        local.set 181
        i64.const 0
        local.set 182
        local.get 181
        local.get 182
        i64.store
        local.get 2
        i32.const 2824
        i32.add
        local.get 182
        i64.store
        local.get 2
        i32.const 2816
        i32.add
        local.get 182
        i64.store
        local.get 2
        i32.const 2808
        i32.add
        local.get 182
        i64.store
        local.get 2
        local.get 182
        i64.store offset=2800
        local.get 2
        i32.const 0
        i32.store offset=2840
        local.get 2
        i32.const 0
        i32.store offset=2844
        local.get 2
        local.get 2
        i64.load offset=2800
        local.get 2
        i32.load offset=2792
        i64.load
        i64.xor
        i64.store offset=2800
        local.get 2
        i32.const 1
        i32.store offset=2848
        local.get 2
        local.get 2
        i64.load offset=2800
        local.get 2
        i32.load offset=2792
        i64.load offset=40
        i64.xor
        i64.store offset=2800
        local.get 2
        i32.const 2
        i32.store offset=2852
        local.get 2
        local.get 2
        i64.load offset=2800
        local.get 2
        i32.load offset=2792
        i64.load offset=80
        i64.xor
        i64.store offset=2800
        local.get 2
        i32.const 3
        i32.store offset=2856
        local.get 2
        local.get 2
        i64.load offset=2800
        local.get 2
        i32.load offset=2792
        i64.load offset=120
        i64.xor
        i64.store offset=2800
        local.get 2
        i32.const 4
        i32.store offset=2860
        local.get 2
        local.get 2
        i64.load offset=2800
        local.get 2
        i32.load offset=2792
        i64.load offset=160
        i64.xor
        i64.store offset=2800
        local.get 2
        i32.const 1
        i32.store offset=2864
        local.get 2
        i32.const 0
        i32.store offset=2868
        local.get 2
        local.get 2
        i64.load offset=2808
        local.get 2
        i32.load offset=2792
        i64.load offset=8
        i64.xor
        i64.store offset=2808
        local.get 2
        i32.const 1
        i32.store offset=2872
        local.get 2
        local.get 2
        i64.load offset=2808
        local.get 2
        i32.load offset=2792
        i64.load offset=48
        i64.xor
        i64.store offset=2808
        local.get 2
        i32.const 2
        i32.store offset=2876
        local.get 2
        local.get 2
        i64.load offset=2808
        local.get 2
        i32.load offset=2792
        i64.load offset=88
        i64.xor
        i64.store offset=2808
        local.get 2
        i32.const 3
        i32.store offset=2880
        local.get 2
        local.get 2
        i64.load offset=2808
        local.get 2
        i32.load offset=2792
        i64.load offset=128
        i64.xor
        i64.store offset=2808
        local.get 2
        i32.const 4
        i32.store offset=2884
        local.get 2
        local.get 2
        i64.load offset=2808
        local.get 2
        i32.load offset=2792
        i64.load offset=168
        i64.xor
        i64.store offset=2808
        local.get 2
        i32.const 2
        i32.store offset=2888
        local.get 2
        i32.const 0
        i32.store offset=2892
        local.get 2
        local.get 2
        i64.load offset=2816
        local.get 2
        i32.load offset=2792
        i64.load offset=16
        i64.xor
        i64.store offset=2816
        local.get 2
        i32.const 1
        i32.store offset=2896
        local.get 2
        local.get 2
        i64.load offset=2816
        local.get 2
        i32.load offset=2792
        i64.load offset=56
        i64.xor
        i64.store offset=2816
        local.get 2
        i32.const 2
        i32.store offset=2900
        local.get 2
        local.get 2
        i64.load offset=2816
        local.get 2
        i32.load offset=2792
        i64.load offset=96
        i64.xor
        i64.store offset=2816
        local.get 2
        i32.const 3
        i32.store offset=2904
        local.get 2
        local.get 2
        i64.load offset=2816
        local.get 2
        i32.load offset=2792
        i64.load offset=136
        i64.xor
        i64.store offset=2816
        local.get 2
        i32.const 4
        i32.store offset=2908
        local.get 2
        local.get 2
        i64.load offset=2816
        local.get 2
        i32.load offset=2792
        i64.load offset=176
        i64.xor
        i64.store offset=2816
        local.get 2
        i32.const 3
        i32.store offset=2912
        local.get 2
        i32.const 0
        i32.store offset=2916
        local.get 2
        local.get 2
        i64.load offset=2824
        local.get 2
        i32.load offset=2792
        i64.load offset=24
        i64.xor
        i64.store offset=2824
        local.get 2
        i32.const 1
        i32.store offset=2920
        local.get 2
        local.get 2
        i64.load offset=2824
        local.get 2
        i32.load offset=2792
        i64.load offset=64
        i64.xor
        i64.store offset=2824
        local.get 2
        i32.const 2
        i32.store offset=2924
        local.get 2
        local.get 2
        i64.load offset=2824
        local.get 2
        i32.load offset=2792
        i64.load offset=104
        i64.xor
        i64.store offset=2824
        local.get 2
        i32.const 3
        i32.store offset=2928
        local.get 2
        local.get 2
        i64.load offset=2824
        local.get 2
        i32.load offset=2792
        i64.load offset=144
        i64.xor
        i64.store offset=2824
        local.get 2
        i32.const 4
        i32.store offset=2932
        local.get 2
        local.get 2
        i64.load offset=2824
        local.get 2
        i32.load offset=2792
        i64.load offset=184
        i64.xor
        i64.store offset=2824
        local.get 2
        i32.const 4
        i32.store offset=2936
        local.get 2
        i32.const 0
        i32.store offset=2940
        local.get 2
        local.get 2
        i64.load offset=2832
        local.get 2
        i32.load offset=2792
        i64.load offset=32
        i64.xor
        i64.store offset=2832
        local.get 2
        i32.const 1
        i32.store offset=2944
        local.get 2
        local.get 2
        i64.load offset=2832
        local.get 2
        i32.load offset=2792
        i64.load offset=72
        i64.xor
        i64.store offset=2832
        local.get 2
        i32.const 2
        i32.store offset=2948
        local.get 2
        local.get 2
        i64.load offset=2832
        local.get 2
        i32.load offset=2792
        i64.load offset=112
        i64.xor
        i64.store offset=2832
        local.get 2
        i32.const 3
        i32.store offset=2952
        local.get 2
        local.get 2
        i64.load offset=2832
        local.get 2
        i32.load offset=2792
        i64.load offset=152
        i64.xor
        i64.store offset=2832
        local.get 2
        i32.const 4
        i32.store offset=2956
        local.get 2
        local.get 2
        i64.load offset=2832
        local.get 2
        i32.load offset=2792
        i64.load offset=192
        i64.xor
        i64.store offset=2832
        local.get 2
        i32.const 0
        i32.store offset=2960
        local.get 2
        i32.const 0
        i32.store offset=2964
        local.get 2
        i32.load offset=2792
        local.set 183
        local.get 183
        local.get 183
        i64.load
        local.get 2
        i64.load offset=2832
        local.get 0
        local.get 2
        i64.load offset=2808
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store
        local.get 2
        i32.const 1
        i32.store offset=2968
        local.get 2
        i32.load offset=2792
        local.set 184
        local.get 184
        local.get 184
        i64.load offset=40
        local.get 2
        i64.load offset=2832
        local.get 0
        local.get 2
        i64.load offset=2808
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 2
        i32.store offset=2972
        local.get 2
        i32.load offset=2792
        local.set 185
        local.get 185
        local.get 185
        i64.load offset=80
        local.get 2
        i64.load offset=2832
        local.get 0
        local.get 2
        i64.load offset=2808
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=80
        local.get 2
        i32.const 3
        i32.store offset=2976
        local.get 2
        i32.load offset=2792
        local.set 186
        local.get 186
        local.get 186
        i64.load offset=120
        local.get 2
        i64.load offset=2832
        local.get 0
        local.get 2
        i64.load offset=2808
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=120
        local.get 2
        i32.const 4
        i32.store offset=2980
        local.get 2
        i32.load offset=2792
        local.set 187
        local.get 187
        local.get 187
        i64.load offset=160
        local.get 2
        i64.load offset=2832
        local.get 0
        local.get 2
        i64.load offset=2808
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=160
        local.get 2
        i32.const 1
        i32.store offset=2984
        local.get 2
        i32.const 0
        i32.store offset=2988
        local.get 2
        i32.load offset=2792
        local.set 188
        local.get 188
        local.get 188
        i64.load offset=8
        local.get 2
        i64.load offset=2800
        local.get 0
        local.get 2
        i64.load offset=2816
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=8
        local.get 2
        i32.const 1
        i32.store offset=2992
        local.get 2
        i32.load offset=2792
        local.set 189
        local.get 189
        local.get 189
        i64.load offset=48
        local.get 2
        i64.load offset=2800
        local.get 0
        local.get 2
        i64.load offset=2816
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=2996
        local.get 2
        i32.load offset=2792
        local.set 190
        local.get 190
        local.get 190
        i64.load offset=88
        local.get 2
        i64.load offset=2800
        local.get 0
        local.get 2
        i64.load offset=2816
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=88
        local.get 2
        i32.const 3
        i32.store offset=3000
        local.get 2
        i32.load offset=2792
        local.set 191
        local.get 191
        local.get 191
        i64.load offset=128
        local.get 2
        i64.load offset=2800
        local.get 0
        local.get 2
        i64.load offset=2816
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=128
        local.get 2
        i32.const 4
        i32.store offset=3004
        local.get 2
        i32.load offset=2792
        local.set 192
        local.get 192
        local.get 192
        i64.load offset=168
        local.get 2
        i64.load offset=2800
        local.get 0
        local.get 2
        i64.load offset=2816
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=168
        local.get 2
        i32.const 2
        i32.store offset=3008
        local.get 2
        i32.const 0
        i32.store offset=3012
        local.get 2
        i32.load offset=2792
        local.set 193
        local.get 193
        local.get 193
        i64.load offset=16
        local.get 2
        i64.load offset=2808
        local.get 0
        local.get 2
        i64.load offset=2824
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=16
        local.get 2
        i32.const 1
        i32.store offset=3016
        local.get 2
        i32.load offset=2792
        local.set 194
        local.get 194
        local.get 194
        i64.load offset=56
        local.get 2
        i64.load offset=2808
        local.get 0
        local.get 2
        i64.load offset=2824
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 2
        i32.store offset=3020
        local.get 2
        i32.load offset=2792
        local.set 195
        local.get 195
        local.get 195
        i64.load offset=96
        local.get 2
        i64.load offset=2808
        local.get 0
        local.get 2
        i64.load offset=2824
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=96
        local.get 2
        i32.const 3
        i32.store offset=3024
        local.get 2
        i32.load offset=2792
        local.set 196
        local.get 196
        local.get 196
        i64.load offset=136
        local.get 2
        i64.load offset=2808
        local.get 0
        local.get 2
        i64.load offset=2824
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=136
        local.get 2
        i32.const 4
        i32.store offset=3028
        local.get 2
        i32.load offset=2792
        local.set 197
        local.get 197
        local.get 197
        i64.load offset=176
        local.get 2
        i64.load offset=2808
        local.get 0
        local.get 2
        i64.load offset=2824
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=176
        local.get 2
        i32.const 3
        i32.store offset=3032
        local.get 2
        i32.const 0
        i32.store offset=3036
        local.get 2
        i32.load offset=2792
        local.set 198
        local.get 198
        local.get 198
        i64.load offset=24
        local.get 2
        i64.load offset=2816
        local.get 0
        local.get 2
        i64.load offset=2832
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=24
        local.get 2
        i32.const 1
        i32.store offset=3040
        local.get 2
        i32.load offset=2792
        local.set 199
        local.get 199
        local.get 199
        i64.load offset=64
        local.get 2
        i64.load offset=2816
        local.get 0
        local.get 2
        i64.load offset=2832
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 2
        i32.store offset=3044
        local.get 2
        i32.load offset=2792
        local.set 200
        local.get 200
        local.get 200
        i64.load offset=104
        local.get 2
        i64.load offset=2816
        local.get 0
        local.get 2
        i64.load offset=2832
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=104
        local.get 2
        i32.const 3
        i32.store offset=3048
        local.get 2
        i32.load offset=2792
        local.set 201
        local.get 201
        local.get 201
        i64.load offset=144
        local.get 2
        i64.load offset=2816
        local.get 0
        local.get 2
        i64.load offset=2832
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=144
        local.get 2
        i32.const 4
        i32.store offset=3052
        local.get 2
        i32.load offset=2792
        local.set 202
        local.get 202
        local.get 202
        i64.load offset=184
        local.get 2
        i64.load offset=2816
        local.get 0
        local.get 2
        i64.load offset=2832
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=184
        local.get 2
        i32.const 4
        i32.store offset=3056
        local.get 2
        i32.const 0
        i32.store offset=3060
        local.get 2
        i32.load offset=2792
        local.set 203
        local.get 203
        local.get 203
        i64.load offset=32
        local.get 2
        i64.load offset=2824
        local.get 0
        local.get 2
        i64.load offset=2800
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=32
        local.get 2
        i32.const 1
        i32.store offset=3064
        local.get 2
        i32.load offset=2792
        local.set 204
        local.get 204
        local.get 204
        i64.load offset=72
        local.get 2
        i64.load offset=2824
        local.get 0
        local.get 2
        i64.load offset=2800
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 2
        i32.store offset=3068
        local.get 2
        i32.load offset=2792
        local.set 205
        local.get 205
        local.get 205
        i64.load offset=112
        local.get 2
        i64.load offset=2824
        local.get 0
        local.get 2
        i64.load offset=2800
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=112
        local.get 2
        i32.const 3
        i32.store offset=3072
        local.get 2
        i32.load offset=2792
        local.set 206
        local.get 206
        local.get 206
        i64.load offset=152
        local.get 2
        i64.load offset=2824
        local.get 0
        local.get 2
        i64.load offset=2800
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=152
        local.get 2
        i32.const 4
        i32.store offset=3076
        local.get 2
        i32.load offset=2792
        local.set 207
        local.get 207
        local.get 207
        i64.load offset=192
        local.get 2
        i64.load offset=2824
        local.get 0
        local.get 2
        i64.load offset=2800
        call $math.rotl__anon_8631
        i64.xor
        i64.xor
        i64.store offset=192
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=8
        i64.store offset=3080
        local.get 2
        i32.const 0
        i32.store offset=3088
        local.get 2
        i32.const 10
        i32.store8 offset=3095
        local.get 2
        i32.load offset=2792
        i64.load offset=80
        local.set 208
        local.get 2
        local.get 208
        i64.store offset=3096
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8631
        i64.store offset=80
        local.get 2
        local.get 208
        i64.store offset=3080
        local.get 2
        i32.const 1
        i32.store offset=3104
        local.get 2
        i32.const 7
        i32.store8 offset=3111
        local.get 2
        i32.load offset=2792
        i64.load offset=56
        local.set 209
        local.get 2
        local.get 209
        i64.store offset=3112
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8636
        i64.store offset=56
        local.get 2
        local.get 209
        i64.store offset=3080
        local.get 2
        i32.const 2
        i32.store offset=3120
        local.get 2
        i32.const 11
        i32.store8 offset=3127
        local.get 2
        i32.load offset=2792
        i64.load offset=88
        local.set 210
        local.get 2
        local.get 210
        i64.store offset=3128
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8637
        i64.store offset=88
        local.get 2
        local.get 210
        i64.store offset=3080
        local.get 2
        i32.const 3
        i32.store offset=3136
        local.get 2
        i32.const 17
        i32.store8 offset=3143
        local.get 2
        i32.load offset=2792
        i64.load offset=136
        local.set 211
        local.get 2
        local.get 211
        i64.store offset=3144
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8638
        i64.store offset=136
        local.get 2
        local.get 211
        i64.store offset=3080
        local.get 2
        i32.const 4
        i32.store offset=3152
        local.get 2
        i32.const 18
        i32.store8 offset=3159
        local.get 2
        i32.load offset=2792
        i64.load offset=144
        local.set 212
        local.get 2
        local.get 212
        i64.store offset=3160
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8639
        i64.store offset=144
        local.get 2
        local.get 212
        i64.store offset=3080
        local.get 2
        i32.const 5
        i32.store offset=3168
        local.get 2
        i32.const 3
        i32.store8 offset=3175
        local.get 2
        i32.load offset=2792
        i64.load offset=24
        local.set 213
        local.get 2
        local.get 213
        i64.store offset=3176
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8640
        i64.store offset=24
        local.get 2
        local.get 213
        i64.store offset=3080
        local.get 2
        i32.const 6
        i32.store offset=3184
        local.get 2
        i32.const 5
        i32.store8 offset=3191
        local.get 2
        i32.load offset=2792
        i64.load offset=40
        local.set 214
        local.get 2
        local.get 214
        i64.store offset=3192
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8641
        i64.store offset=40
        local.get 2
        local.get 214
        i64.store offset=3080
        local.get 2
        i32.const 7
        i32.store offset=3200
        local.get 2
        i32.const 16
        i32.store8 offset=3207
        local.get 2
        i32.load offset=2792
        i64.load offset=128
        local.set 215
        local.get 2
        local.get 215
        i64.store offset=3208
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8642
        i64.store offset=128
        local.get 2
        local.get 215
        i64.store offset=3080
        i32.const 8
        local.set 216
        local.get 2
        local.get 216
        i32.store offset=3216
        local.get 2
        local.get 216
        i32.store8 offset=3223
        local.get 2
        i32.load offset=2792
        i64.load offset=64
        local.set 217
        local.get 2
        local.get 217
        i64.store offset=3224
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8643
        i64.store offset=64
        local.get 2
        local.get 217
        i64.store offset=3080
        local.get 2
        i32.const 9
        i32.store offset=3232
        local.get 2
        i32.const 21
        i32.store8 offset=3239
        local.get 2
        i32.load offset=2792
        i64.load offset=168
        local.set 218
        local.get 2
        local.get 218
        i64.store offset=3240
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8644
        i64.store offset=168
        local.get 2
        local.get 218
        i64.store offset=3080
        local.get 2
        i32.const 10
        i32.store offset=3248
        local.get 2
        i32.const 24
        i32.store8 offset=3255
        local.get 2
        i32.load offset=2792
        i64.load offset=192
        local.set 219
        local.get 2
        local.get 219
        i64.store offset=3256
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8645
        i64.store offset=192
        local.get 2
        local.get 219
        i64.store offset=3080
        local.get 2
        i32.const 11
        i32.store offset=3264
        local.get 2
        i32.const 4
        i32.store8 offset=3271
        local.get 2
        i32.load offset=2792
        i64.load offset=32
        local.set 220
        local.get 2
        local.get 220
        i64.store offset=3272
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8646
        i64.store offset=32
        local.get 2
        local.get 220
        i64.store offset=3080
        local.get 2
        i32.const 12
        i32.store offset=3280
        local.get 2
        i32.const 15
        i32.store8 offset=3287
        local.get 2
        i32.load offset=2792
        i64.load offset=120
        local.set 221
        local.get 2
        local.get 221
        i64.store offset=3288
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8647
        i64.store offset=120
        local.get 2
        local.get 221
        i64.store offset=3080
        local.get 2
        i32.const 13
        i32.store offset=3296
        local.get 2
        i32.const 23
        i32.store8 offset=3303
        local.get 2
        i32.load offset=2792
        i64.load offset=184
        local.set 222
        local.get 2
        local.get 222
        i64.store offset=3304
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8648
        i64.store offset=184
        local.get 2
        local.get 222
        i64.store offset=3080
        local.get 2
        i32.const 14
        i32.store offset=3312
        local.get 2
        i32.const 19
        i32.store8 offset=3319
        local.get 2
        i32.load offset=2792
        i64.load offset=152
        local.set 223
        local.get 2
        local.get 223
        i64.store offset=3320
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8649
        i64.store offset=152
        local.get 2
        local.get 223
        i64.store offset=3080
        local.get 2
        i32.const 15
        i32.store offset=3328
        local.get 2
        i32.const 13
        i32.store8 offset=3335
        local.get 2
        i32.load offset=2792
        i64.load offset=104
        local.set 224
        local.get 2
        local.get 224
        i64.store offset=3336
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8650
        i64.store offset=104
        local.get 2
        local.get 224
        i64.store offset=3080
        local.get 2
        i32.const 16
        i32.store offset=3344
        local.get 2
        i32.const 12
        i32.store8 offset=3351
        local.get 2
        i32.load offset=2792
        i64.load offset=96
        local.set 225
        local.get 2
        local.get 225
        i64.store offset=3352
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8651
        i64.store offset=96
        local.get 2
        local.get 225
        i64.store offset=3080
        local.get 2
        i32.const 17
        i32.store offset=3360
        local.get 2
        i32.const 2
        i32.store8 offset=3367
        local.get 2
        i32.load offset=2792
        i64.load offset=16
        local.set 226
        local.get 2
        local.get 226
        i64.store offset=3368
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8652
        i64.store offset=16
        local.get 2
        local.get 226
        i64.store offset=3080
        local.get 2
        i32.const 18
        i32.store offset=3376
        local.get 2
        i32.const 20
        i32.store8 offset=3383
        local.get 2
        i32.load offset=2792
        i64.load offset=160
        local.set 227
        local.get 2
        local.get 227
        i64.store offset=3384
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8653
        i64.store offset=160
        local.get 2
        local.get 227
        i64.store offset=3080
        local.get 2
        i32.const 19
        i32.store offset=3392
        local.get 2
        i32.const 14
        i32.store8 offset=3399
        local.get 2
        i32.load offset=2792
        i64.load offset=112
        local.set 228
        local.get 2
        local.get 228
        i64.store offset=3400
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8654
        i64.store offset=112
        local.get 2
        local.get 228
        i64.store offset=3080
        local.get 2
        i32.const 20
        i32.store offset=3408
        local.get 2
        i32.const 22
        i32.store8 offset=3415
        local.get 2
        i32.load offset=2792
        i64.load offset=176
        local.set 229
        local.get 2
        local.get 229
        i64.store offset=3416
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8655
        i64.store offset=176
        local.get 2
        local.get 229
        i64.store offset=3080
        local.get 2
        i32.const 21
        i32.store offset=3424
        local.get 2
        i32.const 9
        i32.store8 offset=3431
        local.get 2
        i32.load offset=2792
        i64.load offset=72
        local.set 230
        local.get 2
        local.get 230
        i64.store offset=3432
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8656
        i64.store offset=72
        local.get 2
        local.get 230
        i64.store offset=3080
        local.get 2
        i32.const 22
        i32.store offset=3440
        local.get 2
        i32.const 6
        i32.store8 offset=3447
        local.get 2
        i32.load offset=2792
        i64.load offset=48
        local.set 231
        local.get 2
        local.get 231
        i64.store offset=3448
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8657
        i64.store offset=48
        local.get 2
        local.get 231
        i64.store offset=3080
        local.get 2
        i32.const 23
        i32.store offset=3456
        local.get 2
        i32.const 1
        i32.store8 offset=3463
        local.get 2
        i32.load offset=2792
        i64.load offset=8
        local.set 232
        local.get 2
        local.get 232
        i64.store offset=3464
        local.get 2
        i32.load offset=2792
        local.get 0
        local.get 2
        i64.load offset=3080
        call $math.rotl__anon_8658
        i64.store offset=8
        local.get 2
        local.get 232
        i64.store offset=3080
        local.get 2
        i32.const 0
        i32.store offset=3476
        local.get 2
        i32.const 0
        i32.store offset=3480
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load
        i64.store offset=2800
        local.get 2
        i32.const 1
        i32.store offset=3484
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=8
        i64.store offset=2808
        local.get 2
        i32.const 2
        i32.store offset=3488
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=16
        i64.store offset=2816
        local.get 2
        i32.const 3
        i32.store offset=3492
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=24
        i64.store offset=2824
        local.get 2
        i32.const 4
        i32.store offset=3496
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=32
        i64.store offset=2832
        local.get 2
        i32.const 0
        i32.store offset=3500
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2800
        local.get 2
        i64.load offset=2808
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2816
        i64.and
        i64.xor
        i64.store
        local.get 2
        i32.const 1
        i32.store offset=3504
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2808
        local.get 2
        i64.load offset=2816
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2824
        i64.and
        i64.xor
        i64.store offset=8
        local.get 2
        i32.const 2
        i32.store offset=3508
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2816
        local.get 2
        i64.load offset=2824
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2832
        i64.and
        i64.xor
        i64.store offset=16
        local.get 2
        i32.const 3
        i32.store offset=3512
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2824
        local.get 2
        i64.load offset=2832
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2800
        i64.and
        i64.xor
        i64.store offset=24
        local.get 2
        i32.const 4
        i32.store offset=3516
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2832
        local.get 2
        i64.load offset=2800
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2808
        i64.and
        i64.xor
        i64.store offset=32
        local.get 2
        i32.const 1
        i32.store offset=3520
        local.get 2
        i32.const 0
        i32.store offset=3524
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=40
        i64.store offset=2800
        local.get 2
        i32.const 1
        i32.store offset=3528
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=48
        i64.store offset=2808
        local.get 2
        i32.const 2
        i32.store offset=3532
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=56
        i64.store offset=2816
        local.get 2
        i32.const 3
        i32.store offset=3536
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=64
        i64.store offset=2824
        local.get 2
        i32.const 4
        i32.store offset=3540
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=72
        i64.store offset=2832
        local.get 2
        i32.const 0
        i32.store offset=3544
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2800
        local.get 2
        i64.load offset=2808
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2816
        i64.and
        i64.xor
        i64.store offset=40
        local.get 2
        i32.const 1
        i32.store offset=3548
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2808
        local.get 2
        i64.load offset=2816
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2824
        i64.and
        i64.xor
        i64.store offset=48
        local.get 2
        i32.const 2
        i32.store offset=3552
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2816
        local.get 2
        i64.load offset=2824
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2832
        i64.and
        i64.xor
        i64.store offset=56
        local.get 2
        i32.const 3
        i32.store offset=3556
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2824
        local.get 2
        i64.load offset=2832
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2800
        i64.and
        i64.xor
        i64.store offset=64
        local.get 2
        i32.const 4
        i32.store offset=3560
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2832
        local.get 2
        i64.load offset=2800
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2808
        i64.and
        i64.xor
        i64.store offset=72
        local.get 2
        i32.const 2
        i32.store offset=3564
        local.get 2
        i32.const 0
        i32.store offset=3568
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=80
        i64.store offset=2800
        local.get 2
        i32.const 1
        i32.store offset=3572
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=88
        i64.store offset=2808
        local.get 2
        i32.const 2
        i32.store offset=3576
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=96
        i64.store offset=2816
        local.get 2
        i32.const 3
        i32.store offset=3580
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=104
        i64.store offset=2824
        local.get 2
        i32.const 4
        i32.store offset=3584
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=112
        i64.store offset=2832
        local.get 2
        i32.const 0
        i32.store offset=3588
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2800
        local.get 2
        i64.load offset=2808
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2816
        i64.and
        i64.xor
        i64.store offset=80
        local.get 2
        i32.const 1
        i32.store offset=3592
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2808
        local.get 2
        i64.load offset=2816
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2824
        i64.and
        i64.xor
        i64.store offset=88
        local.get 2
        i32.const 2
        i32.store offset=3596
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2816
        local.get 2
        i64.load offset=2824
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2832
        i64.and
        i64.xor
        i64.store offset=96
        local.get 2
        i32.const 3
        i32.store offset=3600
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2824
        local.get 2
        i64.load offset=2832
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2800
        i64.and
        i64.xor
        i64.store offset=104
        local.get 2
        i32.const 4
        i32.store offset=3604
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2832
        local.get 2
        i64.load offset=2800
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2808
        i64.and
        i64.xor
        i64.store offset=112
        local.get 2
        i32.const 3
        i32.store offset=3608
        local.get 2
        i32.const 0
        i32.store offset=3612
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=120
        i64.store offset=2800
        local.get 2
        i32.const 1
        i32.store offset=3616
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=128
        i64.store offset=2808
        local.get 2
        i32.const 2
        i32.store offset=3620
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=136
        i64.store offset=2816
        local.get 2
        i32.const 3
        i32.store offset=3624
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=144
        i64.store offset=2824
        local.get 2
        i32.const 4
        i32.store offset=3628
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=152
        i64.store offset=2832
        local.get 2
        i32.const 0
        i32.store offset=3632
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2800
        local.get 2
        i64.load offset=2808
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2816
        i64.and
        i64.xor
        i64.store offset=120
        local.get 2
        i32.const 1
        i32.store offset=3636
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2808
        local.get 2
        i64.load offset=2816
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2824
        i64.and
        i64.xor
        i64.store offset=128
        local.get 2
        i32.const 2
        i32.store offset=3640
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2816
        local.get 2
        i64.load offset=2824
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2832
        i64.and
        i64.xor
        i64.store offset=136
        local.get 2
        i32.const 3
        i32.store offset=3644
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2824
        local.get 2
        i64.load offset=2832
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2800
        i64.and
        i64.xor
        i64.store offset=144
        local.get 2
        i32.const 4
        i32.store offset=3648
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2832
        local.get 2
        i64.load offset=2800
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2808
        i64.and
        i64.xor
        i64.store offset=152
        local.get 2
        i32.const 4
        i32.store offset=3652
        local.get 2
        i32.const 0
        i32.store offset=3656
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=160
        i64.store offset=2800
        local.get 2
        i32.const 1
        i32.store offset=3660
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=168
        i64.store offset=2808
        local.get 2
        i32.const 2
        i32.store offset=3664
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=176
        i64.store offset=2816
        local.get 2
        i32.const 3
        i32.store offset=3668
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=184
        i64.store offset=2824
        local.get 2
        i32.const 4
        i32.store offset=3672
        local.get 2
        local.get 2
        i32.load offset=2792
        i64.load offset=192
        i64.store offset=2832
        local.get 2
        i32.const 0
        i32.store offset=3676
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2800
        local.get 2
        i64.load offset=2808
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2816
        i64.and
        i64.xor
        i64.store offset=160
        local.get 2
        i32.const 1
        i32.store offset=3680
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2808
        local.get 2
        i64.load offset=2816
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2824
        i64.and
        i64.xor
        i64.store offset=168
        local.get 2
        i32.const 2
        i32.store offset=3684
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2816
        local.get 2
        i64.load offset=2824
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2832
        i64.and
        i64.xor
        i64.store offset=176
        local.get 2
        i32.const 3
        i32.store offset=3688
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2824
        local.get 2
        i64.load offset=2832
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2800
        i64.and
        i64.xor
        i64.store offset=184
        local.get 2
        i32.const 4
        i32.store offset=3692
        local.get 2
        i32.load offset=2792
        local.get 2
        i64.load offset=2832
        local.get 2
        i64.load offset=2800
        i64.const -1
        i64.xor
        local.get 2
        i64.load offset=2808
        i64.and
        i64.xor
        i64.store offset=192
        local.get 2
        i32.load offset=2792
        local.set 233
        local.get 233
        local.get 233
        i64.load
        local.get 179
        i64.xor
        i64.store
        local.get 2
        i32.load offset=8
        i32.const 1
        i32.add
        local.set 234
        block  ;; label = @3
          local.get 234
          i32.eqz
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 2
        local.get 234
        i32.store offset=8
        br 0 (;@2;)
      end
    end
    local.get 2
    i32.const 3696
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.keccak_p.KeccakF_1600_.extractBytes (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32 i32 i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 96
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 1
    i32.store
    local.get 4
    local.get 3
    i32.store offset=8
    local.get 4
    local.get 2
    i32.store offset=4
    local.get 4
    local.get 1
    i32.store offset=12
    local.get 4
    local.get 3
    i32.store offset=20
    local.get 4
    local.get 2
    i32.store offset=16
    local.get 4
    i32.const 0
    i32.store offset=24
    loop  ;; label = @1
      local.get 4
      i32.load offset=24
      local.set 5
      local.get 5
      i32.const 8
      i32.add
      local.set 6
      block  ;; label = @2
        local.get 6
        local.get 5
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 6
                local.get 4
                i32.load offset=20
                i32.le_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 4
                i32.load offset=24
                local.set 7
                local.get 4
                i32.load offset=20
                local.set 8
                local.get 7
                local.get 4
                i32.load offset=16
                i32.add
                local.set 9
                local.get 7
                i32.const 8
                i32.add
                local.set 10
                local.get 10
                local.get 8
                i32.le_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 3 (;@2;)
            end
            br 1 (;@3;)
          end
          local.get 0
          local.get 10
          local.get 8
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.load offset=12
        local.set 11
        local.get 4
        i32.load offset=24
        i32.const 3
        i32.shr_u
        local.set 12
        block  ;; label = @3
          block  ;; label = @4
            local.get 12
            i32.const 25
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 12
          i32.const 25
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 11
        local.get 12
        i32.const 3
        i32.shl
        i32.add
        i64.load
        local.set 13
        local.get 4
        local.get 9
        i32.store offset=28
        local.get 4
        local.get 13
        i64.store offset=32
        local.get 4
        i32.const 1
        i32.const 1
        i32.and
        i32.store8 offset=47
        local.get 4
        local.get 13
        i64.store offset=48
        local.get 9
        local.get 4
        i64.load offset=48
        i64.store align=1
        local.get 4
        i32.load offset=24
        local.set 14
        local.get 14
        i32.const 8
        i32.add
        local.set 15
        block  ;; label = @3
          local.get 15
          local.get 14
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 15
        i32.store offset=24
        br 1 (;@1;)
      end
    end
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 4
              i32.load offset=24
              local.get 4
              i32.load offset=20
              i32.lt_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              local.get 4
              i64.const 0
              i64.store offset=56
              local.get 4
              i32.const 56
              i32.add
              local.set 16
              local.get 4
              i32.load offset=12
              local.set 17
              local.get 4
              i32.load offset=24
              i32.const 3
              i32.shr_u
              local.set 18
              local.get 18
              i32.const 25
              i32.lt_u
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            br 3 (;@1;)
          end
          br 1 (;@2;)
        end
        local.get 0
        local.get 18
        i32.const 25
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 17
      local.get 18
      i32.const 3
      i32.shl
      i32.add
      i64.load
      local.set 19
      local.get 4
      local.get 16
      i32.store offset=68
      local.get 4
      local.get 19
      i64.store offset=72
      local.get 4
      i32.const 1
      i32.const 1
      i32.and
      i32.store8 offset=87
      local.get 4
      local.get 19
      i64.store offset=88
      local.get 16
      local.get 4
      i64.load offset=88
      i64.store align=1
      local.get 4
      i32.load offset=24
      local.set 20
      local.get 4
      i32.load offset=20
      local.set 21
      local.get 20
      local.get 4
      i32.load offset=16
      i32.add
      local.set 22
      block  ;; label = @2
        block  ;; label = @3
          local.get 20
          local.get 21
          i32.le_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        local.get 20
        local.get 21
        call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
        unreachable
      end
      local.get 21
      local.get 20
      i32.sub
      local.set 23
      block  ;; label = @2
        block  ;; label = @3
          local.get 21
          local.get 21
          i32.le_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        local.get 21
        local.get 21
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 23
      local.set 24
      local.get 22
      local.set 25
      local.get 4
      i32.load offset=20
      local.set 26
      local.get 26
      local.get 4
      i32.load offset=24
      i32.sub
      local.set 27
      block  ;; label = @2
        local.get 27
        local.get 26
        i32.gt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      local.get 4
      i32.const 56
      i32.add
      local.set 28
      local.get 27
      i32.const 0
      i32.sub
      local.set 29
      block  ;; label = @2
        block  ;; label = @3
          local.get 27
          i32.const 8
          i32.le_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        local.get 27
        i32.const 8
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 29
      local.set 30
      local.get 28
      local.set 31
      block  ;; label = @2
        block  ;; label = @3
          local.get 24
          local.get 30
          i32.eq
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
        unreachable
      end
      local.get 31
      local.get 24
      i32.add
      local.set 32
      local.get 25
      local.get 24
      i32.add
      local.set 33
      block  ;; label = @2
        block  ;; label = @3
          local.get 25
          local.get 32
          i32.ge_u
          local.get 31
          local.get 33
          i32.ge_u
          i32.or
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
        unreachable
      end
      block  ;; label = @2
        local.get 24
        i32.eqz
        br_if 0 (;@2;)
        local.get 25
        local.get 31
        local.get 24
        memory.copy
      end
    end
    local.get 4
    i32.const 96
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $crypto.keccak_p.KeccakF_1600_.addBytes (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i64 i32 i32 i32 i32 i64 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i32 i64 i64)
    global.get $m6__stack_pointer
    i32.const 96
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 1
    i32.store offset=12
    local.get 4
    local.get 3
    i32.store offset=20
    local.get 4
    local.get 2
    i32.store offset=16
    local.get 4
    local.get 1
    i32.store offset=24
    local.get 4
    local.get 3
    i32.store offset=32
    local.get 4
    local.get 2
    i32.store offset=28
    local.get 4
    i32.const 0
    i32.store offset=36
    loop  ;; label = @1
      local.get 4
      i32.load offset=36
      local.set 5
      local.get 5
      i32.const 8
      i32.add
      local.set 6
      block  ;; label = @2
        local.get 6
        local.get 5
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 6
                local.get 4
                i32.load offset=32
                i32.le_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 4
                i32.load offset=24
                local.set 7
                local.get 4
                i32.load offset=36
                i32.const 3
                i32.shr_u
                local.set 8
                local.get 8
                i32.const 25
                i32.lt_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 3 (;@2;)
            end
            br 1 (;@3;)
          end
          local.get 0
          local.get 8
          i32.const 25
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 7
        local.get 8
        i32.const 3
        i32.shl
        i32.add
        local.set 9
        local.get 9
        i64.load
        local.set 10
        local.get 4
        i32.load offset=36
        local.set 11
        local.get 4
        i32.load offset=32
        local.set 12
        local.get 11
        local.get 4
        i32.load offset=28
        i32.add
        local.set 13
        local.get 11
        i32.const 8
        i32.add
        local.set 14
        block  ;; label = @3
          block  ;; label = @4
            local.get 14
            local.get 12
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 14
          local.get 12
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        local.get 13
        i32.store offset=40
        local.get 4
        i32.const 1
        i32.const 1
        i32.and
        i32.store8 offset=47
        local.get 4
        local.get 13
        i64.load align=1
        i64.store offset=48
        local.get 4
        i64.load offset=48 align=1
        local.set 15
        local.get 4
        local.get 15
        i64.store offset=56
        local.get 15
        local.set 16
        local.get 9
        local.get 10
        local.get 16
        i64.xor
        i64.store
        local.get 4
        i32.load offset=36
        local.set 17
        local.get 17
        i32.const 8
        i32.add
        local.set 18
        block  ;; label = @3
          local.get 18
          local.get 17
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 18
        i32.store offset=36
        br 1 (;@1;)
      end
    end
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            local.get 4
            i32.load offset=36
            local.get 4
            i32.load offset=32
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            local.get 4
            i64.const 0
            i64.store offset=64
            local.get 4
            i32.load offset=32
            local.set 19
            local.get 19
            local.get 4
            i32.load offset=36
            i32.sub
            local.set 20
            local.get 20
            local.get 19
            i32.gt_u
            i32.const 1
            i32.and
            br_if 1 (;@3;)
            br 2 (;@2;)
          end
          br 2 (;@1;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
        unreachable
      end
      local.get 4
      i32.const 64
      i32.add
      local.set 21
      local.get 20
      i32.const 0
      i32.sub
      local.set 22
      block  ;; label = @2
        block  ;; label = @3
          local.get 20
          i32.const 8
          i32.le_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        local.get 20
        i32.const 8
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 22
      local.set 23
      local.get 21
      local.set 24
      local.get 4
      i32.load offset=36
      local.set 25
      local.get 4
      i32.load offset=32
      local.set 26
      local.get 25
      local.get 4
      i32.load offset=28
      i32.add
      local.set 27
      block  ;; label = @2
        block  ;; label = @3
          local.get 25
          local.get 26
          i32.le_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        local.get 25
        local.get 26
        call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
        unreachable
      end
      local.get 26
      local.get 25
      i32.sub
      local.set 28
      block  ;; label = @2
        block  ;; label = @3
          local.get 26
          local.get 26
          i32.le_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        local.get 26
        local.get 26
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 28
      local.set 29
      local.get 27
      local.set 30
      block  ;; label = @2
        block  ;; label = @3
          local.get 23
          local.get 29
          i32.eq
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
        unreachable
      end
      local.get 30
      local.get 23
      i32.add
      local.set 31
      local.get 24
      local.get 23
      i32.add
      local.set 32
      block  ;; label = @2
        block  ;; label = @3
          local.get 24
          local.get 31
          i32.ge_u
          local.get 30
          local.get 32
          i32.ge_u
          i32.or
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
        unreachable
      end
      block  ;; label = @2
        local.get 23
        i32.eqz
        br_if 0 (;@2;)
        local.get 24
        local.get 30
        local.get 23
        memory.copy
      end
      local.get 4
      i32.load offset=24
      local.set 33
      local.get 4
      i32.load offset=36
      i32.const 3
      i32.shr_u
      local.set 34
      block  ;; label = @2
        block  ;; label = @3
          local.get 34
          i32.const 25
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          br 1 (;@2;)
        end
        local.get 0
        local.get 34
        i32.const 25
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      local.get 33
      local.get 34
      i32.const 3
      i32.shl
      i32.add
      local.set 35
      local.get 35
      i64.load
      local.set 36
      local.get 4
      i32.const 64
      i32.add
      local.set 37
      local.get 4
      local.get 37
      i32.store offset=72
      local.get 4
      i32.const 1
      i32.const 1
      i32.and
      i32.store8 offset=79
      local.get 4
      local.get 37
      i64.load align=1
      i64.store offset=80
      local.get 4
      i64.load offset=80 align=1
      local.set 38
      local.get 4
      local.get 38
      i64.store offset=88
      local.get 38
      local.set 39
      local.get 35
      local.get 36
      local.get 39
      i64.xor
      i64.store
    end
    local.get 4
    i32.const 96
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $math.rotl__anon_8631 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 1
    i32.store8 offset=15
    local.get 1
    i64.const 1
    i64.shl
    local.get 1
    i64.const 63
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8636 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 3
    i32.store8 offset=15
    local.get 1
    i64.const 3
    i64.shl
    local.get 1
    i64.const 61
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8637 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 6
    i32.store8 offset=15
    local.get 1
    i64.const 6
    i64.shl
    local.get 1
    i64.const 58
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8638 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 10
    i32.store8 offset=15
    local.get 1
    i64.const 10
    i64.shl
    local.get 1
    i64.const 54
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8639 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 15
    i32.store8 offset=15
    local.get 1
    i64.const 15
    i64.shl
    local.get 1
    i64.const 49
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8640 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 21
    i32.store8 offset=15
    local.get 1
    i64.const 21
    i64.shl
    local.get 1
    i64.const 43
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8641 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 28
    i32.store8 offset=15
    local.get 1
    i64.const 28
    i64.shl
    local.get 1
    i64.const 36
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8642 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 36
    i32.store8 offset=15
    local.get 1
    i64.const 36
    i64.shl
    local.get 1
    i64.const 28
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8643 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 45
    i32.store8 offset=15
    local.get 1
    i64.const 45
    i64.shl
    local.get 1
    i64.const 19
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8644 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 55
    i32.store8 offset=15
    local.get 1
    i64.const 55
    i64.shl
    local.get 1
    i64.const 9
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8645 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 2
    i32.store8 offset=15
    local.get 1
    i64.const 2
    i64.shl
    local.get 1
    i64.const 62
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8646 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 14
    i32.store8 offset=15
    local.get 1
    i64.const 14
    i64.shl
    local.get 1
    i64.const 50
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8647 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 27
    i32.store8 offset=15
    local.get 1
    i64.const 27
    i64.shl
    local.get 1
    i64.const 37
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8648 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 41
    i32.store8 offset=15
    local.get 1
    i64.const 41
    i64.shl
    local.get 1
    i64.const 23
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8649 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 56
    i32.store8 offset=15
    local.get 1
    i64.const 56
    i64.shl
    local.get 1
    i64.const 8
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8650 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 8
    i32.store8 offset=15
    local.get 1
    i64.const 8
    i64.shl
    local.get 1
    i64.const 56
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8651 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 25
    i32.store8 offset=15
    local.get 1
    i64.const 25
    i64.shl
    local.get 1
    i64.const 39
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8652 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 43
    i32.store8 offset=15
    local.get 1
    i64.const 43
    i64.shl
    local.get 1
    i64.const 21
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8653 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 62
    i32.store8 offset=15
    local.get 1
    i64.const 62
    i64.shl
    local.get 1
    i64.const 2
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8654 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 18
    i32.store8 offset=15
    local.get 1
    i64.const 18
    i64.shl
    local.get 1
    i64.const 46
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8655 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 39
    i32.store8 offset=15
    local.get 1
    i64.const 39
    i64.shl
    local.get 1
    i64.const 25
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8656 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 61
    i32.store8 offset=15
    local.get 1
    i64.const 61
    i64.shl
    local.get 1
    i64.const 3
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8657 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 20
    i32.store8 offset=15
    local.get 1
    i64.const 20
    i64.shl
    local.get 1
    i64.const 44
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $math.rotl__anon_8658 (type $t_6_12) (param i32 i64) (result i64)
    (local i32 i64)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i64.store
    local.get 2
    i32.const 44
    i32.store8 offset=15
    local.get 1
    i64.const 44
    i64.shl
    local.get 1
    i64.const 20
    i64.shr_u
    i64.or
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $__zig_is_named_enum_value_crypto.keccak_p.State.Op (type $t_6_13) (param i32) (result i32)
    (local i32)
    block  ;; label = @1
      block  ;; label = @2
        local.get 0
        i32.const 7
        i32.and
        i32.const 4
        i32.eq
        br_if 0 (;@2;)
        i32.const 29
        local.set 1
        local.get 0
        local.get 1
        i32.shl
        local.get 1
        i32.shr_s
        i32.const 0
        i32.lt_s
        br_if 1 (;@1;)
      end
      i32.const 1
      return
    end
    i32.const 0
    return)
  (func $crypto.keccak_p.KeccakF_1600_.addByte (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 32
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 1
    i32.store offset=12
    local.get 4
    local.get 2
    i32.store8 offset=19
    local.get 4
    local.get 3
    i32.store offset=20
    local.get 4
    local.get 1
    i32.store offset=24
    local.get 3
    i32.const 7
    i32.and
    i32.const 3
    i32.shl
    i32.const 504
    i32.and
    local.set 5
    block  ;; label = @1
      local.get 5
      i32.const 6
      i32.shr_u
      i32.const 0
      i32.ne
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 5
    local.set 6
    local.get 4
    local.get 5
    i32.const 63
    i32.and
    i32.store8 offset=31
    local.get 4
    i32.load offset=24
    local.set 7
    local.get 3
    i32.const 3
    i32.shr_u
    local.set 8
    block  ;; label = @1
      block  ;; label = @2
        local.get 8
        i32.const 25
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 8
      i32.const 25
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 7
    local.get 8
    i32.const 3
    i32.shl
    i32.add
    local.set 9
    local.get 9
    local.get 9
    i64.load
    local.get 2
    i64.extend_i32_u
    i64.const 255
    i64.and
    local.get 6
    i64.extend_i32_u
    i64.const 63
    i64.and
    i64.shl
    i64.xor
    i64.store
    local.get 4
    i32.const 32
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $root.route_decode (type $t_6_4) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 8208
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 0
    i32.store
    local.get 4
    local.get 1
    i32.store offset=4
    local.get 4
    local.get 2
    i32.store offset=8
    local.get 4
    local.get 3
    i32.store offset=12
    local.get 4
    i32.const 32
    i32.store offset=152
    local.get 4
    local.get 4
    i32.const 16
    i32.add
    i32.store offset=148
    local.get 4
    i32.const 0
    i32.store offset=144
    local.get 4
    local.get 0
    i32.store offset=156
    local.get 4
    local.get 2
    i32.store offset=160
    i32.const 8000
    local.set 5
    i32.const 170
    local.set 6
    block  ;; label = @1
      local.get 5
      i32.eqz
      br_if 0 (;@1;)
      local.get 4
      i32.const 164
      i32.add
      local.get 6
      local.get 5
      memory.fill
    end
    local.get 4
    i32.load offset=156
    local.set 7
    local.get 1
    local.set 8
    i32.const 100
    local.set 9
    local.get 3
    local.get 9
    local.get 3
    local.get 9
    i32.lt_u
    select
    local.set 10
    local.get 4
    i32.const 164
    i32.add
    local.set 11
    local.get 10
    i32.const 0
    i32.sub
    local.set 12
    block  ;; label = @1
      block  ;; label = @2
        local.get 10
        i32.const 100
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 4
      i32.const 144
      i32.add
      local.get 10
      i32.const 100
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 12
    local.set 13
    local.get 11
    local.set 14
    local.get 4
    i32.const 144
    i32.add
    local.get 7
    local.get 8
    local.get 14
    local.get 13
    call $route.routeDeltaDecode
    local.set 15
    local.get 4
    local.get 15
    i32.store offset=8164
    local.get 4
    i32.const 0
    i32.store offset=8168
    local.get 4
    i32.const 0
    i32.store offset=8172
    block  ;; label = @1
      loop  ;; label = @2
        local.get 4
        i32.load offset=8172
        local.set 16
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 16
                local.get 15
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 4
                local.get 16
                i32.store offset=8176
                local.get 4
                i32.load offset=8168
                local.get 4
                i32.load offset=160
                i32.add
                local.set 17
                local.get 16
                i32.const 100
                i32.lt_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 4
          i32.const 144
          i32.add
          local.get 16
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.const 164
        i32.add
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.const 10
        i32.add
        local.set 18
        local.get 18
        i32.const 32
        i32.add
        local.set 19
        local.get 17
        i32.const 32
        i32.add
        local.set 20
        block  ;; label = @3
          block  ;; label = @4
            local.get 17
            local.get 19
            i32.ge_u
            local.get 18
            local.get 20
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 144
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        local.get 17
        local.get 18
        i64.load align=1
        i64.store align=1
        i32.const 24
        local.set 21
        local.get 17
        local.get 21
        i32.add
        local.get 18
        local.get 21
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 16
        local.set 22
        local.get 17
        local.get 22
        i32.add
        local.get 18
        local.get 22
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 8
        local.set 23
        local.get 17
        local.get 23
        i32.add
        local.get 18
        local.get 23
        i32.add
        i64.load align=1
        i64.store align=1
        local.get 4
        i32.load offset=8168
        local.set 24
        local.get 24
        i32.const 32
        i32.add
        local.set 25
        block  ;; label = @3
          local.get 25
          local.get 24
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 144
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 25
        i32.store offset=8168
        local.get 4
        i32.load offset=8168
        local.get 4
        i32.load offset=160
        i32.add
        local.set 26
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            i32.const 100
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 144
          i32.add
          local.get 16
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.const 164
        i32.add
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.const 42
        i32.add
        local.set 27
        local.get 27
        i32.const 32
        i32.add
        local.set 28
        local.get 26
        i32.const 32
        i32.add
        local.set 29
        block  ;; label = @3
          block  ;; label = @4
            local.get 26
            local.get 28
            i32.ge_u
            local.get 27
            local.get 29
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 144
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        local.get 26
        local.get 27
        i64.load align=1
        i64.store align=1
        i32.const 24
        local.set 30
        local.get 26
        local.get 30
        i32.add
        local.get 27
        local.get 30
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 16
        local.set 31
        local.get 26
        local.get 31
        i32.add
        local.get 27
        local.get 31
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 8
        local.set 32
        local.get 26
        local.get 32
        i32.add
        local.get 27
        local.get 32
        i32.add
        i64.load align=1
        i64.store align=1
        local.get 4
        i32.load offset=8168
        local.set 33
        local.get 33
        i32.const 32
        i32.add
        local.set 34
        block  ;; label = @3
          local.get 34
          local.get 33
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 144
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 34
        i32.store offset=8168
        local.get 4
        i32.load offset=8168
        local.get 4
        i32.load offset=160
        i32.add
        local.set 35
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            i32.const 100
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 144
          i32.add
          local.get 16
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.const 164
        i32.add
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.load16_u offset=8
        local.set 36
        local.get 4
        local.get 35
        i32.store offset=8180
        local.get 4
        local.get 36
        i32.store16 offset=8186
        local.get 4
        i32.const 0
        i32.const 1
        i32.and
        i32.store8 offset=8189
        i32.const 8
        local.set 37
        local.get 4
        local.get 36
        local.get 37
        i32.shl
        local.get 36
        i32.const 65280
        i32.and
        local.get 37
        i32.shr_u
        i32.or
        i32.store16 offset=8190
        local.get 35
        local.get 4
        i32.load16_u offset=8190
        i32.store16 align=1
        local.get 4
        i32.load offset=8168
        local.set 38
        local.get 38
        i32.const 2
        i32.add
        local.set 39
        block  ;; label = @3
          local.get 39
          local.get 38
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 144
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 39
        i32.store offset=8168
        local.get 4
        i32.load offset=8168
        local.get 4
        i32.load offset=160
        i32.add
        local.set 40
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            i32.const 100
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 144
          i32.add
          local.get 16
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.const 164
        i32.add
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.load
        local.set 41
        local.get 4
        local.get 40
        i32.store offset=8192
        local.get 4
        local.get 41
        i32.store offset=8196
        local.get 4
        i32.const 0
        i32.const 1
        i32.and
        i32.store8 offset=8203
        i32.const 24
        local.set 42
        local.get 41
        local.get 42
        i32.shr_u
        local.set 43
        i32.const 8
        local.set 44
        local.get 41
        local.get 44
        i32.shr_u
        local.set 45
        i32.const 65280
        local.set 46
        local.get 4
        local.get 43
        local.get 45
        local.get 46
        i32.and
        i32.or
        local.get 41
        local.get 42
        i32.shl
        local.get 41
        local.get 46
        i32.and
        local.get 44
        i32.shl
        i32.or
        i32.or
        i32.store offset=8204
        local.get 40
        local.get 4
        i32.load offset=8204
        i32.store align=1
        local.get 4
        i32.load offset=8168
        local.set 47
        local.get 47
        i32.const 4
        i32.add
        local.set 48
        block  ;; label = @3
          local.get 48
          local.get 47
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 144
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 48
        i32.store offset=8168
        local.get 4
        i32.load offset=8168
        local.get 4
        i32.load offset=160
        i32.add
        local.set 49
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            i32.const 100
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 144
          i32.add
          local.get 16
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 49
        local.get 4
        i32.const 164
        i32.add
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.load8_u offset=74
        i32.store8
        local.get 4
        i32.load offset=8168
        i32.const 1
        i32.add
        local.set 50
        block  ;; label = @3
          local.get 50
          i32.eqz
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 144
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 50
        i32.store offset=8168
        local.get 4
        i32.load offset=8168
        local.get 4
        i32.load offset=160
        i32.add
        local.set 51
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            i32.const 100
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 144
          i32.add
          local.get 16
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 51
        local.get 4
        i32.const 164
        i32.add
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.load8_u offset=75
        i32.store8
        local.get 4
        i32.load offset=8168
        i32.const 1
        i32.add
        local.set 52
        block  ;; label = @3
          local.get 52
          i32.eqz
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 144
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 52
        i32.store offset=8168
        local.get 4
        local.get 16
        i32.const 1
        i32.add
        i32.store offset=8172
        br 0 (;@2;)
      end
    end
    block  ;; label = @1
      local.get 15
      i32.const 2147483647
      i32.le_u
      i32.const 1
      i32.and
      br_if 0 (;@1;)
      local.get 4
      i32.const 144
      i32.add
      call $debug.FullPanic__function_'defaultPanic'__.integerOutOfBounds
      unreachable
    end
    local.get 4
    i32.const 8208
    i32.add
    global.set $m6__stack_pointer
    local.get 15
    return)
  (func $route.routeDeltaDecode (type $t_6_7) (param i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 96
    i32.sub
    local.set 5
    local.get 5
    global.set $m6__stack_pointer
    local.get 5
    local.get 2
    i32.store offset=12
    local.get 5
    local.get 1
    i32.store offset=8
    local.get 5
    local.get 4
    i32.store offset=20
    local.get 5
    local.get 3
    i32.store offset=16
    local.get 5
    local.get 2
    i32.store offset=28
    local.get 5
    local.get 1
    i32.store offset=24
    local.get 5
    local.get 4
    i32.store offset=36
    local.get 5
    local.get 3
    i32.store offset=32
    block  ;; label = @1
      local.get 5
      i32.load offset=28
      i32.const 2
      i32.lt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      i32.const 0
      local.set 6
      local.get 5
      i32.const 96
      i32.add
      global.set $m6__stack_pointer
      local.get 6
      return
    end
    local.get 5
    i32.load offset=28
    local.set 7
    local.get 5
    i32.load offset=24
    local.set 8
    block  ;; label = @1
      block  ;; label = @2
        i32.const 2
        local.get 7
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 2
      local.get 7
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 5
    local.get 8
    i32.store offset=40
    local.get 5
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=45
    local.get 5
    local.get 8
    i32.load16_u align=1
    i32.store16 offset=46
    local.get 5
    i32.load16_u offset=46 align=1
    local.set 9
    local.get 5
    local.get 9
    i32.store16 offset=48
    i32.const 8
    local.set 10
    local.get 9
    local.get 10
    i32.shl
    local.get 9
    i32.const 65280
    i32.and
    local.get 10
    i32.shr_u
    i32.or
    local.set 11
    local.get 11
    local.set 12
    local.get 5
    local.get 12
    i32.store16 offset=50
    local.get 12
    i32.const 65535
    i32.and
    local.set 13
    local.get 5
    i32.load offset=36
    local.set 14
    local.get 13
    local.get 14
    local.get 13
    local.get 14
    i32.lt_u
    select
    local.set 15
    local.get 5
    local.get 15
    i32.store offset=52
    local.get 5
    i32.const 2
    i32.store offset=56
    local.get 5
    i32.const 0
    i32.store offset=60
    block  ;; label = @1
      loop  ;; label = @2
        local.get 5
        i32.load offset=60
        local.set 16
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 16
              local.get 15
              i32.lt_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              local.get 5
              local.get 16
              i32.store offset=64
              local.get 5
              i32.load offset=56
              local.set 17
              local.get 17
              i32.const 72
              i32.add
              local.set 18
              local.get 18
              local.get 17
              i32.lt_u
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            br 3 (;@1;)
          end
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        block  ;; label = @3
          local.get 18
          local.get 5
          i32.load offset=28
          i32.gt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 5
          i32.const 96
          i32.add
          global.set $m6__stack_pointer
          local.get 16
          return
        end
        local.get 5
        i32.load offset=36
        local.set 19
        local.get 5
        i32.load offset=32
        local.set 20
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            local.get 19
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 16
          local.get 19
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 20
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.const 10
        i32.add
        local.set 21
        local.get 5
        i32.load offset=56
        local.set 22
        local.get 5
        i32.load offset=28
        local.set 23
        local.get 22
        local.get 5
        i32.load offset=24
        i32.add
        local.set 24
        local.get 22
        i32.const 32
        i32.add
        local.set 25
        block  ;; label = @3
          block  ;; label = @4
            local.get 25
            local.get 23
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 25
          local.get 23
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 24
        i32.const 32
        i32.add
        local.set 26
        local.get 21
        i32.const 32
        i32.add
        local.set 27
        block  ;; label = @3
          block  ;; label = @4
            local.get 21
            local.get 26
            i32.ge_u
            local.get 24
            local.get 27
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        local.get 21
        local.get 24
        i64.load align=1
        i64.store align=1
        i32.const 24
        local.set 28
        local.get 21
        local.get 28
        i32.add
        local.get 24
        local.get 28
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 16
        local.set 29
        local.get 21
        local.get 29
        i32.add
        local.get 24
        local.get 29
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 8
        local.set 30
        local.get 21
        local.get 30
        i32.add
        local.get 24
        local.get 30
        i32.add
        i64.load align=1
        i64.store align=1
        local.get 5
        i32.load offset=56
        local.set 31
        local.get 31
        i32.const 32
        i32.add
        local.set 32
        block  ;; label = @3
          local.get 32
          local.get 31
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 32
        i32.store offset=56
        local.get 5
        i32.load offset=36
        local.set 33
        local.get 5
        i32.load offset=32
        local.set 34
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            local.get 33
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 16
          local.get 33
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 34
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.const 42
        i32.add
        local.set 35
        local.get 5
        i32.load offset=56
        local.set 36
        local.get 5
        i32.load offset=28
        local.set 37
        local.get 36
        local.get 5
        i32.load offset=24
        i32.add
        local.set 38
        local.get 36
        i32.const 32
        i32.add
        local.set 39
        block  ;; label = @3
          block  ;; label = @4
            local.get 39
            local.get 37
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 39
          local.get 37
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 38
        i32.const 32
        i32.add
        local.set 40
        local.get 35
        i32.const 32
        i32.add
        local.set 41
        block  ;; label = @3
          block  ;; label = @4
            local.get 35
            local.get 40
            i32.ge_u
            local.get 38
            local.get 41
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        local.get 35
        local.get 38
        i64.load align=1
        i64.store align=1
        i32.const 24
        local.set 42
        local.get 35
        local.get 42
        i32.add
        local.get 38
        local.get 42
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 16
        local.set 43
        local.get 35
        local.get 43
        i32.add
        local.get 38
        local.get 43
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 8
        local.set 44
        local.get 35
        local.get 44
        i32.add
        local.get 38
        local.get 44
        i32.add
        i64.load align=1
        i64.store align=1
        local.get 5
        i32.load offset=56
        local.set 45
        local.get 45
        i32.const 32
        i32.add
        local.set 46
        block  ;; label = @3
          local.get 46
          local.get 45
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 46
        i32.store offset=56
        local.get 5
        i32.load offset=36
        local.set 47
        local.get 5
        i32.load offset=32
        local.set 48
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            local.get 47
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 16
          local.get 47
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 48
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.const 8
        i32.add
        local.set 49
        local.get 5
        i32.load offset=56
        local.set 50
        local.get 5
        i32.load offset=28
        local.set 51
        local.get 50
        local.get 5
        i32.load offset=24
        i32.add
        local.set 52
        local.get 50
        i32.const 2
        i32.add
        local.set 53
        block  ;; label = @3
          block  ;; label = @4
            local.get 53
            local.get 51
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 53
          local.get 51
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 5
        local.get 52
        i32.store offset=68
        local.get 5
        i32.const 0
        i32.const 1
        i32.and
        i32.store8 offset=75
        local.get 5
        local.get 52
        i32.load16_u align=1
        i32.store16 offset=76
        local.get 5
        i32.load16_u offset=76 align=1
        local.set 54
        local.get 5
        local.get 54
        i32.store16 offset=78
        i32.const 8
        local.set 55
        local.get 54
        local.get 55
        i32.shl
        local.get 54
        i32.const 65280
        i32.and
        local.get 55
        i32.shr_u
        i32.or
        local.set 56
        local.get 49
        local.get 56
        i32.store16
        local.get 5
        i32.load offset=56
        local.set 57
        local.get 57
        i32.const 2
        i32.add
        local.set 58
        block  ;; label = @3
          local.get 58
          local.get 57
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 58
        i32.store offset=56
        local.get 5
        i32.load offset=36
        local.set 59
        local.get 5
        i32.load offset=32
        local.set 60
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            local.get 59
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 16
          local.get 59
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 60
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        local.set 61
        local.get 5
        i32.load offset=56
        local.set 62
        local.get 5
        i32.load offset=28
        local.set 63
        local.get 62
        local.get 5
        i32.load offset=24
        i32.add
        local.set 64
        local.get 62
        i32.const 4
        i32.add
        local.set 65
        block  ;; label = @3
          block  ;; label = @4
            local.get 65
            local.get 63
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 65
          local.get 63
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 5
        local.get 64
        i32.store offset=80
        local.get 5
        i32.const 0
        i32.const 1
        i32.and
        i32.store8 offset=87
        local.get 5
        local.get 64
        i32.load align=1
        i32.store offset=88
        local.get 5
        i32.load offset=88 align=1
        local.set 66
        local.get 5
        local.get 66
        i32.store offset=92
        i32.const 24
        local.set 67
        local.get 66
        local.get 67
        i32.shr_u
        local.set 68
        i32.const 8
        local.set 69
        local.get 66
        local.get 69
        i32.shr_u
        local.set 70
        i32.const 65280
        local.set 71
        local.get 68
        local.get 70
        local.get 71
        i32.and
        i32.or
        local.get 66
        local.get 67
        i32.shl
        local.get 66
        local.get 71
        i32.and
        local.get 69
        i32.shl
        i32.or
        i32.or
        local.set 72
        local.get 61
        local.get 72
        i32.store
        local.get 5
        i32.load offset=56
        local.set 73
        local.get 73
        i32.const 4
        i32.add
        local.set 74
        block  ;; label = @3
          local.get 74
          local.get 73
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 74
        i32.store offset=56
        local.get 5
        i32.load offset=36
        local.set 75
        local.get 5
        i32.load offset=32
        local.set 76
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            local.get 75
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 16
          local.get 75
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 76
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.const 74
        i32.add
        local.set 77
        local.get 5
        i32.load offset=56
        local.set 78
        local.get 5
        i32.load offset=28
        local.set 79
        local.get 5
        i32.load offset=24
        local.set 80
        block  ;; label = @3
          block  ;; label = @4
            local.get 78
            local.get 79
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 78
          local.get 79
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 77
        local.get 80
        local.get 78
        i32.add
        i32.load8_u
        i32.store8
        local.get 5
        i32.load offset=56
        i32.const 1
        i32.add
        local.set 81
        block  ;; label = @3
          local.get 81
          i32.eqz
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 81
        i32.store offset=56
        local.get 5
        i32.load offset=36
        local.set 82
        local.get 5
        i32.load offset=32
        local.set 83
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            local.get 82
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 16
          local.get 82
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 83
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.const 75
        i32.add
        local.set 84
        local.get 5
        i32.load offset=56
        local.set 85
        local.get 5
        i32.load offset=28
        local.set 86
        local.get 5
        i32.load offset=24
        local.set 87
        block  ;; label = @3
          block  ;; label = @4
            local.get 85
            local.get 86
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 85
          local.get 86
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 84
        local.get 87
        local.get 85
        i32.add
        i32.load8_u
        i32.store8
        local.get 5
        i32.load offset=56
        i32.const 1
        i32.add
        local.set 88
        block  ;; label = @3
          local.get 88
          i32.eqz
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 88
        i32.store offset=56
        local.get 5
        i32.load offset=36
        local.set 89
        local.get 5
        i32.load offset=32
        local.set 90
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            local.get 89
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 16
          local.get 89
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 90
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.const 1
        i32.store8 offset=76
        local.get 5
        i32.load offset=36
        local.set 91
        local.get 5
        i32.load offset=32
        local.set 92
        block  ;; label = @3
          block  ;; label = @4
            local.get 16
            local.get 91
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 16
          local.get 91
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 92
        local.get 16
        i32.const 80
        i32.mul
        i32.add
        i32.const 0
        i32.store offset=4
        local.get 5
        local.get 16
        i32.const 1
        i32.add
        i32.store offset=60
        br 0 (;@2;)
      end
    end
    local.get 5
    i32.const 96
    i32.add
    global.set $m6__stack_pointer
    local.get 15
    return)
  (func $root.route_encode (type $t_6_4) (param i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 8224
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 0
    i32.store offset=4
    local.get 4
    local.get 1
    i32.store offset=8
    local.get 4
    local.get 2
    i32.store offset=12
    local.get 4
    local.get 3
    i32.store offset=16
    local.get 4
    i32.const 32
    i32.store offset=156
    local.get 4
    local.get 4
    i32.const 20
    i32.add
    i32.store offset=152
    i32.const 0
    local.set 5
    local.get 4
    local.get 5
    i32.store offset=148
    local.get 4
    local.get 0
    i32.store offset=160
    local.get 4
    local.get 2
    i32.store offset=164
    i32.const 100
    local.set 6
    local.get 1
    local.get 6
    local.get 1
    local.get 6
    i32.lt_u
    select
    local.set 7
    local.get 7
    local.set 8
    local.get 4
    local.get 7
    i32.const 127
    i32.and
    i32.store8 offset=171
    i32.const 8000
    local.set 9
    i32.const 170
    local.set 10
    block  ;; label = @1
      local.get 9
      i32.eqz
      br_if 0 (;@1;)
      local.get 4
      i32.const 172
      i32.add
      local.get 10
      local.get 9
      memory.fill
    end
    local.get 4
    local.get 5
    i32.store offset=8172
    local.get 4
    local.get 5
    i32.store offset=8176
    block  ;; label = @1
      loop  ;; label = @2
        local.get 4
        i32.load offset=8176
        local.set 11
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 11
                local.get 7
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 4
                local.get 11
                i32.store offset=8180
                local.get 11
                i32.const 100
                i32.lt_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 4
          i32.const 148
          i32.add
          local.get 11
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.const 172
        i32.add
        local.get 11
        i32.const 80
        i32.mul
        i32.add
        i32.const 10
        i32.add
        local.set 12
        local.get 4
        i32.load offset=8172
        local.get 4
        i32.load offset=160
        i32.add
        local.set 13
        local.get 13
        i32.const 32
        i32.add
        local.set 14
        local.get 12
        i32.const 32
        i32.add
        local.set 15
        block  ;; label = @3
          block  ;; label = @4
            local.get 12
            local.get 14
            i32.ge_u
            local.get 13
            local.get 15
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 148
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        local.get 12
        local.get 13
        i64.load align=1
        i64.store align=1
        i32.const 24
        local.set 16
        local.get 12
        local.get 16
        i32.add
        local.get 13
        local.get 16
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 16
        local.set 17
        local.get 12
        local.get 17
        i32.add
        local.get 13
        local.get 17
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 8
        local.set 18
        local.get 12
        local.get 18
        i32.add
        local.get 13
        local.get 18
        i32.add
        i64.load align=1
        i64.store align=1
        local.get 4
        i32.load offset=8172
        local.set 19
        local.get 19
        i32.const 32
        i32.add
        local.set 20
        block  ;; label = @3
          local.get 20
          local.get 19
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 148
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 20
        i32.store offset=8172
        block  ;; label = @3
          block  ;; label = @4
            local.get 11
            i32.const 100
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 148
          i32.add
          local.get 11
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.const 172
        i32.add
        local.get 11
        i32.const 80
        i32.mul
        i32.add
        i32.const 42
        i32.add
        local.set 21
        local.get 4
        i32.load offset=8172
        local.get 4
        i32.load offset=160
        i32.add
        local.set 22
        local.get 22
        i32.const 32
        i32.add
        local.set 23
        local.get 21
        i32.const 32
        i32.add
        local.set 24
        block  ;; label = @3
          block  ;; label = @4
            local.get 21
            local.get 23
            i32.ge_u
            local.get 22
            local.get 24
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 148
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        local.get 21
        local.get 22
        i64.load align=1
        i64.store align=1
        i32.const 24
        local.set 25
        local.get 21
        local.get 25
        i32.add
        local.get 22
        local.get 25
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 16
        local.set 26
        local.get 21
        local.get 26
        i32.add
        local.get 22
        local.get 26
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 8
        local.set 27
        local.get 21
        local.get 27
        i32.add
        local.get 22
        local.get 27
        i32.add
        i64.load align=1
        i64.store align=1
        local.get 4
        i32.load offset=8172
        local.set 28
        local.get 28
        i32.const 32
        i32.add
        local.set 29
        block  ;; label = @3
          local.get 29
          local.get 28
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 148
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 29
        i32.store offset=8172
        block  ;; label = @3
          block  ;; label = @4
            local.get 11
            i32.const 100
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 148
          i32.add
          local.get 11
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.const 172
        i32.add
        local.get 11
        i32.const 80
        i32.mul
        i32.add
        i32.const 8
        i32.add
        local.set 30
        local.get 4
        i32.load offset=8172
        local.get 4
        i32.load offset=160
        i32.add
        local.set 31
        local.get 4
        local.get 31
        i32.store offset=8184
        local.get 4
        i32.const 0
        i32.const 1
        i32.and
        i32.store8 offset=8191
        local.get 4
        local.get 31
        i32.load16_u align=1
        i32.store16 offset=8192
        local.get 4
        i32.load16_u offset=8192 align=1
        local.set 32
        local.get 4
        local.get 32
        i32.store16 offset=8194
        i32.const 8
        local.set 33
        local.get 32
        local.get 33
        i32.shl
        local.get 32
        i32.const 65280
        i32.and
        local.get 33
        i32.shr_u
        i32.or
        local.set 34
        local.get 30
        local.get 34
        i32.store16
        local.get 4
        i32.load offset=8172
        local.set 35
        local.get 35
        i32.const 2
        i32.add
        local.set 36
        block  ;; label = @3
          local.get 36
          local.get 35
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 148
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 36
        i32.store offset=8172
        block  ;; label = @3
          block  ;; label = @4
            local.get 11
            i32.const 100
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 148
          i32.add
          local.get 11
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.const 172
        i32.add
        local.get 11
        i32.const 80
        i32.mul
        i32.add
        local.set 37
        local.get 4
        i32.load offset=8172
        local.get 4
        i32.load offset=160
        i32.add
        local.set 38
        local.get 4
        local.get 38
        i32.store offset=8196
        local.get 4
        i32.const 0
        i32.const 1
        i32.and
        i32.store8 offset=8203
        local.get 4
        local.get 38
        i32.load align=1
        i32.store offset=8204
        local.get 4
        i32.load offset=8204 align=1
        local.set 39
        local.get 4
        local.get 39
        i32.store offset=8208
        i32.const 24
        local.set 40
        local.get 39
        local.get 40
        i32.shr_u
        local.set 41
        i32.const 8
        local.set 42
        local.get 39
        local.get 42
        i32.shr_u
        local.set 43
        i32.const 65280
        local.set 44
        local.get 41
        local.get 43
        local.get 44
        i32.and
        i32.or
        local.get 39
        local.get 40
        i32.shl
        local.get 39
        local.get 44
        i32.and
        local.get 42
        i32.shl
        i32.or
        i32.or
        local.set 45
        local.get 37
        local.get 45
        i32.store
        local.get 4
        i32.load offset=8172
        local.set 46
        local.get 46
        i32.const 4
        i32.add
        local.set 47
        block  ;; label = @3
          local.get 47
          local.get 46
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 148
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 47
        i32.store offset=8172
        block  ;; label = @3
          block  ;; label = @4
            local.get 11
            i32.const 100
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 148
          i32.add
          local.get 11
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.const 172
        i32.add
        local.get 11
        i32.const 80
        i32.mul
        i32.add
        local.get 4
        i32.load offset=8172
        local.get 4
        i32.load offset=160
        i32.add
        i32.load8_u
        i32.store8 offset=74
        local.get 4
        i32.load offset=8172
        i32.const 1
        i32.add
        local.set 48
        block  ;; label = @3
          local.get 48
          i32.eqz
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 148
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 48
        i32.store offset=8172
        block  ;; label = @3
          block  ;; label = @4
            local.get 11
            i32.const 100
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 4
          i32.const 148
          i32.add
          local.get 11
          i32.const 100
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 4
        i32.const 172
        i32.add
        local.get 11
        i32.const 80
        i32.mul
        i32.add
        local.get 4
        i32.load offset=8172
        local.get 4
        i32.load offset=160
        i32.add
        i32.load8_u
        i32.store8 offset=75
        local.get 4
        i32.load offset=8172
        i32.const 1
        i32.add
        local.set 49
        block  ;; label = @3
          local.get 49
          i32.eqz
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 4
          i32.const 148
          i32.add
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 4
        local.get 49
        i32.store offset=8172
        local.get 4
        local.get 11
        i32.const 1
        i32.add
        i32.store offset=8176
        br 0 (;@2;)
      end
    end
    local.get 4
    i32.const 172
    i32.add
    local.set 50
    local.get 8
    i32.const 127
    i32.and
    local.set 51
    local.get 51
    i32.const 0
    i32.sub
    local.set 52
    block  ;; label = @1
      block  ;; label = @2
        local.get 51
        i32.const 100
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 4
      i32.const 148
      i32.add
      local.get 51
      i32.const 100
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 4
    local.get 52
    i32.store offset=8216
    local.get 4
    local.get 50
    i32.store offset=8212
    local.get 4
    i32.load offset=8216
    local.set 53
    local.get 4
    i32.load offset=8212
    local.set 54
    local.get 4
    i32.load offset=164
    local.set 55
    local.get 3
    local.set 56
    local.get 4
    i32.const 148
    i32.add
    local.get 54
    local.get 53
    local.get 55
    local.get 56
    call $route.routeDeltaEncode
    local.set 57
    local.get 4
    local.get 57
    i32.store offset=8220
    block  ;; label = @1
      local.get 57
      i32.const 2147483647
      i32.le_u
      i32.const 1
      i32.and
      br_if 0 (;@1;)
      local.get 4
      i32.const 148
      i32.add
      call $debug.FullPanic__function_'defaultPanic'__.integerOutOfBounds
      unreachable
    end
    local.get 4
    i32.const 8224
    i32.add
    global.set $m6__stack_pointer
    local.get 57
    return)
  (func $route.routeDeltaEncode (type $t_6_7) (param i32 i32 i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 96
    i32.sub
    local.set 5
    local.get 5
    global.set $m6__stack_pointer
    local.get 5
    local.get 2
    i32.store offset=4
    local.get 5
    local.get 1
    i32.store
    local.get 5
    local.get 4
    i32.store offset=12
    local.get 5
    local.get 3
    i32.store offset=8
    local.get 5
    local.get 2
    i32.store offset=20
    local.get 5
    local.get 1
    i32.store offset=16
    local.get 5
    local.get 4
    i32.store offset=28
    local.get 5
    local.get 3
    i32.store offset=24
    local.get 5
    i32.const 0
    i32.store offset=32
    local.get 5
    i32.load offset=20
    local.set 6
    local.get 5
    i32.load offset=28
    local.set 7
    local.get 7
    i32.const -2
    i32.add
    local.set 8
    block  ;; label = @1
      local.get 8
      local.get 7
      i32.gt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 8
    i32.const 72
    i32.div_u
    local.set 9
    local.get 6
    local.get 9
    local.get 6
    local.get 9
    i32.lt_u
    select
    local.set 10
    block  ;; label = @1
      local.get 10
      i32.const 65535
      i32.le_u
      i32.const 1
      i32.and
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.integerOutOfBounds
      unreachable
    end
    local.get 10
    local.set 11
    local.get 5
    local.get 10
    i32.store16 offset=38
    local.get 5
    i32.load offset=32
    local.set 12
    local.get 5
    i32.load offset=28
    local.set 13
    local.get 12
    local.get 5
    i32.load offset=24
    i32.add
    local.set 14
    local.get 12
    i32.const 2
    i32.add
    local.set 15
    block  ;; label = @1
      block  ;; label = @2
        local.get 15
        local.get 13
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 15
      local.get 13
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 5
    local.get 14
    i32.store offset=40
    local.get 5
    local.get 11
    i32.store16 offset=46
    local.get 5
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=49
    i32.const 8
    local.set 16
    local.get 5
    local.get 11
    local.get 16
    i32.shl
    local.get 11
    i32.const 65280
    i32.and
    local.get 16
    i32.shr_u
    i32.or
    i32.store16 offset=50
    local.get 14
    local.get 5
    i32.load16_u offset=50
    i32.store16 align=1
    local.get 5
    i32.load offset=32
    local.set 17
    local.get 17
    i32.const 2
    i32.add
    local.set 18
    block  ;; label = @1
      local.get 18
      local.get 17
      i32.lt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
      unreachable
    end
    local.get 5
    local.get 18
    i32.store offset=32
    local.get 5
    i32.const 0
    i32.store offset=52
    local.get 11
    i32.const 65535
    i32.and
    local.set 19
    block  ;; label = @1
      loop  ;; label = @2
        local.get 5
        i32.load offset=52
        local.set 20
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              block  ;; label = @6
                local.get 20
                local.get 19
                i32.lt_u
                i32.const 1
                i32.and
                i32.eqz
                br_if 0 (;@6;)
                local.get 5
                local.get 20
                i32.store offset=56
                local.get 5
                i32.load offset=20
                local.set 21
                local.get 5
                i32.load offset=16
                local.set 22
                local.get 20
                local.get 21
                i32.lt_u
                i32.const 1
                i32.and
                br_if 1 (;@5;)
                br 2 (;@4;)
              end
              br 4 (;@1;)
            end
            br 1 (;@3;)
          end
          local.get 0
          local.get 20
          local.get 21
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 22
        local.get 20
        i32.const 80
        i32.mul
        i32.add
        local.set 23
        local.get 5
        local.get 23
        i32.store offset=60
        local.get 5
        local.get 23
        i32.store offset=64
        local.get 5
        i32.load offset=32
        local.set 24
        local.get 5
        i32.load offset=28
        local.set 25
        local.get 24
        local.get 5
        i32.load offset=24
        i32.add
        local.set 26
        local.get 24
        i32.const 32
        i32.add
        local.set 27
        block  ;; label = @3
          block  ;; label = @4
            local.get 27
            local.get 25
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 27
          local.get 25
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 5
        i32.load offset=60
        i32.const 10
        i32.add
        local.set 28
        local.get 28
        i32.const 32
        i32.add
        local.set 29
        local.get 26
        i32.const 32
        i32.add
        local.set 30
        block  ;; label = @3
          block  ;; label = @4
            local.get 26
            local.get 29
            i32.ge_u
            local.get 28
            local.get 30
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        local.get 26
        local.get 28
        i64.load align=1
        i64.store align=1
        i32.const 24
        local.set 31
        local.get 26
        local.get 31
        i32.add
        local.get 28
        local.get 31
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 16
        local.set 32
        local.get 26
        local.get 32
        i32.add
        local.get 28
        local.get 32
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 8
        local.set 33
        local.get 26
        local.get 33
        i32.add
        local.get 28
        local.get 33
        i32.add
        i64.load align=1
        i64.store align=1
        local.get 5
        i32.load offset=32
        local.set 34
        local.get 34
        i32.const 32
        i32.add
        local.set 35
        block  ;; label = @3
          local.get 35
          local.get 34
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 35
        i32.store offset=32
        local.get 5
        i32.load offset=32
        local.set 36
        local.get 5
        i32.load offset=28
        local.set 37
        local.get 36
        local.get 5
        i32.load offset=24
        i32.add
        local.set 38
        local.get 36
        i32.const 32
        i32.add
        local.set 39
        block  ;; label = @3
          block  ;; label = @4
            local.get 39
            local.get 37
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 39
          local.get 37
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 5
        i32.load offset=60
        i32.const 42
        i32.add
        local.set 40
        local.get 40
        i32.const 32
        i32.add
        local.set 41
        local.get 38
        i32.const 32
        i32.add
        local.set 42
        block  ;; label = @3
          block  ;; label = @4
            local.get 38
            local.get 41
            i32.ge_u
            local.get 40
            local.get 42
            i32.ge_u
            i32.or
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
          unreachable
        end
        local.get 38
        local.get 40
        i64.load align=1
        i64.store align=1
        i32.const 24
        local.set 43
        local.get 38
        local.get 43
        i32.add
        local.get 40
        local.get 43
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 16
        local.set 44
        local.get 38
        local.get 44
        i32.add
        local.get 40
        local.get 44
        i32.add
        i64.load align=1
        i64.store align=1
        i32.const 8
        local.set 45
        local.get 38
        local.get 45
        i32.add
        local.get 40
        local.get 45
        i32.add
        i64.load align=1
        i64.store align=1
        local.get 5
        i32.load offset=32
        local.set 46
        local.get 46
        i32.const 32
        i32.add
        local.set 47
        block  ;; label = @3
          local.get 47
          local.get 46
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 47
        i32.store offset=32
        local.get 5
        i32.load offset=32
        local.set 48
        local.get 5
        i32.load offset=28
        local.set 49
        local.get 48
        local.get 5
        i32.load offset=24
        i32.add
        local.set 50
        local.get 48
        i32.const 2
        i32.add
        local.set 51
        block  ;; label = @3
          block  ;; label = @4
            local.get 51
            local.get 49
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 51
          local.get 49
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 5
        i32.load offset=60
        i32.load16_u offset=8
        local.set 52
        local.get 5
        local.get 50
        i32.store offset=68
        local.get 5
        local.get 52
        i32.store16 offset=74
        local.get 5
        i32.const 0
        i32.const 1
        i32.and
        i32.store8 offset=77
        i32.const 8
        local.set 53
        local.get 5
        local.get 52
        local.get 53
        i32.shl
        local.get 52
        i32.const 65280
        i32.and
        local.get 53
        i32.shr_u
        i32.or
        i32.store16 offset=78
        local.get 50
        local.get 5
        i32.load16_u offset=78
        i32.store16 align=1
        local.get 5
        i32.load offset=32
        local.set 54
        local.get 54
        i32.const 2
        i32.add
        local.set 55
        block  ;; label = @3
          local.get 55
          local.get 54
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 55
        i32.store offset=32
        local.get 5
        i32.load offset=32
        local.set 56
        local.get 5
        i32.load offset=28
        local.set 57
        local.get 56
        local.get 5
        i32.load offset=24
        i32.add
        local.set 58
        local.get 56
        i32.const 4
        i32.add
        local.set 59
        block  ;; label = @3
          block  ;; label = @4
            local.get 59
            local.get 57
            i32.le_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 59
          local.get 57
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 5
        i32.load offset=60
        i32.load
        local.set 60
        local.get 5
        local.get 58
        i32.store offset=80
        local.get 5
        local.get 60
        i32.store offset=84
        local.get 5
        i32.const 0
        i32.const 1
        i32.and
        i32.store8 offset=91
        i32.const 24
        local.set 61
        local.get 60
        local.get 61
        i32.shr_u
        local.set 62
        i32.const 8
        local.set 63
        local.get 60
        local.get 63
        i32.shr_u
        local.set 64
        i32.const 65280
        local.set 65
        local.get 5
        local.get 62
        local.get 64
        local.get 65
        i32.and
        i32.or
        local.get 60
        local.get 61
        i32.shl
        local.get 60
        local.get 65
        i32.and
        local.get 63
        i32.shl
        i32.or
        i32.or
        i32.store offset=92
        local.get 58
        local.get 5
        i32.load offset=92
        i32.store align=1
        local.get 5
        i32.load offset=32
        local.set 66
        local.get 66
        i32.const 4
        i32.add
        local.set 67
        block  ;; label = @3
          local.get 67
          local.get 66
          i32.lt_u
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 67
        i32.store offset=32
        local.get 5
        i32.load offset=32
        local.set 68
        local.get 5
        i32.load offset=28
        local.set 69
        local.get 5
        i32.load offset=24
        local.set 70
        block  ;; label = @3
          block  ;; label = @4
            local.get 68
            local.get 69
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 68
          local.get 69
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 70
        local.get 68
        i32.add
        local.get 5
        i32.load offset=60
        i32.load8_u offset=74
        i32.store8
        local.get 5
        i32.load offset=32
        i32.const 1
        i32.add
        local.set 71
        block  ;; label = @3
          local.get 71
          i32.eqz
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 71
        i32.store offset=32
        local.get 5
        i32.load offset=32
        local.set 72
        local.get 5
        i32.load offset=28
        local.set 73
        local.get 5
        i32.load offset=24
        local.set 74
        block  ;; label = @3
          block  ;; label = @4
            local.get 72
            local.get 73
            i32.lt_u
            i32.const 1
            i32.and
            i32.eqz
            br_if 0 (;@4;)
            br 1 (;@3;)
          end
          local.get 0
          local.get 72
          local.get 73
          call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
          unreachable
        end
        local.get 74
        local.get 72
        i32.add
        local.get 5
        i32.load offset=60
        i32.load8_u offset=75
        i32.store8
        local.get 5
        i32.load offset=32
        i32.const 1
        i32.add
        local.set 75
        block  ;; label = @3
          local.get 75
          i32.eqz
          i32.const 1
          i32.and
          i32.eqz
          br_if 0 (;@3;)
          local.get 0
          call $debug.FullPanic__function_'defaultPanic'__.integerOverflow
          unreachable
        end
        local.get 5
        local.get 75
        i32.store offset=32
        local.get 5
        local.get 20
        i32.const 1
        i32.add
        i32.store offset=52
        br 0 (;@2;)
      end
    end
    local.get 5
    i32.load offset=32
    local.set 76
    local.get 5
    i32.const 96
    i32.add
    global.set $m6__stack_pointer
    local.get 76
    return)
  (func $root.beacon_encode (type $t_6_0) (param i32 i32 i32 i32 i32 i32)
    (local i32 i32 i32 i32 i64 i32 i32 i64 i32 i32 i64 i32 i32 i64 i32 i64 i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 416
    i32.sub
    local.set 6
    local.get 6
    global.set $m6__stack_pointer
    local.get 6
    local.get 0
    i32.store offset=8
    local.get 6
    local.get 1
    i32.store offset=12
    local.get 6
    local.get 2
    i32.store offset=16
    local.get 6
    local.get 3
    i32.store8 offset=22
    local.get 6
    local.get 4
    i32.store8 offset=23
    local.get 6
    local.get 5
    i32.store offset=24
    local.get 6
    i32.const 32
    i32.store offset=164
    local.get 6
    local.get 6
    i32.const 28
    i32.add
    i32.store offset=160
    i32.const 0
    local.set 7
    local.get 6
    local.get 7
    i32.store offset=156
    local.get 6
    local.get 1
    i32.store offset=168
    local.get 6
    local.get 2
    i32.store offset=172
    local.get 6
    local.get 5
    i32.store offset=176
    local.get 6
    i32.load offset=168
    local.set 8
    i32.const 24
    local.set 9
    local.get 8
    local.get 9
    i32.add
    i64.load align=1
    local.set 10
    local.get 9
    local.get 6
    i32.const 264
    i32.add
    i32.add
    local.set 11
    local.get 11
    local.get 10
    i64.store
    i32.const 16
    local.set 12
    local.get 8
    local.get 12
    i32.add
    i64.load align=1
    local.set 13
    local.get 12
    local.get 6
    i32.const 264
    i32.add
    i32.add
    local.set 14
    local.get 14
    local.get 13
    i64.store
    i32.const 8
    local.set 15
    local.get 8
    local.get 15
    i32.add
    i64.load align=1
    local.set 16
    local.get 15
    local.get 6
    i32.const 264
    i32.add
    i32.add
    local.set 17
    local.get 17
    local.get 16
    i64.store
    local.get 6
    local.get 8
    i64.load align=1
    i64.store offset=264
    local.get 6
    i32.load offset=172
    local.set 18
    local.get 18
    local.get 9
    i32.add
    i64.load align=1
    local.set 19
    local.get 9
    local.get 6
    i32.const 296
    i32.add
    i32.add
    local.set 20
    local.get 20
    local.get 19
    i64.store
    local.get 18
    local.get 12
    i32.add
    i64.load align=1
    local.set 21
    local.get 12
    local.get 6
    i32.const 296
    i32.add
    i32.add
    local.set 22
    local.get 22
    local.get 21
    i64.store
    local.get 18
    local.get 15
    i32.add
    i64.load align=1
    local.set 23
    local.get 15
    local.get 6
    i32.const 296
    i32.add
    i32.add
    local.set 24
    local.get 24
    local.get 23
    i64.store
    local.get 6
    local.get 18
    i64.load align=1
    i64.store offset=296
    local.get 6
    i32.const 156
    i32.add
    local.get 3
    call $beacon.BeaconFlags.fromByte
    local.set 25
    local.get 6
    i32.const 1
    i32.store8 offset=344
    local.get 6
    local.get 0
    i32.store offset=332
    local.get 6
    i32.const 369
    i32.add
    local.get 11
    i64.load
    i64.store align=1
    local.get 6
    i32.const 361
    i32.add
    local.get 14
    i64.load
    i64.store align=1
    local.get 6
    i32.const 353
    i32.add
    local.get 17
    i64.load
    i64.store align=1
    local.get 6
    local.get 6
    i64.load offset=264
    i64.store offset=345 align=1
    local.get 6
    i32.const 401
    i32.add
    local.get 20
    i64.load
    i64.store align=1
    local.get 6
    i32.const 393
    i32.add
    local.get 22
    i64.load
    i64.store align=1
    local.get 6
    i32.const 385
    i32.add
    local.get 24
    i64.load
    i64.store align=1
    local.get 6
    local.get 6
    i64.load offset=296
    i64.store offset=377 align=1
    local.get 6
    local.get 7
    i32.store offset=336
    local.get 6
    local.get 7
    i32.store offset=409 align=1
    local.get 6
    i32.const 255
    i32.store8 offset=413
    local.get 6
    local.get 7
    i32.store offset=340
    local.get 6
    local.get 25
    i32.store8 offset=414
    local.get 6
    local.get 4
    i32.store8 offset=415
    i32.const 84
    local.set 26
    block  ;; label = @1
      local.get 26
      i32.eqz
      br_if 0 (;@1;)
      local.get 6
      i32.const 180
      i32.add
      local.get 6
      i32.const 332
      i32.add
      local.get 26
      memory.copy
    end
    local.get 6
    i32.load offset=176
    local.set 27
    i32.const 128
    local.set 28
    i32.const 1049936
    local.set 29
    local.get 29
    local.set 30
    i32.const 6
    local.set 31
    local.get 31
    local.set 32
    local.get 6
    i32.const 156
    i32.add
    local.get 6
    i32.const 180
    i32.add
    local.get 27
    local.get 28
    local.get 30
    local.get 31
    local.get 29
    local.get 32
    call $beacon.Beacon.ethernetEncode
    local.get 6
    i32.const 416
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $beacon.BeaconFlags.fromByte (type $t_6_11) (param i32 i32) (result i32)
    (local i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i32.store8 offset=14
    local.get 2
    local.get 1
    i32.const 1
    i32.and
    local.get 2
    i32.load8_u offset=15
    i32.const 254
    i32.and
    i32.or
    i32.store8 offset=15
    local.get 2
    local.get 1
    i32.const 2
    i32.and
    local.get 2
    i32.load8_u offset=15
    i32.const 253
    i32.and
    i32.or
    i32.store8 offset=15
    local.get 2
    local.get 1
    i32.const 4
    i32.and
    local.get 2
    i32.load8_u offset=15
    i32.const 251
    i32.and
    i32.or
    i32.store8 offset=15
    local.get 2
    local.get 1
    i32.const 8
    i32.and
    local.get 2
    i32.load8_u offset=15
    i32.const 247
    i32.and
    i32.or
    i32.store8 offset=15
    local.get 2
    local.get 2
    i32.load8_u offset=15
    i32.const 15
    i32.and
    i32.store8 offset=15
    local.get 2
    i32.load8_u offset=15
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $beacon.Beacon.ethernetEncode (type $t_6_14) (param i32 i32 i32 i32 i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 160
    i32.sub
    local.set 8
    local.get 8
    global.set $m6__stack_pointer
    local.get 8
    local.get 1
    i32.store
    local.get 8
    local.get 3
    i32.store offset=8
    local.get 8
    local.get 2
    i32.store offset=4
    local.get 8
    local.get 5
    i32.store offset=16
    local.get 8
    local.get 4
    i32.store offset=12
    local.get 8
    local.get 7
    i32.store offset=24
    local.get 8
    local.get 6
    i32.store offset=20
    local.get 8
    local.get 1
    i32.store offset=28
    local.get 8
    local.get 3
    i32.store offset=36
    local.get 8
    local.get 2
    i32.store offset=32
    local.get 8
    local.get 5
    i32.store offset=44
    local.get 8
    local.get 4
    i32.store offset=40
    local.get 8
    local.get 7
    i32.store offset=52
    local.get 8
    local.get 6
    i32.store offset=48
    local.get 8
    i32.load offset=44
    local.set 9
    local.get 8
    i32.load offset=40
    local.set 10
    block  ;; label = @1
      block  ;; label = @2
        i32.const 6
        local.get 9
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 6
      local.get 9
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    i32.const 4
    local.set 11
    local.get 10
    local.get 11
    i32.add
    i32.load16_u align=1
    local.set 12
    local.get 11
    local.get 8
    i32.const 56
    i32.add
    i32.add
    local.get 12
    i32.store16
    local.get 8
    local.get 10
    i32.load align=1
    i32.store offset=56
    i32.const 4
    local.set 13
    local.get 13
    local.get 8
    i32.const 64
    i32.add
    i32.add
    local.get 13
    local.get 8
    i32.const 56
    i32.add
    i32.add
    i32.load16_u align=1
    i32.store16
    local.get 8
    local.get 8
    i32.load offset=56 align=1
    i32.store offset=64
    block  ;; label = @1
      block  ;; label = @2
        block  ;; label = @3
          block  ;; label = @4
            block  ;; label = @5
              local.get 8
              i32.load offset=52
              i32.const 6
              i32.ge_u
              i32.const 1
              i32.and
              i32.eqz
              br_if 0 (;@5;)
              local.get 8
              i32.load offset=52
              local.set 14
              local.get 8
              i32.load offset=48
              local.set 15
              i32.const 6
              local.get 14
              i32.le_u
              i32.const 1
              i32.and
              br_if 1 (;@4;)
              br 2 (;@3;)
            end
            i32.const 1049936
            local.set 16
            br 3 (;@1;)
          end
          br 1 (;@2;)
        end
        local.get 0
        i32.const 6
        local.get 14
        call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
        unreachable
      end
      i32.const 4
      local.set 17
      local.get 15
      local.get 17
      i32.add
      i32.load16_u align=1
      local.set 18
      local.get 17
      local.get 8
      i32.const 72
      i32.add
      i32.add
      local.get 18
      i32.store16
      local.get 8
      local.get 15
      i32.load align=1
      i32.store offset=72
      local.get 8
      i32.const 72
      i32.add
      local.set 16
    end
    local.get 16
    local.set 19
    i32.const 4
    local.set 20
    local.get 19
    local.get 20
    i32.add
    i32.load16_u align=1
    local.set 21
    local.get 20
    local.get 8
    i32.const 80
    i32.add
    i32.add
    local.get 21
    i32.store16
    local.get 8
    local.get 19
    i32.load align=1
    i32.store offset=80
    local.get 8
    i32.load offset=36
    local.set 22
    local.get 8
    i32.load offset=32
    local.set 23
    block  ;; label = @1
      block  ;; label = @2
        i32.const 6
        local.get 22
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 6
      local.get 22
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 8
    i32.const 80
    i32.add
    i32.const 6
    i32.add
    local.set 24
    local.get 23
    i32.const 6
    i32.add
    local.set 25
    block  ;; label = @1
      block  ;; label = @2
        local.get 23
        local.get 24
        i32.ge_u
        local.get 8
        i32.const 80
        i32.add
        local.get 25
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    local.get 23
    local.get 8
    i32.load offset=80
    i32.store align=1
    i32.const 4
    local.set 26
    local.get 23
    local.get 26
    i32.add
    local.get 26
    local.get 8
    i32.const 80
    i32.add
    i32.add
    i32.load16_u
    i32.store16 align=1
    local.get 8
    i32.load offset=36
    local.set 27
    local.get 8
    i32.load offset=32
    i32.const 6
    i32.add
    local.set 28
    block  ;; label = @1
      block  ;; label = @2
        i32.const 12
        local.get 27
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 12
      local.get 27
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 8
    i32.const 64
    i32.add
    i32.const 6
    i32.add
    local.set 29
    local.get 28
    i32.const 6
    i32.add
    local.set 30
    block  ;; label = @1
      block  ;; label = @2
        local.get 28
        local.get 29
        i32.ge_u
        local.get 8
        i32.const 64
        i32.add
        local.get 30
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    local.get 28
    local.get 8
    i32.load offset=64
    i32.store align=1
    i32.const 4
    local.set 31
    local.get 28
    local.get 31
    i32.add
    local.get 31
    local.get 8
    i32.const 64
    i32.add
    i32.add
    i32.load16_u
    i32.store16 align=1
    local.get 8
    i32.load offset=36
    local.set 32
    local.get 8
    i32.load offset=32
    i32.const 12
    i32.add
    local.set 33
    block  ;; label = @1
      block  ;; label = @2
        i32.const 14
        local.get 32
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 14
      local.get 32
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 8
    local.get 33
    i32.store offset=88
    local.get 8
    i32.const 2845
    i32.store16 offset=92
    local.get 8
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=95
    local.get 33
    i32.const 0
    i32.load16_u offset=1049942 align=1
    i32.store16 align=1
    local.get 8
    i32.load offset=36
    local.set 34
    local.get 8
    i32.load offset=32
    i32.const 14
    i32.add
    local.set 35
    block  ;; label = @1
      block  ;; label = @2
        i32.const 14
        local.get 34
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 14
      local.get 34
      call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
      unreachable
    end
    local.get 34
    i32.const 14
    i32.sub
    local.set 36
    block  ;; label = @1
      block  ;; label = @2
        local.get 34
        local.get 34
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 34
      local.get 34
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 8
    local.get 36
    i32.store offset=100
    local.get 8
    local.get 35
    i32.store offset=96
    local.get 8
    local.get 36
    i32.store offset=108
    local.get 8
    local.get 35
    i32.store offset=104
    local.get 8
    i32.load offset=100
    local.set 37
    local.get 8
    i32.load offset=96
    local.set 38
    block  ;; label = @1
      block  ;; label = @2
        i32.const 0
        local.get 37
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 0
      local.get 37
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 38
    local.get 8
    i32.load offset=28
    i32.load8_u offset=12
    i32.store8
    local.get 8
    i32.load offset=100
    local.set 39
    local.get 8
    i32.load offset=96
    i32.const 1
    i32.add
    local.set 40
    block  ;; label = @1
      block  ;; label = @2
        i32.const 5
        local.get 39
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 5
      local.get 39
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 8
    i32.load offset=28
    i32.load
    local.set 41
    local.get 8
    local.get 40
    i32.store offset=112
    local.get 8
    local.get 41
    i32.store offset=116
    local.get 8
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=123
    i32.const 24
    local.set 42
    local.get 41
    local.get 42
    i32.shr_u
    local.set 43
    i32.const 8
    local.set 44
    local.get 41
    local.get 44
    i32.shr_u
    local.set 45
    i32.const 65280
    local.set 46
    local.get 8
    local.get 43
    local.get 45
    local.get 46
    i32.and
    i32.or
    local.get 41
    local.get 42
    i32.shl
    local.get 41
    local.get 46
    i32.and
    local.get 44
    i32.shl
    i32.or
    i32.or
    i32.store offset=124
    local.get 40
    local.get 8
    i32.load offset=124
    i32.store align=1
    local.get 8
    i32.load offset=100
    local.set 47
    local.get 8
    i32.load offset=96
    i32.const 5
    i32.add
    local.set 48
    block  ;; label = @1
      block  ;; label = @2
        i32.const 37
        local.get 47
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 37
      local.get 47
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 8
    i32.load offset=28
    i32.const 13
    i32.add
    local.set 49
    local.get 49
    i32.const 32
    i32.add
    local.set 50
    local.get 48
    i32.const 32
    i32.add
    local.set 51
    block  ;; label = @1
      block  ;; label = @2
        local.get 48
        local.get 50
        i32.ge_u
        local.get 49
        local.get 51
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    local.get 48
    local.get 49
    i64.load align=1
    i64.store align=1
    i32.const 24
    local.set 52
    local.get 48
    local.get 52
    i32.add
    local.get 49
    local.get 52
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 16
    local.set 53
    local.get 48
    local.get 53
    i32.add
    local.get 49
    local.get 53
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 8
    local.set 54
    local.get 48
    local.get 54
    i32.add
    local.get 49
    local.get 54
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 8
    i32.load offset=100
    local.set 55
    local.get 8
    i32.load offset=96
    i32.const 37
    i32.add
    local.set 56
    block  ;; label = @1
      block  ;; label = @2
        i32.const 69
        local.get 55
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 69
      local.get 55
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 8
    i32.load offset=28
    i32.const 45
    i32.add
    local.set 57
    local.get 57
    i32.const 32
    i32.add
    local.set 58
    local.get 56
    i32.const 32
    i32.add
    local.set 59
    block  ;; label = @1
      block  ;; label = @2
        local.get 56
        local.get 58
        i32.ge_u
        local.get 57
        local.get 59
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    local.get 56
    local.get 57
    i64.load align=1
    i64.store align=1
    i32.const 24
    local.set 60
    local.get 56
    local.get 60
    i32.add
    local.get 57
    local.get 60
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 16
    local.set 61
    local.get 56
    local.get 61
    i32.add
    local.get 57
    local.get 61
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 8
    local.set 62
    local.get 56
    local.get 62
    i32.add
    local.get 57
    local.get 62
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 8
    i32.load offset=100
    local.set 63
    local.get 8
    i32.load offset=96
    i32.const 69
    i32.add
    local.set 64
    block  ;; label = @1
      block  ;; label = @2
        i32.const 73
        local.get 63
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 73
      local.get 63
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 8
    i32.load offset=28
    i32.load offset=4
    local.set 65
    local.get 8
    local.get 64
    i32.store offset=128
    local.get 8
    local.get 65
    i32.store offset=132
    local.get 8
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=139
    i32.const 24
    local.set 66
    local.get 65
    local.get 66
    i32.shr_u
    local.set 67
    i32.const 8
    local.set 68
    local.get 65
    local.get 68
    i32.shr_u
    local.set 69
    i32.const 65280
    local.set 70
    local.get 8
    local.get 67
    local.get 69
    local.get 70
    i32.and
    i32.or
    local.get 65
    local.get 66
    i32.shl
    local.get 65
    local.get 70
    i32.and
    local.get 68
    i32.shl
    i32.or
    i32.or
    i32.store offset=140
    local.get 64
    local.get 8
    i32.load offset=140
    i32.store align=1
    local.get 8
    i32.load offset=100
    local.set 71
    local.get 8
    i32.load offset=96
    i32.const 73
    i32.add
    local.set 72
    block  ;; label = @1
      block  ;; label = @2
        i32.const 77
        local.get 71
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 77
      local.get 71
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 8
    i32.load offset=28
    i32.const 77
    i32.add
    local.set 73
    local.get 73
    i32.const 4
    i32.add
    local.set 74
    local.get 72
    i32.const 4
    i32.add
    local.set 75
    block  ;; label = @1
      block  ;; label = @2
        local.get 72
        local.get 74
        i32.ge_u
        local.get 73
        local.get 75
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    local.get 72
    local.get 73
    i32.load align=1
    i32.store align=1
    local.get 8
    i32.load offset=100
    local.set 76
    local.get 8
    i32.load offset=96
    local.set 77
    block  ;; label = @1
      block  ;; label = @2
        i32.const 77
        local.get 76
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 77
      local.get 76
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 77
    local.get 8
    i32.load offset=28
    i32.load8_u offset=81
    i32.store8 offset=77
    local.get 8
    i32.load offset=100
    local.set 78
    local.get 8
    i32.load offset=96
    i32.const 78
    i32.add
    local.set 79
    block  ;; label = @1
      block  ;; label = @2
        i32.const 82
        local.get 78
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 82
      local.get 78
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 8
    i32.load offset=28
    i32.load offset=8
    local.set 80
    local.get 8
    local.get 79
    i32.store offset=144
    local.get 8
    local.get 80
    i32.store offset=148
    local.get 8
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=155
    i32.const 24
    local.set 81
    local.get 80
    local.get 81
    i32.shr_u
    local.set 82
    i32.const 8
    local.set 83
    local.get 80
    local.get 83
    i32.shr_u
    local.set 84
    i32.const 65280
    local.set 85
    local.get 8
    local.get 82
    local.get 84
    local.get 85
    i32.and
    i32.or
    local.get 80
    local.get 81
    i32.shl
    local.get 80
    local.get 85
    i32.and
    local.get 83
    i32.shl
    i32.or
    i32.or
    i32.store offset=156
    local.get 79
    local.get 8
    i32.load offset=156
    i32.store align=1
    local.get 8
    i32.load offset=100
    local.set 86
    local.get 8
    i32.load offset=96
    local.set 87
    block  ;; label = @1
      block  ;; label = @2
        i32.const 82
        local.get 86
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 82
      local.get 86
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 87
    local.get 0
    local.get 8
    i32.load offset=28
    i32.load8_u offset=82
    call $beacon.BeaconFlags.toByte
    i32.store8 offset=82
    local.get 8
    i32.load offset=100
    local.set 88
    local.get 8
    i32.load offset=96
    local.set 89
    block  ;; label = @1
      block  ;; label = @2
        i32.const 83
        local.get 88
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 83
      local.get 88
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 89
    local.get 8
    i32.load offset=28
    i32.load8_u offset=83
    i32.store8 offset=83
    local.get 8
    i32.load offset=100
    local.set 90
    local.get 8
    i32.load offset=96
    i32.const 84
    i32.add
    local.set 91
    block  ;; label = @1
      block  ;; label = @2
        i32.const 84
        local.get 90
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 84
      local.get 90
      call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
      unreachable
    end
    local.get 90
    i32.const 84
    i32.sub
    local.set 92
    block  ;; label = @1
      block  ;; label = @2
        local.get 90
        local.get 90
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 90
      local.get 90
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 92
    local.set 93
    local.get 91
    local.set 94
    i32.const 0
    local.set 95
    block  ;; label = @1
      local.get 93
      i32.eqz
      br_if 0 (;@1;)
      local.get 94
      local.get 95
      local.get 93
      memory.fill
    end
    local.get 8
    i32.const 160
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $beacon.BeaconFlags.toByte (type $t_6_11) (param i32 i32) (result i32)
    (local i32 i32)
    global.get $m6__stack_pointer
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 1
    i32.store8 offset=14
    local.get 2
    local.get 1
    i32.store8 offset=15
    local.get 2
    i32.load8_u offset=15
    i32.const 15
    i32.and
    local.set 3
    local.get 2
    i32.const 16
    i32.add
    global.set $m6__stack_pointer
    local.get 3
    return)
  (func $root.beacon_decode (type $t_6_6) (param i32 i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 464
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 0
    i32.store
    local.get 3
    local.get 1
    i32.store offset=4
    local.get 3
    local.get 2
    i32.store offset=8
    local.get 3
    i32.const 32
    i32.store offset=148
    local.get 3
    local.get 3
    i32.const 12
    i32.add
    i32.store offset=144
    local.get 3
    i32.const 0
    i32.store offset=140
    local.get 3
    local.get 0
    i32.store offset=152
    local.get 3
    local.get 2
    i32.store offset=156
    local.get 3
    i32.load offset=152
    local.set 4
    local.get 1
    local.set 5
    local.get 3
    i32.const 160
    i32.add
    local.get 3
    i32.const 140
    i32.add
    local.get 4
    local.get 5
    call $beacon.Beacon.ethernetDecode
    local.get 3
    i32.load8_u offset=244
    local.set 6
    i32.const 0
    local.set 7
    block  ;; label = @1
      block  ;; label = @2
        local.get 6
        i32.const 255
        i32.and
        local.get 7
        i32.const 255
        i32.and
        i32.ne
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        i32.const 84
        local.set 8
        block  ;; label = @3
          local.get 8
          i32.eqz
          br_if 0 (;@3;)
          local.get 3
          i32.const 248
          i32.add
          local.get 3
          i32.const 160
          i32.add
          local.get 8
          memory.copy
        end
        local.get 3
        i32.const 248
        i32.add
        local.set 9
        br 1 (;@1;)
      end
      i32.const -1
      local.set 10
      local.get 3
      i32.const 464
      i32.add
      global.set $m6__stack_pointer
      local.get 10
      return
    end
    local.get 9
    local.set 11
    i32.const 84
    local.set 12
    block  ;; label = @1
      local.get 12
      i32.eqz
      br_if 0 (;@1;)
      local.get 3
      i32.const 332
      i32.add
      local.get 11
      local.get 12
      memory.copy
    end
    local.get 3
    i32.load offset=156
    local.set 13
    local.get 3
    i32.load offset=332
    local.set 14
    local.get 3
    local.get 13
    i32.store offset=416
    local.get 3
    local.get 14
    i32.store offset=420
    local.get 3
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=427
    i32.const 24
    local.set 15
    local.get 14
    local.get 15
    i32.shr_u
    local.set 16
    i32.const 8
    local.set 17
    local.get 14
    local.get 17
    i32.shr_u
    local.set 18
    i32.const 65280
    local.set 19
    local.get 3
    local.get 16
    local.get 18
    local.get 19
    i32.and
    i32.or
    local.get 14
    local.get 15
    i32.shl
    local.get 14
    local.get 19
    i32.and
    local.get 17
    i32.shl
    i32.or
    i32.or
    i32.store offset=428
    local.get 13
    local.get 3
    i32.load offset=428
    i32.store align=1
    local.get 3
    i32.load offset=156
    i32.const 4
    i32.add
    local.set 20
    local.get 3
    i32.const 332
    i32.add
    i32.const 13
    i32.add
    local.set 21
    local.get 21
    i32.const 32
    i32.add
    local.set 22
    local.get 20
    i32.const 32
    i32.add
    local.set 23
    block  ;; label = @1
      block  ;; label = @2
        local.get 20
        local.get 22
        i32.ge_u
        local.get 21
        local.get 23
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 3
      i32.const 140
      i32.add
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    local.get 20
    local.get 21
    i64.load align=1
    i64.store align=1
    i32.const 24
    local.set 24
    local.get 20
    local.get 24
    i32.add
    local.get 21
    local.get 24
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 16
    local.set 25
    local.get 20
    local.get 25
    i32.add
    local.get 21
    local.get 25
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 8
    local.set 26
    local.get 20
    local.get 26
    i32.add
    local.get 21
    local.get 26
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 3
    i32.load offset=156
    i32.const 36
    i32.add
    local.set 27
    local.get 3
    i32.const 332
    i32.add
    i32.const 45
    i32.add
    local.set 28
    local.get 28
    i32.const 32
    i32.add
    local.set 29
    local.get 27
    i32.const 32
    i32.add
    local.set 30
    block  ;; label = @1
      block  ;; label = @2
        local.get 27
        local.get 29
        i32.ge_u
        local.get 28
        local.get 30
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 3
      i32.const 140
      i32.add
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    local.get 27
    local.get 28
    i64.load align=1
    i64.store align=1
    i32.const 24
    local.set 31
    local.get 27
    local.get 31
    i32.add
    local.get 28
    local.get 31
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 16
    local.set 32
    local.get 27
    local.get 32
    i32.add
    local.get 28
    local.get 32
    i32.add
    i64.load align=1
    i64.store align=1
    i32.const 8
    local.set 33
    local.get 27
    local.get 33
    i32.add
    local.get 28
    local.get 33
    i32.add
    i64.load align=1
    i64.store align=1
    local.get 3
    i32.load offset=156
    i32.const 68
    i32.add
    local.set 34
    local.get 3
    i32.load offset=336
    local.set 35
    local.get 3
    local.get 34
    i32.store offset=432
    local.get 3
    local.get 35
    i32.store offset=436
    local.get 3
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=443
    i32.const 24
    local.set 36
    local.get 35
    local.get 36
    i32.shr_u
    local.set 37
    i32.const 8
    local.set 38
    local.get 35
    local.get 38
    i32.shr_u
    local.set 39
    i32.const 65280
    local.set 40
    local.get 3
    local.get 37
    local.get 39
    local.get 40
    i32.and
    i32.or
    local.get 35
    local.get 36
    i32.shl
    local.get 35
    local.get 40
    i32.and
    local.get 38
    i32.shl
    i32.or
    i32.or
    i32.store offset=444
    local.get 34
    local.get 3
    i32.load offset=444
    i32.store align=1
    local.get 3
    i32.load offset=156
    i32.const 72
    i32.add
    local.set 41
    local.get 3
    i32.const 332
    i32.add
    i32.const 77
    i32.add
    local.set 42
    local.get 42
    i32.const 4
    i32.add
    local.set 43
    local.get 41
    i32.const 4
    i32.add
    local.set 44
    block  ;; label = @1
      block  ;; label = @2
        local.get 41
        local.get 43
        i32.ge_u
        local.get 42
        local.get 44
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 3
      i32.const 140
      i32.add
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    local.get 41
    local.get 42
    i32.load align=1
    i32.store align=1
    local.get 3
    i32.load offset=156
    local.get 3
    i32.load8_u offset=413
    i32.store8 offset=76
    local.get 3
    i32.load offset=156
    i32.const 77
    i32.add
    local.set 45
    local.get 3
    i32.load offset=340
    local.set 46
    local.get 3
    local.get 45
    i32.store offset=448
    local.get 3
    local.get 46
    i32.store offset=452
    local.get 3
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=459
    i32.const 24
    local.set 47
    local.get 46
    local.get 47
    i32.shr_u
    local.set 48
    i32.const 8
    local.set 49
    local.get 46
    local.get 49
    i32.shr_u
    local.set 50
    i32.const 65280
    local.set 51
    local.get 3
    local.get 48
    local.get 50
    local.get 51
    i32.and
    i32.or
    local.get 46
    local.get 47
    i32.shl
    local.get 46
    local.get 51
    i32.and
    local.get 49
    i32.shl
    i32.or
    i32.or
    i32.store offset=460
    local.get 45
    local.get 3
    i32.load offset=460
    i32.store align=1
    local.get 3
    i32.load offset=156
    local.set 52
    local.get 3
    i32.load8_u offset=414
    local.set 53
    local.get 52
    local.get 3
    i32.const 140
    i32.add
    local.get 53
    call $beacon.BeaconFlags.toByte
    i32.store8 offset=81
    local.get 3
    i32.load offset=156
    local.get 3
    i32.load8_u offset=415
    i32.store8 offset=82
    i32.const 0
    local.set 54
    local.get 3
    i32.const 464
    i32.add
    global.set $m6__stack_pointer
    local.get 54
    return)
  (func $beacon.Beacon.ethernetDecode (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i32 i64 i32 i64 i32 i32 i32 i64 i32 i64 i32 i64 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 336
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 3
    i32.store offset=8
    local.get 4
    local.get 2
    i32.store offset=4
    local.get 4
    local.get 3
    i32.store offset=16
    local.get 4
    local.get 2
    i32.store offset=12
    block  ;; label = @1
      local.get 4
      i32.load offset=16
      i32.const 128
      i32.lt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      i32.const 1049944
      local.set 5
      i32.const 88
      local.set 6
      block  ;; label = @2
        local.get 6
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        local.get 5
        local.get 6
        memory.copy
      end
      local.get 4
      i32.const 336
      i32.add
      global.set $m6__stack_pointer
      return
    end
    local.get 4
    i32.load offset=16
    local.set 7
    local.get 4
    i32.load offset=12
    i32.const 12
    i32.add
    local.set 8
    block  ;; label = @1
      block  ;; label = @2
        i32.const 14
        local.get 7
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 14
      local.get 7
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 4
    local.get 8
    i32.store offset=20
    local.get 4
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=25
    local.get 4
    local.get 8
    i32.load16_u align=1
    i32.store16 offset=26
    local.get 4
    i32.load16_u offset=26 align=1
    local.set 9
    local.get 4
    local.get 9
    i32.store16 offset=28
    i32.const 8
    local.set 10
    local.get 9
    local.get 10
    i32.shl
    local.get 9
    i32.const 65280
    i32.and
    local.get 10
    i32.shr_u
    i32.or
    local.set 11
    local.get 11
    local.set 12
    local.get 4
    local.get 12
    i32.store16 offset=30
    i32.const 2845
    local.set 13
    block  ;; label = @1
      local.get 12
      i32.const 65535
      i32.and
      local.get 13
      i32.const 65535
      i32.and
      i32.ne
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      i32.const 1049944
      local.set 14
      i32.const 88
      local.set 15
      block  ;; label = @2
        local.get 15
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        local.get 14
        local.get 15
        memory.copy
      end
      local.get 4
      i32.const 336
      i32.add
      global.set $m6__stack_pointer
      return
    end
    local.get 4
    i32.load offset=16
    local.set 16
    local.get 4
    i32.load offset=12
    i32.const 14
    i32.add
    local.set 17
    block  ;; label = @1
      block  ;; label = @2
        i32.const 14
        local.get 16
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 14
      local.get 16
      call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
      unreachable
    end
    local.get 16
    i32.const 14
    i32.sub
    local.set 18
    block  ;; label = @1
      block  ;; label = @2
        local.get 16
        local.get 16
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      local.get 16
      local.get 16
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 4
    local.get 18
    i32.store offset=36
    local.get 4
    local.get 17
    i32.store offset=32
    local.get 4
    local.get 18
    i32.store offset=44
    local.get 4
    local.get 17
    i32.store offset=40
    local.get 4
    i32.load offset=36
    local.set 19
    local.get 4
    i32.load offset=32
    local.set 20
    block  ;; label = @1
      block  ;; label = @2
        i32.const 0
        local.get 19
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 0
      local.get 19
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 20
    i32.load8_u
    local.set 21
    i32.const 1
    local.set 22
    block  ;; label = @1
      local.get 21
      i32.const 255
      i32.and
      local.get 22
      i32.const 255
      i32.and
      i32.ne
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      i32.const 1049944
      local.set 23
      i32.const 88
      local.set 24
      block  ;; label = @2
        local.get 24
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        local.get 23
        local.get 24
        memory.copy
      end
      local.get 4
      i32.const 336
      i32.add
      global.set $m6__stack_pointer
      return
    end
    local.get 4
    i32.load offset=36
    local.set 25
    local.get 4
    i32.load offset=32
    local.set 26
    block  ;; label = @1
      block  ;; label = @2
        i32.const 0
        local.get 25
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 0
      local.get 25
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 26
    i32.load8_u
    local.set 27
    local.get 4
    i32.load offset=36
    local.set 28
    local.get 4
    i32.load offset=32
    i32.const 1
    i32.add
    local.set 29
    block  ;; label = @1
      block  ;; label = @2
        i32.const 5
        local.get 28
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 5
      local.get 28
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 4
    local.get 29
    i32.store offset=48
    local.get 4
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=55
    local.get 4
    local.get 29
    i32.load align=1
    i32.store offset=56
    local.get 4
    i32.load offset=56 align=1
    local.set 30
    local.get 4
    local.get 30
    i32.store offset=60
    i32.const 24
    local.set 31
    local.get 30
    local.get 31
    i32.shr_u
    local.set 32
    i32.const 8
    local.set 33
    local.get 30
    local.get 33
    i32.shr_u
    local.set 34
    i32.const 65280
    local.set 35
    local.get 32
    local.get 34
    local.get 35
    i32.and
    i32.or
    local.get 30
    local.get 31
    i32.shl
    local.get 30
    local.get 35
    i32.and
    local.get 33
    i32.shl
    i32.or
    i32.or
    local.set 36
    local.get 36
    local.set 37
    local.get 4
    i32.load offset=36
    local.set 38
    local.get 4
    i32.load offset=32
    i32.const 5
    i32.add
    local.set 39
    block  ;; label = @1
      block  ;; label = @2
        i32.const 37
        local.get 38
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 37
      local.get 38
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    i32.const 24
    local.set 40
    local.get 39
    local.get 40
    i32.add
    i64.load align=1
    local.set 41
    local.get 40
    local.get 4
    i32.const 64
    i32.add
    i32.add
    local.get 41
    i64.store
    i32.const 16
    local.set 42
    local.get 39
    local.get 42
    i32.add
    i64.load align=1
    local.set 43
    local.get 42
    local.get 4
    i32.const 64
    i32.add
    i32.add
    local.get 43
    i64.store
    i32.const 8
    local.set 44
    local.get 39
    local.get 44
    i32.add
    i64.load align=1
    local.set 45
    local.get 44
    local.get 4
    i32.const 64
    i32.add
    i32.add
    local.get 45
    i64.store
    local.get 4
    local.get 39
    i64.load align=1
    i64.store offset=64
    local.get 4
    i32.load offset=36
    local.set 46
    local.get 4
    i32.load offset=32
    i32.const 37
    i32.add
    local.set 47
    block  ;; label = @1
      block  ;; label = @2
        i32.const 69
        local.get 46
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 69
      local.get 46
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    i32.const 24
    local.set 48
    local.get 47
    local.get 48
    i32.add
    i64.load align=1
    local.set 49
    local.get 48
    local.get 4
    i32.const 96
    i32.add
    i32.add
    local.get 49
    i64.store
    i32.const 16
    local.set 50
    local.get 47
    local.get 50
    i32.add
    i64.load align=1
    local.set 51
    local.get 50
    local.get 4
    i32.const 96
    i32.add
    i32.add
    local.get 51
    i64.store
    i32.const 8
    local.set 52
    local.get 47
    local.get 52
    i32.add
    i64.load align=1
    local.set 53
    local.get 52
    local.get 4
    i32.const 96
    i32.add
    i32.add
    local.get 53
    i64.store
    local.get 4
    local.get 47
    i64.load align=1
    i64.store offset=96
    local.get 4
    i32.load offset=36
    local.set 54
    local.get 4
    i32.load offset=32
    i32.const 69
    i32.add
    local.set 55
    block  ;; label = @1
      block  ;; label = @2
        i32.const 73
        local.get 54
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 73
      local.get 54
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 4
    local.get 55
    i32.store offset=128
    local.get 4
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=135
    local.get 4
    local.get 55
    i32.load align=1
    i32.store offset=136
    local.get 4
    i32.load offset=136 align=1
    local.set 56
    local.get 4
    local.get 56
    i32.store offset=140
    i32.const 24
    local.set 57
    local.get 56
    local.get 57
    i32.shr_u
    local.set 58
    i32.const 8
    local.set 59
    local.get 56
    local.get 59
    i32.shr_u
    local.set 60
    i32.const 65280
    local.set 61
    local.get 58
    local.get 60
    local.get 61
    i32.and
    i32.or
    local.get 56
    local.get 57
    i32.shl
    local.get 56
    local.get 61
    i32.and
    local.get 59
    i32.shl
    i32.or
    i32.or
    local.set 62
    local.get 62
    local.set 63
    local.get 4
    i32.load offset=36
    local.set 64
    local.get 4
    i32.load offset=32
    i32.const 73
    i32.add
    local.set 65
    block  ;; label = @1
      block  ;; label = @2
        i32.const 77
        local.get 64
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 77
      local.get 64
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 4
    local.get 65
    i32.load align=1
    i32.store offset=144
    local.get 4
    i32.load offset=36
    local.set 66
    local.get 4
    i32.load offset=32
    local.set 67
    block  ;; label = @1
      block  ;; label = @2
        i32.const 77
        local.get 66
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 77
      local.get 66
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 67
    i32.load8_u offset=77
    local.set 68
    local.get 4
    i32.load offset=36
    local.set 69
    local.get 4
    i32.load offset=32
    i32.const 78
    i32.add
    local.set 70
    block  ;; label = @1
      block  ;; label = @2
        i32.const 82
        local.get 69
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 82
      local.get 69
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 4
    local.get 70
    i32.store offset=148
    local.get 4
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=155
    local.get 4
    local.get 70
    i32.load align=1
    i32.store offset=156
    local.get 4
    i32.load offset=156 align=1
    local.set 71
    local.get 4
    local.get 71
    i32.store offset=160
    i32.const 24
    local.set 72
    local.get 71
    local.get 72
    i32.shr_u
    local.set 73
    i32.const 8
    local.set 74
    local.get 71
    local.get 74
    i32.shr_u
    local.set 75
    i32.const 65280
    local.set 76
    local.get 73
    local.get 75
    local.get 76
    i32.and
    i32.or
    local.get 71
    local.get 72
    i32.shl
    local.get 71
    local.get 76
    i32.and
    local.get 74
    i32.shl
    i32.or
    i32.or
    local.set 77
    local.get 77
    local.set 78
    local.get 4
    i32.load offset=36
    local.set 79
    local.get 4
    i32.load offset=32
    local.set 80
    block  ;; label = @1
      block  ;; label = @2
        i32.const 82
        local.get 79
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 82
      local.get 79
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 1
    local.get 80
    i32.load8_u offset=82
    call $beacon.BeaconFlags.fromByte
    local.set 81
    local.get 4
    i32.load offset=36
    local.set 82
    local.get 4
    i32.load offset=32
    local.set 83
    block  ;; label = @1
      block  ;; label = @2
        i32.const 83
        local.get 82
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 83
      local.get 82
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 83
    i32.load8_u offset=83
    local.set 84
    local.get 4
    local.get 27
    i32.store8 offset=176
    local.get 4
    local.get 37
    i32.store offset=164
    local.get 4
    i32.const 164
    i32.add
    i32.const 13
    i32.add
    local.set 85
    local.get 85
    local.get 4
    i64.load offset=64
    i64.store align=1
    i32.const 24
    local.set 86
    local.get 85
    local.get 86
    i32.add
    local.get 86
    local.get 4
    i32.const 64
    i32.add
    i32.add
    i64.load
    i64.store align=1
    i32.const 16
    local.set 87
    local.get 85
    local.get 87
    i32.add
    local.get 87
    local.get 4
    i32.const 64
    i32.add
    i32.add
    i64.load
    i64.store align=1
    i32.const 8
    local.set 88
    local.get 85
    local.get 88
    i32.add
    local.get 88
    local.get 4
    i32.const 64
    i32.add
    i32.add
    i64.load
    i64.store align=1
    local.get 4
    i32.const 164
    i32.add
    i32.const 45
    i32.add
    local.set 89
    local.get 89
    local.get 4
    i64.load offset=96
    i64.store align=1
    i32.const 24
    local.set 90
    local.get 89
    local.get 90
    i32.add
    local.get 90
    local.get 4
    i32.const 96
    i32.add
    i32.add
    i64.load
    i64.store align=1
    i32.const 16
    local.set 91
    local.get 89
    local.get 91
    i32.add
    local.get 91
    local.get 4
    i32.const 96
    i32.add
    i32.add
    i64.load
    i64.store align=1
    i32.const 8
    local.set 92
    local.get 89
    local.get 92
    i32.add
    local.get 92
    local.get 4
    i32.const 96
    i32.add
    i32.add
    i64.load
    i64.store align=1
    local.get 4
    local.get 63
    i32.store offset=168
    local.get 4
    i32.const 164
    i32.add
    i32.const 77
    i32.add
    local.get 4
    i32.load offset=144
    i32.store align=1
    local.get 4
    local.get 68
    i32.store8 offset=245
    local.get 4
    local.get 78
    i32.store offset=172
    local.get 4
    local.get 81
    i32.store8 offset=246
    local.get 4
    local.get 84
    i32.store8 offset=247
    i32.const 84
    local.set 93
    block  ;; label = @1
      local.get 93
      i32.eqz
      br_if 0 (;@1;)
      local.get 4
      i32.const 248
      i32.add
      local.get 4
      i32.const 164
      i32.add
      local.get 93
      memory.copy
    end
    local.get 4
    i32.const 1
    i32.store8 offset=332
    i32.const 88
    local.set 94
    block  ;; label = @1
      local.get 94
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      local.get 4
      i32.const 248
      i32.add
      local.get 94
      memory.copy
    end
    local.get 4
    i32.const 336
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $root.cell_encode (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 208
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 0
    i32.store offset=8
    local.get 4
    local.get 1
    i32.store offset=12
    local.get 4
    local.get 2
    i32.store offset=16
    local.get 4
    local.get 3
    i32.store offset=20
    local.get 4
    i32.const 32
    i32.store offset=160
    local.get 4
    local.get 4
    i32.const 24
    i32.add
    i32.store offset=156
    local.get 4
    i32.const 0
    i32.store offset=152
    local.get 4
    local.get 0
    i32.store offset=164
    local.get 4
    local.get 1
    i32.store offset=168
    local.get 4
    local.get 3
    i32.store offset=172
    local.get 4
    i32.load offset=164
    local.set 5
    local.get 4
    local.get 5
    i32.store offset=176
    local.get 4
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=183
    local.get 4
    local.get 5
    i32.load align=1
    i32.store offset=184
    local.get 4
    i32.load offset=184 align=1
    local.set 6
    local.get 4
    local.get 6
    i32.store offset=188
    i32.const 24
    local.set 7
    local.get 6
    local.get 7
    i32.shr_u
    local.set 8
    i32.const 8
    local.set 9
    local.get 6
    local.get 9
    i32.shr_u
    local.set 10
    i32.const 65280
    local.set 11
    local.get 8
    local.get 10
    local.get 11
    i32.and
    i32.or
    local.get 6
    local.get 7
    i32.shl
    local.get 6
    local.get 11
    i32.and
    local.get 9
    i32.shl
    i32.or
    i32.or
    local.set 12
    local.get 12
    local.set 13
    local.get 4
    local.get 13
    i32.store offset=192
    local.get 4
    i32.load offset=164
    i32.load8_u offset=4
    local.set 14
    local.get 4
    local.get 14
    i32.store8 offset=199
    i32.const 509
    local.set 15
    local.get 2
    local.get 15
    local.get 2
    local.get 15
    i32.lt_u
    select
    local.set 16
    local.get 4
    i32.load offset=168
    local.set 17
    local.get 4
    local.get 16
    i32.store offset=204
    local.get 4
    local.get 17
    i32.store offset=200
    local.get 4
    i32.load offset=172
    local.set 18
    i32.const 514
    local.set 19
    local.get 4
    i32.const 152
    i32.add
    local.get 13
    local.get 14
    local.get 17
    local.get 16
    local.get 18
    local.get 19
    call $cell.Cell.encodeWithPayload
    local.get 4
    i32.const 208
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $cell.Cell.encodeWithPayload (type $t_6_15) (param i32 i32 i32 i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 64
    i32.sub
    local.set 7
    local.get 7
    global.set $m6__stack_pointer
    local.get 7
    local.get 1
    i32.store offset=4
    local.get 7
    local.get 2
    i32.store8 offset=11
    local.get 7
    local.get 4
    i32.store offset=16
    local.get 7
    local.get 3
    i32.store offset=12
    local.get 7
    local.get 6
    i32.store offset=24
    local.get 7
    local.get 5
    i32.store offset=20
    local.get 7
    local.get 4
    i32.store offset=32
    local.get 7
    local.get 3
    i32.store offset=28
    local.get 7
    local.get 6
    i32.store offset=40
    local.get 7
    local.get 5
    i32.store offset=36
    local.get 7
    i32.load offset=40
    local.set 8
    local.get 7
    i32.load offset=36
    local.set 9
    block  ;; label = @1
      block  ;; label = @2
        i32.const 4
        local.get 8
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 4
      local.get 8
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 7
    local.get 9
    i32.store offset=44
    local.get 7
    local.get 1
    i32.store offset=48
    local.get 7
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=55
    i32.const 24
    local.set 10
    local.get 1
    local.get 10
    i32.shr_u
    local.set 11
    i32.const 8
    local.set 12
    local.get 1
    local.get 12
    i32.shr_u
    local.set 13
    i32.const 65280
    local.set 14
    local.get 7
    local.get 11
    local.get 13
    local.get 14
    i32.and
    i32.or
    local.get 1
    local.get 10
    i32.shl
    local.get 1
    local.get 14
    i32.and
    local.get 12
    i32.shl
    i32.or
    i32.or
    i32.store offset=56
    local.get 9
    local.get 7
    i32.load offset=56
    i32.store align=1
    local.get 7
    i32.load offset=40
    local.set 15
    local.get 7
    i32.load offset=36
    local.set 16
    block  ;; label = @1
      block  ;; label = @2
        i32.const 4
        local.get 15
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 4
      local.get 15
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 16
    local.get 2
    i32.store8 offset=4
    local.get 7
    i32.load offset=40
    local.set 17
    local.get 7
    i32.load offset=36
    i32.const 5
    i32.add
    local.set 18
    block  ;; label = @1
      block  ;; label = @2
        i32.const 5
        local.get 17
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      i32.const 5
      local.get 17
      call $debug.FullPanic__function_'defaultPanic'__.startGreaterThanEnd
      unreachable
    end
    local.get 17
    i32.const 5
    i32.sub
    local.set 19
    block  ;; label = @1
      block  ;; label = @2
        local.get 17
        local.get 17
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 17
      local.get 17
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    i32.const 0
    local.set 20
    block  ;; label = @1
      local.get 19
      i32.eqz
      br_if 0 (;@1;)
      local.get 18
      local.get 20
      local.get 19
      memory.fill
    end
    local.get 7
    i32.load offset=32
    local.set 21
    i32.const 509
    local.set 22
    local.get 21
    local.get 22
    local.get 21
    local.get 22
    i32.lt_u
    select
    local.set 23
    local.get 23
    local.set 24
    local.get 7
    local.get 23
    i32.const 511
    i32.and
    i32.store16 offset=62
    local.get 7
    i32.load offset=40
    local.set 25
    local.get 7
    i32.load offset=36
    local.set 26
    i32.const 5
    local.set 27
    local.get 26
    local.get 27
    i32.add
    local.set 28
    local.get 23
    local.get 27
    i32.add
    local.set 29
    block  ;; label = @1
      block  ;; label = @2
        local.get 29
        local.get 25
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 29
      local.get 25
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 23
    local.set 30
    local.get 28
    local.set 31
    local.get 7
    i32.load offset=32
    local.set 32
    local.get 7
    i32.load offset=28
    local.set 33
    local.get 24
    i32.const 511
    i32.and
    local.set 34
    local.get 34
    i32.const 0
    i32.sub
    local.set 35
    block  ;; label = @1
      block  ;; label = @2
        local.get 34
        local.get 32
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      local.get 34
      local.get 32
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 35
    local.set 36
    local.get 33
    local.set 37
    block  ;; label = @1
      block  ;; label = @2
        local.get 30
        local.get 36
        i32.eq
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.copyLenMismatch
      unreachable
    end
    local.get 37
    local.get 30
    i32.add
    local.set 38
    local.get 31
    local.get 30
    i32.add
    local.set 39
    block  ;; label = @1
      block  ;; label = @2
        local.get 31
        local.get 38
        i32.ge_u
        local.get 37
        local.get 39
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 0
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    block  ;; label = @1
      local.get 30
      i32.eqz
      br_if 0 (;@1;)
      local.get 31
      local.get 37
      local.get 30
      memory.copy
    end
    local.get 7
    i32.const 64
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $root.cell_decode (type $t_6_11) (param i32 i32) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 1744
    i32.sub
    local.set 2
    local.get 2
    global.set $m6__stack_pointer
    local.get 2
    local.get 0
    i32.store offset=4
    local.get 2
    local.get 1
    i32.store offset=8
    local.get 2
    i32.const 32
    i32.store offset=148
    local.get 2
    local.get 2
    i32.const 12
    i32.add
    i32.store offset=144
    local.get 2
    i32.const 0
    i32.store offset=140
    local.get 2
    local.get 0
    i32.store offset=152
    local.get 2
    local.get 1
    i32.store offset=156
    local.get 2
    i32.load offset=152
    local.set 3
    i32.const 514
    local.set 4
    local.get 2
    i32.const 160
    i32.add
    local.get 2
    i32.const 140
    i32.add
    local.get 3
    local.get 4
    call $cell.Cell.decode
    local.get 2
    i32.load8_u offset=680
    local.set 5
    i32.const 0
    local.set 6
    block  ;; label = @1
      block  ;; label = @2
        local.get 5
        i32.const 255
        i32.and
        local.get 6
        i32.const 255
        i32.and
        i32.ne
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        i32.const 520
        local.set 7
        block  ;; label = @3
          local.get 7
          i32.eqz
          br_if 0 (;@3;)
          local.get 2
          i32.const 688
          i32.add
          local.get 2
          i32.const 160
          i32.add
          local.get 7
          memory.copy
        end
        local.get 2
        i32.const 688
        i32.add
        local.set 8
        br 1 (;@1;)
      end
      i32.const -1
      local.set 9
      local.get 2
      i32.const 1744
      i32.add
      global.set $m6__stack_pointer
      local.get 9
      return
    end
    local.get 8
    local.set 10
    i32.const 520
    local.set 11
    block  ;; label = @1
      local.get 11
      i32.eqz
      br_if 0 (;@1;)
      local.get 2
      i32.const 1208
      i32.add
      local.get 10
      local.get 11
      memory.copy
    end
    local.get 2
    i32.load offset=156
    local.set 12
    local.get 2
    i32.load offset=1208
    local.set 13
    local.get 2
    local.get 12
    i32.store offset=1728
    local.get 2
    local.get 13
    i32.store offset=1732
    local.get 2
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=1739
    i32.const 24
    local.set 14
    local.get 13
    local.get 14
    i32.shr_u
    local.set 15
    i32.const 8
    local.set 16
    local.get 13
    local.get 16
    i32.shr_u
    local.set 17
    i32.const 65280
    local.set 18
    local.get 2
    local.get 15
    local.get 17
    local.get 18
    i32.and
    i32.or
    local.get 13
    local.get 14
    i32.shl
    local.get 13
    local.get 18
    i32.and
    local.get 16
    i32.shl
    i32.or
    i32.or
    i32.store offset=1740
    local.get 12
    local.get 2
    i32.load offset=1740
    i32.store align=1
    local.get 2
    i32.load offset=156
    local.get 2
    i32.load8_u offset=1212
    i32.store8 offset=4
    local.get 2
    i32.load offset=156
    i32.const 5
    i32.add
    local.set 19
    local.get 2
    i32.const 1208
    i32.add
    i32.const 8
    i32.add
    local.set 20
    local.get 20
    i32.const 509
    i32.add
    local.set 21
    local.get 19
    i32.const 509
    i32.add
    local.set 22
    block  ;; label = @1
      block  ;; label = @2
        local.get 19
        local.get 21
        i32.ge_u
        local.get 20
        local.get 22
        i32.ge_u
        i32.or
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 2
      i32.const 140
      i32.add
      call $debug.FullPanic__function_'defaultPanic'__.memcpyAlias
      unreachable
    end
    i32.const 509
    local.set 23
    block  ;; label = @1
      local.get 23
      i32.eqz
      br_if 0 (;@1;)
      local.get 19
      local.get 20
      local.get 23
      memory.copy
    end
    i32.const 0
    local.set 24
    local.get 2
    i32.const 1744
    i32.add
    global.set $m6__stack_pointer
    local.get 24
    return)
  (func $cell.Cell.decode (type $t_6_1) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32 i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 1600
    i32.sub
    local.set 4
    local.get 4
    global.set $m6__stack_pointer
    local.get 4
    local.get 3
    i32.store offset=12
    local.get 4
    local.get 2
    i32.store offset=8
    local.get 4
    local.get 3
    i32.store offset=20
    local.get 4
    local.get 2
    i32.store offset=16
    block  ;; label = @1
      local.get 4
      i32.load offset=20
      i32.const 514
      i32.lt_u
      i32.const 1
      i32.and
      i32.eqz
      br_if 0 (;@1;)
      i32.const 1050032
      local.set 5
      i32.const 528
      local.set 6
      block  ;; label = @2
        local.get 6
        i32.eqz
        br_if 0 (;@2;)
        local.get 0
        local.get 5
        local.get 6
        memory.copy
      end
      local.get 4
      i32.const 1600
      i32.add
      global.set $m6__stack_pointer
      return
    end
    local.get 4
    i32.load offset=20
    local.set 7
    local.get 4
    i32.load offset=16
    local.set 8
    block  ;; label = @1
      block  ;; label = @2
        i32.const 4
        local.get 7
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 4
      local.get 7
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 4
    local.get 8
    i32.store offset=24
    local.get 4
    i32.const 0
    i32.const 1
    i32.and
    i32.store8 offset=31
    local.get 4
    local.get 8
    i32.load align=1
    i32.store offset=32
    local.get 4
    i32.load offset=32 align=1
    local.set 9
    local.get 4
    local.get 9
    i32.store offset=36
    i32.const 24
    local.set 10
    local.get 9
    local.get 10
    i32.shr_u
    local.set 11
    i32.const 8
    local.set 12
    local.get 9
    local.get 12
    i32.shr_u
    local.set 13
    i32.const 65280
    local.set 14
    local.get 11
    local.get 13
    local.get 14
    i32.and
    i32.or
    local.get 9
    local.get 10
    i32.shl
    local.get 9
    local.get 14
    i32.and
    local.get 12
    i32.shl
    i32.or
    i32.or
    local.set 15
    local.get 15
    local.set 16
    local.get 4
    i32.load offset=20
    local.set 17
    local.get 4
    i32.load offset=16
    local.set 18
    block  ;; label = @1
      block  ;; label = @2
        i32.const 4
        local.get 17
        i32.lt_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 4
      local.get 17
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    local.get 16
    i64.extend_i32_u
    local.get 18
    i64.load8_u offset=4
    i64.const 32
    i64.shl
    i64.or
    local.set 19
    local.get 4
    i32.load offset=20
    local.set 20
    local.get 4
    i32.load offset=16
    i32.const 5
    i32.add
    local.set 21
    block  ;; label = @1
      block  ;; label = @2
        i32.const 514
        local.get 20
        i32.le_u
        i32.const 1
        i32.and
        i32.eqz
        br_if 0 (;@2;)
        br 1 (;@1;)
      end
      local.get 1
      i32.const 514
      local.get 20
      call $debug.FullPanic__function_'defaultPanic'__.outOfBounds
      unreachable
    end
    i32.const 509
    local.set 22
    block  ;; label = @1
      local.get 22
      i32.eqz
      br_if 0 (;@1;)
      local.get 4
      i32.const 43
      i32.add
      local.get 21
      local.get 22
      memory.copy
    end
    local.get 4
    local.get 19
    i64.const 32
    i64.shr_u
    i64.store8 offset=556
    local.get 4
    local.get 19
    i64.store32 offset=552
    local.get 4
    i32.const 552
    i32.add
    i32.const 8
    i32.add
    local.set 23
    i32.const 509
    local.set 24
    block  ;; label = @1
      local.get 24
      i32.eqz
      br_if 0 (;@1;)
      local.get 23
      local.get 4
      i32.const 43
      i32.add
      local.get 24
      memory.copy
    end
    i32.const 520
    local.set 25
    block  ;; label = @1
      local.get 25
      i32.eqz
      br_if 0 (;@1;)
      local.get 4
      i32.const 1072
      i32.add
      local.get 4
      i32.const 552
      i32.add
      local.get 25
      memory.copy
    end
    local.get 4
    i32.const 1
    i32.store8 offset=1592
    i32.const 528
    local.set 26
    block  ;; label = @1
      local.get 26
      i32.eqz
      br_if 0 (;@1;)
      local.get 0
      local.get 4
      i32.const 1072
      i32.add
      local.get 26
      memory.copy
    end
    local.get 4
    i32.const 1600
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $root.sha3_256 (type $t_6_2) (param i32 i32 i32)
    (local i32 i32 i32 i32)
    global.get $m6__stack_pointer
    i32.const 160
    i32.sub
    local.set 3
    local.get 3
    global.set $m6__stack_pointer
    local.get 3
    local.get 0
    i32.store
    local.get 3
    local.get 1
    i32.store offset=4
    local.get 3
    local.get 2
    i32.store offset=8
    local.get 3
    i32.const 32
    i32.store offset=148
    local.get 3
    local.get 3
    i32.const 12
    i32.add
    i32.store offset=144
    local.get 3
    i32.const 0
    i32.store offset=140
    local.get 3
    local.get 0
    i32.store offset=152
    local.get 3
    local.get 2
    i32.store offset=156
    local.get 3
    i32.load offset=152
    local.set 4
    local.get 1
    local.set 5
    local.get 3
    i32.load offset=156
    local.set 6
    local.get 3
    i32.const 140
    i32.add
    local.get 4
    local.get 5
    local.get 6
    i32.const 1049400
    call $crypto.sha3.Keccak_1600_256_6_24_.hash
    local.get 3
    i32.const 160
    i32.add
    global.set $m6__stack_pointer
    return)
  (func $root.version (type $t_6_16) (result i32)
    i32.const 1
    return)
  (table  5 5 funcref)
  (memory  17)
  (global $m6__stack_pointer (mut i32) (i32.const 1048576))
  (export "identity_from_pubkey" (func $root.identity_from_pubkey))
  (export "route_decode" (func $root.route_decode))
  (export "route_encode" (func $root.route_encode))
  (export "beacon_encode" (func $root.beacon_encode))
  (export "beacon_decode" (func $root.beacon_decode))
  (export "cell_encode" (func $root.cell_encode))
  (export "cell_decode" (func $root.cell_decode))
  (export "sha3_256" (func $root.sha3_256))
  (export "version" (func $root.version))
  (elem  (i32.const 1) func $Io.Writer.fixedDrain $Io.Writer.unimplementedSendFile $Io.Writer.noopFlush $Io.Writer.failingRebase)
  (data $.rodata (i32.const 1048576) "integer overflow\00source and destination arguments have non-equal lengths\00@memcpy arguments alias\00division by zero\00cannot squeeze right after initializing\00cannot squeeze before initializing\00cannot permute before initializing\00cannot absorb right after squeezing\00cannot squeeze right after absorbing\00switch on corrupt value\00integer does not fit in destination type\00reached unreachable code\00cannot transition to uninitialized\0000010203040506070809101112131415161718192021222324252627282930313233343536373839404142434445464748495051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899\001\000\00(msg truncated)\00\00q\02\10\00\01\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00o\02\10\00\01\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00index out of bounds: index \00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\02 \00\00, len start index  is larger than end index \00\00\00\00\00\00\00\00\01\00\00\00\02\00\00\00\03\00\00\00\04\00\00\00\00\00\00\00\04\00\00\00\00\00\00\00\01\00\00\00\06\00\00\00\00\00\00\00\01\00\00\00\00\00\00\00\82\80\00\00\00\00\00\00\8a\80\00\00\00\00\00\80\00\80\00\80\00\00\00\80\8b\80\00\00\00\00\00\00\01\00\00\80\00\00\00\00\81\80\00\80\00\00\00\80\09\80\00\00\00\00\00\80\8a\00\00\00\00\00\00\00\88\00\00\00\00\00\00\00\09\80\00\80\00\00\00\00\0a\00\00\80\00\00\00\00\8b\80\00\80\00\00\00\00\8b\00\00\00\00\00\00\80\89\80\00\00\00\00\00\80\03\80\00\00\00\00\00\80\02\80\00\00\00\00\00\80\80\00\00\00\00\00\00\80\0a\80\00\00\00\00\00\00\0a\00\00\80\00\00\00\80\81\80\00\80\00\00\00\80\80\80\00\00\00\00\00\80\01\00\00\80\00\00\00\00\08\80\00\80\00\00\00\80\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\ff\ff\ff\ff\ff\ff\0b\1d\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")