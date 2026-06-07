(module
  ;; ── Host imports (provided by er runtime, or JIT-compiled to syscalls) ──
  (import "host" "sock_open"  (func $sock_open  (param i32 i32 i32) (result i32)))
  (import "host" "sock_send"  (func $sock_send  (param i32 i32 i32) (result i32)))
  (import "host" "sock_recv"  (func $sock_recv  (param i32 i32 i32) (result i32)))
  (import "host" "sock_close" (func $sock_close (param i32) (result i32)))
  ;; ── Linux syscall imports (for JIT-compiled Wayland client ELF) ──────
  ;; These are always compiled to inline syscall by the JIT.
  ;; The host provides no-op stubs (never called in interpreter mode).
  (import "linux" "poll"         (func $sys_poll         (param i32 i32 i32) (result i32)))
  (import "linux" "mmap"         (func $sys_mmap         (param i32 i32 i32 i32 i32 i32) (result i32)))
  (import "linux" "munmap"       (func $sys_munmap       (param i32 i32) (result i32)))
  (import "linux" "socket"       (func $sys_socket       (param i32 i32 i32) (result i32)))
  (import "linux" "connect"      (func $sys_connect      (param i32 i32 i32) (result i32)))
  (import "linux" "sendmsg"      (func $sys_sendmsg      (param i32 i32 i32) (result i32)))
  (import "linux" "memfd_create" (func $sys_memfd_create (param i32 i32) (result i32)))
  (import "linux" "ftruncate"    (func $sys_ftruncate    (param i32 i32) (result i32)))
