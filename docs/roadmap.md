# Roadmap

Tracks whitepaper execution order.

## A. Program (Anchor)

- [ ] Global config
- [ ] Worker stake lifecycle
- [ ] Job posting
- [ ] Assignment + stake locking
- [ ] Result submission + Ed25519 verification
- [ ] Finalize + payout + unlock
- [ ] Cancel expired
- [ ] Slash worker

## B. Runtime

- [ ] Canonical CBOR bundle parsing/hash
- [ ] WASM validation (imports + no FP)
- [ ] Hostcalls
- [ ] Limits (memory/instructions)
- [ ] CLI

## C. Worker

- [ ] Key/stake commands
- [ ] Heartbeat/assignments
- [ ] Runtime invocation
- [ ] Chain submit transaction path

## D. Scheduler

- [ ] Worker registry
- [ ] Job create API
- [ ] Chain watcher
- [ ] Assignment/finalization/cancellation

## E. Storage

- [ ] Bundle PUT/GET by hash
- [ ] Immutability controls
