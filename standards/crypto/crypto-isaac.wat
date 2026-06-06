  ;; ISAAC CSPRNG — Bob Jenkins' cryptographically-secure PRNG.
  ;; State buffer layout (2064 bytes):
  ;;   Offset 0:     mem[256]   (1024 bytes)
  ;;   Offset 1024:  rsl[256]   (1024 bytes)
  ;;   Offset 2048:  aa         (4 bytes)
  ;;   Offset 2052:  bb         (4 bytes)
  ;;   Offset 2056:  cc         (4 bytes)
  ;;   Offset 2060:  count      (4 bytes)
  ;;
  ;; Exports:
  ;;   isaac_seed(state_ptr, seed_ptr, seed_word_count) -> 0
  ;;   isaac_next(state_ptr) -> i32 (0-255)
  ;;   isaac_peek(state_ptr) -> i32 (0-255)
  (func (export "proto_standard_id") (result i32) i32.const 300535)

  (func $mix (param $a i32) (param $b i32) (param $c i32) (param $d i32)
             (param $e i32) (param $f i32) (param $g i32) (param $h i32)
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

  (func $generate (param $s i32)
    (local $i i32) (local $x i32) (local $aa i32) (local $y i32)
    (local $aai i32) (local $xi i32) (local $yi i32) (local $bb i32)

    i32.const 2056 local.set $aai
    local.get $s local.get $aai i32.add
    local.get $s local.get $aai i32.add i32.load i32.const 1 i32.add
    local.tee $aa
    i32.store

    i32.const 2052 local.set $aai
    local.get $s local.get $aai i32.add
    local.get $s local.get $aai i32.add i32.load local.get $aa i32.add
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
        if
          local.get $aa i32.const 6 i32.shr_u local.get $aa i32.xor local.set $aa
        else
          local.get $i i32.const 3 i32.and i32.const 2 i32.eq
          if
            local.get $aa i32.const 2 i32.shl local.get $aa i32.xor local.set $aa
          else
            local.get $aa i32.const 16 i32.shr_u local.get $aa i32.xor local.set $aa
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

  (func (export "isaac_seed") (param $s i32) (param $seed i32) (param $n i32) (result i32)
    (local $i i32) (local $a i32) (local $b i32) (local $c i32)
    (local $d i32) (local $e i32) (local $f i32) (local $g i32) (local $h i32)
    (local $cnt i32) (local $t i32)

    i32.const 0 local.set $i
    block $zero_done
    loop $zero_loop
      local.get $i i32.const 516 i32.ge_u br_if $zero_done
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 0 i32.store
      local.get $i i32.const 1 i32.add local.set $i
      br $zero_loop
    end
    end

    local.get $n i32.const 256 i32.gt_u if
      i32.const 256 local.set $n
    end

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
    local.tee $a
    local.set $b
    local.get $a
    local.set $c
    local.get $a
    local.set $d
    local.get $a
    local.set $e
    local.get $a
    local.set $f
    local.get $a
    local.set $g
    local.get $a
    local.set $h

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

      local.get $a
      local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.add local.set $a
      local.get $b
      local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 4 i32.add i32.load
      i32.add local.set $b
      local.get $c
      local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 8 i32.add i32.load
      i32.add local.set $c
      local.get $d
      local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 12 i32.add i32.load
      i32.add local.set $d
      local.get $e
      local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 16 i32.add i32.load
      i32.add local.set $e
      local.get $f
      local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 20 i32.add i32.load
      i32.add local.set $f
      local.get $g
      local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 24 i32.add i32.load
      i32.add local.set $g
      local.get $h
      local.get $s i32.const 1024 i32.add local.get $i i32.const 2 i32.shl i32.add i32.const 28 i32.add i32.load
      i32.add local.set $h

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

      local.get $a
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.add local.set $a
      local.get $b
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 4 i32.add i32.load
      i32.add local.set $b
      local.get $c
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 8 i32.add i32.load
      i32.add local.set $c
      local.get $d
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 12 i32.add i32.load
      i32.add local.set $d
      local.get $e
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 16 i32.add i32.load
      i32.add local.set $e
      local.get $f
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 20 i32.add i32.load
      i32.add local.set $f
      local.get $g
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 24 i32.add i32.load
      i32.add local.set $g
      local.get $h
      local.get $s local.get $i i32.const 2 i32.shl i32.add i32.const 28 i32.add i32.load
      i32.add local.set $h

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

    local.get $s call $generate
    local.get $s i32.const 2060 i32.add i32.const 256 i32.store
    i32.const 0
  )

  (func (export "isaac_next") (param $s i32) (result i32)
    (local $cnt i32)
    local.get $s i32.const 2060 i32.add i32.load i32.eqz if
      local.get $s call $generate
      local.get $s i32.const 2060 i32.add i32.const 256 i32.store
    end
    local.get $s i32.const 2060 i32.add
    local.get $s i32.const 2060 i32.add i32.load i32.const 1 i32.sub
    local.tee $cnt
    i32.store
    local.get $s i32.const 1024 i32.add local.get $cnt i32.const 2 i32.shl i32.add i32.load
    i32.const 0xff i32.and
  )

  (func (export "isaac_peek") (param $s i32) (result i32)
    local.get $s i32.const 2060 i32.add i32.load i32.eqz if
      local.get $s call $generate
      local.get $s i32.const 2060 i32.add i32.const 256 i32.store
    end
    local.get $s i32.const 2060 i32.add i32.load i32.const 1 i32.sub
    i32.const 2 i32.shl
    i32.const 1024 i32.add
    local.get $s i32.add
    i32.load
    i32.const 0xff i32.and
  )
