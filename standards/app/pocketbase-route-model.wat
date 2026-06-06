
  (func (export "proto_standard_id") (result i32)
    i32.const 300096)

  (func $m148hash (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $h i32)
    i32.const 5381
    local.set $h
    loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $h
        i32.const 33
        i32.mul
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.add
        local.set $h
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    local.get $h)

  (func $seg (param $len i32) (param $m148hash i32) (param $want_len i32) (param $want_hash i32) (result i32)
    local.get $len
    local.get $want_len
    i32.eq
    local.get $m148hash
    local.get $want_hash
    i32.eq
    i32.and)

  (func $is_valid_name_byte (param $b i32) (result i32)
    local.get $b
    i32.const 48
    i32.ge_u
    local.get $b
    i32.const 57
    i32.le_u
    i32.and
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and
    i32.or
    local.get $b
    i32.const 97
    i32.ge_u
    local.get $b
    i32.const 122
    i32.le_u
    i32.and
    i32.or
    local.get $b
    i32.const 95
    i32.eq
    i32.or)

  (func (export "pocketbase_valid_name") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    local.get $len
    i32.eqz
    if
      i32.const 0
      return
    end
    loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_valid_name_byte
        i32.eqz
        if
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const 1)

  (func $pocketbase_method_code (export "pocketbase_method_code") (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 3
    i32.eq
    if
      local.get $ptr
      i32.load8_u
      i32.const 71
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 69
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      if
        i32.const 1
        return
      end
      local.get $ptr
      i32.load8_u
      i32.const 80
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 85
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      if
        i32.const 4
        return
      end
    end
    local.get $len
    i32.const 4
    i32.eq
    if
      local.get $ptr
      i32.load8_u
      i32.const 80
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 79
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 83
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      if
        i32.const 2
        return
      end
    end
    local.get $len
    i32.const 5
    i32.eq
    if
      local.get $ptr
      i32.load8_u
      i32.const 80
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 65
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 67
      i32.eq
      i32.and
      local.get $ptr
      i32.const 4
      i32.add
      i32.load8_u
      i32.const 72
      i32.eq
      i32.and
      if
        i32.const 3
        return
      end
    end
    local.get $len
    i32.const 6
    i32.eq
    if
      local.get $ptr
      i32.load8_u
      i32.const 68
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 69
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 76
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 69
      i32.eq
      i32.and
      local.get $ptr
      i32.const 4
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      local.get $ptr
      i32.const 5
      i32.add
      i32.load8_u
      i32.const 69
      i32.eq
      i32.and
      if
        i32.const 5
        return
      end
    end
    local.get $len
    i32.const 7
    i32.eq
    if
      local.get $ptr
      i32.load8_u
      i32.const 79
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 80
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 84
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 73
      i32.eq
      i32.and
      local.get $ptr
      i32.const 4
      i32.add
      i32.load8_u
      i32.const 79
      i32.eq
      i32.and
      local.get $ptr
      i32.const 5
      i32.add
      i32.load8_u
      i32.const 78
      i32.eq
      i32.and
      local.get $ptr
      i32.const 6
      i32.add
      i32.load8_u
      i32.const 83
      i32.eq
      i32.and
      if
        i32.const 6
        return
      end
    end
    i32.const 0)

  (func $method_allowed (param $method i32) (param $mask i32) (result i32)
    local.get $method
    i32.const 0
    i32.gt_s
    if (result i32)
      local.get $mask
      local.get $method
      i32.shr_u
      i32.const 1
      i32.and
    else
      i32.const 0
    end)

  (func $finish (param $out i32) (param $route i32) (param $method i32) (param $mask i32) (param $auth i32) (param $ok_status i32) (param $activity i32) (result i32)
    (local $allowed i32)
    local.get $method
    local.get $mask
    call $method_allowed
    local.set $allowed
    local.get $out
    local.get $route
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $method
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $allowed
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $auth
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $allowed
    if (result i32)
      local.get $ok_status
    else
      i32.const 405
    end
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $activity
    i32.store
    local.get $route)

  (func $finish_404 (param $out i32) (param $method i32) (result i32)
    local.get $out
    i32.const 99
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $method
    i32.store
    local.get $out
    i32.const 8
    i32.add
    i32.const 1
    i32.store
    local.get $out
    i32.const 12
    i32.add
    i32.const 0
    i32.store
    local.get $out
    i32.const 16
    i32.add
    i32.const 404
    i32.store
    local.get $out
    i32.const 20
    i32.add
    i32.const 1
    i32.store
    i32.const 99)

  (func (export "pocketbase_classify_route") (param $m_ptr i32) (param $m_len i32) (param $p_ptr i32) (param $p_len i32) (param $out i32) (result i32)
    (local $method i32)
    (local $start i32)
    (local $end i32)
    (local $i i32)
    (local $idx i32)
    (local $cur_len i32)
    (local $cur_hash i32)
    (local $l1 i32) (local $h1 i32)
    (local $l2 i32) (local $h2 i32)
    (local $l3 i32) (local $h3 i32)
    (local $l4 i32) (local $h4 i32)
    (local $l5 i32) (local $h5 i32)
    (local $b i32)
    local.get $m_ptr
    local.get $m_len
    call $pocketbase_method_code
    local.set $method

    local.get $p_len
    local.set $end
    i32.const 0
    local.set $i
    loop $query
      local.get $i
      local.get $end
      i32.lt_u
      if
        local.get $p_ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 63
        i32.eq
        if
          local.get $i
          local.set $end
        else
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $query
        end
      end
    end
    loop $lead
      local.get $start
      local.get $end
      i32.lt_u
      local.get $p_ptr
      local.get $start
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      if
        local.get $start
        i32.const 1
        i32.add
        local.set $start
        br $lead
      end
    end
    loop $trail
      local.get $end
      local.get $start
      i32.gt_u
      local.get $p_ptr
      local.get $end
      i32.const 1
      i32.sub
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      if
        local.get $end
        i32.const 1
        i32.sub
        local.set $end
        br $trail
      end
    end

    local.get $start
    local.get $end
    i32.eq
    if
      local.get $out
      i32.const 1
      local.get $method
      i32.const 2
      i32.const 0
      i32.const 200
      i32.const 1
      call $finish
      return
    end

    i32.const 1
    local.set $idx
    i32.const 5381
    local.set $cur_hash
    local.get $start
    local.set $i
    loop $parse
      local.get $i
      local.get $end
      i32.le_u
      if
        local.get $i
        local.get $end
        i32.eq
        if
          i32.const 47
          local.set $b
        else
          local.get $p_ptr
          local.get $i
          i32.add
          i32.load8_u
          local.set $b
        end
        local.get $b
        i32.const 47
        i32.eq
        if
          local.get $idx
          i32.const 1
          i32.eq
          if
            local.get $cur_len
            local.set $l1
            local.get $cur_hash
            local.set $h1
          end
          local.get $idx
          i32.const 2
          i32.eq
          if
            local.get $cur_len
            local.set $l2
            local.get $cur_hash
            local.set $h2
          end
          local.get $idx
          i32.const 3
          i32.eq
          if
            local.get $cur_len
            local.set $l3
            local.get $cur_hash
            local.set $h3
          end
          local.get $idx
          i32.const 4
          i32.eq
          if
            local.get $cur_len
            local.set $l4
            local.get $cur_hash
            local.set $h4
          end
          local.get $idx
          i32.const 5
          i32.eq
          if
            local.get $cur_len
            local.set $l5
            local.get $cur_hash
            local.set $h5
          end
          local.get $idx
          i32.const 1
          i32.add
          local.set $idx
          i32.const 0
          local.set $cur_len
          i32.const 5381
          local.set $cur_hash
        else
          local.get $cur_hash
          i32.const 33
          i32.mul
          local.get $b
          i32.add
          local.set $cur_hash
          local.get $cur_len
          i32.const 1
          i32.add
          local.set $cur_len
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $parse
      end
    end
    local.get $idx
    i32.const 1
    i32.sub
    local.set $idx

    local.get $l1
    local.get $h1
    i32.const 1
    i32.const 0x0002b604
    call $seg
    if
      local.get $idx
      i32.const 1
      i32.eq
      if
        local.get $out
        i32.const 1
        local.get $method
        i32.const 2
        i32.const 0
        i32.const 200
        i32.const 1
        call $finish
        return
      end
      local.get $idx
      i32.const 2
      i32.eq
      if
        local.get $l2
        local.get $h2
        i32.const 11
        i32.const 0x0ca56be4
        call $seg
        if
          local.get $out
          i32.const 2
          local.get $method
          i32.const 2
          i32.const 0
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l2
        local.get $h2
        i32.const 20
        i32.const 0xf5593098
        call $seg
        local.get $l2
        local.get $h2
        i32.const 16
        i32.const 0x7d7aa7f1
        call $seg
        i32.or
        if
          local.get $out
          i32.const 2
          local.get $method
          i32.const 2
          i32.const 0
          i32.const 200
          i32.const 1
          call $finish
          return
        end
      end
      local.get $idx
      i32.const 3
      i32.eq
      if
        local.get $l2
        local.get $h2
        i32.const 6
        i32.const 0xf2853858
        call $seg
        if
          local.get $out
          i32.const 2
          local.get $method
          i32.const 2
          i32.const 0
          i32.const 200
          i32.const 0
          call $finish
          return
        end
      end
    end

    local.get $idx
    i32.const 3
    i32.eq
    local.get $l1
    local.get $h1
    i32.const 11
    i32.const 0x4e327181
    call $seg
    i32.and
    local.get $l2
    local.get $h2
    i32.const 14
    i32.const 0x9202368b
    call $seg
    i32.and
    if
      local.get $out
      i32.const 41
      local.get $method
      i32.const 2
      i32.const 0
      i32.const 200
      i32.const 1
      call $finish
      return
    end

    local.get $l1
    local.get $h1
    i32.const 3
    i32.const 0x0b885e5f
    call $seg
    i32.eqz
    if
      local.get $out
      local.get $method
      call $finish_404
      return
    end

    local.get $idx
    i32.const 2
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 6
    i32.const 0x01d23c9b
    call $seg
    i32.and
    if
      local.get $out
      i32.const 3
      local.get $method
      i32.const 2
      i32.const 0
      i32.const 200
      i32.const 0
      call $finish
      return
    end

    local.get $l2
    local.get $h2
    i32.const 11
    i32.const 0x5d358c24
    call $seg
    if
      local.get $idx
      i32.const 2
      i32.eq
      if
        local.get $out
        i32.const 10
        local.get $method
        i32.const 6
        i32.const 2
        i32.const 200
        i32.const 1
        call $finish
        return
      end
      local.get $idx
      i32.const 3
      i32.eq
      if
        local.get $l3
        local.get $h3
        i32.const 6
        i32.const 0x04c06f80
        call $seg
        if
          local.get $out
          i32.const 11
          local.get $method
          i32.const 16
          i32.const 2
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l3
        local.get $h3
        i32.const 4
        i32.const 0x7c9a91cc
        call $seg
        if
          local.get $out
          i32.const 12
          local.get $method
          i32.const 6
          i32.const 2
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $out
        i32.const 13
        local.get $method
        i32.const 42
        i32.const 2
        i32.const 200
        i32.const 1
        call $finish
        return
      end
      local.get $idx
      i32.const 4
      i32.eq
      if
        local.get $l4
        local.get $h4
        i32.const 8
        i32.const 0xe9e0dc6b
        call $seg
        if
          local.get $out
          i32.const 14
          local.get $method
          i32.const 32
          i32.const 2
          i32.const 204
          i32.const 1
          call $finish
          return
        end
        local.get $l4
        local.get $h4
        i32.const 7
        i32.const 0x3e05fd17
        call $seg
        if
          local.get $out
          i32.const 15
          local.get $method
          i32.const 6
          i32.const 4
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l4
        local.get $h4
        i32.const 18
        i32.const 0x1b71ace0
        call $seg
        local.get $l4
        local.get $h4
        i32.const 16
        i32.const 0xaba79580
        call $seg
        i32.or
        if
          local.get $out
          i32.const 19
          local.get $method
          i32.const 4
          i32.const 0
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l4
        local.get $h4
        i32.const 12
        i32.const 0xc1a02cd3
        call $seg
        if
          local.get $out
          i32.const 21
          local.get $method
          i32.const 4
          i32.const 3
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l4
        local.get $h4
        i32.const 12
        i32.const 0x41b500f8
        call $seg
        if
          local.get $out
          i32.const 22
          local.get $method
          i32.const 2
          i32.const 0
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $out
        i32.const 13
        local.get $method
        i32.const 42
        i32.const 2
        i32.const 200
        i32.const 1
        call $finish
        return
      end
      local.get $idx
      i32.const 5
      i32.eq
      if
        local.get $l4
        local.get $h4
        i32.const 7
        i32.const 0x3e05fd17
        call $seg
        if
          local.get $out
          i32.const 16
          local.get $method
          i32.const 42
          i32.const 4
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $l4
        local.get $h4
        i32.const 11
        i32.const 0xdf2143ac
        call $seg
        if
          local.get $out
          i32.const 24
          local.get $method
          i32.const 4
          i32.const 2
          i32.const 200
          i32.const 1
          call $finish
          return
        end
        local.get $out
        i32.const 23
        local.get $method
        i32.const 4
        i32.const 0
        i32.const 200
        i32.const 1
        call $finish
        return
      end
    end

    local.get $idx
    i32.const 5
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 5
    i32.const 0x0f703038
    call $seg
    i32.and
    if
      local.get $out
      i32.const 17
      local.get $method
      i32.const 2
      i32.const 4
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $idx
    i32.const 2
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 8
    i32.const 0xf9e632d8
    call $seg
    i32.and
    if
      local.get $out
      i32.const 25
      local.get $method
      i32.const 6
      i32.const 4
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $idx
    i32.const 2
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 6
    i32.const 0xf1728ec1
    call $seg
    i32.and
    if
      local.get $out
      i32.const 30
      local.get $method
      i32.const 6
      i32.const 5
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $idx
    i32.const 3
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 6
    i32.const 0xf1728ec1
    call $seg
    i32.and
    if
      local.get $out
      i32.const 31
      local.get $method
      i32.const 4
      i32.const 0
      i32.const 200
      i32.const 1
      call $finish
      return
    end

    local.get $l2
    local.get $h2
    i32.const 7
    i32.const 0x650b6b4e
    call $seg
    if
      local.get $out
      i32.const 32
      local.get $method
      local.get $idx
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 6
      else
        local.get $idx
        i32.const 3
        i32.eq
        if (result i32)
          i32.const 34
        else
          i32.const 4
        end
      end
      i32.const 2
      local.get $idx
      i32.const 3
      i32.eq
      local.get $l4
      local.get $h4
      i32.const 7
      i32.const 0x3f2a3809
      call $seg
      i32.and
      if (result i32)
        i32.const 200
      else
        i32.const 200
      end
      i32.const 1
      call $finish
      return
    end

    local.get $idx
    i32.const 2
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 10
    i32.const 0xf5d6b242
    call $seg
    i32.and
    if
      local.get $out
      i32.const 34
      local.get $method
      i32.const 6
      i32.const 2
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $l2
    local.get $h2
    i32.const 8
    i32.const 0x1304dc16
    call $seg
    if
      local.get $out
      i32.const 35
      local.get $method
      local.get $idx
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 10
      else
        i32.const 4
      end
      i32.const 2
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $l2
    local.get $h2
    i32.const 4
    i32.const 0x7c9a2e5a
    call $seg
    if
      local.get $out
      i32.const 36
      local.get $method
      i32.const 2
      i32.const 2
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $l2
    local.get $h2
    i32.const 5
    i32.const 0x0f3ee40a
    call $seg
    if
      local.get $out
      i32.const 37
      local.get $method
      local.get $idx
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 4
      end
      i32.const 2
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $idx
    i32.const 2
    i32.eq
    local.get $l2
    local.get $h2
    i32.const 5
    i32.const 0x0f238ce7
    call $seg
    i32.and
    if
      local.get $out
      i32.const 38
      local.get $method
      i32.const 4
      i32.const 0
      i32.const 200
      i32.const 1
      call $finish
      return
    end
    local.get $l2
    local.get $h2
    i32.const 7
    i32.const 0x533ea70f
    call $seg
    if
      local.get $out
      i32.const 40
      local.get $method
      local.get $idx
      i32.const 5
      i32.eq
      if (result i32)
        i32.const 34
      else
        i32.const 22
      end
      i32.const 2
      i32.const 200
      i32.const 1
      call $finish
      return
    end

    local.get $out
    local.get $method
    call $finish_404)

  (func (export "pocketbase_collection_kind_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $m148hash
    local.set $h
    local.get $len
    i32.const 4
    i32.eq
    local.get $h
    i32.const 0x7c944157
    i32.eq
    i32.and
    if
      i32.const 2
      return
    end
    i32.const 1)

  (func (export "pocketbase_actor_kind_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $m148hash
    local.set $h
    local.get $len
    i32.const 5
    i32.eq
    local.get $h
    i32.const 0x0f12fc8e
    i32.eq
    i32.and
    if
      i32.const 1
      return
    end
    local.get $len
    i32.const 6
    i32.eq
    local.get $h
    i32.const 0x1926f824
    i32.eq
    i32.and
    if
      i32.const 2
      return
    end
    local.get $len
    i32.const 4
    i32.eq
    local.get $h
    i32.const 0x7c96cb25
    i32.eq
    i32.and
    if
      i32.const 3
      return
    end
    i32.const 0)

  (func (export "pocketbase_auth_status") (param $auth_req i32) (param $actor_kind i32) (param $admin_exists i32) (result i32)
    local.get $auth_req
    i32.eqz
    if
      i32.const 200
      return
    end
    local.get $auth_req
    i32.const 5
    i32.eq
    local.get $admin_exists
    i32.eqz
    i32.and
    if
      i32.const 200
      return
    end
    local.get $actor_kind
    i32.eqz
    if
      i32.const 401
      return
    end
    local.get $auth_req
    i32.const 1
    i32.eq
    if
      i32.const 200
      return
    end
    local.get $auth_req
    i32.const 2
    i32.eq
    local.get $actor_kind
    i32.const 1
    i32.eq
    i32.and
    if
      i32.const 200
      return
    end
    local.get $auth_req
    i32.const 3
    i32.eq
    local.get $actor_kind
    i32.const 2
    i32.eq
    i32.and
    if
      i32.const 200
      return
    end
    local.get $auth_req
    i32.const 4
    i32.eq
    if
      i32.const 200
      return
    end
    local.get $auth_req
    i32.const 5
    i32.eq
    local.get $admin_exists
    i32.eqz
    i32.or
    local.get $actor_kind
    i32.const 1
    i32.eq
    i32.or
    if
      i32.const 200
      return
    end
    i32.const 403)

  (func (export "pocketbase_field_type_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $m148hash
    local.set $h
    local.get $len
    i32.const 4
    i32.eq
    if
      local.get $h
      i32.const 0x7c9e690a
      i32.eq
      if i32.const 1 return end
      local.get $h
      i32.const 0x7c94b391
      i32.eq
      if i32.const 6 return end
      local.get $h
      i32.const 0x7c959163
      i32.eq
      if i32.const 7 return end
      local.get $h
      i32.const 0x7c99279f
      i32.eq
      if i32.const 10 return end
    end
    local.get $len
    i32.const 3
    i32.eq
    local.get $h
    i32.const 0x0b88b3b8
    i32.eq
    i32.and
    if i32.const 3 return end
    local.get $len
    i32.const 5
    i32.eq
    local.get $h
    i32.const 0x0f601aed
    i32.eq
    i32.and
    if i32.const 4 return end
    local.get $len
    i32.const 6
    i32.eq
    if
      local.get $h
      i32.const 0xfac52eac
      i32.eq
      if i32.const 2 return end
      local.get $h
      i32.const 0x10f9208e
      i32.eq
      if i32.const 5 return end
      local.get $h
      i32.const 0x1b80e3c5
      i32.eq
      if i32.const 9 return end
    end
    local.get $len
    i32.const 8
    i32.eq
    if
      local.get $h
      i32.const 0x45aa423c
      i32.eq
      if i32.const 8 return end
      local.get $h
      i32.const 0x12c7e483
      i32.eq
      if i32.const 12 return end
      local.get $h
      i32.const 0xf1bcda2a
      i32.eq
      if i32.const 13 return end
      local.get $h
      i32.const 0x17f6dc38
      i32.eq
      if i32.const 14 return end
    end
    i32.const 0)

  (func (export "pocketbase_field_category") (param $type_code i32) (result i32)
    local.get $type_code
    i32.const 1
    i32.eq
    local.get $type_code
    i32.const 2
    i32.eq
    i32.or
    local.get $type_code
    i32.const 3
    i32.eq
    i32.or
    local.get $type_code
    i32.const 4
    i32.eq
    i32.or
    local.get $type_code
    i32.const 14
    i32.eq
    i32.or
    if i32.const 1 return end
    local.get $type_code
    i32.const 5
    i32.eq
    if i32.const 2 return end
    local.get $type_code
    i32.const 6
    i32.eq
    if i32.const 3 return end
    local.get $type_code
    i32.const 7
    i32.eq
    local.get $type_code
    i32.const 8
    i32.eq
    i32.or
    if i32.const 4 return end
    local.get $type_code
    i32.const 9
    i32.eq
    if i32.const 5 return end
    local.get $type_code
    i32.const 10
    i32.eq
    if i32.const 6 return end
    local.get $type_code
    i32.const 11
    i32.eq
    if i32.const 7 return end
    local.get $type_code
    i32.const 12
    i32.eq
    if i32.const 8 return end
    local.get $type_code
    i32.const 13
    i32.eq
    if i32.const 9 return end
    i32.const 10)

  (func (export "pocketbase_email_shape") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $at i32)
    (local $dot_after i32)
    local.get $len
    i32.eqz
    if i32.const 0 return end
    loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 64
        i32.eq
        if
          local.get $i
          i32.eqz
          if i32.const 0 return end
          i32.const 1
          local.set $at
        else
          local.get $at
          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          i32.const 46
          i32.eq
          i32.and
          if
            i32.const 1
            local.set $dot_after
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    local.get $at
    local.get $dot_after
    i32.and
    local.get $ptr
    local.get $len
    i32.const 1
    i32.sub
    i32.add
    i32.load8_u
    i32.const 46
    i32.ne
    i32.and)

  (func (export "pocketbase_url_shape") (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 7
    i32.ge_u
    if
      local.get $ptr
      i32.load8_u
      i32.const 104
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 116
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 116
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 112
      i32.eq
      i32.and
      local.get $ptr
      i32.const 4
      i32.add
      i32.load8_u
      i32.const 58
      i32.eq
      i32.and
      local.get $ptr
      i32.const 5
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      local.get $ptr
      i32.const 6
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      if i32.const 1 return end
    end
    local.get $len
    i32.const 8
    i32.ge_u
    if
      local.get $ptr
      i32.load8_u
      i32.const 104
      i32.eq
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 116
      i32.eq
      i32.and
      local.get $ptr
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 116
      i32.eq
      i32.and
      local.get $ptr
      i32.const 3
      i32.add
      i32.load8_u
      i32.const 112
      i32.eq
      i32.and
      local.get $ptr
      i32.const 4
      i32.add
      i32.load8_u
      i32.const 115
      i32.eq
      i32.and
      local.get $ptr
      i32.const 5
      i32.add
      i32.load8_u
      i32.const 58
      i32.eq
      i32.and
      local.get $ptr
      i32.const 6
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      local.get $ptr
      i32.const 7
      i32.add
      i32.load8_u
      i32.const 47
      i32.eq
      i32.and
      if i32.const 1 return end
    end
    i32.const 0)

  (func (export "pocketbase_pack_status") (param $code i32) (result i32)
    (local $status i32)
    local.get $code
    i32.const 100
    i32.lt_u
    local.get $code
    i32.const 999
    i32.gt_u
    i32.or
    if
      i32.const 500
      local.set $status
    else
      local.get $code
      local.set $status
    end
    local.get $status
    i32.const 100
    i32.div_u
    i32.const 16
    i32.shl
    local.get $status
    i32.or))
