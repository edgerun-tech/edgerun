# EdgeRun Bootstrap Provisioning

## Pivot Explanation

EdgeRun is temporarily operator-coordinated, not permanently centralized.
Nodes connect to the bootstrap coordinator for visibility, scheduling, routing hints, and debugging.
Authority remains in signed node streams.

**Core Principle**: Bootstrap can be centralized for coordination without centralizing authority.

- The coordinator **observes and assists** — it does not own.
- The node stream **remains authoritative**.
- Controller authority comes from the node genesis accepting a controller-signed provisioning contract.
- The node private key is **never baked into the binary**.

## Trust Chain

```
Controller signature proves:
  "I authorized this build/config to bootstrap under my control."

Node signature proves:
  "This newly generated node identity owns this genesis stream."

Genesis event proves:
  "This node began life under this provisioning contract."

Controller acceptance proves:
  "The controller recognizes this node identity as the result of that contract."

Coordinator proves nothing by itself. It only transports/observerves.
```

## Provisioning Model

**One provisioning contract = one intended node.**

Single-use only by default because the contract contains node-specific settings:

- node label/nickname
- intended role
- scheduler tags
- bootstrap coordinator endpoint
- networking profile
- region/location hint
- resource advertisement policy
- controller identity
- initial controller capability basis
- expected hardware/assurance policy
- build artifact hash
- config hash
- expiry

**Do not implement multi-use provisioning in this path.**

If multi-use is ever needed later, create a separate `ProvisioningTemplate` concept that generates many single-use contracts.

## Terminology

**Use:**
- `bootstrap_coordinator`
- `coordination_node`
- `controller`
- `provisioning_contract`
- `node_genesis_claim`
- `controller_acceptance`
- `authority_source`
- `observed_by`
- `reported_by`

**Avoid:**
- `master`
- `central_authority`
- `root_node`
- `global_truth`
- `owner_server`

## State Machines

### Provisioning Contract Status
```
ISSUED → BAKED_INTO_ARTIFACT → FIRST_BOOT_CLAIM_RECEIVED → ACCEPTED
                                                          → REJECTED
                                                          → REPLAY_REJECTED (if consumed)
                                                          → CONSUMED
```

### Node Provisioning States
```
PENDING → CLAIM_SENT → ACCEPTED (controller accepted)
                   → REJECTED (controller rejected)
                   → REPLAY_REJECTED (contract already consumed)
```

## Flow

### Build Time
1. Controller creates a single-use `ProvisioningContract`.
2. Contract includes controller identity, node label, coordinator endpoint, config hash, build hash, initial policy, expiry, and use policy.
3. Controller signs the contract.
4. Contract is baked into the node binary/config artifact.
5. **No node private key is baked in.**

### First Boot
1. Node loads baked `ProvisioningContract`.
2. Node verifies controller signature if controller public identity is available in the contract/artifact.
3. Node generates or loads a local hardware-backed/software keypair.
4. Node creates genesis event with:
   - node identity
   - controller identity
   - provisioning contract hash
   - node label
   - node role/tags
   - config hash
   - build artifact hash
   - coordinator hint
   - initial controller capability basis
5. Node signs genesis event with node key.
6. Node creates `NodeGenesisClaim`.
7. Node sends `ProvisioningContract` + `NodeGenesisClaim` + genesis event to bootstrap coordinator/controller.
8. Controller verifies:
   - contract signature
   - contract expiry
   - contract not revoked
   - provisioning_id not consumed
   - contract is single-node/single-use
   - genesis commits to contract hash
   - node signature validates
   - build/config hash acceptable
   - optional hardware/assurance constraints
9. Controller records accepted/rejected decision.
10. Node records controller acceptance if accepted.

## Hard Invariants

- **The coordinator is not global truth.**
- **The coordinator is not the trust root.**
- **The coordinator does not own node-local state.**
- **Command delivery is not authority.**
- **Scheduler assignment is not execution proof.**
- **Route hints are advisory.**
- **Capability advertisements are not automatic grants.**
- **Node state is authoritative only through the node's signed stream.**
- **Controller authority comes from the node genesis accepting a controller-signed provisioning contract.**
- **The node private key is never baked into the binary.**

## Node Stream Events

- `NODE_GENESIS_CREATED`
- `PROVISIONING_CLAIM_SENT`
- `CONTROLLER_ACCEPTANCE_OBSERVED`
- `CONTROLLER_REJECTION_OBSERVED`
- `PROVISIONING_REPLAY_WARNING` (if applicable)

## Controller Stream/Events

- `PROVISIONING_CONTRACT_ISSUED`
- `NODE_GENESIS_CLAIM_RECEIVED`
- `NODE_PROVISIONING_ACCEPTED`
- `NODE_PROVISIONING_REJECTED`
- `PROVISIONING_REPLAY_REJECTED`
- `PROVISIONING_CONTRACT_REVOKED`

