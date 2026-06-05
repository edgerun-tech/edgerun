(module
  (memory (export "memory") 1)

  ;; Standard 300097: portable Linux machine inventory classifiers extracted from
  ;; crates/node/edgerun-machine-report. Host code gathers facts; this module
  ;; deterministically parses labels and applies deployment-mode policy.

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300097)

  (func $is_wsp (param $b i32) (result i32)
    local.get $b
    i32.const 32
    i32.eq
    local.get $b
    i32.const 9
    i32.eq
    i32.or
    local.get $b
    i32.const 10
    i32.eq
    i32.or
    local.get $b
    i32.const 13
    i32.eq
    i32.or)

  (func $lower (param $b i32) (result i32)
    local.get $b
    i32.const 65
    i32.ge_u
    local.get $b
    i32.const 90
    i32.le_u
    i32.and
    if (result i32)
      local.get $b
      i32.const 32
      i32.add
    else
      local.get $b
    end)

  (func $trim_start (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (local $i i32)
    local.get $start
    local.set $i
    (block $done
      (loop $loop
        local.get $i
        local.get $end
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $is_wsp
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop))
    local.get $i)

  (func $trim_end (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (local $i i32)
    local.get $end
    local.set $i
    (block $done
      (loop $loop
        local.get $i
        local.get $start
        i32.le_u
        br_if $done
        local.get $ptr
        local.get $i
        i32.const 1
        i32.sub
        i32.add
        i32.load8_u
        call $is_wsp
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        br $loop))
    local.get $i)

  (func $hash_lower_trim (param $ptr i32) (param $len i32) (result i32)
    (local $start i32)
    (local $end i32)
    (local $i i32)
    (local $h i32)
    local.get $ptr
    i32.const 0
    local.get $len
    call $trim_start
    local.set $start
    local.get $ptr
    local.get $start
    local.get $len
    call $trim_end
    local.set $end
    i32.const 2166136261
    local.set $h
    local.get $start
    local.set $i
    (block $done
      (loop $loop
        local.get $i
        local.get $end
        i32.ge_u
        br_if $done
        local.get $h
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $lower
        i32.xor
        i32.const 16777619
        i32.mul
        local.set $h
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop))
    local.get $h)

  (func $eq_lit_at (param $ptr i32) (param $len i32) (param $pos i32) (param $lit i32) (param $lit_len i32) (result i32)
    (local $i i32)
    local.get $pos
    local.get $lit_len
    i32.add
    local.get $len
    i32.gt_u
    if
      i32.const 0
      return
    end
    i32.const 0
    local.set $i
    (block $done
      (loop $loop
        local.get $i
        local.get $lit_len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $pos
        i32.add
        local.get $i
        i32.add
        i32.load8_u
        call $lower
        local.get $lit
        local.get $i
        i32.add
        i32.load8_u
        i32.ne
        if
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop))
    i32.const 1)

  (func $contains_lit (param $ptr i32) (param $len i32) (param $lit i32) (param $lit_len i32) (result i32)
    (local $i i32)
    i32.const 0
    local.set $i
    (block $done
      (loop $loop
        local.get $i
        local.get $len
        i32.gt_u
        br_if $done
        local.get $ptr
        local.get $len
        local.get $i
        local.get $lit
        local.get $lit_len
        call $eq_lit_at
        if
          i32.const 1
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop))
    i32.const 0)

  ;; String constants used for substring classification.
  (data (i32.const 8192) "muslglibcgnu libc")

  ;; Label codes:
  ;; os: linux=1
  ;; arch: x86_64=1, aarch64=2, arm=3, riscv64=4
  ;; libc: musl=1, glibc=2
  ;; service managers: systemd=1, openrc=2, runit=3, s6=4, dinit=5,
  ;; supervisor=6, cron=7, rc.local=8
  ;; container runtimes: docker=1, podman=2, containerd=3
  (func (export "linux_os_code") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    local.get $len
    call $hash_lower_trim
    i32.const 3971716381
    i32.eq
    if (result i32) i32.const 1 else i32.const 0 end)

  (func (export "linux_arch_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $hash_lower_trim
    local.set $h
    local.get $h
    i32.const 1604325168
    i32.eq
    local.get $h
    i32.const 3507024215
    i32.eq
    i32.or
    if (result i32)
      i32.const 1
    else
      local.get $h
      i32.const 530990966
      i32.eq
      local.get $h
      i32.const 3010551159
      i32.eq
      i32.or
      if (result i32)
        i32.const 2
      else
        local.get $h
        i32.const 946808757
        i32.eq
        if (result i32)
          i32.const 3
        else
          local.get $h
          i32.const 1803807150
          i32.eq
          if (result i32) i32.const 4 else i32.const 0 end
        end
      end
    end)

  (func (export "linux_libc_code") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    local.get $len
    i32.const 8192
    i32.const 4
    call $contains_lit
    if
      i32.const 1
      return
    end
    local.get $ptr
    local.get $len
    i32.const 8196
    i32.const 5
    call $contains_lit
    local.get $ptr
    local.get $len
    i32.const 8201
    i32.const 8
    call $contains_lit
    i32.or
    if (result i32) i32.const 2 else i32.const 0 end)

  (func $linux_service_manager_code (export "linux_service_manager_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $hash_lower_trim
    local.set $h
    local.get $h
    i32.const 306922600
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $h
      i32.const 3356834086
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $h
        i32.const 2297787159
        i32.eq
        if (result i32)
          i32.const 3
        else
          local.get $h
          i32.const 156351068
          i32.eq
          if (result i32)
            i32.const 4
          else
            local.get $h
            i32.const 3339445165
            i32.eq
            if (result i32)
              i32.const 5
            else
              local.get $h
              i32.const 1933137719
              i32.eq
              local.get $h
              i32.const 3057940393
              i32.eq
              i32.or
              if (result i32)
                i32.const 6
              else
                local.get $h
                i32.const 2623740297
                i32.eq
                if (result i32)
                  i32.const 7
                else
                  local.get $h
                  i32.const 879326223
                  i32.eq
                  if (result i32) i32.const 8 else i32.const 0 end
                end
              end
            end
          end
        end
      end
    end)

  (func (export "linux_pid1_service_manager_code") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    local.get $len
    call $linux_service_manager_code)

  (func (export "linux_container_runtime_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $hash_lower_trim
    local.set $h
    local.get $h
    i32.const 199450061
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $h
      i32.const 3679876552
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $h
        i32.const 670600980
        i32.eq
        local.get $h
        i32.const 4065585376
        i32.eq
        i32.or
        if (result i32) i32.const 3 else i32.const 0 end
      end
    end)

  (func (export "linux_bool_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $hash_lower_trim
    local.set $h
    local.get $h
    i32.const 1319056784
    i32.eq
    local.get $h
    i32.const 1303515621
    i32.eq
    i32.or
    local.get $h
    i32.const 873244444
    i32.eq
    i32.or
    if
      i32.const 1
      return
    end
    local.get $h
    i32.const 1647734778
    i32.eq
    local.get $h
    i32.const 184981848
    i32.eq
    i32.or
    local.get $h
    i32.const 890022063
    i32.eq
    i32.or
    if (result i32) i32.const 0 else i32.const -1 end)

  (func (export "linux_low_port_bind_allowed") (param $uid i32) (param $unprivileged_start i32) (result i32)
    local.get $uid
    i32.const 0
    i32.eq
    local.get $unprivileged_start
    i32.const 0
    i32.eq
    i32.or)

  ;; Inventory key codes follow render_deployment_machine_inventory keys:
  ;; os=1 arch=2 kernel=3 libc=4 pid1=5 uid=6 root=7 sudo=8 doas=9
  ;; low_port_bind=10 writable_paths=11 service_managers=12
  ;; container_runtimes=13 available_modes=14
  (func $linux_inventory_key_code (export "linux_inventory_key_code") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    local.get $ptr
    local.get $len
    call $hash_lower_trim
    local.set $h
    local.get $h
    i32.const 1580477207
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $h
      i32.const 2952804297
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $h
        i32.const 580800216
        i32.eq
        if (result i32)
          i32.const 3
        else
          local.get $h
          i32.const 4258394845
          i32.eq
          if (result i32)
            i32.const 4
          else
            local.get $h
            i32.const 3293471977
            i32.eq
            if (result i32)
              i32.const 5
            else
              local.get $h
              i32.const 1556604621
              i32.eq
              if (result i32)
                i32.const 6
              else
                local.get $h
                i32.const 553455173
                i32.eq
                if (result i32)
                  i32.const 7
                else
                  local.get $h
                  i32.const 689258020
                  i32.eq
                  if (result i32)
                    i32.const 8
                  else
                    local.get $h
                    i32.const 954650836
                    i32.eq
                    if (result i32)
                      i32.const 9
                    else
                      local.get $h
                      i32.const 3341888925
                      i32.eq
                      if (result i32)
                        i32.const 10
                      else
                        local.get $h
                        i32.const 2248814206
                        i32.eq
                        if (result i32)
                          i32.const 11
                        else
                          local.get $h
                          i32.const 2603063385
                          i32.eq
                          if (result i32)
                            i32.const 12
                          else
                            local.get $h
                            i32.const 2477775144
                            i32.eq
                            if (result i32)
                              i32.const 13
                            else
                              local.get $h
                              i32.const 869043507
                              i32.eq
                              if (result i32) i32.const 14 else i32.const 0 end
                            end
                          end
                        end
                      end
                    end
                  end
                end
              end
            end
          end
        end
      end
    end)

  ;; Parse one inventory line split by ':' or '='. out layout:
  ;; key_off, key_len, value_off, value_len. Returns key code, 0 for unknown,
  ;; -1 when no key/value separator is present.
  (func (export "linux_inventory_line_scan")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $i i32)
    (local $ks i32)
    (local $ke i32)
    (local $vs i32)
    (local $ve i32)
    i32.const 0
    local.set $i
    (block $found
      (loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        if
          i32.const -1
          return
        end
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 58
        i32.eq
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 61
        i32.eq
        i32.or
        br_if $found
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))
    local.get $ptr
    i32.const 0
    local.get $i
    call $trim_start
    local.set $ks
    local.get $ptr
    local.get $ks
    local.get $i
    call $trim_end
    local.set $ke
    local.get $ptr
    local.get $i
    i32.const 1
    i32.add
    local.get $len
    call $trim_start
    local.set $vs
    local.get $ptr
    local.get $vs
    local.get $len
    call $trim_end
    local.set $ve
    local.get $out
    local.get $ks
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $ke
    local.get $ks
    i32.sub
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $vs
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $ve
    local.get $vs
    i32.sub
    i32.store
    local.get $ptr
    local.get $ks
    i32.add
    local.get $ke
    local.get $ks
    i32.sub
    call $linux_inventory_key_code)

  ;; Service manager mask bits use the manager codes above: bit 0 systemd,
  ;; bit 1 openrc, bit 2 runit, bit 3 s6, bit 4 dinit, bit 5 supervisor,
  ;; bit 6 cron, bit 7 rc.local.
  (func (export "linux_service_manager_bit") (param $code i32) (result i32)
    local.get $code
    i32.const 1
    i32.lt_s
    local.get $code
    i32.const 8
    i32.gt_s
    i32.or
    if (result i32)
      i32.const 0
    else
      i32.const 1
      local.get $code
      i32.const 1
      i32.sub
      i32.shl
    end)

  ;; Runtime mask bits: bit 0 docker, bit 1 podman, bit 2 containerd.
  (func (export "linux_container_runtime_bit") (param $code i32) (result i32)
    local.get $code
    i32.const 1
    i32.lt_s
    local.get $code
    i32.const 3
    i32.gt_s
    i32.or
    if (result i32)
      i32.const 0
    else
      i32.const 1
      local.get $code
      i32.const 1
      i32.sub
      i32.shl
    end)

  ;; Access flag bits: root=1, sudo=2, doas=4, low_port_bind=8.
  ;; Deployment mode result bits:
  ;; 0 foreground
  ;; 1 user:systemd, 2 system:systemd
  ;; 3 system:openrc, 4 system:runit, 5 system:s6, 6 system:dinit
  ;; 7 user:supervisor, 8 user:cron, 9 system:rc.local
  ;; 10 container:docker, 11 container:podman, 12 container:containerd
  ;; 13 init, 14 bare-metal
  (func (export "linux_deployment_mode_bitmask")
    (param $service_mask i32) (param $runtime_mask i32) (param $access_flags i32)
    (result i32)
    (local $out i32)
    (local $priv i32)
    i32.const 1
    local.set $out
    local.get $access_flags
    i32.const 7
    i32.and
    i32.const 0
    i32.ne
    local.set $priv
    local.get $service_mask
    i32.const 1
    i32.and
    if
      local.get $out
      i32.const 2
      i32.or
      local.set $out
      local.get $priv
      if
        local.get $out
        i32.const 4
        i32.or
        local.set $out
      end
    end
    local.get $priv
    if
      local.get $service_mask
      i32.const 2
      i32.and
      if local.get $out i32.const 8 i32.or local.set $out end
      local.get $service_mask
      i32.const 4
      i32.and
      if local.get $out i32.const 16 i32.or local.set $out end
      local.get $service_mask
      i32.const 8
      i32.and
      if local.get $out i32.const 32 i32.or local.set $out end
      local.get $service_mask
      i32.const 16
      i32.and
      if local.get $out i32.const 64 i32.or local.set $out end
      local.get $service_mask
      i32.const 128
      i32.and
      if local.get $out i32.const 512 i32.or local.set $out end
    end
    local.get $service_mask
    i32.const 32
    i32.and
    if local.get $out i32.const 128 i32.or local.set $out end
    local.get $service_mask
    i32.const 64
    i32.and
    if local.get $out i32.const 256 i32.or local.set $out end
    local.get $runtime_mask
    i32.const 1
    i32.and
    if local.get $out i32.const 1024 i32.or local.set $out end
    local.get $runtime_mask
    i32.const 2
    i32.and
    if local.get $out i32.const 2048 i32.or local.set $out end
    local.get $runtime_mask
    i32.const 4
    i32.and
    if local.get $out i32.const 4096 i32.or local.set $out end
    local.get $access_flags
    i32.const 1
    i32.and
    if
      local.get $out
      i32.const 8192
      i32.or
      i32.const 16384
      i32.or
      local.set $out
    end
    local.get $out)
)
