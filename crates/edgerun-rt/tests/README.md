# Test Layer Organization

Tests are organized into dependency layers. Each layer builds on the layers below it.
If a test fails, the fault is in that layer or below — never above.

```
Layer 10: E2E Integration      ← depends on all layers
Layer 9:  Stress / Race        ← depends on layers 1-8
Layer 8:  Higher-Level         ← fs, process, signal, rate_limiter, abort, metrics, shutdown
Layer 7:  Network I/O          ← tcp, udp, unix, unix_dgram
Layer 6:  I/O Primitives       ← async_fd, cursor, duplex, buf, io_util, lines
Layer 5:  Timers               ← sleep, timeout, interval
Layer 4:  Control Flow         ← join!, select!, JoinSet
Layer 3:  Channels             ← oneshot, mpsc, unbounded, watch, broadcast
Layer 2:  Sync Primitives      ← async Mutex, RwLock, Barrier
Layer 1:  Basic Async          ← Notify, Semaphore, CancellationToken, OnceCell, Latch
Layer 0:  Core Runtime         ← waker, ready_queue, task_map, reactor, yield_now
            (src/*_test.rs — pure unit tests, no runtime)
```

## Running Tests

```bash
# Run all tests (layers execute in dependency order naturally)
cargo test

# Run a specific layer
cargo test --test layer0_core/yield_now_test
cargo test --test layer3_channels/mpsc_test

# Run all tests in a layer (use glob pattern with cargo nextest or manually)
cargo test --test "layer1_*"
cargo test --test "layer2_*"
```

## Debugging Strategy

1. **Start at Layer 0** — if these fail, the runtime is fundamentally broken
2. **Move up layer by layer** — each layer validates a specific abstraction
3. **If Layer N fails but N-1 passes** — the bug is in layer N's implementation
4. **Stress tests (Layer 9)** only run after all functional tests pass
5. **E2E (Layer 10)** is the final integration validation

## Adding New Tests

- Place the test in the **lowest layer** that provides all its dependencies
- If a test needs the runtime → Layer 1+
- If a test needs channels → Layer 3+
- If a test needs network → Layer 7+
- If a test validates cross-cutting behavior → Layer 9 or 10
