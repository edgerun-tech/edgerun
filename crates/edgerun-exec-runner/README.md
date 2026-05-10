# edgerun-exec-runner

Boot-time Linux ELF runner for EdgeRun development.

It intentionally does not sandbox. It receives an ELF over a framed byte stream, verifies SHA-256, writes it to `/run/edgerun/jobs/<job>/program`, executes it, streams stdout/stderr back as frames, returns the exit code, deletes the job directory, and waits for the next executable.

The transport is stdin/stdout first so it can be wrapped by serial, TCP, RFCOMM, BLE, vsock, or an initramfs console without changing the executor.

## Build

```sh
cd crates/edgerun-exec-runner
cargo build --release
```

The crate is standalone and has no external dependencies.

## Test locally

Create a test ELF:

```sh
cat > /tmp/hello.c <<'EOF'
#include <stdio.h>
#include <unistd.h>

int main(int argc, char **argv) {
  printf("hello from remote ELF\\n");
  fprintf(stderr, "stderr works too\\n");
  printf("argc=%d\\n", argc);
  for (int i = 0; i < argc; i++) printf("argv[%d]=%s\\n", i, argv[i]);
  return 42;
}
EOF
cc -O2 -static -o /tmp/hello /tmp/hello.c 2>/dev/null || cc -O2 -o /tmp/hello /tmp/hello.c
```

Run through the framed protocol:

```sh
python3 tools/send-elf.py target/release/edgerun-exec-runner /tmp/hello arg1 arg2
```

Expected output includes:

```text
hello from remote ELF
argc=3
argv[0]=./program
argv[1]=arg1
argv[2]=arg2
[runner:exit] 42
```

## Protocol

Frame header is 24 bytes:

```text
magic      4 bytes  "ERXR"
version    u16      1
msg_type   u16
seq        u64
len        u32
reserved   u32      0
payload    len bytes
```

Message types:

```text
1  HELLO
2  EXEC_BEGIN
3  EXEC_CHUNK
4  EXEC_END
10 STDOUT
11 STDERR
12 EXIT
13 ERROR
14 LOG
```

`EXEC_BEGIN` payload:

```text
job_id      32 bytes
size        u64
sha256      32 bytes
timeout_ms  u64, 0 = no timeout
argc        u16
envc        u16
argv[]      repeated length-prefixed UTF-8 strings
env[]       repeated key/value length-prefixed UTF-8 strings
```

`EXEC_CHUNK` payload is raw executable bytes. `EXEC_END` has empty payload.

## Using as init/PID 1

Boot the current Linux kernel with:

```text
init=/usr/local/bin/edgerun-exec-runner
```

For initramfs use, place the binary as `/init` or call it from `/init` after mounting `/proc`, `/sys`, `/dev`, and `/run`. The runner attempts to mount those itself quietly, but a real initramfs should still mount them explicitly.

## Transport wrappers

The executor speaks only framed bytes on stdin/stdout. To use Bluetooth, run it behind a byte transport wrapper:

```text
Bluetooth RFCOMM / BLE / serial / TCP / vsock
  <-> framed byte stream
  <-> edgerun-exec-runner
```

For development, test over stdin/stdout first, then wrap it with RFCOMM or vsock.
