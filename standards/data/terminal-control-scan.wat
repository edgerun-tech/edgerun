

  (func $m175emit
    (param $out i32) (param $cap i32) (param $count i32)
    (param $kind i32) (param $start i32) (param $len i32)
    (param $param_start i32) (param $param_len i32)
    (param $final i32) (param $flags i32) (param $status i32)
    (result i32)
    (local $base i32)
    local.get $count
    local.get $cap
    i32.ge_u
    if
      i32.const -1
      return
    end
    local.get $out
    local.get $count
    i32.const 32
    i32.mul
    i32.add
    local.set $base
    local.get $base
    local.get $kind
    i32.store
    local.get $base
    i32.const 4
    i32.add
    local.get $start
    i32.store
    local.get $base
    i32.const 8
    i32.add
    local.get $len
    i32.store
    local.get $base
    i32.const 12
    i32.add
    local.get $param_start
    i32.store
    local.get $base
    i32.const 16
    i32.add
    local.get $param_len
    i32.store
    local.get $base
    i32.const 20
    i32.add
    local.get $final
    i32.store
    local.get $base
    i32.const 24
    i32.add
    local.get $flags
    i32.store
    local.get $base
    i32.const 28
    i32.add
    local.get $status
    i32.store
    local.get $count
    i32.const 1
    i32.add)

  (func $is_execute (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.lt_u
    local.get $b
    i32.const 127
    i32.eq
    i32.or)

  (func (export "terminal_control_scan")
    (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32)
    (result i64)
    (local $i i32)
    (local $b i32)
    (local $start i32)
    (local $seq_start i32)
    (local $param_start i32)
    (local $final_pos i32)
    (local $data_start i32)
    (local $count i32)
    (local $next i32)

    loop $scan
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        local.set $b

        local.get $b
        i32.const 27
        i32.eq
        if
          local.get $i
          local.set $seq_start
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          local.get $i
          local.get $len
          i32.ge_u
          if
            local.get $out local.get $cap local.get $count
            i32.const 9 local.get $seq_start i32.const 1
            local.get $seq_start i32.const 1
            i32.const 0 i32.const 0 i32.const 1
            call $m175emit
            local.tee $count
            i32.const -1
            i32.eq
            if
              i32.const 2
              local.get $cap
              call $pack
              return
            end
            br $scan
          end

          local.get $ptr
          local.get $i
          i32.add
          i32.load8_u
          local.set $b

          ;; CSI: ESC [ params/intermediates final
          local.get $b
          i32.const 91
          i32.eq
          if
            local.get $i
            i32.const 1
            i32.add
            local.set $param_start
            local.get $param_start
            local.set $i
            block $csi_done
              loop $csi
                local.get $i
                local.get $len
                i32.ge_u
                if
                  local.get $out local.get $cap local.get $count
                  i32.const 9 local.get $seq_start
                  local.get $len local.get $seq_start i32.sub
                  local.get $param_start
                  local.get $len local.get $param_start i32.sub
                  i32.const 0 i32.const 1 i32.const 1
                  call $m175emit
                  local.tee $count
                  i32.const -1
                  i32.eq
                  if
                    i32.const 2 local.get $cap call $pack return
                  end
                  br $csi_done
                end
                local.get $ptr local.get $i i32.add i32.load8_u
                local.set $b
                local.get $b
                i32.const 64
                i32.ge_u
                local.get $b
                i32.const 126
                i32.le_u
                i32.and
                if
                  local.get $i
                  local.set $final_pos
                  local.get $out local.get $cap local.get $count
                  i32.const 4 local.get $seq_start
                  local.get $final_pos local.get $seq_start i32.sub i32.const 1 i32.add
                  local.get $param_start
                  local.get $final_pos local.get $param_start i32.sub
                  local.get $b i32.const 0 i32.const 0
                  call $m175emit
                  local.tee $count
                  i32.const -1
                  i32.eq
                  if
                    i32.const 2 local.get $cap call $pack return
                  end
                  local.get $final_pos i32.const 1 i32.add local.set $i
                  br $csi_done
                end
                local.get $i i32.const 1 i32.add local.set $i
                br $csi
              end
            end
            br $scan
          end

          ;; OSC: ESC ] payload BEL or ST
          local.get $b
          i32.const 93
          i32.eq
          if
            local.get $i i32.const 1 i32.add local.set $param_start
            local.get $param_start local.set $i
            block $osc_done
              loop $osc
                local.get $i local.get $len i32.ge_u
                if
                  local.get $out local.get $cap local.get $count
                  i32.const 9 local.get $seq_start
                  local.get $len local.get $seq_start i32.sub
                  local.get $param_start
                  local.get $len local.get $param_start i32.sub
                  i32.const 0 i32.const 2 i32.const 1
                  call $m175emit
                  local.tee $count
                  i32.const -1 i32.eq
                  if
                    i32.const 2 local.get $cap call $pack return
                  end
                  br $osc_done
                end
                local.get $ptr local.get $i i32.add i32.load8_u
                local.set $b
                local.get $b i32.const 7 i32.eq
                if
                  local.get $out local.get $cap local.get $count
                  i32.const 5 local.get $seq_start
                  local.get $i local.get $seq_start i32.sub i32.const 1 i32.add
                  local.get $param_start
                  local.get $i local.get $param_start i32.sub
                  i32.const 7 i32.const 1 i32.const 0
                  call $m175emit
                  local.tee $count
                  i32.const -1 i32.eq
                  if
                    i32.const 2 local.get $cap call $pack return
                  end
                  local.get $i i32.const 1 i32.add local.set $i
                  br $osc_done
                end
                local.get $b i32.const 27 i32.eq
                local.get $i i32.const 1 i32.add local.get $len i32.lt_u
                i32.and
                if
                  local.get $ptr local.get $i i32.const 1 i32.add i32.add i32.load8_u
                  i32.const 92
                  i32.eq
                  if
                    local.get $out local.get $cap local.get $count
                    i32.const 5 local.get $seq_start
                    local.get $i local.get $seq_start i32.sub i32.const 2 i32.add
                    local.get $param_start
                    local.get $i local.get $param_start i32.sub
                    i32.const 92 i32.const 2 i32.const 0
                    call $m175emit
                    local.tee $count
                    i32.const -1 i32.eq
                    if
                      i32.const 2 local.get $cap call $pack return
                    end
                    local.get $i i32.const 2 i32.add local.set $i
                    br $osc_done
                  end
                end
                local.get $i i32.const 1 i32.add local.set $i
                br $osc
              end
            end
            br $scan
          end

          ;; DCS: ESC P params final, payload, ST
          local.get $b
          i32.const 80
          i32.eq
          if
            local.get $i i32.const 1 i32.add local.set $param_start
            local.get $param_start local.set $i
            block $dcs_done
              block $dcs_header_missing
                loop $dcs_header
                  local.get $i local.get $len i32.ge_u
                  br_if $dcs_header_missing
                  local.get $ptr local.get $i i32.add i32.load8_u
                  local.set $b
                  local.get $b i32.const 64 i32.ge_u
                  local.get $b i32.const 126 i32.le_u
                  i32.and
                  if
                    local.get $i local.set $final_pos
                    local.get $out local.get $cap local.get $count
                    i32.const 6 local.get $seq_start
                    local.get $final_pos local.get $seq_start i32.sub i32.const 1 i32.add
                    local.get $param_start
                    local.get $final_pos local.get $param_start i32.sub
                    local.get $b i32.const 0 i32.const 0
                    call $m175emit
                    local.tee $count
                    i32.const -1 i32.eq
                    if
                      i32.const 2 local.get $cap call $pack return
                    end
                    local.get $final_pos i32.const 1 i32.add local.set $data_start
                    local.get $data_start local.set $i
                    block $dcs_body_done
                      loop $dcs_body
                        local.get $i local.get $len i32.ge_u
                        if
                          local.get $out local.get $cap local.get $count
                          i32.const 9 local.get $seq_start
                          local.get $len local.get $seq_start i32.sub
                          local.get $data_start
                          local.get $len local.get $data_start i32.sub
                          i32.const 0 i32.const 3 i32.const 1
                          call $m175emit
                          local.tee $count
                          i32.const -1 i32.eq
                          if
                            i32.const 2 local.get $cap call $pack return
                          end
                          br $dcs_body_done
                        end
                        local.get $ptr local.get $i i32.add i32.load8_u
                        i32.const 27
                        i32.eq
                        local.get $i i32.const 1 i32.add local.get $len i32.lt_u
                        i32.and
                        if
                          local.get $ptr local.get $i i32.const 1 i32.add i32.add i32.load8_u
                          i32.const 92
                          i32.eq
                          if
                            local.get $i local.get $data_start i32.gt_u
                            if
                              local.get $out local.get $cap local.get $count
                              i32.const 7 local.get $data_start
                              local.get $i local.get $data_start i32.sub
                              local.get $data_start
                              local.get $i local.get $data_start i32.sub
                              i32.const 0 i32.const 0 i32.const 0
                              call $m175emit
                              local.tee $count
                              i32.const -1 i32.eq
                              if
                                i32.const 2 local.get $cap call $pack return
                              end
                            end
                            local.get $out local.get $cap local.get $count
                            i32.const 8 local.get $i i32.const 2
                            local.get $i i32.const 0
                            i32.const 92 i32.const 2 i32.const 0
                            call $m175emit
                            local.tee $count
                            i32.const -1 i32.eq
                            if
                              i32.const 2 local.get $cap call $pack return
                            end
                            local.get $i i32.const 2 i32.add local.set $i
                            br $dcs_body_done
                          end
                        end
                        local.get $i i32.const 1 i32.add local.set $i
                        br $dcs_body
                      end
                    end
                    br $dcs_done
                  end
                  local.get $i i32.const 1 i32.add local.set $i
                  br $dcs_header
                end
              end
              local.get $out local.get $cap local.get $count
              i32.const 9 local.get $seq_start
              local.get $len local.get $seq_start i32.sub
              local.get $param_start
              local.get $len local.get $param_start i32.sub
              i32.const 0 i32.const 3 i32.const 1
              call $m175emit
              local.tee $count
              i32.const -1 i32.eq
              if
                i32.const 2 local.get $cap call $pack return
              end
            end
            br $scan
          end

          ;; Generic ESC with intermediates.
          local.get $i local.set $param_start
          block $esc_done
            loop $esc
              local.get $i local.get $len i32.ge_u
              if
                local.get $out local.get $cap local.get $count
                i32.const 9 local.get $seq_start
                local.get $len local.get $seq_start i32.sub
                local.get $param_start
                local.get $len local.get $param_start i32.sub
                i32.const 0 i32.const 4 i32.const 1
                call $m175emit
                local.tee $count
                i32.const -1 i32.eq
                if
                  i32.const 2 local.get $cap call $pack return
                end
                br $esc_done
              end
              local.get $ptr local.get $i i32.add i32.load8_u
              local.set $b
              local.get $b i32.const 48 i32.ge_u
              local.get $b i32.const 126 i32.le_u
              i32.and
              if
                local.get $out local.get $cap local.get $count
                i32.const 3 local.get $seq_start
                local.get $i local.get $seq_start i32.sub i32.const 1 i32.add
                local.get $param_start
                local.get $i local.get $param_start i32.sub
                local.get $b i32.const 0 i32.const 0
                call $m175emit
                local.tee $count
                i32.const -1 i32.eq
                if
                  i32.const 2 local.get $cap call $pack return
                end
                local.get $i i32.const 1 i32.add local.set $i
                br $esc_done
              end
              local.get $i i32.const 1 i32.add local.set $i
              br $esc
            end
          end
          br $scan
        end

        local.get $b
        call $is_execute
        if
          local.get $out local.get $cap local.get $count
          i32.const 2 local.get $i i32.const 1
          local.get $i i32.const 0
          local.get $b i32.const 0 i32.const 0
          call $m175emit
          local.tee $count
          i32.const -1 i32.eq
          if
            i32.const 2 local.get $cap call $pack return
          end
          local.get $i i32.const 1 i32.add local.set $i
          br $scan
        end

        local.get $b
        call $is_print
        if
          local.get $i local.set $start
          loop $print
            local.get $i
            local.get $len
            i32.lt_u
            if
              local.get $ptr local.get $i i32.add i32.load8_u
              local.tee $b
              call $is_print
              local.get $b i32.const 27 i32.ne
              i32.and
              if
                local.get $i i32.const 1 i32.add local.set $i
                br $print
              end
            end
          end
          local.get $out local.get $cap local.get $count
          i32.const 1 local.get $start
          local.get $i local.get $start i32.sub
          local.get $start
          local.get $i local.get $start i32.sub
          i32.const 0 i32.const 0 i32.const 0
          call $m175emit
          local.tee $count
          i32.const -1 i32.eq
          if
            i32.const 2 local.get $cap call $pack return
          end
          br $scan
        end

        local.get $i i32.const 1 i32.add local.set $i
        br $scan
      end
    end

    i32.const 0
    local.get $count
    call $pack)