## Failure Cases

### Contract Expired
- Node rejects contract on first boot if expired.
- Controller rejects claim if contract expired.

### Contract Reused (Replay)
- Controller rejects second claim for same `provisioning_id`.
- Status: `REPLAY_REJECTED`.
- Node records warning in stream.

### Node Genesis Does Not Commit to Contract
- Controller rejects if genesis event does not include contract hash.
- Controller rejects if genesis event contract hash does not match.

### Node Signature Invalid
- Controller rejects if node signature on genesis claim does not validate.

### Controller Signature Invalid
- Node warns on first boot but may continue (depending on policy).
- Node records warning in stream.

### Config Hash Mismatch
- Controller rejects if actual config hash does not match contract.
- Controller may accept with warning if policy allows.

### Build Hash Mismatch
- Controller rejects if actual build hash does not match contract.
- Critical for supply chain integrity.

### Claim Arrives After Consumed
- Controller returns `REPLAY_REJECTED`.
- Node must generate new identity or use different contract.

### Coordinator Relays Stale/Duplicate Claim
- Controller deduplicates by `provisioning_id` + `node_identity`.
- Controller processes only first valid claim per contract.

### Node Boots Offline and Queues Claim
- Node stores claim locally.
- Node retries sending to coordinator when connectivity restored.
- Claim includes original timestamp.

### Controller Rejects Claim
- Controller records `NODE_PROVISIONING_REJECTED` with reason.
- Node records `CONTROLLER_REJECTION_OBSERVED`.
- Node may retry with different identity if policy allows.

### Controller Accepts but Node Does Not Observe Acceptance
- Node retries polling coordinator.
- Coordinator maintains acceptance state for retry.
- Eventually consistent within coordinator timeout.

## Dashboard Implications

The dashboard must distinguish:
- **authoritative** node stream state
- **coordinator** observation
- **scheduler** assignment (not execution proof)
- **route** hint (advisory)
- **cached** UI state
- **agent** claim (not fact)
- **unknown** state

### Node Card Should Show
```
Node: n1-pattaya
Connection: online via bootstrap coordinator
Provisioning: consumed / accepted
Node identity: node_...
Stream head: seq 184 / hash ...
Authority source: node stream
Coordinator observation: reachable 8s ago
CPU: 32 cores · 67%
Memory: 84/128 GB
Jobs: 3 running
```

### Job/Scheduling UI Must Show
- assigned by coordinator (advisory)
- accepted by worker (authoritative after stream commit)
- started event (authoritative)
- completed/failed event (authoritative)
- verification status

**Do not show assignment as execution.**

### Resource Aggregation
A user can own multiple nodes. Dashboard must show aggregate resources and allow splitting by node.

Aggregate:
- total cores
- weighted CPU utilization: `sum(node_cores * node_utilization) / sum(node_cores)`
- total memory
- used memory
- total disk
- GPU/NPU resources
- connected nodes / stale nodes / unhealthy nodes
- active jobs / queued jobs

Click/expand shows per-node breakdown.

## Visualization Rules

- Use proper charts: stacked resource bars, line charts, heatmaps, tables, DAG/timeline
- **Never chart missing data as zero**
- **Show unknown/stale/demo explicitly**
- Source labels must be visible
- Stale data must be dimmed
- Demo data must be labeled

## Future Migration Path

**Current:**
- All nodes connect to operator coordination node
- Coordinator assists with inventory, scheduling, routing hints

**Later:**
- Multiple coordinators
- Federated schedulers
- Peer-assisted routing
- Direct node-to-node sessions
- Marketplace routing
- Decentralized query/repair

**Do not hardcode single permanent coordinator into protocol names or storage.**

## Agent/Assistant Implications

The assistant must understand this pivot. Assistant must know:

- Nodes connect to the bootstrap coordinator for now — this is coordination, not authority
- Provisioning is single-use, controller-signed, node-key-generated
- Node stream is authoritative
- Scheduler assignments are proposals until node commits
- Dashboard state must mark source of truth
- Agent claims are not facts — completion requires evidence

Assistant should always answer operational questions with source labels:
- "reported by node" (authoritative)
- "observed by coordinator" (advisory)
- "cached by dashboard" (stale)
- "claimed by agent" (unverified)
- "verified by event/test/receipt" (evidence-backed)
- "unknown" (no data)

### Example Assistant Answer

**Q: "What is going on with my nodes?"**

**A:**
```
Aggregate resources: 128 cores @ 64% utilization, 312/512 GB memory
Connected: 7/9 nodes online (2 stale >60s)
Provisioning: 9 issued, 7 accepted, 2 pending first boot
Coordinator observations: all reachable within 30s
Authoritative stream state: 7 nodes with head seq > 100
Unknowns: 2 nodes not yet claimed
Next verification: check node stream heads for commit events
```
