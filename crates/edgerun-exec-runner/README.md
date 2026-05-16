# edgerun-exec-runner

Boot-time Linux ELF runner for EdgeRun development.

It intentionally does not sandbox. It receives executable bytes over a framed byte stream, verifies SHA-256, and runs the executable while streaming stdout/stderr and exit status back as frames.

There are two binaries:

```text
edgerun-exec-runner  transient receive-and-run executor
edgerun-exec-cas     content-addressed RAM blob store + execute-by-hash
```

The transport is stdin/stdout first so it can be wrapped by serial, TCP, RFCOMM, BLE, vsock, or an initramfs console without changing the executor.

## Build

```sh
cd crates/edgerun-exec-runner
cargo build --release
```

The crate is standalone and has no external Rust dependencies.

`edgerun-exec-cas` currently shells out to `sha256sum` when validating stored blobs. In an initramfs, include BusyBox/coreutils `sha256sum` or replace it later with the in-tree SHA-256 helper.

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

Transient receive-and-run:

```sh
python3 tools/send-elf.py target/release/edgerun-exec-runner /tmp/hello arg1 arg2
```

Content-addressed store and run:

```sh
python3 tools/send-cas.py target/release/edgerun-exec-cas put-run /tmp/hello arg1 arg2
```

Store only:

```sh
python3 tools/send-cas.py target/release/edgerun-exec-cas put /tmp/hello
```

Run by hash requires the client to know the same ELF bytes. The test client computes the hash from the ELF and sends an EXEC frame for that hash:

```sh
python3 tools/send-cas.py target/release/edgerun-exec-cas run /tmp/hello arg1 arg2
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

## Content-addressed storage

`edgerun-exec-cas` stores blobs in RAM under:

```text
/run/edgerun/blobs/sha256/<first-2>/<next-2>/<full-sha256-hex>
```

Jobs get temporary working directories under:

```text
/run/edgerun/jobs/
```

The runner supports:

```text
PUT_BEGIN / PUT_CHUNK / PUT_END   store blob by expected SHA-256
EXEC                              execute existing blob by SHA-256
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

Common message types:

```text
1  HELLO
10 STDOUT
11 STDERR
12 EXIT
13 ERROR
14 LOG
```

Transient message types:

```text
2  EXEC_BEGIN
3  EXEC_CHUNK
4  EXEC_END
```

CAS message types:

```text
20 PUT_BEGIN
21 PUT_CHUNK
22 PUT_END
23 PUT_OK
24 EXEC
25 BLOB_EXISTS
```

`PUT_BEGIN` payload:

```text
sha256      32 bytes
size        u64
mode        u32, usually 0700
```

`EXEC` payload:

```text
sha256      32 bytes
timeout_ms  u64, 0 = no timeout
argc        u16
envc        u16
argv[]      repeated length-prefixed UTF-8 strings
env[]       repeated key/value length-prefixed UTF-8 strings
```

## Using as init/PID 1

Boot the current Linux kernel with one of:

```text
init=/usr/local/bin/edgerun-exec-runner
init=/usr/local/bin/edgerun-exec-cas
```

For initramfs use, place the chosen binary as `/init` or call it from `/init` after mounting `/proc`, `/sys`, `/dev`, and `/run`. The runners attempt to mount those themselves quietly, but a real initramfs should still mount them explicitly.

## Transport wrappers

The executor speaks only framed bytes on stdin/stdout. To use Bluetooth, run it behind a byte transport wrapper:

```text
Bluetooth RFCOMM / BLE / serial / TCP / vsock
  <-> framed byte stream
  <-> edgerun-exec-cas
```

For development, test over stdin/stdout first, then wrap it with RFCOMM or vsock.
