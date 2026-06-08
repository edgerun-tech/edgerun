(module
  ;; Linux syscall imports — JIT compiles to inline syscall
  (import "linux" "read"  (func $sys_read  (param i32 i32 i32) (result i32)))
  (import "linux" "write" (func $sys_write (param i32 i32 i32) (result i32)))
  (import "linux" "open"  (func $sys_open  (param i32 i32 i32) (result i32)))
  (import "linux" "close" (func $sys_close (param i32) (result i32)))
  (import "linux" "poll"  (func $sys_poll  (param i32 i32 i32) (result i32)))
  (import "linux" "mmap"  (func $sys_mmap  (param i32 i32 i32 i32 i32 i32) (result i32)))
  (import "linux" "munmap" (func $sys_munmap (param i32 i32) (result i32)))
  (import "linux" "socket" (func $sys_socket (param i32 i32 i32) (result i32)))
  (import "linux" "connect" (func $sys_connect (param i32 i32 i32) (result i32)))
  (import "linux" "sendmsg" (func $sys_sendmsg (param i32 i32 i32) (result i32)))
  (import "linux" "memfd_create" (func $sys_memfd_create (param i32 i32) (result i32)))
  (import "linux" "ftruncate" (func $sys_ftruncate (param i32 i32) (result i32)))

  ;; NOTE: (memory ...) removed — shared memory is defined by the hosting module.
  ;;       When run standalone, this module inherits the host memory.

  ;; Helper: write a Wayland message header at the given base address.
  ;; header at $base: u32 object_id, u16 opcode, u16 size
  (func $put_header (param $base i32) (param $obj i32) (param $op i32) (param $sz i32)
    (i32.store (local.get $base) (local.get $obj))
    (i32.store16 (i32.add (local.get $base) (i32.const 4)) (local.get $op))
    (i32.store16 (i32.add (local.get $base) (i32.const 6)) (local.get $sz))
  )

  (func (export "wayland_main") (result i32)
    (local $fd i32) (local $r i32)
    (local $off i32) (local $obj i32) (local $op i32) (local $sz i32)
    (local $gn i32) (local $gv i32) (local $slen i32) (local $nlen i32)
    (local $has_cmp i32) (local $has_xwm i32) (local $has_shm i32)
    (local $cmp_id i32) (local $xwm_id i32) (local $shm_id i32)
    (local $surf_id i32) (local $xsurf_id i32) (local $top_id i32)
    (local $shm_fd i32) (local $shm_ptr i32) (local $shm_sz i32)
    (local $px i32) (local $px_end i32) (local $frame i32)

    ;; ─── STAGE 1: socket + connect ───────────────────────────────────

    (local.set $fd (call $sys_socket (i32.const 1) (i32.const 524289) (i32.const 0)))
    (if (i32.lt_s (local.get $fd) (i32.const 0)) (then (return (i32.const 1))))

    (i32.store16 (i32.const 0x100) (i32.const 1))
    (i64.store (i32.const 0x102) (i64.const 0x6E75722F6E75722F))  ;; /run/run
    (i64.store (i32.const 0x10A) (i64.const 0x3030303172657375))  ;; user1000
    (i64.store (i32.const 0x112) (i64.const 0x6C61772F))          ;; /wal
    (i64.store (i32.const 0x11A) (i64.const 0x302D646E61))        ;; and-0
    (i32.store8 (i32.const 0x122) (i32.const 0))

    (local.set $r (call $sys_connect (local.get $fd) (i32.const 0x100) (i32.const 110)))
    (if (i32.lt_s (local.get $r) (i32.const 0)) (then (return (i32.const 2))))

    ;; ─── STAGE 2: wl_display.get_registry (obj=1, op=1, sz=12) ───

    (call $put_header (i32.const 0x200) (i32.const 1) (i32.const 1) (i32.const 12))
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 12)))

    ;; ─── STAGE 3: read events, detect registry globals ────────────

    (local.set $has_cmp (i32.const 0))
    (local.set $has_xwm (i32.const 0))
    (local.set $has_shm (i32.const 0))

    (block $globals_done
      (loop $globals_loop
        (local.set $r (call $sys_read (local.get $fd) (i32.const 0x400) (i32.const 4096)))
        (if (i32.le_s (local.get $r) (i32.const 0)) (then (return (i32.const 3))))
        (local.set $off (i32.const 0))

        (block $batch_done
          (loop $batch_loop
            (if (i32.ge_u (local.get $off) (local.get $r)) (then (br $batch_done)))
            (local.set $obj (i32.load (i32.add (i32.const 0x400) (local.get $off))))
            (local.set $op (i32.load16_u (i32.add (i32.const 0x404) (local.get $off))))
            (local.set $sz (i32.load16_u (i32.add (i32.const 0x406) (local.get $off))))

            ;; wl_registry.global (obj=2, op=0)
            (if (i32.and (i32.eq (local.get $obj) (i32.const 2)) (i32.eq (local.get $op) (i32.const 0)))
              (then
                (local.set $gn (i32.load (i32.add (i32.const 0x408) (local.get $off))))
                (local.set $slen (i32.load (i32.add (i32.const 0x40C) (local.get $off))))
                (local.set $nlen (i32.and (i32.add (local.get $slen) (i32.const 3)) (i32.const -4)))
                (local.set $gv (i32.load (i32.add (i32.add (i32.const 0x410) (local.get $off)) (local.get $nlen))))

                ;; wl_compositor (strlen+1=13): first 4 bytes = "wl_c" = 0x635F6C77
                (if (i32.eqz (local.get $has_cmp))
                  (then
                    (if (i32.eq (local.get $slen) (i32.const 13))
                      (then
                        (if (i32.eq (i32.load (i32.add (i32.const 0x410) (local.get $off))) (i32.const 0x635F6C77))
                          (then (local.set $cmp_id (local.get $gn)) (local.set $has_cmp (i32.const 1)))
                        )
                      )
                    )
                  )
                )

                ;; xdg_wm_base (strlen+1=11): first 4 bytes = "xdg_" = 0x5F676478
                (if (i32.eqz (local.get $has_xwm))
                  (then
                    (if (i32.eq (local.get $slen) (i32.const 11))
                      (then
                        (if (i32.eq (i32.load (i32.add (i32.const 0x410) (local.get $off))) (i32.const 0x5F676478))
                          (then (local.set $xwm_id (local.get $gn)) (local.set $has_xwm (i32.const 1)))
                        )
                      )
                    )
                  )
                )

                ;; wl_shm (strlen+1=7): first 4 bytes = "wl_s" = 0x735F6C77
                (if (i32.eqz (local.get $has_shm))
                  (then
                    (if (i32.eq (local.get $slen) (i32.const 7))
                      (then
                        (if (i32.eq (i32.load (i32.add (i32.const 0x410) (local.get $off))) (i32.const 0x735F6C77))
                          (then (local.set $shm_id (local.get $gn)) (local.set $has_shm (i32.const 1)))
                        )
                      )
                    )
                  )
                )
              )
            )

            (local.set $off (i32.add (local.get $off) (local.get $sz)))
            (br $batch_loop)
          )
        )

        (if (i32.and (i32.and (local.get $has_cmp) (local.get $has_xwm)) (local.get $has_shm))
          (then (br $globals_done))
        )
        (br $globals_loop)
      )
    )

    ;; ─── STAGE 4: bind registry globals ──────────────────────────

    ;; "wl_compositor\0\0\0" (13 + 3 pad = 16 bytes)
    ;; wl_registry.bind: obj=2, op=0, sz=40
    ;; payload: name(u32), str_len(u32)=13, data(16), ver(u32)=1, new_id(u32)=3
    (call $put_header (i32.const 0x200) (i32.const 2) (i32.const 0) (i32.const 40))
    (i32.store (i32.const 0x208) (local.get $cmp_id))
    (i32.store (i32.const 0x20C) (i32.const 13))
    (i64.store (i32.const 0x210) (i64.const 0x6F706D6F635F6C77))  ;; "wl_compo"
    (i64.store (i32.const 0x218) (i64.const 0x000000726F746973))  ;; "sitor\0\0\0"
    (i32.store (i32.const 0x220) (i32.const 1))
    (i32.store (i32.const 0x224) (i32.const 3))
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 40)))

    ;; "xdg_wm_base\0\0\0\0" (11 + 1 pad = 12 bytes)
    ;; bind: obj=2, op=0, sz=36, payload: name+str(4+12=16)+ver+new_id(8)=36
    (call $put_header (i32.const 0x200) (i32.const 2) (i32.const 0) (i32.const 36))
    (i32.store (i32.const 0x208) (local.get $xwm_id))
    (i32.store (i32.const 0x20C) (i32.const 11))
    (i64.store (i32.const 0x210) (i64.const 0x625F77645F676478))  ;; "xdg_wm_b"
    (i64.store (i32.const 0x218) (i64.const 0x0000000000007361))  ;; "ase\0\0\0\0"
    (i32.store (i32.const 0x220) (i32.const 1))
    (i32.store (i32.const 0x224) (i32.const 4))
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 36)))

    ;; "wl_shm\0\0" (7 + 1 pad = 8 bytes)
    ;; bind: obj=2, op=0, sz=32, payload: name+str(4+8=12)+ver+new_id(8)=32
    (call $put_header (i32.const 0x200) (i32.const 2) (i32.const 0) (i32.const 32))
    (i32.store (i32.const 0x208) (local.get $shm_id))
    (i32.store (i32.const 0x20C) (i32.const 7))
    (i64.store (i32.const 0x210) (i64.const 0x00006D68735F6C77))  ;; "wl_shm\0\0"
    (i32.store (i32.const 0x220) (i32.const 1))
    (i32.store (i32.const 0x224) (i32.const 5))
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 32)))

    ;; Drain events after binds
    (drop (call $sys_read (local.get $fd) (i32.const 0x400) (i32.const 4096)))

    ;; ─── STAGE 5: wl_compositor.create_surface (obj=3, op=0, sz=12) → id=6 ─

    (call $put_header (i32.const 0x200) (i32.const 3) (i32.const 0) (i32.const 12))
    (i32.store (i32.const 0x208) (i32.const 6))
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 12)))

    ;; ─── STAGE 6: xdg_wm_base.get_xdg_surface (obj=4, op=2, sz=16) ──

    (call $put_header (i32.const 0x200) (i32.const 4) (i32.const 2) (i32.const 16))
    (i32.store (i32.const 0x208) (i32.const 7))            ;; new_id = xdg_surface
    (i32.store (i32.const 0x20C) (i32.const 6))            ;; surface = 6
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 16)))

    ;; ─── STAGE 7: xdg_surface.get_toplevel (obj=7, op=1, sz=12) → id=8 ─

    (call $put_header (i32.const 0x200) (i32.const 7) (i32.const 1) (i32.const 12))
    (i32.store (i32.const 0x208) (i32.const 8))
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 12)))

    ;; ─── STAGE 8: xdg_toplevel.set_title (obj=8, op=2, sz=20) ──────

    ;; "EdgeRun\0" strlen+1=8, padded to 8 → 8+4+8=20
    (call $put_header (i32.const 0x200) (i32.const 8) (i32.const 2) (i32.const 20))
    (i32.store (i32.const 0x208) (i32.const 8))
    (i64.store (i32.const 0x20C) (i64.const 0x006E755265646745))  ;; "EdgeRun\0"
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 20)))

    ;; xdg_toplevel.set_app_id (obj=8, op=3, sz=24)
    ;; "er-window\0\0" strlen+1=10, padded to 12 → 8+4+12=24
    (call $put_header (i32.const 0x200) (i32.const 8) (i32.const 3) (i32.const 24))
    (i32.store (i32.const 0x208) (i32.const 10))
    (i64.store (i32.const 0x20C) (i64.const 0x6F646E69772D7265))  ;; "er-windo"
    (i32.store16 (i32.const 0x214) (i32.const 0x0077))            ;; "w\0"
    (i32.store16 (i32.const 0x216) (i32.const 0))                 ;; pad
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 24)))

    ;; ─── STAGE 9: wl_surface.frame (obj=6, op=3, sz=12) + commit (op=6, sz=8) ─

    (call $put_header (i32.const 0x200) (i32.const 6) (i32.const 3) (i32.const 12))
    (i32.store (i32.const 0x208) (i32.const 11))             ;; callback id
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 12)))

    (call $put_header (i32.const 0x200) (i32.const 6) (i32.const 6) (i32.const 8))
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 8)))

    ;; Drain events (xdg_surface.configure, wl_callback.done)
    (drop (call $sys_read (local.get $fd) (i32.const 0x400) (i32.const 4096)))

    ;; ─── STAGE 10: SHM pool (memfd_create + ftruncate + mmap + sendmsg) ──

    (local.set $shm_sz (i32.const 1920000))                    ;; 800*600*4

    (i64.store (i32.const 0x100) (i64.const 0x00006D68732D6C77))  ;; "wl-shm\0\0"
    (local.set $shm_fd (call $sys_memfd_create (i32.const 0x100) (i32.const 0)))
    (if (i32.lt_s (local.get $shm_fd) (i32.const 0)) (then (return (i32.const 5))))

    (local.set $r (call $sys_ftruncate (local.get $shm_fd) (local.get $shm_sz)))
    (if (i32.lt_s (local.get $r) (i32.const 0)) (then (return (i32.const 6))))

    (local.set $shm_ptr (call $sys_mmap
      (i32.const 0) (local.get $shm_sz) (i32.const 3) (i32.const 1) (local.get $shm_fd) (i32.const 0)))
    (if (i32.eq (local.get $shm_ptr) (i32.const -1)) (then (return (i32.const 7))))

    ;; msghdr at 0x600 (56 bytes)
    (i64.store (i32.const 0x600) (i64.const 0))              ;; msg_name = NULL
    (i64.store (i32.const 0x608) (i64.const 0))              ;; msg_namelen=0 + pad
    (i64.store (i32.const 0x610) (i64.const 0x640))          ;; msg_iov
    (i64.store (i32.const 0x618) (i64.const 1))              ;; msg_iovlen
    (i64.store (i32.const 0x620) (i64.const 0x660))          ;; msg_control
    (i64.store (i32.const 0x628) (i64.const 24))             ;; msg_controllen
    (i64.store (i32.const 0x630) (i64.const 0))              ;; msg_flags + pad

    ;; iovec at 0x640 (16 bytes)
    (i64.store (i32.const 0x640) (i64.const 0x680))          ;; iov_base
    (i64.store (i32.const 0x648) (i64.const 16))             ;; iov_len

    ;; cmsghdr at 0x660 (24 bytes)
    (i64.store (i32.const 0x660) (i64.const 20))             ;; cmsg_len = CMSG_LEN(4)
    (i32.store (i32.const 0x668) (i32.const 1))              ;; cmsg_level = SOL_SOCKET
    (i32.store (i32.const 0x66C) (i32.const 1))              ;; cmsg_type = SCM_RIGHTS
    (i32.store (i32.const 0x670) (local.get $shm_fd))        ;; fd
    (i32.store (i32.const 0x674) (i32.const 0))              ;; pad

    ;; wl_shm.create_pool (obj=5, op=0, sz=16): new_id=9, size=1920000
    (call $put_header (i32.const 0x680) (i32.const 5) (i32.const 0) (i32.const 16))
    (i32.store (i32.const 0x688) (i32.const 9))              ;; new_id
    (i32.store (i32.const 0x68C) (local.get $shm_sz))        ;; size

    (local.set $r (call $sys_sendmsg (local.get $fd) (i32.const 0x600) (i32.const 0)))
    (if (i32.lt_s (local.get $r) (i32.const 0)) (then (return (i32.const 8))))

    ;; ─── STAGE 11: wl_shm_pool.create_buffer (obj=9, op=0, sz=28) ──

    (call $put_header (i32.const 0x200) (i32.const 9) (i32.const 0) (i32.const 28))
    (i32.store (i32.const 0x208) (i32.const 10))             ;; new_id
    (i32.store (i32.const 0x20C) (i32.const 0))              ;; offset
    (i32.store (i32.const 0x210) (i32.const 800))            ;; width
    (i32.store (i32.const 0x214) (i32.const 600))            ;; height
    (i32.store (i32.const 0x218) (i32.const 3200))           ;; stride
    (i32.store (i32.const 0x21C) (i32.const 1))              ;; format = XRGB8888
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 28)))

    ;; ─── STAGE 12: fill framebuffer (solid orange) ──────────────

    (local.set $px (local.get $shm_ptr))
    (local.set $px_end (i32.add (local.get $shm_ptr) (local.get $shm_sz)))
    (block $fill_done
      (loop $fill_loop
        (if (i32.ge_u (local.get $px) (local.get $px_end)) (then (br $fill_done)))
        (i32.store8 (local.get $px) (i32.const 0x00))         ;; B
        (i32.store8 (i32.add (local.get $px) (i32.const 1)) (i32.const 0xCC))  ;; G
        (i32.store8 (i32.add (local.get $px) (i32.const 2)) (i32.const 0xFF))  ;; R
        (i32.store8 (i32.add (local.get $px) (i32.const 3)) (i32.const 0xFF))  ;; A
        (local.set $px (i32.add (local.get $px) (i32.const 4)))
        (br $fill_loop)
      )
    )

    ;; ─── STAGE 13: surface.attach (op=1, sz=20) + commit (op=6, sz=8) ──

    (call $put_header (i32.const 0x200) (i32.const 6) (i32.const 1) (i32.const 20))
    (i32.store (i32.const 0x208) (i32.const 10))             ;; buffer
    (i64.store (i32.const 0x20C) (i64.const 0))              ;; x=0, y=0
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 20)))

    (call $put_header (i32.const 0x200) (i32.const 6) (i32.const 6) (i32.const 8))
    (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 8)))

    ;; ─── STAGE 14: event loop (poll + read + redraw) ────────────

    (local.set $frame (i32.const 0))

    (block $loop_done
      (loop $event_loop
        (local.set $frame (i32.add (local.get $frame) (i32.const 1)))
        (if (i32.gt_u (local.get $frame) (i32.const 500)) (then (br $loop_done)))

        (i32.store (i32.const 0x700) (local.get $fd))
        (i32.store16 (i32.const 0x704) (i32.const 1))        ;; POLLIN
        (i32.store16 (i32.const 0x706) (i32.const 0))
        (local.set $r (call $sys_poll (i32.const 0x700) (i32.const 1) (i32.const -1)))
        (if (i32.le_s (local.get $r) (i32.const 0)) (then (br $loop_done)))

        (local.set $r (call $sys_read (local.get $fd) (i32.const 0x400) (i32.const 4096)))
        (if (i32.le_s (local.get $r) (i32.const 0)) (then (br $loop_done)))

        (local.set $off (i32.const 0))
        (block $dispatch_done
          (loop $dispatch_loop
            (if (i32.ge_u (local.get $off) (local.get $r)) (then (br $dispatch_done)))
            (local.set $obj (i32.load (i32.add (i32.const 0x400) (local.get $off))))
            (local.set $op (i32.load16_u (i32.add (i32.const 0x404) (local.get $off))))
            (local.set $sz (i32.load16_u (i32.add (i32.const 0x406) (local.get $off))))

            ;; wl_callback.done (obj=11, op=0) → redraw
            (if (i32.and (i32.eq (local.get $obj) (i32.const 11)) (i32.eq (local.get $op) (i32.const 0)))
              (then
                (local.set $px (local.get $shm_ptr))
                (local.set $px_end (i32.add (local.get $shm_ptr) (local.get $shm_sz)))
                (block $redraw_done
                  (loop $redraw_loop
                    (if (i32.ge_u (local.get $px) (local.get $px_end)) (then (br $redraw_done)))
                    (i32.store8 (local.get $px)
                      (i32.and (i32.shl (local.get $frame) (i32.const 3)) (i32.const 0xFF)))
                    (i32.store8 (i32.add (local.get $px) (i32.const 1))
                      (i32.and (i32.shl (local.get $frame) (i32.const 2)) (i32.const 0xFF)))
                    (i32.store8 (i32.add (local.get $px) (i32.const 2)) (i32.const 0xFF))
                    (i32.store8 (i32.add (local.get $px) (i32.const 3)) (i32.const 0xFF))
                    (local.set $px (i32.add (local.get $px) (i32.const 4)))
                    (br $redraw_loop)
                  )
                )

                ;; Next frame: frame(6,3,12) + attach(6,1,20) + commit(6,6,8)
                (call $put_header (i32.const 0x200) (i32.const 6) (i32.const 3) (i32.const 12))
                (i32.store (i32.const 0x208) (i32.const 11))
                (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 12)))

                (call $put_header (i32.const 0x200) (i32.const 6) (i32.const 1) (i32.const 20))
                (i32.store (i32.const 0x208) (i32.const 10))
                (i64.store (i32.const 0x20C) (i64.const 0))
                (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 20)))

                (call $put_header (i32.const 0x200) (i32.const 6) (i32.const 6) (i32.const 8))
                (drop (call $sys_write (local.get $fd) (i32.const 0x200) (i32.const 8)))
              )
            )

            (local.set $off (i32.add (local.get $off) (local.get $sz)))
            (br $dispatch_loop)
          )
        )

        (br $event_loop)
      )
    )

    ;; ─── STAGE 15: cleanup ─────────────────────────────────────

    (call $sys_munmap (local.get $shm_ptr) (local.get $shm_sz))
    (call $sys_close (local.get $shm_fd))
    (call $sys_close (local.get $fd))

    (return (i32.const 0))
  )
)
