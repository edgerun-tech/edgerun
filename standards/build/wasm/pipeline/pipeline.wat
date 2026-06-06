(module
  ;; EdgeRun shared runtime core — owns linear memory, exports shared helpers.
  ;; All other modules import memory + helpers from here.

  (memory (export "memory") 16384)

  ;; Character classification LUT at 0x1000 (256 bytes)
  ;; bit 0: digit, bit 1: uppercase, bit 2: lowercase, bit 3: tchar,
  ;; bit 4: hex, bit 5: ws, bit 6: scheme, bit 7: dns-label
  (data (i32.const 0x1000) "\00\00\00\00\00\00\00\00\00\20\20\00\00\20\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\20\08\00\08\08\08\08\08\00\00\08\48\00\c8\48\00\d9\d9\d9\d9\d9\d9\d9\d9\d9\d9\00\00\00\00\00\00\00\da\da\da\da\da\da\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\ca\00\00\00\08\08\08\dc\dc\dc\dc\dc\dc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\cc\00\08\00\08\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")

  ;; Lowercase mapping LUT at 0x2000 (256 bytes)
  (data (i32.const 0x2000) "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\61\62\63\64\65\66\67\68\69\6a\6b\6c\6d\6e\6f\70\71\72\73\74\75\76\77\78\79\7a\00\00\00\00\00\00\61\62\63\64\65\66\67\68\69\6a\6b\6c\6d\6e\6f\70\71\72\73\74\75\76\77\78\79\7a\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")

  ;; ── Shared memory layout constants (imported by interpreter + compiler) ──
  (global $OFF_TYPES_BUF (export "OFF_TYPES_BUF") i32 (i32.const 264))
  (global $OFF_CODE_BUF (export "OFF_CODE_BUF") i32 (i32.const 21792))
  (global $OFF_FUNCTIONS_BUF (export "OFF_FUNCTIONS_BUF") i32 (i32.const 17688))
  (global $OFF_DECODED_OPS (export "OFF_DECODED_OPS") i32 (i32.const 0xA0000))
  (global $OFF_DECODED_COUNT (export "OFF_DECODED_COUNT") i32 (i32.const 89864))
  (global $DEC_SZ (export "DEC_SZ") i32 (i32.const 32))
  (global $SZ_TYPE (export "SZ_TYPE") i32 (i32.const 256))
  (global $SZ_FUNC (export "SZ_FUNC") i32 (i32.const 16))
  (global $SZ_CODE (export "SZ_CODE") i32 (i32.const 64))

  ;; ── Character classification helpers ──

  (func $char_class (export "char_class") (param $b i32) (result i32)
    (i32.load8_u offset=0x1000 (local.get $b)))

  (func $is_digit (export "is_digit") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 1)))

  (func $is_upper (export "is_upper") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 2)))

  (func $is_lower (export "is_lower") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 4)))

  (func $is_alpha (export "is_alpha") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 6)))

  (func $is_tchar (export "is_tchar") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 8)))

  (func $is_hex (export "is_hex") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 16)))

  (func $is_ws (export "is_ws") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 32)))

  (func $is_scheme_byte (export "is_scheme_byte") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 64)))

  (func $is_label_byte (export "is_label_byte") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 128)))

  (func $to_lower (export "to_lower") (param $b i32) (result i32)
    (local $cl i32)
    (local.set $cl (call $char_class (local.get $b)))
    (if (i32.and (local.get $cl) (i32.const 2))
      (then (return (i32.or (local.get $b) (i32.const 32)))))
    (if (i32.and (local.get $cl) (i32.const 4))
      (then (return (local.get $b))))
    (i32.const 0))

  (func $to_upper (export "to_upper") (param $b i32) (result i32)
    (if (i32.and (call $char_class (local.get $b)) (i32.const 4))
      (then (return (i32.sub (local.get $b) (i32.const 32)))))
    (local.get $b))

  (func $is_alnum (export "is_alnum") (param $b i32) (result i32)
    (i32.and (call $char_class (local.get $b)) (i32.const 7)))

  (func $is_print (export "is_print") (param $b i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $b) (i32.const 32))
      (i32.le_u (local.get $b) (i32.const 126))))

  (func $is_cont (export "is_cont") (param $b i32) (result i32)
    (i32.eq (i32.and (local.get $b) (i32.const 0xC0)) (i32.const 0x80)))

  ;; ── Pack/unpack helpers ──

  (func $pack (export "pack") (param $status i32) (param $value i32) (result i64)
    (i64.or
      (i64.extend_i32_u (local.get $value))
      (i64.shl (i64.extend_i32_u (local.get $status)) (i64.const 32))))

  (func $pack_u16 (export "pack_u16") (param $a i32) (param $b i32) (result i32)
    (i32.or
      (local.get $b)
      (i32.shl (local.get $a) (i32.const 8))))

  (func $byte (export "byte") (param $v i32) (param $i i32) (result i32)
    (i32.and
      (i32.shr_u (local.get $v) (i32.shl (local.get $i) (i32.const 3)))
      (i32.const 0xFF)))

  (func $has (export "has") (param $v i32) (param $mask i32) (result i32)
    (i32.ne (i32.and (local.get $v) (local.get $mask)) (i32.const 0)))

  ;; ── Shared memcpy ──
  (func $memcpy (export "memcpy") (param $dst i32) (param $src i32) (param $len i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (func $memcpy_off (export "memcpy_off") (param $dst i32) (param $doff i32) (param $src i32) (param $soff i32) (param $len i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (i32.add (local.get $dst) (local.get $doff)) (local.get $i))
          (i32.load8_u
            (i32.add (i32.add (local.get $src) (local.get $soff)) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  ;; ── SIMD stubs ──

  (func $simd_memchr (export "simd_memchr") (param $ptr i32) (param $len i32) (param $byte i32) (result i32)
    (local $i i32) (local $vec v128) (local $cmp v128) (local $mask i32)
    (local.set $i (local.get $ptr))
    (block $done
      (loop $loop
        (br_if $done (i32.lt_u (local.get $len) (i32.const 16)))
        (local.set $vec (v128.load (local.get $i)))
        (local.set $cmp (i8x16.eq (local.get $vec) (i8x16.splat (i32.wrap_i64 (i64.extend_i32_u (local.get $byte))))))
        (local.set $mask (i32x4.bitmask (local.get $cmp)))
        (if (local.get $mask)
          (then (return (i32.add (local.get $i) (i32.ctz (local.get $mask))))))
        (local.set $i (i32.add (local.get $i) (i32.const 16)))
        (local.set $len (i32.sub (local.get $len) (i32.const 16)))
        (br $loop)))
    (block $r_done
      (loop $r_loop
        (br_if $r_done (i32.eqz (local.get $len)))
        (if (i32.eq (i32.load8_u (local.get $i)) (local.get $byte))
          (then (return (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (local.set $len (i32.sub (local.get $len) (i32.const 1)))
        (br $r_loop)))
    (i32.const -1))

  (func $simd_memrchr (export "simd_memrchr") (param $ptr i32) (param $len i32) (param $byte i32) (result i32)
    (local $i i32) (local $end i32) (local $vec v128) (local $cmp v128) (local $mask i32)
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (local.set $i (i32.sub (local.get $end) (i32.const 16)))
    (block $done
      (loop $loop
        (br_if $done (i32.lt_u (local.get $i) (local.get $ptr)))
        (local.set $vec (v128.load (local.get $i)))
        (local.set $cmp (i8x16.eq (local.get $vec) (i8x16.splat (i32.wrap_i64 (i64.extend_i32_u (local.get $byte))))))
        (local.set $mask (i32x4.bitmask (local.get $cmp)))
        (if (local.get $mask)
          (then (return (i32.add (local.get $i) (i32.ctz (local.get $mask))))))
        (local.set $i (i32.sub (local.get $i) (i32.const 16)))
        (br $loop)))
    (local.set $i (i32.sub (local.get $end) (i32.const 1)))
    (block $r_done
      (loop $r_loop
        (br_if $r_done (i32.lt_u (local.get $i) (local.get $ptr)))
        (if (i32.eq (i32.load8_u (local.get $i)) (local.get $byte))
          (then (return (local.get $i))))
        (local.set $i (i32.sub (local.get $i) (i32.const 1)))
        (br $r_loop)))
    (i32.const -1))

  ;; ── Shared status codes (all modules should use these) ──
  (global $STATUS_OK           (export "STATUS_OK")           i32 (i32.const 0))
  (global $STATUS_INPUT_SHORT  (export "STATUS_INPUT_SHORT")  i32 (i32.const 1))
  (global $STATUS_OUTPUT_SHORT (export "STATUS_OUTPUT_SHORT") i32 (i32.const 2))
  (global $STATUS_INVALID      (export "STATUS_INVALID")      i32 (i32.const 3))
  (global $STATUS_OVERFLOW     (export "STATUS_OVERFLOW")     i32 (i32.const 4))
  (global $STATUS_TRUNCATED    (export "STATUS_TRUNCATED")    i32 (i32.const 5))
  (global $STATUS_TOO_LONG     (export "STATUS_TOO_LONG")     i32 (i32.const 6))
  (global $STATUS_MORE         (export "STATUS_MORE")         i32 (i32.const 7))
  (global $STATUS_TIMEOUT      (export "STATUS_TIMEOUT")      i32 (i32.const 8))

  ;; ── Global epoch — shared tick counter across all modules ──
  ;; Schedulers increment this before driving pipelines. Stages read it
  ;; for cross-pipeline synchronization and tick-based timeouts.
  (global $epoch (export "epoch") (mut i32) (i32.const 0))
  (global $LINUX_SYS_X64_READ                    (export "LINUX_SYS_X64_READ") i32 (i32.const 0))
  (global $LINUX_SYS_X64_WRITE                   (export "LINUX_SYS_X64_WRITE") i32 (i32.const 1))
  (global $LINUX_SYS_X64_OPEN                    (export "LINUX_SYS_X64_OPEN") i32 (i32.const 2))
  (global $LINUX_SYS_X64_CLOSE                   (export "LINUX_SYS_X64_CLOSE") i32 (i32.const 3))
  (global $LINUX_SYS_X64_STAT                    (export "LINUX_SYS_X64_STAT") i32 (i32.const 4))
  (global $LINUX_SYS_X64_FSTAT                   (export "LINUX_SYS_X64_FSTAT") i32 (i32.const 5))
  (global $LINUX_SYS_X64_LSTAT                   (export "LINUX_SYS_X64_LSTAT") i32 (i32.const 6))
  (global $LINUX_SYS_X64_POLL                    (export "LINUX_SYS_X64_POLL") i32 (i32.const 7))
  (global $LINUX_SYS_X64_LSEEK                   (export "LINUX_SYS_X64_LSEEK") i32 (i32.const 8))
  (global $LINUX_SYS_X64_MMAP                    (export "LINUX_SYS_X64_MMAP") i32 (i32.const 9))
  (global $LINUX_SYS_X64_MPROTECT                (export "LINUX_SYS_X64_MPROTECT") i32 (i32.const 10))
  (global $LINUX_SYS_X64_MUNMAP                  (export "LINUX_SYS_X64_MUNMAP") i32 (i32.const 11))
  (global $LINUX_SYS_X64_BRK                     (export "LINUX_SYS_X64_BRK") i32 (i32.const 12))
  (global $LINUX_SYS_X64_RT_SIGACTION            (export "LINUX_SYS_X64_RT_SIGACTION") i32 (i32.const 13))
  (global $LINUX_SYS_X64_RT_SIGPROCMASK          (export "LINUX_SYS_X64_RT_SIGPROCMASK") i32 (i32.const 14))
  (global $LINUX_SYS_X64_RT_SIGRETURN            (export "LINUX_SYS_X64_RT_SIGRETURN") i32 (i32.const 15))
  (global $LINUX_SYS_X64_IOCTL                   (export "LINUX_SYS_X64_IOCTL") i32 (i32.const 16))
  (global $LINUX_SYS_X64_PREAD64                 (export "LINUX_SYS_X64_PREAD64") i32 (i32.const 17))
  (global $LINUX_SYS_X64_PWRITE64                (export "LINUX_SYS_X64_PWRITE64") i32 (i32.const 18))
  (global $LINUX_SYS_X64_READV                   (export "LINUX_SYS_X64_READV") i32 (i32.const 19))
  (global $LINUX_SYS_X64_WRITEV                  (export "LINUX_SYS_X64_WRITEV") i32 (i32.const 20))
  (global $LINUX_SYS_X64_ACCESS                  (export "LINUX_SYS_X64_ACCESS") i32 (i32.const 21))
  (global $LINUX_SYS_X64_PIPE                    (export "LINUX_SYS_X64_PIPE") i32 (i32.const 22))
  (global $LINUX_SYS_X64_SELECT                  (export "LINUX_SYS_X64_SELECT") i32 (i32.const 23))
  (global $LINUX_SYS_X64_SCHED_YIELD             (export "LINUX_SYS_X64_SCHED_YIELD") i32 (i32.const 24))
  (global $LINUX_SYS_X64_MREMAP                  (export "LINUX_SYS_X64_MREMAP") i32 (i32.const 25))
  (global $LINUX_SYS_X64_MSYNC                   (export "LINUX_SYS_X64_MSYNC") i32 (i32.const 26))
  (global $LINUX_SYS_X64_MINCORE                 (export "LINUX_SYS_X64_MINCORE") i32 (i32.const 27))
  (global $LINUX_SYS_X64_MADVISE                 (export "LINUX_SYS_X64_MADVISE") i32 (i32.const 28))
  (global $LINUX_SYS_X64_SHMGET                  (export "LINUX_SYS_X64_SHMGET") i32 (i32.const 29))
  (global $LINUX_SYS_X64_SHMAT                   (export "LINUX_SYS_X64_SHMAT") i32 (i32.const 30))
  (global $LINUX_SYS_X64_SHMCTL                  (export "LINUX_SYS_X64_SHMCTL") i32 (i32.const 31))
  (global $LINUX_SYS_X64_DUP                     (export "LINUX_SYS_X64_DUP") i32 (i32.const 32))
  (global $LINUX_SYS_X64_DUP2                    (export "LINUX_SYS_X64_DUP2") i32 (i32.const 33))
  (global $LINUX_SYS_X64_PAUSE                   (export "LINUX_SYS_X64_PAUSE") i32 (i32.const 34))
  (global $LINUX_SYS_X64_NANOSLEEP               (export "LINUX_SYS_X64_NANOSLEEP") i32 (i32.const 35))
  (global $LINUX_SYS_X64_GETITIMER               (export "LINUX_SYS_X64_GETITIMER") i32 (i32.const 36))
  (global $LINUX_SYS_X64_ALARM                   (export "LINUX_SYS_X64_ALARM") i32 (i32.const 37))
  (global $LINUX_SYS_X64_SETITIMER               (export "LINUX_SYS_X64_SETITIMER") i32 (i32.const 38))
  (global $LINUX_SYS_X64_GETPID                  (export "LINUX_SYS_X64_GETPID") i32 (i32.const 39))
  (global $LINUX_SYS_X64_SENDFILE                (export "LINUX_SYS_X64_SENDFILE") i32 (i32.const 40))
  (global $LINUX_SYS_X64_SOCKET                  (export "LINUX_SYS_X64_SOCKET") i32 (i32.const 41))
  (global $LINUX_SYS_X64_CONNECT                 (export "LINUX_SYS_X64_CONNECT") i32 (i32.const 42))
  (global $LINUX_SYS_X64_ACCEPT                  (export "LINUX_SYS_X64_ACCEPT") i32 (i32.const 43))
  (global $LINUX_SYS_X64_SENDTO                  (export "LINUX_SYS_X64_SENDTO") i32 (i32.const 44))
  (global $LINUX_SYS_X64_RECVFROM                (export "LINUX_SYS_X64_RECVFROM") i32 (i32.const 45))
  (global $LINUX_SYS_X64_SENDMSG                 (export "LINUX_SYS_X64_SENDMSG") i32 (i32.const 46))
  (global $LINUX_SYS_X64_RECVMSG                 (export "LINUX_SYS_X64_RECVMSG") i32 (i32.const 47))
  (global $LINUX_SYS_X64_SHUTDOWN                (export "LINUX_SYS_X64_SHUTDOWN") i32 (i32.const 48))
  (global $LINUX_SYS_X64_BIND                    (export "LINUX_SYS_X64_BIND") i32 (i32.const 49))
  (global $LINUX_SYS_X64_LISTEN                  (export "LINUX_SYS_X64_LISTEN") i32 (i32.const 50))
  (global $LINUX_SYS_X64_GETSOCKNAME             (export "LINUX_SYS_X64_GETSOCKNAME") i32 (i32.const 51))
  (global $LINUX_SYS_X64_GETPEERNAME             (export "LINUX_SYS_X64_GETPEERNAME") i32 (i32.const 52))
  (global $LINUX_SYS_X64_SOCKETPAIR              (export "LINUX_SYS_X64_SOCKETPAIR") i32 (i32.const 53))
  (global $LINUX_SYS_X64_SETSOCKOPT              (export "LINUX_SYS_X64_SETSOCKOPT") i32 (i32.const 54))
  (global $LINUX_SYS_X64_GETSOCKOPT              (export "LINUX_SYS_X64_GETSOCKOPT") i32 (i32.const 55))
  (global $LINUX_SYS_X64_CLONE                   (export "LINUX_SYS_X64_CLONE") i32 (i32.const 56))
  (global $LINUX_SYS_X64_FORK                    (export "LINUX_SYS_X64_FORK") i32 (i32.const 57))
  (global $LINUX_SYS_X64_VFORK                   (export "LINUX_SYS_X64_VFORK") i32 (i32.const 58))
  (global $LINUX_SYS_X64_EXECVE                  (export "LINUX_SYS_X64_EXECVE") i32 (i32.const 59))
  (global $LINUX_SYS_X64_EXIT                    (export "LINUX_SYS_X64_EXIT") i32 (i32.const 60))
  (global $LINUX_SYS_X64_WAIT4                   (export "LINUX_SYS_X64_WAIT4") i32 (i32.const 61))
  (global $LINUX_SYS_X64_KILL                    (export "LINUX_SYS_X64_KILL") i32 (i32.const 62))
  (global $LINUX_SYS_X64_UNAME                   (export "LINUX_SYS_X64_UNAME") i32 (i32.const 63))
  (global $LINUX_SYS_X64_SEMGET                  (export "LINUX_SYS_X64_SEMGET") i32 (i32.const 64))
  (global $LINUX_SYS_X64_SEMOP                   (export "LINUX_SYS_X64_SEMOP") i32 (i32.const 65))
  (global $LINUX_SYS_X64_SEMCTL                  (export "LINUX_SYS_X64_SEMCTL") i32 (i32.const 66))
  (global $LINUX_SYS_X64_SHMDT                   (export "LINUX_SYS_X64_SHMDT") i32 (i32.const 67))
  (global $LINUX_SYS_X64_MSGGET                  (export "LINUX_SYS_X64_MSGGET") i32 (i32.const 68))
  (global $LINUX_SYS_X64_MSGSND                  (export "LINUX_SYS_X64_MSGSND") i32 (i32.const 69))
  (global $LINUX_SYS_X64_MSGRCV                  (export "LINUX_SYS_X64_MSGRCV") i32 (i32.const 70))
  (global $LINUX_SYS_X64_MSGCTL                  (export "LINUX_SYS_X64_MSGCTL") i32 (i32.const 71))
  (global $LINUX_SYS_X64_FCNTL                   (export "LINUX_SYS_X64_FCNTL") i32 (i32.const 72))
  (global $LINUX_SYS_X64_FLOCK                   (export "LINUX_SYS_X64_FLOCK") i32 (i32.const 73))
  (global $LINUX_SYS_X64_FSYNC                   (export "LINUX_SYS_X64_FSYNC") i32 (i32.const 74))
  (global $LINUX_SYS_X64_FDATASYNC               (export "LINUX_SYS_X64_FDATASYNC") i32 (i32.const 75))
  (global $LINUX_SYS_X64_TRUNCATE                (export "LINUX_SYS_X64_TRUNCATE") i32 (i32.const 76))
  (global $LINUX_SYS_X64_FTRUNCATE               (export "LINUX_SYS_X64_FTRUNCATE") i32 (i32.const 77))
  (global $LINUX_SYS_X64_GETDENTS                (export "LINUX_SYS_X64_GETDENTS") i32 (i32.const 78))
  (global $LINUX_SYS_X64_GETCWD                  (export "LINUX_SYS_X64_GETCWD") i32 (i32.const 79))
  (global $LINUX_SYS_X64_CHDIR                   (export "LINUX_SYS_X64_CHDIR") i32 (i32.const 80))
  (global $LINUX_SYS_X64_FCHDIR                  (export "LINUX_SYS_X64_FCHDIR") i32 (i32.const 81))
  (global $LINUX_SYS_X64_RENAME                  (export "LINUX_SYS_X64_RENAME") i32 (i32.const 82))
  (global $LINUX_SYS_X64_MKDIR                   (export "LINUX_SYS_X64_MKDIR") i32 (i32.const 83))
  (global $LINUX_SYS_X64_RMDIR                   (export "LINUX_SYS_X64_RMDIR") i32 (i32.const 84))
  (global $LINUX_SYS_X64_CREAT                   (export "LINUX_SYS_X64_CREAT") i32 (i32.const 85))
  (global $LINUX_SYS_X64_LINK                    (export "LINUX_SYS_X64_LINK") i32 (i32.const 86))
  (global $LINUX_SYS_X64_UNLINK                  (export "LINUX_SYS_X64_UNLINK") i32 (i32.const 87))
  (global $LINUX_SYS_X64_SYMLINK                 (export "LINUX_SYS_X64_SYMLINK") i32 (i32.const 88))
  (global $LINUX_SYS_X64_READLINK                (export "LINUX_SYS_X64_READLINK") i32 (i32.const 89))
  (global $LINUX_SYS_X64_CHMOD                   (export "LINUX_SYS_X64_CHMOD") i32 (i32.const 90))
  (global $LINUX_SYS_X64_FCHMOD                  (export "LINUX_SYS_X64_FCHMOD") i32 (i32.const 91))
  (global $LINUX_SYS_X64_CHOWN                   (export "LINUX_SYS_X64_CHOWN") i32 (i32.const 92))
  (global $LINUX_SYS_X64_FCHOWN                  (export "LINUX_SYS_X64_FCHOWN") i32 (i32.const 93))
  (global $LINUX_SYS_X64_LCHOWN                  (export "LINUX_SYS_X64_LCHOWN") i32 (i32.const 94))
  (global $LINUX_SYS_X64_UMASK                   (export "LINUX_SYS_X64_UMASK") i32 (i32.const 95))
  (global $LINUX_SYS_X64_GETTIMEOFDAY            (export "LINUX_SYS_X64_GETTIMEOFDAY") i32 (i32.const 96))
  (global $LINUX_SYS_X64_GETRLIMIT               (export "LINUX_SYS_X64_GETRLIMIT") i32 (i32.const 97))
  (global $LINUX_SYS_X64_GETRUSAGE               (export "LINUX_SYS_X64_GETRUSAGE") i32 (i32.const 98))
  (global $LINUX_SYS_X64_SYSINFO                 (export "LINUX_SYS_X64_SYSINFO") i32 (i32.const 99))
  (global $LINUX_SYS_X64_TIMES                   (export "LINUX_SYS_X64_TIMES") i32 (i32.const 100))
  (global $LINUX_SYS_X64_PTRACE                  (export "LINUX_SYS_X64_PTRACE") i32 (i32.const 101))
  (global $LINUX_SYS_X64_GETUID                  (export "LINUX_SYS_X64_GETUID") i32 (i32.const 102))
  (global $LINUX_SYS_X64_SYSLOG                  (export "LINUX_SYS_X64_SYSLOG") i32 (i32.const 103))
  (global $LINUX_SYS_X64_GETGID                  (export "LINUX_SYS_X64_GETGID") i32 (i32.const 104))
  (global $LINUX_SYS_X64_SETUID                  (export "LINUX_SYS_X64_SETUID") i32 (i32.const 105))
  (global $LINUX_SYS_X64_SETGID                  (export "LINUX_SYS_X64_SETGID") i32 (i32.const 106))
  (global $LINUX_SYS_X64_GETEUID                 (export "LINUX_SYS_X64_GETEUID") i32 (i32.const 107))
  (global $LINUX_SYS_X64_GETEGID                 (export "LINUX_SYS_X64_GETEGID") i32 (i32.const 108))
  (global $LINUX_SYS_X64_SETPGID                 (export "LINUX_SYS_X64_SETPGID") i32 (i32.const 109))
  (global $LINUX_SYS_X64_GETPPID                 (export "LINUX_SYS_X64_GETPPID") i32 (i32.const 110))
  (global $LINUX_SYS_X64_GETPGRP                 (export "LINUX_SYS_X64_GETPGRP") i32 (i32.const 111))
  (global $LINUX_SYS_X64_SETSID                  (export "LINUX_SYS_X64_SETSID") i32 (i32.const 112))
  (global $LINUX_SYS_X64_SETREUID                (export "LINUX_SYS_X64_SETREUID") i32 (i32.const 113))
  (global $LINUX_SYS_X64_SETREGID                (export "LINUX_SYS_X64_SETREGID") i32 (i32.const 114))
  (global $LINUX_SYS_X64_GETGROUPS               (export "LINUX_SYS_X64_GETGROUPS") i32 (i32.const 115))
  (global $LINUX_SYS_X64_SETGROUPS               (export "LINUX_SYS_X64_SETGROUPS") i32 (i32.const 116))
  (global $LINUX_SYS_X64_SETRESUID               (export "LINUX_SYS_X64_SETRESUID") i32 (i32.const 117))
  (global $LINUX_SYS_X64_GETRESUID               (export "LINUX_SYS_X64_GETRESUID") i32 (i32.const 118))
  (global $LINUX_SYS_X64_SETRESGID               (export "LINUX_SYS_X64_SETRESGID") i32 (i32.const 119))
  (global $LINUX_SYS_X64_GETRESGID               (export "LINUX_SYS_X64_GETRESGID") i32 (i32.const 120))
  (global $LINUX_SYS_X64_GETPGID                 (export "LINUX_SYS_X64_GETPGID") i32 (i32.const 121))
  (global $LINUX_SYS_X64_SETFSUID                (export "LINUX_SYS_X64_SETFSUID") i32 (i32.const 122))
  (global $LINUX_SYS_X64_SETFSGID                (export "LINUX_SYS_X64_SETFSGID") i32 (i32.const 123))
  (global $LINUX_SYS_X64_GETSID                  (export "LINUX_SYS_X64_GETSID") i32 (i32.const 124))
  (global $LINUX_SYS_X64_CAPGET                  (export "LINUX_SYS_X64_CAPGET") i32 (i32.const 125))
  (global $LINUX_SYS_X64_CAPSET                  (export "LINUX_SYS_X64_CAPSET") i32 (i32.const 126))
  (global $LINUX_SYS_X64_RT_SIGPENDING           (export "LINUX_SYS_X64_RT_SIGPENDING") i32 (i32.const 127))
  (global $LINUX_SYS_X64_RT_SIGTIMEDWAIT         (export "LINUX_SYS_X64_RT_SIGTIMEDWAIT") i32 (i32.const 128))
  (global $LINUX_SYS_X64_RT_SIGQUEUEINFO         (export "LINUX_SYS_X64_RT_SIGQUEUEINFO") i32 (i32.const 129))
  (global $LINUX_SYS_X64_RT_SIGSUSPEND           (export "LINUX_SYS_X64_RT_SIGSUSPEND") i32 (i32.const 130))
  (global $LINUX_SYS_X64_SIGALTSTACK             (export "LINUX_SYS_X64_SIGALTSTACK") i32 (i32.const 131))
  (global $LINUX_SYS_X64_UTIME                   (export "LINUX_SYS_X64_UTIME") i32 (i32.const 132))
  (global $LINUX_SYS_X64_MKNOD                   (export "LINUX_SYS_X64_MKNOD") i32 (i32.const 133))
  (global $LINUX_SYS_X64_USELIB                  (export "LINUX_SYS_X64_USELIB") i32 (i32.const 134))
  (global $LINUX_SYS_X64_PERSONALITY             (export "LINUX_SYS_X64_PERSONALITY") i32 (i32.const 135))
  (global $LINUX_SYS_X64_USTAT                   (export "LINUX_SYS_X64_USTAT") i32 (i32.const 136))
  (global $LINUX_SYS_X64_STATFS                  (export "LINUX_SYS_X64_STATFS") i32 (i32.const 137))
  (global $LINUX_SYS_X64_FSTATFS                 (export "LINUX_SYS_X64_FSTATFS") i32 (i32.const 138))
  (global $LINUX_SYS_X64_SYSFS                   (export "LINUX_SYS_X64_SYSFS") i32 (i32.const 139))
  (global $LINUX_SYS_X64_GETPRIORITY             (export "LINUX_SYS_X64_GETPRIORITY") i32 (i32.const 140))
  (global $LINUX_SYS_X64_SETPRIORITY             (export "LINUX_SYS_X64_SETPRIORITY") i32 (i32.const 141))
  (global $LINUX_SYS_X64_SCHED_SETPARAM          (export "LINUX_SYS_X64_SCHED_SETPARAM") i32 (i32.const 142))
  (global $LINUX_SYS_X64_SCHED_GETPARAM          (export "LINUX_SYS_X64_SCHED_GETPARAM") i32 (i32.const 143))
  (global $LINUX_SYS_X64_SCHED_SETSCHEDULER      (export "LINUX_SYS_X64_SCHED_SETSCHEDULER") i32 (i32.const 144))
  (global $LINUX_SYS_X64_SCHED_GETSCHEDULER      (export "LINUX_SYS_X64_SCHED_GETSCHEDULER") i32 (i32.const 145))
  (global $LINUX_SYS_X64_SCHED_GET_PRIORITY_MAX  (export "LINUX_SYS_X64_SCHED_GET_PRIORITY_MAX") i32 (i32.const 146))
  (global $LINUX_SYS_X64_SCHED_GET_PRIORITY_MIN  (export "LINUX_SYS_X64_SCHED_GET_PRIORITY_MIN") i32 (i32.const 147))
  (global $LINUX_SYS_X64_SCHED_RR_GET_INTERVAL   (export "LINUX_SYS_X64_SCHED_RR_GET_INTERVAL") i32 (i32.const 148))
  (global $LINUX_SYS_X64_MLOCK                   (export "LINUX_SYS_X64_MLOCK") i32 (i32.const 149))
  (global $LINUX_SYS_X64_MUNLOCK                 (export "LINUX_SYS_X64_MUNLOCK") i32 (i32.const 150))
  (global $LINUX_SYS_X64_MLOCKALL                (export "LINUX_SYS_X64_MLOCKALL") i32 (i32.const 151))
  (global $LINUX_SYS_X64_MUNLOCKALL              (export "LINUX_SYS_X64_MUNLOCKALL") i32 (i32.const 152))
  (global $LINUX_SYS_X64_VHANGUP                 (export "LINUX_SYS_X64_VHANGUP") i32 (i32.const 153))
  (global $LINUX_SYS_X64_MODIFY_LDT              (export "LINUX_SYS_X64_MODIFY_LDT") i32 (i32.const 154))
  (global $LINUX_SYS_X64_PIVOT_ROOT              (export "LINUX_SYS_X64_PIVOT_ROOT") i32 (i32.const 155))
  (global $LINUX_SYS_X64_SYSCTL                  (export "LINUX_SYS_X64_SYSCTL") i32 (i32.const 156))
  (global $LINUX_SYS_X64_PRCTL                   (export "LINUX_SYS_X64_PRCTL") i32 (i32.const 157))
  (global $LINUX_SYS_X64_ARCH_PRCTL              (export "LINUX_SYS_X64_ARCH_PRCTL") i32 (i32.const 158))
  (global $LINUX_SYS_X64_ADJTIMEX                (export "LINUX_SYS_X64_ADJTIMEX") i32 (i32.const 159))
  (global $LINUX_SYS_X64_SETRLIMIT               (export "LINUX_SYS_X64_SETRLIMIT") i32 (i32.const 160))
  (global $LINUX_SYS_X64_CHROOT                  (export "LINUX_SYS_X64_CHROOT") i32 (i32.const 161))
  (global $LINUX_SYS_X64_SYNC                    (export "LINUX_SYS_X64_SYNC") i32 (i32.const 162))
  (global $LINUX_SYS_X64_ACCT                    (export "LINUX_SYS_X64_ACCT") i32 (i32.const 163))
  (global $LINUX_SYS_X64_SETTIMEOFDAY            (export "LINUX_SYS_X64_SETTIMEOFDAY") i32 (i32.const 164))
  (global $LINUX_SYS_X64_MOUNT                   (export "LINUX_SYS_X64_MOUNT") i32 (i32.const 165))
  (global $LINUX_SYS_X64_UMOUNT2                 (export "LINUX_SYS_X64_UMOUNT2") i32 (i32.const 166))
  (global $LINUX_SYS_X64_SWAPON                  (export "LINUX_SYS_X64_SWAPON") i32 (i32.const 167))
  (global $LINUX_SYS_X64_SWAPOFF                 (export "LINUX_SYS_X64_SWAPOFF") i32 (i32.const 168))
  (global $LINUX_SYS_X64_REBOOT                  (export "LINUX_SYS_X64_REBOOT") i32 (i32.const 169))
  (global $LINUX_SYS_X64_SETHOSTNAME             (export "LINUX_SYS_X64_SETHOSTNAME") i32 (i32.const 170))
  (global $LINUX_SYS_X64_SETDOMAINNAME           (export "LINUX_SYS_X64_SETDOMAINNAME") i32 (i32.const 171))
  (global $LINUX_SYS_X64_IOPL                    (export "LINUX_SYS_X64_IOPL") i32 (i32.const 172))
  (global $LINUX_SYS_X64_IOPERM                  (export "LINUX_SYS_X64_IOPERM") i32 (i32.const 173))
  (global $LINUX_SYS_X64_CREATE_MODULE           (export "LINUX_SYS_X64_CREATE_MODULE") i32 (i32.const 174))
  (global $LINUX_SYS_X64_INIT_MODULE             (export "LINUX_SYS_X64_INIT_MODULE") i32 (i32.const 175))
  (global $LINUX_SYS_X64_DELETE_MODULE           (export "LINUX_SYS_X64_DELETE_MODULE") i32 (i32.const 176))
  (global $LINUX_SYS_X64_GET_KERNEL_SYMS         (export "LINUX_SYS_X64_GET_KERNEL_SYMS") i32 (i32.const 177))
  (global $LINUX_SYS_X64_QUERY_MODULE            (export "LINUX_SYS_X64_QUERY_MODULE") i32 (i32.const 178))
  (global $LINUX_SYS_X64_QUOTACTL                (export "LINUX_SYS_X64_QUOTACTL") i32 (i32.const 179))
  (global $LINUX_SYS_X64_NFSSERVCTL              (export "LINUX_SYS_X64_NFSSERVCTL") i32 (i32.const 180))
  (global $LINUX_SYS_X64_GETPMSG                 (export "LINUX_SYS_X64_GETPMSG") i32 (i32.const 181))
  (global $LINUX_SYS_X64_PUTPMSG                 (export "LINUX_SYS_X64_PUTPMSG") i32 (i32.const 182))
  (global $LINUX_SYS_X64_AFS_SYSCALL             (export "LINUX_SYS_X64_AFS_SYSCALL") i32 (i32.const 183))
  (global $LINUX_SYS_X64_TUXCALL                 (export "LINUX_SYS_X64_TUXCALL") i32 (i32.const 184))
  (global $LINUX_SYS_X64_SECURITY                (export "LINUX_SYS_X64_SECURITY") i32 (i32.const 185))
  (global $LINUX_SYS_X64_GETTID                  (export "LINUX_SYS_X64_GETTID") i32 (i32.const 186))
  (global $LINUX_SYS_X64_READAHEAD               (export "LINUX_SYS_X64_READAHEAD") i32 (i32.const 187))
  (global $LINUX_SYS_X64_SETXATTR                (export "LINUX_SYS_X64_SETXATTR") i32 (i32.const 188))
  (global $LINUX_SYS_X64_LSETXATTR               (export "LINUX_SYS_X64_LSETXATTR") i32 (i32.const 189))
  (global $LINUX_SYS_X64_FSETXATTR               (export "LINUX_SYS_X64_FSETXATTR") i32 (i32.const 190))
  (global $LINUX_SYS_X64_GETXATTR                (export "LINUX_SYS_X64_GETXATTR") i32 (i32.const 191))
  (global $LINUX_SYS_X64_LGETXATTR               (export "LINUX_SYS_X64_LGETXATTR") i32 (i32.const 192))
  (global $LINUX_SYS_X64_FGETXATTR               (export "LINUX_SYS_X64_FGETXATTR") i32 (i32.const 193))
  (global $LINUX_SYS_X64_LISTXATTR               (export "LINUX_SYS_X64_LISTXATTR") i32 (i32.const 194))
  (global $LINUX_SYS_X64_LLISTXATTR              (export "LINUX_SYS_X64_LLISTXATTR") i32 (i32.const 195))
  (global $LINUX_SYS_X64_FLISTXATTR              (export "LINUX_SYS_X64_FLISTXATTR") i32 (i32.const 196))
  (global $LINUX_SYS_X64_REMOVEXATTR             (export "LINUX_SYS_X64_REMOVEXATTR") i32 (i32.const 197))
  (global $LINUX_SYS_X64_LREMOVEXATTR            (export "LINUX_SYS_X64_LREMOVEXATTR") i32 (i32.const 198))
  (global $LINUX_SYS_X64_FREMOVEXATTR            (export "LINUX_SYS_X64_FREMOVEXATTR") i32 (i32.const 199))
  (global $LINUX_SYS_X64_TKILL                   (export "LINUX_SYS_X64_TKILL") i32 (i32.const 200))
  (global $LINUX_SYS_X64_TIME                    (export "LINUX_SYS_X64_TIME") i32 (i32.const 201))
  (global $LINUX_SYS_X64_FUTEX                   (export "LINUX_SYS_X64_FUTEX") i32 (i32.const 202))
  (global $LINUX_SYS_X64_SCHED_SETAFFINITY       (export "LINUX_SYS_X64_SCHED_SETAFFINITY") i32 (i32.const 203))
  (global $LINUX_SYS_X64_SCHED_GETAFFINITY       (export "LINUX_SYS_X64_SCHED_GETAFFINITY") i32 (i32.const 204))
  (global $LINUX_SYS_X64_SET_THREAD_AREA         (export "LINUX_SYS_X64_SET_THREAD_AREA") i32 (i32.const 205))
  (global $LINUX_SYS_X64_IO_SETUP                (export "LINUX_SYS_X64_IO_SETUP") i32 (i32.const 206))
  (global $LINUX_SYS_X64_IO_DESTROY              (export "LINUX_SYS_X64_IO_DESTROY") i32 (i32.const 207))
  (global $LINUX_SYS_X64_IO_GETEVENTS            (export "LINUX_SYS_X64_IO_GETEVENTS") i32 (i32.const 208))
  (global $LINUX_SYS_X64_IO_SUBMIT               (export "LINUX_SYS_X64_IO_SUBMIT") i32 (i32.const 209))
  (global $LINUX_SYS_X64_IO_CANCEL               (export "LINUX_SYS_X64_IO_CANCEL") i32 (i32.const 210))
  (global $LINUX_SYS_X64_GET_THREAD_AREA         (export "LINUX_SYS_X64_GET_THREAD_AREA") i32 (i32.const 211))
  (global $LINUX_SYS_X64_LOOKUP_DCOOKIE          (export "LINUX_SYS_X64_LOOKUP_DCOOKIE") i32 (i32.const 212))
  (global $LINUX_SYS_X64_EPOLL_CREATE            (export "LINUX_SYS_X64_EPOLL_CREATE") i32 (i32.const 213))
  (global $LINUX_SYS_X64_EPOLL_CTL_OLD           (export "LINUX_SYS_X64_EPOLL_CTL_OLD") i32 (i32.const 214))
  (global $LINUX_SYS_X64_EPOLL_WAIT_OLD          (export "LINUX_SYS_X64_EPOLL_WAIT_OLD") i32 (i32.const 215))
  (global $LINUX_SYS_X64_REMAP_FILE_PAGES        (export "LINUX_SYS_X64_REMAP_FILE_PAGES") i32 (i32.const 216))
  (global $LINUX_SYS_X64_GETDENTS64              (export "LINUX_SYS_X64_GETDENTS64") i32 (i32.const 217))
  (global $LINUX_SYS_X64_SET_TID_ADDRESS         (export "LINUX_SYS_X64_SET_TID_ADDRESS") i32 (i32.const 218))
  (global $LINUX_SYS_X64_RESTART_SYSCALL         (export "LINUX_SYS_X64_RESTART_SYSCALL") i32 (i32.const 219))
  (global $LINUX_SYS_X64_SEMTIMEDOP              (export "LINUX_SYS_X64_SEMTIMEDOP") i32 (i32.const 220))
  (global $LINUX_SYS_X64_FADVISE64               (export "LINUX_SYS_X64_FADVISE64") i32 (i32.const 221))
  (global $LINUX_SYS_X64_TIMER_CREATE            (export "LINUX_SYS_X64_TIMER_CREATE") i32 (i32.const 222))
  (global $LINUX_SYS_X64_TIMER_SETTIME           (export "LINUX_SYS_X64_TIMER_SETTIME") i32 (i32.const 223))
  (global $LINUX_SYS_X64_TIMER_GETTIME           (export "LINUX_SYS_X64_TIMER_GETTIME") i32 (i32.const 224))
  (global $LINUX_SYS_X64_TIMER_GETOVERRUN        (export "LINUX_SYS_X64_TIMER_GETOVERRUN") i32 (i32.const 225))
  (global $LINUX_SYS_X64_TIMER_DELETE            (export "LINUX_SYS_X64_TIMER_DELETE") i32 (i32.const 226))
  (global $LINUX_SYS_X64_CLOCK_SETTIME           (export "LINUX_SYS_X64_CLOCK_SETTIME") i32 (i32.const 227))
  (global $LINUX_SYS_X64_CLOCK_GETTIME           (export "LINUX_SYS_X64_CLOCK_GETTIME") i32 (i32.const 228))
  (global $LINUX_SYS_X64_CLOCK_GETRES            (export "LINUX_SYS_X64_CLOCK_GETRES") i32 (i32.const 229))
  (global $LINUX_SYS_X64_CLOCK_NANOSLEEP         (export "LINUX_SYS_X64_CLOCK_NANOSLEEP") i32 (i32.const 230))
  (global $LINUX_SYS_X64_EXIT_GROUP              (export "LINUX_SYS_X64_EXIT_GROUP") i32 (i32.const 231))
  (global $LINUX_SYS_X64_EPOLL_WAIT              (export "LINUX_SYS_X64_EPOLL_WAIT") i32 (i32.const 232))
  (global $LINUX_SYS_X64_EPOLL_CTL               (export "LINUX_SYS_X64_EPOLL_CTL") i32 (i32.const 233))
  (global $LINUX_SYS_X64_TGKILL                  (export "LINUX_SYS_X64_TGKILL") i32 (i32.const 234))
  (global $LINUX_SYS_X64_UTIMES                  (export "LINUX_SYS_X64_UTIMES") i32 (i32.const 235))
  (global $LINUX_SYS_X64_VSERVER                 (export "LINUX_SYS_X64_VSERVER") i32 (i32.const 236))
  (global $LINUX_SYS_X64_MBIND                   (export "LINUX_SYS_X64_MBIND") i32 (i32.const 237))
  (global $LINUX_SYS_X64_SET_MEMPOLICY           (export "LINUX_SYS_X64_SET_MEMPOLICY") i32 (i32.const 238))
  (global $LINUX_SYS_X64_GET_MEMPOLICY           (export "LINUX_SYS_X64_GET_MEMPOLICY") i32 (i32.const 239))
  (global $LINUX_SYS_X64_MQ_OPEN                 (export "LINUX_SYS_X64_MQ_OPEN") i32 (i32.const 240))
  (global $LINUX_SYS_X64_MQ_UNLINK               (export "LINUX_SYS_X64_MQ_UNLINK") i32 (i32.const 241))
  (global $LINUX_SYS_X64_MQ_TIMEDSEND            (export "LINUX_SYS_X64_MQ_TIMEDSEND") i32 (i32.const 242))
  (global $LINUX_SYS_X64_MQ_TIMEDRECEIVE         (export "LINUX_SYS_X64_MQ_TIMEDRECEIVE") i32 (i32.const 243))
  (global $LINUX_SYS_X64_MQ_NOTIFY               (export "LINUX_SYS_X64_MQ_NOTIFY") i32 (i32.const 244))
  (global $LINUX_SYS_X64_MQ_GETSETATTR           (export "LINUX_SYS_X64_MQ_GETSETATTR") i32 (i32.const 245))
  (global $LINUX_SYS_X64_KEXEC_LOAD              (export "LINUX_SYS_X64_KEXEC_LOAD") i32 (i32.const 246))
  (global $LINUX_SYS_X64_WAITID                  (export "LINUX_SYS_X64_WAITID") i32 (i32.const 247))
  (global $LINUX_SYS_X64_ADD_KEY                 (export "LINUX_SYS_X64_ADD_KEY") i32 (i32.const 248))
  (global $LINUX_SYS_X64_REQUEST_KEY             (export "LINUX_SYS_X64_REQUEST_KEY") i32 (i32.const 249))
  (global $LINUX_SYS_X64_KEYCTL                  (export "LINUX_SYS_X64_KEYCTL") i32 (i32.const 250))
  (global $LINUX_SYS_X64_IOPRIO_SET              (export "LINUX_SYS_X64_IOPRIO_SET") i32 (i32.const 251))
  (global $LINUX_SYS_X64_IOPRIO_GET              (export "LINUX_SYS_X64_IOPRIO_GET") i32 (i32.const 252))
  (global $LINUX_SYS_X64_INOTIFY_INIT            (export "LINUX_SYS_X64_INOTIFY_INIT") i32 (i32.const 253))
  (global $LINUX_SYS_X64_INOTIFY_ADD_WATCH       (export "LINUX_SYS_X64_INOTIFY_ADD_WATCH") i32 (i32.const 254))
  (global $LINUX_SYS_X64_INOTIFY_RM_WATCH        (export "LINUX_SYS_X64_INOTIFY_RM_WATCH") i32 (i32.const 255))
  (global $LINUX_SYS_X64_MIGRATE_PAGES           (export "LINUX_SYS_X64_MIGRATE_PAGES") i32 (i32.const 256))
  (global $LINUX_SYS_X64_OPENAT                  (export "LINUX_SYS_X64_OPENAT") i32 (i32.const 257))
  (global $LINUX_SYS_X64_MKDIRAT                 (export "LINUX_SYS_X64_MKDIRAT") i32 (i32.const 258))
  (global $LINUX_SYS_X64_MKNODAT                 (export "LINUX_SYS_X64_MKNODAT") i32 (i32.const 259))
  (global $LINUX_SYS_X64_FCHOWNAT                (export "LINUX_SYS_X64_FCHOWNAT") i32 (i32.const 260))
  (global $LINUX_SYS_X64_FUTIMESAT               (export "LINUX_SYS_X64_FUTIMESAT") i32 (i32.const 261))
  (global $LINUX_SYS_X64_NEWFSTATAT              (export "LINUX_SYS_X64_NEWFSTATAT") i32 (i32.const 262))
  (global $LINUX_SYS_X64_UNLINKAT                (export "LINUX_SYS_X64_UNLINKAT") i32 (i32.const 263))
  (global $LINUX_SYS_X64_RENAMEAT                (export "LINUX_SYS_X64_RENAMEAT") i32 (i32.const 264))
  (global $LINUX_SYS_X64_LINKAT                  (export "LINUX_SYS_X64_LINKAT") i32 (i32.const 265))
  (global $LINUX_SYS_X64_SYMLINKAT               (export "LINUX_SYS_X64_SYMLINKAT") i32 (i32.const 266))
  (global $LINUX_SYS_X64_READLINKAT              (export "LINUX_SYS_X64_READLINKAT") i32 (i32.const 267))
  (global $LINUX_SYS_X64_FCHMODAT                (export "LINUX_SYS_X64_FCHMODAT") i32 (i32.const 268))
  (global $LINUX_SYS_X64_FACCESSAT               (export "LINUX_SYS_X64_FACCESSAT") i32 (i32.const 269))
  (global $LINUX_SYS_X64_PSELECT6                (export "LINUX_SYS_X64_PSELECT6") i32 (i32.const 270))
  (global $LINUX_SYS_X64_PPOLL                   (export "LINUX_SYS_X64_PPOLL") i32 (i32.const 271))
  (global $LINUX_SYS_X64_UNSHARE                 (export "LINUX_SYS_X64_UNSHARE") i32 (i32.const 272))
  (global $LINUX_SYS_X64_SET_ROBUST_LIST         (export "LINUX_SYS_X64_SET_ROBUST_LIST") i32 (i32.const 273))
  (global $LINUX_SYS_X64_GET_ROBUST_LIST         (export "LINUX_SYS_X64_GET_ROBUST_LIST") i32 (i32.const 274))
  (global $LINUX_SYS_X64_SPLICE                  (export "LINUX_SYS_X64_SPLICE") i32 (i32.const 275))
  (global $LINUX_SYS_X64_TEE                     (export "LINUX_SYS_X64_TEE") i32 (i32.const 276))
  (global $LINUX_SYS_X64_SYNC_FILE_RANGE         (export "LINUX_SYS_X64_SYNC_FILE_RANGE") i32 (i32.const 277))
  (global $LINUX_SYS_X64_VMSPLICE                (export "LINUX_SYS_X64_VMSPLICE") i32 (i32.const 278))
  (global $LINUX_SYS_X64_MOVE_PAGES              (export "LINUX_SYS_X64_MOVE_PAGES") i32 (i32.const 279))
  (global $LINUX_SYS_X64_UTIMENSAT               (export "LINUX_SYS_X64_UTIMENSAT") i32 (i32.const 280))
  (global $LINUX_SYS_X64_EPOLL_PWAIT             (export "LINUX_SYS_X64_EPOLL_PWAIT") i32 (i32.const 281))
  (global $LINUX_SYS_X64_SIGNALFD                (export "LINUX_SYS_X64_SIGNALFD") i32 (i32.const 282))
  (global $LINUX_SYS_X64_TIMERFD_CREATE          (export "LINUX_SYS_X64_TIMERFD_CREATE") i32 (i32.const 283))
  (global $LINUX_SYS_X64_EVENTFD                 (export "LINUX_SYS_X64_EVENTFD") i32 (i32.const 284))
  (global $LINUX_SYS_X64_FALLOCATE               (export "LINUX_SYS_X64_FALLOCATE") i32 (i32.const 285))
  (global $LINUX_SYS_X64_TIMERFD_SETTIME         (export "LINUX_SYS_X64_TIMERFD_SETTIME") i32 (i32.const 286))
  (global $LINUX_SYS_X64_TIMERFD_GETTIME         (export "LINUX_SYS_X64_TIMERFD_GETTIME") i32 (i32.const 287))
  (global $LINUX_SYS_X64_ACCEPT4                 (export "LINUX_SYS_X64_ACCEPT4") i32 (i32.const 288))
  (global $LINUX_SYS_X64_SIGNALFD4               (export "LINUX_SYS_X64_SIGNALFD4") i32 (i32.const 289))
  (global $LINUX_SYS_X64_EVENTFD2                (export "LINUX_SYS_X64_EVENTFD2") i32 (i32.const 290))
  (global $LINUX_SYS_X64_EPOLL_CREATE1           (export "LINUX_SYS_X64_EPOLL_CREATE1") i32 (i32.const 291))
  (global $LINUX_SYS_X64_DUP3                    (export "LINUX_SYS_X64_DUP3") i32 (i32.const 292))
  (global $LINUX_SYS_X64_PIPE2                   (export "LINUX_SYS_X64_PIPE2") i32 (i32.const 293))
  (global $LINUX_SYS_X64_INOTIFY_INIT1           (export "LINUX_SYS_X64_INOTIFY_INIT1") i32 (i32.const 294))
  (global $LINUX_SYS_X64_PREADV                  (export "LINUX_SYS_X64_PREADV") i32 (i32.const 295))
  (global $LINUX_SYS_X64_PWRITEV                 (export "LINUX_SYS_X64_PWRITEV") i32 (i32.const 296))
  (global $LINUX_SYS_X64_RT_TGSIGQUEUEINFO       (export "LINUX_SYS_X64_RT_TGSIGQUEUEINFO") i32 (i32.const 297))
  (global $LINUX_SYS_X64_PERF_EVENT_OPEN         (export "LINUX_SYS_X64_PERF_EVENT_OPEN") i32 (i32.const 298))
  (global $LINUX_SYS_X64_RECVMMSG                (export "LINUX_SYS_X64_RECVMMSG") i32 (i32.const 299))
  (global $LINUX_SYS_X64_FANOTIFY_INIT           (export "LINUX_SYS_X64_FANOTIFY_INIT") i32 (i32.const 300))
  (global $LINUX_SYS_X64_FANOTIFY_MARK           (export "LINUX_SYS_X64_FANOTIFY_MARK") i32 (i32.const 301))
  (global $LINUX_SYS_X64_PRLIMIT64               (export "LINUX_SYS_X64_PRLIMIT64") i32 (i32.const 302))
  (global $LINUX_SYS_X64_NAME_TO_HANDLE_AT       (export "LINUX_SYS_X64_NAME_TO_HANDLE_AT") i32 (i32.const 303))
  (global $LINUX_SYS_X64_OPEN_BY_HANDLE_AT       (export "LINUX_SYS_X64_OPEN_BY_HANDLE_AT") i32 (i32.const 304))
  (global $LINUX_SYS_X64_CLOCK_ADJTIME           (export "LINUX_SYS_X64_CLOCK_ADJTIME") i32 (i32.const 305))
  (global $LINUX_SYS_X64_SYNCFS                  (export "LINUX_SYS_X64_SYNCFS") i32 (i32.const 306))
  (global $LINUX_SYS_X64_SENDMMSG                (export "LINUX_SYS_X64_SENDMMSG") i32 (i32.const 307))
  (global $LINUX_SYS_X64_SETNS                   (export "LINUX_SYS_X64_SETNS") i32 (i32.const 308))
  (global $LINUX_SYS_X64_GETCPU                  (export "LINUX_SYS_X64_GETCPU") i32 (i32.const 309))
  (global $LINUX_SYS_X64_PROCESS_VM_READV        (export "LINUX_SYS_X64_PROCESS_VM_READV") i32 (i32.const 310))
  (global $LINUX_SYS_X64_PROCESS_VM_WRITEV       (export "LINUX_SYS_X64_PROCESS_VM_WRITEV") i32 (i32.const 311))
  (global $LINUX_SYS_X64_KCMP                    (export "LINUX_SYS_X64_KCMP") i32 (i32.const 312))
  (global $LINUX_SYS_X64_FINIT_MODULE            (export "LINUX_SYS_X64_FINIT_MODULE") i32 (i32.const 313))
  (global $LINUX_SYS_X64_SCHED_SETATTR           (export "LINUX_SYS_X64_SCHED_SETATTR") i32 (i32.const 314))
  (global $LINUX_SYS_X64_SCHED_GETATTR           (export "LINUX_SYS_X64_SCHED_GETATTR") i32 (i32.const 315))
  (global $LINUX_SYS_X64_RENAMEAT2               (export "LINUX_SYS_X64_RENAMEAT2") i32 (i32.const 316))
  (global $LINUX_SYS_X64_SECCOMP                 (export "LINUX_SYS_X64_SECCOMP") i32 (i32.const 317))
  (global $LINUX_SYS_X64_GETRANDOM               (export "LINUX_SYS_X64_GETRANDOM") i32 (i32.const 318))
  (global $LINUX_SYS_X64_MEMFD_CREATE            (export "LINUX_SYS_X64_MEMFD_CREATE") i32 (i32.const 319))
  (global $LINUX_SYS_X64_KEXEC_FILE_LOAD         (export "LINUX_SYS_X64_KEXEC_FILE_LOAD") i32 (i32.const 320))
  (global $LINUX_SYS_X64_BPF                     (export "LINUX_SYS_X64_BPF") i32 (i32.const 321))
  (global $LINUX_SYS_X64_EXECVEAT                (export "LINUX_SYS_X64_EXECVEAT") i32 (i32.const 322))
  (global $LINUX_SYS_X64_USERFAULTFD             (export "LINUX_SYS_X64_USERFAULTFD") i32 (i32.const 323))
  (global $LINUX_SYS_X64_MEMBARRIER              (export "LINUX_SYS_X64_MEMBARRIER") i32 (i32.const 324))
  (global $LINUX_SYS_X64_MLOCK2                  (export "LINUX_SYS_X64_MLOCK2") i32 (i32.const 325))
  (global $LINUX_SYS_X64_COPY_FILE_RANGE         (export "LINUX_SYS_X64_COPY_FILE_RANGE") i32 (i32.const 326))
  (global $LINUX_SYS_X64_PREADV2                 (export "LINUX_SYS_X64_PREADV2") i32 (i32.const 327))
  (global $LINUX_SYS_X64_PWRITEV2                (export "LINUX_SYS_X64_PWRITEV2") i32 (i32.const 328))
  (global $LINUX_SYS_X64_PKEY_MPROTECT           (export "LINUX_SYS_X64_PKEY_MPROTECT") i32 (i32.const 329))
  (global $LINUX_SYS_X64_PKEY_ALLOC              (export "LINUX_SYS_X64_PKEY_ALLOC") i32 (i32.const 330))
  (global $LINUX_SYS_X64_PKEY_FREE               (export "LINUX_SYS_X64_PKEY_FREE") i32 (i32.const 331))
  (global $LINUX_SYS_X64_STATX                   (export "LINUX_SYS_X64_STATX") i32 (i32.const 332))
  (global $LINUX_SYS_X64_IO_PGETEVENTS           (export "LINUX_SYS_X64_IO_PGETEVENTS") i32 (i32.const 333))
  (global $LINUX_SYS_X64_RSEQ                    (export "LINUX_SYS_X64_RSEQ") i32 (i32.const 334))
  (global $LINUX_SYS_X64_URETPROBE               (export "LINUX_SYS_X64_URETPROBE") i32 (i32.const 335))
  (global $LINUX_SYS_X64_UPROBE                  (export "LINUX_SYS_X64_UPROBE") i32 (i32.const 336))
  (global $LINUX_SYS_X64_PIDFD_SEND_SIGNAL       (export "LINUX_SYS_X64_PIDFD_SEND_SIGNAL") i32 (i32.const 424))
  (global $LINUX_SYS_X64_IO_URING_SETUP          (export "LINUX_SYS_X64_IO_URING_SETUP") i32 (i32.const 425))
  (global $LINUX_SYS_X64_IO_URING_ENTER          (export "LINUX_SYS_X64_IO_URING_ENTER") i32 (i32.const 426))
  (global $LINUX_SYS_X64_IO_URING_REGISTER       (export "LINUX_SYS_X64_IO_URING_REGISTER") i32 (i32.const 427))
  (global $LINUX_SYS_X64_OPEN_TREE               (export "LINUX_SYS_X64_OPEN_TREE") i32 (i32.const 428))
  (global $LINUX_SYS_X64_MOVE_MOUNT              (export "LINUX_SYS_X64_MOVE_MOUNT") i32 (i32.const 429))
  (global $LINUX_SYS_X64_FSOPEN                  (export "LINUX_SYS_X64_FSOPEN") i32 (i32.const 430))
  (global $LINUX_SYS_X64_FSCONFIG                (export "LINUX_SYS_X64_FSCONFIG") i32 (i32.const 431))
  (global $LINUX_SYS_X64_FSMOUNT                 (export "LINUX_SYS_X64_FSMOUNT") i32 (i32.const 432))
  (global $LINUX_SYS_X64_FSPICK                  (export "LINUX_SYS_X64_FSPICK") i32 (i32.const 433))
  (global $LINUX_SYS_X64_PIDFD_OPEN              (export "LINUX_SYS_X64_PIDFD_OPEN") i32 (i32.const 434))
  (global $LINUX_SYS_X64_CLONE3                  (export "LINUX_SYS_X64_CLONE3") i32 (i32.const 435))
  (global $LINUX_SYS_X64_CLOSE_RANGE             (export "LINUX_SYS_X64_CLOSE_RANGE") i32 (i32.const 436))
  (global $LINUX_SYS_X64_OPENAT2                 (export "LINUX_SYS_X64_OPENAT2") i32 (i32.const 437))
  (global $LINUX_SYS_X64_PIDFD_GETFD             (export "LINUX_SYS_X64_PIDFD_GETFD") i32 (i32.const 438))
  (global $LINUX_SYS_X64_FACCESSAT2              (export "LINUX_SYS_X64_FACCESSAT2") i32 (i32.const 439))
  (global $LINUX_SYS_X64_PROCESS_MADVISE         (export "LINUX_SYS_X64_PROCESS_MADVISE") i32 (i32.const 440))
  (global $LINUX_SYS_X64_EPOLL_PWAIT2            (export "LINUX_SYS_X64_EPOLL_PWAIT2") i32 (i32.const 441))
  (global $LINUX_SYS_X64_MOUNT_SETATTR           (export "LINUX_SYS_X64_MOUNT_SETATTR") i32 (i32.const 442))
  (global $LINUX_SYS_X64_QUOTACTL_FD             (export "LINUX_SYS_X64_QUOTACTL_FD") i32 (i32.const 443))
  (global $LINUX_SYS_X64_LANDLOCK_CREATE_RULESET (export "LINUX_SYS_X64_LANDLOCK_CREATE_RULESET") i32 (i32.const 444))
  (global $LINUX_SYS_X64_LANDLOCK_ADD_RULE       (export "LINUX_SYS_X64_LANDLOCK_ADD_RULE") i32 (i32.const 445))
  (global $LINUX_SYS_X64_LANDLOCK_RESTRICT_SELF  (export "LINUX_SYS_X64_LANDLOCK_RESTRICT_SELF") i32 (i32.const 446))
  (global $LINUX_SYS_X64_MEMFD_SECRET            (export "LINUX_SYS_X64_MEMFD_SECRET") i32 (i32.const 447))
  (global $LINUX_SYS_X64_PROCESS_MRELEASE        (export "LINUX_SYS_X64_PROCESS_MRELEASE") i32 (i32.const 448))
  (global $LINUX_SYS_X64_FUTEX_WAITV             (export "LINUX_SYS_X64_FUTEX_WAITV") i32 (i32.const 449))
  (global $LINUX_SYS_X64_SET_MEMPOLICY_HOME_NODE (export "LINUX_SYS_X64_SET_MEMPOLICY_HOME_NODE") i32 (i32.const 450))
  (global $LINUX_SYS_X64_CACHESTAT               (export "LINUX_SYS_X64_CACHESTAT") i32 (i32.const 451))
  (global $LINUX_SYS_X64_FCHMODAT2               (export "LINUX_SYS_X64_FCHMODAT2") i32 (i32.const 452))
  (global $LINUX_SYS_X64_MAP_SHADOW_STACK        (export "LINUX_SYS_X64_MAP_SHADOW_STACK") i32 (i32.const 453))
  (global $LINUX_SYS_X64_FUTEX_WAKE              (export "LINUX_SYS_X64_FUTEX_WAKE") i32 (i32.const 454))
  (global $LINUX_SYS_X64_FUTEX_WAIT              (export "LINUX_SYS_X64_FUTEX_WAIT") i32 (i32.const 455))
  (global $LINUX_SYS_X64_FUTEX_REQUEUE           (export "LINUX_SYS_X64_FUTEX_REQUEUE") i32 (i32.const 456))
  (global $LINUX_SYS_X64_STATMOUNT               (export "LINUX_SYS_X64_STATMOUNT") i32 (i32.const 457))
  (global $LINUX_SYS_X64_LISTMOUNT               (export "LINUX_SYS_X64_LISTMOUNT") i32 (i32.const 458))
  (global $LINUX_SYS_X64_LSM_GET_SELF_ATTR       (export "LINUX_SYS_X64_LSM_GET_SELF_ATTR") i32 (i32.const 459))
  (global $LINUX_SYS_X64_LSM_SET_SELF_ATTR       (export "LINUX_SYS_X64_LSM_SET_SELF_ATTR") i32 (i32.const 460))
  (global $LINUX_SYS_X64_LSM_LIST_MODULES        (export "LINUX_SYS_X64_LSM_LIST_MODULES") i32 (i32.const 461))
  (global $LINUX_SYS_X64_MSEAL                   (export "LINUX_SYS_X64_MSEAL") i32 (i32.const 462))
  (global $LINUX_SYS_X64_SETXATTRAT              (export "LINUX_SYS_X64_SETXATTRAT") i32 (i32.const 463))
  (global $LINUX_SYS_X64_GETXATTRAT              (export "LINUX_SYS_X64_GETXATTRAT") i32 (i32.const 464))
  (global $LINUX_SYS_X64_LISTXATTRAT             (export "LINUX_SYS_X64_LISTXATTRAT") i32 (i32.const 465))
  (global $LINUX_SYS_X64_REMOVEXATTRAT           (export "LINUX_SYS_X64_REMOVEXATTRAT") i32 (i32.const 466))
  (global $LINUX_SYS_X64_OPEN_TREE_ATTR          (export "LINUX_SYS_X64_OPEN_TREE_ATTR") i32 (i32.const 467))
  (global $LINUX_SYS_X64_FILE_GETATTR            (export "LINUX_SYS_X64_FILE_GETATTR") i32 (i32.const 468))
  (global $LINUX_SYS_X64_FILE_SETATTR            (export "LINUX_SYS_X64_FILE_SETATTR") i32 (i32.const 469))
  (global $LINUX_SYS_X64_LISTNS                  (export "LINUX_SYS_X64_LISTNS") i32 (i32.const 470))
  (global $LINUX_SYS_X64_RSEQ_SLICE_YIELD        (export "LINUX_SYS_X64_RSEQ_SLICE_YIELD") i32 (i32.const 471))

  ;; ── Linux aarch64 syscall numbers (only where they differ from x86_64) ──
  (global $LINUX_SYS_AARCH64_EXIT       (export "LINUX_SYS_AARCH64_EXIT")       i32 (i32.const 93))
  (global $LINUX_SYS_AARCH64_CLONE      (export "LINUX_SYS_AARCH64_CLONE")      i32 (i32.const 220))
  (global $LINUX_SYS_AARCH64_EXECVE     (export "LINUX_SYS_AARCH64_EXECVE")     i32 (i32.const 221))
  (global $LINUX_SYS_AARCH64_MMAP       (export "LINUX_SYS_AARCH64_MMAP")       i32 (i32.const 222))
  (global $LINUX_SYS_AARCH64_MOUNT      (export "LINUX_SYS_AARCH64_MOUNT")      i32 (i32.const 40))
  (global $LINUX_SYS_AARCH64_UMOUNT2    (export "LINUX_SYS_AARCH64_UMOUNT2")    i32 (i32.const 39))
  (global $LINUX_SYS_AARCH64_PIVOT_ROOT (export "LINUX_SYS_AARCH64_PIVOT_ROOT") i32 (i32.const 41))
  (global $LINUX_SYS_AARCH64_UNSHARE    (export "LINUX_SYS_AARCH64_UNSHARE")    i32 (i32.const 97))
  (global $LINUX_SYS_AARCH64_SETNS      (export "LINUX_SYS_AARCH64_SETNS")      i32 (i32.const 268))
  (global $LINUX_SYS_AARCH64_SETHOSTNAME    (export "LINUX_SYS_AARCH64_SETHOSTNAME")    i32 (i32.const 161))
  (global $LINUX_SYS_AARCH64_SETDOMAINNAME  (export "LINUX_SYS_AARCH64_SETDOMAINNAME")  i32 (i32.const 162))
  (global $LINUX_SYS_AARCH64_SECCOMP    (export "LINUX_SYS_AARCH64_SECCOMP")    i32 (i32.const 277))
  (global $LINUX_SYS_AARCH64_CAPSET     (export "LINUX_SYS_AARCH64_CAPSET")     i32 (i32.const 91))
  (global $LINUX_SYS_AARCH64_PRCTL      (export "LINUX_SYS_AARCH64_PRCTL")      i32 (i32.const 167))
  (global $LINUX_SYS_AARCH64_SETUID     (export "LINUX_SYS_AARCH64_SETUID")     i32 (i32.const 146))
  (global $LINUX_SYS_AARCH64_SETGID     (export "LINUX_SYS_AARCH64_SETGID")     i32 (i32.const 144))
  (global $LINUX_SYS_AARCH64_SETGROUPS  (export "LINUX_SYS_AARCH64_SETGROUPS")  i32 (i32.const 159))
  (global $LINUX_SYS_AARCH64_PRLIMIT64  (export "LINUX_SYS_AARCH64_PRLIMIT64")  i32 (i32.const 261))
  (global $LINUX_SYS_AARCH64_SCHED_SETATTR (export "LINUX_SYS_AARCH64_SCHED_SETATTR") i32 (i32.const 274))
  (global $LINUX_SYS_AARCH64_IOPRIO_SET (export "LINUX_SYS_AARCH64_IOPRIO_SET") i32 (i32.const 30))
  (global $LINUX_SYS_AARCH64_UMASK      (export "LINUX_SYS_AARCH64_UMASK")      i32 (i32.const 166))
  (global $LINUX_SYS_AARCH64_KILL       (export "LINUX_SYS_AARCH64_KILL")       i32 (i32.const 129))
  (global $LINUX_SYS_AARCH64_WAIT4      (export "LINUX_SYS_AARCH64_WAIT4")      i32 (i32.const 260))
  (global $LINUX_SYS_AARCH64_BPF        (export "LINUX_SYS_AARCH64_BPF")        i32 (i32.const 280))
  (global $LINUX_SYS_AARCH64_OPEN_TREE  (export "LINUX_SYS_AARCH64_OPEN_TREE")  i32 (i32.const 428))
  (global $LINUX_SYS_AARCH64_MOVE_MOUNT (export "LINUX_SYS_AARCH64_MOVE_MOUNT") i32 (i32.const 429))
  (global $LINUX_SYS_AARCH64_MOUNT_SETATTR (export "LINUX_SYS_AARCH64_MOUNT_SETATTR") i32 (i32.const 442))

  ;; ── Linux ARM32 (EABI) syscall numbers (only where they differ) ──
  (global $LINUX_SYS_ARM32_EXIT         (export "LINUX_SYS_ARM32_EXIT")         i32 (i32.const 1))

  ;; ── Bounds check ──
  ;; Returns 1 if offset + need <= len, 0 otherwise.
  (func $bounds_check (export "bounds_check")
    (param $len i32) (param $offset i32) (param $need i32) (result i32)
    (if (result i32)
      (i32.lt_u (local.get $len) (local.get $need))
      (then (i32.const 0))
      (else
        (i32.le_u
          (local.get $offset)
          (i32.sub (local.get $len) (local.get $need))))))

  ;; ── Shared big-endian read helpers (returns i64: status<<32 | value) ──

  (func $read_u16_be (export "read_u16_be")
    (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $bounds_check (local.get $len) (local.get $offset) (i32.const 2))
      (then
        (call $pack (i32.const 0)
          (i32.or
            (i32.shl (i32.load8_u (i32.add (local.get $ptr) (local.get $offset))) (i32.const 8))
            (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 1)))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func $read_u24_be (export "read_u24_be")
    (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $bounds_check (local.get $len) (local.get $offset) (i32.const 3))
      (then
        (call $pack (i32.const 0)
          (i32.or
            (i32.or
              (i32.shl (i32.load8_u (i32.add (local.get $ptr) (local.get $offset))) (i32.const 16))
              (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 1)))) (i32.const 8)))
            (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 2)))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func $read_u32_be (export "read_u32_be")
    (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $bounds_check (local.get $len) (local.get $offset) (i32.const 4))
      (then
        (call $pack (i32.const 0)
          (i32.or
            (i32.or
              (i32.or
                (i32.shl (i32.load8_u (i32.add (local.get $ptr) (local.get $offset))) (i32.const 24))
                (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 1)))) (i32.const 16)))
              (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 2)))) (i32.const 8)))
            (i32.load8_u (i32.add (local.get $ptr) (i32.add (local.get $offset) (i32.const 3)))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  ;; ── LEB128 decoders ──

  (func $read_leb128_u (export "read_leb128_u")
    (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (local $pos i32) (local $val i64) (local $b i32) (local $shift i32)
    (local.set $pos (local.get $offset))
    (local.set $val (i64.const 0))
    (local.set $shift (i32.const 0))
    (block $done
      (loop $loop
        (if (i32.ge_u (local.get $pos) (local.get $len))
          (then
            (return (call $pack (i32.const 1) (i32.wrap_i64 (local.get $val))))))
        (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (local.set $val
          (i64.or (local.get $val)
            (i64.shl (i64.extend_i32_u (i32.and (local.get $b) (i32.const 0x7f))) (i64.extend_i32_u (local.get $shift)))))
        (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80)))
          (then
            (return (call $pack (i32.const 0)
              (i32.or (local.get $pos) (i32.shl (i32.wrap_i64 (local.get $val)) (i32.const 8)))))))
        (br $loop)))
    (call $pack (i32.const 5) (i32.const 0)))

  (func $read_leb128_s (export "read_leb128_s")
    (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (local $pos i32) (local $val i64) (local $b i32) (local $shift i32)
    (local.set $pos (local.get $offset))
    (local.set $val (i64.const 0))
    (local.set $shift (i32.const 0))
    (block $done
      (loop $loop
        (if (i32.ge_u (local.get $pos) (local.get $len))
          (then
            (return (call $pack (i32.const 1) (i32.wrap_i64 (local.get $val))))))
        (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (local.set $val
          (i64.or (local.get $val)
            (i64.shl (i64.extend_i32_u (i32.and (local.get $b) (i32.const 0x7f))) (i64.extend_i32_u (local.get $shift)))))
        (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80)))
          (then
            (if (i32.and (local.get $b) (i32.const 0x40))
              (then
                (local.set $val
                  (i64.or (local.get $val)
                    (i64.shl (i64.const -1) (i64.extend_i32_u (local.get $shift)))))))
            (return (call $pack (i32.const 0)
              (i32.or (local.get $pos) (i32.shl (i32.wrap_i64 (local.get $val)) (i32.const 8)))))))
        (br $loop)))
    (call $pack (i32.const 5) (i32.const 0)))

  ;; ── Protocol exports ──

  (func $proto_abi_version (export "proto_abi_version") (result i32)
    (i32.const 2))

  (func $proto_standard_id (export "proto_standard_id") (result i32)
    (i32.const 0))

  (func $simd_capabilities (export "simd_capabilities") (result i32)
    (i32.const 1))

  ;; Base64url encoding (RFC 4648 §5) — no padding, -_ alphabet.

    ;; Standard ID removed — merged into single module

  (func $m86b64url_char (param $n i32) (result i32)
    local.get $n
    i32.const 26
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 65
      i32.add
    else
      local.get $n
      i32.const 52
      i32.lt_u
      if (result i32)
        local.get $n
        i32.const 26
        i32.sub
        i32.const 97
        i32.add
      else
        local.get $n
        i32.const 62
        i32.lt_u
        if (result i32)
          local.get $n
          i32.const 52
          i32.sub
          i32.const 48
          i32.add
        else
          local.get $n
          i32.const 62
          i32.eq
          if (result i32)
            i32.const 45           ;; '-' instead of '+'
          else
            i32.const 95           ;; '_' instead of '/'
          end
        end
      end
    end)

  ;; base64url_encode(src_ptr, src_len, dst_ptr, dcap) -> status:i32, written:i32 packed as i64
  ;; No padding. Returns (0,written) on success, (negative,0) on error.
  (func $b64_encode (export "base64url_encode") (param $src i32) (param $slen i32) (param $dst i32) (param $dcap i32) (result i64)
    (local $i i32)
    (local $o i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $b2 i32)
    (local $triple i32)

    ;; estimate output: ((slen + 2) / 3) * 4
    local.get $slen
    i32.const 2
    i32.add
    i32.const 3
    i32.div_u
    i32.const 4
    i32.mul
    local.set $o
    local.get $dcap
    local.get $o
    i32.lt_u
    if
      i64.const -1
      return
    end

    i32.const 0
    local.set $o

    block $done
    loop $main
      local.get $i
      local.get $slen
      i32.ge_u
      br_if $done

      ;; load 3 bytes
      local.get $src
      local.get $i
      i32.add
      i32.load8_u
      local.set $b0
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      local.get $slen
      i32.ge_u
      if
        ;; 1 byte left → 2 chars
        local.get $b0
        i32.const 2
        i32.shl
        i32.const 255
        i32.and
        call $m86b64url_char
        local.set $b2
        local.get $dst
        local.get $o
        i32.add
        local.get $b2
        i32.store8
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        local.get $b0
        i32.const 4
        i32.shr_u
        i32.const 15
        i32.and
        call $m86b64url_char
        local.set $b2
        local.get $dst
        local.get $o
        i32.add
        local.get $b2
        i32.store8
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        br $done
      end

      local.get $src
      local.get $i
      i32.add
      i32.load8_u
      local.set $b1
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      local.get $slen
      i32.ge_u
      if
        ;; 2 bytes → 3 chars
        local.get $b0
        i32.const 2
        i32.shl
        local.get $b1
        i32.const 6
        i32.shr_u
        i32.or
        i32.const 255
        i32.and
        call $m86b64url_char
        local.set $b2
        local.get $dst
        local.get $o
        i32.add
        local.get $b2
        i32.store8
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        local.get $b1
        i32.const 2
        i32.shl
        i32.const 63
        i32.and
        call $m86b64url_char
        local.set $b2
        local.get $dst
        local.get $o
        i32.add
        local.get $b2
        i32.store8
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        local.get $b1
        i32.const 4
        i32.shr_u
        i32.const 15
        i32.and
        call $m86b64url_char
        local.set $b2
        local.get $dst
        local.get $o
        i32.add
        local.get $b2
        i32.store8
        local.get $o
        i32.const 1
        i32.add
        local.set $o
        br $done
      end

      ;; 3 bytes → 4 chars
      local.get $src
      local.get $i
      i32.add
      i32.load8_u
      local.set $b2
      local.get $i
      i32.const 1
      i32.add
      local.set $i

      local.get $b0
      i32.const 16
      i32.shl
      local.get $b1
      i32.const 8
      i32.shl
      i32.or
      local.get $b2
      i32.or
      local.set $triple

      local.get $triple
      i32.const 18
      i32.shr_u
      i32.const 63
      i32.and
      call $m86b64url_char
      local.set $b0
      local.get $dst
      local.get $o
      i32.add
      local.get $b0
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      local.get $triple
      i32.const 12
      i32.shr_u
      i32.const 63
      i32.and
      call $m86b64url_char
      local.set $b0
      local.get $dst
      local.get $o
      i32.add
      local.get $b0
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      local.get $triple
      i32.const 6
      i32.shr_u
      i32.const 63
      i32.and
      call $m86b64url_char
      local.set $b0
      local.get $dst
      local.get $o
      i32.add
      local.get $b0
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      local.get $triple
      i32.const 63
      i32.and
      call $m86b64url_char
      local.set $b0
      local.get $dst
      local.get $o
      i32.add
      local.get $b0
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      br $main
    end
    end

    ;; result = status=0, written=o
    i32.const 0
    local.get $o
    call $pack)

  ;; base64url_decode(src_ptr, src_len, dst_ptr, dcap) -> status:i32, written:i32 packed as i64
  ;; Decodes base64url (no padding). Rejects invalid chars.
  (func $b64_decode (export "base64url_decode") (param $src i32) (param $slen i32) (param $dst i32) (param $dcap i32) (result i64)
    (local $i i32)
    (local $o i32)
    (local $c0 i32)
    (local $c1 i32)
    (local $c2 i32)
    (local $c3 i32)
    (local $quad i32)

    ;; estimate max output: (slen / 4) * 3
    local.get $slen
    i32.const 3
    i32.mul
    i32.const 2
    i32.shr_u
    local.set $o
    local.get $dcap
    local.get $o
    i32.lt_u
    if
      i64.const -2
      return
    end

    i32.const 0
    local.set $o

    block $done
    loop $main
      local.get $i
      i32.const 4
      i32.add
      local.tee $i
      local.get $slen
      i32.gt_u
      br_if $done

      ;; decode 4 chars
      local.get $src
      local.get $i
      i32.const 4
      i32.sub
      i32.add
      i32.load8_u
      call $m86b64url_value
      local.tee $c0
      i32.const 0
      i32.lt_s
      br_if $done

      local.get $src
      local.get $i
      i32.const 3
      i32.sub
      i32.add
      i32.load8_u
      call $m86b64url_value
      local.tee $c1
      i32.const 0
      i32.lt_s
      br_if $done

      local.get $src
      local.get $i
      i32.const 2
      i32.sub
      i32.add
      i32.load8_u
      call $m86b64url_value
      local.tee $c2
      i32.const 0
      i32.lt_s
      br_if $done

      local.get $src
      local.get $i
      i32.const 1
      i32.sub
      i32.add
      i32.load8_u
      call $m86b64url_value
      local.tee $c3
      i32.const 0
      i32.lt_s
      br_if $done

      ;; reconstruct 4*6 = 24 bits → 3 bytes
      local.get $c0
      i32.const 18
      i32.shl
      local.get $c1
      i32.const 12
      i32.shl
      i32.or
      local.get $c2
      i32.const 6
      i32.shl
      i32.or
      local.get $c3
      i32.or
      local.set $quad

      local.get $dst
      local.get $o
      i32.add
      local.get $quad
      i32.const 16
      i32.shr_u
      i32.const 255
      i32.and
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      local.get $dst
      local.get $o
      i32.add
      local.get $quad
      i32.const 8
      i32.shr_u
      i32.const 255
      i32.and
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      local.get $dst
      local.get $o
      i32.add
      local.get $quad
      i32.const 255
      i32.and
      i32.store8
      local.get $o
      i32.const 1
      i32.add
      local.set $o

      br $main
    end
    end

    i32.const 0
    local.get $o
    call $pack)

  (func $m86b64url_value (param $c i32) (result i32)
    local.get $c
    i32.const 65
    i32.ge_u
    local.get $c
    i32.const 90
    i32.le_u
    i32.and
    if (result i32)
      local.get $c
      i32.const 65
      i32.sub
    else
      local.get $c
      i32.const 97
      i32.ge_u
      local.get $c
      i32.const 122
      i32.le_u
      i32.and
      if (result i32)
        local.get $c
        i32.const 97
        i32.sub
        i32.const 26
        i32.add
      else
        local.get $c
        i32.const 48
        i32.ge_u
        local.get $c
        i32.const 57
        i32.le_u
        i32.and
        if (result i32)
          local.get $c
          i32.const 48
          i32.sub
          i32.const 52
          i32.add
        else
          local.get $c
          i32.const 45           ;; '-'
          i32.eq
          if (result i32)
            i32.const 62
          else
            local.get $c
          i32.const 95         ;; '_'
          i32.eq
          if (result i32)
            i32.const 63
          else
            i32.const -1
          end
        end
      end
    end
    end
  )
(func $m85b64_char (param $n i32) (result i32)
    local.get $n
    i32.const 26
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 65
      i32.add
    else
      local.get $n
      i32.const 52
      i32.lt_u
      if (result i32)
        local.get $n
        i32.const 26
        i32.sub
        i32.const 97
        i32.add
      else
        local.get $n
        i32.const 62
        i32.lt_u
        if (result i32)
          local.get $n
          i32.const 52
          i32.sub
          i32.const 48
          i32.add
        else
          local.get $n
          i32.const 62
          i32.eq
          if (result i32)
            i32.const 43
          else
            i32.const 47
          end
        end
      end
    end)

  (func $m85b64_value (param $c i32) (result i32)
    local.get $c
    i32.const 65
    i32.ge_u
    local.get $c
    i32.const 90
    i32.le_u
    i32.and
    if (result i32)
      local.get $c
      i32.const 65
      i32.sub
    else
      local.get $c
      i32.const 97
      i32.ge_u
      local.get $c
      i32.const 122
      i32.le_u
      i32.and
      if (result i32)
        local.get $c
        i32.const 97
        i32.sub
        i32.const 26
        i32.add
      else
        local.get $c
        i32.const 48
        i32.ge_u
        local.get $c
        i32.const 57
        i32.le_u
        i32.and
        if (result i32)
          local.get $c
          i32.const 48
          i32.sub
          i32.const 52
          i32.add
        else
          local.get $c
          i32.const 43
          i32.eq
          if (result i32)
            i32.const 62
          else
            local.get $c
            i32.const 47
            i32.eq
            if (result i32)
              i32.const 63
            else
              i32.const -1
            end
          end
        end
      end
    end)

  (func $base64_encode (export "base64_standard_encode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $groups i32)
    (local $rem i32)
    (local $out_len i32)
    (local $i i32)
    (local $j i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $b2 i32)
    local.get $in_len
    i32.const 3
    i32.div_u
    local.set $groups
    local.get $in_len
    i32.const 3
    i32.rem_u
    local.set $rem
    local.get $groups
    i32.const 1073741823
    i32.gt_u
    if (result i64)
      i32.const 4
      i32.const 0
      call $pack
    else
      local.get $groups
      i32.const 1073741823
      i32.eq
      local.get $rem
      i32.const 0
      i32.ne
      i32.and
      if (result i64)
        i32.const 4
        i32.const 0
        call $pack
      else
        local.get $groups
        local.get $rem
        i32.const 0
        i32.ne
        i32.add
        i32.const 2
        i32.shl
        local.set $out_len
        local.get $out_len
        local.get $out_cap
        i32.gt_u
        if (result i64)
          i32.const 2
          i32.const 0
          call $pack
        else
          loop $loop
            local.get $i
            local.get $groups
            i32.lt_u
            if
              local.get $in_ptr
              local.get $i
              i32.const 3
              i32.mul
              i32.add
              i32.load8_u
              local.set $b0
              local.get $in_ptr
              local.get $i
              i32.const 3
              i32.mul
              i32.add
              i32.const 1
              i32.add
              i32.load8_u
              local.set $b1
              local.get $in_ptr
              local.get $i
              i32.const 3
              i32.mul
              i32.add
              i32.const 2
              i32.add
              i32.load8_u
              local.set $b2
              local.get $out_ptr
              local.get $j
              i32.add
              local.get $b0
              i32.const 2
              i32.shr_u
              call $m85b64_char
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 1
              i32.add
              local.get $b0
              i32.const 4
              i32.shl
              local.get $b1
              i32.const 4
              i32.shr_u
              i32.or
              i32.const 63
              i32.and
              call $m85b64_char
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 2
              i32.add
              local.get $b1
              i32.const 2
              i32.shl
              local.get $b2
              i32.const 6
              i32.shr_u
              i32.or
              i32.const 63
              i32.and
              call $m85b64_char
              i32.store8
              local.get $out_ptr
              local.get $j
              i32.add
              i32.const 3
              i32.add
              local.get $b2
              i32.const 63
              i32.and
              call $m85b64_char
              i32.store8
              local.get $i
              i32.const 1
              i32.add
              local.set $i
              local.get $j
              i32.const 4
              i32.add
              local.set $j
              br $loop
            end
          end
          local.get $rem
          i32.const 1
          i32.eq
          if
            local.get $in_ptr
            local.get $groups
            i32.const 3
            i32.mul
            i32.add
            i32.load8_u
            local.set $b0
            local.get $out_ptr
            local.get $j
            i32.add
            local.get $b0
            i32.const 2
            i32.shr_u
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 1
            i32.add
            local.get $b0
            i32.const 4
            i32.shl
            i32.const 63
            i32.and
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 2
            i32.add
            i32.const 61
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 3
            i32.add
            i32.const 61
            i32.store8
          end
          local.get $rem
          i32.const 2
          i32.eq
          if
            local.get $in_ptr
            local.get $groups
            i32.const 3
            i32.mul
            i32.add
            i32.load8_u
            local.set $b0
            local.get $in_ptr
            local.get $groups
            i32.const 3
            i32.mul
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            local.set $b1
            local.get $out_ptr
            local.get $j
            i32.add
            local.get $b0
            i32.const 2
            i32.shr_u
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 1
            i32.add
            local.get $b0
            i32.const 4
            i32.shl
            local.get $b1
            i32.const 4
            i32.shr_u
            i32.or
            i32.const 63
            i32.and
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 2
            i32.add
            local.get $b1
            i32.const 2
            i32.shl
            i32.const 63
            i32.and
            call $m85b64_char
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 3
            i32.add
            i32.const 61
            i32.store8
          end
          i32.const 0
          local.get $out_len
          call $pack
        end
      end
    end)

  (func (export "base64_standard_decode")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $groups i32)
    (local $full_groups i32)
    (local $pad i32)
    (local $out_len i32)
    (local $i i32)
    (local $j i32)
    (local $v0 i32)
    (local $v1 i32)
    (local $v2 i32)
    (local $v3 i32)
    local.get $in_len
    i32.const 3
    i32.and
    if (result i64)
      i32.const 3
      i32.const 0
      call $pack
    else
      local.get $in_len
      i32.const 4
      i32.div_u
      local.set $groups
      local.get $groups
      local.set $full_groups
      local.get $in_len
      if
        local.get $in_ptr
        local.get $in_len
        i32.add
        i32.const 1
        i32.sub
        i32.load8_u
        i32.const 61
        i32.eq
        if
          i32.const 1
          local.set $pad
          local.get $in_ptr
          local.get $in_len
          i32.add
          i32.const 2
          i32.sub
          i32.load8_u
          i32.const 61
          i32.eq
          if
            i32.const 2
            local.set $pad
          end
        end
      end
      local.get $pad
      if
        local.get $groups
        i32.const 0
        i32.eq
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $groups
        i32.const 1
        i32.sub
        local.set $full_groups
      end
      local.get $groups
      i32.const 3
      i32.mul
      local.get $pad
      i32.sub
      local.set $out_len
      local.get $out_len
      local.get $out_cap
      i32.gt_u
      if (result i64)
        i32.const 2
        i32.const 0
        call $pack
      else
        loop $loop
          local.get $i
          local.get $full_groups
          i32.lt_u
          if
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v0
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v1
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.const 2
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v2
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.const 3
            i32.add
            i32.load8_u
            call $m85b64_value
            local.tee $v3
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $out_ptr
            local.get $j
            i32.add
            local.get $v0
            i32.const 2
            i32.shl
            local.get $v1
            i32.const 4
            i32.shr_u
            i32.or
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 1
            i32.add
            local.get $v1
            i32.const 4
            i32.shl
            local.get $v2
            i32.const 2
            i32.shr_u
            i32.or
            i32.store8
            local.get $out_ptr
            local.get $j
            i32.add
            i32.const 2
            i32.add
            local.get $v2
            i32.const 6
            i32.shl
            local.get $v3
            i32.or
            i32.store8
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            local.get $j
            i32.const 3
            i32.add
            local.set $j
            br $loop
          end
        end
        local.get $pad
        i32.const 1
        i32.eq
        if
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v0
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v1
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 2
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v2
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $v2
          i32.const 3
          i32.and
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $out_ptr
          local.get $j
          i32.add
          local.get $v0
          i32.const 2
          i32.shl
          local.get $v1
          i32.const 4
          i32.shr_u
          i32.or
          i32.store8
          local.get $out_ptr
          local.get $j
          i32.add
          i32.const 1
          i32.add
          local.get $v1
          i32.const 4
          i32.shl
          local.get $v2
          i32.const 2
          i32.shr_u
          i32.or
          i32.store8
        end
        local.get $pad
        i32.const 2
        i32.eq
        if
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v0
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $in_ptr
          local.get $full_groups
          i32.const 4
          i32.mul
          i32.add
          i32.const 1
          i32.add
          i32.load8_u
          call $m85b64_value
          local.tee $v1
          i32.const 0
          i32.lt_s
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $v1
          i32.const 15
          i32.and
          if
            i32.const 3
            i32.const 0
            call $pack
            return
          end
          local.get $out_ptr
          local.get $j
          i32.add
          local.get $v0
          i32.const 2
          i32.shl
          local.get $v1
          i32.const 4
          i32.shr_u
          i32.or
          i32.store8
        end
        i32.const 0
        local.get $out_len
        call $pack
            end
    end)


;; ws-accept — WebSocket accept handshake (key + GUID → SHA-1 → base64)

  (data (i32.const 62024) "258EAFA5-E914-47DA-95CA-C5AB0DC85B11")

  (global $MSG_ADDR i32 (i32.const 62000))
  (global $DIGEST_ADDR i32 (i32.const 62100))

  (func $b64val (param $ch i32) (result i32)
    local.get $ch
    i32.const 65
    i32.ge_u
    local.get $ch
    i32.const 90
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 65
      i32.sub
      return
    end
    local.get $ch
    i32.const 97
    i32.ge_u
    local.get $ch
    i32.const 122
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 71
      i32.sub
      return
    end
    local.get $ch
    i32.const 48
    i32.ge_u
    local.get $ch
    i32.const 57
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 4
      i32.add
      return
    end
    local.get $ch
    i32.const 43
    i32.eq
    if
      i32.const 62
      return
    end
    local.get $ch
    i32.const 47
    i32.eq
    if
      i32.const 63
      return
    end
    i32.const -1)

  ;; base64 encode imported from encoding-base64 as $base64_encode

  (func (export "ws_accept_key") (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $i i32)
    (local $v i32)
    (local $out_i i32)
    (local $result i64)
    (local $digest i32)
    local.get $in_len
    i32.const 24
    i32.ne
    if
      (return (call $pack (i32.const 4) (i32.const 0)))
    end
    local.get $in_ptr
    i32.const 22
    i32.add
    i32.load8_u
    i32.const 61
    i32.ne
    local.get $in_ptr
    i32.const 23
    i32.add
    i32.load8_u
    i32.const 61
    i32.ne
    i32.or
    if
      (return (call $pack (i32.const 3) (i32.const 0)))
    end
    (loop $validate
      local.get $in_ptr
      local.get $i
      i32.add
      i32.load8_u
      call $b64val
      local.set $v
      local.get $v
      i32.const 0
      i32.lt_s
      if
        (return (call $pack (i32.const 3) (i32.const 0)))
      end
      local.get $i
      i32.const 21
      i32.eq
      local.get $v
      i32.const 15
      i32.and
      i32.const 0
      i32.ne
      i32.and
      if
        (return (call $pack (i32.const 3) (i32.const 0)))
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      local.get $i
      i32.const 22
      i32.lt_u
      br_if $validate)
    local.get $out_cap
    i32.const 28
    i32.lt_u
    if
      (return (call $pack (i32.const 2) (i32.const 0)))
    end
    ;; Assemble 60-byte message: key + GUID
    (local.set $i (i32.const 0))
    (loop $copy_key
      (i32.store8
        (i32.add (global.get $MSG_ADDR) (local.get $i))
        (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $copy_key (i32.lt_u (local.get $i) (i32.const 24))))
    ;; Compute SHA-1 of message
    (local.set $result
      (call $sha1 (global.get $MSG_ADDR) (i32.const 60) (global.get $DIGEST_ADDR)))
    (local.set $v (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $v)
      (then (return (call $pack (local.get $v) (i32.const 0)))))
    (local.set $digest (global.get $DIGEST_ADDR))
    ;; Base64-encode the 20-byte digest into 28-byte output (standard base64 with padding)
    local.get $digest i32.const 20 local.get $out_ptr local.get $out_cap
    call $base64_encode
    local.set $result
    local.get $result
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    if (result i64)
      local.get $result
    else
      (call $pack (i32.const 0) (i32.const 28))
    end)

(func $ec_has (param $len i32) (param $offset i32) (param $need i32) (result i32)
    (if (result i32)
      (i32.lt_u (local.get $len) (local.get $need))
      (then (i32.const 0))
      (else
        (i32.le_u
          (local.get $offset)
          (i32.sub (local.get $len) (local.get $need))))))

  (func (export "read_u16_le") (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $ec_has (local.get $len) (local.get $offset) (i32.const 2))
      (then
        (call $pack
          (i32.const 0)
          (i32.or
            (i32.load8_u (i32.add (local.get $ptr) (local.get $offset)))
            (i32.shl
              (i32.load8_u
                (i32.add
                  (local.get $ptr)
                  (i32.add (local.get $offset) (i32.const 1))))
              (i32.const 8)))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func (export "read_u32_le") (param $ptr i32) (param $len i32) (param $offset i32) (result i64)
    (if (result i64)
      (call $ec_has (local.get $len) (local.get $offset) (i32.const 4))
      (then
        (call $pack
          (i32.const 0)
          (i32.or
            (i32.or
              (i32.load8_u (i32.add (local.get $ptr) (local.get $offset)))
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 1))))
                (i32.const 8)))
            (i32.or
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 2))))
                (i32.const 16))
              (i32.shl
                (i32.load8_u
                  (i32.add
                    (local.get $ptr)
                    (i32.add (local.get $offset) (i32.const 3))))
                (i32.const 24))))))
      (else (call $pack (i32.const 1) (i32.const 0)))))

  (func (export "varint_encode_u64")
    (param $value_low i32) (param $value_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $value i64)
    (local $next i64)
    (local $written i32)
    (local $byte i32)
    (local.set $value
      (i64.or
        (i64.extend_i32_u (local.get $value_low))
        (i64.shl (i64.extend_i32_u (local.get $value_high)) (i64.const 32))))
    (loop $again
      (if (i32.ge_u (local.get $written) (local.get $out_cap))
        (then (return (call $pack (i32.const 2) (local.get $written)))))
      (local.set $next (i64.shr_u (local.get $value) (i64.const 7)))
      (local.set $byte (i32.and (i32.wrap_i64 (local.get $value)) (i32.const 127)))
      (if (i64.ne (local.get $next) (i64.const 0))
        (then (local.set $byte (i32.or (local.get $byte) (i32.const 128)))))
      (i32.store8
        (i32.add (local.get $out_ptr) (local.get $written))
        (local.get $byte))
      (local.set $written (i32.add (local.get $written) (i32.const 1)))
      (local.set $value (local.get $next))
      (br_if $again (i64.ne (local.get $value) (i64.const 0))))
    (call $pack (i32.const 0) (local.get $written)))

  (func (export "varint_decode_u64")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i64)
    (local $i i32)
    (local $shift i32)
    (local $b i32)
    (local $result i64)
    (if (i32.eqz (local.get $in_len))
      (then (return (call $pack (i32.const 1) (i32.const 0)))))
    (loop $again
      (if (i32.ge_u (local.get $i) (local.get $in_len))
        (then (return (call $pack (i32.const 5) (local.get $i)))))
      (if (i32.ge_u (local.get $i) (i32.const 10))
        (then (return (call $pack (i32.const 6) (local.get $i)))))
      (local.set $b (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
      (if
        (i32.and
          (i32.eq (local.get $shift) (i32.const 63))
          (i32.ne (i32.and (local.get $b) (i32.const 126)) (i32.const 0)))
        (then (return (call $pack (i32.const 4) (local.get $i)))))
      (local.set $result
        (i64.or
          (local.get $result)
          (i64.shl
            (i64.extend_i32_u (i32.and (local.get $b) (i32.const 127)))
            (i64.extend_i32_u (local.get $shift)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if (i32.eqz (i32.and (local.get $b) (i32.const 128)))
        (then
          (i64.store (local.get $out_ptr) (local.get $result))
          (return (call $pack (i32.const 0) (local.get $i)))))
      (if (i32.eq (local.get $i) (i32.const 10))
        (then (return (call $pack (i32.const 6) (local.get $i)))))
      (local.set $shift (i32.add (local.get $shift) (i32.const 7)))
      (br $again))
    (call $pack (i32.const 5) (local.get $i)))

  (func (export "quic_varint_encode_u64")
    (param $value_low i32) (param $value_high i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $value i64)
    (local.set $value
      (i64.or
        (i64.extend_i32_u (local.get $value_low))
        (i64.shl (i64.extend_i32_u (local.get $value_high)) (i64.const 32))))
    (if (i64.gt_u (local.get $value) (i64.const 4611686018427387903))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (if (i64.le_u (local.get $value) (i64.const 63))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 1))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8 (local.get $out_ptr) (i32.wrap_i64 (local.get $value)))
        (return (call $pack (i32.const 0) (i32.const 1)))))
    (if (i64.le_u (local.get $value) (i64.const 16383))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 2))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8
          (local.get $out_ptr)
          (i32.or
            (i32.const 64)
            (i32.and
              (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8)))
              (i32.const 63))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 1))
          (i32.wrap_i64 (local.get $value)))
        (return (call $pack (i32.const 0) (i32.const 2)))))
    (if (i64.le_u (local.get $value) (i64.const 1073741823))
      (then
        (if (i32.lt_u (local.get $out_cap) (i32.const 4))
          (then (return (call $pack (i32.const 2) (i32.const 0)))))
        (i32.store8
          (local.get $out_ptr)
          (i32.or
            (i32.const 128)
            (i32.and
              (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 24)))
              (i32.const 63))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 1))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 16))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 2))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8))))
        (i32.store8
          (i32.add (local.get $out_ptr) (i32.const 3))
          (i32.wrap_i64 (local.get $value)))
        (return (call $pack (i32.const 0) (i32.const 4)))))
    (if (i32.lt_u (local.get $out_cap) (i32.const 8))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (i32.store8
      (local.get $out_ptr)
      (i32.or
        (i32.const 192)
        (i32.and
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 56)))
          (i32.const 63))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 1))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 48))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 2))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 40))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 3))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 32))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 4))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 24))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 5))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 16))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 6))
      (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 8))))
    (i32.store8
      (i32.add (local.get $out_ptr) (i32.const 7))
      (i32.wrap_i64 (local.get $value)))
    (call $pack (i32.const 0) (i32.const 8)))

  (func (export "quic_varint_decode_u64")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i64)
    (local $first i32)
    (local $need i32)
    (local $i i32)
    (local $value i64)
    (if (i32.eqz (local.get $in_len))
      (then (return (call $pack (i32.const 1) (i32.const 0)))))
    (local.set $first (i32.load8_u (local.get $in_ptr)))
    (local.set $need
      (i32.shl
        (i32.const 1)
        (i32.shr_u (local.get $first) (i32.const 6))))
    (if (i32.lt_u (local.get $in_len) (local.get $need))
      (then (return (call $pack (i32.const 5) (i32.const 0)))))
    (local.set $value (i64.extend_i32_u (i32.and (local.get $first) (i32.const 63))))
    (local.set $i (i32.const 1))
    (loop $again
      (if (i32.lt_u (local.get $i) (local.get $need))
        (then
          (local.set $value
            (i64.or
              (i64.shl (local.get $value) (i64.const 8))
              (i64.extend_i32_u
                (i32.load8_u
                  (i32.add (local.get $in_ptr) (local.get $i))))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again))))
    (i64.store (local.get $out_ptr) (local.get $value))
    (call $pack (i32.const 0) (local.get $need)))

  (func (export "crc32") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $j i32)
    (local $crc i32)
    (local.set $crc (i32.const -1))
    (loop $bytes
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $crc
            (i32.xor
              (local.get $crc)
              (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
          (local.set $j (i32.const 0))
          (loop $bits
            (if (i32.lt_u (local.get $j) (i32.const 8))
              (then
                (if (i32.and (local.get $crc) (i32.const 1))
                  (then
                    (local.set $crc
                      (i32.xor
                        (i32.shr_u (local.get $crc) (i32.const 1))
                        (i32.const 0xedb88320))))
                  (else
                    (local.set $crc
                      (i32.shr_u (local.get $crc) (i32.const 1)))))
                (local.set $j (i32.add (local.get $j) (i32.const 1)))
                (br $bits))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $bytes))))
    (i32.xor (local.get $crc) (i32.const -1)))

  (func $adler32_update (export "adler32_update") (param $initial i32) (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $a i32)
    (local $b i32)
    (local.set $a (i32.and (local.get $initial) (i32.const 65535)))
    (local.set $b
      (i32.and
        (i32.shr_u (local.get $initial) (i32.const 16))
        (i32.const 65535)))
    (loop $bytes
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (local.set $a
            (i32.rem_u
              (i32.add
                (local.get $a)
                (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
              (i32.const 65521)))
          (local.set $b
            (i32.rem_u
              (i32.add (local.get $b) (local.get $a))
              (i32.const 65521)))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $bytes))))
    (i32.or
      (i32.shl (local.get $b) (i32.const 16))
      (local.get $a)))

  (func (export "adler32") (param $ptr i32) (param $len i32) (result i32)
    (call $adler32_update (i32.const 1) (local.get $ptr) (local.get $len)))

  (func (export "crc32_simd") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $j i32)
    (local $k i32)
    (local $crc i32)
    (local $word i32)
    (local.set $crc (i32.const -1))
    (loop $loop
      (if (i32.lt_u (local.get $i) (local.get $len))
        (then
          (if (i32.le_u (i32.add (local.get $i) (i32.const 4)) (local.get $len))
            (then
              (local.set $word
                (i32.load (i32.add (local.get $ptr) (local.get $i))))
              (local.set $k (i32.const 0))
              (loop $quad
                (if (i32.lt_u (local.get $k) (i32.const 4))
                  (then
                    (local.set $crc
                      (i32.xor
                        (local.get $crc)
                        (i32.and (local.get $word) (i32.const 0xff))))
                    (local.set $j (i32.const 0))
                    (loop $bits
                      (if (i32.lt_u (local.get $j) (i32.const 8))
                        (then
                          (if (i32.and (local.get $crc) (i32.const 1))
                            (then
                              (local.set $crc
                                (i32.xor
                                  (i32.shr_u (local.get $crc) (i32.const 1))
                                  (i32.const 0xedb88320))))
                            (else
                              (local.set $crc
                                (i32.shr_u (local.get $crc) (i32.const 1)))))
                          (local.set $j (i32.add (local.get $j) (i32.const 1)))
                          (br $bits))))
                    (local.set $word
                      (i32.shr_u (local.get $word) (i32.const 8)))
                    (local.set $k (i32.add (local.get $k) (i32.const 1)))
                    (br $quad))))
              (local.set $i (i32.add (local.get $i) (i32.const 4)))
              (br $loop))
            (else
              (local.set $crc
                (i32.xor
                  (local.get $crc)
                  (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
              (local.set $j (i32.const 0))
              (loop $bits_tail
                (if (i32.lt_u (local.get $j) (i32.const 8))
                  (then
                    (if (i32.and (local.get $crc) (i32.const 1))
                      (then
                        (local.set $crc
                          (i32.xor
                            (i32.shr_u (local.get $crc) (i32.const 1))
                            (i32.const 0xedb88320))))
                      (else
                        (local.set $crc
                          (i32.shr_u (local.get $crc) (i32.const 1)))))
                    (local.set $j (i32.add (local.get $j) (i32.const 1)))
                    (br $bits_tail))))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $loop))))))
    (i32.xor (local.get $crc) (i32.const -1)))

  (func (export "adler32_simd") (param $ptr i32) (param $len i32) (result i32)
    (call $adler32_update_simd (i32.const 1) (local.get $ptr) (local.get $len)))

  (func $adler32_update_simd (export "adler32_update_simd")
    (param $initial i32) (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $a i32)
    (local $b i32)
    (local $a0 i32)
    (local $sum i32)
    (local $wsum i32)
    (local $v v128)
    (local $b0 i32)
    (local $b1 i32)
    (local $b2 i32)
    (local $b3 i32)
    (local $b4 i32)
    (local $b5 i32)
    (local $b6 i32)
    (local $b7 i32)
    (local $b8 i32)
    (local $b9 i32)
    (local $b10 i32)
    (local $b11 i32)
    (local $b12 i32)
    (local $b13 i32)
    (local $b14 i32)
    (local $b15 i32)
    (local.set $a
      (i32.and (local.get $initial) (i32.const 65535)))
    (local.set $b
      (i32.and
        (i32.shr_u (local.get $initial) (i32.const 16))
        (i32.const 65535)))
    (loop $loop
      (if (i32.le_u (i32.add (local.get $i) (i32.const 16)) (local.get $len))
        (then
          (local.set $v
            (v128.load (i32.add (local.get $ptr) (local.get $i))))
          (local.set $a0 (local.get $a))
          (local.set $b0
            (i8x16.extract_lane_u 0 (local.get $v)))
          (local.set $b1
            (i8x16.extract_lane_u 1 (local.get $v)))
          (local.set $b2
            (i8x16.extract_lane_u 2 (local.get $v)))
          (local.set $b3
            (i8x16.extract_lane_u 3 (local.get $v)))
          (local.set $b4
            (i8x16.extract_lane_u 4 (local.get $v)))
          (local.set $b5
            (i8x16.extract_lane_u 5 (local.get $v)))
          (local.set $b6
            (i8x16.extract_lane_u 6 (local.get $v)))
          (local.set $b7
            (i8x16.extract_lane_u 7 (local.get $v)))
          (local.set $b8
            (i8x16.extract_lane_u 8 (local.get $v)))
          (local.set $b9
            (i8x16.extract_lane_u 9 (local.get $v)))
          (local.set $b10
            (i8x16.extract_lane_u 10 (local.get $v)))
          (local.set $b11
            (i8x16.extract_lane_u 11 (local.get $v)))
          (local.set $b12
            (i8x16.extract_lane_u 12 (local.get $v)))
          (local.set $b13
            (i8x16.extract_lane_u 13 (local.get $v)))
          (local.set $b14
            (i8x16.extract_lane_u 14 (local.get $v)))
          (local.set $b15
            (i8x16.extract_lane_u 15 (local.get $v)))
          (local.set $sum
            (i32.add
              (i32.add
                (i32.add
                  (i32.add
                    (i32.add
                      (i32.add
                        (i32.add
                          (i32.add
                            (i32.add
                              (i32.add
                                (i32.add
                                  (i32.add
                                    (i32.add
                                      (i32.add
                                        (i32.add
                                          (local.get $b0)
                                          (local.get $b1))
                                        (local.get $b2))
                                      (local.get $b3))
                                    (local.get $b4))
                                  (local.get $b5))
                                (local.get $b6))
                              (local.get $b7))
                            (local.get $b8))
                          (local.get $b9))
                        (local.get $b10))
                      (local.get $b11))
                    (local.get $b12))
                  (local.get $b13))
                (local.get $b14))
              (local.get $b15)))
          (local.set $wsum
            (i32.add
              (i32.add
                (i32.add
                  (i32.add
                    (i32.add
                      (i32.add
                        (i32.add
                          (i32.add
                            (i32.add
                              (i32.add
                                (i32.add
                                  (i32.add
                                    (i32.add
                                      (i32.add
                                        (i32.add
                                          (i32.mul (i32.const 16) (local.get $b0))
                                          (i32.mul (i32.const 15) (local.get $b1)))
                                        (i32.mul (i32.const 14) (local.get $b2)))
                                      (i32.mul (i32.const 13) (local.get $b3)))
                                    (i32.mul (i32.const 12) (local.get $b4)))
                                  (i32.mul (i32.const 11) (local.get $b5)))
                                (i32.mul (i32.const 10) (local.get $b6)))
                              (i32.mul (i32.const 9) (local.get $b7)))
                            (i32.mul (i32.const 8) (local.get $b8)))
                          (i32.mul (i32.const 7) (local.get $b9)))
                        (i32.mul (i32.const 6) (local.get $b10)))
                      (i32.mul (i32.const 5) (local.get $b11)))
                    (i32.mul (i32.const 4) (local.get $b12)))
                  (i32.mul (i32.const 3) (local.get $b13)))
                (i32.mul (i32.const 2) (local.get $b14)))
              (i32.mul (i32.const 1) (local.get $b15))))
          (local.set $a
            (i32.rem_u
              (i32.add (local.get $a) (local.get $sum))
              (i32.const 65521)))
          (local.set $b
            (i32.rem_u
              (i32.add
                (i32.add
                  (local.get $b)
                  (i32.mul (i32.const 16) (local.get $a0)))
                (local.get $wsum))
              (i32.const 65521)))
          (local.set $i (i32.add (local.get $i) (i32.const 16)))
          (br $loop))
        (else
          (if (i32.lt_u (local.get $i) (local.get $len))
            (then
              (local.set $a
                (i32.rem_u
                  (i32.add
                    (local.get $a)
                    (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
                  (i32.const 65521)))
              (local.set $b
                (i32.rem_u
                  (i32.add (local.get $b) (local.get $a))
                  (i32.const 65521)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $loop))))))
    (i32.or
      (i32.shl (local.get $b) (i32.const 16))
      (local.get $a)))

  ;; ── Byte-at-a-time CRC-32 update ──
  ;; crc32_update_byte(crc, byte) → crc
  (func $crc32_update_byte (export "crc32_update_byte") (param $crc i32) (param $byte i32) (result i32)
    (local $c i32) (local $i i32)
    (local.set $c (i32.xor (local.get $crc) (local.get $byte)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $bits
        (br_if $done (i32.eq (local.get $i) (i32.const 8)))
        (if (i32.and (local.get $c) (i32.const 1))
          (then
            (local.set $c
              (i32.xor (i32.shr_u (local.get $c) (i32.const 1)) (i32.const 0xedb88320))))
          (else
            (local.set $c (i32.shr_u (local.get $c) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $bits)))
    (local.get $c))

  ;; ── Byte-at-a-time Adler-32 update ──
  ;; adler32_update_byte(adler, byte) → adler
  (func $adler32_update_byte (export "adler32_update_byte") (param $adler i32) (param $byte i32) (result i32)
    (local $s1 i32) (local $s2 i32)
    (local.set $s1 (i32.and (local.get $adler) (i32.const 65535)))
    (local.set $s2 (i32.shr_u (local.get $adler) (i32.const 16)))
    (local.set $s1 (i32.rem_u (i32.add (local.get $s1) (local.get $byte)) (i32.const 65521)))
    (local.set $s2 (i32.rem_u (i32.add (local.get $s2) (local.get $s1)) (i32.const 65521)))
    (i32.or (local.get $s1) (i32.shl (local.get $s2) (i32.const 16))))
;; crypto-sha1 — SHA-1 hash (work buffer at 0x10000, 80×4 = 320 bytes)

  (func $m61range_ok (param $ptr i32) (param $len i32) (result i32)
    (local $end i32)
    local.get $ptr
    local.get $len
    i32.add
    local.set $end
    local.get $end
    local.get $ptr
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $end
    i32.const 65536
    i32.le_u)

  (func $m61w_addr (param $i i32) (result i32)
    i32.const 65536
    local.get $i
    i32.const 2
    i32.shl
    i32.add)

  (func $padded_byte
    (param $ptr i32)
    (param $len i32)
    (param $padded_len i32)
    (param $index i32)
    (result i32)
    (local $bit_len i64)
    (local $tail_index i32)
    local.get $index
    local.get $len
    i32.lt_u
    if
      local.get $ptr
      local.get $index
      i32.add
      i32.load8_u
      return
    end
    local.get $index
    local.get $len
    i32.eq
    if
      i32.const 0x80
      return
    end
    local.get $index
    local.get $padded_len
    i32.const 8
    i32.sub
    i32.ge_u
    if
      local.get $len
      i64.extend_i32_u
      i64.const 3
      i64.shl
      local.set $bit_len
      local.get $index
      local.get $padded_len
      i32.const 8
      i32.sub
      i32.sub
      local.set $tail_index
      local.get $bit_len
      i64.const 7
      local.get $tail_index
      i64.extend_i32_u
      i64.sub
      i64.const 8
      i64.mul
      i64.shr_u
      i32.wrap_i64
      i32.const 0xff
      i32.and
      return
    end
    i32.const 0)

  (func $word
    (param $ptr i32)
    (param $len i32)
    (param $padded_len i32)
    (param $index i32)
    (result i32)
    (local $base i32)
    local.get $index
    local.set $base
    local.get $ptr
    local.get $len
    local.get $padded_len
    local.get $base
    call $padded_byte
    i32.const 24
    i32.shl
    local.get $ptr
    local.get $len
    local.get $padded_len
    local.get $base
    i32.const 1
    i32.add
    call $padded_byte
    i32.const 16
    i32.shl
    i32.or
    local.get $ptr
    local.get $len
    local.get $padded_len
    local.get $base
    i32.const 2
    i32.add
    call $padded_byte
    i32.const 8
    i32.shl
    i32.or
    local.get $ptr
    local.get $len
    local.get $padded_len
    local.get $base
    i32.const 3
    i32.add
    call $padded_byte
    i32.or)

  (func $m61store_be32 (param $ptr i32) (param $value i32)
    local.get $ptr
    local.get $value
    i32.const 24
    i32.shr_u
    i32.store8
    local.get $ptr
    i32.const 1
    i32.add
    local.get $value
    i32.const 16
    i32.shr_u
    i32.store8
    local.get $ptr
    i32.const 2
    i32.add
    local.get $value
    i32.const 8
    i32.shr_u
    i32.store8
    local.get $ptr
    i32.const 3
    i32.add
    local.get $value
    i32.store8)

  (func $sha1 (export "sha1")
    (param $ptr i32)
    (param $len i32)
    (param $out_ptr i32)
    (result i64)
    (local $padded_len i32)
    (local $block i32)
    (local $i i32)
    (local $a i32)
    (local $b i32)
    (local $c i32)
    (local $d i32)
    (local $e i32)
    (local $f i32)
    (local $k i32)
    (local $temp i32)
    (local $h0 i32)
    (local $h1 i32)
    (local $h2 i32)
    (local $h3 i32)
    (local $h4 i32)

    local.get $out_ptr
    i32.const 20
    call $m61range_ok
    i32.eqz
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $ptr
    local.get $len
    call $m61range_ok
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $len
    i32.const 9
    i32.add
    i32.const 63
    i32.add
    i32.const -64
    i32.and
    local.set $padded_len

    i32.const 0x67452301
    local.set $h0
    i32.const 0xefcdab89
    local.set $h1
    i32.const 0x98badcfe
    local.set $h2
    i32.const 0x10325476
    local.set $h3
    i32.const 0xc3d2e1f0
    local.set $h4

    i32.const 0
    local.set $block
    block $done_blocks
      loop $blocks
        local.get $block
        local.get $padded_len
        i32.ge_u
        br_if $done_blocks

        i32.const 0
        local.set $i
        block $done_load
          loop $load
            local.get $i
            i32.const 16
            i32.ge_u
            br_if $done_load
            local.get $i
            call $m61w_addr
            local.get $ptr
            local.get $len
            local.get $padded_len
            local.get $block
            local.get $i
            i32.const 2
            i32.shl
            i32.add
            call $word
            i32.store
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $load
          end
        end

        i32.const 16
        local.set $i
        block $done_expand
          loop $expand
            local.get $i
            i32.const 80
            i32.ge_u
            br_if $done_expand
            local.get $i
            call $m61w_addr
            local.get $i
            i32.const 3
            i32.sub
            call $m61w_addr
            i32.load
            local.get $i
            i32.const 8
            i32.sub
            call $m61w_addr
            i32.load
            i32.xor
            local.get $i
            i32.const 14
            i32.sub
            call $m61w_addr
            i32.load
            i32.xor
            local.get $i
            i32.const 16
            i32.sub
            call $m61w_addr
            i32.load
            i32.xor
            i32.const 1
            i32.rotl
            i32.store
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $expand
          end
        end

        local.get $h0
        local.set $a
        local.get $h1
        local.set $b
        local.get $h2
        local.set $c
        local.get $h3
        local.set $d
        local.get $h4
        local.set $e

        i32.const 0
        local.set $i
        block $done_rounds
          loop $rounds
            local.get $i
            i32.const 80
            i32.ge_u
            br_if $done_rounds

            local.get $i
            i32.const 20
            i32.lt_u
            if
              local.get $b
              local.get $c
              i32.and
              local.get $b
              i32.const -1
              i32.xor
              local.get $d
              i32.and
              i32.or
              local.set $f
              i32.const 0x5a827999
              local.set $k
            else
              local.get $i
              i32.const 40
              i32.lt_u
              if
                local.get $b
                local.get $c
                i32.xor
                local.get $d
                i32.xor
                local.set $f
                i32.const 0x6ed9eba1
                local.set $k
              else
                local.get $i
                i32.const 60
                i32.lt_u
                if
                  local.get $b
                  local.get $c
                  i32.and
                  local.get $b
                  local.get $d
                  i32.and
                  i32.or
                  local.get $c
                  local.get $d
                  i32.and
                  i32.or
                  local.set $f
                  i32.const 0x8f1bbcdc
                  local.set $k
                else
                  local.get $b
                  local.get $c
                  i32.xor
                  local.get $d
                  i32.xor
                  local.set $f
                  i32.const 0xca62c1d6
                  local.set $k
                end
              end
            end

            local.get $a
            i32.const 5
            i32.rotl
            local.get $f
            i32.add
            local.get $e
            i32.add
            local.get $k
            i32.add
            local.get $i
            call $m61w_addr
            i32.load
            i32.add
            local.set $temp
            local.get $d
            local.set $e
            local.get $c
            local.set $d
            local.get $b
            i32.const 30
            i32.rotl
            local.set $c
            local.get $a
            local.set $b
            local.get $temp
            local.set $a

            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $rounds
          end
        end

        local.get $h0
        local.get $a
        i32.add
        local.set $h0
        local.get $h1
        local.get $b
        i32.add
        local.set $h1
        local.get $h2
        local.get $c
        i32.add
        local.set $h2
        local.get $h3
        local.get $d
        i32.add
        local.set $h3
        local.get $h4
        local.get $e
        i32.add
        local.set $h4

        local.get $block
        i32.const 64
        i32.add
        local.set $block
        br $blocks
      end
    end

    local.get $out_ptr
    local.get $h0
    call $m61store_be32
    local.get $out_ptr
    i32.const 4
    i32.add
    local.get $h1
    call $m61store_be32
    local.get $out_ptr
    i32.const 8
    i32.add
    local.get $h2
    call $m61store_be32
    local.get $out_ptr
    i32.const 12
    i32.add
    local.get $h3
    call $m61store_be32
    local.get $out_ptr
    i32.const 16
    i32.add
    local.get $h4
    call $m61store_be32

    i32.const 0
    i32.const 20
    call $pack)

;; deflate-inflate — raw deflate scan & inflate

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid_data,
  ;; 6 limit_exceeded, 7 unsupported.
  ;; Record, little-endian u32:
  ;; 0: consumed
  ;; 4: written or scanned stored payload total
  ;; 8: block_count
  ;; 12: last_block_type
  ;; 16: crc32 over emitted bytes
  ;; 20: adler32 over emitted bytes
  (global $br_src_ptr (mut i32) (i32.const 0))
  (global $br_src_len (mut i32) (i32.const 0))
  (global $br_bitpos (mut i32) (i32.const 0))
  (global $br_bitbuf (mut i32) (i32.const 0))
  (global $br_bitcnt (mut i32) (i32.const 0))

  (func $br_init (param $src_ptr i32) (param $src_len i32)
    (global.set $br_src_ptr (local.get $src_ptr))
    (global.set $br_src_len (local.get $src_len))
    (global.set $br_bitpos (i32.const 0))
    (global.set $br_bitbuf (i32.const 0))
    (global.set $br_bitcnt (i32.const 0)))

  (func $br_read (param $n i32) (result i32)
    (local $byte_off i32)
    (local $byte i32)
    (local $mask i32)
    (local $bits i32)
    (block $ready
      (loop $fill
        (br_if $ready (i32.ge_u (global.get $br_bitcnt) (local.get $n)))
        (local.set $byte_off (i32.shr_u (global.get $br_bitpos) (i32.const 3)))
        (if (i32.ge_u (local.get $byte_off) (global.get $br_src_len))
          (then (return (i32.const -1))))
        (local.set $byte
          (i32.load8_u (i32.add (global.get $br_src_ptr) (local.get $byte_off))))
        (global.set $br_bitbuf
          (i32.or
            (global.get $br_bitbuf)
            (i32.shl (local.get $byte) (global.get $br_bitcnt))))
        (global.set $br_bitcnt (i32.add (global.get $br_bitcnt) (i32.const 8)))
        (global.set $br_bitpos (i32.add (global.get $br_bitpos) (i32.const 8)))
        (br $fill)))
    (local.set $mask (i32.sub (i32.shl (i32.const 1) (local.get $n)) (i32.const 1)))
    (local.set $bits (i32.and (global.get $br_bitbuf) (local.get $mask)))
    (global.set $br_bitbuf (i32.shr_u (global.get $br_bitbuf) (local.get $n)))
    (global.set $br_bitcnt (i32.sub (global.get $br_bitcnt) (local.get $n)))
    (local.get $bits))

  (func $br_align_byte
    (local $used_bits i32)
    (local.set $used_bits (i32.sub (global.get $br_bitpos) (global.get $br_bitcnt)))
    (global.set $br_bitpos
      (i32.and
        (i32.add (local.get $used_bits) (i32.const 7))
        (i32.const -8)))
    (global.set $br_bitbuf (i32.const 0))
    (global.set $br_bitcnt (i32.const 0)))

  (func $br_consumed_bytes (result i32)
    (local $used_bits i32)
    (local.set $used_bits (i32.sub (global.get $br_bitpos) (global.get $br_bitcnt)))
    (i32.shr_u (i32.add (local.get $used_bits) (i32.const 7)) (i32.const 3)))

  (func $reverse_bits (param $v i32) (param $n i32) (result i32)
    (local $r i32)
    (loop $bits
      (if (i32.eqz (local.get $n))
        (then (return (local.get $r))))
      (local.set $r
        (i32.or
          (i32.shl (local.get $r) (i32.const 1))
          (i32.and (local.get $v) (i32.const 1))))
      (local.set $v (i32.shr_u (local.get $v) (i32.const 1)))
      (local.set $n (i32.sub (local.get $n) (i32.const 1)))
      (br $bits))
    (local.get $r))

  (func $fixed_symbol (result i32)
    (local $bits i32)
    (local $next i32)
    (local $rev i32)
    (local.set $bits (call $br_read (i32.const 7)))
    (if (i32.lt_s (local.get $bits) (i32.const 0))
      (then (return (i32.const -1))))
    (local.set $rev (call $reverse_bits (local.get $bits) (i32.const 7)))
    (if (i32.le_u (local.get $rev) (i32.const 23))
      (then (return
        (i32.or (i32.shl (i32.const 7) (i32.const 16))
                (i32.add (i32.const 256) (local.get $rev))))))

    (local.set $next (call $br_read (i32.const 1)))
    (if (i32.lt_s (local.get $next) (i32.const 0))
      (then (return (i32.const -1))))
    (local.set $bits (i32.or (local.get $bits) (i32.shl (local.get $next) (i32.const 7))))
    (local.set $rev (call $reverse_bits (local.get $bits) (i32.const 8)))
    (if (i32.and
          (i32.ge_u (local.get $rev) (i32.const 48))
          (i32.le_u (local.get $rev) (i32.const 191)))
      (then (return
        (i32.or (i32.shl (i32.const 8) (i32.const 16))
                (i32.sub (local.get $rev) (i32.const 48))))))
    (if (i32.and
          (i32.ge_u (local.get $rev) (i32.const 192))
          (i32.le_u (local.get $rev) (i32.const 199)))
      (then (return
        (i32.or (i32.shl (i32.const 8) (i32.const 16))
                (i32.add (i32.const 280) (i32.sub (local.get $rev) (i32.const 192)))))))

    (local.set $next (call $br_read (i32.const 1)))
    (if (i32.lt_s (local.get $next) (i32.const 0))
      (then (return (i32.const -1))))
    (local.set $bits (i32.or (local.get $bits) (i32.shl (local.get $next) (i32.const 8))))
    (local.set $rev (call $reverse_bits (local.get $bits) (i32.const 9)))
    (if (i32.and
          (i32.ge_u (local.get $rev) (i32.const 400))
          (i32.le_u (local.get $rev) (i32.const 511)))
      (then (return
        (i32.or (i32.shl (i32.const 9) (i32.const 16))
                (i32.add (i32.const 144) (i32.sub (local.get $rev) (i32.const 400)))))))
    (i32.const -3))

  (func $length_base (param $sym i32) (result i32)
    (if (i32.le_u (local.get $sym) (i32.const 264))
      (then (return (i32.add (i32.const 3) (i32.sub (local.get $sym) (i32.const 257))))))
    (if (i32.le_u (local.get $sym) (i32.const 268))
      (then (return (i32.add (i32.const 11) (i32.shl (i32.sub (local.get $sym) (i32.const 265)) (i32.const 1))))))
    (if (i32.le_u (local.get $sym) (i32.const 272))
      (then (return (i32.add (i32.const 19) (i32.shl (i32.sub (local.get $sym) (i32.const 269)) (i32.const 2))))))
    (if (i32.le_u (local.get $sym) (i32.const 276))
      (then (return (i32.add (i32.const 35) (i32.shl (i32.sub (local.get $sym) (i32.const 273)) (i32.const 3))))))
    (if (i32.le_u (local.get $sym) (i32.const 280))
      (then (return (i32.add (i32.const 67) (i32.shl (i32.sub (local.get $sym) (i32.const 277)) (i32.const 4))))))
    (if (i32.le_u (local.get $sym) (i32.const 284))
      (then (return (i32.add (i32.const 131) (i32.shl (i32.sub (local.get $sym) (i32.const 281)) (i32.const 5))))))
    (if (i32.eq (local.get $sym) (i32.const 285))
      (then (return (i32.const 258))))
    (i32.const -1))

  (func $length_extra (param $sym i32) (result i32)
    (if (i32.le_u (local.get $sym) (i32.const 264)) (then (return (i32.const 0))))
    (if (i32.le_u (local.get $sym) (i32.const 268)) (then (return (i32.const 1))))
    (if (i32.le_u (local.get $sym) (i32.const 272)) (then (return (i32.const 2))))
    (if (i32.le_u (local.get $sym) (i32.const 276)) (then (return (i32.const 3))))
    (if (i32.le_u (local.get $sym) (i32.const 280)) (then (return (i32.const 4))))
    (if (i32.le_u (local.get $sym) (i32.const 284)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $sym) (i32.const 285)) (then (return (i32.const 0))))
    (i32.const -1))

  (func $dist_base (param $sym i32) (result i32)
    (local $extra i32)
    (local $base i32)
    (if (i32.gt_u (local.get $sym) (i32.const 29))
      (then (return (i32.const -1))))
    (if (i32.lt_u (local.get $sym) (i32.const 4))
      (then (return (i32.add (local.get $sym) (i32.const 1)))))
    (local.set $extra (i32.sub (i32.shr_u (local.get $sym) (i32.const 1)) (i32.const 1)))
    (local.set $base (i32.add (i32.shl (i32.const 1) (i32.add (local.get $extra) (i32.const 1))) (i32.const 1)))
    (if (i32.and (local.get $sym) (i32.const 1))
      (then (local.set $base (i32.add (local.get $base) (i32.shl (i32.const 1) (local.get $extra))))))
    (local.get $base))

  (func $dist_extra (param $sym i32) (result i32)
    (if (i32.gt_u (local.get $sym) (i32.const 29))
      (then (return (i32.const -1))))
    (if (i32.lt_u (local.get $sym) (i32.const 4))
      (then (return (i32.const 0))))
    (i32.sub (i32.shr_u (local.get $sym) (i32.const 1)) (i32.const 1)))

  (func $cl_order (param $idx i32) (result i32)
    (if (i32.eq (local.get $idx) (i32.const 0)) (then (return (i32.const 16))))
    (if (i32.eq (local.get $idx) (i32.const 1)) (then (return (i32.const 17))))
    (if (i32.eq (local.get $idx) (i32.const 2)) (then (return (i32.const 18))))
    (if (i32.eq (local.get $idx) (i32.const 3)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $idx) (i32.const 4)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $idx) (i32.const 5)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $idx) (i32.const 6)) (then (return (i32.const 9))))
    (if (i32.eq (local.get $idx) (i32.const 7)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $idx) (i32.const 8)) (then (return (i32.const 10))))
    (if (i32.eq (local.get $idx) (i32.const 9)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $idx) (i32.const 10)) (then (return (i32.const 11))))
    (if (i32.eq (local.get $idx) (i32.const 11)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $idx) (i32.const 12)) (then (return (i32.const 12))))
    (if (i32.eq (local.get $idx) (i32.const 13)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $idx) (i32.const 14)) (then (return (i32.const 13))))
    (if (i32.eq (local.get $idx) (i32.const 15)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $idx) (i32.const 16)) (then (return (i32.const 14))))
    (if (i32.eq (local.get $idx) (i32.const 17)) (then (return (i32.const 1))))
    (i32.const 15))

  (func $zero_u32_table (param $ptr i32) (param $count i32)
    (local $i i32)
    (loop $zero
      (if (i32.lt_u (local.get $i) (local.get $count))
        (then
          (i32.store (i32.add (local.get $ptr) (i32.shl (local.get $i) (i32.const 2))) (i32.const 0))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $zero)))))

  (func $build_table
    (param $lens_ptr i32)
    (param $symbol_count i32)
    (param $symbols_ptr i32)
    (param $counts_ptr i32)
    (param $offsets_ptr i32)
    (param $maxbits i32)
    (result i32)
    (local $i i32)
    (local $len i32)
    (local $total i32)
    (local $left i32)
    (local $sum i32)
    (local $pos i32)
    (call $zero_u32_table (local.get $counts_ptr) (i32.const 16))
    (call $zero_u32_table (local.get $offsets_ptr) (i32.const 16))
    (local.set $i (i32.const 0))
    (loop $count_lens
      (if (i32.lt_u (local.get $i) (local.get $symbol_count))
        (then
          (local.set $len (i32.load8_u (i32.add (local.get $lens_ptr) (local.get $i))))
          (if (i32.gt_u (local.get $len) (local.get $maxbits)) (then (return (i32.const 3))))
          (if (local.get $len)
            (then
              (i32.store
                (i32.add (local.get $counts_ptr) (i32.shl (local.get $len) (i32.const 2)))
                (i32.add
                  (i32.load (i32.add (local.get $counts_ptr) (i32.shl (local.get $len) (i32.const 2))))
                  (i32.const 1)))
              (local.set $total (i32.add (local.get $total) (i32.const 1)))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $count_lens))))
    (if (i32.eqz (local.get $total)) (then (return (i32.const 3))))
    (local.set $left (i32.const 1))
    (local.set $i (i32.const 1))
    (loop $check_left
      (if (i32.le_u (local.get $i) (local.get $maxbits))
        (then
          (local.set $left
            (i32.sub
              (i32.shl (local.get $left) (i32.const 1))
              (i32.load (i32.add (local.get $counts_ptr) (i32.shl (local.get $i) (i32.const 2))))))
          (if (i32.lt_s (local.get $left) (i32.const 0)) (then (return (i32.const 3))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $check_left))))
    (local.set $sum (i32.const 0))
    (local.set $i (i32.const 1))
    (loop $make_offsets
      (if (i32.le_u (local.get $i) (local.get $maxbits))
        (then
          (i32.store (i32.add (local.get $offsets_ptr) (i32.shl (local.get $i) (i32.const 2))) (local.get $sum))
          (local.set $sum
            (i32.add
              (local.get $sum)
              (i32.load (i32.add (local.get $counts_ptr) (i32.shl (local.get $i) (i32.const 2))))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $make_offsets))))
    (local.set $i (i32.const 0))
    (loop $sort_symbols
      (if (i32.lt_u (local.get $i) (local.get $symbol_count))
        (then
          (local.set $len (i32.load8_u (i32.add (local.get $lens_ptr) (local.get $i))))
          (if (local.get $len)
            (then
              (local.set $pos (i32.load (i32.add (local.get $offsets_ptr) (i32.shl (local.get $len) (i32.const 2)))))
              (i32.store (i32.add (local.get $symbols_ptr) (i32.shl (local.get $pos) (i32.const 2))) (local.get $i))
              (i32.store
                (i32.add (local.get $offsets_ptr) (i32.shl (local.get $len) (i32.const 2)))
                (i32.add (local.get $pos) (i32.const 1)))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $sort_symbols))))
    (i32.const 0))

  (func $decode_symbol
    (param $symbols_ptr i32)
    (param $counts_ptr i32)
    (param $maxbits i32)
    (result i32)
    (local $code i32)
    (local $first i32)
    (local $index i32)
    (local $len i32)
    (local $bit i32)
    (local $count i32)
    (local.set $len (i32.const 1))
    (loop $decode
      (if (i32.le_u (local.get $len) (local.get $maxbits))
        (then
          (local.set $bit (call $br_read (i32.const 1)))
          (if (i32.lt_s (local.get $bit) (i32.const 0)) (then (return (i32.const -1))))
          (local.set $code (i32.or (local.get $code) (local.get $bit)))
          (local.set $count (i32.load (i32.add (local.get $counts_ptr) (i32.shl (local.get $len) (i32.const 2)))))
          (if (i32.lt_u (i32.sub (local.get $code) (local.get $first)) (local.get $count))
            (then
              (return
                (i32.load
                  (i32.add
                    (local.get $symbols_ptr)
                    (i32.shl
                      (i32.add (local.get $index) (i32.sub (local.get $code) (local.get $first)))
                      (i32.const 2)))))))
          (local.set $index (i32.add (local.get $index) (local.get $count)))
          (local.set $first (i32.shl (i32.add (local.get $first) (local.get $count)) (i32.const 1)))
          (local.set $code (i32.shl (local.get $code) (i32.const 1)))
          (local.set $len (i32.add (local.get $len) (i32.const 1)))
          (br $decode))))
    (i32.const -3))

  (func $m66write_record
    (param $out i32)
    (param $consumed i32)
    (param $written i32)
    (param $block_count i32)
    (param $last_block_type i32)
    (param $crc32 i32)
    (param $adler32 i32)
    (i32.store (local.get $out) (local.get $consumed))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $written))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $block_count))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $last_block_type))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $crc32))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $adler32)))

  (func (export "deflate_scan_blocks")
    (param $src_ptr i32)
    (param $src_len i32)
    (param $out_record i32)
    (result i32)
    (local $bfinal i32)
    (local $btype i32)
    (local $off i32)
    (local $block_len i32)
    (local $nlen i32)
    (local $payload_end i32)
    (local $block_count i32)
    (local $payload_total i32)
    (call $br_init (local.get $src_ptr) (local.get $src_len))
    (loop $blocks
      (local.set $bfinal (call $br_read (i32.const 1)))
      (if (i32.lt_s (local.get $bfinal) (i32.const 0)) (then (return (i32.const 1))))
      (local.set $btype (call $br_read (i32.const 2)))
      (if (i32.lt_s (local.get $btype) (i32.const 0)) (then (return (i32.const 1))))
      (if (i32.eq (local.get $btype) (i32.const 3)) (then (return (i32.const 3))))
      (if (i32.ne (local.get $btype) (i32.const 0))
        (then
          (call $m66write_record
            (local.get $out_record)
            (call $br_consumed_bytes)
            (local.get $payload_total)
            (local.get $block_count)
            (local.get $btype)
            (i32.const 0)
            (i32.const 1))
          (return (i32.const 7))))
      (call $br_align_byte)
      (local.set $off (i32.shr_u (global.get $br_bitpos) (i32.const 3)))
      (if (i32.gt_u (i32.add (local.get $off) (i32.const 4)) (local.get $src_len))
        (then (return (i32.const 1))))
      (local.set $block_len
        (i32.or
          (i32.load8_u (i32.add (local.get $src_ptr) (local.get $off)))
          (i32.shl
            (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 1))))
            (i32.const 8))))
      (local.set $nlen
        (i32.or
          (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 2))))
          (i32.shl
            (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 3))))
            (i32.const 8))))
      (if (i32.ne (i32.and (i32.xor (local.get $block_len) (local.get $nlen)) (i32.const 65535)) (i32.const 65535))
        (then (return (i32.const 3))))
      (local.set $payload_end (i32.add (i32.add (local.get $off) (i32.const 4)) (local.get $block_len)))
      (if (i32.or
            (i32.lt_u (local.get $payload_end) (local.get $off))
            (i32.gt_u (local.get $payload_end) (local.get $src_len)))
        (then (return (i32.const 1))))
      (local.set $block_count (i32.add (local.get $block_count) (i32.const 1)))
      (local.set $payload_total (i32.add (local.get $payload_total) (local.get $block_len)))
      (global.set $br_bitpos (i32.shl (local.get $payload_end) (i32.const 3)))
      (global.set $br_bitbuf (i32.const 0))
      (global.set $br_bitcnt (i32.const 0))
      (if (local.get $bfinal)
        (then
          (call $m66write_record
            (local.get $out_record)
            (local.get $payload_end)
            (local.get $payload_total)
            (local.get $block_count)
            (i32.const 0)
            (i32.const 0)
            (i32.const 1))
          (return (i32.const 0))))
      (br $blocks))
    (i32.const 1))

  (func (export "deflate_inflate_raw")
    (param $src_ptr i32)
    (param $src_len i32)
    (param $out_ptr i32)
    (param $out_cap i32)
    (param $out_limit i32)
    (param $out_record i32)
    (result i32)
    (local $bfinal i32)
    (local $btype i32)
    (local $off i32)
    (local $block_len i32)
    (local $nlen i32)
    (local $payload_off i32)
    (local $payload_end i32)
    (local $written i32)
    (local $block_count i32)
    (local $crc i32)
    (local $adler i32)
    (local $i i32)
    (local $byte i32)
    (local $packed i32)
    (local $sym i32)
    (local $extra_bits i32)
    (local $extra_val i32)
    (local $length i32)
    (local $dist_sym i32)
    (local $dist i32)
    (local $copy_from i32)
    (local $hlit i32)
    (local $hdist i32)
    (local $hclen i32)
    (local $total_lens i32)
    (local $lens_index i32)
    (local $code i32)
    (local $repeat_count i32)
    (local $prev_len i32)
    (local.set $crc (i32.const -1))
    (local.set $adler (i32.const 1))
    (call $br_init (local.get $src_ptr) (local.get $src_len))
    (loop $blocks
      (local.set $bfinal (call $br_read (i32.const 1)))
      (if (i32.lt_s (local.get $bfinal) (i32.const 0)) (then (return (i32.const 1))))
      (local.set $btype (call $br_read (i32.const 2)))
      (if (i32.lt_s (local.get $btype) (i32.const 0)) (then (return (i32.const 1))))
      (if (i32.eq (local.get $btype) (i32.const 3)) (then (return (i32.const 3))))
      (if (i32.eq (local.get $btype) (i32.const 2))
        (then
          (local.set $hlit (call $br_read (i32.const 5)))
          (if (i32.lt_s (local.get $hlit) (i32.const 0)) (then (return (i32.const 1))))
          (local.set $hlit (i32.add (local.get $hlit) (i32.const 257)))
          (local.set $hdist (call $br_read (i32.const 5)))
          (if (i32.lt_s (local.get $hdist) (i32.const 0)) (then (return (i32.const 1))))
          (local.set $hdist (i32.add (local.get $hdist) (i32.const 1)))
          (local.set $hclen (call $br_read (i32.const 4)))
          (if (i32.lt_s (local.get $hclen) (i32.const 0)) (then (return (i32.const 1))))
          (local.set $hclen (i32.add (local.get $hclen) (i32.const 4)))
          (if (i32.or
                (i32.or (i32.gt_u (local.get $hlit) (i32.const 286)) (i32.gt_u (local.get $hdist) (i32.const 32)))
                (i32.gt_u (local.get $hclen) (i32.const 19)))
            (then (return (i32.const 3))))
          (local.set $i (i32.const 0))
          (loop $zero_cl_lens
            (if (i32.lt_u (local.get $i) (i32.const 19))
              (then
                (i32.store8 (i32.add (i32.const 591724) (local.get $i)) (i32.const 0))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $zero_cl_lens))))
          (local.set $i (i32.const 0))
          (loop $read_cl_lens
            (if (i32.lt_u (local.get $i) (local.get $hclen))
              (then
                (local.set $code (call $br_read (i32.const 3)))
                (if (i32.lt_s (local.get $code) (i32.const 0)) (then (return (i32.const 1))))
                (i32.store8
                  (i32.add (i32.const 591724) (call $cl_order (local.get $i)))
                  (local.get $code))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $read_cl_lens))))
          (if (call $build_table
                (i32.const 591724)
                (i32.const 19)
                (i32.const 589952)
                (i32.const 589824)
                (i32.const 589888)
                (i32.const 7))
            (then (return (i32.const 3))))
          (local.set $total_lens (i32.add (local.get $hlit) (local.get $hdist)))
          (local.set $i (i32.const 0))
          (loop $zero_dyn_lens
            (if (i32.lt_u (local.get $i) (i32.const 320))
              (then
                (i32.store8 (i32.add (i32.const 591724) (local.get $i)) (i32.const 0))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $zero_dyn_lens))))
          (local.set $lens_index (i32.const 0))
          (local.set $prev_len (i32.const 0))
          (loop $read_dyn_lens
            (if (i32.lt_u (local.get $lens_index) (local.get $total_lens))
              (then
                (local.set $code (call $decode_symbol (i32.const 589952) (i32.const 589824) (i32.const 7)))
                (if (i32.lt_s (local.get $code) (i32.const 0))
                  (then
                    (if (i32.eq (local.get $code) (i32.const -1))
                      (then (return (i32.const 1)))
                      (else (return (i32.const 3))))))
                (if (i32.le_u (local.get $code) (i32.const 15))
                  (then
                    (i32.store8 (i32.add (i32.const 591724) (local.get $lens_index)) (local.get $code))
                    (local.set $prev_len (local.get $code))
                    (local.set $lens_index (i32.add (local.get $lens_index) (i32.const 1)))
                    (br $read_dyn_lens)))
                (if (i32.eq (local.get $code) (i32.const 16))
                  (then
                    (if (i32.eqz (local.get $lens_index)) (then (return (i32.const 3))))
                    (local.set $repeat_count (call $br_read (i32.const 2)))
                    (if (i32.lt_s (local.get $repeat_count) (i32.const 0)) (then (return (i32.const 1))))
                    (local.set $repeat_count (i32.add (local.get $repeat_count) (i32.const 3)))
                    (if (i32.gt_u (i32.add (local.get $lens_index) (local.get $repeat_count)) (local.get $total_lens))
                      (then (return (i32.const 3))))
                    (loop $repeat_prev
                      (if (local.get $repeat_count)
                        (then
                          (i32.store8 (i32.add (i32.const 591724) (local.get $lens_index)) (local.get $prev_len))
                          (local.set $lens_index (i32.add (local.get $lens_index) (i32.const 1)))
                          (local.set $repeat_count (i32.sub (local.get $repeat_count) (i32.const 1)))
                          (br $repeat_prev))))
                    (br $read_dyn_lens)))
                (if (i32.eq (local.get $code) (i32.const 17))
                  (then
                    (local.set $repeat_count (call $br_read (i32.const 3)))
                    (if (i32.lt_s (local.get $repeat_count) (i32.const 0)) (then (return (i32.const 1))))
                    (local.set $repeat_count (i32.add (local.get $repeat_count) (i32.const 3)))
                    (if (i32.gt_u (i32.add (local.get $lens_index) (local.get $repeat_count)) (local.get $total_lens))
                      (then (return (i32.const 3))))
                    (local.set $prev_len (i32.const 0))
                    (local.set $lens_index (i32.add (local.get $lens_index) (local.get $repeat_count)))
                    (br $read_dyn_lens)))
                (if (i32.eq (local.get $code) (i32.const 18))
                  (then
                    (local.set $repeat_count (call $br_read (i32.const 7)))
                    (if (i32.lt_s (local.get $repeat_count) (i32.const 0)) (then (return (i32.const 1))))
                    (local.set $repeat_count (i32.add (local.get $repeat_count) (i32.const 11)))
                    (if (i32.gt_u (i32.add (local.get $lens_index) (local.get $repeat_count)) (local.get $total_lens))
                      (then (return (i32.const 3))))
                    (local.set $prev_len (i32.const 0))
                    (local.set $lens_index (i32.add (local.get $lens_index) (local.get $repeat_count)))
                    (br $read_dyn_lens)))
                (return (i32.const 3)))))
          (if (i32.eqz (i32.load8_u (i32.add (i32.const 591724) (i32.const 256))))
            (then (return (i32.const 3))))
          (if (call $build_table
                (i32.const 591724)
                (local.get $hlit)
                (i32.const 590208)
                (i32.const 590080)
                (i32.const 590144)
                (i32.const 15))
            (then (return (i32.const 3))))
          (if (call $build_table
                (i32.add (i32.const 591724) (local.get $hlit))
                (local.get $hdist)
                (i32.const 591552)
                (i32.const 591424)
                (i32.const 591488)
                (i32.const 15))
            (then (return (i32.const 3))))
          (loop $dynamic
            (local.set $sym (call $decode_symbol (i32.const 590208) (i32.const 590080) (i32.const 15)))
            (if (i32.lt_s (local.get $sym) (i32.const 0))
              (then
                (if (i32.eq (local.get $sym) (i32.const -1))
                  (then (return (i32.const 1)))
                  (else (return (i32.const 3))))))
            (if (i32.lt_u (local.get $sym) (i32.const 256))
              (then
                (if (i32.ge_u (local.get $written) (local.get $out_cap)) (then (return (i32.const 2))))
                (if (i32.ge_u (local.get $written) (local.get $out_limit)) (then (return (i32.const 6))))
                (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $sym))
                (local.set $crc (call $crc32_update_byte (local.get $crc) (local.get $sym)))
                (local.set $adler (call $adler32_update_byte (local.get $adler) (local.get $sym)))
                (local.set $written (i32.add (local.get $written) (i32.const 1)))
                (br $dynamic)))
            (if (i32.eq (local.get $sym) (i32.const 256))
              (then
                (local.set $block_count (i32.add (local.get $block_count) (i32.const 1)))
                (if (local.get $bfinal)
                  (then
                    (call $m66write_record
                      (local.get $out_record)
                      (call $br_consumed_bytes)
                      (local.get $written)
                      (local.get $block_count)
                      (i32.const 2)
                      (i32.xor (local.get $crc) (i32.const -1))
                      (local.get $adler))
                    (return (i32.const 0))))
                (br $blocks)))
            (if (i32.gt_u (local.get $sym) (i32.const 285)) (then (return (i32.const 3))))
            (local.set $length (call $length_base (local.get $sym)))
            (local.set $extra_bits (call $length_extra (local.get $sym)))
            (if (i32.lt_s (local.get $length) (i32.const 0)) (then (return (i32.const 3))))
            (if (local.get $extra_bits)
              (then
                (local.set $extra_val (call $br_read (local.get $extra_bits)))
                (if (i32.lt_s (local.get $extra_val) (i32.const 0)) (then (return (i32.const 1))))
                (local.set $length (i32.add (local.get $length) (local.get $extra_val)))))
            (local.set $dist_sym (call $decode_symbol (i32.const 591552) (i32.const 591424) (i32.const 15)))
            (if (i32.lt_s (local.get $dist_sym) (i32.const 0))
              (then
                (if (i32.eq (local.get $dist_sym) (i32.const -1))
                  (then (return (i32.const 1)))
                  (else (return (i32.const 3))))))
            (if (i32.gt_u (local.get $dist_sym) (i32.const 29)) (then (return (i32.const 3))))
            (local.set $dist (call $dist_base (local.get $dist_sym)))
            (local.set $extra_bits (call $dist_extra (local.get $dist_sym)))
            (if (local.get $extra_bits)
              (then
                (local.set $extra_val (call $br_read (local.get $extra_bits)))
                (if (i32.lt_s (local.get $extra_val) (i32.const 0)) (then (return (i32.const 1))))
                (local.set $dist (i32.add (local.get $dist) (local.get $extra_val)))))
            (if (i32.or
                  (i32.gt_u (local.get $dist) (local.get $written))
                  (i32.gt_u (local.get $dist) (i32.const 32768)))
              (then (return (i32.const 3))))
            (if (i32.gt_u (i32.add (local.get $written) (local.get $length)) (local.get $out_cap)) (then (return (i32.const 2))))
            (if (i32.gt_u (i32.add (local.get $written) (local.get $length)) (local.get $out_limit)) (then (return (i32.const 6))))
            (local.set $i (i32.const 0))
            (loop $copy_dyn_match
              (if (i32.lt_u (local.get $i) (local.get $length))
                (then
                  (local.set $copy_from
                    (i32.sub
                      (i32.add (local.get $written) (local.get $i))
                      (local.get $dist)))
                  (local.set $byte (i32.load8_u (i32.add (local.get $out_ptr) (local.get $copy_from))))
                  (i32.store8
                    (i32.add (local.get $out_ptr) (i32.add (local.get $written) (local.get $i)))
                    (local.get $byte))
                  (local.set $crc (call $crc32_update_byte (local.get $crc) (local.get $byte)))
                  (local.set $adler (call $adler32_update_byte (local.get $adler) (local.get $byte)))
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $copy_dyn_match))))
            (local.set $written (i32.add (local.get $written) (local.get $length)))
            (br $dynamic))))
      (if (i32.eq (local.get $btype) (i32.const 1))
        (then
          (loop $fixed
            (local.set $packed (call $fixed_symbol))
            (if (i32.lt_s (local.get $packed) (i32.const 0))
              (then
                (if (i32.eq (local.get $packed) (i32.const -1))
                  (then (return (i32.const 1)))
                  (else (return (i32.const 3))))))
            (local.set $sym (i32.and (local.get $packed) (i32.const 65535)))
            (if (i32.lt_u (local.get $sym) (i32.const 256))
              (then
                (if (i32.ge_u (local.get $written) (local.get $out_cap)) (then (return (i32.const 2))))
                (if (i32.ge_u (local.get $written) (local.get $out_limit)) (then (return (i32.const 6))))
                (i32.store8 (i32.add (local.get $out_ptr) (local.get $written)) (local.get $sym))
                (local.set $crc (call $crc32_update_byte (local.get $crc) (local.get $sym)))
                (local.set $adler (call $adler32_update_byte (local.get $adler) (local.get $sym)))
                (local.set $written (i32.add (local.get $written) (i32.const 1)))
                (br $fixed)))
            (if (i32.eq (local.get $sym) (i32.const 256))
              (then
                (local.set $block_count (i32.add (local.get $block_count) (i32.const 1)))
                (if (local.get $bfinal)
                  (then
                    (call $m66write_record
                      (local.get $out_record)
                      (call $br_consumed_bytes)
                      (local.get $written)
                      (local.get $block_count)
                      (i32.const 1)
                      (i32.xor (local.get $crc) (i32.const -1))
                      (local.get $adler))
                    (return (i32.const 0))))
                (br $blocks)))
            (if (i32.gt_u (local.get $sym) (i32.const 285)) (then (return (i32.const 3))))
            (local.set $length (call $length_base (local.get $sym)))
            (local.set $extra_bits (call $length_extra (local.get $sym)))
            (if (i32.lt_s (local.get $length) (i32.const 0)) (then (return (i32.const 3))))
            (if (local.get $extra_bits)
              (then
                (local.set $extra_val (call $br_read (local.get $extra_bits)))
                (if (i32.lt_s (local.get $extra_val) (i32.const 0)) (then (return (i32.const 1))))
                (local.set $length (i32.add (local.get $length) (local.get $extra_val)))))
            (local.set $dist_sym (call $br_read (i32.const 5)))
            (if (i32.lt_s (local.get $dist_sym) (i32.const 0)) (then (return (i32.const 1))))
            (local.set $dist_sym (call $reverse_bits (local.get $dist_sym) (i32.const 5)))
            (if (i32.gt_u (local.get $dist_sym) (i32.const 29)) (then (return (i32.const 3))))
            (local.set $dist (call $dist_base (local.get $dist_sym)))
            (local.set $extra_bits (call $dist_extra (local.get $dist_sym)))
            (if (local.get $extra_bits)
              (then
                (local.set $extra_val (call $br_read (local.get $extra_bits)))
                (if (i32.lt_s (local.get $extra_val) (i32.const 0)) (then (return (i32.const 1))))
                (local.set $dist (i32.add (local.get $dist) (local.get $extra_val)))))
            (if (i32.or
                  (i32.gt_u (local.get $dist) (local.get $written))
                  (i32.gt_u (local.get $dist) (i32.const 32768)))
              (then (return (i32.const 3))))
            (if (i32.gt_u (i32.add (local.get $written) (local.get $length)) (local.get $out_cap)) (then (return (i32.const 2))))
            (if (i32.gt_u (i32.add (local.get $written) (local.get $length)) (local.get $out_limit)) (then (return (i32.const 6))))
            (local.set $i (i32.const 0))
            (loop $copy_match
              (if (i32.lt_u (local.get $i) (local.get $length))
                (then
                  (local.set $copy_from
                    (i32.sub
                      (i32.add (local.get $written) (local.get $i))
                      (local.get $dist)))
                  (local.set $byte (i32.load8_u (i32.add (local.get $out_ptr) (local.get $copy_from))))
                  (i32.store8
                    (i32.add (local.get $out_ptr) (i32.add (local.get $written) (local.get $i)))
                    (local.get $byte))
                  (local.set $crc (call $crc32_update_byte (local.get $crc) (local.get $byte)))
                  (local.set $adler (call $adler32_update_byte (local.get $adler) (local.get $byte)))
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $copy_match))))
            (local.set $written (i32.add (local.get $written) (local.get $length)))
            (br $fixed))))
      (if (i32.ne (local.get $btype) (i32.const 0))
        (then
          (call $m66write_record
            (local.get $out_record)
            (call $br_consumed_bytes)
            (local.get $written)
            (local.get $block_count)
            (local.get $btype)
            (i32.xor (local.get $crc) (i32.const -1))
            (local.get $adler))
          (return (i32.const 7))))
      (call $br_align_byte)
      (local.set $off (i32.shr_u (global.get $br_bitpos) (i32.const 3)))
      (if (i32.gt_u (i32.add (local.get $off) (i32.const 4)) (local.get $src_len)) (then (return (i32.const 1))))
      (local.set $block_len
        (i32.or
          (i32.load8_u (i32.add (local.get $src_ptr) (local.get $off)))
          (i32.shl
            (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 1))))
            (i32.const 8))))
      (local.set $nlen
        (i32.or
          (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 2))))
          (i32.shl
            (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $off) (i32.const 3))))
            (i32.const 8))))
      (if (i32.ne (i32.and (i32.xor (local.get $block_len) (local.get $nlen)) (i32.const 65535)) (i32.const 65535))
        (then (return (i32.const 3))))
      (local.set $payload_off (i32.add (local.get $off) (i32.const 4)))
      (local.set $payload_end (i32.add (local.get $payload_off) (local.get $block_len)))
      (if (i32.or
            (i32.lt_u (local.get $payload_end) (local.get $payload_off))
            (i32.gt_u (local.get $payload_end) (local.get $src_len)))
        (then (return (i32.const 1))))
      (if (i32.gt_u (i32.add (local.get $written) (local.get $block_len)) (local.get $out_cap)) (then (return (i32.const 2))))
      (if (i32.gt_u (i32.add (local.get $written) (local.get $block_len)) (local.get $out_limit)) (then (return (i32.const 6))))
      (local.set $i (i32.const 0))
      (loop $copy
        (if (i32.lt_u (local.get $i) (local.get $block_len))
          (then
            (local.set $byte (i32.load8_u (i32.add (local.get $src_ptr) (i32.add (local.get $payload_off) (local.get $i)))))
            (i32.store8 (i32.add (local.get $out_ptr) (i32.add (local.get $written) (local.get $i))) (local.get $byte))
            (local.set $crc (call $crc32_update_byte (local.get $crc) (local.get $byte)))
            (local.set $adler (call $adler32_update_byte (local.get $adler) (local.get $byte)))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $copy))))
      (local.set $written (i32.add (local.get $written) (local.get $block_len)))
      (local.set $block_count (i32.add (local.get $block_count) (i32.const 1)))
      (global.set $br_bitpos (i32.shl (local.get $payload_end) (i32.const 3)))
      (global.set $br_bitbuf (i32.const 0))
      (global.set $br_bitcnt (i32.const 0))
      (if (local.get $bfinal)
        (then
          (call $m66write_record
            (local.get $out_record)
            (local.get $payload_end)
            (local.get $written)
            (local.get $block_count)
            (i32.const 0)
            (i32.xor (local.get $crc) (i32.const -1))
            (local.get $adler))
          (return (i32.const 0))))
      (br $blocks))
    (i32.const 1))

  ;; Pipe Core — byte pipes + bump allocators

    ;; Standard ID removed — merged into single module

  ;; ── Pipe struct layout (16 byte header + data[cap]) ──
  ;; +0:  rd    read cursor from data start
  ;; +4:  wr    write cursor from data start
  ;; +8:  cap   total data capacity
  ;; +12: state/mode 0=open linear, 1=closed, 2=open circular, 3=closed circular
  ;; +16: data[cap]
  (global $PIPE_RD     i32 (i32.const 0))
  (global $PIPE_WR     i32 (i32.const 4))
  (global $PIPE_CAP    i32 (i32.const 8))
  (global $PIPE_STATE  i32 (i32.const 12))
  (global $PIPE_HEADER i32 (i32.const 16))
  (global $PIPE_MODE_CIRCULAR i32 (i32.const 2))
  (global $PIPE_MODE_MASK    i32 (i32.const 2))
  (global $PIPE_CLOSED       i32 (i32.const 1))

  ;; ── Bump allocator: 0x40000 – 0x80000 ──
  (global $HEAP_START i32 (i32.const 0x40000))
  (global $HEAP_END   i32 (i32.const 0x80000))
  (global $heap_ptr (mut i32) (i32.const 0x40000))

  (func $pipe_alloc (export "pipe_alloc") (param $size i32) (result i32)
    (local $ptr i32)
    (local.set $ptr (global.get $heap_ptr))
    (if (i32.gt_u (i32.add (local.get $ptr) (local.get $size)) (global.get $HEAP_END))
      (then (return (i32.const -1))))
    (global.set $heap_ptr (i32.add (global.get $heap_ptr) (local.get $size)))
    local.get $ptr)

  ;; ── Pipe creation ──

  (func $pipe_create (export "pipe_create") (param $cap i32) (result i32)
    (local $p i32)
    (if (i32.eqz (local.get $cap)) (then (return (i32.const -1))))
    (local.set $p (call $pipe_alloc
      (i32.add (global.get $PIPE_HEADER) (local.get $cap))))
    (if (i32.eq (local.get $p) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $p) (i32.const 0))
    (i32.store offset=4 (local.get $p) (i32.const 0))
    (i32.store offset=8 (local.get $p) (local.get $cap))
    (i32.store offset=12 (local.get $p) (i32.const 0))
    local.get $p)

  ;; pipe_create_aligned(cap, align) — capacity rounded up to align multiple
  (func $pipe_create_aligned (export "pipe_create_aligned") (param $cap i32) (param $align i32) (result i32)
    (local $aligned i32)
    (if (i32.eqz (local.get $cap)) (then (return (i32.const -1))))
    (if (i32.le_u (local.get $align) (i32.const 1))
      (then (return (call $pipe_create (local.get $cap)))))
    (local.set $aligned
      (i32.mul
        (i32.div_u
          (i32.add (local.get $cap) (i32.sub (local.get $align) (i32.const 1)))
          (local.get $align))
        (local.get $align)))
    (call $pipe_create (local.get $aligned)))

  (func (export "pipe_set_mode") (param $p i32) (param $mode i32)
    (i32.store offset=12 (local.get $p)
      (i32.or (i32.load offset=12 (local.get $p)) (local.get $mode))))

  ;; ── Helper: available bytes (handles circular wrap) ──
  (func $pipe_fill (param $p i32) (result i32)
    (local $rd i32) (local $wr i32) (local $cap i32)
    (local.set $rd (i32.load offset=0 (local.get $p)))
    (local.set $wr (i32.load offset=4 (local.get $p)))
    (if (result i32) (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR))
      (then
        (local.set $cap (i32.load offset=8 (local.get $p)))
        (if (result i32) (i32.ge_u (local.get $wr) (local.get $rd))
          (then (i32.sub (local.get $wr) (local.get $rd)))
          (else (i32.sub (i32.add (local.get $cap) (local.get $wr)) (local.get $rd)))))
      (else (i32.sub (local.get $wr) (local.get $rd)))))

  ;; ── Helper: space available (cap - fill) ──
  (func $pipe_room (param $p i32) (result i32)
    (i32.sub (i32.load offset=8 (local.get $p)) (call $pipe_fill (local.get $p))))

  ;; ── Write ──

  (func $pipe_write (export "pipe_write")
    (param $p i32) (param $src i32) (param $len i32) (result i32)
    (local $wr i32) (local $cap i32)
    (if (i32.eqz (local.get $len)) (then (return (global.get $STATUS_OK))))
    (if (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR))
      (then (return (call $pipe_write_circ (local.get $p) (local.get $src) (local.get $len)))))
    (local.set $wr (i32.load offset=4 (local.get $p)))
    (local.set $cap (i32.load offset=8 (local.get $p)))
    (if (i32.gt_u (i32.add (local.get $wr) (local.get $len)) (local.get $cap))
      (then (return (global.get $STATUS_OVERFLOW))))
    (call $memcpy_off
      (i32.add (local.get $p) (global.get $PIPE_HEADER))
      (local.get $wr)
      (local.get $src)
      (i32.const 0)
      (local.get $len))
    (i32.store offset=4 (local.get $p)
      (i32.add (local.get $wr) (local.get $len)))
    (global.get $STATUS_OK))

  (func $pipe_write_circ (param $p i32) (param $src i32) (param $len i32) (result i32)
    (local $wr i32) (local $rd i32) (local $cap i32) (local $fill i32)
    (local $off i32) (local $remain i32) (local $base i32)
    (local.set $rd (i32.load offset=0 (local.get $p)))
    (local.set $wr (i32.load offset=4 (local.get $p)))
    (local.set $cap (i32.load offset=8 (local.get $p)))
    (if (i32.ge_u (local.get $wr) (local.get $rd))
      (then (local.set $fill (i32.sub (local.get $wr) (local.get $rd))))
      (else (local.set $fill (i32.sub (i32.add (local.get $cap) (local.get $wr)) (local.get $rd)))))
    (if (i32.gt_u (i32.add (local.get $fill) (local.get $len)) (local.get $cap))
      (then (return (global.get $STATUS_OVERFLOW))))
    (local.set $off (local.get $wr))
    (local.set $remain (i32.sub (local.get $cap) (local.get $off)))
    (local.set $base (i32.add (local.get $p) (global.get $PIPE_HEADER)))
    (if (i32.le_u (local.get $len) (local.get $remain))
      (then
        (call $memcpy_off (local.get $base) (local.get $off) (local.get $src) (i32.const 0) (local.get $len))
        (i32.store offset=4 (local.get $p) (i32.add (local.get $off) (local.get $len))))
      (else
        (call $memcpy_off (local.get $base) (local.get $off) (local.get $src) (i32.const 0) (local.get $remain))
        (call $memcpy_off (local.get $base) (i32.const 0) (local.get $src) (local.get $remain) (i32.sub (local.get $len) (local.get $remain)))
        (i32.store offset=4 (local.get $p) (i32.sub (local.get $len) (local.get $remain)))))
    global.get $STATUS_OK)

  ;; ── Read ──

  (func $pipe_read (export "pipe_read")
    (param $p i32) (param $dst i32) (param $max i32) (result i32)
    (local $avail i32) (local $rd i32) (local $n i32)
    (local.set $avail (call $pipe_fill (local.get $p)))
    (if (i32.eqz (local.get $avail)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $avail) (local.get $max))
      (then (local.set $avail (local.get $max))))
    (if (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR))
      (then (return (call $pipe_read_circ (local.get $p) (local.get $dst) (local.get $avail)))))
    (local.set $rd (i32.load offset=0 (local.get $p)))
    (call $memcpy_off
      (local.get $dst) (i32.const 0)
      (i32.add (local.get $p) (global.get $PIPE_HEADER))
      (local.get $rd)
      (local.get $avail))
    (i32.store offset=0 (local.get $p)
      (i32.add (local.get $rd) (local.get $avail)))
    (if (i32.eq (i32.load offset=0 (local.get $p))
                (i32.load offset=4 (local.get $p)))
      (then
        (i32.store offset=0 (local.get $p) (i32.const 0))
        (i32.store offset=4 (local.get $p) (i32.const 0))))
    local.get $avail)
  (func $pipe_read_circ (param $p i32) (param $dst i32) (param $max i32) (result i32)
    (local $rd i32) (local $cap i32) (local $base i32)
    (local $remain i32) (local $n i32)
    (local.set $rd (i32.load offset=0 (local.get $p)))
    (local.set $cap (i32.load offset=8 (local.get $p)))
    (local.set $base (i32.add (local.get $p) (global.get $PIPE_HEADER)))
    (local.set $remain (i32.sub (local.get $cap) (local.get $rd)))
    (if (i32.le_u (local.get $max) (local.get $remain))
      (then
        (call $memcpy_off (local.get $dst) (i32.const 0) (local.get $base) (local.get $rd) (local.get $max))
        (i32.store offset=0 (local.get $p) (i32.add (local.get $rd) (local.get $max))))
      (else
        (call $memcpy_off (local.get $dst) (i32.const 0) (local.get $base) (local.get $rd) (local.get $remain))
        (local.set $n (i32.sub (local.get $max) (local.get $remain)))
        (call $memcpy_off (local.get $dst) (local.get $remain) (local.get $base) (i32.const 0) (local.get $n))
        (i32.store offset=0 (local.get $p) (local.get $n))))
    local.get $max)

  ;; ── Zero-copy read ──
  ;; Returns pointer to readable data (first contiguous segment).
  ;; For linear pipes: direct pointer to pipe data buffer + rd offset.
  ;; For circular pipes: only returns the segment from rd to end-of-buffer;
  ;;   caller must handle wrap-around or use pipe_read for full copy.
  ;; Writes available byte count to [len_ptr].
  (func $pipe_read_ptr (export "pipe_read_ptr") (param $p i32) (param $len_ptr i32) (result i32)
    (local $avail i32)
    (local.set $avail (call $pipe_fill (local.get $p)))
    (if (i32.eqz (local.get $avail))
      (then
        (i32.store (local.get $len_ptr) (i32.const 0))
        (return (i32.const 0))))
    (if (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR))
      (then
        ;; Cap at first contiguous segment (rd → cap)
        (local.set $avail
          (call $min_u (local.get $avail)
            (i32.sub (i32.load offset=8 (local.get $p))
                     (i32.load offset=0 (local.get $p)))))))
    (i32.store (local.get $len_ptr) (local.get $avail))
    (i32.add (i32.add (local.get $p) (global.get $PIPE_HEADER))
             (i32.load offset=0 (local.get $p))))

  ;; Advance read cursor by n bytes after zero-copy read.
  (func $pipe_advance (export "pipe_advance") (param $p i32) (param $n i32)
    (local $rd i32) (local $cap i32)
    (if (i32.eqz (local.get $n)) (then (return)))
    (local.set $rd (i32.load offset=0 (local.get $p)))
    (if (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR))
      (then
        (local.set $cap (i32.load offset=8 (local.get $p)))
        (local.set $rd (i32.add (local.get $rd) (local.get $n)))
        (if (i32.ge_u (local.get $rd) (local.get $cap))
          (then (local.set $rd (i32.sub (local.get $rd) (local.get $cap)))))
        (i32.store offset=0 (local.get $p) (local.get $rd)))
      (else
        (i32.store offset=0 (local.get $p)
          (i32.add (local.get $rd) (local.get $n)))
        (if (i32.eq (i32.load offset=0 (local.get $p))
                    (i32.load offset=4 (local.get $p)))
          (then
            (i32.store offset=0 (local.get $p) (i32.const 0))
            (i32.store offset=4 (local.get $p) (i32.const 0)))))))

  (func $min_u (param $a i32) (param $b i32) (result i32)
    (if (result i32) (i32.lt_u (local.get $a) (local.get $b))
      (then (local.get $a))
      (else (local.get $b))))

  ;; ── Query ──

  (func $pipe_available (export "pipe_available") (param $p i32) (result i32)
    (call $pipe_fill (local.get $p)))

  (func (export "pipe_space") (param $p i32) (result i32)
    (call $pipe_room (local.get $p)))

  (func (export "pipe_state") (param $p i32) (result i32)
    (i32.load offset=12 (local.get $p)))

  ;; ── Close / Reset ──

  (func $pipe_close (export "pipe_close") (param $p i32)
    (i32.store offset=12 (local.get $p)
      (i32.or (global.get $PIPE_CLOSED)
              (i32.and (i32.load offset=12 (local.get $p)) (global.get $PIPE_MODE_CIRCULAR)))))

  ;; ── Heap lifecycle management ──
  ;; Snapshot saves current heap pointer; Restore rolls back to snapshot.
  ;; Intermediate allocations (pipes, nodes) above the snapshot are freed.
  ;; Persistent allocations below the snapshot are preserved.
  (func $pipe_snapshot (export "pipe_snapshot") (result i32)
    (global.get $heap_ptr))

  (func $pipe_restore (export "pipe_restore") (param $snap i32)
    (global.set $heap_ptr (local.get $snap)))

  (func (export "pipe_reset_heap")
    (global.set $heap_ptr (global.get $HEAP_START)))

  (func (export "pipe_reset") (param $p i32)
    (i32.store offset=0 (local.get $p) (i32.const 0))
    (i32.store offset=4 (local.get $p) (i32.const 0))
    (i32.store offset=12 (local.get $p) (i32.const 0)))

  ;; ── Pipe drain ──

  (func $pipe_drain (export "pipe_drain")
    (param $src i32) (param $dst i32) (param $tmp i32) (param $tcap i32) (result i32)
    (local $n i32)
    (local.set $n (call $pipe_read (local.get $src) (local.get $tmp) (local.get $tcap)))
    (if (i32.le_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))
    (call $pipe_write (local.get $dst) (local.get $tmp) (local.get $n))
    (return (local.get $n)))

  ;; memcpy imported from edgerun-core as $memcpy_off(dst, doff, src, soff, len)

  ;; Frame Core — framed I/O (8-byte header)

    ;; Standard ID removed — merged into single module

  ;; ── Shared header scratch ──
  (global $HDR i32 (i32.const 0x3FFF0))

  ;; ── Frame: [stream_id:u32_le][payload_len:u32_le][payload] ──

  ;; frame_write(pipe, stream_id, data, len) → status
  (func $frame_write (export "frame_write")
    (param $pipe i32) (param $stream_id i32) (param $data i32) (param $len i32) (result i32)
    (local $r i32)
    (i32.store (global.get $HDR) (local.get $stream_id))
    (i32.store offset=4 (global.get $HDR) (local.get $len))
    (local.set $r (call $pipe_write (local.get $pipe) (global.get $HDR) (i32.const 8)))
    (if (i32.ne (local.get $r) (global.get $STATUS_OK)) (then (return (local.get $r))))
    (call $pipe_write (local.get $pipe) (local.get $data) (local.get $len)))

  ;; frame_read(pipe, scratch, scap) → pack(status, stream_id)
  ;; On success: stores payload_len at scratch[0..4], payload at scratch[4..4+payload_len)
  ;; scratch must be at least payload_len + 4 bytes
  (func $frame_read (export "frame_read")
    (param $pipe i32) (param $scratch i32) (param $scap i32) (result i64)
    (local $r i32) (local $avail i32) (local $stream_id i32) (local $payload_len i32)
    (local $remaining i32) (local $chunk i32)
    ;; Read 8-byte header
    (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (i32.const 8)))
    (if (i32.lt_s (local.get $r) (i32.const 8))
      (then (return (call $pack (global.get $STATUS_INPUT_SHORT) (i32.const 0)))))
    (local.set $stream_id (i32.load (local.get $scratch)))
    (local.set $payload_len (i32.load offset=4 (local.get $scratch)))
    ;; Check space
    (if (i32.gt_u (i32.add (local.get $payload_len) (i32.const 4)) (local.get $scap))
      (then
        ;; Skip payload in chunks using scratch as temp buffer (avoids HDR overflow)
        (local.set $remaining (local.get $payload_len))
        (block $skip_done
          (loop $skip_loop
            (br_if $skip_done (i32.eqz (local.get $remaining)))
            (local.set $chunk (local.get $remaining))
            (if (i32.gt_u (local.get $chunk) (local.get $scap))
              (then (local.set $chunk (local.get $scap))))
            (drop (call $pipe_read (local.get $pipe) (local.get $scratch) (local.get $chunk)))
            (local.set $remaining (i32.sub (local.get $remaining) (local.get $chunk)))
            (br $skip_loop)))
        (return (call $pack (global.get $STATUS_OVERFLOW) (local.get $stream_id)))))
    ;; Write payload_len to scratch[0..4], payload to scratch[4..]
    (i32.store (local.get $scratch) (local.get $payload_len))
    (local.set $r (call $pipe_read (local.get $pipe)
      (i32.add (local.get $scratch) (i32.const 4)) (local.get $payload_len)))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (call $pack (i32.sub (i32.const 0) (local.get $r)) (i32.const 0)))))
    (call $pack (global.get $STATUS_OK) (local.get $stream_id)))

  ;; ── Convenience: read frame payload directly to output pipe ──
  ;; frame_route(pipe, stream_dst, scratch) → pack(status, stream_id)
  ;; Reads header from pipe, then reads payload directly to stream_dst
  ;; scratch[0..7] used for header only
  (func (export "frame_route")
    (param $pipe i32) (param $stream_dst i32) (param $scratch i32) (result i64)
    (local $r i32) (local $stream_id i32) (local $payload_len i32)
    (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (i32.const 8)))
    (if (i32.lt_s (local.get $r) (i32.const 8))
      (then (return (call $pack (global.get $STATUS_INPUT_SHORT) (i32.const 0)))))
    (local.set $stream_id (i32.load (local.get $scratch)))
    (local.set $payload_len (i32.load offset=4 (local.get $scratch)))
    ;; Read payload directly to output pipe via scratch in chunks
    (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (local.get $payload_len)))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (call $pack (i32.sub (i32.const 0) (local.get $r)) (i32.const 0)))))
    (drop (call $pipe_write (local.get $stream_dst) (local.get $scratch) (local.get $payload_len)))
    (return (call $pack (global.get $STATUS_OK) (local.get $stream_id))))

  ;; ── Message queue: [len:u32_le][payload] ──

  (func (export "msg_write")
    (param $pipe i32) (param $data i32) (param $len i32) (result i32)
    (local $r i32)
    (i32.store (global.get $HDR) (local.get $len))
    (local.set $r (call $pipe_write (local.get $pipe) (global.get $HDR) (i32.const 4)))
    (if (i32.ne (local.get $r) (global.get $STATUS_OK)) (then (return (local.get $r))))
    (call $pipe_write (local.get $pipe) (local.get $data) (local.get $len)))

  ;; msg_read(pipe, scratch, scap) → payload_len (bytes in scratch) | 0 | negative error
  ;; On success: stores payload_len at scratch[0..4], payload at scratch[4..4+payload_len)
  (func (export "msg_read")
    (param $pipe i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $r i32) (local $payload_len i32)
    (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (i32.const 4)))
    (if (i32.lt_s (local.get $r) (i32.const 4))
      (then (return (i32.const 0))))
    (local.set $payload_len (i32.load (local.get $scratch)))
    (if (i32.gt_u (i32.add (local.get $payload_len) (i32.const 4)) (local.get $scap))
      (then (return (i32.sub (i32.const 0) (global.get $STATUS_OVERFLOW)))))
    (i32.store (local.get $scratch) (local.get $payload_len))
    (local.set $r (call $pipe_read (local.get $pipe)
      (i32.add (local.get $scratch) (i32.const 4)) (local.get $payload_len)))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then (return (local.get $r))))
    local.get $payload_len)

  ;; Pipeline Core — pipeline_run + dispatch table

    ;; Standard ID removed — merged into single module

  ;; ── Stage dispatch table ──
  (table (export "stage_table") 64 funcref)

  ;; ── Stage type constants (dispatch table indices) ──
  (func (export "STAGE_PASSTHROUGH") (result i32) i32.const 0)
  (func (export "STAGE_HEX_ENCODE")  (result i32) i32.const 1)
  (func (export "STAGE_HEX_DECODE")  (result i32) i32.const 2)
  (func (export "STAGE_B64_ENCODE")  (result i32) i32.const 3)
  (func (export "STAGE_B64_DECODE")  (result i32) i32.const 4)
  (func (export "STAGE_TRANSPORT")   (result i32) i32.const 5)
  (func (export "STAGE_MUX_STATIC")  (result i32) i32.const 6)
  (func (export "STAGE_DEMUX_STATIC") (result i32) i32.const 7)
  (func (export "STAGE_MUX_DYNAMIC") (result i32) i32.const 8)
  (func (export "STAGE_DEMUX_DYNAMIC") (result i32) i32.const 9)
  (func (export "STAGE_WS_FRAME")     (result i32) i32.const 10)
  (func (export "STAGE_WS_ENCODE")    (result i32) i32.const 11)
  (func (export "STAGE_WS_DECODE")    (result i32) i32.const 12)
  (func (export "STAGE_EXEC")         (result i32) i32.const 13)
  (func (export "STAGE_FRAME_PACER")  (result i32) i32.const 15)

  ;; ── Stage function type ──
  ;; (input_pipe, output_pipe, config_ptr, config_len, scratch, scap, state_ptr) -> result
  (type $stage_fn (func (param i32 i32 i32 i32 i32 i32 i32) (result i32)))

  ;; ── Pipeline descriptor layout ──
  ;; +0:  magic      i32
  ;; +4:  version    i32
  ;; +8:  pipe_cap   i32  — intermediate pipe capacity
  ;; +12: stage_count i32
  ;; +16: tick       i32  — incremented per pipeline_run call
  ;; +20: stages[] — each:
  ;;   +0:  stage_type i32
  ;;   +4:  config     i32
  ;;   +8:  config_len i32
  ;;   +12: state_ptr  i32
  ;;   total: 16 bytes
  (func (export "PIPELINE_MAGIC")   (result i32) i32.const 0x50495045)
  (func (export "PIPELINE_VERSION") (result i32) i32.const 1)

  (global $PD_MAGIC    i32 (i32.const 0))
  (global $PD_VERSION  i32 (i32.const 4))
  (global $PD_PIPE_CAP i32 (i32.const 8))
  (global $PD_COUNT    i32 (i32.const 12))
  (global $PD_TICK     i32 (i32.const 16))
  (global $PD_FRAME    i32 (i32.const 20))
  (global $PD_STAGES   i32 (i32.const 24))
  (global $PS_TYPE     i32 (i32.const 0))
  (global $PS_CONFIG   i32 (i32.const 4))
  (global $PS_CLEN     i32 (i32.const 8))
  (global $PS_STATE    i32 (i32.const 12))
  (global $PS_SIZE     i32 (i32.const 16))

  ;; pipeline_create(pipe_cap, stage_count) → desc_ptr | -1
  (func (export "pipeline_create") (param $pcap i32) (param $count i32) (result i32)
    (local $desc i32) (local $sz i32)
    (local.set $sz (i32.add (global.get $PD_STAGES)
      (i32.mul (local.get $count) (global.get $PS_SIZE))))
    (local.set $desc (call $pipe_alloc (local.get $sz)))
    (if (i32.eq (local.get $desc) (i32.const -1))
      (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $desc) (i32.const 0x50495045))
    (i32.store offset=4 (local.get $desc) (i32.const 1))
    (i32.store offset=8 (local.get $desc) (local.get $pcap))
    (i32.store offset=12 (local.get $desc) (local.get $count))
    (i32.store offset=16 (local.get $desc) (i32.const 0))
    (i32.store offset=20 (local.get $desc) (i32.const 0))
    local.get $desc)

  ;; pipeline_set_stage(desc, index, stage_type, config, config_len)
  (func (export "pipeline_set_stage")
    (param $desc i32) (param $idx i32) (param $stype i32)
    (param $cfg i32) (param $clen i32)
    (local $slot i32)
    (local.set $slot
      (i32.add (global.get $PD_STAGES)
        (i32.mul (local.get $idx) (global.get $PS_SIZE))))
    (i32.store (i32.add (local.get $desc) (local.get $slot)) (local.get $stype))
    (i32.store offset=4 (i32.add (local.get $desc) (local.get $slot)) (local.get $cfg))
    (i32.store offset=8 (i32.add (local.get $desc) (local.get $slot)) (local.get $clen)))

  ;; pipeline_set_stage_state(desc, index, state_ptr)
  (func (export "pipeline_set_stage_state")
    (param $desc i32) (param $idx i32) (param $state i32)
    (local $slot i32)
    (local.set $slot
      (i32.add (global.get $PD_STAGES)
        (i32.mul (local.get $idx) (global.get $PS_SIZE))))
    (i32.store offset=12 (i32.add (local.get $desc) (local.get $slot)) (local.get $state)))

  (func (export "pipeline_get_stage_type") (param $desc i32) (param $idx i32) (result i32)
    (local $slot i32)
    (local.set $slot
      (i32.add (global.get $PD_STAGES)
        (i32.mul (local.get $idx) (global.get $PS_SIZE))))
    (i32.load (i32.add (local.get $desc) (local.get $slot))))

  (func (export "pipeline_get_tick") (param $desc i32) (result i32)
    (i32.load offset=16 (local.get $desc)))

  (func (export "pipeline_set_frame_size") (param $desc i32) (param $frame i32)
    (i32.store offset=20 (local.get $desc) (local.get $frame)))

  (func (export "pipeline_get_frame_size") (param $desc i32) (result i32)
    (i32.load offset=20 (local.get $desc)))

  ;; pipeline_run(desc, input_pipe, output_pipe, scratch, scap) → OK | MORE | error
  ;; Fuses consecutive batch stages (state_ptr==0) by reusing a single intermediate pipe.
  ;; When frame_size > 0, intermediate pipes use aligned capacity for zero-copy SIMD.
  (func $pipeline_run (export "pipeline_run")
    (param $desc i32) (param $input i32) (param $output i32)
    (param $scratch i32) (param $scap i32) (result i32)
    (local $count i32) (local $pcap i32) (local $frame i32)
    (local $i i32) (local $stype i32)
    (local $out i32) (local $prev i32) (local $result i32)
    (local $stages i32) (local $slot i32)
    (local $cfg i32) (local $clen i32) (local $state i32)
    (local $snapshot i32) (local $last_i i32)
    (local $reusable i32) (local $in_batch i32)

    (local.set $count (i32.load offset=12 (local.get $desc)))
    (local.set $last_i (i32.sub (local.get $count) (i32.const 1)))
    (local.set $pcap (i32.load offset=8 (local.get $desc)))
    (local.set $frame (i32.load offset=20 (local.get $desc)))
    (local.set $stages (i32.add (local.get $desc) (global.get $PD_STAGES)))
    (local.set $prev (local.get $input))

    ;; Increment tick for this run
    (i32.store offset=16 (local.get $desc)
      (i32.add (i32.load offset=16 (local.get $desc)) (i32.const 1)))

    ;; Save heap snapshot before allocating intermediate pipes
    (local.set $snapshot (call $pipe_snapshot))

    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
        (local.set $stype (i32.load (i32.add (local.get $stages) (local.get $slot))))
        (local.set $cfg (i32.load offset=4 (i32.add (local.get $stages) (local.get $slot))))
        (local.set $clen (i32.load offset=8 (i32.add (local.get $stages) (local.get $slot))))
        (local.set $state (i32.load offset=12 (i32.add (local.get $stages) (local.get $slot))))

        ;; Write current tick to state[0] if stage has state
        (if (local.get $state)
          (then (i32.store (local.get $state) (i32.load offset=16 (local.get $desc)))))

        ;; Determine output pipe.
        ;; Optimisation: consecutive batch stages reuse a single intermediate pipe.
        (if (i32.eq (local.get $i) (local.get $last_i))
          (then
            (local.set $out (local.get $output))
            (local.set $in_batch (i32.const 0)))
          (else
            (if (i32.and (local.get $in_batch) (i32.eqz (local.get $state)))
              (then
                ;; Consecutive batch stage: reuse the same pipe as prev and out.
                ;; The stage reads from prev (draining it, auto-reset) then writes to out.
                (local.set $out (local.get $prev)))
              (else
                (if (local.get $frame)
                  (then
                    (local.set $out (call $pipe_create_aligned (local.get $pcap) (local.get $frame)))
                    (if (i32.eq (local.get $out) (i32.const -1))
                      (then (local.set $result (i32.const -1)) (br $done))))
                  (else
                    (local.set $out (call $pipe_create (local.get $pcap)))
                    (if (i32.eq (local.get $out) (i32.const -1))
                      (then (local.set $result (i32.const -1)) (br $done)))))
                (if (i32.eqz (local.get $state))
                  (then
                    (local.set $reusable (local.get $out))
                    (local.set $in_batch (i32.const 1))))))))

        ;; Call stage via dispatch table
        (local.set $result
          (call_indirect (type $stage_fn)
            (local.get $prev) (local.get $out) (local.get $cfg) (local.get $clen)
            (local.get $scratch) (local.get $scap) (local.get $state)
            (local.get $stype)))

        ;; Break on error or yield
        (if (i32.or
              (i32.lt_s (local.get $result) (i32.const 0))
              (i32.eq (local.get $result) (global.get $STATUS_MORE)))
          (then (br $done)))

        ;; Stage completed — reset result to OK for pipeline return value
        (local.set $result (global.get $STATUS_OK))

        ;; Close previous intermediate if it was a distinct pipe (not fused/reused)
        (if (i32.and (local.get $i) (i32.ne (local.get $prev) (local.get $out)))
          (then (call $pipe_close (local.get $prev))))

        ;; End batch run if current stage has state (streaming)
        (if (local.get $state)
          (then (local.set $in_batch (i32.const 0))))

        (local.set $prev (local.get $out))
        (local.set $slot (i32.add (local.get $slot) (global.get $PS_SIZE)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))

    ;; On full success, restore heap to free intermediate pipes.
    ;; On MORE (yield), intermediate pipes must persist for next call.
    (if (i32.eq (local.get $result) (global.get $STATUS_OK))
      (then (call $pipe_restore (local.get $snapshot))))
    local.get $result)

  ;; ── Built-in passthrough stage (table index 0) ──
  (func $stage_passthrough
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (call $pipe_drain (local.get $input) (local.get $output) (local.get $scratch) (local.get $scap)))

  (elem (i32.const 0) $stage_passthrough)

  ;; Frame Pacer Stage — tick-driven accumulation to frame_size chunks
  ;; Slots into any pipeline to provide frame-aligned data to downstream stages.
  ;;
  ;; State (80 bytes):
  ;;   +0:  tick          i32  — pipeline tick (RO, written by pipeline_run)
  ;;   +4:  frame_size    i32  — stride (from config[0])
  ;;   +8:  buf_len       i32  — bytes buffered so far
  ;;   +12: last_flush    i32  — tick when last frame was emitted
  ;;   +16: buf[64]             — internal buffer
  ;;
  ;; Config (4 bytes):
  ;;   +0: timeout_ticks i32  — flush partial frame after N idle ticks (0=never)

  (func $process_frame_pacer
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $tick i32) (local $frame_size i32) (local $buf_len i32) (local $last_flush i32)
    (local $timeout i32) (local $avail i32) (local $space i32) (local $read i32)
    (local $buf i32)

    (local.set $tick (i32.load offset=0 (local.get $state)))
    (local.set $frame_size (i32.load offset=4 (local.get $state)))
    (local.set $buf_len (i32.load offset=8 (local.get $state)))
    (local.set $last_flush (i32.load offset=12 (local.get $state)))
    (local.set $buf (i32.add (local.get $state) (i32.const 16)))
    (local.set $timeout (i32.load offset=0 (local.get $cfg)))

    ;; First call: init frame_size from config, yield
    (if (i32.eqz (local.get $frame_size))
      (then
        (local.set $frame_size (i32.load offset=0 (local.get $cfg)))
        (i32.store offset=4 (local.get $state) (local.get $frame_size))
        (return (global.get $STATUS_MORE))))

    ;; Accumulate input data into buffer
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.gt_u (local.get $avail) (i32.const 0))
      (then
        (local.set $space (i32.sub (local.get $frame_size) (local.get $buf_len)))
        (if (i32.gt_u (local.get $space) (i32.const 0))
          (then
            (local.set $read
              (call $pipe_read (local.get $input)
                (i32.add (local.get $buf) (local.get $buf_len))
                (call $min_u (local.get $space) (local.get $avail))))
            (local.set $buf_len (i32.add (local.get $buf_len) (local.get $read)))
            (i32.store offset=8 (local.get $state) (local.get $buf_len))))))

    ;; Full frame: write, shift, yield if more input
    (if (i32.ge_u (local.get $buf_len) (local.get $frame_size))
      (then
        (drop (call $pipe_write (local.get $output) (local.get $buf) (local.get $frame_size)))
        (local.set $buf_len (i32.sub (local.get $buf_len) (local.get $frame_size)))
        (if (i32.gt_u (local.get $buf_len) (i32.const 0))
          (then
            (call $memcpy_off
              (local.get $buf) (i32.const 0)
              (local.get $buf) (local.get $frame_size)
              (local.get $buf_len))))
        (i32.store offset=8 (local.get $state) (local.get $buf_len))
        (i32.store offset=12 (local.get $state) (local.get $tick))
        (if (call $pipe_available (local.get $input))
          (then (return (global.get $STATUS_MORE))))
        (return (global.get $STATUS_OK))))

    ;; Partial frame: check timeout
    (if (i32.gt_u (local.get $buf_len) (i32.const 0))
      (then
        (if (i32.and (local.get $timeout)
              (i32.ge_u
                (i32.sub (local.get $tick) (local.get $last_flush))
                (local.get $timeout)))
          (then
            (drop (call $pipe_write (local.get $output) (local.get $buf) (local.get $buf_len)))
            (i32.store offset=8 (local.get $state) (i32.const 0))
            (i32.store offset=12 (local.get $state) (local.get $tick))
            (return (global.get $STATUS_OK))))
        (return (global.get $STATUS_MORE))))

    (global.get $STATUS_OK))


  ;; Encoding Text — hex + base64url pipeline stages

    ;; Standard ID removed — merged into single module


  (func $hex_char (param $n i32) (result i32)
    local.get $n
    i32.const 10
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 48
      i32.add
    else
      local.get $n
      i32.const 87
      i32.add
    end)

  (func $m90hex_nibble (param $c i32) (result i32)
    local.get $c
    i32.const 48
    i32.ge_u
    local.get $c
    i32.const 57
    i32.le_u
    i32.and
    if (result i32)
      local.get $c
      i32.const 48
      i32.sub
    else
      local.get $c
      i32.const 97
      i32.ge_u
      local.get $c
      i32.const 102
      i32.le_u
      i32.and
      if (result i32)
        local.get $c
        i32.const 87
        i32.sub
      else
        local.get $c
        i32.const 65
        i32.ge_u
        local.get $c
        i32.const 70
        i32.le_u
        i32.and
        if (result i32)
          local.get $c
          i32.const 55
          i32.sub
        else
          i32.const -1
        end
      end
    end)

  ;; base64url functions imported from encoding-base64url as $b64_encode/$b64_decode

  (func $hex_encode_lower (export "hex_encode_lower")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32)
    (local $out_len i32)
    (local $b i32)
    local.get $in_len
    i32.const 2147483647
    i32.gt_u
    if (result i64)
      i32.const 4
      i32.const 0
      call $pack
    else
      local.get $in_len
      i32.const 1
      i32.shl
      local.tee $out_len
      local.get $out_cap
      i32.gt_u
      if (result i64)
        i32.const 2
        i32.const 0
        call $pack
      else
        loop $loop
          local.get $i
          local.get $in_len
          i32.lt_u
          if
            local.get $in_ptr
            local.get $i
            i32.add
            i32.load8_u
            local.set $b
            local.get $out_ptr
            local.get $i
            i32.const 1
            i32.shl
            i32.add
            local.get $b
            i32.const 4
            i32.shr_u
            call $hex_char
            i32.store8
            local.get $out_ptr
            local.get $i
            i32.const 1
            i32.shl
            i32.add
            i32.const 1
            i32.add
            local.get $b
            i32.const 15
            i32.and
            call $hex_char
            i32.store8
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $loop
          end
        end
        i32.const 0
        local.get $out_len
        call $pack
      end
    end)

  (func $hex_decode_strict (export "hex_decode_strict")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32)
    (local $out_len i32)
    (local $hi i32)
    (local $lo i32)
    local.get $in_len
    i32.const 1
    i32.and
    if (result i64)
      i32.const 3
      i32.const 0
      call $pack
    else
      local.get $in_len
      i32.const 1
      i32.shr_u
      local.tee $out_len
      local.get $out_cap
      i32.gt_u
      if (result i64)
        i32.const 2
        i32.const 0
        call $pack
      else
        loop $loop
          local.get $i
          local.get $out_len
          i32.lt_u
          if
            local.get $in_ptr
            local.get $i
            i32.const 1
            i32.shl
            i32.add
            i32.load8_u
            call $m90hex_nibble
            local.tee $hi
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $in_ptr
            local.get $i
            i32.const 1
            i32.shl
            i32.add
            i32.const 1
            i32.add
            i32.load8_u
            call $m90hex_nibble
            local.tee $lo
            i32.const 0
            i32.lt_s
            if
              i32.const 3
              i32.const 0
              call $pack
              return
            end
            local.get $out_ptr
            local.get $i
            i32.add
            local.get $hi
            i32.const 4
            i32.shl
            local.get $lo
            i32.or
            i32.store8
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $loop
          end
        end
        i32.const 0
        local.get $out_len
        call $pack
      end
    end)

  ;; ── SIMD hex encode: 16 bytes → 32 hex chars ──
  (func $hex_encode_simd (export "hex_encode_simd")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32) (local $o i32) (local $b i32)
    (local $v v128) (local $nibbles_hi v128) (local $nibbles_lo v128)
    (local $gt9 v128) (local $delta v128)
    (local $chars_hi v128) (local $chars_lo v128)
    (local $out0 v128) (local $out1 v128)

    (if (i32.gt_u (i32.shl (local.get $in_len) (i32.const 1)) (local.get $out_cap))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))

    (block $loop_end
      (br_if $loop_end (i32.lt_u (local.get $in_len) (i32.const 16)))
      (loop $loop
        (br_if $loop_end
          (i32.ge_u (local.get $i) (i32.sub (local.get $in_len) (i32.const 15))))
        (local.set $v (v128.load (i32.add (local.get $in_ptr) (local.get $i))))
        (local.set $nibbles_hi
          (v128.and
            (i8x16.shr_u (local.get $v) (i32.const 4))
            (i8x16.splat (i32.const 15))))
        (local.set $nibbles_lo
          (v128.and (local.get $v) (i8x16.splat (i32.const 15))))
        (local.set $gt9
          (i8x16.gt_u (local.get $nibbles_hi) (i8x16.splat (i32.const 9))))
        (local.set $delta
          (v128.bitselect
            (i8x16.splat (i32.const 87)) (i8x16.splat (i32.const 48)) (local.get $gt9)))
        (local.set $chars_hi
          (i8x16.add (local.get $nibbles_hi) (local.get $delta)))
        (local.set $gt9
          (i8x16.gt_u (local.get $nibbles_lo) (i8x16.splat (i32.const 9))))
        (local.set $delta
          (v128.bitselect
            (i8x16.splat (i32.const 87)) (i8x16.splat (i32.const 48)) (local.get $gt9)))
        (local.set $chars_lo
          (i8x16.add (local.get $nibbles_lo) (local.get $delta)))
        (local.set $out0
          (i8x16.shuffle 0 16 1 17 2 18 3 19 4 20 5 21 6 22 7 23
            (local.get $chars_hi) (local.get $chars_lo)))
        (local.set $out1
          (i8x16.shuffle 8 24 9 25 10 26 11 27 12 28 13 29 14 15 30 31
            (local.get $chars_hi) (local.get $chars_lo)))
        (v128.store (i32.add (local.get $out_ptr) (local.get $o)) (local.get $out0))
        (v128.store (i32.add (local.get $out_ptr) (i32.add (local.get $o) (i32.const 16))) (local.get $out1))
        (local.set $i (i32.add (local.get $i) (i32.const 16)))
        (local.set $o (i32.add (local.get $o) (i32.const 32)))
        (br $loop)))

    (block $tail_end
      (loop $tail
        (br_if $tail_end (i32.ge_u (local.get $i) (local.get $in_len)))
        (local.set $b (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
        (i32.store8
          (i32.add (local.get $out_ptr) (local.get $o))
          (call $hex_char (i32.shr_u (local.get $b) (i32.const 4))))
        (local.set $o (i32.add (local.get $o) (i32.const 1)))
        (i32.store8
          (i32.add (local.get $out_ptr) (local.get $o))
          (call $hex_char (i32.and (local.get $b) (i32.const 15))))
        (local.set $o (i32.add (local.get $o) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $tail)))
    (call $pack (i32.const 0) (local.get $o)))

  ;; ── SIMD hex decode: 32 hex chars → 16 bytes ──
  (func $hex_decode_simd (export "hex_decode_simd")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32) (local $o i32) (local $c i32) (local $hi i32) (local $lo i32)
    (local $v0 v128) (local $v1 v128)
    (local $digit0 v128) (local $lower0 v128) (local $upper0 v128)
    (local $digit1 v128) (local $lower1 v128) (local $upper1 v128)
    (local $valid0 v128) (local $valid1 v128)
    (local $val0 v128) (local $val1 v128)
    (local $evens v128) (local $odds v128) (local $bytes v128)

    (if (i32.and (local.get $in_len) (i32.const 1))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (if (i32.gt_u (i32.shr_u (local.get $in_len) (i32.const 1)) (local.get $out_cap))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))

    (block $loop_end
      (br_if $loop_end (i32.lt_u (local.get $in_len) (i32.const 32)))
      (loop $loop
        (br_if $loop_end
          (i32.ge_u (local.get $i) (i32.sub (local.get $in_len) (i32.const 31))))
        (local.set $v0 (v128.load (i32.add (local.get $in_ptr) (local.get $i))))
        (local.set $v1 (v128.load (i32.add (local.get $in_ptr) (i32.add (local.get $i) (i32.const 16)))))

        (local.set $digit0
          (v128.and
            (i8x16.ge_u (local.get $v0) (i8x16.splat (i32.const 48)))
            (i8x16.le_u (local.get $v0) (i8x16.splat (i32.const 57)))))
        (local.set $digit1
          (v128.and
            (i8x16.ge_u (local.get $v1) (i8x16.splat (i32.const 48)))
            (i8x16.le_u (local.get $v1) (i8x16.splat (i32.const 57)))))

        (local.set $lower0
          (v128.and
            (i8x16.ge_u (local.get $v0) (i8x16.splat (i32.const 97)))
            (i8x16.le_u (local.get $v0) (i8x16.splat (i32.const 102)))))
        (local.set $lower1
          (v128.and
            (i8x16.ge_u (local.get $v1) (i8x16.splat (i32.const 97)))
            (i8x16.le_u (local.get $v1) (i8x16.splat (i32.const 102)))))

        (local.set $upper0
          (v128.and
            (i8x16.ge_u (local.get $v0) (i8x16.splat (i32.const 65)))
            (i8x16.le_u (local.get $v0) (i8x16.splat (i32.const 70)))))
        (local.set $upper1
          (v128.and
            (i8x16.ge_u (local.get $v1) (i8x16.splat (i32.const 65)))
            (i8x16.le_u (local.get $v1) (i8x16.splat (i32.const 70)))))

        (local.set $valid0 (v128.or (v128.or (local.get $digit0) (local.get $lower0)) (local.get $upper0)))
        (local.set $valid1 (v128.or (v128.or (local.get $digit1) (local.get $lower1)) (local.get $upper1)))
        (if (i32.eqz (i32.and (i8x16.all_true (local.get $valid0)) (i8x16.all_true (local.get $valid1))))
          (then (return (call $pack (i32.const 3) (local.get $i)))))

        ;; nibble = digit ? (v-48) : upper ? (v-55) : (v-87)
        (local.set $val0
          (v128.bitselect
            (i8x16.sub (local.get $v0) (i8x16.splat (i32.const 48)))
            (v128.bitselect
              (i8x16.sub (local.get $v0) (i8x16.splat (i32.const 55)))
              (i8x16.sub (local.get $v0) (i8x16.splat (i32.const 87)))
              (local.get $upper0))
            (local.get $digit0)))
        (local.set $val1
          (v128.bitselect
            (i8x16.sub (local.get $v1) (i8x16.splat (i32.const 48)))
            (v128.bitselect
              (i8x16.sub (local.get $v1) (i8x16.splat (i32.const 55)))
              (i8x16.sub (local.get $v1) (i8x16.splat (i32.const 87)))
              (local.get $upper1))
            (local.get $digit1)))

        ;; Deinterleave: pairs (n0,n1)→byte0, etc.
        (local.set $evens
          (i8x16.shuffle 0 2 4 6 8 10 12 14 16 18 20 22 24 26 28 30
            (local.get $val0) (local.get $val1)))
        (local.set $odds
          (i8x16.shuffle 1 3 5 7 9 11 13 15 17 19 21 23 25 27 29 31
            (local.get $val0) (local.get $val1)))
        (local.set $bytes
          (v128.or (i8x16.shl (local.get $evens) (i32.const 4)) (local.get $odds)))
        (v128.store (i32.add (local.get $out_ptr) (local.get $o)) (local.get $bytes))

        (local.set $i (i32.add (local.get $i) (i32.const 32)))
        (local.set $o (i32.add (local.get $o) (i32.const 16)))
        (br $loop)))

    ;; Scalar tail
    (block $tail_end
      (loop $tail
        (br_if $tail_end (i32.ge_u (local.get $i) (local.get $in_len)))
        (local.set $c (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
        (local.set $hi (call $m90hex_nibble (local.get $c)))
        (if (i32.lt_s (local.get $hi) (i32.const 0))
          (then (return (call $pack (i32.const 3) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (local.set $c (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
        (local.set $lo (call $m90hex_nibble (local.get $c)))
        (if (i32.lt_s (local.get $lo) (i32.const 0))
          (then (return (call $pack (i32.const 3) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (i32.store8
          (i32.add (local.get $out_ptr) (local.get $o))
          (i32.or (i32.shl (local.get $hi) (i32.const 4)) (local.get $lo)))
        (local.set $o (i32.add (local.get $o) (i32.const 1)))
        (br $tail)))
    (call $pack (i32.const 0) (local.get $o)))

  ;; ── Pipeline stage: hex encode (zero-copy input, SIMD accelerated) ──
  ;; Reads directly from pipe buffer via pipe_read_ptr, writes output to scratch.
  (func (export "process_hex_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $in_ptr i32) (local $read i32)
    (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (local.set $in_ptr (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $hex_encode_simd
      (local.get $in_ptr) (local.get $read)
      (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (call $pipe_advance (local.get $input) (local.get $read))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)

  ;; ── Pipeline stage: base64url nopad encode (zero-copy input) ──
  (func (export "process_b64_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $in_ptr i32) (local $read i32)
    (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (local.set $in_ptr (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $b64_encode
      (local.get $in_ptr) (local.get $read)
      (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (call $pipe_advance (local.get $input) (local.get $read))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)

  ;; ── Pipeline stage: base64url nopad decode (zero-copy input) ──
  (func (export "process_b64_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $in_ptr i32) (local $read i32)
    (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (local.set $in_ptr (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $b64_decode
      (local.get $in_ptr) (local.get $read)
      (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (call $pipe_advance (local.get $input) (local.get $read))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)

  ;; ── Pipeline stage: hex decode (zero-copy input, SIMD accelerated) ──
  (func (export "process_hex_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $in_ptr i32) (local $read i32)
    (local $result i64) (local $out_len i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (local.set $in_ptr (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $result (call $hex_decode_simd
      (local.get $in_ptr) (local.get $read)
      (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (call $pipe_advance (local.get $input) (local.get $read))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $out_len)))
    local.get $out_len)
  ;; Socket Core — transport stage with tick-based timeout
;; Abstract socket layer — transport-agnostic byte stream I/O.
  ;;
  ;; Socket types define the transport. Config structs are fixed-size
  ;; records stored in linear memory, passed by pointer to the host's
  ;; sock_open implementation.
  ;;
  ;; Socket type  Config struct (ptr+len)
  ;; ───────────  ─────────────────────────────────
  ;; TCP=0        host_ptr[4] host_len[4] port[2] pad[2]   (12 bytes)
  ;; TLS=1        host_ptr[4] host_len[4] port[2] pad[2]   (12 bytes)
  ;; CDP=2        ws_url_ptr[4] ws_url_len[4]              (8 bytes)
  ;; SOCKS5=3     proxy_host_ptr[4] proxy_host_len[4]
  ;;              proxy_port[2] pad[2]
  ;;              target_host_ptr[4] target_host_len[4]
  ;;              target_port[2] pad[2]                    (24 bytes)
  ;; HTTP_PROXY=4 proxy_host_ptr[4] proxy_host_len[4]
  ;;              proxy_port[2] pad[2]
  ;;              target_host_ptr[4] target_host_len[4]
  ;;              target_port[2] pad[2]                    (24 bytes)
  ;; UDP=5        host_ptr[4] host_len[4] port[2] pad[2]   (12 bytes)
    ;; Standard ID removed — merged into single module

  ;; ── Socket type constants ──
  (func (export "SOCK_TCP")        (result i32) i32.const 0)
  (func (export "SOCK_TLS")        (result i32) i32.const 1)
  (func (export "SOCK_CDP")        (result i32) i32.const 2)
  (func (export "SOCK_SOCKS5")     (result i32) i32.const 3)
  (func (export "SOCK_HTTP_PROXY") (result i32) i32.const 4)
  (func (export "SOCK_UDP")        (result i32) i32.const 5)

  ;; ── Config struct sizes ──
  (func (export "SOCK_CFG_TCP_SIZE")        (result i32) i32.const 12)
  (func (export "SOCK_CFG_TLS_SIZE")        (result i32) i32.const 12)
  (func (export "SOCK_CFG_CDP_SIZE")        (result i32) i32.const 8)
  (func (export "SOCK_CFG_SOCKS5_SIZE")     (result i32) i32.const 24)
  (func (export "SOCK_CFG_HTTP_PROXY_SIZE") (result i32) i32.const 24)
  (func (export "SOCK_CFG_UDP_SIZE")        (result i32) i32.const 12)

  ;; ── SOCKS5 handshake builders ──

  ;; socks5_build_greeting(out_ptr, out_cap) -> i64
  ;; Builds SOCKS5 greeting: [0x05, 0x01, 0x00] (no auth).
  ;; Returns packed (status, length)
  (func (export "socks5_build_greeting")
    (param $out i32) (param $ocap i32) (result i64)
    local.get $ocap i32.const 3 i32.lt_u
    if i64.const -2 return end
    local.get $out i32.const 0 i32.add i32.const 5 i32.store8
    local.get $out i32.const 1 i32.add i32.const 1 i32.store8
    local.get $out i32.const 2 i32.add i32.const 0 i32.store8
    i64.const 0 i32.const 3 i64.extend_i32_u i64.const 32 i64.shl i64.or)

  ;; socks5_parse_greeting_response(data_ptr, data_len) -> i32
  ;; Returns 0 if server accepted no-auth, -1 on error.
  (func (export "socks5_parse_greeting_response")
    (param $data i32) (param $dlen i32) (result i32)
    local.get $dlen i32.const 2 i32.lt_u
    if i32.const -1 return end
    local.get $data i32.load8_u offset=0 i32.const 5 i32.ne
    if i32.const -1 return end
    local.get $data i32.load8_u offset=1 i32.const 0 i32.ne
    if i32.const -1 return end
    i32.const 0)

  ;; socks5_build_connect(out_ptr, out_cap, host_ptr, host_len, port) -> i64
  ;; Builds SOCKS5 connect request with domain name.
  ;; Returns packed (status, length)
  (func (export "socks5_build_connect")
    (param $out i32) (param $ocap i32)
    (param $host i32) (param $hlen i32)
    (param $port i32) (result i64)
    (local $need i32) (local $i i32)

    i32.const 7  ;; ver+cmd+rsv+atype+len = 5 bytes, +hlen + 2 port
    local.get $hlen
    i32.add
    local.set $need

    local.get $ocap local.get $need i32.lt_u
    if i64.const -2 return end

    local.get $out i32.const 0 i32.add i32.const 5 i32.store8     ;; ver = 5
    local.get $out i32.const 1 i32.add i32.const 1 i32.store8     ;; cmd = CONNECT
    local.get $out i32.const 2 i32.add i32.const 0 i32.store8     ;; rsv = 0
    local.get $out i32.const 3 i32.add i32.const 3 i32.store8     ;; atyp = DOMAINNAME
    local.get $out i32.const 4 i32.add local.get $hlen i32.store8 ;; domain length

    ;; copy host name
    i32.const 0 local.set $i
    block $cpy_done
    loop $cpy
      local.get $i local.get $hlen i32.ge_u br_if $cpy_done
      local.get $out i32.const 5 i32.add local.get $i i32.add
      local.get $host local.get $i i32.add i32.load8_u
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $cpy
    end
    end

    ;; port (big-endian)
    local.get $out i32.const 5 i32.add local.get $hlen i32.add
    local.get $port i32.const 8 i32.shr_u i32.const 255 i32.and
    i32.store8
    local.get $out i32.const 6 i32.add local.get $hlen i32.add
    local.get $port i32.const 255 i32.and
    i32.store8

    i64.const 0
    local.get $need
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; socks5_parse_connect_response(data_ptr, data_len) -> i32
  ;; Returns 0 if connect succeeded, -1 on error.
  ;; On success, the proxy has connected to the target.
  (func (export "socks5_parse_connect_response")
    (param $data i32) (param $dlen i32) (result i32)
    local.get $dlen i32.const 10 i32.lt_u
    if i32.const -1 return end
    local.get $data i32.load8_u offset=0 i32.const 5 i32.ne
    if i32.const -1 return end
    local.get $data i32.load8_u offset=1 i32.const 0 i32.ne  ;; rep must be 0 (succeeded)
    if i32.const -1 return end
    i32.const 0)

  ;; ── HTTP CONNECT proxy handshake builders ──

  ;; http_proxy_build_request(out_ptr, out_cap, host_ptr, host_len, port) -> i64
  ;; Builds "CONNECT host:port HTTP/1.1\r\nHost: host:port\r\n\r\n"
  ;; Returns packed (status, length)
  (func (export "http_proxy_build_request")
    (param $out i32) (param $ocap i32)
    (param $host i32) (param $hlen i32)
    (param $port i32) (result i64)
    (local $o i32)

    ;; "CONNECT "
    local.get $o i32.const 7 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 67 i32.store8   ;; 'C'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 79 i32.store8   ;; 'O'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 78 i32.store8   ;; 'N'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 78 i32.store8   ;; 'N'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 69 i32.store8   ;; 'E'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 67 i32.store8   ;; 'C'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 84 i32.store8   ;; 'T'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 32 i32.store8   ;; ' '
    local.get $o i32.const 1 i32.add local.set $o

    ;; host
    local.get $o local.get $hlen i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $host local.get $out local.get $o local.get $hlen call $m166memcpy_to_off
    local.get $o local.get $hlen i32.add local.set $o

    ;; ":port "
    local.get $o i32.const 1 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 58 i32.store8   ;; ':'
    local.get $o i32.const 1 i32.add local.set $o

    local.get $port local.get $out local.get $o call $m166emit_u32_dec
    local.set $o

    local.get $o i32.const 1 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 32 i32.store8   ;; ' '
    local.get $o i32.const 1 i32.add local.set $o

    ;; "HTTP/1.1\r\n"
    local.get $o i32.const 10 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 72 i32.store8   ;; 'H'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 84 i32.store8   ;; 'T'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 84 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 80 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 47 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 49 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 46 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 49 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 13 i32.store8   ;; CR
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8   ;; LF
    local.get $o i32.const 1 i32.add local.set $o

    ;; "Host: "
    local.get $o i32.const 6 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 72 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 115 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 116 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 58 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 32 i32.store8
    local.get $o i32.const 1 i32.add local.set $o

    ;; host again
    local.get $o local.get $hlen i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $host local.get $out local.get $o local.get $hlen call $m166memcpy_to_off
    local.get $o local.get $hlen i32.add local.set $o

    ;; ":port"
    local.get $out local.get $o i32.add i32.const 58 i32.store8
    local.get $o i32.const 1 i32.add local.set $o

    local.get $port local.get $out local.get $o call $m166emit_u32_dec
    local.set $o

    ;; "\r\n\r\n"
    local.get $o i32.const 4 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 13 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 13 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8
    local.get $o i32.const 1 i32.add local.set $o

    i64.const 0
    local.get $o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; http_proxy_parse_response(data_ptr, data_len) -> i32
  ;; Check for "200" in HTTP response line.
  ;; Returns 0 if connected, -1 on error.
  (func (export "http_proxy_parse_response")
    (param $data i32) (param $dlen i32) (result i32)
    local.get $dlen i32.const 12 i32.lt_u
    if i32.const -1 return end
    ;; check for "HTTP/1.1 200" or "HTTP/1.0 200"
    local.get $data i32.load8_u offset=0 i32.const 72 i32.ne  ;; 'H'
    if i32.const -1 return end
    local.get $data i32.load8_u offset=9 i32.const 50 i32.ne  ;; '2'
    if i32.const -1 return end
    local.get $data i32.load8_u offset=10 i32.const 48 i32.ne ;; '0'
    if i32.const -1 return end
    local.get $data i32.load8_u offset=11 i32.const 48 i32.ne ;; '0'
    if i32.const -1 return end
    i32.const 0)

  ;; ── Config struct builders ──

  ;; sock_cfg_build_tcp(out, cap, host_ptr, host_len, port) -> i64
  ;; Writes TCP config struct, returns (0, 12)
  (func (export "sock_cfg_build_tcp")
    (param $out i32) (param $ocap i32)
    (param $host i32) (param $hlen i32)
    (param $port i32) (result i64)
    local.get $ocap i32.const 12 i32.lt_u
    if i64.const -2 return end
    local.get $out i32.const 0 i32.add local.get $host i32.store
    local.get $out i32.const 4 i32.add local.get $hlen i32.store
    local.get $out i32.const 8 i32.add local.get $port i32.store16
    i64.const 0 i32.const 12 i64.extend_i32_u i64.const 32 i64.shl i64.or)

  ;; sock_cfg_build_tls(out, cap, host_ptr, host_len, port) -> i64
  ;; TLS config is same layout as TCP (host+port+SNI)
  (func (export "sock_cfg_build_tls")
    (param $out i32) (param $ocap i32)
    (param $host i32) (param $hlen i32)
    (param $port i32) (result i64)
    local.get $ocap i32.const 12 i32.lt_u
    if i64.const -2 return end
    local.get $out i32.const 0 i32.add local.get $host i32.store
    local.get $out i32.const 4 i32.add local.get $hlen i32.store
    local.get $out i32.const 8 i32.add local.get $port i32.store16
    i64.const 0 i32.const 12 i64.extend_i32_u i64.const 32 i64.shl i64.or)

  ;; sock_cfg_build_socks5(out, cap,
  ;;                       proxy_host, proxy_hlen, proxy_port,
  ;;                       target_host, target_hlen, target_port) -> i64
  (func (export "sock_cfg_build_socks5")
    (param $out i32) (param $ocap i32)
    (param $phost i32) (param $phlen i32) (param $pport i32)
    (param $thost i32) (param $thlen i32) (param $tport i32) (result i64)
    local.get $ocap i32.const 24 i32.lt_u
    if i64.const -2 return end
    local.get $out i32.const 0 i32.add local.get $phost i32.store
    local.get $out i32.const 4 i32.add local.get $phlen i32.store
    local.get $out i32.const 8 i32.add local.get $pport i32.store16
    local.get $out i32.const 12 i32.add local.get $thost i32.store
    local.get $out i32.const 16 i32.add local.get $thlen i32.store
    local.get $out i32.const 20 i32.add local.get $tport i32.store16
    i64.const 0 i32.const 24 i64.extend_i32_u i64.const 32 i64.shl i64.or)

  ;; ── helpers ──

  ;; memcpy_to_off(src, dst, dst_off, len)
  (func $m166memcpy_to_off (param $src i32) (param $dst i32) (param $off i32) (param $len i32)
    (local $i i32)
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $dst local.get $off i32.add local.get $i i32.add
      local.get $src local.get $i i32.add i32.load8_u
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)

  ;; emit_u32_dec(n, out, offset) -> new_offset
  (func $m166emit_u32_dec (param $n i32) (param $out i32) (param $off i32) (result i32)
    (local $buf i32) (local $digits i32) (local $d i32)
    i32.const 768 local.set $buf
    i32.const 0 local.set $digits
    local.get $n i32.eqz
    if
      local.get $out local.get $off i32.add i32.const 48 i32.store8
      local.get $off i32.const 1 i32.add
      return
    end
    block $extract_done
    loop $extract
      local.get $n i32.eqz br_if $extract_done
      local.get $buf local.get $digits i32.add
      local.get $n i32.const 10 i32.rem_u i32.const 48 i32.add
      i32.store8
      local.get $digits i32.const 1 i32.add local.set $digits
      local.get $n i32.const 10 i32.div_u local.set $n
      br $extract
    end
    end
    block $emit_done
    loop $emit_loop
      local.get $d local.get $digits i32.ge_u br_if $emit_done
      local.get $out local.get $off i32.add local.get $d i32.add
      local.get $buf local.get $digits i32.const 1 i32.sub local.get $d i32.sub i32.add
      i32.load8_u
      i32.store8
      local.get $d i32.const 1 i32.add local.set $d
      br $emit_loop
    end
    end
    local.get $off local.get $digits i32.add)

  ;; ── Socket abstraction (pipe-based I/O) ──
  ;;
  ;; Socket struct (24 bytes):
  ;;   +0:  send_pipe  — pipe handle for outgoing data
  ;;   +4:  recv_pipe  — pipe handle for incoming data
  ;;   +8:  state      — 0=closed 1=open 2=connecting 3=connected
  ;;   +12: sock_type  — TCP=0 TLS=1 ...
  ;;   +16: cfg_ptr    — pointer to stored config
  ;;   +20: cfg_len    — config length

  (func (export "SOCK_OPEN")       (result i32) i32.const 1)
  (func (export "SOCK_CONNECTING") (result i32) i32.const 2)
  (func (export "SOCK_CONNECTED")  (result i32) i32.const 3)
  (func (export "SOCK_STRUCT_SIZE") (result i32) i32.const 24)

  ;; sock_open(type, cfg_ptr, cfg_len, pipe_cap) → socket_handle | -1
  ;; Allocates a socket struct + two pipes (send + recv).
  (func (export "sock_open")
    (param $type i32) (param $cfg i32) (param $clen i32) (param $pcap i32) (result i32)
    (local $s i32) (local $snd i32) (local $rcv i32)
    (local.set $s (call $pipe_alloc (i32.const 24)))
    (if (i32.eq (local.get $s) (i32.const -1)) (then (return (i32.const -1))))
    (local.set $snd (call $pipe_create (local.get $pcap)))
    (if (i32.eq (local.get $snd) (i32.const -1)) (then (return (i32.const -1))))
    (local.set $rcv (call $pipe_create (local.get $pcap)))
    (if (i32.eq (local.get $rcv) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $s) (local.get $snd))
    (i32.store offset=4 (local.get $s) (local.get $rcv))
    (i32.store offset=8 (local.get $s) (i32.const 1))  ;; state = open
    (i32.store offset=12 (local.get $s) (local.get $type))
    (i32.store offset=16 (local.get $s) (local.get $cfg))
    (i32.store offset=20 (local.get $s) (local.get $clen))
    local.get $s)

  ;; sock_send(socket, data, len) → status
  ;; Writes data to the socket's send pipe.
  (func (export "sock_send")
    (param $s i32) (param $data i32) (param $len i32) (result i32)
    (call $pipe_write (i32.load offset=0 (local.get $s)) (local.get $data) (local.get $len)))

  ;; sock_recv(socket, dst, max) → bytes_read
  ;; Reads from the socket's recv pipe.
  (func (export "sock_recv")
    (param $s i32) (param $dst i32) (param $max i32) (result i32)
    (call $pipe_read (i32.load offset=4 (local.get $s)) (local.get $dst) (local.get $max)))

  ;; sock_close(socket) — closes both pipes, marks socket closed
  (func (export "sock_close") (param $s i32)
    (call $pipe_close (i32.load offset=0 (local.get $s)))
    (call $pipe_close (i32.load offset=4 (local.get $s)))
    (i32.store offset=8 (local.get $s) (i32.const 0)))

  ;; sock_get_state(socket) → state
  (func (export "sock_get_state") (param $s i32) (result i32)
    (i32.load offset=8 (local.get $s)))

  ;; sock_get_send_pipe(socket) → pipe_handle
  (func (export "sock_get_send_pipe") (param $s i32) (result i32)
    (i32.load offset=0 (local.get $s)))

  ;; sock_get_recv_pipe(socket) → pipe_handle
  (func (export "sock_get_recv_pipe") (param $s i32) (result i32)
    (i32.load offset=4 (local.get $s)))

  ;; sock_pipe(from, to, tmp, tcap) → bytes_piped | error
  ;; Pipes data from from_sock's recv pipe into to_sock's send pipe.
  ;; Uses tmp buffer of tcap bytes as scratch space.
  (func $sock_pipe (export "sock_pipe")
    (param $from i32) (param $to i32) (param $tmp i32) (param $tcap i32) (result i32)
    (local $n i32)
    (local.set $n (call $pipe_read
      (i32.load offset=4 (local.get $from)) (local.get $tmp) (local.get $tcap)))
    (if (i32.le_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))
    (call $pipe_write
      (i32.load offset=0 (local.get $to)) (local.get $tmp) (local.get $n))
    (return (local.get $n)))

  ;; ── Pipeline stage: transport ──
  ;; Reads from input pipe, sends via socket, reads response, writes to output pipe.
  ;; Config: pointer to a 4-byte i32 socket handle.
  ;; State layout (16 bytes):
  ;;   +0:  tick         i32 (RO, written by pipeline_run)
  ;;   +4:  phase        i32 (0=idle, 1=awaiting recv)
  ;;   +8:  start_tick   i32 (tick when phase=1 was entered)
  ;;   +12: timeout_ticks i32 (max ticks to wait before returning TIMEOUT, 0=infinite)
  ;; (input_pipe, output_pipe, config_ptr, config_len, scratch, scap, state_ptr) → OK | MORE | error
  (func (export "process_transport")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $sock i32) (local $n i32) (local $phase i32) (local $sent i32)
    (local $elapsed i32)
    (if (i32.lt_u (local.get $clen) (i32.const 4))
      (then (return (i32.const -1))))
    (local.set $sock (i32.load (local.get $cfg)))
    (local.set $phase (i32.load offset=4 (local.get $state)))

    ;; Phase 1 (awaiting response): check timeout, then try recv again
    (if (i32.eq (local.get $phase) (i32.const 1))
      (then
        ;; Compute elapsed ticks since entering phase 1
        (local.set $elapsed
          (i32.sub (i32.load (local.get $state)) (i32.load offset=8 (local.get $state))))
        ;; If timeout_ticks > 0 and elapsed >= timeout_ticks, return TIMEOUT
        (if (i32.load offset=12 (local.get $state))
          (then
            (if (i32.ge_u (local.get $elapsed) (i32.load offset=12 (local.get $state)))
              (then (return (global.get $STATUS_TIMEOUT))))))
        (local.set $n (call $pipe_read (i32.load offset=4 (local.get $sock)) (local.get $scratch) (local.get $scap)))
        (if (i32.gt_s (local.get $n) (i32.const 0))
          (then
            (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $n)))
            (i32.store offset=4 (local.get $state) (i32.const 0))
            (return (local.get $n))))
        (return (global.get $STATUS_MORE))))

    ;; Phase 0 (idle): try send + recv
    ;; Read data to send from input pipe
    (local.set $n (call $pipe_read (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then
        (drop (call $pipe_write (i32.load offset=0 (local.get $sock)) (local.get $scratch) (local.get $n)))))
    (local.set $sent (local.get $n))

    ;; Read response from socket recv pipe
    (local.set $n (call $pipe_read (i32.load offset=4 (local.get $sock)) (local.get $scratch) (local.get $scap)))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then
        (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $n)))
        (return (local.get $n))))
    (if (i32.eqz (local.get $n))
      (then
        ;; No response yet — if we sent something, mark awaiting and return MORE
        (if (i32.gt_s (local.get $sent) (i32.const 0))
          (then
            (i32.store offset=4 (local.get $state) (i32.const 1))
            (i32.store offset=8 (local.get $state) (i32.load (local.get $state)))
            (return (global.get $STATUS_MORE))))
        (return (i32.const 0))))
    local.get $n)

  ;; Session Core — pipeline session scheduler
  ;; Manages upstream + downstream pipeline pairs sharing a socket and epoch.

  ;; Session descriptor layout (48 bytes, at 0x98000+)
  ;; +0:  up_desc    i32  — upstream pipeline descriptor
  ;; +4:  down_desc  i32  — downstream pipeline descriptor
  ;; +8:  socket     i32  — socket handle (shared socket struct ptr)
  ;; +12: app_input  i32  — upstream input pipe (app → network)
  ;; +16: net_output i32  — upstream output pipe (network-bound data)
  ;; +20: net_input  i32  — downstream input pipe (network data)
  ;; +24: app_output i32  — downstream output pipe (app-bound data)
  ;; +28: scratch    i32  — scratch buffer pointer
  ;; +32: scap       i32  — scratch buffer capacity
  ;; +36: state      i32  — 0=idle 1=active 2=error
  ;; +40: epoch      i32  — local epoch counter
  ;; +44: (reserved)

  (global $SD_UP_DESC    i32 (i32.const 0))
  (global $SD_DOWN_DESC  i32 (i32.const 4))
  (global $SD_SOCKET     i32 (i32.const 8))
  (global $SD_APP_IN     i32 (i32.const 12))
  (global $SD_NET_OUT    i32 (i32.const 16))
  (global $SD_NET_IN     i32 (i32.const 20))
  (global $SD_APP_OUT    i32 (i32.const 24))
  (global $SD_SCRATCH    i32 (i32.const 28))
  (global $SD_SCAP       i32 (i32.const 32))
  (global $SD_STATE      i32 (i32.const 36))
  (global $SD_EPOCH      i32 (i32.const 40))
  (global $SD_SIZE       i32 (i32.const 48))

  ;; Session state constants
  (func (export "SESSION_IDLE")   (result i32) i32.const 0)
  (func (export "SESSION_ACTIVE") (result i32) i32.const 1)
  (func (export "SESSION_ERROR")  (result i32) i32.const 2)

  ;; session_create() → session_ptr | -1
  ;; Allocates a session descriptor from the bump heap.
  (func (export "session_create") (result i32)
    (local $s i32)
    (local.set $s (call $pipe_alloc (global.get $SD_SIZE)))
    (if (i32.eq (local.get $s) (i32.const -1))
      (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $s) (i32.const 0))
    (i32.store offset=4 (local.get $s) (i32.const 0))
    (i32.store offset=8 (local.get $s) (i32.const 0))
    (i32.store offset=12 (local.get $s) (i32.const 0))
    (i32.store offset=16 (local.get $s) (i32.const 0))
    (i32.store offset=20 (local.get $s) (i32.const 0))
    (i32.store offset=24 (local.get $s) (i32.const 0))
    (i32.store offset=28 (local.get $s) (i32.const 0))
    (i32.store offset=32 (local.get $s) (i32.const 0))
    (i32.store offset=36 (local.get $s) (i32.const 0))
    (i32.store offset=40 (local.get $s) (i32.const 0))
    local.get $s)

  ;; session_configure(session, up_desc, down_desc, socket, app_in, net_out, net_in, app_out, scratch, scap)
  (func (export "session_configure")
    (param $s i32) (param $up i32) (param $down i32) (param $sock i32)
    (param $app_in i32) (param $net_out i32) (param $net_in i32) (param $app_out i32)
    (param $scratch i32) (param $scap i32)
    (i32.store offset=0 (local.get $s) (local.get $up))
    (i32.store offset=4 (local.get $s) (local.get $down))
    (i32.store offset=8 (local.get $s) (local.get $sock))
    (i32.store offset=12 (local.get $s) (local.get $app_in))
    (i32.store offset=16 (local.get $s) (local.get $net_out))
    (i32.store offset=20 (local.get $s) (local.get $net_in))
    (i32.store offset=24 (local.get $s) (local.get $app_out))
    (i32.store offset=28 (local.get $s) (local.get $scratch))
    (i32.store offset=32 (local.get $s) (local.get $scap)))

  ;; session_get_state(session) → state
  (func (export "session_get_state") (param $s i32) (result i32)
    (i32.load offset=36 (local.get $s)))

  ;; session_get_epoch(session) → epoch
  (func (export "session_get_epoch") (param $s i32) (result i32)
    (i32.load offset=40 (local.get $s)))

  ;; session_drive(session) → OK | MORE | TIMEOUT | error
  ;; Drives one tick of the session: increments global epoch, runs upstream
  ;; then downstream pipeline.
  (func (export "session_drive") (param $s i32) (result i32)
    (local $up i32) (local $down i32) (local $sock i32)
    (local $app_in i32) (local $net_out i32) (local $net_in i32) (local $app_out i32)
    (local $scratch i32) (local $scap i32)
    (local $result i32) (local $state i32)

    ;; Check session state — abort on error
    (local.set $state (i32.load offset=36 (local.get $s)))
    (if (i32.eq (local.get $state) (i32.const 2))
      (then (return (i32.const -1))))

    ;; Load session fields
    (local.set $up (i32.load offset=0 (local.get $s)))
    (local.set $down (i32.load offset=4 (local.get $s)))
    (local.set $sock (i32.load offset=8 (local.get $s)))
    (local.set $app_in (i32.load offset=12 (local.get $s)))
    (local.set $net_out (i32.load offset=16 (local.get $s)))
    (local.set $net_in (i32.load offset=20 (local.get $s)))
    (local.set $app_out (i32.load offset=24 (local.get $s)))
    (local.set $scratch (i32.load offset=28 (local.get $s)))
    (local.set $scap (i32.load offset=32 (local.get $s)))

    ;; Increment global epoch
    (global.set $epoch
      (i32.add (global.get $epoch) (i32.const 1)))

    ;; Save epoch to session
    (i32.store offset=40 (local.get $s) (global.get $epoch))

    ;; Set session state to active
    (i32.store offset=36 (local.get $s) (i32.const 1))

    ;; Drive upstream pipeline (app → network)
    (local.set $result
      (call $pipeline_run
        (local.get $up) (local.get $app_in) (local.get $net_out)
        (local.get $scratch) (local.get $scap)))

    ;; Check upstream result
    (if (i32.or
          (i32.lt_s (local.get $result) (i32.const 0))
          (i32.eq (local.get $result) (global.get $STATUS_MORE)))
      (then
        (if (i32.lt_s (local.get $result) (i32.const 0))
          (then (i32.store offset=36 (local.get $s) (i32.const 2))))
        (return (local.get $result))))

    ;; Drive downstream pipeline (network → app)
    (local.set $result
      (call $pipeline_run
        (local.get $down) (local.get $net_in) (local.get $app_out)
        (local.get $scratch) (local.get $scap)))

    ;; Check downstream result
    (if (i32.or
          (i32.lt_s (local.get $result) (i32.const 0))
          (i32.eq (local.get $result) (global.get $STATUS_MORE)))
      (then
        (if (i32.lt_s (local.get $result) (i32.const 0))
          (then (i32.store offset=36 (local.get $s) (i32.const 2))))
        (return (local.get $result))))

    ;; Both pipelines completed OK
    (global.get $STATUS_OK))

  ;; session_reset(session) — reset session to idle
  (func (export "session_reset") (param $s i32)
    (i32.store offset=36 (local.get $s) (i32.const 0))
    (i32.store offset=40 (local.get $s) (i32.const 0)))

  ;; Mux Core — static/dynamic mux + demux

    ;; Standard ID removed — merged into single module

  ;; ════════════════════════════════════════════════════════════════
  ;; Static Mux — fixed array of stream pipes → framed output
  ;; Config blob: [output_pipe:i32][count:i32][pipe_0:i32][pipe_1:i32]...
  ;; Mux descriptor: [type:i32=0][output:i32][count:i32][pipe_0:i32]...
  ;; ════════════════════════════════════════════════════════════════

  (func (export "mux_static_create") (param $cfg i32) (result i32)
    (local $count i32) (local $sz i32) (local $mux i32)
    (local.set $count (i32.load offset=4 (local.get $cfg)))
    (local.set $sz (i32.add (i32.const 12) (i32.mul (local.get $count) (i32.const 4))))
    (local.set $mux (call $pipe_alloc (local.get $sz)))
    (if (i32.eq (local.get $mux) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $mux) (i32.const 0))
    (i32.store offset=4 (local.get $mux) (i32.load (local.get $cfg)))
    (i32.store offset=8 (local.get $mux) (local.get $count))
    (call $memcpy_off (local.get $mux) (i32.const 12) (local.get $cfg) (i32.const 8)
      (i32.mul (local.get $count) (i32.const 4)))
    local.get $mux)

  ;; mux_static_run(mux, scratch, scap) → frames_written | error
  ;; Round-robin: read from each stream pipe, frame and write to output
  (func (export "mux_static_run")
    (param $mux i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $output i32) (local $count i32) (local $i i32)
    (local $pipe i32) (local $avail i32) (local $r i32) (local $total i32)
    (local.set $output (i32.load offset=4 (local.get $mux)))
    (local.set $count (i32.load offset=8 (local.get $mux)))
    (block $done
      (loop $streams
        (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
        (local.set $pipe (i32.load (i32.add (local.get $mux)
          (i32.add (i32.const 12) (i32.mul (local.get $i) (i32.const 4))))))
        (local.set $avail (call $pipe_available (local.get $pipe)))
        (if (i32.gt_u (local.get $avail) (i32.const 0))
          (then
            (if (i32.gt_u (local.get $avail) (local.get $scap))
              (then (local.set $avail (local.get $scap))))
            (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (local.get $avail)))
            (if (i32.lt_s (local.get $r) (i32.const 0))
              (then (local.set $total (local.get $r)) (br $done)))
            (local.set $r (call $frame_write (local.get $output) (local.get $i) (local.get $scratch) (local.get $r)))
            (if (i32.ne (local.get $r) (global.get $STATUS_OK))
              (then (local.set $total (i32.sub (i32.const 0) (local.get $r))) (br $done)))
            (local.set $total (i32.add (local.get $total) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $streams)))
    local.get $total)

  ;; ════════════════════════════════════════════════════════════════
  ;; Static Demux — read framed input, route to output by stream_id
  ;; Config blob: [input_pipe:i32][count:i32][pipe_0:i32][pipe_1:i32]...
  ;; Demux descriptor: [type:i32=2][input:i32][count:i32][pipe_0:i32]...
  ;; ════════════════════════════════════════════════════════════════

  (func (export "demux_static_create") (param $cfg i32) (result i32)
    (local $count i32) (local $sz i32) (local $demux i32)
    (local.set $count (i32.load offset=4 (local.get $cfg)))
    (local.set $sz (i32.add (i32.const 12) (i32.mul (local.get $count) (i32.const 4))))
    (local.set $demux (call $pipe_alloc (local.get $sz)))
    (if (i32.eq (local.get $demux) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $demux) (i32.const 2))
    (i32.store offset=4 (local.get $demux) (i32.load (local.get $cfg)))
    (i32.store offset=8 (local.get $demux) (local.get $count))
    (call $memcpy_off (local.get $demux) (i32.const 12) (local.get $cfg) (i32.const 8)
      (i32.mul (local.get $count) (i32.const 4)))
    local.get $demux)

  ;; demux_static_run(demux, scratch, scap) → 1 (routed) | 0 (no frame) | error
  ;; Read one frame from input, route payload to output pipe[stream_id]
  (func (export "demux_static_run")
    (param $demux i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $input i32) (local $count i32) (local $result i64)
    (local $status i32) (local $stream_id i32) (local $pipe i32)
    (local $avail i32)
    (local.set $input (i32.load offset=4 (local.get $demux)))
    (local.set $count (i32.load offset=8 (local.get $demux)))
    ;; Peek available bytes — need at least 8 for header
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.lt_u (local.get $avail) (i32.const 8))
      (then (return (i32.const 0))))
    ;; Read header bytes to get stream_id
    (local.set $result (call $frame_read (local.get $input) (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (i32.ne (local.get $status) (global.get $STATUS_OK))
      (then
        ;; OVERFLOW means scratch too small — skip the frame
        (if (i32.eq (local.get $status) (global.get $STATUS_OVERFLOW))
          (then
            ;; frame_read already skipped the payload, so this frame is lost
            (return (i32.const 0))))
        (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $stream_id (i32.wrap_i64 (local.get $result)))
    ;; Route
    (if (i32.ge_u (local.get $stream_id) (local.get $count))
      (then (return (i32.const -1))))
    (local.set $pipe (i32.load (i32.add (local.get $demux)
      (i32.add (i32.const 12) (i32.mul (local.get $stream_id) (i32.const 4))))))
    ;; payload_len at scratch[0..4], payload at scratch[4..]
    (local.set $avail (i32.load (local.get $scratch)))
    (drop (call $pipe_write (local.get $pipe) (i32.add (local.get $scratch) (i32.const 4)) (local.get $avail)))
    (return (i32.const 1)))

  ;; ════════════════════════════════════════════════════════════════
  ;; Dynamic Mux — linked list of (stream_id, pipe) → framed output
  ;; ════════════════════════════════════════════════════════════════

  ;; stream_node: +0 stream_id, +4 pipe, +8 next

  (func (export "mux_dynamic_create") (param $output i32) (result i32)
    (local $mux i32)
    (local.set $mux (call $pipe_alloc (i32.const 20)))
    (if (i32.eq (local.get $mux) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $mux) (i32.const 1))
    (i32.store offset=4 (local.get $mux) (local.get $output))
    (i32.store offset=8 (local.get $mux) (i32.const 0))
    (i32.store offset=12 (local.get $mux) (i32.const 0))
    (i32.store offset=16 (local.get $mux) (i32.const 0))  ;; freelist
    local.get $mux)

  (func (export "mux_add_stream") (param $mux i32) (param $stream_id i32) (param $pipe i32) (result i32)
    (local $node i32)
    ;; Try freelist first, then allocate
    (local.set $node (i32.load offset=16 (local.get $mux)))
    (if (local.get $node)
      (then
        (i32.store offset=16 (local.get $mux) (i32.load offset=8 (local.get $node))))
      (else
        (local.set $node (call $pipe_alloc (i32.const 12)))
        (if (i32.eq (local.get $node) (i32.const -1)) (then (return (global.get $STATUS_OVERFLOW))))))
    (i32.store offset=0 (local.get $node) (local.get $stream_id))
    (i32.store offset=4 (local.get $node) (local.get $pipe))
    (i32.store offset=8 (local.get $node) (i32.load offset=8 (local.get $mux)))
    (i32.store offset=8 (local.get $mux) (local.get $node))
    (i32.store offset=12 (local.get $mux) (i32.add (i32.load offset=12 (local.get $mux)) (i32.const 1)))
    global.get $STATUS_OK)

  (func (export "mux_remove_stream") (param $mux i32) (param $stream_id i32) (result i32)
    (local $prev i32) (local $curr i32) (local $next i32)
    (local.set $curr (i32.load offset=8 (local.get $mux)))
    (block $found
      (loop $walk
        (br_if $found (i32.eqz (local.get $curr)))
        (if (i32.eq (i32.load (local.get $curr)) (local.get $stream_id))
          (then
            (local.set $next (i32.load offset=8 (local.get $curr)))
            (if (local.get $prev)
              (then (i32.store offset=8 (local.get $prev) (local.get $next)))
              (else (i32.store offset=8 (local.get $mux) (local.get $next))))
            (i32.store offset=12 (local.get $mux)
              (i32.sub (i32.load offset=12 (local.get $mux)) (i32.const 1)))
            ;; Add node to freelist
            (i32.store offset=8 (local.get $curr) (i32.load offset=16 (local.get $mux)))
            (i32.store offset=16 (local.get $mux) (local.get $curr))
            (br $found)))
        (local.set $prev (local.get $curr))
        (local.set $curr (i32.load offset=8 (local.get $curr)))
        (br $walk)))
    global.get $STATUS_OK)

  ;; mux_dynamic_run(mux, scratch, scap) → frames_written | error
  (func $mux_dynamic_run (export "mux_dynamic_run")
    (param $mux i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $output i32) (local $curr i32) (local $stream_id i32)
    (local $pipe i32) (local $avail i32) (local $r i32) (local $total i32)
    (local.set $output (i32.load offset=4 (local.get $mux)))
    (local.set $curr (i32.load offset=8 (local.get $mux)))
    (block $done
      (loop $walk
        (br_if $done (i32.eqz (local.get $curr)))
        (local.set $stream_id (i32.load (local.get $curr)))
        (local.set $pipe (i32.load offset=4 (local.get $curr)))
        (local.set $avail (call $pipe_available (local.get $pipe)))
        (if (i32.gt_u (local.get $avail) (i32.const 0))
          (then
            (if (i32.gt_u (local.get $avail) (local.get $scap))
              (then (local.set $avail (local.get $scap))))
            (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (local.get $avail)))
            (if (i32.lt_s (local.get $r) (i32.const 0))
              (then (local.set $total (local.get $r)) (br $done)))
            (local.set $r (call $frame_write (local.get $output) (local.get $stream_id) (local.get $scratch) (local.get $r)))
            (if (i32.ne (local.get $r) (global.get $STATUS_OK))
              (then (local.set $total (i32.sub (i32.const 0) (local.get $r))) (br $done)))
            (local.set $total (i32.add (local.get $total) (i32.const 1)))))
        (local.set $curr (i32.load offset=8 (local.get $curr)))
        (br $walk)))
    local.get $total)

  ;; ════════════════════════════════════════════════════════════════
  ;; Dynamic Demux — hash table: stream_id → output_pipe
  ;; Descriptor: [type:i32=3][input:i32][slot_count:i32][slot_0:stream_id,pipe]...
  ;; slot = 8 bytes, slot_count must be power of 2
  ;; stream_id=0 = empty slot
  ;; ════════════════════════════════════════════════════════════════

  (func (export "demux_dynamic_create") (param $input i32) (param $slot_count i32) (result i32)
    (local $demux i32) (local $sz i32)
    (local.set $sz (i32.add (i32.const 12) (i32.mul (local.get $slot_count) (i32.const 8))))
    (local.set $demux (call $pipe_alloc (local.get $sz)))
    (if (i32.eq (local.get $demux) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $demux) (i32.const 3))
    (i32.store offset=4 (local.get $demux) (local.get $input))
    (i32.store offset=8 (local.get $demux) (local.get $slot_count))
    local.get $demux)

  ;; Linear probe: hash(stream_id) = stream_id & mask
  ;; Returns slot address or 0 if not found
  (func $demux_lookup (param $demux i32) (param $stream_id i32) (result i32)
    (local $slot_count i32) (local $mask i32) (local $base i32)
    (local $sid i32) (local $i i32)
    (if (i32.eqz (local.get $stream_id)) (then (return (i32.const 0))))
    (local.set $slot_count (i32.load offset=8 (local.get $demux)))
    (local.set $mask (i32.sub (local.get $slot_count) (i32.const 1)))
    (local.set $base (i32.add (local.get $demux) (i32.const 12)))
    (local.set $i (i32.and (local.get $stream_id) (local.get $mask)))
    (block $found
      (loop $probe
        (local.set $sid (i32.load (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8)))))
        (if (i32.eq (local.get $sid) (local.get $stream_id))
          (then (return (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8))))))
        (if (i32.eqz (local.get $sid))
          (then (return (i32.const 0))))
        (local.set $i (i32.and (i32.add (local.get $i) (i32.const 1)) (local.get $mask)))
        (br $probe)))
    (i32.const 0))

  (func (export "demux_register_stream")
    (param $demux i32) (param $stream_id i32) (param $pipe i32) (result i32)
    (local $slot_count i32) (local $mask i32) (local $base i32)
    (local $i i32) (local $sid i32)
    (if (i32.eqz (local.get $stream_id)) (then (return (global.get $STATUS_OVERFLOW))))
    (local.set $slot_count (i32.load offset=8 (local.get $demux)))
    (local.set $mask (i32.sub (local.get $slot_count) (i32.const 1)))
    (local.set $base (i32.add (local.get $demux) (i32.const 12)))
    (local.set $i (i32.and (local.get $stream_id) (local.get $mask)))
    (block $done
      (loop $probe
        (local.set $sid (i32.load (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8)))))
        (if (i32.eqz (local.get $sid))
          (then
            (i32.store (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8))) (local.get $stream_id))
            (i32.store offset=4 (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8))) (local.get $pipe))
            (br $done)))
        (if (i32.eq (local.get $sid) (local.get $stream_id))
          (then
            (i32.store offset=4 (i32.add (local.get $base) (i32.mul (local.get $i) (i32.const 8))) (local.get $pipe))
            (br $done)))
        (local.set $i (i32.and (i32.add (local.get $i) (i32.const 1)) (local.get $mask)))
        (br $probe)))
    global.get $STATUS_OK)

  (func (export "demux_unregister_stream") (param $demux i32) (param $stream_id i32) (result i32)
    (local $slot i32)
    (local.set $slot (call $demux_lookup (local.get $demux) (local.get $stream_id)))
    (if (local.get $slot)
      (then
        (i32.store (local.get $slot) (i32.const 0))
        (i32.store offset=4 (local.get $slot) (i32.const 0))))
    global.get $STATUS_OK)

  ;; demux_dynamic_run(demux, scratch, scap) → 1 (routed) | 0 (no frame) | error
  (func $demux_dynamic_run (export "demux_dynamic_run")
    (param $demux i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $input i32) (local $result i64) (local $status i32)
    (local $stream_id i32) (local $slot i32) (local $pipe i32)
    (local $avail i32)
    (local.set $input (i32.load offset=4 (local.get $demux)))
    ;; Need at least 8 bytes
    (local.set $avail (call $pipe_available (local.get $input)))
    (if (i32.lt_u (local.get $avail) (i32.const 8))
      (then (return (i32.const 0))))
    (local.set $result (call $frame_read (local.get $input) (local.get $scratch) (local.get $scap)))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (i32.ne (local.get $status) (global.get $STATUS_OK))
      (then
        (if (i32.eq (local.get $status) (global.get $STATUS_OVERFLOW))
          (then (return (i32.const 0))))
        (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $stream_id (i32.wrap_i64 (local.get $result)))
    (local.set $slot (call $demux_lookup (local.get $demux) (local.get $stream_id)))
    (if (i32.eqz (local.get $slot))
      (then (return (i32.const -1))))
    (local.set $pipe (i32.load offset=4 (local.get $slot)))
    (local.set $avail (i32.load (local.get $scratch)))
    (drop (call $pipe_write (local.get $pipe) (i32.add (local.get $scratch) (i32.const 4)) (local.get $avail)))
    (i32.const 1))

  ;; ════════════════════════════════════════════════════════════════
  ;; Pipeline stage process functions
  ;; Signature: (input, output, config, clen, scratch, scap) → result
  ;; ════════════════════════════════════════════════════════════════

  ;; process_mux_static — reads from stream pipes in config, frame-writes to output
  ;; Config: [count][pipe_0][pipe_1]...
  ;; State layout (12 bytes):
  ;;   +0: tick          i32 (RO, written by pipeline_run)
  ;;   +4: epoch_len     i32 (0 = no batching, drain every call)
  ;;   +8: last_flush_tick i32 (last tick when drain occurred)
  (func (export "process_mux_static")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $count i32) (local $i i32) (local $pipe i32)
    (local $avail i32) (local $r i32) (local $total i32)
    (local $tick i32) (local $epoch_len i32) (local $last_flush i32)
    (local.set $count (i32.load (local.get $cfg)))

    ;; Epoch batching: if state != 0, check if it's time to drain
    (if (local.get $state)
      (then
        (local.set $tick (i32.load (local.get $state)))
        (local.set $epoch_len (i32.load offset=4 (local.get $state)))
        (local.set $last_flush (i32.load offset=8 (local.get $state)))
        (if (i32.lt_u (i32.sub (local.get $tick) (local.get $last_flush)) (local.get $epoch_len))
          (then (return (i32.const 0))))))

    (block $done
      (loop $streams
        (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
        (local.set $pipe (i32.load (i32.add (local.get $cfg) (i32.add (i32.const 4) (i32.mul (local.get $i) (i32.const 4))))))
        (local.set $avail (call $pipe_available (local.get $pipe)))
        (if (i32.gt_u (local.get $avail) (i32.const 0))
          (then
            (if (i32.gt_u (local.get $avail) (local.get $scap))
              (then (local.set $avail (local.get $scap))))
            (local.set $r (call $pipe_read (local.get $pipe) (local.get $scratch) (local.get $avail)))
            (if (i32.lt_s (local.get $r) (i32.const 0))
              (then (local.set $total (local.get $r)) (br $done)))
            (local.set $r (call $frame_write (local.get $output) (local.get $i) (local.get $scratch) (local.get $r)))
            (if (i32.ne (local.get $r) (global.get $STATUS_OK))
              (then (local.set $total (i32.sub (i32.const 0) (local.get $r))) (br $done)))
            (local.set $total (i32.add (local.get $total) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $streams)))

    ;; Update last_flush_tick after drain (only if state != 0)
    (if (local.get $state)
      (then (i32.store offset=8 (local.get $state) (i32.load (local.get $state)))))
    local.get $total)

  ;; Helper: configure epoch batching on a mux_static state block
  ;; State must be non-zero and point to a 12-byte block
  (func (export "mux_static_set_epoch")
    (param $state i32) (param $epoch_len i32) (result i32)
    (if (i32.eqz (local.get $state))
      (then (return (i32.const -1))))
    (i32.store offset=4 (local.get $state) (local.get $epoch_len))
    (i32.store offset=8 (local.get $state) (i32.const 0))
    (i32.const 0))

  ;; process_demux_static — reads framed from input, routes to stream pipes in config
  ;; Config: [count][pipe_0][pipe_1]...
  ;; Drains all available frames in one call
  (func (export "process_demux_static")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $count i32) (local $avail i32) (local $result i64)
    (local $status i32) (local $stream_id i32) (local $pipe i32)
    (local $total i32)
    (local.set $count (i32.load (local.get $cfg)))
    (block $done
      (loop $frames
        (local.set $avail (call $pipe_available (local.get $input)))
        (br_if $done (i32.lt_u (local.get $avail) (i32.const 8)))
        (local.set $result (call $frame_read (local.get $input) (local.get $scratch) (local.get $scap)))
        (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
        (if (i32.ne (local.get $status) (global.get $STATUS_OK))
          (then
            (if (i32.eq (local.get $status) (global.get $STATUS_OVERFLOW))
              (then (br $done)))
            (local.set $total (local.get $status)) (br $done)))
        (local.set $stream_id (i32.wrap_i64 (local.get $result)))
        (if (i32.ge_u (local.get $stream_id) (local.get $count))
          (then (local.set $total (i32.const -1)) (br $done)))
        (local.set $pipe (i32.load (i32.add (local.get $cfg) (i32.add (i32.const 4) (i32.mul (local.get $stream_id) (i32.const 4))))))
        (local.set $avail (i32.load (local.get $scratch)))
        (drop (call $pipe_write (local.get $pipe) (i32.add (local.get $scratch) (i32.const 4)) (local.get $avail)))
        (local.set $total (i32.add (local.get $total) (i32.const 1)))
        (br $frames)))
    (if (i32.lt_s (local.get $total) (i32.const 0))
      (then (return (local.get $total))))
    (global.get $STATUS_OK))

  ;; process_mux_dynamic — calls mux_dynamic_run on pre-created mux handle
  ;; Config: [mux_handle:i32]
  (func (export "process_mux_dynamic")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $mux i32)
    (local.set $mux (i32.load (local.get $cfg)))
    (call $mux_dynamic_run (local.get $mux) (local.get $scratch) (local.get $scap)))

  ;; process_demux_dynamic — calls demux_dynamic_run on pre-created demux handle
  ;; Config: [demux_handle:i32]
  (func (export "process_demux_dynamic")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $demux i32)
    (local.set $demux (i32.load (local.get $cfg)))
    (call $demux_dynamic_run (local.get $demux) (local.get $scratch) (local.get $scap)))

  ;; memcpy imported from edgerun-core as $memcpy_off(dst, doff, src, soff, len)

;; Standard ID removed — merged into single module

  (func (export "ws_decode_prefix") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $opcode i32)
    (local $code i32)
    local.get $len
    i32.const 2
    i32.lt_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    i32.load8_u
    local.set $b0
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.set $b1
    local.get $b0
    i32.const 15
    i32.and
    local.set $opcode
    local.get $b1
    i32.const 127
    i32.and
    local.set $code
    local.get $opcode
    i32.const 0
    i32.eq
    local.get $opcode
    i32.const 1
    i32.eq
    i32.or
    local.get $opcode
    i32.const 2
    i32.eq
    i32.or
    local.get $opcode
    i32.const 8
    i32.eq
    i32.or
    local.get $opcode
    i32.const 9
    i32.eq
    i32.or
    local.get $opcode
    i32.const 10
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $opcode
    i32.const 8
    i32.ge_u
    if
      local.get $b0
      i32.const 128
      i32.and
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $code
      i32.const 125
      i32.gt_u
      if
        i32.const 3
        return
      end
    end
    local.get $out
    local.get $b0
    i32.const 128
    i32.and
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $opcode
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $b1
    i32.const 128
    i32.and
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $code
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $code
    i32.const 126
    i32.eq
    if (result i32)
      i32.const 2
    else
      local.get $code
      i32.const 127
      i32.eq
      if (result i32)
        i32.const 8
      else
        i32.const 0
      end
    end
    i32.store
    i32.const 0)

  (func (export "ws_decode_payload_len") (param $ptr i32) (param $len i32) (param $code i32) (param $max_len i32) (param $out i32) (result i32)
    (local $low i32)
    (local $high i32)
    (local $extra i32)
    local.get $code
    i32.const 126
    i32.lt_u
    if
      local.get $code
      local.set $low
      i32.const 0
      local.set $high
      i32.const 0
      local.set $extra
    else
      local.get $code
      i32.const 126
      i32.eq
      if
        local.get $len
        i32.const 2
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.load8_u
        i32.const 8
        i32.shl
        local.get $ptr
        i32.const 1
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 0
        local.set $high
        i32.const 2
        local.set $extra
      else
        local.get $code
        i32.const 127
        i32.ne
        if
          i32.const 3
          return
        end
        local.get $len
        i32.const 8
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.load8_u
        i32.const 128
        i32.and
        if
          i32.const 3
          return
        end
        local.get $ptr
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 3
        i32.add
        i32.load8_u
        i32.or
        local.set $high
        local.get $ptr
        i32.const 4
        i32.add
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 5
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 6
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 7
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 8
        local.set $extra
      end
    end
    local.get $code
    i32.const 126
    i32.eq
    local.get $high
    i32.eqz
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    i32.and
    local.get $code
    i32.const 127
    i32.eq
    local.get $high
    i32.eqz
    local.get $low
    i32.const 65536
    i32.lt_u
    i32.and
    i32.and
    i32.or
    if
      i32.const 3
      return
    end
    local.get $high
    i32.eqz
    i32.eqz
    local.get $low
    local.get $max_len
    i32.gt_u
    i32.or
    if
      i32.const 4
      return
    end
    local.get $out
    local.get $low
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $high
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $extra
    i32.store
    i32.const 0)

  (func $ws_apply_mask (export "ws_apply_mask_in_place") (param $payload_ptr i32) (param $payload_len i32) (param $mask i32) (result i32)
    (local $i i32)
    (local $shift i32)
    loop $mask_loop
      local.get $i
      local.get $payload_len
      i32.ge_u
      if
        i32.const 0
        return
      end
      local.get $i
      i32.const 3
      i32.and
      i32.const 3
      i32.xor
      i32.const 8
      i32.mul
      local.set $shift
      local.get $payload_ptr
      local.get $i
      i32.add
      local.get $payload_ptr
      local.get $i
      i32.add
      i32.load8_u
      local.get $mask
      local.get $shift
      i32.shr_u
      i32.const 255
      i32.and
      i32.xor
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $mask_loop
    end
    i32.const 0)

  (func $ws_parse_header (export "ws_parse_header") (param $ptr i32) (param $len i32) (param $max_len i32) (param $out i32) (result i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $code i32)
    (local $extra i32)
    (local $masked i32)
    (local $header_len i32)
    (local $low i32)
    (local $high i32)
    local.get $len
    i32.const 2
    i32.lt_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    i32.load8_u
    local.set $b0
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.set $b1
    local.get $b1
    i32.const 127
    i32.and
    local.set $code
    i32.const 2
    local.set $header_len
    local.get $b1
    i32.const 128
    i32.and
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end
    local.set $masked

    local.get $b0
    i32.const 15
    i32.and
    i32.const 0
    i32.eq
    local.get $b0
    i32.const 15
    i32.and
    i32.const 1
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 2
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 8
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 9
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 10
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $b0
    i32.const 15
    i32.and
    i32.const 8
    i32.ge_u
    if
      local.get $b0
      i32.const 128
      i32.and
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $code
      i32.const 125
      i32.gt_u
      if
        i32.const 3
        return
      end
    end

    local.get $code
    i32.const 126
    i32.lt_u
    if
      local.get $code
      local.set $low
      i32.const 0
      local.set $high
      i32.const 0
      local.set $extra
    else
      local.get $code
      i32.const 126
      i32.eq
      if
        local.get $len
        i32.const 4
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        local.get $ptr
        i32.const 3
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 0
        local.set $high
        i32.const 2
        local.set $extra
        i32.const 4
        local.set $header_len
      else
        local.get $len
        i32.const 10
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 128
        i32.and
        if
          i32.const 3
          return
        end
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 3
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 4
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 5
        i32.add
        i32.load8_u
        i32.or
        local.set $high
        local.get $ptr
        i32.const 6
        i32.add
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 7
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 8
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 9
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 8
        local.set $extra
        i32.const 10
        local.set $header_len
      end
    end

    local.get $code
    i32.const 126
    i32.eq
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    local.get $code
    i32.const 127
    i32.eq
    local.get $high
    i32.eqz
    local.get $low
    i32.const 65536
    i32.lt_u
    i32.and
    i32.and
    i32.or
    if
      i32.const 3
      return
    end
    local.get $high
    i32.eqz
    i32.eqz
    local.get $low
    local.get $max_len
    i32.gt_u
    i32.or
    if
      i32.const 4
      return
    end
    local.get $masked
    if
      local.get $len
      local.get $header_len
      i32.const 4
      i32.add
      i32.lt_u
      if
        i32.const 1
        return
      end
    end

    local.get $out
    local.get $b0
    i32.const 128
    i32.and
    if (result i32) i32.const 1 else i32.const 0 end
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $b0
    i32.const 112
    i32.and
    i32.const 4
    i32.shr_u
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $b0
    i32.const 15
    i32.and
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $masked
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $low
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $high
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $header_len
    local.get $masked
    if (result i32) i32.const 4 else i32.const 0 end
    i32.add
    i32.store
    local.get $out
    i32.const 28
    i32.add
    local.get $masked
    if (result i32)
      local.get $ptr
      local.get $header_len
      i32.add
      i32.load8_u
      i32.const 24
      i32.shl
      local.get $ptr
      local.get $header_len
      i32.add
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 16
      i32.shl
      i32.or
      local.get $ptr
      local.get $header_len
      i32.add
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 8
      i32.shl
      i32.or
      local.get $ptr
      local.get $header_len
      i32.add
      i32.const 3
      i32.add
      i32.load8_u
      i32.or
    else
      i32.const 0
    end
    i32.store
    i32.const 0)

  (func $ws_write_hdr (export "ws_write_frame_header") (param $opcode i32) (param $flags i32) (param $low i32) (param $high i32) (param $mask_present i32) (param $mask i32) (param $out i32) (param $cap i32) (result i64)
    (local $written i32)
    local.get $opcode
    i32.const 0
    i32.eq
    local.get $opcode
    i32.const 1
    i32.eq
    i32.or
    local.get $opcode
    i32.const 2
    i32.eq
    i32.or
    local.get $opcode
    i32.const 8
    i32.eq
    i32.or
    local.get $opcode
    i32.const 9
    i32.eq
    i32.or
    local.get $opcode
    i32.const 10
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $opcode
    i32.const 8
    i32.ge_u
    if
      local.get $flags
      i32.const 128
      i32.and
      i32.eqz
      local.get $high
      i32.const 0
      i32.ne
      i32.or
      local.get $low
      i32.const 125
      i32.gt_u
      i32.or
      if
        i32.const 3
        i32.const 0
        call $pack
        return
      end
    end
    local.get $high
    i32.const 2147483648
    i32.ge_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $high
    i32.eqz
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    if
      i32.const 2
      local.set $written
    else
      local.get $high
      i32.eqz
      local.get $low
      i32.const 65536
      i32.lt_u
      i32.and
      if
        i32.const 4
        local.set $written
      else
        i32.const 10
        local.set $written
      end
    end
    local.get $mask_present
    if
      local.get $written
      i32.const 4
      i32.add
      local.set $written
    end
    local.get $cap
    local.get $written
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $out
    local.get $flags
    i32.const 240
    i32.and
    local.get $opcode
    i32.const 15
    i32.and
    i32.or
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $mask_present
    if (result i32) i32.const 128 else i32.const 0 end
    local.get $written
    local.get $mask_present
    if (result i32) i32.const 4 else i32.const 0 end
    i32.sub
    i32.const 2
    i32.eq
    if (result i32)
      local.get $low
    else
      local.get $written
      local.get $mask_present
      if (result i32) i32.const 4 else i32.const 0 end
      i32.sub
      i32.const 4
      i32.eq
      if (result i32) i32.const 126 else i32.const 127 end
    end
    i32.or
    i32.store8
    local.get $written
    local.get $mask_present
    if (result i32) i32.const 4 else i32.const 0 end
    i32.sub
    i32.const 4
    i32.eq
    if
      local.get $out
      i32.const 2
      i32.add
      local.get $low
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 3
      i32.add
      local.get $low
      i32.store8
    end
    local.get $written
    local.get $mask_present
    if (result i32) i32.const 4 else i32.const 0 end
    i32.sub
    i32.const 10
    i32.eq
    if
      local.get $out
      i32.const 2
      i32.add
      local.get $high
      i32.const 24
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 3
      i32.add
      local.get $high
      i32.const 16
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 4
      i32.add
      local.get $high
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 5
      i32.add
      local.get $high
      i32.store8
      local.get $out
      i32.const 6
      i32.add
      local.get $low
      i32.const 24
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 7
      i32.add
      local.get $low
      i32.const 16
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 8
      i32.add
      local.get $low
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 9
      i32.add
      local.get $low
      i32.store8
    end
    local.get $mask_present
    if
      local.get $out
      local.get $written
      i32.const 4
      i32.sub
      i32.add
      local.get $mask
      i32.const 24
      i32.shr_u
      i32.store8
      local.get $out
      local.get $written
      i32.const 3
      i32.sub
      i32.add
      local.get $mask
      i32.const 16
      i32.shr_u
      i32.store8
      local.get $out
      local.get $written
      i32.const 2
      i32.sub
      i32.add
      local.get $mask
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      local.get $written
      i32.const 1
      i32.sub
      i32.add
      local.get $mask
      i32.store8
    end
    i32.const 0
    local.get $written
    call $pack)

  (func (export "ws_parse_close_payload") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    local.get $len
    i32.const 125
    i32.gt_u
    if
      i32.const 3
      return
    end
    local.get $out
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32) i32.const 1 else i32.const 0 end
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32)
      local.get $ptr
      i32.load8_u
      i32.const 8
      i32.shl
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.or
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32) local.get $len i32.const 2 i32.sub else i32.const 0 end
    i32.store
    i32.const 0)

  (func (export "ws_write_close_payload") (param $code i32) (param $reason_ptr i32) (param $reason_len i32) (param $out i32) (param $cap i32) (result i64)
    (local $i i32)
    local.get $code
    i32.const 65535
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $reason_len
    i32.const 123
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $cap
    local.get $reason_len
    i32.const 2
    i32.add
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $out
    local.get $code
    i32.const 8
    i32.shr_u
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $code
    i32.store8
    loop $copy
      local.get $i
      local.get $reason_len
      i32.ge_u
      if
        i32.const 0
        local.get $reason_len
        i32.const 2
        i32.add
        call $pack
        return
      end
      local.get $out
      i32.const 2
      i32.add
      local.get $i
      i32.add
      local.get $reason_ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $copy
    end
    i32.const 0
    i32.const 0
    call $pack)

  (func $ws_write_srv_hdr (export "ws_write_server_frame_header") (param $opcode i32) (param $low i32) (param $high i32) (param $out i32) (param $cap i32) (result i64)
    (local $written i32)
    local.get $opcode
    i32.const 0
    i32.eq
    local.get $opcode
    i32.const 1
    i32.eq
    i32.or
    local.get $opcode
    i32.const 2
    i32.eq
    i32.or
    local.get $opcode
    i32.const 8
    i32.eq
    i32.or
    local.get $opcode
    i32.const 9
    i32.eq
    i32.or
    local.get $opcode
    i32.const 10
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $opcode
    i32.const 8
    i32.ge_u
    local.get $high
    i32.eqz
    local.get $low
    i32.const 125
    i32.gt_u
    i32.and
    i32.and
    local.get $opcode
    i32.const 8
    i32.ge_u
    local.get $high
    i32.const 0
    i32.ne
    i32.and
    i32.or
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $high
    i32.eqz
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    if
      i32.const 2
      local.set $written
      local.get $cap
      local.get $written
      i32.lt_u
      if
        i32.const 2
        i32.const 0
        call $pack
        return
      end
      local.get $out
      local.get $opcode
      i32.const 128
      i32.or
      i32.store8
      local.get $out
      i32.const 1
      i32.add
      local.get $low
      i32.store8
    else
      local.get $high
      i32.eqz
      local.get $low
      i32.const 65536
      i32.lt_u
      i32.and
      if
        i32.const 4
        local.set $written
        local.get $cap
        local.get $written
        i32.lt_u
        if
          i32.const 2
          i32.const 0
          call $pack
          return
        end
        local.get $out
        local.get $opcode
        i32.const 128
        i32.or
        i32.store8
        local.get $out
        i32.const 1
        i32.add
        i32.const 126
        i32.store8
        local.get $out
        i32.const 2
        i32.add
        local.get $low
        i32.const 8
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 3
        i32.add
        local.get $low
        i32.store8
      else
        i32.const 10
        local.set $written
        local.get $high
        i32.const 2147483648
        i32.ge_u
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $cap
        local.get $written
        i32.lt_u
        if
          i32.const 2
          i32.const 0
          call $pack
          return
        end
        local.get $out
        local.get $opcode
        i32.const 128
        i32.or
        i32.store8
        local.get $out
        i32.const 1
        i32.add
        i32.const 127
        i32.store8
        local.get $out
        i32.const 2
        i32.add
        local.get $high
        i32.const 24
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 3
        i32.add
        local.get $high
        i32.const 16
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 4
        i32.add
        local.get $high
        i32.const 8
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 5
        i32.add
        local.get $high
        i32.store8
        local.get $out
        i32.const 6
        i32.add
        local.get $low
        i32.const 24
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 7
        i32.add
        local.get $low
        i32.const 16
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 8
        i32.add
        local.get $low
        i32.const 8
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 9
        i32.add
        local.get $low
        i32.store8
      end
    end
    i32.const 0
    local.get $written
    call $pack)

;; Standard ID removed — merged into single module

  ;; ════════════════════════════════════════════════════════════════
  ;; ws_encode — payload → WS frame (pure transform, batch)
  ;;
  ;; Config (optional, 4 bytes): [opcode:i32]
  ;;   +0: opcode (1=text, 2=binary, default=2). 0 defaults to binary.
  ;;
  ;; Reads payload from input pipe, wraps in WS server frame (FIN, no mask),
  ;; writes complete WS frame to output pipe.
  ;;
  ;; Signature: (input, output, cfg, clen, scratch, scap, state) → bytes_written
  ;; Batch stage: state is ignored (pass 0).
  ;; ════════════════════════════════════════════════════════════════

  (func (export "ws_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $opcode i32) (local $n i32) (local $result i64)
    (local $status i32) (local $hdr_len i32)

    (local.set $opcode (i32.const 2))
    (if (i32.ge_u (local.get $clen) (i32.const 4))
      (then
        (local.set $opcode (i32.load (local.get $cfg)))
        (if (i32.eqz (local.get $opcode))
          (then (local.set $opcode (i32.const 2))))))

    (if (i32.lt_u (local.get $scap) (i32.const 17))
      (then (return (i32.const -1))))
    ;; Read payload into scratch+16 (leave 16 bytes for max header)
    (local.set $n (call $pipe_read (local.get $input)
      (i32.add (local.get $scratch) (i32.const 16))
      (i32.sub (local.get $scap) (i32.const 16))))
    (if (i32.le_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))

    ;; Write WS server frame header to scratch[0..9] (max 10 bytes, no mask)
    (local.set $result (call $ws_write_srv_hdr
      (local.get $opcode) (local.get $n) (i32.const 0)
      (local.get $scratch) (i32.const 10)))
    (local.set $status (i32.wrap_i64 (local.get $result)))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $hdr_len (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))

    ;; Write header to output
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $hdr_len)))
    ;; Write payload to output
    (drop (call $pipe_write (local.get $output)
      (i32.add (local.get $scratch) (i32.const 16)) (local.get $n)))
    (i32.add (local.get $hdr_len) (local.get $n)))

  ;; ════════════════════════════════════════════════════════════════
  ;; ws_decode — WS frame → payload (pure transform, streaming)
  ;;
  ;; Config: none (pass 0, 0).
  ;;
  ;; State layout (148 bytes):
  ;;   +0:   tick      i32 (RO)
  ;;   +4:   (reserved)
  ;;   +8:   dbuf_len  i32
  ;;   +12:  dbuf[136] decode buffer (partial frame data)
  ;;
  ;; Thin wrapper around $decode_frame_socket with dbuf_off=8 and no send_pipe.
  ;; ════════════════════════════════════════════════════════════════

  (func (export "ws_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (call $decode_frame_socket
      (local.get $input) (local.get $scratch) (local.get $scap)
      (local.get $state) (i32.const 8) (local.get $output) (i32.const 0)))

  ;; ════════════════════════════════════════════════════════════════
  ;; process_ws_frame — bidirectional WS framing (socket mode)
  ;; Config: pointer to socket handle (4 bytes)
  ;; State layout (148 bytes):
  ;;   +0:   tick          i32 (RO)
  ;;   +4:   phase         i32 (0=idle, 1=awaiting)
  ;;   +8:   start_tick    i32
  ;;   +12:  timeout_ticks i32
  ;;   +16:  dbuf_len      i32
  ;;   +20:  dbuf[128]     decode buffer
  ;;
  ;; Reads payload from input, wraps in WS frame, writes to socket send pipe.
  ;; Reads raw bytes from socket recv pipe, decodes WS frames, writes payload
  ;; to output pipe. Ping→Pong, Close→Close echo.
  ;; ════════════════════════════════════════════════════════════════

  (func (export "process_ws_frame")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $sock i32) (local $send_pipe i32) (local $recv_pipe i32)
    (local $phase i32) (local $elapsed i32) (local $sent i32)
    (local $n i32) (local $result i64) (local $status i32)

    (if (i32.lt_u (local.get $clen) (i32.const 4))
      (then (return (i32.const -1))))
    (local.set $sock (i32.load (local.get $cfg)))
    (local.set $send_pipe (i32.load (local.get $sock)))
    (local.set $recv_pipe (i32.load offset=4 (local.get $sock)))
    (local.set $phase (i32.load offset=4 (local.get $state)))

    ;; Phase 1 (awaiting response)
    (if (i32.eq (local.get $phase) (i32.const 1))
      (then
        (local.set $elapsed
          (i32.sub (i32.load (local.get $state)) (i32.load offset=8 (local.get $state))))
        (if (i32.load offset=12 (local.get $state))
          (then
            (if (i32.ge_u (local.get $elapsed) (i32.load offset=12 (local.get $state)))
              (then (return (global.get $STATUS_TIMEOUT))))))

        ;; Decode from recv pipe
        (local.set $n (call $decode_frame_socket
          (local.get $recv_pipe) (local.get $scratch) (local.get $scap)
          (local.get $state) (i32.const 20) (local.get $output) (local.get $send_pipe)))
        (if (i32.gt_s (local.get $n) (i32.const 0))
          (then
            (if (i32.ne (local.get $n) (global.get $STATUS_MORE))
              (then
                (i32.store offset=4 (local.get $state) (i32.const 0))
                (return (local.get $n))))))
        (if (i32.lt_s (local.get $n) (i32.const 0))
          (then (return (local.get $n))))
        (return (global.get $STATUS_MORE))))

    ;; Phase 0 (idle): encode + try decode

    ;; Encode: read payload, wrap in WS frame, send
    (if (i32.lt_u (local.get $scap) (i32.const 17))
      (then (return (i32.const -1))))
    (local.set $n (call $pipe_read (local.get $input)
      (i32.add (local.get $scratch) (i32.const 16))
      (i32.sub (local.get $scap) (i32.const 16))))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then
        (local.set $sent (local.get $n))
        (local.set $result (call $ws_write_srv_hdr
          (i32.const 2) (local.get $n) (i32.const 0)
          (local.get $scratch) (i32.const 10)))
        (local.set $status (i32.wrap_i64 (local.get $result)))
        (if (i32.eqz (local.get $status))
          (then
            (local.set $n (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
            (drop (call $pipe_write (local.get $send_pipe) (local.get $scratch) (local.get $n)))
            (drop (call $pipe_write (local.get $send_pipe)
              (i32.add (local.get $scratch) (i32.const 16)) (local.get $sent)))))))

    ;; Decode from recv pipe
    (local.set $n (call $decode_frame_socket
      (local.get $recv_pipe) (local.get $scratch) (local.get $scap)
      (local.get $state) (i32.const 20) (local.get $output) (local.get $send_pipe)))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then
        (if (i32.ne (local.get $n) (global.get $STATUS_MORE))
          (then (return (local.get $n))))))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))

    ;; Decode returned MORE
    (if (local.get $sent)
      (then
        (i32.store offset=4 (local.get $state) (i32.const 1))
        (i32.store offset=8 (local.get $state) (i32.load (local.get $state)))))
    (global.get $STATUS_MORE))

  ;; ── $decode_frame_socket — same as ws_decode but reads from recv pipe ──
  ;; and handles control frames (ping→pong, close→close echo).
  ;; dbuf stored at state + dbuf_off (rather than fixed offset +8).
  (func $decode_frame_socket
    (param $recv i32) (param $scratch i32) (param $scap i32)
    (param $state i32) (param $dbuf_off i32)
    (param $output i32) (param $send_pipe i32) (result i32)
    (local $dbuf_len i32) (local $n i32) (local $total i32) (local $r i32)
    (local $hdr_out i32)
    (local $fin i32) (local $opcode i32) (local $masked i32)
    (local $hdr_len i32) (local $payload_len i32) (local $mask_key i32)
    (local $frame_size i32) (local $off i32)
    (local $result i64) (local $status i32) (local $written i32)
    (local $dbuf_cap i32)

    (if (i32.lt_u (local.get $scap) (i32.const 32))
      (then (return (i32.const -1))))
    (local.set $dbuf_cap (i32.sub (i32.const 144) (local.get $dbuf_off)))
    (local.set $dbuf_len (i32.load (i32.add (local.get $state) (local.get $dbuf_off))))

    (if (i32.gt_u (local.get $dbuf_len) (i32.const 0))
      (then
        (call $memcpy (local.get $scratch)
          (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (local.get $dbuf_len))))

    (local.set $n (call $pipe_read (local.get $recv)
      (i32.add (local.get $scratch) (local.get $dbuf_len))
      (i32.sub (local.get $scap) (local.get $dbuf_len))))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then (local.set $n (i32.const 0))))
    (local.set $total (i32.add (local.get $dbuf_len) (local.get $n)))

    (if (i32.lt_u (local.get $total) (i32.const 2))
      (then
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $total))
        (return (global.get $STATUS_MORE))))

    (local.set $hdr_out (i32.sub (local.get $scap) (i32.const 32)))
    (local.set $r (call $ws_parse_header
      (local.get $scratch) (local.get $total) (local.get $scap) (local.get $hdr_out)))

    (if (i32.eq (local.get $r) (i32.const 1))
      (then
        (if (i32.gt_u (local.get $total) (local.get $dbuf_cap))
          (then (return (i32.const -5))))
        (call $memcpy (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (local.get $scratch) (local.get $total))
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $total))
        (return (global.get $STATUS_MORE))))

    (if (i32.eq (local.get $r) (i32.const 3)) (then (return (i32.const -3))))
    (if (i32.eq (local.get $r) (i32.const 4)) (then (return (i32.const -4))))

    (local.set $fin    (i32.load offset=0 (local.get $hdr_out)))
    (local.set $opcode (i32.load offset=8 (local.get $hdr_out)))
    (local.set $masked (i32.load offset=12 (local.get $hdr_out)))
    (local.set $payload_len (i32.load offset=16 (local.get $hdr_out)))
    (local.set $hdr_len (i32.load offset=24 (local.get $hdr_out)))
    (local.set $mask_key (i32.load offset=28 (local.get $hdr_out)))

    (local.set $frame_size (i32.add (local.get $hdr_len) (local.get $payload_len)))
    (if (i32.gt_u (local.get $frame_size) (local.get $total))
      (then
        (if (i32.gt_u (local.get $total) (local.get $dbuf_cap))
          (then (return (i32.const -5))))
        (call $memcpy (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (local.get $scratch) (local.get $total))
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $total))
        (return (global.get $STATUS_MORE))))

    (local.set $off (local.get $hdr_len))

    (if (local.get $masked)
      (then
        (drop (call $ws_apply_mask
          (i32.add (local.get $scratch) (local.get $off))
          (local.get $payload_len) (local.get $mask_key)))))

    (block $ctrl
      (if (i32.and (i32.ge_u (local.get $opcode) (i32.const 1))
                   (i32.le_u (local.get $opcode) (i32.const 2)))
        (then
          (drop (call $pipe_write (local.get $output)
            (i32.add (local.get $scratch) (local.get $off))
            (local.get $payload_len)))
          (br $ctrl)))

      (if (i32.eq (local.get $opcode) (i32.const 0))
        (then
          (drop (call $pipe_write (local.get $output)
            (i32.add (local.get $scratch) (local.get $off))
            (local.get $payload_len)))
          (br $ctrl)))

      (if (i32.eq (local.get $opcode) (i32.const 9))
        (then
          (if (local.get $send_pipe)
            (then
              (local.set $result (call $ws_write_hdr
                (i32.const 10) (i32.const 128)
                (local.get $payload_len) (i32.const 0)
                (i32.const 0) (i32.const 0)
                (local.get $scratch) (i32.const 10)))
              (local.set $status (i32.wrap_i64 (local.get $result)))
              (if (i32.eqz (local.get $status))
                (then
                  (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
                  (drop (call $pipe_write (local.get $send_pipe) (local.get $scratch) (local.get $written)))
                  (drop (call $pipe_write (local.get $send_pipe)
                    (i32.add (local.get $scratch) (local.get $off))
                    (local.get $payload_len)))))))
          (br $ctrl)))

      (if (i32.eq (local.get $opcode) (i32.const 8))
        (then
          (if (local.get $send_pipe)
            (then
              (local.set $result (call $ws_write_hdr
                (i32.const 8) (i32.const 128)
                (local.get $payload_len) (i32.const 0)
                (i32.const 0) (i32.const 0)
                (local.get $scratch) (i32.const 10)))
              (local.set $status (i32.wrap_i64 (local.get $result)))
              (if (i32.eqz (local.get $status))
                (then
                  (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
                  (drop (call $pipe_write (local.get $send_pipe) (local.get $scratch) (local.get $written)))
                  (drop (call $pipe_write (local.get $send_pipe)
                    (i32.add (local.get $scratch) (local.get $off))
                    (local.get $payload_len)))))))
          (br $ctrl))))

    ;; Save overflow
    (local.set $off (i32.add (local.get $off) (local.get $payload_len)))
    (local.set $n (i32.sub (local.get $total) (local.get $off)))
    (if (i32.gt_u (local.get $n) (i32.const 0))
      (then
        (if (i32.gt_u (local.get $n) (local.get $dbuf_cap))
          (then (return (i32.const -5))))
        (call $memcpy (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (i32.add (local.get $scratch) (local.get $off)) (local.get $n))
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $n)))
      (else
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (i32.const 0))))

    (if (i32.le_u (local.get $opcode) (i32.const 2))
      (then (return (local.get $payload_len))))
    (if (i32.eq (local.get $opcode) (i32.const 9))
      (then (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 8))
      (then (return (i32.const 1))))
    (i32.const 1))

  ;; memcpy imported from edgerun-core as $memcpy(dst, src, len)

  ;; ── Memory offsets ──────────────────────────────────────────────────
  (global $OFF_ERR         i32 (i32.const 0))
  (global $OFF_SCRATCH0    i32 (i32.const 8))
  (global $OFF_SCRATCH1    i32 (i32.const 16))
  (global $OFF_SCRATCH2    i32 (i32.const 24))
  (global $OFF_SCRATCH3    i32 (i32.const 32))
  (global $FAST_STACK_LEN  (mut i32) (i32.const 0))
  (global $OFF_WASM_PTR    i32 (i32.const 64))
  (global $OFF_WASM_LEN    i32 (i32.const 72))

  ;; Type section
  (global $OFF_TYPE_COUNT i32 (i32.const 256))

  ;; Import section
  (global $OFF_IMPORT_COUNT i32 (i32.const 16648))
  (global $OFF_IMPORTS_BUF  i32 (i32.const 16656))

  ;; Function section
  (global $OFF_FUNCTION_COUNT i32 (i32.const 17680))

  ;; Code section
  (global $OFF_CODE_COUNT i32 (i32.const 21784))

  ;; Export section
  (global $OFF_EXPORT_COUNT i32 (i32.const 38176))
  (global $OFF_EXPORTS_BUF  i32 (i32.const 38184))

  ;; Global section
  (global $OFF_GLOBAL_COUNT i32 (i32.const 40232))
  (global $OFF_GLOBALS_BUF  i32 (i32.const 40240))

  ;; Table state
  (global $OFF_TABLE_HAS i32 (i32.const 42288))
  (global $OFF_TABLE_MIN i32 (i32.const 42292))
  (global $OFF_TABLE_MAX i32 (i32.const 42300))
  (global $OFF_TABLE_DATA i32 (i32.const 42308))

  ;; Memory state
  (global $OFF_MEM_MIN i32 (i32.const 43300))
  (global $OFF_MEM_MAX i32 (i32.const 43308))

  ;; Start function index
  (global $OFF_START_FUNC i32 (i32.const 43316))

  ;; Data segments
  (global $OFF_DATA_COUNT i32 (i32.const 43324))
  (global $OFF_DATA_BUF   i32 (i32.const 43332))

  ;; Element segments
  (global $OFF_ELEM_COUNT i32 (i32.const 45380))
  (global $OFF_ELEM_BUF   i32 (i32.const 45388))

  (global $OFF_DBG (export "dbg") i32 (i32.const 0x0F0000))

  ;; SIMD encoding control: 0 = old (AArch64), 1 = modern wabt
  ;; Set by the host before calling decode_opcodes for modern-encoding WASM.
  (global $SIMD_MODERN_ENCODING (export "simd_modern_encoding") (mut i32) (i32.const 0))

  ;; Execution state
  (global $OFF_EXEC_LOCAL_COUNT i32 (i32.const 80000))
  (global $OFF_EXEC_LOCALS      i32 (i32.const 80008))
  (global $OFF_EXEC_STACK_LEN   i32 (i32.const 80520))
  (global $OFF_EXEC_STACK       i32 (i32.const 80528))
  (global $OFF_EXEC_CTRL_LEN    i32 (i32.const 88720))
  (global $OFF_EXEC_CTRL        i32 (i32.const 88728))
  (global $OFF_EXEC_DEC_IDX     i32 (i32.const 89752))
  (global $OFF_EXEC_DEC_END     i32 (i32.const 89760))
  (global $OFF_EXEC_RESULT      i32 (i32.const 89768))
  (global $OFF_EXEC_RES_COUNT   i32 (i32.const 89800))
  (global $OFF_EXEC_BODY_PTR    i32 (i32.const 89808))
  (global $OFF_EXEC_BODY_LEN    i32 (i32.const 89816))
  (global $OFF_EXEC_READER_OFF  i32 (i32.const 89824))
  (global $OFF_EXEC_TYPE_IDX    i32 (i32.const 89832))
  (global $OFF_EXEC_CALL_DEPTH  i32 (i32.const 89840))
  (global $OFF_EXEC_FRAME_PTR   i32 (i32.const 89848))
  (global $OFF_EXEC_MOD_VALID   i32 (i32.const 89856))

  (global $OFF_GUEST_MEM_PAGES i32 (i32.const 89868))  ;; current guest memory pages (mutable)

  (global $OFF_FRAME_SAVE i32 (i32.const 0x70000))
  (global $FRAME_SAVE_SZ  i32 (i32.const 65536))
  (global $OFF_NAMES_BUF     i32 (i32.const 0x90000))
  (global $OFF_NAMES_PTR    i32 (i32.const 0x8FFF0))
  (global $OFF_GUEST_MEM_BASE i32 (i32.const 0x200000))
  (global $OFF_CALL_ARGS    i32 (i32.const 0x8FE00))

  ;; Label stack for computing matching block/loop/if/else/end during decode
  (global $OFF_LABEL_STACK     i32 (i32.const 0x8F000))
  (global $OFF_LABEL_STACK_PTR i32 (i32.const 0x8EFFC))

  ;; ── Resource limits ─────────────────────────────────────────────────
  (global $MAX_FUNCTIONS i32 (i32.const 256))
  (global $MAX_IMPORTS   i32 (i32.const 16))
  (global $MAX_TYPES     i32 (i32.const 64))
  (global $MAX_LOCALS    i32 (i32.const 64))
  (global $MAX_STACK     i32 (i32.const 1024))
  (global $MAX_CTRL      i32 (i32.const 64))
  (global $MAX_GLOBALS   i32 (i32.const 64))
  (global $MAX_EXPORTS   i32 (i32.const 64))
  (global $MAX_TABLE     i32 (i32.const 256))
  (global $MAX_DATAS     i32 (i32.const 64))
  (global $MAX_ELEMS     i32 (i32.const 64))
  (global $MAX_DECODED   i32 (i32.const 32768))
  (global $DECODED_SZ    i32 (i32.const 32))

  ;; ── Struct sizes ────────────────────────────────────────────────────
  (global $SZ_EXPORT i32 (i32.const 32))
  (global $SZ_GLOBAL i32 (i32.const 32))
  (global $SZ_IMPORT i32 (i32.const 64))
  (global $SZ_DATA   i32 (i32.const 32))
  (global $SZ_ELEM   i32 (i32.const 512))
  (global $SZ_CTRL   i32 (i32.const 16))

  ;; ── WASM format ─────────────────────────────────────────────────────
  (global $TI32 i32 (i32.const 0x7f))
  (global $TI64 i32 (i32.const 0x7e))
  (global $TF32 i32 (i32.const 0x7d))
  (global $TF64 i32 (i32.const 0x7c))

  (global $SEC_TYPE i32 (i32.const 1))
  (global $SEC_IMPORT i32 (i32.const 2))
  (global $SEC_FUNCTION i32 (i32.const 3))
  (global $SEC_TABLE i32 (i32.const 4))
  (global $SEC_MEMORY i32 (i32.const 5))
  (global $SEC_GLOBAL i32 (i32.const 6))
  (global $SEC_EXPORT i32 (i32.const 7))
  (global $SEC_START i32 (i32.const 8))
  (global $SEC_ELEMENT i32 (i32.const 9))
  (global $SEC_CODE i32 (i32.const 10))
  (global $SEC_DATA i32 (i32.const 11))

  (global $EXT_FUNC i32 (i32.const 0))
  (global $EXT_TABLE i32 (i32.const 1))
  (global $EXT_MEM i32 (i32.const 2))
  (global $EXT_GLOBAL i32 (i32.const 3))

  ;; ── Error codes ─────────────────────────────────────────────────────
  (global $OK           i32 (i32.const 0))
  (global $ERR_UNSUP    i32 (i32.const 1))
  (global $ERR_CORRUPT  i32 (i32.const 2))
  (global $ERR_STK_UND  i32 (i32.const 3))
  (global $ERR_STK_OV   i32 (i32.const 4))
  (global $ERR_PARSE    i32 (i32.const 7))
  (global $ERR_NOIMP    i32 (i32.const 8))
  (global $ERR_TM       i32 (i32.const 9))
  (global $ERR_UNK_OP   i32 (i32.const 10))
  (global $ERR_NO_MEM   i32 (i32.const 12))
  (global $ERR_IDX      i32 (i32.const 13))
  (global $ERR_TRAP     i32 (i32.const 16))
  (global $ERR_ARITH    i32 (i32.const 17))
  (global $ERR_MISS_IMP i32 (i32.const 18))
  (global $ERR_MISS_EXP i32 (i32.const 19))
  (global $ERR_BAD_ARGUMENT i32 (i32.const 20))
  (global $ERR_RECUR    i32 (i32.const 31))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; LEB128 Decoding
  ;; ═════════════════════════════════════════════════════════════════════

  (func $leb_u32 (param $offset i32) (result i32)
    (local $r i32) (local $s i32) (local $b i32) (local $c i32) (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (local.set $c (local.get $offset))
    (block $end
      (loop $lp
        (if (i32.ge_u (local.get $c) (local.get $l)) (then (return (global.get $ERR_PARSE))))
        (local.set $b (i32.load8_u (i32.add (local.get $p) (local.get $c))))
        (local.set $c (i32.add (local.get $c) (i32.const 1)))
        (local.set $r (i32.or (local.get $r) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (local.get $s))))
        (local.set $s (i32.add (local.get $s) (i32.const 7)))
        (if (i32.gt_u (local.get $s) (i32.const 35)) (then (return (global.get $ERR_PARSE))))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80))) (then (br $end)))
        (br $lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $r))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $c) (local.get $offset)))
    (return (global.get $OK))
  )

  (func $leb_i32 (param $offset i32) (result i32)
    (local $r i32) (local $s i32) (local $b i32) (local $c i32) (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (local.set $c (local.get $offset))
    (block $end
      (loop $lp
        (if (i32.ge_u (local.get $c) (local.get $l)) (then (return (global.get $ERR_PARSE))))
        (local.set $b (i32.load8_u (i32.add (local.get $p) (local.get $c))))
        (local.set $c (i32.add (local.get $c) (i32.const 1)))
        (local.set $r (i32.or (local.get $r) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (local.get $s))))
        (local.set $s (i32.add (local.get $s) (i32.const 7)))
        (if (i32.gt_u (local.get $s) (i32.const 35)) (then (return (global.get $ERR_PARSE))))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80))) (then (br $end)))
        (br $lp)
      )
    )
    (if (i32.and (local.get $b) (i32.const 0x40))
      (then (local.set $r (i32.or (local.get $r) (i32.shl (i32.const -1) (local.get $s)))))
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $r))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $c) (local.get $offset)))
    (return (global.get $OK))
  )

  (func $leb_i64 (param $offset i32) (result i32)
    (local $r_lo i32) (local $r_hi i32) (local $s i32) (local $b i32) (local $c i32) (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (local.set $c (local.get $offset))
    (block $end
      (loop $lp
        (if (i32.ge_u (local.get $c) (local.get $l)) (then (return (global.get $ERR_PARSE))))
        (local.set $b (i32.load8_u (i32.add (local.get $p) (local.get $c))))
        (local.set $c (i32.add (local.get $c) (i32.const 1)))
        (if (i32.lt_s (local.get $s) (i32.const 32))
          (then
            (local.set $r_lo (i32.or (local.get $r_lo) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (local.get $s))))
          )
          (else
            (local.set $r_hi (i32.or (local.get $r_hi) (i32.shl (i32.and (local.get $b) (i32.const 0x7f)) (i32.sub (local.get $s) (i32.const 32)))))
          )
        )
        (local.set $s (i32.add (local.get $s) (i32.const 7)))
        (if (i32.gt_u (local.get $s) (i32.const 63)) (then (return (global.get $ERR_PARSE))))
        (if (i32.eqz (i32.and (local.get $b) (i32.const 0x80))) (then (br $end)))
        (br $lp)
      )
    )
    (if (i32.and (local.get $b) (i32.const 0x40))
      (then
        (if (i32.lt_s (local.get $s) (i32.const 32))
          (then (local.set $r_lo (i32.or (local.get $r_lo) (i32.shl (i32.const -1) (local.get $s)))))
          (else (local.set $r_hi (i32.or (local.get $r_hi) (i32.shl (i32.const -1) (i32.sub (local.get $s) (i32.const 32))))))
        )
      )
    )
    (i64.store (global.get $OFF_SCRATCH0)
      (i64.or (i64.extend_i32_u (local.get $r_lo)) (i64.shl (i64.extend_i32_u (local.get $r_hi)) (i64.const 32)))
    )
    (i32.store (global.get $OFF_SCRATCH2) (i32.sub (local.get $c) (local.get $offset)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; String / name reader (length-prefixed UTF-8 byte sequence)
  ;; stores name bytes to OFFSET_NAMES_BUF, returns (name_offset, bytes_consumed)
  ;; in scratch0/scratch1
  ;; ═════════════════════════════════════════════════════════════════════
  (func $read_name (param $offset i32) (result i32)
    (local $len i32) (local $adv i32) (local $dst i32) (local $p i32) (local $i i32)
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $len (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $adv (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $offset (i32.add (local.get $offset) (local.get $adv)))
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $dst (i32.load (global.get $OFF_NAMES_PTR)))
    (local.set $i (i32.const 0))
    (block $copy_end
      (loop $copy_lp
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $copy_end)))
        (i32.store8 (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $p) (local.get $offset) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy_lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $dst))
    (i32.store (global.get $OFF_SCRATCH1) (i32.add (local.get $adv) (local.get $len)))
    ;; Advance names buffer
    (i32.store (global.get $OFF_NAMES_PTR) (i32.add (local.get $dst) (local.get $len)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Section header reader
  ;; Output: scratch0=sec_id, scratch1=sec_len, scratch2=content_offset, scratch3=end_offset
  ;; ═════════════════════════════════════════════════════════════════════
  (func $read_section_header (param $offset i32) (result i32)
    (local $id i32) (local $p i32) (local $l i32) (local $leb_adv i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (if (i32.ge_u (local.get $offset) (local.get $l)) (then (return (global.get $ERR_PARSE))))

    ;; Read section ID (1 byte)
    (local.set $id (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

    ;; Read section size (LEB128) - result in scratch0, advance in scratch1
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $leb_adv (i32.load (global.get $OFF_SCRATCH1)))

    ;; Save section_size before overwriting scratch0
    (i32.store (global.get $OFF_SCRATCH2) (i32.load (global.get $OFF_SCRATCH0)))  ;; sec_size saved in scratch2 temporarily

    ;; Store results
    (i32.store (global.get $OFF_SCRATCH0) (local.get $id))                    ;; sec_id -> scratch0
    ;; scratch1 = sec_size (moved from scratch2 to scratch1)
    (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH2)))  ;; sec_size -> scratch1
    ;; scratch2 = content_offset = offset + leb_adv
    (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $offset) (local.get $leb_adv)))
    ;; scratch3 = end_offset = content_offset + sec_size
    (i32.store (global.get $OFF_SCRATCH3)
      (i32.add
        (i32.add (local.get $offset) (local.get $leb_adv))
        (i32.load (global.get $OFF_SCRATCH1))
      )
    )
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Module header: magic + version
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_header (result i32)
    (local $p i32) (local $l i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $l (i32.load (global.get $OFF_WASM_LEN)))
    (if (i32.lt_u (local.get $l) (i32.const 8)) (then (return (global.get $ERR_PARSE))))
    (if (i32.ne (i32.load (local.get $p)) (i32.const 0x6d736100)) (then (return (global.get $ERR_PARSE))))
    (if (i32.ne (i32.load (i32.add (local.get $p) (i32.const 4))) (i32.const 1)) (then (return (global.get $ERR_PARSE))))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse one type entry (functype)
  ;; Reads at offset, writes to scratch0 = new_offset after consuming type
  ;; Returns error
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_one_type (param $offset i32) (result i32)
    (local $tc i32) (local $rc i32) (local $base i32) (local $i i32) (local $wp i32)

    (local.set $tc (i32.load (global.get $OFF_TYPE_COUNT)))
    (if (i32.ge_u (local.get $tc) (global.get $MAX_TYPES)) (then (return (global.get $ERR_PARSE))))

    ;; Type entry starts with 0x60 (functype)
    (local.set $wp (i32.load (global.get $OFF_WASM_PTR)))
    (if (i32.ne (i32.load8_u (i32.add (local.get $wp) (local.get $offset))) (i32.const 0x60))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

    ;; Read param count
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $rc (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $rc) (i32.const 32)) (then (return (global.get $ERR_PARSE))))

    ;; Compute type entry base
    (local.set $base (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $tc) (global.get $SZ_TYPE))))

    ;; Write param types (up to 32 entries, each 1 byte)
    (local.set $i (i32.const 0))
    (block $pl
      (loop $pl_lp
        (if (i32.ge_u (local.get $i) (local.get $rc)) (then (br $pl)))
        (local.set $wp (i32.load (global.get $OFF_WASM_PTR)))
        (i32.store8 (i32.add (local.get $base) (local.get $i))
          (i32.load8_u (i32.add (local.get $wp) (local.get $offset)))
        )
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $pl_lp)
      )
    )

    ;; Write param_count (16-bit at offset 128 within type entry)
    (i32.store16 (i32.add (local.get $base) (i32.const 128)) (local.get $rc))

    ;; Read result count
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $rc (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $rc) (i32.const 4)) (then (return (global.get $ERR_PARSE))))

    ;; Write result types (start at offset 132 = 128 + 4)
    (local.set $i (i32.const 0))
    (block $rl
      (loop $rl_lp
        (if (i32.ge_u (local.get $i) (local.get $rc)) (then (br $rl)))
        (local.set $wp (i32.load (global.get $OFF_WASM_PTR)))
        (i32.store8 (i32.add (local.get $base) (i32.const 132) (local.get $i))
          (i32.load8_u (i32.add (local.get $wp) (local.get $offset)))
        )
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $rl_lp)
      )
    )

    ;; Write result_count (16-bit at offset 132 + 4 = 136)
    (i32.store16 (i32.add (local.get $base) (i32.const 136)) (local.get $rc))

    ;; Increment type count
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (i32.load (global.get $OFF_TYPE_COUNT)) (i32.const 1)))

    ;; Return new offset
    (i32.store (global.get $OFF_SCRATCH0) (local.get $offset))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse type section
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_type_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $err i32) (local $end i32)

    (local.set $end (i32.add (local.get $offset) (local.get $size)))

    ;; Read type count
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

    (block $lp
      (loop $continue
        (if (i32.eqz (local.get $count)) (then (br $lp)))
        (local.set $err (call $parse_one_type (local.get $offset)))
        (if (local.get $err) (then (return (local.get $err))))
        (local.set $offset (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $count (i32.sub (local.get $count) (i32.const 1)))
        (br $continue)
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse limits: reads either "min" or "min, max" pair
  ;; Input: offset, returns: scratch0=min, scratch1=max(-1=unset), scratch2=advance
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_limits (param $offset i32) (result i32)
    (local $flags i32) (local $min i32) (local $adv i32) (local $p i32)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))

    ;; Read flags byte: 0 = min only, 1 = min + max
    (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

    ;; Read min
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $min (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $adv (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $offset (i32.add (local.get $offset) (local.get $adv)))
    (i32.store (global.get $OFF_SCRATCH0) (local.get $min))

    (if (local.get $flags)
      (then
        ;; Read max
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
      )
      (else
        ;; max = -1 (unlimited)
        (i32.store (global.get $OFF_SCRATCH1) (i32.const -1))
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse function section (list of type indices)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_function_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $fc i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_FUNCTIONS)) (then (return (global.get $ERR_PARSE))))

    (local.set $i (i32.const 0))
    (local.set $fc (i32.load (global.get $OFF_FUNCTION_COUNT)))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        ;; Read type index
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        ;; Store to functions_buf[fc + i] = type_index (first 8 bytes of 16-byte entry)
        (i32.store
          (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (i32.add (local.get $fc) (local.get $i)) (global.get $SZ_FUNC)))
          (i32.load (global.get $OFF_SCRATCH0))
        )
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.add (local.get $fc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse import section
  ;; Each import: module_name(string) + field_name(string) + kind(1 byte) + kind_data
  ;; For function: kind_data = type_index(LEB128)
  ;; For table: kind_data = elem_type(1) + limits
  ;; For memory: kind_data = limits
  ;; For global: kind_data = val_type(1) + mutability(1)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_import_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32)
    (local $kind i32) (local $p i32) (local $end i32) (local $flags i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_IMPORTS)) (then (return (global.get $ERR_PARSE))))

    (local.set $end (i32.add (local.get $offset) (local.get $size)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))

        ;; Skip module name
        (if (call $read_name (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Skip field name
        (if (call $read_name (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Read import kind
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $kind (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        (if (i32.eq (local.get $kind) (global.get $EXT_FUNC))
          (then
            ;; Function import: skip type index
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
          )
        )
        (if (i32.eq (local.get $kind) (global.get $EXT_TABLE))
          (then
            ;; Table import: elem_type (1 byte) + limits
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            ;; flags (1 byte)
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            ;; min
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            ;; optional max
            (if (local.get $flags)
              (then
                (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
          )
        )
        (if (i32.eq (local.get $kind) (global.get $EXT_MEM))
          (then
            ;; Memory import: limits
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            (if (local.get $flags)
              (then
                (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
          )
        )
        (if (i32.eq (local.get $kind) (global.get $EXT_GLOBAL))
          (then
            ;; Global import: val_type (1 byte) + mutability (1 byte)
            (local.set $offset (i32.add (local.get $offset) (i32.const 2)))
          )
        )

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    ;; Store import count
    (i32.store (global.get $OFF_IMPORT_COUNT) (local.get $count))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse global section
  ;; Each global: val_type(1) + mutability(1) + init_expr(opcodes ending with end)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_global_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $gc i32) (local $end i32)
    (local $op i32) (local $p i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_GLOBALS)) (then (return (global.get $ERR_PARSE))))

    (local.set $end (i32.add (local.get $offset) (local.get $size)))
    (local.set $gc (i32.load (global.get $OFF_GLOBAL_COUNT)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))

        ;; Skip val_type (1 byte) + mutability (1 byte)
        (local.set $offset (i32.add (local.get $offset) (i32.const 2)))

        ;; Parse init expression: read opcodes until end (0x0B)
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (if (i32.eq (local.get $op) (i32.const 0x41))  ;; i32.const
          (then
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (i32.store
              (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (i32.add (local.get $gc) (local.get $i)) (i32.const 2)))
              (i32.load (global.get $OFF_SCRATCH0))
            )
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
          )
          (else
            ;; Unsupported init expr: skip to end (0x0B) silently, value stays 0
            (block $skip_expr
              (loop $skip_cont
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (i32.eq (local.get $op) (i32.const 0x0B)) (then (br $skip_expr)))
                (br $skip_cont)
              )
            )
            ;; Default to 0
            (i32.store
              (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (i32.add (local.get $gc) (local.get $i)) (i32.const 2)))
              (i32.const 0)
            )
          )
        )

        ;; Skip end
        (if (i32.ne (local.get $op) (i32.const 0x0B))
          (then
            ;; Skip remaining opcodes until end (0x0B)
            (block $skip_end
              (loop $skip_end_cont
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (i32.eq (local.get $op) (i32.const 0x0B)) (then (br $skip_end)))
                (br $skip_end_cont)
              )
            )
          )
        )

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_GLOBAL_COUNT) (i32.add (local.get $gc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse memory section (memory type + limits)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_memory_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.ne (local.get $count) (i32.const 1)) (then (return (global.get $ERR_UNSUP))))

    ;; Parse limits
    (if (call $parse_limits (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (i32.store (global.get $OFF_MEM_MIN) (i32.load (global.get $OFF_SCRATCH0)))
    (i32.store (global.get $OFF_MEM_MAX) (i32.load (global.get $OFF_SCRATCH1)))
    (i32.store (global.get $OFF_GUEST_MEM_PAGES) (i32.load (global.get $OFF_SCRATCH0)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse table section (elem_type + limits)
  ;; MVP: at most 1 table, elem_type must be 0x70 (funcref)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_table_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $elem_type i32) (local $flags i32)
    (local $min i32) (local $max i32) (local $p i32) (local $i i32)

    (i32.store (global.get $OFF_DBG) (i32.const 50))  ;; dbg: entered parse_table_section

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (i32.store (global.get $OFF_DBG) (i32.const 51))  ;; dbg: after leb count
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (i32.const 1)) (then (return (global.get $ERR_UNSUP))))
    (if (i32.eqz (local.get $count)) (then (return (global.get $OK))))

    (i32.store (global.get $OFF_DBG) (i32.const 52))  ;; dbg: count is 1, reading elem_type

    ;; elem_type (1 byte) must be 0x70 (funcref)
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $elem_type (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
    (if (i32.ne (local.get $elem_type) (i32.const 0x70)) (then (return (global.get $ERR_UNSUP))))

    (i32.store (global.get $OFF_DBG) (i32.const 53))  ;; dbg: elem_type is 0x70

    ;; Parse limits: flags (1 byte) + min (LEB128) + [max (LEB128)]
    (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $flags (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $min (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (local.set $max (i32.const 0))
    (if (i32.and (local.get $flags) (i32.const 1))
      (then
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $max (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
      )
    )
    (i32.store (global.get $OFF_DBG) (i32.const 54))  ;; dbg: limits read, checking max
    (if (i32.gt_u (local.get $min) (global.get $MAX_TABLE)) (then (return (global.get $ERR_PARSE))))
    (if (i32.and (local.get $flags) (i32.const 1))
      (then (if (i32.gt_u (local.get $max) (global.get $MAX_TABLE)) (then (return (global.get $ERR_PARSE)))))
    )

    ;; Store table info
    (i32.store8 (global.get $OFF_TABLE_HAS) (i32.const 1))
    (i32.store (global.get $OFF_TABLE_MIN) (local.get $min))
    (i32.store (global.get $OFF_TABLE_MAX) (local.get $max))

    ;; Initialize all table entries to -1 (null funcref)
    (local.set $i (i32.const 0))
    (block $init_lp
      (loop $init_cont
        (if (i32.ge_u (local.get $i) (local.get $min)) (then (br $init_lp)))
        (i32.store
          (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (local.get $i) (i32.const 2)))
          (i32.const -1)
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $init_cont)
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse export section (list of exports)
  ;; Each export: name(string) + kind(1 byte) + index(LEB128)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_export_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $ec i32) (local $base i32)
    (local $name_off i32) (local $name_len i32) (local $kind i32) (local $idx i32) (local $p i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_EXPORTS)) (then (return (global.get $ERR_PARSE))))

    (local.set $ec (i32.load (global.get $OFF_EXPORT_COUNT)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))

        (local.set $base (i32.add (global.get $OFF_EXPORTS_BUF) (i32.mul (i32.add (local.get $ec) (local.get $i)) (global.get $SZ_EXPORT))))

        ;; Read name string
        (if (call $read_name (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $name_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $name_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $offset (i32.add (local.get $offset) (local.get $name_len)))

        ;; Read kind (1 byte)
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $kind (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        ;; Read index (LEB128)
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $idx (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Store to export entry
        (i32.store (local.get $base) (local.get $name_off))           ;; name_ptr
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $name_len))  ;; name_len
        (i32.store8 (i32.add (local.get $base) (i32.const 16)) (local.get $kind))    ;; kind
        (i32.store (i32.add (local.get $base) (i32.const 24)) (local.get $idx))      ;; index

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.add (local.get $ec) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse start section (single function index)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_start_section (param $offset i32) (param $size i32) (result i32)
    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (i32.store (global.get $OFF_START_FUNC) (i32.load (global.get $OFF_SCRATCH0)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse code section (function bodies)
  ;; Each body: size(LEB128) + local_count(LEB128) + (locals: count + type)* + bytecode
  ;; We store: body_offset, body_len, local_count
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_code_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $cc i32) (local $base i32)
    (local $body_size i32) (local $body_end i32)
    (local $local_count i32) (local $lc i32)
    (local $num_locals i32) (local $type i32) (local $j i32)
    (local $p i32)

    (i32.store (global.get $OFF_DBG) (i32.const 90))  ;; dbg: entered parse_code_section
    (local.set $cc (i32.load (global.get $OFF_CODE_COUNT)))

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_FUNCTIONS)) (then (return (global.get $ERR_PARSE))))

    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))

        ;; Read body size
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $body_size (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; body_end = offset + body_size (offset is after the size LEB, body_size is the content size)
        (local.set $body_end (i32.add (local.get $offset) (local.get $body_size)))

        ;; Store body_offset (relative to wasm_base), body_len
        (local.set $base (i32.add (global.get $OFF_CODE_BUF) (i32.mul (i32.add (local.get $cc) (local.get $i)) (global.get $SZ_CODE))))
        (i32.store (i32.add (local.get $base) (i32.const 0)) (local.get $offset))     ;; body_offset
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $body_size))  ;; body_len

        ;; Read local count
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $local_count (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Parse local declarations
        (local.set $lc (i32.const 0))
        (local.set $j (i32.const 0))
        (block $llp
          (loop $llcont
            (if (i32.ge_u (local.get $j) (local.get $local_count)) (then (br $llp)))
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $num_locals (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (local.set $type (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
            (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
            (local.set $lc (i32.add (local.get $lc) (local.get $num_locals)))
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $llcont)
          )
        )
        (i32.store (i32.add (local.get $base) (i32.const 16)) (local.get $lc))  ;; local_count

        ;; Decode opcodes
        (i32.store (global.get $OFF_DBG) (i32.const 80))  ;; dbg: before decode
        (if (call $decode_opcodes (local.get $offset) (i32.sub (local.get $body_end) (local.get $offset)))
          (then (i32.store (global.get $OFF_DBG) (i32.const 81)) (return (global.get $ERR_PARSE)))  ;; dbg: decode fail
        )
        (i32.store (global.get $OFF_DBG) (i32.const 82))  ;; dbg: after decode
        ;; Compute end targets for structured control flow
        (if (call $compute_end_targets
              (i32.load (global.get $OFF_SCRATCH0))
              (i32.load (global.get $OFF_SCRATCH1))
            )
          (then (i32.store (global.get $OFF_DBG) (i32.const 83)) (return (global.get $ERR_PARSE)))  ;; dbg: compute fail
        )
        (i32.store (global.get $OFF_DBG) (i32.const 84))  ;; dbg: after compute
        ;; Store decoded_start (scratch0) and decoded_count (scratch1) in code entry
        (i32.store (i32.add (local.get $base) (i32.const 24)) (i32.load (global.get $OFF_SCRATCH0)))  ;; decoded_start
        (i32.store (i32.add (local.get $base) (i32.const 32)) (i32.load (global.get $OFF_SCRATCH1)))  ;; decoded_count

        ;; Skip to body_end
        (local.set $offset (local.get $body_end))

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_CODE_COUNT) (i32.add (local.get $cc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse data section (list of data segments)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_data_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $dc i32) (local $base i32)
    (local $mode i32) (local $seg_size i32) (local $p i32) (local $j i32)
    (local $dest_addr i32) (local $wasm_base i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_DATAS)) (then (return (global.get $ERR_PARSE))))

    (local.set $dc (i32.load (global.get $OFF_DATA_COUNT)))
    (local.set $wasm_base (i32.load (global.get $OFF_WASM_PTR)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (local.set $base (i32.add (global.get $OFF_DATA_BUF) (i32.mul (i32.add (local.get $dc) (local.get $i)) (global.get $SZ_DATA))))

        ;; Read mode (0=active, 1=passive, 2=active with memory index)
        (local.set $mode (i32.load8_u (i32.add (local.get $wasm_base) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        (local.set $dest_addr (i32.const 0))
        (if (i32.or (i32.eq (local.get $mode) (i32.const 2)) (i32.eq (local.get $mode) (i32.const 0)))
          (then
            ;; Active: parse i32.const <value> 0x0B
            (if (i32.eq (i32.load8_u (i32.add (local.get $wasm_base) (local.get $offset))) (i32.const 0x41))
              (then
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $dest_addr (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
            ;; Skip remaining init expression until 0x0B
            (block $ie
              (loop $iel
                (if (i32.eq (i32.load8_u (i32.add (local.get $wasm_base) (local.get $offset))) (i32.const 0x0B))
                  (then
                    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                    (br $ie)
                  )
                )
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (br $iel)
              )
            )
          )
        )

        ;; Read segment data size
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $seg_size (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Store data segment info
        (i32.store (local.get $base) (local.get $offset))      ;; data_offset (in guest wasm)
        (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $dest_addr))  ;; dest_addr (in guest memory)
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $seg_size))  ;; data_len

        ;; Copy data bytes to guest memory (only for active segments)
        (if (i32.eq (local.get $mode) (i32.const 1))
          (then)  ;; passive segment: skip copy to memory
          (else
            (if (i32.and (i32.eqz (local.get $dest_addr)) (i32.eqz (local.get $seg_size)))
              (then)
              (else
            ;; Copy seg_size bytes from wasm_base+data_offset to guest_mem_base+dest_addr
            (local.set $p (i32.const 0))
            (block $copy_lp
              (loop $copy_cont
                (if (i32.ge_u (local.get $p) (local.get $seg_size)) (then (br $copy_lp)))
                (i32.store8
                  (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $dest_addr) (local.get $p)))
                  (i32.load8_u (i32.add (local.get $wasm_base) (i32.add (local.get $offset) (local.get $p))))
                )
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (br $copy_cont)
              )
            )
            )
          )
        )
      )

        ;; Skip data bytes (advance offset past the data)
        (local.set $offset (i32.add (local.get $offset) (local.get $seg_size)))

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_DATA_COUNT) (i32.add (local.get $dc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse element section (segment data for table initialization)
  ;; Each element: mode + [table_idx] + offset_expr + count + func_indices
  ;; ═════════════════════════════════════════════════════════════════════
  (func $parse_element_section (param $offset i32) (param $size i32) (result i32)
    (local $count i32) (local $i i32) (local $mode i32) (local $table_idx i32)
    (local $dest_offset i32) (local $elem_count i32) (local $j i32)
    (local $func_idx i32) (local $p i32) (local $end i32)
    (local $elem_base i32) (local $dc i32)

    (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
    (local.set $count (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
    (if (i32.gt_u (local.get $count) (global.get $MAX_ELEMS)) (then (return (global.get $ERR_PARSE))))

    (local.set $end (i32.add (local.get $offset) (local.get $size)))
    (local.set $dc (i32.load (global.get $OFF_ELEM_COUNT)))
    (local.set $i (i32.const 0))
    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $lp)))
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))

        (local.set $elem_base (i32.add (global.get $OFF_ELEM_BUF) (i32.mul (i32.add (local.get $dc) (local.get $i)) (global.get $SZ_ELEM))))

        ;; Read mode
        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $mode (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        (local.set $table_idx (i32.const 0))
        (local.set $dest_offset (i32.const 0))

        (if (i32.eq (local.get $mode) (i32.const 0))
          (then
            ;; Mode 0: active, implicit table 0. Parse init_expr (i32.const <value> 0x0B)
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x41))
              (then
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $dest_offset (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
            ;; Skip to end (0x0B)
            (block $ie
              (loop $iel
                (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x0B))
                  (then
                    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                    (br $ie)
                  )
                )
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (br $iel)
              )
            )
          )
        )
        (if (i32.eq (local.get $mode) (i32.const 1))
          (then
            ;; Mode 1: passive segment - stored for table.init
          )
        )
        (if (i32.eq (local.get $mode) (i32.const 2))
          (then
            ;; Mode 2: active with explicit table index
            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $table_idx (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
            (if (i32.ne (local.get $table_idx) (i32.const 0)) (then (return (global.get $ERR_UNSUP))))

            ;; Parse init_expr
            (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
            (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x41))
              (then
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                (local.set $dest_offset (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              )
            )
            (block $ie2
              (loop $iel2
                (if (i32.ge_u (local.get $offset) (local.get $end)) (then (return (global.get $ERR_PARSE))))
                (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                (if (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $offset))) (i32.const 0x0B))
                  (then
                    (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                    (br $ie2)
                  )
                )
                (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                (br $iel2)
              )
            )
          )
        )

        ;; Store mode to element buffer (0=dropped flag, then mode, dest_offset)
        (i32.store (local.get $elem_base) (i32.const 0))           ;; dropped = 0 (active)
        (i32.store (i32.add (local.get $elem_base) (i32.const 4)) (local.get $dest_offset))  ;; dest_offset

        ;; Read element count
        (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
        (local.set $elem_count (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

        ;; Store element count
        (i32.store (i32.add (local.get $elem_base) (i32.const 8)) (local.get $elem_count))

        ;; Read and store each func idx in the table and element buffer
        (local.set $j (i32.const 0))
        (block $elem_lp
          (loop $elem_cont
            (if (i32.ge_u (local.get $j) (local.get $elem_count)) (then (br $elem_lp)))

            (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
            (local.set $func_idx (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))

            ;; Store func_idx in element buffer for table.init
            (i32.store
              (i32.add (local.get $elem_base) (i32.add (i32.const 12) (i32.shl (local.get $j) (i32.const 2))))
              (local.get $func_idx)
            )

            ;; For active segments (mode 0/2), also write to table immediately
            (if (i32.le_u (local.get $mode) (i32.const 2))
              (then
                (if (i32.ne (local.get $mode) (i32.const 1))
                  (then
                    (i32.store
                      (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $dest_offset) (local.get $j)) (i32.const 2)))
                      (local.get $func_idx)
                    )
                  )
                )
              )
            )

            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $elem_cont)
          )
        )

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cont)
      )
    )
    (i32.store (global.get $OFF_ELEM_COUNT) (i32.add (local.get $dc) (local.get $count)))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Decode opcodes: translate raw bytecode to DecodedOp entries
  ;; Input: offset (in wasm bytes), len (bytecode length)
  ;; Output: decoded_start (index into decoded_ops cache), decoded_count
  ;; Stored to scratch0/scratch1
  ;; ═════════════════════════════════════════════════════════════════════
  (func $decode_opcodes (export "decode_opcodes") (param $offset i32) (param $len i32) (result i32)
    (local $end i32) (local $start_idx i32) (local $dc i32)
    (local $op i32) (local $p i32) (local $imm0 i32) (local $imm1 i32)
    (local $adv i32) (local $base i32)

    (local.set $end (i32.add (local.get $offset) (local.get $len)))
    (local.set $start_idx (i32.load (global.get $OFF_DECODED_COUNT)))  ;; running decoded op count
    (local.set $dc (local.get $start_idx))

    (block $lp
      (loop $cont
        (if (i32.ge_u (local.get $offset) (local.get $end)) (then (br $lp)))

        (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
        (local.set $op (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
        (local.set $offset (i32.add (local.get $offset) (i32.const 1)))

        ;; Compute decoded op base in cache
        (local.set $base (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (local.get $dc) (global.get $DEC_SZ))))
        ;; Store opcode
        (i32.store8 (local.get $base) (local.get $op))

        ;; Decode immediates based on opcode
        (block $op_handled
          ;; ── No immediate ops ──
          (if (i32.le_u (local.get $op) (i32.const 0x01))   ;; unreachable, nop
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x0B))     ;; end
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x0F))     ;; return
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x1A))     ;; drop
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x45))     ;; i32.eqz
            (then (br $op_handled))
          )
          ;; All comparison, arithmetic, conversion ops (0x46-0xC4 excl block/loop/if)
          (if (i32.and (i32.ge_u (local.get $op) (i32.const 0x46)) (i32.le_u (local.get $op) (i32.const 0xC4)))
            (then
              (if (i32.or (i32.eq (local.get $op) (i32.const 0x02)) (i32.eq (local.get $op) (i32.const 0x03)))
                (then)  ;; block/loop handled below
                (else (br $op_handled))
              )
            )
          )

          ;; ── LEB128 immediate ops ──
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x0C)) (i32.eq (local.get $op) (i32.const 0x0D))) ;; br, br_if
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x10))     ;; call
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x11))     ;; call_indirect
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.and (i32.ge_u (local.get $op) (i32.const 0x20)) (i32.le_u (local.get $op) (i32.const 0x26))) ;; local.get/set/tee/global.get/set, table.get/set
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x41))     ;; i32.const (signed LEB128)
            (then
              (if (call $leb_i32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x42))     ;; i64.const (signed LEB128)
            (then
              (if (call $leb_i64 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i64.store (i32.add (local.get $base) (i32.const 4)) (i64.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH2))))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x43))     ;; f32.const
            (then
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (i32.store (i32.add (local.get $base) (i32.const 4))
                (i32.load (i32.add (local.get $p) (local.get $offset)))
              )
              (local.set $offset (i32.add (local.get $offset) (i32.const 4)))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x44))     ;; f64.const
            (then
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (i64.store (i32.add (local.get $base) (i32.const 4))
                (i64.load (i32.add (local.get $p) (local.get $offset)))
              )
              (local.set $offset (i32.add (local.get $offset) (i32.const 8)))
              (br $op_handled)
            )
          )

          ;; ── Memory ops (load/store): align + offset ──
          (if (i32.and (i32.ge_u (local.get $op) (i32.const 0x28)) (i32.le_u (local.get $op) (i32.const 0x3E)))
            (then
              ;; align (LEB128)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              ;; offset (LEB128)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))  ;; align
              (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $imm1))  ;; mem offset
              (br $op_handled)
            )
          )

          ;; ── Block/loop/if: block type ──
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x02)) (i32.eq (local.get $op) (i32.const 0x03)))
            (then
              ;; Block type: either empty(0x40), a value type byte, or a signed LEB128 type index
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (local.set $imm0 (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0x04))     ;; if
            (then
              ;; Same as block
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (local.set $imm0 (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))
              (br $op_handled)
            )
          )

          ;; ── br_table: count + labels + default ──
          (if (i32.eq (local.get $op) (i32.const 0x0E))
            (then
              ;; Read label count
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              ;; Skip labels + default
              (local.set $imm0 (i32.load (i32.add (local.get $base) (i32.const 4))))
              (local.set $imm1 (i32.const 0))
              (block $bt_lp
                (loop $bt_cont
                  (if (i32.ge_u (local.get $imm1) (local.get $imm0)) (then (br $bt_lp)))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (local.set $imm1 (i32.add (local.get $imm1) (i32.const 1)))
                  (br $bt_cont)
                )
              )
              ;; Read default label and store as imm1 (at base+8)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )

          ;; ── else (0x05) ──
          (if (i32.eq (local.get $op) (i32.const 0x05))
            (then (br $op_handled))
          )

          ;; ── select (0x1B, 0x1C) ──
          (if (i32.eq (local.get $op) (i32.const 0x1B))
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0x1C))
            (then
              ;; typed select: skip result types
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )

          ;; ── memory.size (0x3F), memory.grow (0x40): 1 byte immediate (0x00) ──
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x3F)) (i32.eq (local.get $op) (i32.const 0x40)))
            (then
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))  ;; skip 0x00 byte
              (br $op_handled)
            )
          )

          ;; ── ref.null (0xD0): 1 byte immediate (reftype) ──
          (if (i32.eq (local.get $op) (i32.const 0xD0))
            (then
              (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
              (i32.store (i32.add (local.get $base) (i32.const 4))
                (i32.load8_u (i32.add (local.get $p) (local.get $offset)))
              )
              (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
              (br $op_handled)
            )
          )
          (if (i32.eq (local.get $op) (i32.const 0xD1))     ;; ref.is_null
            (then (br $op_handled))
          )
          (if (i32.eq (local.get $op) (i32.const 0xD2))     ;; ref.func
            (then
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (br $op_handled)
            )
          )

          ;; ── Extended prefix (0xFC) ──
          (if (i32.eq (local.get $op) (i32.const 0xFC))
            (then
              ;; Read sub-opcode
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              ;; Consume reserved immediates based on sub-opcode
              (local.set $imm0 (i32.load (i32.add (local.get $base) (i32.const 4))))
              (if (i32.eq (local.get $imm0) (i32.const 0x0A))     ;; memory.copy: 2 reserved bytes
                (then (local.set $offset (i32.add (local.get $offset) (i32.const 2))))
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0B))     ;; memory.fill: 1 reserved byte
                (then (local.set $offset (i32.add (local.get $offset) (i32.const 1))))
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x08))     ;; memory.init: data_seg idx + 1 reserved byte
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x09))     ;; data.drop: data_seg idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0C))     ;; table.init: elem_idx + table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 12)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0D))     ;; elem.drop: elem_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0E))     ;; table.copy: dst + src table idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 12)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0F))     ;; table.grow: table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x10))     ;; table.size: table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x11))     ;; table.fill: table_idx
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                )
              )
              (br $op_handled)
            )
          )

          ;; ── SIMD prefix (0xFD) ──
          (if (i32.eq (local.get $op) (i32.const 0xFD))
            (then
              ;; Read sub-opcode (LEB128 u32)
              (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
              (i32.store (i32.add (local.get $base) (i32.const 4)) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
              (local.set $imm0 (i32.load (i32.add (local.get $base) (i32.const 4))))

              ;; Memory ops (raw sub-opcodes 0x00-0x1F except 0x0C): align + offset
              ;; Uses raw $imm0 BEFORE translation to capture both old and modern encodings
              (if (i32.and (i32.le_u (local.get $imm0) (i32.const 0x1F))
                           (i32.ne (local.get $imm0) (i32.const 0x0C)))
                (then
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (if (call $leb_u32 (local.get $offset)) (then (return (global.get $ERR_PARSE))))
                  (i32.store (i32.add (local.get $base) (i32.const 8)) (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $offset (i32.add (local.get $offset) (i32.load (global.get $OFF_SCRATCH1))))
                  (i32.store (i32.add (local.get $base) (i32.const 12)) (local.get $imm1))  ;; align
                )
              )

              ;; v128.const (sub-opcode 0x0C): read 16 bytes into base+16
              (if (i32.eq (local.get $imm0) (i32.const 0x0C))
                (then
                  (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                  (i64.store (i32.add (local.get $base) (i32.const 16))
                    (i64.load (i32.add (local.get $p) (local.get $offset))))
                  (i64.store (i32.add (local.get $base) (i32.const 24))
                    (i64.load (i32.add (local.get $p) (i32.add (local.get $offset) (i32.const 8)))))
                  (local.set $offset (i32.add (local.get $offset) (i32.const 16)))
                )
              )

              ;; ── Canonicalize sub-opcode (modern wabt → old encoding) ──
              ;; Only applied when $SIMD_MODERN_ENCODING is set to 1 (default 0).
              ;; Old-encoding input passes through unchanged.
              (if (global.get $SIMD_MODERN_ENCODING)
                (then
                  ;; Memory ops (modern store at 0x0B → canonical 0x1B)
                  (if (i32.eq (local.get $imm0) (i32.const 0x0B)) (then (local.set $imm0 (i32.const 0x1B))))
                  ;; Load-splat extends (modern only)
                  (if (i32.eq (local.get $imm0) (i32.const 0x07)) (then (local.set $imm0 (i32.const 0xA6))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x08)) (then (local.set $imm0 (i32.const 0xA7))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x09)) (then (local.set $imm0 (i32.const 0xA8))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x0A)) (then (local.set $imm0 (i32.const 0xA9))))
                  ;; Swizzle (modern only)
                  (if (i32.eq (local.get $imm0) (i32.const 0x0E)) (then (local.set $imm0 (i32.const 0xAA))))
                  ;; Splats
                  (if (i32.eq (local.get $imm0) (i32.const 0x0F)) (then (local.set $imm0 (i32.const 0x2D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x10)) (then (local.set $imm0 (i32.const 0x31))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x11)) (then (local.set $imm0 (i32.const 0x35))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x12)) (then (local.set $imm0 (i32.const 0x39))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x13)) (then (local.set $imm0 (i32.const 0x3A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x14)) (then (local.set $imm0 (i32.const 0x3D))))
                  ;; Extract/replace lanes
                  (if (i32.eq (local.get $imm0) (i32.const 0x15)) (then (local.set $imm0 (i32.const 0x2E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x16)) (then (local.set $imm0 (i32.const 0x2F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x17)) (then (local.set $imm0 (i32.const 0x30))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x18)) (then (local.set $imm0 (i32.const 0x32))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x19)) (then (local.set $imm0 (i32.const 0x33))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1A)) (then (local.set $imm0 (i32.const 0x34))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1B)) (then (local.set $imm0 (i32.const 0x36))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1C)) (then (local.set $imm0 (i32.const 0x37))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1D)) (then (local.set $imm0 (i32.const 0x38))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1E)) (then (local.set $imm0 (i32.const 0xAB))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x1F)) (then (local.set $imm0 (i32.const 0x3B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x20)) (then (local.set $imm0 (i32.const 0x3C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x21)) (then (local.set $imm0 (i32.const 0x3E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x22)) (then (local.set $imm0 (i32.const 0x3F))))
                  ;; Integer comparisons (modern 0x23-0x40 → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x23)) (then (local.set $imm0 (i32.const 0x47))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x24)) (then (local.set $imm0 (i32.const 0x48))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x25)) (then (local.set $imm0 (i32.const 0x4B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x26)) (then (local.set $imm0 (i32.const 0x4A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x27)) (then (local.set $imm0 (i32.const 0x4D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x28)) (then (local.set $imm0 (i32.const 0x4C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x29)) (then (local.set $imm0 (i32.const 0x4F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2A)) (then (local.set $imm0 (i32.const 0x4E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2B)) (then (local.set $imm0 (i32.const 0x49))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2C)) (then (local.set $imm0 (i32.const 0xAE))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2D)) (then (local.set $imm0 (i32.const 0x58))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2E)) (then (local.set $imm0 (i32.const 0x59))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x2F)) (then (local.set $imm0 (i32.const 0x5C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x30)) (then (local.set $imm0 (i32.const 0x5B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x31)) (then (local.set $imm0 (i32.const 0x5E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x32)) (then (local.set $imm0 (i32.const 0x5D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x33)) (then (local.set $imm0 (i32.const 0x60))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x34)) (then (local.set $imm0 (i32.const 0x5F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x35)) (then (local.set $imm0 (i32.const 0x5A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x36)) (then (local.set $imm0 (i32.const 0xAF))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x37)) (then (local.set $imm0 (i32.const 0x69))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x38)) (then (local.set $imm0 (i32.const 0x6A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x39)) (then (local.set $imm0 (i32.const 0x6D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3A)) (then (local.set $imm0 (i32.const 0x6C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3B)) (then (local.set $imm0 (i32.const 0x6F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3C)) (then (local.set $imm0 (i32.const 0x6E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3D)) (then (local.set $imm0 (i32.const 0x71))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3E)) (then (local.set $imm0 (i32.const 0x70))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x3F)) (then (local.set $imm0 (i32.const 0x6B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x40)) (then (local.set $imm0 (i32.const 0xB0))))
                  ;; Float comparisons (modern 0x41-0x4C → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x41)) (then (local.set $imm0 (i32.const 0x8C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x42)) (then (local.set $imm0 (i32.const 0x8B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x43)) (then (local.set $imm0 (i32.const 0x8E))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x44)) (then (local.set $imm0 (i32.const 0x90))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x45)) (then (local.set $imm0 (i32.const 0x92))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x46)) (then (local.set $imm0 (i32.const 0x94))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x47)) (then (local.set $imm0 (i32.const 0x9D))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x48)) (then (local.set $imm0 (i32.const 0x9C))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x49)) (then (local.set $imm0 (i32.const 0x9F))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4A)) (then (local.set $imm0 (i32.const 0xA1))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4B)) (then (local.set $imm0 (i32.const 0xA3))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4C)) (then (local.set $imm0 (i32.const 0xA5))))
                  ;; v128 bitwise (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x4D)) (then (local.set $imm0 (i32.const 0xAC))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x4E)) (then (local.set $imm0 (i32.const 0x43))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x50)) (then (local.set $imm0 (i32.const 0x44))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x51)) (then (local.set $imm0 (i32.const 0x45))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x52)) (then (local.set $imm0 (i32.const 0xAD))))
                  ;; i8x16 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x61)) (then (local.set $imm0 (i32.const 0x46))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x6E)) (then (local.set $imm0 (i32.const 0x40))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x71)) (then (local.set $imm0 (i32.const 0x41))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x77)) (then (local.set $imm0 (i32.const 0xB8))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x79)) (then (local.set $imm0 (i32.const 0xBA))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x7B)) (then (local.set $imm0 (i32.const 0xBB))))
                  ;; i16x8 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0x81)) (then (local.set $imm0 (i32.const 0x57))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x8E)) (then (local.set $imm0 (i32.const 0x51))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x91)) (then (local.set $imm0 (i32.const 0x52))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x95)) (then (local.set $imm0 (i32.const 0x61))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x97)) (then (local.set $imm0 (i32.const 0xBC))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x99)) (then (local.set $imm0 (i32.const 0xBE))))
                  (if (i32.eq (local.get $imm0) (i32.const 0x9B)) (then (local.set $imm0 (i32.const 0xBF))))
                  ;; i32x4 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xA1)) (then (local.set $imm0 (i32.const 0x68))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xAE)) (then (local.set $imm0 (i32.const 0x62))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xB1)) (then (local.set $imm0 (i32.const 0x63))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xB5)) (then (local.set $imm0 (i32.const 0x72))))
                  ;; i64x2 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xC1)) (then (local.set $imm0 (i32.const 0x79))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xCE)) (then (local.set $imm0 (i32.const 0x73))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xD1)) (then (local.set $imm0 (i32.const 0x74))))
                  ;; f32x4 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xE0)) (then (local.set $imm0 (i32.const 0x88))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE1)) (then (local.set $imm0 (i32.const 0x8A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE4)) (then (local.set $imm0 (i32.const 0x84))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE5)) (then (local.set $imm0 (i32.const 0x85))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE6)) (then (local.set $imm0 (i32.const 0x87))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE7)) (then (local.set $imm0 (i32.const 0x89))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE8)) (then (local.set $imm0 (i32.const 0xE1))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xE9)) (then (local.set $imm0 (i32.const 0xE2))))
                  ;; f64x2 ops (modern → canonical)
                  (if (i32.eq (local.get $imm0) (i32.const 0xEC)) (then (local.set $imm0 (i32.const 0x9A))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xED)) (then (local.set $imm0 (i32.const 0x9B))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF0)) (then (local.set $imm0 (i32.const 0x95))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF1)) (then (local.set $imm0 (i32.const 0x96))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF2)) (then (local.set $imm0 (i32.const 0x98))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF3)) (then (local.set $imm0 (i32.const 0x99))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF4)) (then (local.set $imm0 (i32.const 0xE3))))
                  (if (i32.eq (local.get $imm0) (i32.const 0xF5)) (then (local.set $imm0 (i32.const 0xE4))))
                )
              )

              ;; Store canonical sub-opcode (translated or pass-through)
              (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $imm0))

              ;; Lane index ops (extract_lane, replace_lane only): 1-byte lane index
              ;; Note: splat ops (0x2D, 0x31, 0x35, 0x39, 0x3A, 0x3D) do NOT have a lane index
              ;; Check specific extract/replace ranges: [0x2E-0x30], [0x32-0x34], [0x36-0x38], [0x3B-0x3C], [0x3E-0x3F]
              (local.set $adv (i32.const 0))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x2E)) (i32.le_u (local.get $imm0) (i32.const 0x30)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x32)) (i32.le_u (local.get $imm0) (i32.const 0x34)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x36)) (i32.le_u (local.get $imm0) (i32.const 0x38)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x3B)) (i32.le_u (local.get $imm0) (i32.const 0x3C)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.and (i32.ge_u (local.get $imm0) (i32.const 0x3E)) (i32.le_u (local.get $imm0) (i32.const 0x3F)))
                (then (local.set $adv (i32.const 1))))
              (if (i32.eq (local.get $imm0) (i32.const 0xAB))
                (then (local.set $adv (i32.const 1))))  ;; i64x2.replace_lane (new canonical)
              (if (local.get $adv)
                (then
                  (local.set $p (i32.load (global.get $OFF_WASM_PTR)))
                  (i32.store (i32.add (local.get $base) (i32.const 8))
                    (i32.load8_u (i32.add (local.get $p) (local.get $offset))))
                  (local.set $offset (i32.add (local.get $offset) (i32.const 1)))
                )
              )

              ;; All other 0xFD ops: no extra immediate
              (br $op_handled)
            )
          )

          ;; Unhandled opcode
          (return (global.get $ERR_UNSUP))
        )

        ;; Store decoded op marker
        (local.set $dc (i32.add (local.get $dc) (i32.const 1)))
        (br $cont)
      )
    )

    ;; Store start index and count
    (i32.store (global.get $OFF_SCRATCH0) (local.get $start_idx))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $dc) (local.get $start_idx)))
    (i32.store (global.get $OFF_DECODED_COUNT) (local.get $dc))

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Post-decode pass: compute next_idx for block/loop/if/else
  ;; Scans decoded ops for a function and sets next_idx to point to
  ;; the matching end+1 (for block/if/else) or loop start (for loop)
  ;; Input: start_idx, count (decoded op indices)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $compute_end_targets (export "compute_end_targets") (param $start i32) (param $count i32) (result i32)
    (local $pos i32) (local $op_base i32) (local $op i32)
    (local $sp i32) (local $label_base i32) (local $label_pos i32)

    (i32.store (global.get $OFF_LABEL_STACK_PTR) (i32.const 0))
    (local.set $label_base (global.get $OFF_LABEL_STACK))

    (local.set $pos (i32.const 0))
    (block $lp
      (loop $cont
        (local.set $op_base
          (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (i32.add (local.get $start) (local.get $pos)) (global.get $DEC_SZ)))
        )
        (local.set $op (i32.load8_u (local.get $op_base)))

        (block $skip
          ;; block (0x02), loop (0x03), if (0x04): push current position
          (if (i32.or (i32.eq (local.get $op) (i32.const 0x02))
                      (i32.or (i32.eq (local.get $op) (i32.const 0x03))
                              (i32.eq (local.get $op) (i32.const 0x04))))
            (then
              (local.set $sp (i32.load (global.get $OFF_LABEL_STACK_PTR)))
              (i32.store
                (i32.add (local.get $label_base) (i32.shl (local.get $sp) (i32.const 2)))
                (local.get $pos)
              )
              (i32.store (global.get $OFF_LABEL_STACK_PTR) (i32.add (local.get $sp) (i32.const 1)))
              (br $skip)
            )
          )

          ;; else (0x05): pop top (should be if), set its next_idx, push else
          (if (i32.eq (local.get $op) (i32.const 0x05))
            (then
              (local.set $sp (i32.load (global.get $OFF_LABEL_STACK_PTR)))
              (if (i32.eqz (local.get $sp))
                (then (br $skip))  ;; skip if stack empty (malformed but safe)
              )
              (local.set $sp (i32.sub (local.get $sp) (i32.const 1)))
              (i32.store (global.get $OFF_LABEL_STACK_PTR) (local.get $sp))
              (local.set $label_pos (i32.load (i32.add (local.get $label_base) (i32.shl (local.get $sp) (i32.const 2)))))
              ;; Set next_idx of the matching if to current pos + 1 (skip to else/end)
              (i32.store
                (i32.add (global.get $OFF_DECODED_OPS)
                  (i32.add (i32.mul (local.get $label_pos) (global.get $DEC_SZ)) (i32.const 12))
                )
                (i32.add (local.get $pos) (i32.const 1))
              )
              ;; Push current position as else label
              (i32.store
                (i32.add (local.get $label_base) (i32.shl (local.get $sp) (i32.const 2)))
                (local.get $pos)
              )
              (i32.store (global.get $OFF_LABEL_STACK_PTR) (i32.add (local.get $sp) (i32.const 1)))
              (br $skip)
            )
          )

          ;; end (0x0B): pop top, set its next_idx
          (if (i32.eq (local.get $op) (i32.const 0x0B))
            (then
              (local.set $sp (i32.load (global.get $OFF_LABEL_STACK_PTR)))
              (if (i32.eqz (local.get $sp))
                (then (br $skip))  ;; function-level end: skip
              )
              (local.set $sp (i32.sub (local.get $sp) (i32.const 1)))
              (i32.store (global.get $OFF_LABEL_STACK_PTR) (local.get $sp))
              (local.set $label_pos (i32.load (i32.add (local.get $label_base) (i32.shl (local.get $sp) (i32.const 2)))))
              ;; Check the opcode at the label position to determine if it's a loop
              (local.set $op_base
                (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (i32.add (local.get $start) (local.get $label_pos)) (global.get $DEC_SZ)))
              )
              (local.set $sp (i32.load8_u (local.get $op_base)))
              (i32.store
                (i32.add (global.get $OFF_DECODED_OPS)
                  (i32.add (i32.mul (local.get $label_pos) (global.get $DEC_SZ)) (i32.const 12))
                )
                (if (result i32) (i32.eq (local.get $sp) (i32.const 0x03))
                  (then (local.get $label_pos))
                  (else (i32.add (local.get $pos) (i32.const 1)))
                )
              )
              (br $skip)
            )
          )
        )

        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (if (i32.lt_u (local.get $pos) (local.get $count))
          (then (br $cont))
          (else (br $lp))
        )
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Execution engine ── stack operations
  ;; ═════════════════════════════════════════════════════════════════════
  (func $stack_push (param $val i32) (result i32)
    (if (i32.ge_u (global.get $FAST_STACK_LEN) (global.get $MAX_STACK))
      (then (return (global.get $ERR_STK_OV)))
    )
    (i32.store
      (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2)))
      (local.get $val)
    )
    (global.set $FAST_STACK_LEN (i32.add (global.get $FAST_STACK_LEN) (i32.const 1)))
    (return (global.get $OK))
  )

  (func $stack_pop (result i32)
    (if (i32.eqz (global.get $FAST_STACK_LEN))
      (then (return (global.get $ERR_STK_UND)))
    )
    (global.set $FAST_STACK_LEN (i32.sub (global.get $FAST_STACK_LEN) (i32.const 1)))
    (i32.store (global.get $OFF_SCRATCH0)
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2))))
    )
    (return (global.get $OK))
  )

  (func $stack_peek (result i32)
    (if (i32.eqz (global.get $FAST_STACK_LEN))
      (then (return (global.get $ERR_STK_UND)))
    )
    (i32.store (global.get $OFF_SCRATCH0)
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.sub (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2))))
    )
    (return (global.get $OK))
  )

  ;; ── Fast unchecked stack operations (no scratch0, no error checks) ──
  (func $stack_push_val (param $val i32)
    (i32.store
      (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2)))
      (local.get $val)
    )
    (global.set $FAST_STACK_LEN (i32.add (global.get $FAST_STACK_LEN) (i32.const 1)))
  )

  (func $stack_pop_val (result i32)
    (global.set $FAST_STACK_LEN (i32.sub (global.get $FAST_STACK_LEN) (i32.const 1)))
    (return (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2)))))
  )

  (func $stack_peek_val (result i32)
    (return (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.sub (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2)))))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; i64 stack operations — each i64 value occupies two 4-byte slots
  ;; (low 32 bits at lower address, high 32 bits at higher address)
  ;; ═════════════════════════════════════════════════════════════════════
  (func $stack_push_i64 (param $lo i32) (param $hi i32) (result i32)
    (if (i32.ge_u (i32.add (global.get $FAST_STACK_LEN) (i32.const 2)) (global.get $MAX_STACK))
      (then (return (global.get $ERR_STK_OV)))
    )
    (i32.store
      (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2)))
      (local.get $lo)
    )
    (i32.store
      (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.add (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2)))
      (local.get $hi)
    )
    (global.set $FAST_STACK_LEN (i32.add (global.get $FAST_STACK_LEN) (i32.const 2)))
    (return (global.get $OK))
  )

  (func $stack_pop_i64 (result i32)
    (if (i32.lt_u (global.get $FAST_STACK_LEN) (i32.const 2))
      (then (return (global.get $ERR_STK_UND)))
    )
    (global.set $FAST_STACK_LEN (i32.sub (global.get $FAST_STACK_LEN) (i32.const 2)))
    (i32.store (global.get $OFF_SCRATCH1)  ;; hi
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.add (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2))))
    )
    (i32.store (global.get $OFF_SCRATCH0)  ;; lo
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (global.get $FAST_STACK_LEN) (i32.const 2))))
    )
    (return (global.get $OK))
  )

  (func $stack_peek_i64 (result i32)
    (if (i32.lt_u (global.get $FAST_STACK_LEN) (i32.const 2))
      (then (return (global.get $ERR_STK_UND)))
    )
    (i32.store (global.get $OFF_SCRATCH1)  ;; hi
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.add (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2))))
    )
    (i32.store (global.get $OFF_SCRATCH0)  ;; lo
      (i32.load (i32.add (global.get $OFF_EXEC_STACK) (i32.shl (i32.sub (global.get $FAST_STACK_LEN) (i32.const 1)) (i32.const 2))))
    )
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Control frame stack operations
  ;; ═════════════════════════════════════════════════════════════════════
  (func $ctrl_push (param $kind i32) (param $target i32) (result i32)
    (local $len i32) (local $base i32)
    (local.set $len (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
    (if (i32.ge_u (local.get $len) (global.get $MAX_CTRL))
      (then (return (global.get $ERR_STK_OV)))
    )
    (local.set $base (i32.add (global.get $OFF_EXEC_CTRL) (i32.mul (local.get $len) (global.get $SZ_CTRL))))
    (i32.store (local.get $base) (local.get $kind))
    (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $target))
    (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.add (local.get $len) (i32.const 1)))
    (return (global.get $OK))
  )

  (func $ctrl_pop (result i32)
    (local $len i32) (local $base i32)
    (local.set $len (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
    (if (i32.eqz (local.get $len))
      (then (return (global.get $ERR_STK_UND)))
    )
    (local.set $len (i32.sub (local.get $len) (i32.const 1)))
    (i32.store (global.get $OFF_EXEC_CTRL_LEN) (local.get $len))
    (local.set $base (i32.add (global.get $OFF_EXEC_CTRL) (i32.mul (local.get $len) (global.get $SZ_CTRL))))
    (i32.store (global.get $OFF_SCRATCH0) (i32.load (local.get $base)))       ;; kind
    (i32.store (global.get $OFF_SCRATCH1) (i32.load (i32.add (local.get $base) (i32.const 4))))  ;; target
    (return (global.get $OK))
  )

  (func $ctrl_peek (param $depth i32) (result i32)
    ;; Returns kind of the Nth ancestor frame (0 = innermost)
    ;; Stores target in scratch1
    (local $len i32) (local $base i32)
    (local.set $len (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
    (local.set $len (i32.sub (local.get $len) (i32.const 1)))
    (if (i32.lt_u (local.get $len) (local.get $depth))
      (then (return (global.get $ERR_STK_UND)))
    )
    (local.set $base (i32.add (global.get $OFF_EXEC_CTRL) (i32.mul (i32.sub (local.get $len) (local.get $depth)) (global.get $SZ_CTRL))))
    (i32.store (global.get $OFF_SCRATCH0) (i32.load (local.get $base)))
    (i32.store (global.get $OFF_SCRATCH1) (i32.load (i32.add (local.get $base) (i32.const 4))))
    (return (global.get $OK))
  )

  (func $ctrl_depth (result i32)
    (return (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Execute a decoded function by its function index
  ;; Sets up locals from args if provided, runs dispatch on decoded ops
  ;; ═════════════════════════════════════════════════════════════════════
  (func $exec_fn (param $func_idx i32) (param $args_ptr i32) (param $args_len i32) (result i32)
    (local $code_idx i32) (local $code_base i32)
    (local $local_count i32) (local $i i32) (local $val i32)
    (local $dec_start i32) (local $dec_count i32) (local $dec_idx i32)
    (local $op i32) (local $op_base i32)
    (local $p i32) (local $err i32)
    (local $imm0 i32) (local $imm1 i32) (local $imm2 i32) (local $imm3 i32)
    (local $ctrl_len i32) (local $target i32) (local $kind i32)
    (local $tmp64 i64) (local $tmp64b i64)
    (local $tmp_f32 f32) (local $tmp_f32b f32) (local $tmp_f64 f64) (local $tmp_f64b f64)
    (local $type_idx i32) (local $result_count i32)

    ;; Find code index for this function (adjusting for imports)
    (if (i32.lt_u (local.get $func_idx) (i32.load (global.get $OFF_IMPORT_COUNT)))
      (then (return (global.get $ERR_MISS_IMP)))
    )
    (local.set $code_idx (i32.sub (local.get $func_idx) (i32.load (global.get $OFF_IMPORT_COUNT))))

    (local.set $code_base (i32.add (global.get $OFF_CODE_BUF) (i32.mul (local.get $code_idx) (global.get $SZ_CODE))))

    ;; Look up result count from function type
    (local.set $type_idx (i32.load (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (local.get $code_idx) (global.get $SZ_FUNC)))))
    (local.set $result_count (i32.load16_u (i32.add (global.get $OFF_TYPES_BUF) (i32.add (i32.mul (local.get $type_idx) (global.get $SZ_TYPE)) (i32.const 136)))))

    ;; Read decoded op range
    (local.set $dec_start (i32.load (i32.add (local.get $code_base) (i32.const 24))))
    (local.set $dec_count (i32.load (i32.add (local.get $code_base) (i32.const 32))))
    (local.set $dec_idx (local.get $dec_start))

    ;; Read local count
    (local.set $local_count (i32.load (i32.add (local.get $code_base) (i32.const 16))))
    (i32.store (global.get $OFF_EXEC_LOCAL_COUNT) (local.get $local_count))
    ;; Cache stack length in global for fast access
    (global.set $FAST_STACK_LEN (i32.load (global.get $OFF_EXEC_STACK_LEN)))

    ;; Initialize locals
    (local.set $i (i32.const 0))
    (block $init_lp
      (loop $init_cont
        (if (i32.ge_u (local.get $i) (local.get $local_count)) (then (br $init_lp)))
        (i32.store
          (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2)))
          (i32.const 0)
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $init_cont)
      )
    )

    ;; Copy args into first N locals
    (local.set $i (i32.const 0))
    (block $args_lp
      (loop $args_cont
        (if (i32.ge_u (local.get $i) (local.get $args_len)) (then (br $args_lp)))
        (i32.store
          (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2)))
          (i32.load (i32.add (local.get $args_ptr) (i32.shl (local.get $i) (i32.const 2))))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $args_cont)
      )
    )

    ;; ── Main dispatch loop ──
    (block $dispatch_end
      (loop $dispatch_loop
        ;; Check if we've exhausted decoded ops
        (if (i32.ge_u (local.get $dec_idx) (i32.add (local.get $dec_start) (local.get $dec_count)))
          (then
            (i32.store (global.get $OFF_EXEC_STACK_LEN) (global.get $FAST_STACK_LEN))
            (br $dispatch_end)
          )
        )

        ;; Read decoded op
        (local.set $op_base (i32.add (global.get $OFF_DECODED_OPS) (i32.mul (local.get $dec_idx) (global.get $DEC_SZ))))
        (local.set $op (i32.load8_u (local.get $op_base)))
        (local.set $imm0 (i32.load (i32.add (local.get $op_base) (i32.const 4))))
        (local.set $imm1 (i32.load (i32.add (local.get $op_base) (i32.const 8))))

        (block $op_done
        (block $unsup
          (block $h255
          (block $h254
          (block $h253
          (block $h252
          (block $h251
          (block $h250
          (block $h249
          (block $h248
          (block $h247
          (block $h246
          (block $h245
          (block $h244
          (block $h243
          (block $h242
          (block $h241
          (block $h240
          (block $h239
          (block $h238
          (block $h237
          (block $h236
          (block $h235
          (block $h234
          (block $h233
          (block $h232
          (block $h231
          (block $h230
          (block $h229
          (block $h228
          (block $h227
          (block $h226
          (block $h225
          (block $h224
          (block $h223
          (block $h222
          (block $h221
          (block $h220
          (block $h219
          (block $h218
          (block $h217
          (block $h216
          (block $h215
          (block $h214
          (block $h213
          (block $h212
          (block $h211
          (block $h210
          (block $h209
          (block $h208
          (block $h207
          (block $h206
          (block $h205
          (block $h204
          (block $h203
          (block $h202
          (block $h201
          (block $h200
          (block $h199
          (block $h198
          (block $h197
          (block $h196
          (block $h195
          (block $h194
          (block $h193
          (block $h192
          (block $h191
          (block $h190
          (block $h189
          (block $h188
          (block $h187
          (block $h186
          (block $h185
          (block $h184
          (block $h183
          (block $h182
          (block $h181
          (block $h180
          (block $h179
          (block $h178
          (block $h177
          (block $h176
          (block $h175
          (block $h174
          (block $h173
          (block $h172
          (block $h171
          (block $h170
          (block $h169
          (block $h168
          (block $h167
          (block $h166
          (block $h165
          (block $h164
          (block $h163
          (block $h162
          (block $h161
          (block $h160
          (block $h159
          (block $h158
          (block $h157
          (block $h156
          (block $h155
          (block $h154
          (block $h153
          (block $h152
          (block $h151
          (block $h150
          (block $h149
          (block $h148
          (block $h147
          (block $h146
          (block $h145
          (block $h144
          (block $h143
          (block $h142
          (block $h141
          (block $h140
          (block $h139
          (block $h138
          (block $h137
          (block $h136
          (block $h135
          (block $h134
          (block $h133
          (block $h132
          (block $h131
          (block $h130
          (block $h129
          (block $h128
          (block $h127
          (block $h126
          (block $h125
          (block $h124
          (block $h123
          (block $h122
          (block $h121
          (block $h120
          (block $h119
          (block $h118
          (block $h117
          (block $h116
          (block $h115
          (block $h114
          (block $h113
          (block $h112
          (block $h111
          (block $h110
          (block $h109
          (block $h108
          (block $h107
          (block $h106
          (block $h105
          (block $h104
          (block $h103
          (block $h102
          (block $h101
          (block $h100
          (block $h99
          (block $h98
          (block $h97
          (block $h96
          (block $h95
          (block $h94
          (block $h93
          (block $h92
          (block $h91
          (block $h90
          (block $h89
          (block $h88
          (block $h87
          (block $h86
          (block $h85
          (block $h84
          (block $h83
          (block $h82
          (block $h81
          (block $h80
          (block $h79
          (block $h78
          (block $h77
          (block $h76
          (block $h75
          (block $h74
          (block $h73
          (block $h72
          (block $h71
          (block $h70
          (block $h69
          (block $h68
          (block $h67
          (block $h66
          (block $h65
          (block $h64
          (block $h63
          (block $h62
          (block $h61
          (block $h60
          (block $h59
          (block $h58
          (block $h57
          (block $h56
          (block $h55
          (block $h54
          (block $h53
          (block $h52
          (block $h51
          (block $h50
          (block $h49
          (block $h48
          (block $h47
          (block $h46
          (block $h45
          (block $h44
          (block $h43
          (block $h42
          (block $h41
          (block $h40
          (block $h39
          (block $h38
          (block $h37
          (block $h36
          (block $h35
          (block $h34
          (block $h33
          (block $h32
          (block $h31
          (block $h30
          (block $h29
          (block $h28
          (block $h27
          (block $h26
          (block $h25
          (block $h24
          (block $h23
          (block $h22
          (block $h21
          (block $h20
          (block $h19
          (block $h18
          (block $h17
          (block $h16
          (block $h15
          (block $h14
          (block $h13
          (block $h12
          (block $h11
          (block $h10
          (block $h9
          (block $h8
          (block $h7
          (block $h6
          (block $h5
          (block $h4
          (block $h3
          (block $h2
          (block $h1
          (block $h0
            (if (i32.gt_u (local.get $op) (i32.const 255))
              (then (return (global.get $ERR_UNSUP)))
            )
            (br_table $h0 $h1 $h2 $h3 $h4 $h5 $h6 $h7 $h8 $h9 $h10 $h11 $h12 $h13 $h14 $h15 $h16 $h17 $h18 $h19 $h20 $h21 $h22 $h23 $h24 $h25 $h26 $h27 $h28 $h29 $h30 $h31 $h32 $h33 $h34 $h35 $h36 $h37 $h38 $h39 $h40 $h41 $h42 $h43 $h44 $h45 $h46 $h47 $h48 $h49 $h50 $h51 $h52 $h53 $h54 $h55 $h56 $h57 $h58 $h59 $h60 $h61 $h62 $h63 $h64 $h65 $h66 $h67 $h68 $h69 $h70 $h71 $h72 $h73 $h74 $h75 $h76 $h77 $h78 $h79 $h80 $h81 $h82 $h83 $h84 $h85 $h86 $h87 $h88 $h89 $h90 $h91 $h92 $h93 $h94 $h95 $h96 $h97 $h98 $h99 $h100 $h101 $h102 $h103 $h104 $h105 $h106 $h107 $h108 $h109 $h110 $h111 $h112 $h113 $h114 $h115 $h116 $h117 $h118 $h119 $h120 $h121 $h122 $h123 $h124 $h125 $h126 $h127 $h128 $h129 $h130 $h131 $h132 $h133 $h134 $h135 $h136 $h137 $h138 $h139 $h140 $h141 $h142 $h143 $h144 $h145 $h146 $h147 $h148 $h149 $h150 $h151 $h152 $h153 $h154 $h155 $h156 $h157 $h158 $h159 $h160 $h161 $h162 $h163 $h164 $h165 $h166 $h167 $h168 $h169 $h170 $h171 $h172 $h173 $h174 $h175 $h176 $h177 $h178 $h179 $h180 $h181 $h182 $h183 $h184 $h185 $h186 $h187 $h188 $h189 $h190 $h191 $h192 $h193 $h194 $h195 $h196 $h197 $h198 $h199 $h200 $h201 $h202 $h203 $h204 $h205 $h206 $h207 $h208 $h209 $h210 $h211 $h212 $h213 $h214 $h215 $h216 $h217 $h218 $h219 $h220 $h221 $h222 $h223 $h224 $h225 $h226 $h227 $h228 $h229 $h230 $h231 $h232 $h233 $h234 $h235 $h236 $h237 $h238 $h239 $h240 $h241 $h242 $h243 $h244 $h245 $h246 $h247 $h248 $h249 $h250 $h251 $h252 $h253 $h254 $h255 $unsup (local.get $op))
          )  ;; close $h0
          )  ;; close $h1
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h2
              (local.set $target (i32.load (i32.add (local.get $op_base) (i32.const 12))))  ;; next_idx
              (local.set $err (call $ctrl_push (i32.const 0x02) (local.get $target)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h3
              (local.set $target (i32.add (local.get $dec_idx) (i32.const 1)))  ;; loop br target = first op after loop
              (local.set $err (call $ctrl_push (i32.const 0x03) (local.get $target)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h4
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $target (i32.load (i32.add (local.get $op_base) (i32.const 12))))  ;; next_idx = else/end+1
              (local.set $err (call $ctrl_push (i32.const 0x04) (local.get $target)))
              (if (local.get $err) (then (return (local.get $err))))
              (if (i32.eqz (local.get $val))
                (then
                  ;; condition is 0: skip to else or end
                  (local.set $dec_idx (local.get $target))
                  (br $dispatch_loop)
                )
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h5
              (local.set $target (i32.load (i32.add (local.get $op_base) (i32.const 12))))  ;; next_idx = end+1
              (local.set $dec_idx (local.get $target))
              (br $dispatch_loop)
          )  ;; close $h6
        (return (global.get $ERR_UNSUP))
          )  ;; close $h7
        (return (global.get $ERR_UNSUP))
          )  ;; close $h8
        (return (global.get $ERR_UNSUP))
          )  ;; close $h9
        (return (global.get $ERR_UNSUP))
          )  ;; close $h10
        (return (global.get $ERR_UNSUP))
          )  ;; close $h11
              (local.set $ctrl_len (call $ctrl_depth))
              ;; If no control frames or this is the last op, it's a function return
              (if (i32.or (i32.eqz (local.get $ctrl_len))
                          (i32.eq (local.get $dec_idx) (i32.sub (i32.add (local.get $dec_start) (local.get $dec_count)) (i32.const 1))))
                (then
                  ;; function-level end: pop result_count values and return
                  (if (local.get $result_count)
                    (then
                      (local.set $i (i32.sub (local.get $result_count) (i32.const 1)))
                      (block $fr_res_lp
                        (loop $fr_res_cont
                          (if (i32.lt_s (local.get $i) (i32.const 0)) (then (br $fr_res_lp)))
                          (local.set $err (call $stack_pop))
                          (if (local.get $err) (then (return (local.get $err))))
                          (i64.store
                            (i32.add (global.get $OFF_EXEC_RESULT) (i32.shl (local.get $i) (i32.const 3)))
                            (i64.extend_i32_s (i32.load (global.get $OFF_SCRATCH0)))
                          )
                          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                          (br $fr_res_cont)
                        )
                      )
                    )
                  )
                  (i32.store (global.get $OFF_EXEC_RES_COUNT) (local.get $result_count))
                  (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.const 0))  ;; reset control stack
                  (i32.store (global.get $OFF_EXEC_STACK_LEN) (global.get $FAST_STACK_LEN))
                  (br $dispatch_end)
                )
                (else
                  ;; block/loop/if end: pop control frame
                  (local.set $err (call $ctrl_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $kind (i32.load (global.get $OFF_SCRATCH0)))
                  (local.set $target (i32.load (global.get $OFF_SCRATCH1)))
                  ;; For loops, jump back to loop start (target is loop's own pos)
                  (if (i32.eq (local.get $kind) (i32.const 0x03))  ;; LOOP
                    (then
                      (local.set $dec_idx (local.get $target))
                      (br $dispatch_loop)
                    )
                  )
                  ;; For block/if/else: fall through to next op
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
          )  ;; close $h12
              ;; imm0 = label depth
              (local.set $err (call $ctrl_peek (local.get $imm0)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $target (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $dec_idx (local.get $target))
              (br $dispatch_loop)
          )  ;; close $h13
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.eqz (local.get $val))
                (then
                  ;; condition is 0: fall through
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; condition is true: branch
              (local.set $err (call $ctrl_peek (local.get $imm0)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $target (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $dec_idx (local.get $target))
              (br $dispatch_loop)
          )  ;; close $h14
              ;; imm0 = number of labels (count)
              ;; imm1 = default label depth (stored by decoder)
              ;; Pop selector value (to keep stack balanced)
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              ;; For MVP: use default label from imm1
              (local.set $err (call $ctrl_peek (local.get $imm1)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $target (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $dec_idx (local.get $target))
              (br $dispatch_loop)
          )  ;; close $h15
                ;; Pop result_count values into result buffer
                (if (local.get $result_count)
                  (then
                    (local.set $i (i32.sub (local.get $result_count) (i32.const 1)))
                    (block $ret_res_lp
                      (loop $ret_res_cont
                        (if (i32.lt_s (local.get $i) (i32.const 0)) (then (br $ret_res_lp)))
                        (local.set $err (call $stack_pop))
                        (if (local.get $err) (then (return (local.get $err))))
                        (i64.store
                          (i32.add (global.get $OFF_EXEC_RESULT) (i32.shl (local.get $i) (i32.const 3)))
                          (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0)))
                        )
                        (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                        (br $ret_res_cont)
                      )
                    )
                  )
                )
                (i32.store (global.get $OFF_EXEC_RES_COUNT) (local.get $result_count))
              ;; Reset control stack
              (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.const 0))
              (i32.store (global.get $OFF_EXEC_STACK_LEN) (global.get $FAST_STACK_LEN))
              (br $dispatch_end)
          )  ;; close $h16
              ;; imm0 = function index
              ;; Check if it's an imported function
              (if (i32.lt_u (local.get $imm0) (i32.load (global.get $OFF_IMPORT_COUNT)))
                (then (return (global.get $ERR_MISS_IMP)))
              )
              ;; Look up type to get param count (adjust for imports)
              (local.set $val (i32.load (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (i32.sub (local.get $imm0) (i32.load (global.get $OFF_IMPORT_COUNT))) (global.get $SZ_FUNC)))))
              (local.set $val (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $val) (global.get $SZ_TYPE))))
              (local.set $val (i32.load16_u (i32.add (local.get $val) (i32.const 128))))  ;; param_count
              ;; Pop params into call args buffer (reversed order - WASM convention)
              (local.set $i (i32.sub (local.get $val) (i32.const 1)))
              (block $pop_params
                (loop $pop_cont
                  (if (i32.lt_s (local.get $i) (i32.const 0)) (then (br $pop_params)))
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (i32.store
                    (i32.add (global.get $OFF_CALL_ARGS) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (global.get $OFF_SCRATCH0))
                  )
                  (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                  (br $pop_cont)
                )
              )
              ;; Flush cached stack length to memory before call
              (i32.store (global.get $OFF_EXEC_STACK_LEN) (global.get $FAST_STACK_LEN))
              ;; Save current state to frame save area
              (local.set $p (i32.load (global.get $OFF_EXEC_CALL_DEPTH)))
              (local.set $op_base (i32.add (global.get $OFF_FRAME_SAVE) (i32.mul (local.get $p) (i32.const 272))))
              (i32.store (local.get $op_base) (i32.add (local.get $dec_idx) (i32.const 1)))  ;; return dec_idx
              (i32.store (i32.add (local.get $op_base) (i32.const 4)) (i32.load (global.get $OFF_EXEC_STACK_LEN)))
              (i32.store (i32.add (local.get $op_base) (i32.const 8)) (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
              (i32.store (i32.add (local.get $op_base) (i32.const 12)) (local.get $local_count))
              ;; Save locals
              (local.set $i (i32.const 0))
              (block $save_locals
                (loop $save_cont
                  (if (i32.ge_u (local.get $i) (local.get $local_count)) (then (br $save_locals)))
                  (i32.store
                    (i32.add (local.get $op_base) (i32.const 16) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2))))
                  )
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $save_cont)
                )
              )
              ;; Increment call depth
              (i32.store (global.get $OFF_EXEC_CALL_DEPTH) (i32.add (local.get $p) (i32.const 1)))
              ;; Execute target function (recursive call to $exec_fn)
              (local.set $err (call $exec_fn (local.get $imm0) (global.get $OFF_CALL_ARGS) (local.get $val)))
              (if (local.get $err) (then (return (local.get $err))))
              ;; Restore call depth
              (i32.store (global.get $OFF_EXEC_CALL_DEPTH) (local.get $p))
              ;; Restore caller state from frame save
              (local.set $dec_idx (i32.load (local.get $op_base)))  ;; return dec_idx
              (i32.store (global.get $OFF_EXEC_STACK_LEN) (i32.load (i32.add (local.get $op_base) (i32.const 4))))
              (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.load (i32.add (local.get $op_base) (i32.const 8))))
              (local.set $local_count (i32.load (i32.add (local.get $op_base) (i32.const 12))))
              ;; Reload cached stack length after restore
              (global.set $FAST_STACK_LEN (i32.load (global.get $OFF_EXEC_STACK_LEN)))
              ;; Restore locals
              (local.set $i (i32.const 0))
              (block $rest_locals
                (loop $rest_cont
                  (if (i32.ge_u (local.get $i) (local.get $local_count)) (then (br $rest_locals)))
                  (i32.store
                    (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (i32.add (local.get $op_base) (i32.const 16) (i32.shl (local.get $i) (i32.const 2))))
                  )
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $rest_cont)
                )
              )
              ;; Push results from exec_fn onto value stack
              (local.set $i (i32.const 0))
              (block $call_res_lp
                (loop $call_res_cont
                  (if (i32.ge_u (local.get $i) (i32.load (global.get $OFF_EXEC_RES_COUNT)))
                    (then (br $call_res_lp))
                  )
                  (local.set $err (call $stack_push
                    (i32.wrap_i64 (i64.load (i32.add (global.get $OFF_EXEC_RESULT) (i32.shl (local.get $i) (i32.const 3)))))
                  ))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $call_res_cont)
                )
              )
              (br $dispatch_loop)
          )  ;; close $h17
              ;; imm0 = expected type_idx, imm1 = table_idx (must be 0)
              (if (i32.ne (local.get $imm1) (i32.const 0)) (then (return (global.get $ERR_UNSUP))))
              ;; Check table exists
              (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))

              ;; Pop selector (i32) from value stack
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))

              ;; Check selector in table bounds
              (if (i32.ge_u (local.get $val) (i32.load (global.get $OFF_TABLE_MIN)))
                (then (return (global.get $ERR_TRAP)))
              )

              ;; Read function index from table[selector]
              (local.set $type_idx (i32.load (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (local.get $val) (i32.const 2)))))
              (if (i32.eq (local.get $type_idx) (i32.const -1)) (then (return (global.get $ERR_TRAP))))

              ;; Verify the function type matches expected type
              ;; funcs_buf holds type_idx for each function. But functions include imports,
              ;; so we need to get type_idx for the actual function at func_idx.
              (if (i32.lt_u (local.get $type_idx) (i32.load (global.get $OFF_IMPORT_COUNT)))
                (then (return (global.get $ERR_MISS_IMP)))
              )
              (local.set $p (i32.load
                (i32.add (global.get $OFF_FUNCTIONS_BUF)
                  (i32.mul (i32.sub (local.get $type_idx) (i32.load (global.get $OFF_IMPORT_COUNT))) (global.get $SZ_FUNC))
                )
              ))
              ;; $p = actual type_idx of the target function
              (if (i32.ne (local.get $p) (local.get $imm0)) (then (return (global.get $ERR_TRAP))))

              ;; Look up param count from expected type
              (local.set $val (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $imm0) (global.get $SZ_TYPE))))
              (local.set $val (i32.load16_u (i32.add (local.get $val) (i32.const 128))))

              ;; Pop params into call args buffer
              (local.set $i (i32.sub (local.get $val) (i32.const 1)))
              (block $ci_pop_params
                (loop $ci_pop_cont
                  (if (i32.lt_s (local.get $i) (i32.const 0)) (then (br $ci_pop_params)))
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (i32.store
                    (i32.add (global.get $OFF_CALL_ARGS) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (global.get $OFF_SCRATCH0))
                  )
                  (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                  (br $ci_pop_cont)
                )
              )

              ;; Save current state to frame save area
              (local.set $p (i32.load (global.get $OFF_EXEC_CALL_DEPTH)))
              (local.set $op_base (i32.add (global.get $OFF_FRAME_SAVE) (i32.mul (local.get $p) (i32.const 272))))
              (i32.store (local.get $op_base) (i32.add (local.get $dec_idx) (i32.const 1)))
              (i32.store (i32.add (local.get $op_base) (i32.const 4)) (i32.load (global.get $OFF_EXEC_STACK_LEN)))
              (i32.store (i32.add (local.get $op_base) (i32.const 8)) (i32.load (global.get $OFF_EXEC_CTRL_LEN)))
              (i32.store (i32.add (local.get $op_base) (i32.const 12)) (local.get $local_count))
              ;; Save locals
              (local.set $i (i32.const 0))
              (block $ci_save_locals
                (loop $ci_save_cont
                  (if (i32.ge_u (local.get $i) (local.get $local_count)) (then (br $ci_save_locals)))
                  (i32.store
                    (i32.add (local.get $op_base) (i32.const 16) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2))))
                  )
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $ci_save_cont)
                )
              )
              ;; Increment call depth and call target function
              (i32.store (global.get $OFF_EXEC_CALL_DEPTH) (i32.add (local.get $p) (i32.const 1)))
              (local.set $err (call $exec_fn (local.get $type_idx) (global.get $OFF_CALL_ARGS) (local.get $val)))
              (if (local.get $err) (then (return (local.get $err))))
              ;; Restore call depth
              (i32.store (global.get $OFF_EXEC_CALL_DEPTH) (local.get $p))
              ;; Restore caller state from frame save
              (local.set $dec_idx (i32.load (local.get $op_base)))
              (i32.store (global.get $OFF_EXEC_STACK_LEN) (i32.load (i32.add (local.get $op_base) (i32.const 4))))
              (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.load (i32.add (local.get $op_base) (i32.const 8))))
              (local.set $local_count (i32.load (i32.add (local.get $op_base) (i32.const 12))))
              ;; Restore locals
              (local.set $i (i32.const 0))
              (block $ci_rest_locals
                (loop $ci_rest_cont
                  (if (i32.ge_u (local.get $i) (local.get $local_count)) (then (br $ci_rest_locals)))
                  (i32.store
                    (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $i) (i32.const 2)))
                    (i32.load (i32.add (local.get $op_base) (i32.const 16) (i32.shl (local.get $i) (i32.const 2))))
                  )
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $ci_rest_cont)
                )
              )
              ;; Push results onto value stack
              (local.set $i (i32.const 0))
              (block $ci_res_lp
                (loop $ci_res_cont
                  (if (i32.ge_u (local.get $i) (i32.load (global.get $OFF_EXEC_RES_COUNT)))
                    (then (br $ci_res_lp))
                  )
                  (local.set $err (call $stack_push
                    (i32.wrap_i64 (i64.load (i32.add (global.get $OFF_EXEC_RESULT) (i32.shl (local.get $i) (i32.const 3)))))
                  ))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $ci_res_cont)
                )
              )
              (br $dispatch_loop)
          )  ;; close $h18
        (return (global.get $ERR_UNSUP))
          )  ;; close $h19
        (return (global.get $ERR_UNSUP))
          )  ;; close $h20
        (return (global.get $ERR_UNSUP))
          )  ;; close $h21
        (return (global.get $ERR_UNSUP))
          )  ;; close $h22
        (return (global.get $ERR_UNSUP))
          )  ;; close $h23
        (return (global.get $ERR_UNSUP))
          )  ;; close $h24
        (return (global.get $ERR_UNSUP))
          )  ;; close $h25
        (return (global.get $ERR_UNSUP))
          )  ;; close $h26
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h27
              ;; Pop condition (i32), then val2, then val1
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))  ;; condition
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))  ;; val2
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; val1
              (local.set $err (call $stack_push
                (if (result i32) (i32.eqz (local.get $imm0)) (then (local.get $imm1)) (else (local.get $imm2)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h28
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.eqz (local.get $imm0)) (then (local.get $imm1)) (else (local.get $imm2)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h29
        (return (global.get $ERR_UNSUP))
          )  ;; close $h30
        (return (global.get $ERR_UNSUP))
          )  ;; close $h31
        (return (global.get $ERR_UNSUP))
          )  ;; close $h32
              (local.set $err (call $stack_push
                (i32.load
                  (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $imm0) (i32.const 2)))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h33
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store
                (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $imm0) (i32.const 2)))
                (i32.load (global.get $OFF_SCRATCH0))
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h34
              (local.set $err (call $stack_peek))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store
                (i32.add (global.get $OFF_EXEC_LOCALS) (i32.shl (local.get $imm0) (i32.const 2)))
                (i32.load (global.get $OFF_SCRATCH0))
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h35
              (local.set $err (call $stack_push
                (i32.load (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (local.get $imm0) (i32.const 2))))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h36
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store
                (i32.add (global.get $OFF_GLOBALS_BUF) (i32.shl (local.get $imm0) (i32.const 2)))
                (i32.load (global.get $OFF_SCRATCH0))
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h37
              (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.ge_u (local.get $val) (i32.load (global.get $OFF_TABLE_MIN)))
                (then (return (global.get $ERR_TRAP)))
              )
              (local.set $err (call $stack_push
                (i32.load (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (local.get $val) (i32.const 2))))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h38
              (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; value
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $p (i32.load (global.get $OFF_SCRATCH0)))     ;; index
              (if (i32.ge_u (local.get $p) (i32.load (global.get $OFF_TABLE_MIN)))
                (then (return (global.get $ERR_TRAP)))
              )
              (i32.store
                (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (local.get $p) (i32.const 2)))
                (local.get $val)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h39
        (return (global.get $ERR_UNSUP))
          )  ;; close $h40
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              ;; imm1 = static offset; addr = val + imm1 + guest_mem_base
              (local.set $err (call $stack_push
                (i32.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h41
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h42
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (i32.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h43
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h44
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (i32.load8_s (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h45
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (i32.load8_u (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h46
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (i32.load16_s (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h47
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (i32.load16_u (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h48
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_s (i32.load8_s (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h49
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_u (i32.load8_u (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h50
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_s (i32.load16_s (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h51
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_u (i32.load16_u (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h52
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_s (i32.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h53
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $tmp64 (i64.extend_i32_u (i32.load (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE)))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h54
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; value to store
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $p (i32.load (global.get $OFF_SCRATCH0)))     ;; address
              ;; imm1 = static offset; addr = p + imm1 + guest_mem_base
              (i32.store
                (i32.add (i32.add (local.get $p) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $val)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h55
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (i64.store
                (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $tmp64)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h56
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; f32 bits to store
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $p (i32.load (global.get $OFF_SCRATCH0)))     ;; address
              (i32.store
                (i32.add (i32.add (local.get $p) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $val)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h57
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (i64.store
                (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $tmp64)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h58
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $p (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store8
                (i32.add (i32.add (local.get $p) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $val)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h59
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $p (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store16
                (i32.add (i32.add (local.get $p) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $val)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h60
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; lo
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store8
                (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $imm2)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h61
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store16
                (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $imm2)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h62
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store
                (i32.add (i32.add (local.get $val) (local.get $imm1)) (global.get $OFF_GUEST_MEM_BASE))
                (local.get $imm2)
              )
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h63
              (local.set $err (call $stack_push (i32.load (global.get $OFF_GUEST_MEM_PAGES))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h64
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))  ;; requested pages
              (local.set $imm0 (i32.load (global.get $OFF_GUEST_MEM_PAGES)))  ;; current pages
              ;; Compute max available: host_total_pages - guest_base_pages
              (local.set $imm1 (memory.size))  ;; total host pages
              (local.set $imm1 (i32.sub (local.get $imm1) (i32.const 32)))  ;; minus 32 pages for guest base (0x200000)
              (if (i32.le_u (i32.add (local.get $imm0) (local.get $val)) (local.get $imm1))
                (then
                  ;; Growth fits: update current pages, return old size
                  (i32.store (global.get $OFF_GUEST_MEM_PAGES) (i32.add (local.get $imm0) (local.get $val)))
                  (local.set $err (call $stack_push (local.get $imm0)))
                )
                (else
                  ;; Growth doesn't fit: return -1
                  (local.set $err (call $stack_push (i32.const -1)))
                )
              )
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h65
              (local.set $err (call $stack_push (local.get $imm0)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h66
              (local.set $tmp64 (i64.load (i32.add (local.get $op_base) (i32.const 4))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h67
              (local.set $err (call $stack_push
                (i32.load (i32.add (local.get $op_base) (i32.const 4)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h68
              (local.set $tmp64 (i64.load (i32.add (local.get $op_base) (i32.const 4))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h69
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.eqz (local.get $val)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h70
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.eq (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h71
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.ne (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h72
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.lt_s (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h73
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.lt_u (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h74
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.gt_s (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h75
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.gt_u (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h76
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.le_s (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h77
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.le_u (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h78
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.ge_s (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h79
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.ge_u (local.get $imm0) (local.get $imm1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h80
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (if (result i32) (i64.eqz (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h81
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.eq (local.get $tmp64) (local.get $tmp64b)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h82
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.ne (local.get $tmp64) (local.get $tmp64b)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h83
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.lt_s (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h84
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.lt_u (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h85
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.gt_s (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h86
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.gt_u (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h87
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.le_s (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h88
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.le_u (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h89
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.ge_s (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h90
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (i64.ge_u (local.get $tmp64b) (local.get $tmp64)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h91
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.eq (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h92
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.ne (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h93
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.lt (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h94
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.gt (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h95
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.le (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h96
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (if (result i32)
                  (f32.ge (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))) (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1))))
                  (then (i32.const 1))
                  (else (i32.const 0))
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h97
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.eq (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h98
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.ne (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h99
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.lt (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h100
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.gt (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h101
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.le (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h102
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push
                (if (result i32) (f64.ge (f64.reinterpret_i64 (local.get $tmp64b)) (f64.reinterpret_i64 (local.get $tmp64))) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h103
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.clz (local.get $val))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h104
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.ctz (local.get $val))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h105
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.popcnt (local.get $val))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h106
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.add (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h107
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.sub (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h108
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.mul (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h109
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.eqz (local.get $imm1))
                (then (return (global.get $ERR_TRAP)))  ;; div by zero
              )
              (if (i32.and (i32.eq (local.get $imm0) (i32.const 0x80000000)) (i32.eq (local.get $imm1) (i32.const -1)))
                (then (return (global.get $ERR_TRAP)))  ;; INT_MIN / -1
              )
              (local.set $err (call $stack_push (i32.div_s (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h110
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.eqz (local.get $imm1))
                (then (return (global.get $ERR_TRAP)))  ;; div by zero
              )
              (local.set $err (call $stack_push (i32.div_u (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h111
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.eqz (local.get $imm1))
                (then (return (global.get $ERR_TRAP)))  ;; rem by zero
              )
              (local.set $err (call $stack_push (i32.rem_s (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h112
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (if (i32.eqz (local.get $imm1))
                (then (return (global.get $ERR_TRAP)))  ;; rem by zero
              )
              (local.set $err (call $stack_push (i32.rem_u (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h113
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.and (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h114
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.or (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h115
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.xor (local.get $imm0) (local.get $imm1))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h116
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.shl (local.get $imm0) (i32.and (local.get $imm1) (i32.const 31)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h117
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.shr_s (local.get $imm0) (i32.and (local.get $imm1) (i32.const 31)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h118
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.shr_u (local.get $imm0) (i32.and (local.get $imm1) (i32.const 31)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h119
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.rotl (local.get $imm0) (i32.and (local.get $imm1) (i32.const 31)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h120
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push (i32.rotr (local.get $imm0) (i32.and (local.get $imm1) (i32.const 31)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h121
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.clz (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h122
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.ctz (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h123
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.popcnt (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h124
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.add (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h125
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.sub (local.get $tmp64b) (local.get $tmp64)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h126
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.mul (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h127
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; lo divisor
              (local.set $imm3 (i32.load (global.get $OFF_SCRATCH1)))  ;; hi divisor
              (if (i32.eqz (i32.or (local.get $imm2) (local.get $imm3)))
                (then (return (global.get $ERR_TRAP)))  ;; div by zero
              )
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (if (i32.and (i32.eq (local.get $imm2) (i32.const 0xFFFFFFFF)) (i32.eq (local.get $imm3) (i32.const 0xFFFFFFFF)))
                (then
                  (if (i64.eq (local.get $tmp64) (i64.const -9223372036854775808))
                    (then (return (global.get $ERR_TRAP)))  ;; INT64_MIN / -1
                  )
                )
              )
              (local.set $tmp64b (i64.or (i64.extend_i32_u (local.get $imm2)) (i64.shl (i64.extend_i32_u (local.get $imm3)) (i64.const 32))))
              (local.set $tmp64 (i64.div_s (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h128
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm3 (i32.load (global.get $OFF_SCRATCH1)))
              (if (i32.eqz (i32.or (local.get $imm2) (local.get $imm3)))
                (then (return (global.get $ERR_TRAP)))
              )
              (local.set $tmp64b (i64.or (i64.extend_i32_u (local.get $imm2)) (i64.shl (i64.extend_i32_u (local.get $imm3)) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.div_u (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h129
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm3 (i32.load (global.get $OFF_SCRATCH1)))
              (if (i32.eqz (i32.or (local.get $imm2) (local.get $imm3)))
                (then (return (global.get $ERR_TRAP)))
              )
              (local.set $tmp64b (i64.or (i64.extend_i32_u (local.get $imm2)) (i64.shl (i64.extend_i32_u (local.get $imm3)) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.rem_s (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h130
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm3 (i32.load (global.get $OFF_SCRATCH1)))
              (if (i32.eqz (i32.or (local.get $imm2) (local.get $imm3)))
                (then (return (global.get $ERR_TRAP)))
              )
              (local.set $tmp64b (i64.or (i64.extend_i32_u (local.get $imm2)) (i64.shl (i64.extend_i32_u (local.get $imm3)) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.rem_u (local.get $tmp64) (local.get $tmp64b)))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h131
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (local.get $imm0) (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $imm3 (i32.and (local.get $imm1) (i32.load (global.get $OFF_SCRATCH1))))
              (local.set $err (call $stack_push_i64 (local.get $imm2) (local.get $imm3)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h132
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.or (local.get $imm0) (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $imm3 (i32.or (local.get $imm1) (i32.load (global.get $OFF_SCRATCH1))))
              (local.set $err (call $stack_push_i64 (local.get $imm2) (local.get $imm3)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h133
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm0 (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $imm1 (i32.load (global.get $OFF_SCRATCH1)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.xor (local.get $imm0) (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $imm3 (i32.xor (local.get $imm1) (i32.load (global.get $OFF_SCRATCH1))))
              (local.set $err (call $stack_push_i64 (local.get $imm2) (local.get $imm3)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h134
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (i32.load (global.get $OFF_SCRATCH0)) (i32.const 63)))  ;; shift amount (lo i32 & 63)
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.shl (local.get $tmp64) (i64.extend_i32_u (local.get $imm2))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h135
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (i32.load (global.get $OFF_SCRATCH0)) (i32.const 63)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.shr_s (local.get $tmp64) (i64.extend_i32_u (local.get $imm2))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h136
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (i32.load (global.get $OFF_SCRATCH0)) (i32.const 63)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.shr_u (local.get $tmp64) (i64.extend_i32_u (local.get $imm2))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h137
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (i32.load (global.get $OFF_SCRATCH0)) (i32.const 63)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.rotl (local.get $tmp64) (i64.extend_i32_u (local.get $imm2))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h138
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $imm2 (i32.and (i32.load (global.get $OFF_SCRATCH0)) (i32.const 63)))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.rotr (local.get $tmp64) (i64.extend_i32_u (local.get $imm2))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h139
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.abs (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h140
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.neg (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h141
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.ceil (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h142
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.floor (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h143
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.trunc (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h144
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.nearest (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h145
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp_f32 (f32.sqrt (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (local.get $tmp_f32))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h146
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.add
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h147
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.sub
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h148
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.mul
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h149
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.div
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h150
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.min
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h151
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.max
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h152
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push
                (i32.reinterpret_f32
                  (f32.copysign
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))
                    (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH1)))
                  )
                )
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h153
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.abs (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h154
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.neg (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h155
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.ceil (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h156
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.floor (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h157
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.trunc (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h158
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.nearest (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h159
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.sqrt (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h160
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.add (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h161
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.sub (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h162
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.mul (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h163
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.div (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h164
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.min (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h165
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.max (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h166
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64b (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp_f64 (f64.copysign (f64.reinterpret_i64 (local.get $tmp64)) (f64.reinterpret_i64 (local.get $tmp64b))))
              (local.set $tmp64 (i64.reinterpret_f64 (local.get $tmp_f64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h167
              ;; Pop i64 (2 slots), push lo as i32 (1 slot)
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push (i32.load (global.get $OFF_SCRATCH0))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h168
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push (i32.trunc_f32_s (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h169
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push (i32.trunc_f32_u (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h170
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (i32.trunc_f64_s (f64.reinterpret_i64 (local.get $tmp64)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h171
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (i32.trunc_f64_u (f64.reinterpret_i64 (local.get $tmp64)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h172
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.extend_i32_s (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h173
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $err (call $stack_push_i64
                (i32.wrap_i64 (local.get $tmp64))
                (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h174
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.trunc_f32_s (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h175
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.trunc_f32_u (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h176
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.trunc_f64_s (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h177
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.trunc_f64_u (f64.reinterpret_i64 (local.get $tmp64))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h178
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (f32.convert_i32_s (i32.load (global.get $OFF_SCRATCH0))))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h179
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (f32.convert_i32_u (i32.load (global.get $OFF_SCRATCH0))))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h180
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (f32.convert_i64_s (local.get $tmp64)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h181
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (f32.convert_i64_u (local.get $tmp64)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h182
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $err (call $stack_push (i32.reinterpret_f32 (f32.demote_f64 (f64.reinterpret_i64 (local.get $tmp64))))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h183
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.reinterpret_f64 (f64.convert_i32_s (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h184
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.reinterpret_f64 (f64.convert_i32_u (i32.load (global.get $OFF_SCRATCH0)))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h185
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.reinterpret_f64 (f64.convert_i64_s (local.get $tmp64))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h186
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.reinterpret_f64 (f64.convert_i64_u (local.get $tmp64))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h187
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.reinterpret_f64 (f64.promote_f32 (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h188
              ;; Value on stack is already i32 bit pattern, no-op
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h189
              ;; Value on stack is already i64 (two slots), no-op
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h190
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h191
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h192
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.extend8_s (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $err (call $stack_push (local.get $val)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h193
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.extend16_s (i32.load (global.get $OFF_SCRATCH0))))
              (local.set $err (call $stack_push (local.get $val)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h194
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.extend8_s (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h195
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.extend16_s (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h196
              (local.set $err (call $stack_pop_i64))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
              (local.set $tmp64 (i64.extend32_s (local.get $tmp64)))
              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h197
        (return (global.get $ERR_UNSUP))
          )  ;; close $h198
        (return (global.get $ERR_UNSUP))
          )  ;; close $h199
        (return (global.get $ERR_UNSUP))
          )  ;; close $h200
        (return (global.get $ERR_UNSUP))
          )  ;; close $h201
        (return (global.get $ERR_UNSUP))
          )  ;; close $h202
        (return (global.get $ERR_UNSUP))
          )  ;; close $h203
        (return (global.get $ERR_UNSUP))
          )  ;; close $h204
        (return (global.get $ERR_UNSUP))
          )  ;; close $h205
        (return (global.get $ERR_UNSUP))
          )  ;; close $h206
        (return (global.get $ERR_UNSUP))
          )  ;; close $h207
        (return (global.get $ERR_UNSUP))
          )  ;; close $h208
              (local.set $err (call $stack_push (i32.const -1)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h209
              (local.set $err (call $stack_pop))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $err (call $stack_push
                (if (result i32) (i32.eq (local.get $val) (i32.const -1)) (then (i32.const 1)) (else (i32.const 0)))
              ))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h210
              ;; imm0 = function index, push it as reference value
              (local.set $err (call $stack_push (local.get $imm0)))
              (if (local.get $err) (then (return (local.get $err))))
              (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
              (br $dispatch_loop)
          )  ;; close $h211
        (return (global.get $ERR_UNSUP))
          )  ;; close $h212
        (return (global.get $ERR_UNSUP))
          )  ;; close $h213
        (return (global.get $ERR_UNSUP))
          )  ;; close $h214
        (return (global.get $ERR_UNSUP))
          )  ;; close $h215
        (return (global.get $ERR_UNSUP))
          )  ;; close $h216
        (return (global.get $ERR_UNSUP))
          )  ;; close $h217
        (return (global.get $ERR_UNSUP))
          )  ;; close $h218
        (return (global.get $ERR_UNSUP))
          )  ;; close $h219
        (return (global.get $ERR_UNSUP))
          )  ;; close $h220
        (return (global.get $ERR_UNSUP))
          )  ;; close $h221
        (return (global.get $ERR_UNSUP))
          )  ;; close $h222
        (return (global.get $ERR_UNSUP))
          )  ;; close $h223
        (return (global.get $ERR_UNSUP))
          )  ;; close $h224
        (return (global.get $ERR_UNSUP))
          )  ;; close $h225
        (return (global.get $ERR_UNSUP))
          )  ;; close $h226
        (return (global.get $ERR_UNSUP))
          )  ;; close $h227
        (return (global.get $ERR_UNSUP))
          )  ;; close $h228
        (return (global.get $ERR_UNSUP))
          )  ;; close $h229
        (return (global.get $ERR_UNSUP))
          )  ;; close $h230
        (return (global.get $ERR_UNSUP))
          )  ;; close $h231
        (return (global.get $ERR_UNSUP))
          )  ;; close $h232
        (return (global.get $ERR_UNSUP))
          )  ;; close $h233
        (return (global.get $ERR_UNSUP))
          )  ;; close $h234
        (return (global.get $ERR_UNSUP))
          )  ;; close $h235
        (return (global.get $ERR_UNSUP))
          )  ;; close $h236
        (return (global.get $ERR_UNSUP))
          )  ;; close $h237
        (return (global.get $ERR_UNSUP))
          )  ;; close $h238
        (return (global.get $ERR_UNSUP))
          )  ;; close $h239
        (return (global.get $ERR_UNSUP))
          )  ;; close $h240
        (return (global.get $ERR_UNSUP))
          )  ;; close $h241
        (return (global.get $ERR_UNSUP))
          )  ;; close $h242
        (return (global.get $ERR_UNSUP))
          )  ;; close $h243
        (return (global.get $ERR_UNSUP))
          )  ;; close $h244
        (return (global.get $ERR_UNSUP))
          )  ;; close $h245
        (return (global.get $ERR_UNSUP))
          )  ;; close $h246
        (return (global.get $ERR_UNSUP))
          )  ;; close $h247
        (return (global.get $ERR_UNSUP))
          )  ;; close $h248
        (return (global.get $ERR_UNSUP))
          )  ;; close $h249
        (return (global.get $ERR_UNSUP))
          )  ;; close $h250
        (return (global.get $ERR_UNSUP))
          )  ;; close $h251
        (return (global.get $ERR_UNSUP))
          )  ;; close $h252
              ;; imm0 = sub-opcode
              ;; ── Saturating truncation ops 0x00-0x07 ──
              (if (i32.eq (local.get $imm0) (i32.const 0x00))     ;; i32.trunc_sat_f32_s
                (then
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp_f32 (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))
                  (if (f32.ne (local.get $tmp_f32) (local.get $tmp_f32))
                    (then (local.set $err (call $stack_push (i32.const 0))))
                    (else
                      (if (f32.ge (local.get $tmp_f32) (f32.const 2147483648.0))
                        (then (local.set $err (call $stack_push (i32.const 0x7FFFFFFF))))
                        (else
                          (if (f32.lt (local.get $tmp_f32) (f32.const -2147483648.0))
                            (then (local.set $err (call $stack_push (i32.const 0x80000000))))
                            (else (local.set $err (call $stack_push (i32.trunc_f32_s (local.get $tmp_f32)))))
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x01))     ;; i32.trunc_sat_f32_u
                (then
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp_f32 (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))
                  (if (f32.ne (local.get $tmp_f32) (local.get $tmp_f32))
                    (then (local.set $err (call $stack_push (i32.const 0))))
                    (else
                      (if (f32.ge (local.get $tmp_f32) (f32.const 4294967296.0))
                        (then (local.set $err (call $stack_push (i32.const -1))))
                        (else
                          (if (f32.lt (local.get $tmp_f32) (f32.const 0.0))
                            (then (local.set $err (call $stack_push (i32.const 0))))
                            (else (local.set $err (call $stack_push (i32.trunc_f32_u (local.get $tmp_f32)))))
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x02))     ;; i32.trunc_sat_f64_s
                (then
                  (local.set $err (call $stack_pop_i64))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
                  (local.set $tmp_f64 (f64.reinterpret_i64 (local.get $tmp64)))
                  (if (f64.ne (local.get $tmp_f64) (local.get $tmp_f64))
                    (then (local.set $err (call $stack_push (i32.const 0))))
                    (else
                      (if (f64.ge (local.get $tmp_f64) (f64.const 2147483648.0))
                        (then (local.set $err (call $stack_push (i32.const 0x7FFFFFFF))))
                        (else
                          (if (f64.lt (local.get $tmp_f64) (f64.const -2147483648.0))
                            (then (local.set $err (call $stack_push (i32.const 0x80000000))))
                            (else (local.set $err (call $stack_push (i32.trunc_f64_s (local.get $tmp_f64)))))
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x03))     ;; i32.trunc_sat_f64_u
                (then
                  (local.set $err (call $stack_pop_i64))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
                  (local.set $tmp_f64 (f64.reinterpret_i64 (local.get $tmp64)))
                  (if (f64.ne (local.get $tmp_f64) (local.get $tmp_f64))
                    (then (local.set $err (call $stack_push (i32.const 0))))
                    (else
                      (if (f64.ge (local.get $tmp_f64) (f64.const 4294967296.0))
                        (then (local.set $err (call $stack_push (i32.const -1))))
                        (else
                          (if (f64.lt (local.get $tmp_f64) (f64.const 0.0))
                            (then (local.set $err (call $stack_push (i32.const 0))))
                            (else (local.set $err (call $stack_push (i32.trunc_f64_u (local.get $tmp_f64)))))
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x04))     ;; i64.trunc_sat_f32_s
                (then
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp_f32 (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))
                  (if (f32.ne (local.get $tmp_f32) (local.get $tmp_f32))
                    (then
                      (local.set $tmp64 (i64.const 0))
                      (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                    )
                    (else
                      (if (f32.ge (local.get $tmp_f32) (f32.const 9223372036854775808.0))
                        (then
                          (local.set $tmp64 (i64.const 0x7FFFFFFFFFFFFFFF))
                          (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                        )
                        (else
                          (if (f32.lt (local.get $tmp_f32) (f32.const -9223372036854775808.0))
                            (then
                              (local.set $tmp64 (i64.const 0x8000000000000000))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                            (else
                              (local.set $tmp64 (i64.trunc_f32_s (local.get $tmp_f32)))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x05))     ;; i64.trunc_sat_f32_u
                (then
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp_f32 (f32.reinterpret_i32 (i32.load (global.get $OFF_SCRATCH0))))
                  (if (f32.ne (local.get $tmp_f32) (local.get $tmp_f32))
                    (then
                      (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                    )
                    (else
                      (if (f32.ge (local.get $tmp_f32) (f32.const 18446744073709551616.0))
                        (then
                          (local.set $err (call $stack_push_i64 (i32.const -1) (i32.const -1)))
                        )
                        (else
                          (if (f32.lt (local.get $tmp_f32) (f32.const 0.0))
                            (then
                              (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                            )
                            (else
                              (local.set $tmp64 (i64.trunc_f32_u (local.get $tmp_f32)))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x06))     ;; i64.trunc_sat_f64_s
                (then
                  (local.set $err (call $stack_pop_i64))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
                  (local.set $tmp_f64 (f64.reinterpret_i64 (local.get $tmp64)))
                  (if (f64.ne (local.get $tmp_f64) (local.get $tmp_f64))
                    (then
                      (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                    )
                    (else
                      (if (f64.ge (local.get $tmp_f64) (f64.const 9223372036854775808.0))
                        (then
                          (local.set $tmp64 (i64.const 0x7FFFFFFFFFFFFFFF))
                          (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                        )
                        (else
                          (if (f64.lt (local.get $tmp_f64) (f64.const -9223372036854775808.0))
                            (then
                              (local.set $tmp64 (i64.const 0x8000000000000000))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                            (else
                              (local.set $tmp64 (i64.trunc_f64_s (local.get $tmp_f64)))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x07))     ;; i64.trunc_sat_f64_u
                (then
                  (local.set $err (call $stack_pop_i64))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $tmp64 (i64.or (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH0))) (i64.shl (i64.extend_i32_u (i32.load (global.get $OFF_SCRATCH1))) (i64.const 32))))
                  (local.set $tmp_f64 (f64.reinterpret_i64 (local.get $tmp64)))
                  (if (f64.ne (local.get $tmp_f64) (local.get $tmp_f64))
                    (then
                      (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                    )
                    (else
                      (if (f64.ge (local.get $tmp_f64) (f64.const 18446744073709551616.0))
                        (then
                          (local.set $err (call $stack_push_i64 (i32.const -1) (i32.const -1)))
                        )
                        (else
                          (if (f64.lt (local.get $tmp_f64) (f64.const 0.0))
                            (then
                              (local.set $err (call $stack_push_i64 (i32.const 0) (i32.const 0)))
                            )
                            (else
                              (local.set $tmp64 (i64.trunc_f64_u (local.get $tmp_f64)))
                              (local.set $err (call $stack_push_i64 (i32.wrap_i64 (local.get $tmp64)) (i32.wrap_i64 (i64.shr_u (local.get $tmp64) (i64.const 32)))))
                            )
                          )
                        )
                      )
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0A))     ;; memory.copy
                (then
                  ;; pop n, src, dst
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm3 (i32.load (global.get $OFF_SCRATCH0)))  ;; src
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))  ;; dst
                  ;; Bounds check
                  (local.set $p (i32.shl (i32.load (global.get $OFF_GUEST_MEM_PAGES)) (i32.const 16)))
                  (if (i32.gt_u (i32.add (local.get $val) (local.get $imm2)) (local.get $p))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  (if (i32.gt_u (i32.add (local.get $imm3) (local.get $imm2)) (local.get $p))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Copy bytes
                  (local.set $i (i32.const 0))
                  (block $bulk_copy_lp
                    (loop $bulk_copy_cont
                      (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $bulk_copy_lp)))
                      (i32.store8
                        (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $val) (local.get $i)))
                        (i32.load8_u (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $imm3) (local.get $i))))
                      )
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $bulk_copy_cont)
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x0B))     ;; memory.fill
                (then
                  ;; pop n, val, dst
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm3 (i32.load (global.get $OFF_SCRATCH0)))  ;; val
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))  ;; dst
                  ;; Bounds check
                  (local.set $p (i32.shl (i32.load (global.get $OFF_GUEST_MEM_PAGES)) (i32.const 16)))
                  (if (i32.gt_u (i32.add (local.get $val) (local.get $imm2)) (local.get $p))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Fill bytes (val = dst, imm3 = fill byte, imm2 = count)
                  (local.set $i (i32.const 0))
                  (block $bulk_fill_lp
                    (loop $bulk_fill_cont
                      (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $bulk_fill_lp)))
                      (i32.store8
                        (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $val) (local.get $i)))
                        (i32.and (local.get $imm3) (i32.const 0xFF))
                      )
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $bulk_fill_cont)
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x08))     ;; memory.init
                (then
                  ;; imm1 = data_seg_idx
                  ;; pop n, src_offset, dst
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm3 (i32.load (global.get $OFF_SCRATCH0)))  ;; src_offset
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))  ;; dst
                  ;; Look up data segment info
                  (local.set $p (i32.load (global.get $OFF_DATA_COUNT)))
                  (if (i32.ge_u (local.get $imm1) (local.get $p)) (then (return (global.get $ERR_TRAP))))
                  (local.set $op_base (i32.add (global.get $OFF_DATA_BUF) (i32.mul (local.get $imm1) (global.get $SZ_DATA))))
                  (local.set $p (i32.load (local.get $op_base)))  ;; data_offset in WASM binary
                  (local.set $i (i32.load (i32.add (local.get $op_base) (i32.const 8))))  ;; data_len
                  ;; Check bounds: src_offset + n <= data_len
                  (if (i32.gt_u (i32.add (local.get $imm3) (local.get $imm2)) (local.get $i))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Check bounds: dst + n <= memory_size
                  (local.set $i (i32.shl (i32.load (global.get $OFF_GUEST_MEM_PAGES)) (i32.const 16)))
                  (if (i32.gt_u (i32.add (local.get $val) (local.get $imm2)) (local.get $i))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Copy n bytes from wasm_base[data_offset + src_offset] to guest_mem[dst]
                  (local.set $i (i32.const 0))
                  (local.set $p (i32.add (i32.load (global.get $OFF_WASM_PTR)) (local.get $p)))  ;; absolute data ptr
                  (block $mi_copy_lp
                    (loop $mi_copy_cont
                      (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $mi_copy_lp)))
                      (i32.store8
                        (i32.add (global.get $OFF_GUEST_MEM_BASE) (i32.add (local.get $val) (local.get $i)))
                        (i32.load8_u (i32.add (local.get $p) (i32.add (local.get $imm3) (local.get $i))))
                      )
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $mi_copy_cont)
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (if (i32.eq (local.get $imm0) (i32.const 0x09))     ;; data.drop
                (then
                  ;; imm1 = data_seg_idx
                  ;; Check segment index in bounds
                  (local.set $p (i32.load (global.get $OFF_DATA_COUNT)))
                  (if (i32.ge_u (local.get $imm1) (local.get $p)) (then (return (global.get $ERR_TRAP))))
                  ;; Mark segment as dropped by setting length to 0
                  (i32.store
                    (i32.add (global.get $OFF_DATA_BUF) (i32.add (i32.mul (local.get $imm1) (global.get $SZ_DATA)) (i32.const 8)))
                    (i32.const 0)
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── table.init (0x0C) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x0C))
                (then
                  ;; imm1 = elem_seg_idx
                  (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
                  ;; Pop n, s, d from stack
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm3 (i32.load (global.get $OFF_SCRATCH0)))  ;; s (elem src start)
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; d (table dst start)
                  ;; Check elem segment index in bounds
                  (local.set $p (i32.load (global.get $OFF_ELEM_COUNT)))
                  (if (i32.ge_u (local.get $imm1) (local.get $p)) (then (return (global.get $ERR_TRAP))))
                  ;; Get elem segment base
                  (local.set $p (i32.add (global.get $OFF_ELEM_BUF) (i32.mul (local.get $imm1) (global.get $SZ_ELEM))))
                  ;; Check elem segment not dropped
                  (if (i32.load (local.get $p)) (then (return (global.get $ERR_TRAP))))
                  ;; elem_count from buffer  
                  (local.set $i (i32.load (i32.add (local.get $p) (i32.const 8))))  ;; elem_count
                  ;; Check s + n <= elem_count
                  (if (i32.gt_u (i32.add (local.get $imm3) (local.get $imm2)) (local.get $i))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Check d + n <= table size
                  (local.set $i (i32.load (global.get $OFF_TABLE_MIN)))
                  (if (i32.gt_u (i32.add (local.get $val) (local.get $imm2)) (local.get $i))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Copy from elem buf to table
                  (local.set $i (i32.const 0))
                  (block $ti_lp
                    (loop $ti_cont
                      (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $ti_lp)))
                      (i32.store
                        (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $val) (local.get $i)) (i32.const 2)))
                        (i32.load
                          (i32.add (local.get $p) (i32.add (i32.const 12) (i32.shl (i32.add (local.get $imm3) (local.get $i)) (i32.const 2))))
                        )
                      )
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $ti_cont)
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── elem.drop (0x0D) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x0D))
                (then
                  ;; imm1 = elem_seg_idx
                  (local.set $p (i32.load (global.get $OFF_ELEM_COUNT)))
                  (if (i32.ge_u (local.get $imm1) (local.get $p)) (then (return (global.get $ERR_TRAP))))
                  ;; Mark as dropped
                  (i32.store
                    (i32.add (global.get $OFF_ELEM_BUF) (i32.mul (local.get $imm1) (global.get $SZ_ELEM)))
                    (i32.const 1)
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── table.copy (0x0E) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x0E))
                (then
                  (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
                  ;; Pop n, src_idx, dst_idx
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm3 (i32.load (global.get $OFF_SCRATCH0)))  ;; src_idx
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; dst_idx
                  ;; Bounds check
                  (local.set $p (i32.load (global.get $OFF_TABLE_MIN)))
                  (if (i32.or
                    (i32.gt_u (i32.add (local.get $val) (local.get $imm2)) (local.get $p))
                    (i32.gt_u (i32.add (local.get $imm3) (local.get $imm2)) (local.get $p))
                  ) (then (return (global.get $ERR_TRAP))))
                  ;; Copy (with overlap handling: use forward/backward as appropriate)
                  (if (i32.lt_u (local.get $val) (local.get $imm3))
                    (then
                      ;; Forward copy
                      (local.set $i (i32.const 0))
                      (block $tcf_lp
                        (loop $tcf_cont
                          (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $tcf_lp)))
                          (i32.store
                            (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $val) (local.get $i)) (i32.const 2)))
                            (i32.load
                              (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $imm3) (local.get $i)) (i32.const 2)))
                            )
                          )
                          (local.set $i (i32.add (local.get $i) (i32.const 1)))
                          (br $tcf_cont)
                        )
                      )
                    )
                    (else
                      ;; Backward copy for overlapping ranges
                      (local.set $i (local.get $imm2))
                      (block $tcb_lp
                        (loop $tcb_cont
                          (if (i32.eqz (local.get $i)) (then (br $tcb_lp)))
                          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                          (i32.store
                            (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $val) (local.get $i)) (i32.const 2)))
                            (i32.load
                              (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $imm3) (local.get $i)) (i32.const 2)))
                            )
                          )
                          (br $tcb_cont)
                        )
                      )
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── table.grow (0x0F) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x0F))
                (then
                  (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
                  ;; Pop delta (i32), then value (ref) from stack
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $p (i32.load (global.get $OFF_SCRATCH0)))    ;; delta
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))  ;; value
                  ;; Old size
                  (local.set $imm2 (i32.load (global.get $OFF_TABLE_MIN)))
                  (local.set $imm3 (i32.load (global.get $OFF_TABLE_MAX)))
                  ;; Check growth fits in max
                  (if (i32.gt_u (i32.add (local.get $imm2) (local.get $p)) (local.get $imm3))
                    (then
                      ;; Can't grow: push -1
                      (local.set $err (call $stack_push (i32.const -1)))
                    )
                    (else
                      ;; Grow: update min, init new entries with value
                      (local.set $i (i32.const 0))
                      (block $tg_lp
                        (loop $tg_cont
                          (if (i32.ge_u (local.get $i) (local.get $p)) (then (br $tg_lp)))
                          (i32.store
                            (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $imm2) (local.get $i)) (i32.const 2)))
                            (local.get $val)
                          )
                          (local.set $i (i32.add (local.get $i) (i32.const 1)))
                          (br $tg_cont)
                        )
                      )
                      (i32.store (global.get $OFF_TABLE_MIN) (i32.add (local.get $imm2) (local.get $p)))
                      ;; Push old size
                      (local.set $err (call $stack_push (local.get $imm2)))
                    )
                  )
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── table.size (0x10) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x10))
                (then
                  (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
                  (local.set $err (call $stack_push (i32.load (global.get $OFF_TABLE_MIN))))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              ;; ── table.fill (0x11) ──
              (if (i32.eq (local.get $imm0) (i32.const 0x11))
                (then
                  (if (i32.eqz (i32.load8_u (global.get $OFF_TABLE_HAS))) (then (return (global.get $ERR_TRAP))))
                  ;; Pop n, val, idx
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $imm2 (i32.load (global.get $OFF_SCRATCH0)))  ;; n
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $val (i32.load (global.get $OFF_SCRATCH0)))   ;; val
                  (local.set $err (call $stack_pop))
                  (if (local.get $err) (then (return (local.get $err))))
                  (local.set $p (i32.load (global.get $OFF_SCRATCH0)))     ;; idx
                  ;; Bounds check
                  (if (i32.gt_u (i32.add (local.get $p) (local.get $imm2)) (i32.load (global.get $OFF_TABLE_MIN)))
                    (then (return (global.get $ERR_TRAP)))
                  )
                  ;; Fill
                  (local.set $i (i32.const 0))
                  (block $tfl_lp
                    (loop $tfl_cont
                      (if (i32.ge_u (local.get $i) (local.get $imm2)) (then (br $tfl_lp)))
                      (i32.store
                        (i32.add (global.get $OFF_TABLE_DATA) (i32.shl (i32.add (local.get $p) (local.get $i)) (i32.const 2)))
                        (local.get $val)
                      )
                      (local.set $i (i32.add (local.get $i) (i32.const 1)))
                      (br $tfl_cont)
                    )
                  )
                  (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
                  (br $dispatch_loop)
                )
              )
              (return (global.get $ERR_UNSUP))
          )  ;; close $h253
        (return (global.get $ERR_UNSUP))
          )  ;; close $h254
        (return (global.get $ERR_UNSUP))
          )  ;; close $h255
        (return (global.get $ERR_UNSUP))
        )  ;; close $unsup
        )

        ;; Fallthrough increment for ops that didn't handle dec_idx
        (local.set $dec_idx (i32.add (local.get $dec_idx) (i32.const 1)))
        (br $dispatch_loop)
      )
    )

    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; call: call an exported function by index
  ;; ═════════════════════════════════════════════════════════════════════
  (func $call (export "call") (param $func_idx i32) (param $args_ptr i32) (param $args_len i32) (result i32)
    ;; Reset execution state
    (i32.store (global.get $OFF_EXEC_STACK_LEN) (i32.const 0))
    (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.const 0))
    (i32.store (global.get $OFF_EXEC_RES_COUNT) (i32.const 0))
    ;; Check module is loaded
    (if (i32.eqz (i32.load8_u (global.get $OFF_EXEC_MOD_VALID)))
      (then (return (global.get $ERR_BAD_ARGUMENT)))
    )
    (return (call $exec_fn (local.get $func_idx) (local.get $args_ptr) (local.get $args_len)))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; load: main entry point
  ;; ═════════════════════════════════════════════════════════════════════
  (func $load (export "load") (param $wasm_ptr i32) (param $wasm_len i32) (result i32)
    (local $offset i32) (local $err i32) (local $start_func i32) (local $sec_id i32)

    (i32.store (global.get $OFF_WASM_PTR) (local.get $wasm_ptr))
    (i32.store (global.get $OFF_WASM_LEN) (local.get $wasm_len))

    ;; Reset state
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_IMPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_GLOBAL_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_START_FUNC) (i32.const -1))
    (i32.store (global.get $OFF_DATA_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_ELEM_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_MEM_MIN) (i32.const 0))
    (i32.store (global.get $OFF_MEM_MAX) (i32.const 0))
    (i32.store (global.get $OFF_GUEST_MEM_PAGES) (i32.const 0))
    (i32.store8 (global.get $OFF_TABLE_HAS) (i32.const 0))
    (i32.store (global.get $OFF_NAMES_PTR) (global.get $OFF_NAMES_BUF))
    (i32.store8 (global.get $OFF_EXEC_MOD_VALID) (i32.const 0))
    (i32.store (global.get $OFF_DECODED_COUNT) (i32.const 0))

    (local.set $err (call $parse_header))
    (if (local.get $err) (then (return (local.get $err))))

    (local.set $offset (i32.const 8))

    (block $sec_loop
      (loop $sec_cont
        (if (i32.ge_u (local.get $offset) (local.get $wasm_len)) (then (br $sec_loop)))
        (local.set $err (call $read_section_header (local.get $offset)))
        (if (local.get $err) (then (return (local.get $err))))

        ;; Save section id to protect against scratch0 clobbering by parsers
        (local.set $sec_id (i32.load (global.get $OFF_SCRATCH0)))

        ;; scratch0=sec_id, scratch1=sec_len, scratch2=content_offset, scratch3=end_offset
        (if (i32.eq (local.get $sec_id) (global.get $SEC_TYPE))
          (then
            (local.set $err (call $parse_type_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_IMPORT))
          (then
            (local.set $err (call $parse_import_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_FUNCTION))
          (then
            (local.set $err (call $parse_function_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_TABLE))
          (then
            (local.set $err (call $parse_table_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_MEMORY))
          (then
            (local.set $err (call $parse_memory_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_GLOBAL))
          (then
            (local.set $err (call $parse_global_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_EXPORT))
          (then
            (local.set $err (call $parse_export_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_START))
          (then
            (local.set $err (call $parse_start_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_ELEMENT))
          (then
            (local.set $err (call $parse_element_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_CODE))
          (then
            (local.set $err (call $parse_code_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $sec_id))
          )
        )
        (if (i32.eq (local.get $sec_id) (global.get $SEC_DATA))
          (then
            (local.set $err (call $parse_data_section
              (i32.load (global.get $OFF_SCRATCH2))
              (i32.load (global.get $OFF_SCRATCH1))
            ))
            (if (local.get $err) (then (return (local.get $err))))
          )
        )
        ;; custom (0) and data_count (12): silently skipped

        ;; Advance to next section
        (local.set $offset (i32.load (global.get $OFF_SCRATCH3)))
        (br $sec_cont)
      )
    )

    ;; Execute start function if present
    (local.set $start_func (i32.load (global.get $OFF_START_FUNC)))
    (if (i32.ne (local.get $start_func) (i32.const -1))
      (then
        (i32.store (global.get $OFF_EXEC_STACK_LEN) (i32.const 0))
        (i32.store (global.get $OFF_EXEC_CTRL_LEN) (i32.const 0))
        (i32.store (global.get $OFF_EXEC_RES_COUNT) (i32.const 0))
        (local.set $err (call $exec_fn (local.get $start_func) (i32.const 0) (i32.const 0)))
        (if (local.get $err) (then (return (local.get $err))))
      )
    )

    (i32.store8 (global.get $OFF_EXEC_MOD_VALID) (i32.const 1))
    (return (global.get $OK))
  )

  (func $get_result_value (export "get_result_value") (param $idx i32) (result i64)
    (i64.store (global.get $OFF_SCRATCH0)
      (i64.load (i32.add (global.get $OFF_EXEC_RESULT) (i32.shl (local.get $idx) (i32.const 3))))
    )
    (return (i64.load (global.get $OFF_SCRATCH0)))
  )

  (func $get_result_count (export "get_result_count") (result i32)
    (return (i32.load (global.get $OFF_EXEC_RES_COUNT)))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WAT Parser — parse WAT text format directly into state buffers
  ;; ═════════════════════════════════════════════════════════════════════
  ;;
  ;; Memory layout for WAT parser:
  ;;   0x8C000: WAT source pointer
  ;;   0x8C004: WAT source length
  ;;   0x8C008: Current position
  ;;   0x8C00C: Symbol table count
  ;;   0x8C010: Saved wasm_ptr (for restoration)
  ;;   0x8C014: Saved wasm_len
  ;;   0x8C018: Error position
  ;;   0x8C01C: Body offset (temp)
  ;;   0x8B000: Symbol table (256 entries × 16 bytes)
  ;;   0x8D000: WASM bytecode emission buffer (4KB)

  (global $OFF_WAT_PTR  i32 (i32.const 0x8C000))
  (global $OFF_WAT_LEN  i32 (i32.const 0x8C004))
  (global $OFF_WAT_POS  i32 (i32.const 0x8C008))
  (global $OFF_WAT_SYM  i32 (i32.const 0x8C00C))
  (global $OFF_WAT_SAV_PTR i32 (i32.const 0x8C010))
  (global $OFF_WAT_SAV_LEN i32 (i32.const 0x8C014))
  (global $OFF_WAT_ERR    i32 (i32.const 0x8C018))
  (global $OFF_WAT_TMP    i32 (i32.const 0x8C01C))
  (global $OFF_WAT_SYMS   i32 (i32.const 0x8B000))
  (global $WAT_SYM_SZ     i32 (i32.const 16))
  (global $WAT_MAX_SYMS   i32 (i32.const 256))
  (global $OFF_WAT_BODY   i32 (i32.const 0x8D000))
  (global $WAT_BODY_SZ    i32 (i32.const 4096))
  (global $OFF_WAT_TMP0   i32 (i32.const 0x8C040))
  (global $OFF_WAT_TMP1   i32 (i32.const 0x8C044))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WAT Lexer
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Advance past whitespace and comments starting from pos.
  ;; Returns new position in scratch0, error in return value.
  (func $wat_skip_ws (param $pos i32) (result i32)
    (local $p i32) (local $l i32) (local $b i32) (local $end i32)
    (i32.store (i32.const 0x8C074) (i32.load (global.get $OFF_WAT_PTR)))
    (i32.store (i32.const 0x8C078) (local.get $pos))
    (local.set $p (i32.load (global.get $OFF_WAT_PTR)))
    (local.set $l (i32.load (global.get $OFF_WAT_LEN)))
    (local.set $end (i32.add (local.get $p) (local.get $l)))
    (local.set $p (i32.add (local.get $p) (local.get $pos)))
    (i32.store (i32.const 0x8C07C) (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        ;; space, tab, lf, cr
        (if (i32.eq (local.get $b) (i32.const 0x20)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x09)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x0A)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        (if (i32.eq (local.get $b) (i32.const 0x0D)) (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (br $lp)))
        ;; Line comment: ;;
        (if (i32.eq (local.get $b) (i32.const 0x3B))
          (then
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (if (i32.lt_u (local.get $p) (local.get $end))
              (then
                (local.set $b (i32.load8_u (local.get $p)))
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (if (i32.eq (local.get $b) (i32.const 0x3B))
                  (then
                    ;; Skip to end of line
                    (block $eol
                      (loop $eol_lp
                        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $eol)))
                        (local.set $b (i32.load8_u (local.get $p)))
                        (local.set $p (i32.add (local.get $p) (i32.const 1)))
                        (if (i32.or (i32.eq (local.get $b) (i32.const 0x0A)) (i32.eq (local.get $b) (i32.const 0x0D)))
                          (then (br $eol))
                          (else (br $eol_lp))
                        )
                      )
                    )
                    (br $lp)
                  )
                )
              )
            )
            (br $done)
          )
        )
        ;; Block comment: (;
        (if (i32.eq (local.get $b) (i32.const 0x28))
          (then
            (local.set $p (i32.add (local.get $p) (i32.const 1)))
            (if (i32.lt_u (local.get $p) (local.get $end))
              (then
                (local.set $b (i32.load8_u (local.get $p)))
                (if (i32.eq (local.get $b) (i32.const 0x3B))
                  (then
                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                    ;; Skip to ;)
                    (block $bce
                      (loop $bcl
                        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $bce)))
                        (local.set $b (i32.load8_u (local.get $p)))
                        (local.set $p (i32.add (local.get $p) (i32.const 1)))
                        (if (i32.eq (local.get $b) (i32.const 0x3B))
                          (then
                            (if (i32.lt_u (local.get $p) (local.get $end))
                              (then
                                (local.set $b (i32.load8_u (local.get $p)))
                                (if (i32.eq (local.get $b) (i32.const 0x29))
                                  (then
                                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                                    (br $bce)
                                  )
                                )
                              )
                            )
                          )
                        )
                        (br $bcl)
                      )
                    )
                    (br $lp)
                  )
                  (else
                    ;; Not a block comment, back up
                    (local.set $p (i32.sub (local.get $p) (i32.const 1)))
                  )
                )
              )
            )
            (br $done)
          )
        )
        (br $done)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (i32.sub (local.get $p) (i32.load (global.get $OFF_WAT_PTR))))
    (return (global.get $OK))
  )

  ;; Check if keyword at pos matches given kw_ptr/kw_len (case-insensitive).
  ;; Returns 1 on match, 0 on no match. Does NOT advance position.
  (func $wat_match_kw (param $pos i32) (param $kw_ptr i32) (param $kw_len i32) (result i32)
    (local $i i32) (local $p i32) (local $end i32) (local $b1 i32) (local $b2 i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (local.get $p) (local.get $kw_len)))
    ;; Check bounds
    (if (i32.gt_u (local.get $end) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
      (then (return (i32.const 0)))
    )
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $kw_len)) (then (br $done)))
        (local.set $b1 (i32.load8_u (i32.add (local.get $p) (local.get $i))))
        (local.set $b2 (i32.load8_u (i32.add (local.get $kw_ptr) (local.get $i))))
        ;; Case-insensitive compare: convert $b1 to lowercase
        (if (i32.and (i32.ge_u (local.get $b1) (i32.const 0x41)) (i32.le_u (local.get $b1) (i32.const 0x5A)))
          (then (local.set $b1 (i32.or (local.get $b1) (i32.const 32))))
        )
        (if (i32.and (i32.ge_u (local.get $b2) (i32.const 0x41)) (i32.le_u (local.get $b2) (i32.const 0x5A)))
          (then (local.set $b2 (i32.or (local.get $b2) (i32.const 32))))
        )
        (if (i32.ne (local.get $b1) (local.get $b2)) (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    (return (i32.const 1))
  )

  ;; Read a keyword at pos (identifier characters: a-z A-Z 0-9 . _ + - * / < > ! ~)
  ;; Stores in WAT source: the keyword starts at pos, with given length.
  ;; Output: scratch0=keyword_offset, scratch1=keyword_len, scratch2=new_pos
  ;; Returns error code.
  (func $wat_read_kw (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $start i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $start (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        ;; Check if identifier character
        (block $is_id
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x7A))) (then (br $is_id)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x5A))) (then (br $is_id)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39))) (then (br $is_id)))
          (if (i32.eq (local.get $b) (i32.const 0x2E)) (then (br $is_id)))  ;; .
          (if (i32.eq (local.get $b) (i32.const 0x5F)) (then (br $is_id)))  ;; _
          (if (i32.eq (local.get $b) (i32.const 0x2B)) (then (br $is_id)))  ;; +
          (if (i32.eq (local.get $b) (i32.const 0x2D)) (then (br $is_id)))  ;; -
          (if (i32.eq (local.get $b) (i32.const 0x2A)) (then (br $is_id)))  ;; *
          (if (i32.eq (local.get $b) (i32.const 0x2F)) (then (br $is_id)))  ;; /
          (if (i32.eq (local.get $b) (i32.const 0x3C)) (then (br $is_id)))  ;; <
          (if (i32.eq (local.get $b) (i32.const 0x3E)) (then (br $is_id)))  ;; >
          (if (i32.eq (local.get $b) (i32.const 0x21)) (then (br $is_id)))  ;; !
          (if (i32.eq (local.get $b) (i32.const 0x7E)) (then (br $is_id)))  ;; ~
          (if (i32.eq (local.get $b) (i32.const 0x27)) (then (br $is_id)))  ;; '
          (br $done)
        )
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $lp)
      )
    )
    (if (i32.eq (local.get $start) (local.get $p))
      (then (return (global.get $ERR_PARSE)))
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $pos))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (local.get $start)))
    (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $pos) (i32.sub (local.get $p) (local.get $start))))
    (return (global.get $OK))
  )

  ;; Read a $identifier starting at pos. Store the name (without $) in names buffer.
  ;; Output: scratch0=name_offset (in names buf), scratch1=name_len, scratch2=new_pos
  ;; Returns error code.
  (func $wat_read_id (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $start i32) (local $name_start i32)
    (local $dst i32) (local $i i32) (local $len i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.ne (local.get $b) (i32.const 0x24))  ;; '$'
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $p (i32.add (local.get $p) (i32.const 1)))
    (local.set $start (local.get $p))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $done)))
        (local.set $b (i32.load8_u (local.get $p)))
        (block $is_idc
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x7A))) (then (br $is_idc)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x5A))) (then (br $is_idc)))
          (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39))) (then (br $is_idc)))
          (if (i32.eq (local.get $b) (i32.const 0x2E)) (then (br $is_idc)))  ;; .
          (if (i32.eq (local.get $b) (i32.const 0x5F)) (then (br $is_idc)))  ;; _
          (if (i32.eq (local.get $b) (i32.const 0x2D)) (then (br $is_idc)))  ;; -
          (if (i32.eq (local.get $b) (i32.const 0x2B)) (then (br $is_idc)))  ;; +
          (if (i32.eq (local.get $b) (i32.const 0x27)) (then (br $is_idc)))  ;; '
          (br $done)
        )
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $lp)
      )
    )
    (local.set $len (i32.sub (local.get $p) (local.get $start)))
    (if (i32.eqz (local.get $len)) (then (return (global.get $ERR_PARSE))))
    ;; Copy name to names buffer
    (local.set $dst (i32.load (global.get $OFF_NAMES_PTR)))
    (local.set $i (i32.const 0))
    (block $clp
      (loop $cl
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $clp)))
        (i32.store8 (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $start) (local.get $i)))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $cl)
      )
    )
    (i32.store (global.get $OFF_NAMES_PTR) (i32.add (local.get $dst) (local.get $len)))
    (i32.store (global.get $OFF_SCRATCH0) (local.get $dst))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $len))
    (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $pos) (i32.add (i32.const 1) (local.get $len))))
    (return (global.get $OK))
  )

  ;; Read an unsigned integer (decimal or 0x hex).
  ;; Output: scratch0=value, scratch1=new_pos
  ;; Returns error code.
  (func $wat_read_uint (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $val i32) (local $start i32)
    (local $neg i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $start (local.get $p))
    (local.set $b (i32.load8_u (local.get $p)))
    ;; Optional leading sign for unsigned (will be treated as positive)
    (if (i32.eq (local.get $b) (i32.const 0x2D))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1))) (local.set $neg (i32.const 1)))
    )
    (if (i32.eq (local.get $b) (i32.const 0x2B))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1))))
    )
    ;; Check for hex prefix 0x or 0X
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.eq (local.get $b) (i32.const 0x30))
      (then
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (if (i32.lt_u (local.get $p) (local.get $end))
          (then
            (local.set $b (i32.load8_u (local.get $p)))
            (if (i32.or (i32.eq (local.get $b) (i32.const 0x78)) (i32.eq (local.get $b) (i32.const 0x58)))
              (then
                ;; Hex number
                (local.set $p (i32.add (local.get $p) (i32.const 1)))
                (local.set $val (i32.const 0))
                (block $hd
                  (loop $hl
                    (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $hd)))
                    (local.set $b (i32.load8_u (local.get $p)))
                    (block $hdig
                      (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x30)) (i32.le_u (local.get $b) (i32.const 0x39)))
                        (then (local.set $b (i32.sub (local.get $b) (i32.const 0x30))) (br $hdig)))
                      (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x61)) (i32.le_u (local.get $b) (i32.const 0x66)))
                        (then (local.set $b (i32.sub (local.get $b) (i32.sub (i32.const 0x61) (i32.const 10)))) (br $hdig)))
                      (if (i32.and (i32.ge_u (local.get $b) (i32.const 0x41)) (i32.le_u (local.get $b) (i32.const 0x46)))
                        (then (local.set $b (i32.sub (local.get $b) (i32.sub (i32.const 0x41) (i32.const 10)))) (br $hdig)))
                      (br $hd)
                    )
                    (local.set $val (i32.add (i32.shl (local.get $val) (i32.const 4)) (local.get $b)))
                    (local.set $p (i32.add (local.get $p) (i32.const 1)))
                    (br $hl)
                  )
                )
                (if (i32.eq (local.get $start) (i32.sub (local.get $p) (i32.const 2)))
                  (then (return (global.get $ERR_PARSE)))  ;; "0x" with no digits
                )
                (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
                (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (return (global.get $OK))
              )
            )
            ;; Not hex, it's a decimal 0
            (local.set $val (i32.const 0))
            (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
            (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (return (global.get $OK))
          )
        )
      )
    )
    ;; Decimal
    (local.set $val (i32.const 0))
    (block $dd
      (loop $dl
        (if (i32.ge_u (local.get $p) (local.get $end)) (then (br $dd)))
        (local.set $b (i32.load8_u (local.get $p)))
        (if (i32.or (i32.lt_u (local.get $b) (i32.const 0x30)) (i32.gt_u (local.get $b) (i32.const 0x39)))
          (then (br $dd))
        )
        (local.set $val (i32.add (i32.mul (local.get $val) (i32.const 10)) (i32.sub (local.get $b) (i32.const 0x30))))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        (br $dl)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
    (i32.store (global.get $OFF_SCRATCH1) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (return (global.get $OK))
  )

  ;; Read a signed integer (decimal or 0x hex).
  ;; Output: scratch0=value, scratch1=new_pos
  ;; Returns error code.
  (func $wat_read_sint (param $pos i32) (result i32)
    (local $p i32) (local $end i32) (local $b i32) (local $val i32) (local $neg i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $end (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
    (if (i32.ge_u (local.get $p) (local.get $end))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $neg (i32.const 0))
    (local.set $b (i32.load8_u (local.get $p)))
    (if (i32.eq (local.get $b) (i32.const 0x2D))
      (then (local.set $neg (i32.const 1)) (local.set $p (i32.add (local.get $p) (i32.const 1))))
    )
    (if (i32.eq (local.get $b) (i32.const 0x2B))
      (then (local.set $p (i32.add (local.get $p) (i32.const 1))))
    )
    ;; Save relative offset of number start (after sign)
    (local.set $pos (i32.add (local.get $pos) (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))))
    ;; Use unsigned reader on the rest
    (if (call $wat_read_uint (local.get $pos))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $val (i32.load (global.get $OFF_SCRATCH0)))
    (if (local.get $neg)
      (then
        (local.set $val (i32.sub (i32.const 0) (local.get $val)))
        (i32.store (global.get $OFF_SCRATCH0) (local.get $val))
      )
    )
    ;; Add sign chars to position advance
    (i32.store (global.get $OFF_SCRATCH1)
      (i32.add
        (i32.sub (local.get $p) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
        (i32.load (global.get $OFF_SCRATCH1))
      )
    )
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WAT Symbol Table
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Register a name in the symbol table.
  ;; name_off = offset in names buffer, name_len = length, kind = entity kind, index = entity index
  ;; Returns error (ERR_PARSE on overflow).
  (func $wat_sym_register (param $name_off i32) (param $name_len i32) (param $kind i32) (param $index i32) (result i32)
    (local $sym_cnt i32) (local $base i32)
    (local.set $sym_cnt (i32.load (global.get $OFF_WAT_SYM)))
    (if (i32.ge_u (local.get $sym_cnt) (global.get $WAT_MAX_SYMS))
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $base (i32.add (global.get $OFF_WAT_SYMS) (i32.mul (local.get $sym_cnt) (global.get $WAT_SYM_SZ))))
    (i32.store (i32.add (local.get $base) (i32.const 0)) (local.get $name_off))
    (i32.store (i32.add (local.get $base) (i32.const 4)) (local.get $name_len))
    (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $kind))
    (i32.store (i32.add (local.get $base) (i32.const 12)) (local.get $index))
    (i32.store (global.get $OFF_WAT_SYM) (i32.add (local.get $sym_cnt) (i32.const 1)))
    (return (global.get $OK))
  )

  ;; Look up a name in the symbol table.
  ;; kind = entity kind to search for (-1 = any kind).
  ;; Output: scratch0=index, scratch1=0 if found, 1 if not found.
  (func $wat_sym_lookup (param $name_off i32) (param $name_len i32) (param $kind i32) (result i32)
    (local $sym_cnt i32) (local $i i32) (local $base i32) (local $n_off i32) (local $n_len i32) (local $k i32)
    (local $j i32) (local $b1 i32) (local $b2 i32)
    (local.set $sym_cnt (i32.load (global.get $OFF_WAT_SYM)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $sym_cnt)) (then (br $done)))
        (local.set $base (i32.add (global.get $OFF_WAT_SYMS) (i32.mul (local.get $i) (global.get $WAT_SYM_SZ))))
        (local.set $k (i32.load (i32.add (local.get $base) (i32.const 8))))
        (if (i32.and (i32.ne (local.get $kind) (i32.const -1)) (i32.ne (local.get $k) (local.get $kind)))
          (then (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $lp))
        )
        (local.set $n_len (i32.load (i32.add (local.get $base) (i32.const 4))))
        (if (i32.ne (local.get $n_len) (local.get $name_len))
          (then (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $lp))
        )
        (local.set $n_off (i32.load (i32.add (local.get $base) (i32.const 0))))
        ;; Compare names byte-by-byte
        (local.set $j (i32.const 0))
        (block $cmp_done
          (loop $cmp
            (if (i32.ge_u (local.get $j) (local.get $n_len)) (then (br $cmp_done)))
            (local.set $b1 (i32.load8_u (i32.add (local.get $n_off) (local.get $j))))
            (local.set $b2 (i32.load8_u (i32.add (local.get $name_off) (local.get $j))))
            (if (i32.ne (local.get $b1) (local.get $b2))
              (then (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $lp))
            )
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $cmp)
          )
        )
        ;; Found!
        (i32.store (global.get $OFF_SCRATCH0) (i32.load (i32.add (local.get $base) (i32.const 12))))
        (i32.store (global.get $OFF_SCRATCH1) (i32.const 0))
        (return (global.get $OK))
      )
    )
    (i32.store (global.get $OFF_SCRATCH1) (i32.const 1))
    (return (global.get $OK))
  )

  ;; Clear the symbol table (for a new function body scope).
  (func $wat_sym_clear (result i32)
    (i32.store (global.get $OFF_WAT_SYM) (i32.const 0))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Value type helper: convert WAT type keyword → WASM type byte
  ;; Input: kw_offset, kw_len (from wat_read_kw)
  ;; Output: scratch0 = type byte (0x7F, etc.) or -1 if unknown
  ;; ═════════════════════════════════════════════════════════════════════
  (func $wat_valtype (param $kw_off i32) (param $kw_len i32) (result i32)
    (local $p i32) (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    (local.set $p (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)))
    (local.set $b0 (i32.load8_u (local.get $p)))
    (if (i32.gt_u (local.get $kw_len) (i32.const 1))
      (then (local.set $b1 (i32.load8_u (i32.add (local.get $p) (i32.const 1)))))
    )
    (if (i32.gt_u (local.get $kw_len) (i32.const 2))
      (then (local.set $b2 (i32.load8_u (i32.add (local.get $p) (i32.const 2)))))
    )
    (block $done
      (if (i32.eq (local.get $kw_len) (i32.const 3))
        (then
          (block $try3
            ;; "i32" = 0x69 0x33 0x32
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x69)) (i32.and (i32.eq (local.get $b1) (i32.const 0x33)) (i32.eq (local.get $b2) (i32.const 0x32))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7F)) (return (global.get $OK)))
            )
            ;; "i64" = 0x69 0x36 0x34
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x69)) (i32.and (i32.eq (local.get $b1) (i32.const 0x36)) (i32.eq (local.get $b2) (i32.const 0x34))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7E)) (return (global.get $OK)))
            )
            ;; "f32" = 0x66 0x33 0x32
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x66)) (i32.and (i32.eq (local.get $b1) (i32.const 0x33)) (i32.eq (local.get $b2) (i32.const 0x32))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7D)) (return (global.get $OK)))
            )
            ;; "f64" = 0x66 0x36 0x34
            (if (i32.and (i32.eq (local.get $b0) (i32.const 0x66)) (i32.and (i32.eq (local.get $b1) (i32.const 0x36)) (i32.eq (local.get $b2) (i32.const 0x34))))
              (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7C)) (return (global.get $OK)))
            )
          )
        )
      )
      (if (i32.eq (local.get $kw_len) (i32.const 4))
        (then
          (local.set $b3 (i32.load8_u (i32.add (local.get $p) (i32.const 3))))
          ;; "v128" = 0x76 ('v') 0x31 ('1') 0x32 ('2') 0x38 ('8')
          (if (i32.and (i32.eq (local.get $b0) (i32.const 0x76)) (i32.and (i32.eq (local.get $b1) (i32.const 0x31)) (i32.and (i32.eq (local.get $b2) (i32.const 0x32)) (i32.eq (local.get $b3) (i32.const 0x38)))))
            (then (i32.store (global.get $OFF_SCRATCH0) (i32.const 0x7B)) (return (global.get $OK)))
          )
        )
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (i32.const -1))
    (return (global.get $OK))
  )

  ;; Parse a (result ...) type annotation.
  ;; Expects: at pos, we've already consumed "(" and "result".
  ;; Reads value types until ")".
  ;; Output: scratch0=first_valtype_byte (0x40 if no results), scratch1=new_pos
  (func $wat_parse_result (param $pos i32) (result i32)
    (local $b i32) (local $vt i32)
    (local.set $vt (i32.const 0x40))  ;; default empty
    (block $lp
      (loop $cont
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; )
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $lp))
        )
        ;; Read value type keyword
        (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
          (then (return (global.get $ERR_PARSE)))
        )
        (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        ;; Only store first result type for now
        (br $lp)  ;; only support single result for now
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $vt))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; WASM bytecode emission helpers (to scratch buffer)
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Emit one byte to the body buffer at given offset.
  ;; Output: scratch0 = new offset
  (func $wat_emit_byte (param $off i32) (param $b i32) (result i32)
    (if (i32.ge_u (local.get $off) (global.get $WAT_BODY_SZ))
      (then (return (global.get $ERR_NO_MEM)))
    )
    (i32.store8 (i32.add (global.get $OFF_WAT_BODY) (local.get $off)) (local.get $b))
    (i32.store (global.get $OFF_SCRATCH0) (i32.add (local.get $off) (i32.const 1)))
    (return (global.get $OK))
  )

  ;; Emit a LEB128 unsigned integer.
  ;; Output: scratch0 = new offset
  (func $wat_emit_leb_u32 (param $off i32) (param $val i32) (result i32)
    (local $b i32)
    (block $done
      (loop $lp
        (local.set $b (i32.and (local.get $val) (i32.const 0x7F)))
        (local.set $val (i32.shr_u (local.get $val) (i32.const 7)))
        (if (i32.ne (local.get $val) (i32.const 0))
          (then (local.set $b (i32.or (local.get $b) (i32.const 0x80))))
        )
        (if (call $wat_emit_byte (local.get $off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
        (local.set $off (i32.load (global.get $OFF_SCRATCH0)))
        (if (i32.eqz (local.get $val)) (then (br $done)))
        (br $lp)
      )
    )
    (i32.store (global.get $OFF_SCRATCH0) (local.get $off))
    (return (global.get $OK))
  )

  ;; Emit a LEB128 signed integer (32-bit).
  ;; Output: scratch0 = new offset
  (func $wat_emit_leb_i32 (param $off i32) (param $val i32) (result i32)
    (local $b i32) (local $more i32) (local $sign i32)
    (local.set $sign (i32.and (local.get $val) (i32.const 0x40)))  ;; bit 6 of original
    (block $done
      (loop $lp
        (local.set $b (i32.and (local.get $val) (i32.const 0x7F)))
        (local.set $val (i32.shr_s (local.get $val) (i32.const 7)))
        ;; Check if more bytes needed
        (block $check_more
          (if (i32.eq (local.get $val) (i32.const 0))
            (then
              (if (i32.eqz (i32.and (local.get $b) (i32.const 0x40))) (then (br $check_more)))
            )
            (else
              (if (i32.eq (local.get $val) (i32.const -1))
                (then
                  (if (i32.and (local.get $b) (i32.const 0x40)) (then (br $check_more)))
                )
                (else (br $check_more))
              )
            )
          )
          (local.set $more (i32.const 0))
          (br $done)
        )
        (local.set $more (i32.const 1))
        (local.set $b (i32.or (local.get $b) (i32.const 0x80)))
        (if (call $wat_emit_byte (local.get $off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
        (local.set $off (i32.load (global.get $OFF_SCRATCH0)))
        (br $lp)
      )
    )
    (if (call $wat_emit_byte (local.get $off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
    (local.set $off (i32.load (global.get $OFF_SCRATCH0)))
    (i32.store (global.get $OFF_SCRATCH0) (local.get $off))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Compare rest of keyword bytes (after first byte already matched)
  ;; Input: kw_off, start offset (usually 1), count of bytes to compare,
  ;;        11 expected byte values (unused ones should be 0)
  ;; Returns 1 if mismatch, 0 if match
  ;; ═════════════════════════════════════════════════════════════════════
  (func $wat_kw_match_rest (param $kw_off i32) (param $start i32) (param $count i32)
    (param $p0 i32) (param $p1 i32) (param $p2 i32) (param $p3 i32)
    (param $p4 i32) (param $p5 i32) (param $p6 i32) (param $p7 i32)
    (param $p8 i32) (param $p9 i32)
    (result i32)
    (local $base i32) (local $i i32) (local $b i32)
    (local.set $base (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $lp
        (if (i32.ge_u (local.get $i) (local.get $count)) (then (br $done)))
        (local.set $b (i32.load8_u (i32.add (local.get $base) (i32.add (local.get $start) (local.get $i)))))
        (if (i32.eq (local.get $i) (i32.const 0))
          (then (if (i32.ne (local.get $b) (local.get $p0)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 1))
          (then (if (i32.ne (local.get $b) (local.get $p1)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 2))
          (then (if (i32.ne (local.get $b) (local.get $p2)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 3))
          (then (if (i32.ne (local.get $b) (local.get $p3)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 4))
          (then (if (i32.ne (local.get $b) (local.get $p4)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 5))
          (then (if (i32.ne (local.get $b) (local.get $p5)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 6))
          (then (if (i32.ne (local.get $b) (local.get $p6)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 7))
          (then (if (i32.ne (local.get $b) (local.get $p7)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 8))
          (then (if (i32.ne (local.get $b) (local.get $p8)) (then (return (i32.const 1)))))
        )
        (if (i32.eq (local.get $i) (i32.const 9))
          (then (if (i32.ne (local.get $b) (local.get $p9)) (then (return (i32.const 1)))))
        )
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $lp)
      )
    )
    (return (i32.const 0))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Function body opcode parser: reads WAT opcodes, emits WASM bytecodes
  ;; Input: pos = position in WAT source after func header
  ;; Output: scratch0 = decoded_start, scratch1 = decoded_count
  ;; Returns error code.
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Parse one opcode. Returns: 0=continue, -1=done(')'), positive=error
  (func $wat_parse_body (param $pos i32) (param $body_off i32) (result i32)
    (local $err i32) (local $b i32) (local $kw_off i32)
    (local $kw_len i32) (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)
    (local $imm i32)

    (i32.store (i32.const 0x8C060) (local.get $pos))
    (i32.store (i32.const 0x8C080) (local.get $body_off))

    ;; Skip whitespace
    (i32.store (i32.const 0x8C048) (i32.const 0x7001))
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (i32.store (i32.const 0x8C048) (i32.const 0x7002))
    (i32.store (i32.const 0x8C084) (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    ;; Check for end of function: closing paren at top level
    (i32.store (i32.const 0x8C048) (i32.const 0x7003))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x29))  ;; )
      (then
        (i32.store (i32.const 0x8C048) (i32.const 0x7010))
        ;; Emit end opcode
        (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0B))
          (then (return (global.get $ERR_NO_MEM)))
        )
        (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
        ;; Decode the emitted bytecodes
        (i32.store (global.get $OFF_WAT_TMP) (local.get $body_off))
        (i32.store (global.get $OFF_WAT_SAV_PTR) (i32.load (global.get $OFF_WASM_PTR)))
        (i32.store (global.get $OFF_WAT_SAV_LEN) (i32.load (global.get $OFF_WASM_LEN)))
        (i32.store (global.get $OFF_WASM_PTR) (global.get $OFF_WAT_BODY))
        (i32.store (global.get $OFF_WASM_LEN) (local.get $body_off))
        (local.set $err (call $decode_opcodes (i32.const 0) (local.get $body_off)))
        (if (local.get $err) (then (return (local.get $err))))
        (i32.store (global.get $OFF_WASM_PTR) (i32.load (global.get $OFF_WAT_SAV_PTR)))
        (i32.store (global.get $OFF_WASM_LEN) (i32.load (global.get $OFF_WAT_SAV_LEN)))
        (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
        (i32.store (global.get $OFF_SCRATCH1) (i32.load (global.get $OFF_DECODED_COUNT)))
        (local.set $err (call $compute_end_targets
          (local.get $body_off)
          (i32.load (global.get $OFF_DECODED_COUNT))
        ))
        (if (local.get $err) (then (return (local.get $err))))
        (i32.store (global.get $OFF_SCRATCH2) (i32.add (local.get $pos) (i32.const 1)))
        (return (i32.const -1))  ;; DONE
      )
    )

    ;; Check for end of input
    (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                  (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
      (then (return (global.get $ERR_PARSE)))
    )

    ;; Read opcode keyword
    (i32.store (i32.const 0x8C048) (i32.const 0x7040))
    (i32.store (i32.const 0x8C088) (local.get $pos))
    (local.set $err (call $wat_read_kw (local.get $pos)))
    (i32.store (i32.const 0x8C048) (i32.const 0x7041))
    (if (local.get $err) (then (return (global.get $ERR_PARSE))))
    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

      ;; ─── Opcode dispatch ───
      ;; Inline byte comparison: load first few bytes and compare
      (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
      (i32.store (i32.const 0x8C04C) (local.get $kw_len))
      (i32.store (i32.const 0x8C050) (local.get $b0))
      (i32.store (i32.const 0x8C054) (local.get $kw_off))
      (i32.store (i32.const 0x8C058) (i32.load (global.get $OFF_WAT_PTR)))
        (block $not_unreachable
          (if (i32.ne (local.get $kw_len) (i32.const 11)) (then (br $not_unreachable)))
          (if (i32.ne (local.get $b0) (i32.const 0x75)) (then (br $not_unreachable)))  ;; 'u'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 10)
                (i32.const 0x6e) (i32.const 0x72) (i32.const 0x65) (i32.const 0x61)  ;; nrea
                (i32.const 0x63) (i32.const 0x68) (i32.const 0x61) (i32.const 0x62)  ;; chab
                (i32.const 0x6c) (i32.const 0x65))  ;; le
            (then (br $not_unreachable))
          )
          ;; unreachable
          (local.set $b (i32.const 0x00))
          (if (call $wat_emit_byte (local.get $body_off) (local.get $b)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_nop
          (if (i32.ne (local.get $kw_len) (i32.const 3)) (then (br $not_nop)))
          (if (i32.ne (local.get $b0) (i32.const 0x6e)) (then (br $not_nop)))  ;; 'n'
          (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
          (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
          (if (i32.ne (local.get $b1) (i32.const 0x6f)) (then (br $not_nop)))  ;; 'o'
          (if (i32.ne (local.get $b2) (i32.const 0x70)) (then (br $not_nop)))  ;; 'p'
          ;; nop
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x01)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_i32add
          (if (i32.ne (local.get $kw_len) (i32.const 7)) (then (br $not_i32add)))
          (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32add)))  ;; 'i'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 6)
                (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x61)  ;; 32.a
                (i32.const 0x64) (i32.const 0x64) (i32.const 0) (i32.const 0)  ;; dd
                (i32.const 0) (i32.const 0))
            (then (br $not_i32add))
          )
          ;; i32.add
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x6a)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_i32sub
          (if (i32.ne (local.get $kw_len) (i32.const 7)) (then (br $not_i32sub)))
          (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32sub)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 6)
                (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x73)  ;; 32.s
                (i32.const 0x75) (i32.const 0x62) (i32.const 0) (i32.const 0)  ;; ub
                (i32.const 0) (i32.const 0))
            (then (br $not_i32sub))
          )
          ;; i32.sub
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x6b)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_i32const
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_i32const)))
          (if (i32.ne (local.get $b0) (i32.const 0x69)) (then (br $not_i32const)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x33) (i32.const 0x32) (i32.const 0x2e) (i32.const 0x63)  ;; 32.c
                (i32.const 0x6f) (i32.const 0x6e) (i32.const 0x73) (i32.const 0x74)  ;; onst
                (i32.const 0) (i32.const 0))
            (then (br $not_i32const))
          )
          ;; i32.const — read immediate value
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_sint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x41)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_i32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_localget
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_localget)))
          (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_localget)))  ;; 'l'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)  ;; ocal
                (i32.const 0x2e) (i32.const 0x67) (i32.const 0x65) (i32.const 0x74)  ;; .get
                (i32.const 0) (i32.const 0))
            (then (br $not_localget))
          )
          ;; local.get — read immediate local index
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x20)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_localset
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_localset)))
          (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_localset)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)  ;; ocal
                (i32.const 0x2e) (i32.const 0x73) (i32.const 0x65) (i32.const 0x74)  ;; .set
                (i32.const 0) (i32.const 0))
            (then (br $not_localset))
          )
          ;; local.set — read immediate local index
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x21)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_localtee
          (if (i32.ne (local.get $kw_len) (i32.const 9)) (then (br $not_localtee)))
          (if (i32.ne (local.get $b0) (i32.const 0x6c)) (then (br $not_localtee)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 8)
                (i32.const 0x6f) (i32.const 0x63) (i32.const 0x61) (i32.const 0x6c)  ;; ocal
                (i32.const 0x2e) (i32.const 0x74) (i32.const 0x65) (i32.const 0x65)  ;; .tee
                (i32.const 0) (i32.const 0))
            (then (br $not_localtee))
          )
          ;; local.tee — read immediate local index
          (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_read_uint (local.get $pos)) (then (return (global.get $ERR_PARSE))))
          (local.set $imm (i32.load (global.get $OFF_SCRATCH0)))
          (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x22)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (if (call $wat_emit_leb_u32 (local.get $body_off) (local.get $imm)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_return
          (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_return)))
          (if (i32.ne (local.get $b0) (i32.const 0x72)) (then (br $not_return)))  ;; 'r'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                (i32.const 0x65) (i32.const 0x74) (i32.const 0x75) (i32.const 0x72)  ;; etur
                (i32.const 0x6e) (i32.const 0) (i32.const 0) (i32.const 0)  ;; n
                (i32.const 0) (i32.const 0))
            (then (br $not_return))
          )
          ;; return
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0F)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_drop
          (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_drop)))
          (if (i32.ne (local.get $b0) (i32.const 0x64)) (then (br $not_drop)))  ;; 'd'
          (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
          (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
          (local.set $b3 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 3)))))
          (if (i32.ne (local.get $b1) (i32.const 0x72)) (then (br $not_drop)))  ;; 'r'
          (if (i32.ne (local.get $b2) (i32.const 0x6f)) (then (br $not_drop)))  ;; 'o'
          (if (i32.ne (local.get $b3) (i32.const 0x70)) (then (br $not_drop)))  ;; 'p'
          ;; drop
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x1A)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )
        (block $not_end
          (if (i32.ne (local.get $kw_len) (i32.const 3)) (then (br $not_end)))
          (if (i32.ne (local.get $b0) (i32.const 0x65)) (then (br $not_end)))  ;; 'e'
          (local.set $b1 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 1)))))
          (local.set $b2 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.add (local.get $kw_off) (i32.const 2)))))
          (if (i32.ne (local.get $b1) (i32.const 0x6e)) (then (br $not_end)))  ;; 'n'
          (if (i32.ne (local.get $b2) (i32.const 0x64)) (then (br $not_end)))  ;; 'd'
          ;; end — emit end opcode
          (if (call $wat_emit_byte (local.get $body_off) (i32.const 0x0B)) (then (return (global.get $ERR_NO_MEM))))
          (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
          (i32.store (global.get $OFF_SCRATCH0) (local.get $body_off))
          (i32.store (global.get $OFF_SCRATCH1) (local.get $pos))
          (return (i32.const 0))
        )

        ;; Unknown opcode
        (i32.store (i32.const 0x8C048) (i32.const 0x3100))
        (return (global.get $ERR_PARSE))
      )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Top-level declaration parsers
  ;; ═════════════════════════════════════════════════════════════════════

  ;; Parse (type ...) declaration.
  ;; Input: pos after "(type"
  ;; Output: scratch0 = new_pos, scratch1 = type_index
  (func $wat_parse_type_decl (param $pos i32) (result i32)
    (local $err i32) (local $tc i32) (local $base i32) (local $rc i32)
    (local $i i32) (local $vt i32) (local $b i32) (local $kw_off i32) (local $kw_len i32)
    (local $p0 i32) (local $p1 i32) (local $p2 i32) (local $p3 i32)

    ;; Skip optional $name identifier
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x24))  ;; '$'
      (then
        (if (call $wat_read_id (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )

    ;; Expect "("
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    ;; Expect "func"
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
    (if (i32.ne (local.get $kw_len) (i32.const 4))
      (then (return (global.get $ERR_PARSE)))
    )
    (if (i32.ne (local.get $p0) (i32.const 0x66))  ;; 'f'
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $p1 (i32.load8_u (i32.add (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)) (i32.const 1))))
    (local.set $p2 (i32.load8_u (i32.add (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)) (i32.const 2))))
    (local.set $p3 (i32.load8_u (i32.add (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off)) (i32.const 3))))
    (if (i32.ne (local.get $p1) (i32.const 0x75))  ;; 'u'
      (then (return (global.get $ERR_PARSE)))
    )
    (if (i32.ne (local.get $p2) (i32.const 0x6e))  ;; 'n'
      (then (return (global.get $ERR_PARSE)))
    )
    (if (i32.ne (local.get $p3) (i32.const 0x63))  ;; 'c'
      (then (return (global.get $ERR_PARSE)))
    )
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    ;; Get type count
    (local.set $tc (i32.load (global.get $OFF_TYPE_COUNT)))
    (if (i32.ge_u (local.get $tc) (global.get $MAX_TYPES)) (then (return (global.get $ERR_PARSE))))
    (local.set $base (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $tc) (global.get $SZ_TYPE))))

    ;; Store functype marker at offset 0 (not strictly needed but follows binary pattern)
    ;; The interpreter uses SZ_TYPE = 256 with structured fields, not this marker.

    ;; Parse (param ...) and (result ...)
    (local.set $rc (i32.const 0))
    (block $tlp
      (loop $tcont
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
          (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $tlp))
        )
        ;; Expect "("
        (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
          (then (return (global.get $ERR_PARSE)))
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        ;; Read keyword: "param" or "result"
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

        ;; Check if keyword is "param" (5 bytes)
        (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
        (block $not_param
          (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_param)))
          (if (i32.ne (local.get $p0) (i32.const 0x70)) (then (br $not_param)))  ;; 'p'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
                (i32.const 0x61) (i32.const 0x72) (i32.const 0x61) (i32.const 0x6d)  ;; aram
                (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                (i32.const 0) (i32.const 0))
            (then (br $not_param))
          )
          ;; Read param types until ')'
          (block $plp
              (loop $pcont
                (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
                  (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $plp))
                )
                ;; Skip optional $name
                (if (i32.eq (local.get $b) (i32.const 0x24))
                  (then
                    (if (call $wat_read_id (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  )
                )
                ;; Read value type
                (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                  (then (return (global.get $ERR_PARSE)))
                )
                (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
                ;; Store param type
                (if (i32.gt_u (local.get $rc) (i32.const 31)) (then (return (global.get $ERR_PARSE))))
                (i32.store8 (i32.add (local.get $base) (local.get $rc)) (local.get $vt))
                (local.set $rc (i32.add (local.get $rc) (i32.const 1)))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                (br $pcont)
              )
            )
            (br $tcont)
          )

        ;; Check if keyword is "result" (6 bytes)
        (block $not_result
          (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_result)))
          (if (i32.ne (local.get $p0) (i32.const 0x72)) (then (br $not_result)))  ;; 'r'
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                (i32.const 0x65) (i32.const 0x73) (i32.const 0x75) (i32.const 0x6c)  ;; esul
                (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)  ;; t
                (i32.const 0) (i32.const 0))
            (then (br $not_result))
          )
          ;; Read result types until ')'
          (local.set $i (i32.const 0))
            (block $rlp
              (loop $rcont
                (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
                (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
                  (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $rlp))
                )
                ;; Read value type
                (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
                (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                  (then (return (global.get $ERR_PARSE)))
                )
                (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
                ;; Store result type
                (if (i32.gt_u (local.get $i) (i32.const 3)) (then (return (global.get $ERR_PARSE))))
                (i32.store8 (i32.add (local.get $base) (i32.const 132)) (local.get $vt))  ;; result_types at +132
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
                (br $rcont)
              )
            )
            ;; Store result_count at +136
            (i32.store16 (i32.add (local.get $base) (i32.const 136)) (local.get $i))
            ;; Expect closing ')'
            (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
            (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
            (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (if (i32.ne (local.get $b) (i32.const 0x29)) (then (return (global.get $ERR_PARSE))))
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $tlp)
        )
        (return (global.get $ERR_PARSE))
      )
    )

    ;; Store param_count at +128
    (i32.store16 (i32.add (local.get $base) (i32.const 128)) (local.get $rc))

    ;; Increment type count
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (local.get $tc) (i32.const 1)))

    (i32.store (global.get $OFF_SCRATCH0) (local.get $pos))
    (i32.store (global.get $OFF_SCRATCH1) (local.get $tc))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Parse (func ...) declaration (inline)
  ;; Input: pos after "(func"
  ;; Output: scratch0 = new_pos
  ;; ═════════════════════════════════════════════════════════════════════
  (func $wat_parse_func_decl (param $pos i32) (result i32)
    (local $err i32) (local $b i32) (local $kw_off i32) (local $kw_len i32)
    (local $tc i32) (local $base i32) (local $rc i32) (local $i i32) (local $vt i32)
    (local $fc i32) (local $code_i i32)
    (local $export_name_ptr i32) (local $export_name_len i32)
    (local $dst i32) (local $body_len i32)
    (local $has_export i32)
    (local $p0 i32) (local $p1 i32) (local $p2 i32) (local $p3 i32)
    (local $result_count i32)
    (local $decoded_start i32) (local $decoded_count i32)
    (local $body_off i32)

    ;; ── Skip optional $name ──
    (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.eq (local.get $b) (i32.const 0x24))  ;; '$'
      (then
        (if (call $wat_read_id (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
      )
    )

    ;; ── Check for (export "name") ──
    (local.set $has_export (i32.const 0))
    (block $export_check_done
      ;; Look for '('
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
      (if (i32.ne (local.get $b) (i32.const 0x28)) (then (br $export_check_done)))  ;; not '('
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      ;; Read keyword
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
      (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))
      ;; Check "export" (6 bytes)
      (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (return (global.get $ERR_PARSE))))
      (if (i32.ne (local.get $p0) (i32.const 0x65)) (then (return (global.get $ERR_PARSE))))
      (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
            (i32.const 0x78) (i32.const 0x70) (i32.const 0x6f) (i32.const 0x72)
            (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
            (i32.const 0) (i32.const 0))
        (then (return (global.get $ERR_PARSE)))
      )
      (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

      ;; Parse export name string: "name"
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
      (if (i32.ne (local.get $b) (i32.const 0x22)) (then (return (global.get $ERR_PARSE))))  ;; '"'
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      ;; Read name bytes into names buffer
      (local.set $export_name_ptr (i32.load (global.get $OFF_NAMES_PTR)))
      (local.set $export_name_len (i32.const 0))
      (block $ename_done
        (loop $ename_lp
          (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
          (if (i32.eq (local.get $b) (i32.const 0x22))  ;; closing '"'
            (then
              (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
              (br $ename_done)
            )
          )
          (i32.store8 (i32.add (local.get $export_name_ptr) (local.get $export_name_len)) (local.get $b))
          (local.set $export_name_len (i32.add (local.get $export_name_len) (i32.const 1)))
          (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
          (br $ename_lp)
        )
      )
      ;; Advance names buffer pointer
      (i32.store (global.get $OFF_NAMES_PTR) (i32.add (local.get $export_name_ptr) (local.get $export_name_len)))
      ;; Expect ')'
      (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
      (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
      (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
      (if (i32.ne (local.get $b) (i32.const 0x29)) (then (return (global.get $ERR_PARSE))))  ;; ')'
      (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
      (local.set $has_export (i32.const 1))
    )

    ;; ── Parse inline type: (param ...) and (result ...) ──
    (local.set $tc (i32.load (global.get $OFF_TYPE_COUNT)))
    (if (i32.ge_u (local.get $tc) (global.get $MAX_TYPES)) (then (return (global.get $ERR_PARSE))))
    (local.set $base (i32.add (global.get $OFF_TYPES_BUF) (i32.mul (local.get $tc) (global.get $SZ_TYPE))))
    (local.set $rc (i32.const 0))

    (block $type_done
      (loop $type_lp
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        ;; If we see `)` we're done with type declarations, go to body
        (if (i32.eq (local.get $b) (i32.const 0x29)) (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $type_done)))
        ;; Must be '('
        (if (i32.ne (local.get $b) (i32.const 0x28)) (then (br $type_done)))  ;; not '(', assume body starts
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        ;; Read keyword
        (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (local.set $p0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

        ;; "param" (5 bytes)
        (block $not_p
          (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_p)))
          (if (i32.ne (local.get $p0) (i32.const 0x70)) (then (br $not_p)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
                (i32.const 0x61) (i32.const 0x72) (i32.const 0x61) (i32.const 0x6d)
                (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                (i32.const 0) (i32.const 0))
            (then (br $not_p))
          )
          ;; Param types until ')'
          (block $param_done
            (loop $param_lp
              (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
              (if (i32.eq (local.get $b) (i32.const 0x29)) (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $param_done)))  ;; ')'
              ;; Read value type
              (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                (then (return (global.get $ERR_PARSE)))
              )
              (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store8 (i32.add (local.get $base) (local.get $rc)) (local.get $vt))
              (local.set $rc (i32.add (local.get $rc) (i32.const 1)))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
              (br $param_lp)
            )
          )
          (br $type_lp)
        )

        ;; "result" (6 bytes)
        (block $not_r
          (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_r)))
          (if (i32.ne (local.get $p0) (i32.const 0x72)) (then (br $not_r)))
          (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                (i32.const 0x65) (i32.const 0x73) (i32.const 0x75) (i32.const 0x6c)
                (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
                (i32.const 0) (i32.const 0))
            (then (br $not_r))
          )
          ;; Result types until ')'
          (local.set $result_count (i32.const 0))
          (block $res_done
            (loop $res_lp
              (if (call $wat_skip_ws (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
              (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
              (if (i32.eq (local.get $b) (i32.const 0x29)) (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $res_done)))
              ;; Read value type
              (if (call $wat_read_kw (local.get $pos)) (then (return (global.get $ERR_PARSE))))
              (if (call $wat_valtype (i32.load (global.get $OFF_SCRATCH0)) (i32.load (global.get $OFF_SCRATCH1)))
                (then (return (global.get $ERR_PARSE)))
              )
              (local.set $vt (i32.load (global.get $OFF_SCRATCH0)))
              (i32.store8 (i32.add (local.get $base) (i32.const 132)) (local.get $vt))
              (local.set $result_count (i32.add (local.get $result_count) (i32.const 1)))
              (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
              (br $res_lp)
            )
          )
          (i32.store16 (i32.add (local.get $base) (i32.const 136)) (local.get $result_count))
          (br $type_lp)
        )

        ;; Not param or result — unread the open paren and go to body
        (local.set $pos (i32.sub (local.get $pos) (i32.const 1)))
        (br $type_done)
      )
    )

    ;; Store param_count at +128
    (i32.store16 (i32.add (local.get $base) (i32.const 128)) (local.get $rc))
    ;; Increment type count
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (local.get $tc) (i32.const 1)))

    ;; ── Store function entry ──
    (local.set $fc (i32.load (global.get $OFF_FUNCTION_COUNT)))
    (i32.store (i32.add (global.get $OFF_FUNCTIONS_BUF) (i32.mul (local.get $fc) (global.get $SZ_FUNC))) (local.get $tc))

    ;; ── Parse body ──
    (local.set $body_off (i32.const 0))
    (block $body_done
      (loop $body_loop
        (local.set $err (call $wat_parse_body (local.get $pos) (local.get $body_off)))
        (if (i32.lt_s (local.get $err) (i32.const 0)) (then (br $body_done)))  ;; DONE
        (if (local.get $err)
          (then
            (i32.store (global.get $OFF_WAT_DBG) (i32.const 0x2001))
            (i32.store (global.get $OFF_WAT_TMP) (local.get $err))
            (return (global.get $ERR_PARSE))
          )
        )
        ;; CONTINUE
        (local.set $body_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH1)))
        (br $body_loop)
      )
    )
    ;; After $wat_parse_body: scratch0=decoded_start, scratch1=decoded_count
    (local.set $decoded_start (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $decoded_count (i32.load (global.get $OFF_SCRATCH1)))
    (local.set $body_len (i32.load (global.get $OFF_WAT_TMP)))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    ;; ── Store code entry ──
    (local.set $code_i (i32.load (global.get $OFF_CODE_COUNT)))
    (local.set $base (i32.add (global.get $OFF_CODE_BUF) (i32.mul (local.get $code_i) (global.get $SZ_CODE))))
    (i32.store (i32.add (local.get $base) (i32.const 0)) (i32.const 0))  ;; body_offset = 0
    (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $body_len))  ;; body_len
    (i32.store (i32.add (local.get $base) (i32.const 16)) (i32.const 0))  ;; local_count = 0
    (i32.store (i32.add (local.get $base) (i32.const 24)) (local.get $decoded_start))  ;; decoded_start
    (i32.store (i32.add (local.get $base) (i32.const 32)) (local.get $decoded_count))  ;; decoded_count

    ;; ── Store export entry (if export clause present) ──
    (if (local.get $has_export)
      (then
        (local.set $i (i32.load (global.get $OFF_EXPORT_COUNT)))
        (local.set $base (i32.add (global.get $OFF_EXPORTS_BUF) (i32.mul (local.get $i) (global.get $SZ_EXPORT))))
        (i32.store (i32.add (local.get $base) (i32.const 0)) (local.get $export_name_ptr))  ;; name_ptr
        (i32.store (i32.add (local.get $base) (i32.const 8)) (local.get $export_name_len))  ;; name_len
        (i32.store8 (i32.add (local.get $base) (i32.const 16)) (i32.const 0x00))  ;; kind = func
        (i32.store (i32.add (local.get $base) (i32.const 24)) (local.get $fc))  ;; index
        (i32.store (global.get $OFF_EXPORT_COUNT) (i32.add (local.get $i) (i32.const 1)))
      )
    )

    ;; ── Update counters ──
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.add (local.get $fc) (i32.const 1)))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.add (local.get $code_i) (i32.const 1)))

    ;; ── Skip closing ')' of func — $wat_parse_body already consumed it ──
    (i32.store (global.get $OFF_SCRATCH0) (local.get $pos))
    (return (global.get $OK))
  )

  (global $OFF_WAT_DBG i32 (i32.const 0x8C020))

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Main WAT module parser
  ;; Input: WAT source at wasm_ptr (we'll save it), length = wasm_len
  ;; ═════════════════════════════════════════════════════════════════════

  (func $wat_parse_module (param $wat_ptr i32) (param $wat_len i32) (result i32)
    (local $pos i32) (local $err i32) (local $b i32)
    (local $func_i i32) (local $func_cnt i32) (local $type_idx i32)
    (local $code_i i32) (local $code_cnt i32)
    (local $export_i i32) (local $export_cnt i32)
    (local $import_i i32) (local $import_cnt i32)
    (local $global_i i32) (local $global_cnt i32)
    (local $start_func i32)
    (local $kw_off i32) (local $kw_len i32) (local $open_parens i32)
    (local $b0 i32) (local $b1 i32) (local $b2 i32) (local $b3 i32)

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 1))  ;; debug: entered

    ;; Save original wasm_ptr/len
    (i32.store (global.get $OFF_WAT_SAV_PTR) (i32.load (global.get $OFF_WASM_PTR)))
    (i32.store (global.get $OFF_WAT_SAV_LEN) (i32.load (global.get $OFF_WASM_LEN)))

    ;; Store WAT source as the "wasm" pointer temporarily for lexer access
    (i32.store (global.get $OFF_WAT_PTR) (local.get $wat_ptr))
    (i32.store (global.get $OFF_WAT_LEN) (local.get $wat_len))
    (local.set $pos (i32.const 0))

    ;; Clear symbol table
    (i32.store (global.get $OFF_WAT_SYM) (i32.const 0))

    ;; Clear state counters (reset module state fully)
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_IMPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_GLOBAL_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_TABLE_HAS) (i32.const 0))
    (i32.store (global.get $OFF_MEM_MIN) (i32.const 0))
    (i32.store (global.get $OFF_START_FUNC) (i32.const -1))
    (i32.store (global.get $OFF_ELEM_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_DATA_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_DECODED_COUNT) (i32.const 0))
    (i32.store8 (global.get $OFF_EXEC_MOD_VALID) (i32.const 0))

    ;; Reset names pointer
    (i32.store (global.get $OFF_NAMES_PTR) (global.get $OFF_NAMES_BUF))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 2))

    ;; Skip whitespace
    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 10)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 3))

    ;; Expect "("
    (i32.store (global.get $OFF_WAT_TMP0) (i32.load (global.get $OFF_WAT_PTR)))
    (i32.store (global.get $OFF_WAT_TMP1) (local.get $pos))
    (i32.store (global.get $OFF_WAT_TMP1) (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos)))
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (i32.store8 (global.get $OFF_WAT_DBG) (local.get $b))
    (i32.store8 (global.get $OFF_WAT_TMP0) (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.ne (local.get $b) (i32.const 0x28))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 0x1100)) (return (global.get $ERR_PARSE)))
    )
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 4))

    ;; Expect "module"
    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 12)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 5))

    (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 13)) (return (global.get $ERR_PARSE))))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 6))

    (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
    (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
    (if (i32.ne (local.get $kw_len) (i32.const 6))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 14)) (return (global.get $ERR_PARSE)))
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 7))

    (if (i32.ne (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))) (i32.const 0x6D))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 15)) (return (global.get $ERR_PARSE)))  ;; 'm'
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 8))

    (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
          (i32.const 0x6F) (i32.const 0x64) (i32.const 0x75) (i32.const 0x6C)
          (i32.const 0x65) (i32.const 0) (i32.const 0) (i32.const 0)
          (i32.const 0) (i32.const 0))
      (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 16)) (return (global.get $ERR_PARSE)))
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 9))

    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 20))

    ;; ═══ Pass 1: Scan all declarations, register names, count entities ═══
    (block $pass1_done
      (loop $pass1
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 21)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        ;; Closing paren = end of module
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
          (then
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $pass1_done)
          )
        )

        ;; Must be "("
        (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
          (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 22)) (return (global.get $ERR_PARSE)))
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

        ;; Read keyword
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 23)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 24)) (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

        ;; ── Count declarations by keyword ──
        (block $kw_matched
          (block $not_type
            (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_type)))
            (if (i32.ne (local.get $b0) (i32.const 0x74)) (then (br $not_type)))  ;; 't'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                  (i32.const 0x79) (i32.const 0x70) (i32.const 0x65) (i32.const 0)
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_type))
            )
            (i32.store (global.get $OFF_TYPE_COUNT) (i32.add (i32.load (global.get $OFF_TYPE_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
          (block $not_func
            (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_func)))
            (if (i32.ne (local.get $b0) (i32.const 0x66)) (then (br $not_func)))  ;; 'f'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                  (i32.const 0x75) (i32.const 0x6e) (i32.const 0x63) (i32.const 0)
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_func))
            )
            (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.add (i32.load (global.get $OFF_FUNCTION_COUNT)) (i32.const 1)))
            (i32.store (global.get $OFF_CODE_COUNT) (i32.add (i32.load (global.get $OFF_CODE_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
          (block $not_export
            (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_export)))
            (if (i32.ne (local.get $b0) (i32.const 0x65)) (then (br $not_export)))  ;; 'e'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                  (i32.const 0x78) (i32.const 0x70) (i32.const 0x6f) (i32.const 0x72)  ;; xpor
                  (i32.const 0x74) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_export))
            )
            (i32.store (global.get $OFF_EXPORT_COUNT) (i32.add (i32.load (global.get $OFF_EXPORT_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
          (block $not_memory
            (if (i32.ne (local.get $kw_len) (i32.const 6)) (then (br $not_memory)))
            (if (i32.ne (local.get $b0) (i32.const 0x6d)) (then (br $not_memory)))  ;; 'm'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 5)
                  (i32.const 0x65) (i32.const 0x6d) (i32.const 0x6f) (i32.const 0x72)  ;; emor
                  (i32.const 0x79) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_memory))
            )
            (i32.store (global.get $OFF_MEM_MIN) (i32.const 1))
            (br $kw_matched)
          )
          (block $not_start
            (if (i32.ne (local.get $kw_len) (i32.const 5)) (then (br $not_start)))
            (if (i32.ne (local.get $b0) (i32.const 0x73)) (then (br $not_start)))  ;; 's'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 4)
                  (i32.const 0x74) (i32.const 0x61) (i32.const 0x72) (i32.const 0x74)  ;; tart
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_start))
            )
            (br $kw_matched)
          )
          (block $not_data
            (if (i32.ne (local.get $kw_len) (i32.const 4)) (then (br $not_data)))
            (if (i32.ne (local.get $b0) (i32.const 0x64)) (then (br $not_data)))  ;; 'd'
            (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                  (i32.const 0x61) (i32.const 0x74) (i32.const 0x61) (i32.const 0)
                  (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                  (i32.const 0) (i32.const 0))
              (then (br $not_data))
            )
            (i32.store (global.get $OFF_DATA_COUNT) (i32.add (i32.load (global.get $OFF_DATA_COUNT)) (i32.const 1)))
            (br $kw_matched)
          )
        )

        ;; Find matching closing paren and skip entire declaration
        (local.set $open_parens (i32.const 1))
        (block $skip_decl
          (loop $skip_lp
            (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                          (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
              (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 25)) (return (global.get $ERR_PARSE)))
            )
            (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (if (i32.eq (local.get $b) (i32.const 0x28))  ;; '('
              (then (local.set $open_parens (i32.add (local.get $open_parens) (i32.const 1))))
            )
            (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
              (then
                (local.set $open_parens (i32.sub (local.get $open_parens) (i32.const 1)))
                (if (i32.eqz (local.get $open_parens))
                  (then
                    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
                    (br $skip_decl)
                  )
                )
              )
            )
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $skip_lp)
          )
        )
        (br $pass1)
      )
    )

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 30))

    ;; ═══ Pass 2: Parse declarations into state buffers ═══
    (local.set $pos (i32.const 0))
    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 31)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))

    ;; skip "(module"
    (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
    (if (i32.ne (local.get $b) (i32.const 0x28)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 32)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))

    (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 33)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
    (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 34)) (return (global.get $ERR_PARSE))))
    (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))

    ;; Reset decl counters for pass 2
    (i32.store (global.get $OFF_TYPE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_FUNCTION_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_CODE_COUNT) (i32.const 0))
    (i32.store (global.get $OFF_EXPORT_COUNT) (i32.const 0))

    (block $pass2_done
      (loop $pass2
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 35)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
        (if (i32.eq (local.get $b) (i32.const 0x29))  ;; ')'
          (then (br $pass2_done))
        )
        ;; Must be "("
        (if (i32.ne (local.get $b) (i32.const 0x28))  ;; '('
          (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 36)) (return (global.get $ERR_PARSE)))
        )
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        ;; Read keyword
        (if (call $wat_skip_ws (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 37)) (return (global.get $ERR_PARSE))))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
        (if (call $wat_read_kw (local.get $pos)) (then (i32.store (global.get $OFF_WAT_DBG) (i32.const 38)) (return (global.get $ERR_PARSE))))
        (local.set $kw_off (i32.load (global.get $OFF_SCRATCH0)))
        (local.set $kw_len (i32.load (global.get $OFF_SCRATCH1)))
        (local.set $pos (i32.load (global.get $OFF_SCRATCH2)))
        (local.set $b0 (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $kw_off))))

        (block $kw2_fallthrough
          (if (i32.eq (local.get $kw_len) (i32.const 4))
            (then
              (if (i32.eq (local.get $b0) (i32.const 0x74))  ;; 't'
                (then
                  (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                        (i32.const 0x79) (i32.const 0x70) (i32.const 0x65) (i32.const 0)
                        (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                        (i32.const 0) (i32.const 0))
                    (then (br $kw2_fallthrough))
                  )
                  (if (call $wat_parse_type_decl (local.get $pos))
                    (then (return (global.get $ERR_PARSE)))
                  )
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  (br $pass2)
                )
              )
              (if (i32.eq (local.get $b0) (i32.const 0x66))  ;; 'f'
                (then
                  (if (call $wat_kw_match_rest (local.get $kw_off) (i32.const 1) (i32.const 3)
                        (i32.const 0x75) (i32.const 0x6e) (i32.const 0x63) (i32.const 0)
                        (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
                        (i32.const 0) (i32.const 0))
                    (then (br $kw2_fallthrough))
                  )
                  ;; Parse function decl inline
                  (if (call $wat_parse_func_decl (local.get $pos))
                    (then (return (global.get $ERR_PARSE)))
                  )
                  (local.set $pos (i32.load (global.get $OFF_SCRATCH0)))
                  (br $pass2)
                )
              )
            )
          )
        )

        ;; Unknown declaration — skip it
        (local.set $open_parens (i32.const 1))
        (block $skip2
          (loop $skip2_lp
            (if (i32.ge_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))
                          (i32.add (i32.load (global.get $OFF_WAT_PTR)) (i32.load (global.get $OFF_WAT_LEN))))
              (then (return (global.get $ERR_PARSE)))
            )
            (local.set $b (i32.load8_u (i32.add (i32.load (global.get $OFF_WAT_PTR)) (local.get $pos))))
            (if (i32.eq (local.get $b) (i32.const 0x28)) (then (local.set $open_parens (i32.add (local.get $open_parens) (i32.const 1)))))
            (if (i32.eq (local.get $b) (i32.const 0x29))
              (then
                (local.set $open_parens (i32.sub (local.get $open_parens) (i32.const 1)))
                (if (i32.eqz (local.get $open_parens))
                  (then (local.set $pos (i32.add (local.get $pos) (i32.const 1))) (br $skip2))
                )
              )
            )
            (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
            (br $skip2_lp)
          )
        )
        (br $pass2)
      )
    )

    ;; Restore wasm ptr/len
    (i32.store (global.get $OFF_WASM_PTR) (i32.load (global.get $OFF_WAT_SAV_PTR)))
    (i32.store (global.get $OFF_WASM_LEN) (i32.load (global.get $OFF_WAT_SAV_LEN)))

    (i32.store (global.get $OFF_WAT_DBG) (i32.const 99))
    (return (global.get $OK))
  )

  ;; ═════════════════════════════════════════════════════════════════════
  ;; Exported WAT loader: load_wat(wat_ptr, wat_len) -> error_code
  ;; ═════════════════════════════════════════════════════════════════════

  (func $load_wat (export "load_wat") (param $wat_ptr i32) (param $wat_len i32) (result i32)
    (return (call $wat_parse_module (local.get $wat_ptr) (local.get $wat_len)))
  )

    ;; Standard ID removed — merged into single module

  ;; Config (variable length):
  ;;   +0: func_idx  i32  (function index; -1 = function 0)
  ;;   +4: arg_count i32  (number of i32 arguments)
  ;;   +8: args[]    i32  (inline argument values)

  (func (export "process_exec")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len i32) (local $err i32) (local $func_idx i32)
    (local $arg_count i32) (local $arg_ptr i32)
    (local $res_count i32) (local $result i64)

    ;; 1. Read input from pipe into scratch
    (local.set $len (call $pipe_read (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.le_s (local.get $len) (i32.const 0))
      (then (return (local.get $len))))

    ;; 2. Auto-detect: WASM binary or WAT text
    (if (i32.eq (i32.load (local.get $scratch)) (i32.const 0x6D736100))
      (then (local.set $err (call $load (local.get $scratch) (local.get $len))))
      (else (local.set $err (call $load_wat (local.get $scratch) (local.get $len)))))
    (if (local.get $err)
      (then (return (i32.sub (i32.const 0) (local.get $err)))))

    ;; 3. Parse config
    (local.set $func_idx (i32.const 0))
    (local.set $arg_count (i32.const 0))
    (local.set $arg_ptr (i32.const 0))
    (if (i32.ge_s (local.get $clen) (i32.const 4))
      (then
        (local.set $func_idx (i32.load (local.get $cfg)))
        (if (i32.eq (local.get $func_idx) (i32.const -1))
          (then (local.set $func_idx (i32.const 0))))))
    (if (i32.ge_s (local.get $clen) (i32.const 8))
      (then
        (local.set $arg_count (i32.load offset=4 (local.get $cfg)))
        (if (i32.gt_s (local.get $arg_count) (i32.const 0))
          (then
            (local.set $arg_ptr (i32.add (local.get $cfg) (i32.const 8)))))))

    ;; 4. Call function with args
    (local.set $err (call $call (local.get $func_idx) (local.get $arg_ptr) (local.get $arg_count)))
    (if (local.get $err)
      (then (return (i32.sub (i32.const 0) (local.get $err)))))

    ;; 5. Read first result value and write to output
    (local.set $res_count (call $get_result_count))
    (if (i32.gt_s (local.get $res_count) (i32.const 0))
      (then
        (local.set $result (call $get_result_value (i32.const 0)))
        (i32.store (local.get $scratch) (i32.wrap_i64 (local.get $result)))
        (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 4)))
        (return (i32.const 4))))

    ;; No result — return 0 bytes written
    (i32.const 0))
)
