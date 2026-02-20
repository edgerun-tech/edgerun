<!-- SPDX-License-Identifier: GPL-2.0-only -->
# Developer CLI

`tools/devctl.sh` provides a unified local command surface.

Examples:

```bash
tools/devctl.sh check
tools/devctl.sh test
tools/devctl.sh perf-gate
tools/devctl.sh sweep --duration 8
tools/devctl.sh crash --iterations 1000 --data-dir /tmp/crash --target-mb 2 --random true
tools/devctl.sh rep-bench --mode batch --events 20000 --batch-size 128
tools/devctl.sh enc-demo --provider passphrase --passphrase "dev-secret" --events 1000
tools/devctl.sh enc-demo --provider passphrase --passphrase "dev-secret" --events 1000 --skip-verify
tools/devctl.sh ci-smoke
```

`enc-demo` verifies encrypted transport integrity and decrypt/readback by default.
