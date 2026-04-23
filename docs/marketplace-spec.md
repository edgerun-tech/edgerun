# Open Compute Marketplace Specification

## Overview

A trustless marketplace for renting compute resources (CPU, RAM, storage, bandwidth) using Edgerun nodes as providers and Solana/USDC for payments.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Solana Blockchain                           │
│                                                             │
│  ┌────────────────────┐    ┌──────────────────────────────┐   │
│  │  ProviderRegistry │    │    DeploymentContract       │   │
│  │  - stake collateral   │    │  - created by buyer      │   │
│  │  - register capacity │   │  - holds deposit       │   │
│  │  - track reputation │    │  - burns per-second     │   │
│  │  - slashing events  │    │  - returns leftover     │   │
│  └────────────────────┘    └──────────────────────────────┘   │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  DAO - Fixed pricing, disputes, governance          │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │ Scheduling + Attestations
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Edgerun Network                         │
│                                                             │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Scheduler - Matches deployments to providers     │    │
│  │  - Reputation-based selection                   │    │
│  │  - Uptime tracking                             │    │
│  └────────────────────────────────────────────────────┘    │
│                                                             │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Node - Executes containers, meters resources       │    │
│  │  - Local capacity tracking                     │    │
│  │  - Attestation publishing                       │    │
│  └────────────────────────────────────────────────────┘    │
└────────────────────────────────────────────���────────────────────┘
```

## Resource Model

### Resources Tracked

| Resource | Unit | Metering | Pricing |
|----------|------|----------|--------|
| CPU | core-hour | `allocated_cores` × time | DAO-set |
| RAM | GiB-hour | `allocated_memory_bytes` × time | DAO-set |
| Storage | GiB-hour | `storage_written_bytes` × time | DAO-set |
| Network | Mbit/s | `network_sent_bytes` / period | DAO-set |

**Note**: Network metering uses per-period aggregation to prevent timing attacks.

## On-Chain Components

### ProviderRegistry

```rust
struct ProviderOffer {
    // Identity
    node_id: Pubkey,          // Edgerun node ID (public key)

    // Collateral (slashed for fraud)
    collateral_staked: u64, // USDC micro-units

    // Capacity offered
    cpu_cores: u32,
    memory_bytes: u64,
    storage_bytes: u64,
    network_mbits: u32,

    // Reputation (updated by protocol)
    total_earnings: u64,     // lifetime USDC earned
    slash_count: u32,         // number of slashings
    uptime_percent: u32,     // 0-10000 (2 decimal precision)

    // Status
    status: ProviderStatus,  // Active | Paused | Slashed
}
```

### DeploymentContract

```rust
struct Deployment {
    // Identity
    owner: Pubkey,           // Buyer who created deployment
    provider: Pubkey,        // Provider running this deployment
    container_id: [u8; 32], // Edgerun container ID

    // Resources requested
    cpu_cores: u32,
    memory_bytes: u64,
    storage_bytes: u64,
    network_mbits: u32,

    // Payment
    deposit: u64,             // USDC deposited
    burn_rate_per_second: u64, // USDC micro-units/sec
    spent: u64,               // Total burned so far

    // State
    status: DeploymentStatus, // Created | Running | Paused | Stopped | Disputed

    // Timestamps
    created_at: i64,
    started_at: Option<i64>,
    paused_at: Option<i64>,
}
```

### DeploymentStatus

```rust
enum DeploymentStatus {
    Created,    // Deployment created, not yet started
    Running,    // Actively running, burning per-second
    Paused,     // Paused (state lost), no burning
    Stopped,    // Fully stopped, leftover being returned
    Disputed,   // Frozen pending DAO resolution
}
```

## ProviderStatus

```rust
enum ProviderStatus {
    Active,   // Accepting deployments
    Paused,   // Not accepting, can resume
    Slashed,  // Slashed, banned from marketplace
}
```

## Lifecycle Flows

### Provider Registration

```
1. Provider stakes collateral (USDC)
2. Provider registers capacity on-chain (cpu, memory, storage, network)
3. Provider enters available pool
4. Reputation starts at 0
```

### Deployment Creation

```
1. Buyer creates DeploymentContract with deposit
2. Contract calculates burn_rate based on resources × DAO prices
3. Buyer starts deployment
4. Scheduler selects provider (reputation-based)
5. Container deployed on provider node
6. Per-second burn begins
7. Provider credited periodically
```

### Pause/Resume

```
1. Buyer pauses deployment
2. State saved (or lost if ephemeral)
3. Burn stops, resources freed
4. Buyer resumes → new container, new burn rate
```

### Stop & Refund

```
1. Buyer stops deployment
2. Contract calculates remaining = deposit - spent
3. Remaining returned to buyer
4. Provider keeps earned amount
5. Deployment closed
```

### Dispute Flow

```
1. Buyer opens dispute on Deployment
2. Funds frozen in contract
3. DAO reviews evidence
4. DAO resolves: refund buyer, slash provider, or split
5. Contract executes DAO decision
```

### Slashing

```
Trigger:
- Invalid attestation (promise != deliver)
- Extended downtime (>1hr without notice)
- Resource limit violation
- Security incident

