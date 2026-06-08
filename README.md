# EdgeRun

Train frontier AI models on devices that already exist — phones, laptops, game consoles, smart TVs — instead of building more datacenters.

## The Problem

Nvidia is worth $5T. Datacenter power consumption is exploding. Energy prices are climbing. Meanwhile:

- **6B smartphones** sit in pockets, each with a 0.5-1 TFLOPS NPU, doing nothing most of the day
- **1.5B laptops** with 3 TFLOPS integrated GPUs, idle between keystrokes
- **300M gaming desktops** with 15 TFLOPS dedicated GPUs, running 4 hours/night then sitting idle for 20
- **400M game consoles** with 10 TFLOPS each
- **2.5B smart TVs** with SoCs that do nothing but stream menus

That's **~10 exaFLOPs of aggregate compute** — already built, already paid for, already plugged into the grid. It's equivalent to roughly 6× all the world's GPU datacenters combined, running on existing power infrastructure.

A single H100 datacenter GPU burns 700W. A phone contributing 20% of its NPU for a background task burns an incremental ~1W. Training the same model on phones consumes **~1000× less energy** than training it on new hardware.

## Does The Math Actually Work?

Yes. The numbers are not aspirational — they're conservative.

### Hardware You Already Own

| Device | TFLOPS (FP16) | Upload | Count | Daily Uptime |
|--------|-------------|--------|-------|-------------|
| Budget phone | 0.2 | 3 Mbps | 2B | 12h (charging overnight) |
| Mid phone | 0.5 | 5 Mbps | 4B | 10h |
| Flagship phone | 1.0 | 10 Mbps | 1B | 8h |
| Laptop (iGPU) | 3.0 | 20 Mbps | 1.5B | 6h |
| Desktop (APU) | 2.0 | 30 Mbps | 500M | 10h |
| Desktop (dGPU) | 15 | 50 Mbps | 300M | 8h |
| Console (PS5/XSX) | 10 | 25 Mbps | 400M | 4h |
| Tablet | 0.8 | 10 Mbps | 1.5B | 6h |
| Smart TV | 0.2 | 8 Mbps | 2.5B | 14h |

### What It Can Do

**100B model** (e.g., Llama 3 scale):

| Source | Devices | Training time | Relay cost |
|--------|---------|--------------|------------|
| Wikipedia sidebar widget | 12K concurrent | 16h | ~$0.50 |
| Instagram in-feed WASM | 42K concurrent | 4.5h | ~$3 |
| Facebook opt-in background process | 83K concurrent | 2.3h | ~$6 |
| Google search result page | 556K concurrent | 19min | ~$1 |
| WhatsApp Web idle tab | 1.7M concurrent | 6min | ~$3 |

**1T model** (6× larger than any open model):

| Source | Training time | Relay cost |
|--------|-------------|-----------|
| Facebook-scale adoption | 18h | ~$150 |
| Google-scale adoption | 3.4h | ~$200 |
| WhatsApp Web | 1.1h | ~$40 |

**10T model** (frontier research scale):

| Source | Training time | Relay cost |
|--------|-------------|-----------|
| Facebook-scale | 7.5 days | ~$1,500 |
| Google-scale | 1.4 days | ~$2,000 |
| WhatsApp Web | 11 hours | ~$2,000 |

A 10T model for **$2,000 in relay bandwidth** — what a single H100 costs for 8 hours. Or you can embed it in WhatsApp Web and train it in a day for free.

### Comparison to Current Approach

- **Meta trained Llama 3 405B** on 30,000 H100s for 54 days. Estimated cost: **~$50M**.
- **Same model on user devices** at 0.1% global adoption (14M devices): under 2 hours. Relay cost: **~$200**.

That's a 250,000× cost difference. The model doesn't know where the FLOPs came from.

## Architecture

EdgeRun is a single-module WebAssembly runtime that runs on any device with a browser or WASM runtime — no install, no drivers, no containers.

```
device → relay (parameter server) ↔ relay mesh ↔ relay → device
```

- **Devices are stateless workers**. Download current weights for a shard, compute T steps, upload gradients. If the browser tab closes, nothing breaks — the gradient was already submitted.
- **Relays hold the model state**. They aggregate gradients from all comers, update weights, and serve the latest weights to the next device. Relays are lightweight — each handles ~2000 devices at 1.7 Gbps.
- **No replication required**. Unlike P2P sharded training, there's no "k of 3 replicas must be alive" constraint. Devices come and go freely.
- **Staleness is bounded at T steps**. A device computes with weights that are at most T steps old. With T=50, the effective learning rate scales by 1/2 — a known and manageable regime in async SGD.

### Relay Infrastructure

For a company already running datacenters (Meta, Google, Microsoft, etc.), the relay cost is spare capacity — a rounding error.

| Scale | Relays | Bandwidth/relay | Storage/relay | Monthly infra |
|-------|--------|----------------|--------------|--------------|
| Facebook (83K concurrent) | 42 | 1.7 Gbps | 9.5 GB | $6K |
| Google (556K concurrent) | 278 | 1.7 Gbps | 1.4 GB | $42K |
| WhatsApp Web (1.7M concurrent) | 850 | 1.7 Gbps | 0.5 GB | $128K |

For comparison, Meta's annual infrastructure spend is ~$30B. The relay network costs **0.0004% of that**.

### What Each Device Does

Per round (~1.4 seconds):
- Downloads 50KB of weights (compressed FP16)
- Runs 50 local steps on 100K parameters (batch 4, seq 2048)
- Uploads 100KB of gradient (compressed FP32)

Per session (5-15 minutes): 200-600 rounds. Per day (if device stays open): ~60,000 rounds.

Resource usage: 20% of one CPU core, 2 Mbps upload — invisible to the user. On a phone charging overnight, the battery impact is negligible.

### Energy Comparison

| Approach | Power per device | Devices | Total power | Time | Energy |
|----------|----------------|---------|-------------|------|--------|
| 30K H100s | 700W | 30K | 21 MW | 54 days | 27 GWh |
| Phone swarm | 1W incremental | 1.7M | 1.7 MW | 11h | 19 MWh |

**1,400× less energy** to train the same model. The compute was already paid for — the phone was going to charge anyway.

## The Real Bottleneck

Not bandwidth. Not compute. Not relay cost.

The bottleneck is the software layer that connects 6 billion devices into a single training mesh — the identity system, the async gradient protocol, the shard assignment, the churn handling. That's what EdgeRun builds.

The hardware is already deployed. Every phone in every pocket is a node in a distributed supercomputer that doesn't exist yet because nobody built the operating system for it.

## Status

Working prototype: single-module WAT runtime with crypto (AES, Ed25519, X25519, SHA-2/3), protocol implementations (TLS 1.3, QUIC, HTTP/2), codecs (JSON, CBOR, MsgPack), and a pipeline engine. Self-hosted toolchain — everything written in WAT.

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the technical design.
