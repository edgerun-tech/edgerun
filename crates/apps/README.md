# Edgerun Apps

This directory contains crates that are shaped as node-mediated apps.

Apps may declare capabilities, routes, storage needs, and other runtime-facing
contracts, but they should not directly own listeners, ports, or host resources.
Those boundaries belong to `edgerun-node`, which is the runtime, IPC, router,
policy enforcement point, signer, admission/work adapter, and local audit writer.

Protocol cores and deterministic message formats belong under
`crates/protocol`. App crates should call those cores through runtime APIs
rather than defining alternate wire or authority paths. Any app request that
crosses a node, relay, capability, storage, execution, or settlement boundary
should converge on `edgerun-work` admission instead of app-local authorization.
