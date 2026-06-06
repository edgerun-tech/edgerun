;; String distance — Jaro-Winkler similarity.
  ;; Returns fixed-point distance * 1000  (0 = no match, 1000 = exact match).
  ;; Threshold > 900 is considered a match per the RuneLite source.
  ;;
  ;; Exports:
  ;;   jaro_winkler(a_ptr, a_len, b_ptr, b_len) -> i32  (distance * 1000)
  (func (export "jaro_winkler") (param $a i32) (param $al i32) (param $b i32) (param $bl i32) (result i32)
    (local $i i32) (local $j i32) (local $k i32)
    (local $match_window i32) (local $match_count i32) (local $transpose i32)
    (local $prefix i32) (local $jaro i32) (local $max_prefix i32)

    ;; Zero the match bit-vectors at offset 0 and 256
    i32.const 0 local.set $i
    block $zero_done_a
    loop $zero_a
      local.get $i local.get $al i32.ge_u br_if $zero_done_a
      i32.const 0 local.get $i i32.add i32.const 0 i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $zero_a
    end
    end
    i32.const 0 local.set $i
    block $zero_done_b
    loop $zero_b
      local.get $i local.get $bl i32.ge_u br_if $zero_done_b
      i32.const 256 local.get $i i32.add i32.const 0 i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $zero_b
    end
    end

    local.get $al local.get $bl i32.gt_s
    if (result i32)
      local.get $al
    else
      local.get $bl
    end
    i32.const 1 i32.sub i32.const 2 i32.div_s
    local.tee $match_window
    i32.const 0 i32.lt_s if
      i32.const 0 local.set $match_window
    end

    i32.const 0 local.set $match_count
    i32.const 0 local.set $i
    block $match_done
    loop $match_loop
      local.get $i local.get $al i32.ge_u br_if $match_done
      local.get $i local.get $match_window i32.sub
      local.tee $j
      i32.const 0 i32.lt_s if
        i32.const 0 local.set $j
      end
      local.get $i local.get $match_window i32.add i32.const 1 i32.add
      local.tee $k
      local.get $bl i32.gt_s if
        local.get $bl local.set $k
      end
      block $found
      loop $search
        local.get $j local.get $k i32.ge_s br_if $found
        i32.const 256 local.get $j i32.add i32.load8_u br_if $found
        local.get $a local.get $i i32.add i32.load8_u
        local.get $b local.get $j i32.add i32.load8_u
        i32.ne if
          local.get $j i32.const 1 i32.add local.set $j
          br $search
        end
        i32.const 0 local.get $i i32.add i32.const 1 i32.store8
        i32.const 256 local.get $j i32.add i32.const 1 i32.store8
        local.get $match_count i32.const 1 i32.add local.set $match_count
        br $found
      end
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $match_loop
    end
    end

    local.get $match_count i32.eqz if
      i32.const 0 return
    end

    ;; Count transpositions
    i32.const 0 local.set $transpose
    i32.const 0 local.set $j
    i32.const 0 local.set $i
    block $trans_done
    loop $trans_loop
      local.get $i local.get $al i32.ge_u br_if $trans_done
      i32.const 0 local.get $i i32.add i32.load8_u if
        loop $find_b
          local.get $j local.get $bl i32.ge_u if
            local.get $j i32.const 1 i32.sub local.set $j
            br $trans_done
          end
          i32.const 256 local.get $j i32.add i32.load8_u
          if
            local.get $a local.get $i i32.add i32.load8_u
            local.get $b local.get $j i32.add i32.load8_u
            i32.ne if
              local.get $transpose i32.const 1 i32.add local.set $transpose
            end
            local.get $j i32.const 1 i32.add local.set $j
            br $find_b
          end
          local.get $j i32.const 1 i32.add local.set $j
          br $find_b
        end
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $trans_loop
    end
    end

    local.get $transpose i32.const 2 i32.div_s local.set $transpose

    ;; Jaro = 1/3 * (m/|s1| + m/|s2| + (m-t)/m)
    local.get $match_count i32.const 1000 i32.mul local.get $al i32.div_s
    local.get $match_count i32.const 1000 i32.mul local.get $bl i32.div_s i32.add
    local.get $match_count local.get $transpose i32.sub i32.const 1000 i32.mul local.get $match_count i32.div_s i32.add
    i32.const 3 i32.div_s local.set $jaro

    ;; Winkler prefix boost: up to 4 chars
    i32.const 4 local.set $max_prefix
    i32.const 0 local.set $prefix
    block $pref_done
    loop $pref_loop
      local.get $prefix local.get $al i32.ge_u br_if $pref_done
      local.get $prefix local.get $bl i32.ge_u br_if $pref_done
      local.get $prefix local.get $max_prefix i32.ge_u br_if $pref_done
      local.get $a local.get $prefix i32.add i32.load8_u
      local.get $b local.get $prefix i32.add i32.load8_u
      i32.ne br_if $pref_done
      local.get $prefix i32.const 1 i32.add local.set $prefix
      br $pref_loop
    end
    end

    ;; result = jaro + (prefix * 0.1 * (1 - jaro))
    local.get $jaro
    local.get $prefix i32.const 100 i32.mul
    i32.const 1000 local.get $jaro i32.sub i32.mul
    i32.const 1000 i32.div_s
    i32.add
  )
