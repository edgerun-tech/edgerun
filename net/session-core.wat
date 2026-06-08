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