Process:
1. Attestation fails check
2. On-chain slash event emitted
3. Provider collateral deducted
4. Provider reputation updated
5. DAO receives funds
```

## Reputation System

### Metrics

| Metric | Calculation |
|-------|------------|
| `uptime_percent` | `(running_seconds / total_seconds) × 10000` |
| `total_earnings` | Sum of all USDC earned |
| `slash_count` | Number of slash events |

### Provider Selection

Scheduler selects providers based on:
1. Available capacity for requested resources
2. `uptime_percent` (higher = preferred)
3. `slash_count` (lower = preferred)

## Pricing (DAO-Set)

All prices set by DAO vote:

| Resource | Price (USDC micro-units) |
|----------|------------------------|
| 1 core-hour | 0.01 USDC = 10,000 µ |
| 1 GiB RAM-hour | 0.005 USDC = 5,000 µ |
| 1 GiB storage-hour | 0.001 USDC = 1,000 µ |
| 1 Mbit/s-hour | 0.002 USDC = 2,000 µ |

**Note**: Prices in effect during v0. DAO can propose changes.

## Network Metering

### Implementation: Per-Container Network Namespaces

Linux network namespaces (`netns`) provide accurate per-container bandwidth tracking:

1. Each container gets its own network namespace
2. Traffic flows through veth pairs
3. Bytes sent/received tracked via existing `WorkMeter` (`network_sent_bytes`, `network_received_bytes`)
4. Per-period aggregation prevents timing attacks

**Existing code:**
- `WorkMeter` already tracks: `add_network_sent()`, `add_network_received()`
- Container networking via `NetNamespace` (tested in contest)
- No separate netns needed unless network isolation required

**Metrology:**
- Network metered in 60-second intervals
- Average bandwidth over period
- Burn rate adjusted each period
- Prevents timing attacks via randomized sampling window

## Security Considerations

### Provider Fraud Prevention

| Attack | Mitigation |
|-------|------------|
| Overcommit resources | Local capacity enforcement + attestations |
| Stop providing mid-run | Slashing + reputation hit |
| Fake attestations | On-chain proofs + verification |
| Data breach | Provider security audit (off-chain) |

### Buyer Abuse Prevention

| Attack | Mitigation |
|-------|------------|
| Run zero-day exploits | Image allowlist per provider |
| Resource hogging | Hard limits in contract |
| Duplicate deployments | One per buyer contract |

## Implementation Phases

### Phase 1: Registry + Basic Deployment
- ProviderRegistry on Solana
- DeploymentContract with deposit/burn
- Off-chain scheduler
- Manual pricing (fixed)

### Phase 2: Reputation + Slashing
- Uptime tracking
- Automatic slashing
- DAO dispute resolution

### Phase 3: Market Features
- Provider reputation scores
- Network metering
- Auto-scaling (future)

## Reference Implementation

See existing code:
- `crates/edgerun-node/src/capacity.rs` - Node capacity tracking
- `crates/edgerun-node/src/metering.rs` - Work metering with finalize()
- `crates/edgerun-config/src/types.rs` - Resource kinds (Container, Service, Gateway, HttpRoute)

## Appendix: Key Constants

```rust
const COLLATERAL_PER_CORE: u64 = 100_000_000;      // 100 USDC
const COLLATERAL_PER_GIB_RAM: u64 = 50_000_000; // 50 USDC
const COLLATERAL_PER_GIB_STORAGE: u64 = 10_000_000; // 10 USDC
const COLLATERAL_PER_MBIT: u64 = 5_000_000;       // 5 USDC

const BURN_INTERVAL_SECONDS: u64 = 1;           // Burn every second
const ATTESTATION_INTERVAL_SECONDS: u64 = 3600;   // Attest every hour
const SLASH_WINDOW_SECONDS: u64 = 86400;         // 24 hour window
const MAX_PAUSE_DURATION_SECONDS: u64 = 604800; // 7 days max pause
```