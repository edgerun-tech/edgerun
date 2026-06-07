(module
  ;; ── Host imports ──────────────────────────────────────────────
  (import "host" "sock_open"  (func $sock_open  (param i32 i32 i32) (result i32)))
  (import "host" "sock_send"  (func $sock_send  (param i32 i32 i32) (result i32)))
  (import "host" "sock_recv"  (func $sock_recv  (param i32 i32 i32) (result i32)))
  (import "host" "sock_close" (func $sock_close (param i32) (result i32)))
