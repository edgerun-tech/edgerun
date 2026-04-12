# RFC-0004: Networking & Mesh

**Status:** Working Draft
**Date:** 2026-04-12
**Based on:** Protocol spec §9, §10.2, §14.23-14.27

---

## Abstract

The networking layer implements identity-first, transport-agnostic communication with sessions, routing, reachability hints, relay envelopes, and mesh networking. It maps to protocol spec sections 9 (Networking Model) and 14.23-14.27.

---

## Protocol Spec Networking Model (§9)

### Key Principles

1. **Identity-first, transport-agnostic** — Nodes addressed by identity, not IP
2. **Locators are transport-specific** — BLE, QUIC, WiFi Direct, Relay, Store-Forward
3. **Sessions authenticate identities** — Not merely sockets
4. **Routing is local** — Route is a local plan using known locators, relays, cost, policy
5. **Control/Data/Discovery planes** — Different traffic types on different channels
6. **Receiver-driven pattern** — Announce → Express interest → Fetch body → Query later

### Session Flow (§14.25-14.26)

```
Initiator → SessionHello (identity, protocol versions, nonce, locators)
Responder → SessionAccept (echoed nonce, selected version, locators)
```

### Route Selection (§8.3)

Normalized route score = assignment trust score + reachability quality hint - cost penalty

Tie-breaking: highest score → preferred advertiser → preferred next-hop → earliest timestamp → lexicographically smallest next-hop

---

## Components

### edgerun-storage (Peer Management)

**Implemented:**
- `upsert_peer(node_id_hex, addr, status, is_bootstrap)`
- `update_peer_status(node_id_hex, status)`
- `list_peers()` — All peers with status
- `list_unreachable_peers_with_addr()` — Unreachable subset

### edgerun-network-interface

**Status:** ✅ Functional
**Purpose:** Network interface abstraction — common interface for all network backends.

### edgerun-linux-netif

**Status:** ✅ Functional
**Purpose:** Linux network interface discovery via netlink/sysfs. Lists interfaces, addresses, link state.

### Mesh Networking Stack

| Crate | Status | Purpose |
|-------|--------|---------|
| `edgerun-mesh` | ✅ Functional (180 tests) | Mesh networking protocol definition, routing table, discovery, peer management |
| `edgerun-mesh-link` | ✅ Functional (82 tests) | Raw Ethernet (AF_PACKET), multicast UDP, IP tunnel, UDP broadcast |
| `edgerun-mesh-capability` | ✅ Functional (66 tests) | Capability exchange over mesh (transport, server, client, dispatcher, inbox) |
| `edgerun-mesh-daemon` | ✅ Functional (34 tests) | Event loop: interface discovery, heartbeat, ECDH handshakes, session encryption, outbound queue |
| `edgerun-mesh-session` | ✅ Functional (67 tests) | ECDH handshakes, AES-GCM session encryption, replay protection, rekey support |

### edgerun-dns / edgerun-dhcp / edgerun-dhcpv6

| Crate | Status | Purpose |
|-------|--------|---------|
| `edgerun-dns` | ⚠️ Partial | DNS resolution |
| `edgerun-dhcp` | ⚠️ Partial | DHCP client (IPv4) |
| `edgerun-dhcpv6` | ⚠️ Partial | DHCPv6 client |

### edgerun-tftp

**Status:** ⚠️ Partial
**Purpose:** TFTP protocol — likely for bootstrapping/network boot scenarios.

---

## Spec Mapping

| Spec Section | Implementation | Status |
|---|---|---|
| §9 Networking Model | Identity-first, transport-agnostic | ✅ Conceptual |
| §9.1 Receiver-driven Transport | — | ❌ Not implemented |
| §10.2 Discovery & Session | — | ❌ Not implemented |
| §14.23 ReachabilityHint | Proto type only | ❌ |
| §14.24 RouteAdvertisement | Proto type only | ❌ |
| §14.25 SessionHello | Proto type only | ❌ |
| §14.26 SessionAccept | Proto type only | ❌ |
| §14.27 RelayEnvelope | Proto type only | ❌ |
| §18.3 Ingress Screening | — | ❌ Not implemented |

---

## Outstanding Work

1. **Session hello/accept handling** — Full session establishment flow
2. **Route trust evaluation** — Hard constraints before scoring, normalized score computation
3. **Reachability hint management** — Advertising and consuming reachability hints
4. **Relay envelope processing** — Store-and-forward relay handling
5. **Mesh networking implementation** — All 5 mesh crates are partial/skeleton
6. **Control/data/discovery plane separation** — Traffic classification
7. **Transport binding** — Actual BLE/QUIC/WiFi Direct/Relay implementations
