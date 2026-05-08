# Edgerun Apps

This directory contains crates that are shaped as node-mediated apps.

Apps may declare capabilities, routes, storage needs, and other runtime-facing
contracts, but they should not directly own listeners, ports, or host resources.
Those boundaries belong to `edgerun-node`, which is the runtime, IPC, router,
policy enforcement point, signer, and event-log writer.

Protocol cores and deterministic message formats belong under
`crates/protocol`. App crates should call those cores through runtime APIs
rather than defining alternate wire paths.
