"use client"

import { useMemo, useState } from "react"
import { useStore } from "@nanostores/react"
import {
  Activity,
  BrainCircuit,
  CheckCircle2,
  Cloud,
  Copy,
  Fingerprint,
  HardDrive,
  Link2,
  Network,
  Plus,
  Route,
  Shield,
  Wallet,
  type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { AppHeader, AppToolbar } from "@/components/os/app-chrome"
import { useAuth } from "@/hooks/use-auth"
import { runtimeEventLog, runtimeEventLogStore } from "@/platform/runtime/runtime-event-log"

type NodeRole = "browser" | "admission" | "relay" | "storage" | "compute" | "publishing"
type RuntimeTarget = "browser-wasm" | "localhost-native" | "vps-native" | "remote-native"
type NodeStatus = "running" | "available" | "offline" | "draft"
type ConnectionKind = "submits-to" | "assigns-relay" | "routes-to" | "stores-on" | "inherits-policy"

type PolicyTemplate = {
  id: string
  name: string
  summary: string
  budget: string
  policyHash: string
}

type NodeInstance = {
  id: string
  label: string
  role: NodeRole
  runtime: RuntimeTarget
  status: NodeStatus
  owner: string
  policyId: string
  budget: string
  endpoint: string
  proofRef: string
}

type NetworkConnection = {
  id: string
  from: string
  to: string
  kind: ConnectionKind
  label: string
}

const policies: PolicyTemplate[] = [
  { id: "dao-default", name: "EdgeRun DAO default", summary: "Shared network admission policy for general app/storage work.", budget: "network default", policyHash: "policy:dao:default" },
  { id: "personal", name: "Personal strict", summary: "Only your browser identity, private relays, and your approved storage/compute nodes.", budget: "low daily spend", policyHash: "policy:personal:strict" },
  { id: "family", name: "Family budget", summary: "Separate admission scope for family members with capped app/storage budgets.", budget: "family cap", policyHash: "policy:family:budget" },
  { id: "business", name: "Business apps", summary: "Higher budget for publishing, app hosting, storage sync, and agent work.", budget: "business cap", policyHash: "policy:business:apps" },
]

function roleIcon(role: NodeRole): LucideIcon {
  switch (role) {
    case "browser": return Fingerprint
    case "admission": return Shield
    case "relay": return Route
    case "storage": return HardDrive
    case "compute": return BrainCircuit
    case "publishing": return Cloud
  }
}

function roleLabel(role: NodeRole): string {
  switch (role) {
    case "browser": return "Browser node"
    case "admission": return "Admission"
    case "relay": return "Relay"
    case "storage": return "Storage"
    case "compute": return "Compute"
    case "publishing": return "Publishing"
  }
}

function statusClass(status: NodeStatus) {
  switch (status) {
    case "running": return "border-[var(--status-online)]/30 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    case "available": return "border-primary/30 bg-primary/10 text-primary"
    case "offline": return "border-[var(--status-error)]/30 bg-[var(--status-error)]/10 text-[var(--status-error)]"
    case "draft": return "border-[var(--status-warning)]/30 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
  }
}

function runtimeLabel(runtime: RuntimeTarget): string {
  switch (runtime) {
    case "browser-wasm": return "Browser WASM"
    case "localhost-native": return "Localhost native"
    case "vps-native": return "VPS native"
    case "remote-native": return "Remote native"
  }
}

function short(value: string) {
  return value.length > 28 ? `${value.slice(0, 18)}…${value.slice(-6)}` : value
}

function defaultNodes(owner: string, browserNodeId?: string): NodeInstance[] {
  return [
    { id: "browser-primary", label: "My browser node", role: "browser", runtime: "browser-wasm", status: "running", owner, policyId: "personal", budget: "local approvals", endpoint: browserNodeId ? `node:${browserNodeId}` : "browser:local", proofRef: browserNodeId ? `profile-node:${browserNodeId}` : "profile-node:local" },
    { id: "admission-dao", label: "DAO admission", role: "admission", runtime: "remote-native", status: "available", owner: "EdgeRun DAO", policyId: "dao-default", budget: "network default", endpoint: "admission.edgerun.network", proofRef: "admission:dao:default" },
    { id: "admission-family", label: "Family admission", role: "admission", runtime: "browser-wasm", status: "draft", owner, policyId: "family", budget: "family cap", endpoint: "browser:admission:family", proofRef: "admission:family:draft" },
    { id: "relay-private", label: "Private relay", role: "relay", runtime: "localhost-native", status: "draft", owner, policyId: "personal", budget: "private traffic", endpoint: "ws://127.0.0.1:8787", proofRef: "relay:private:draft" },
    { id: "storage-home", label: "Home storage", role: "storage", runtime: "localhost-native", status: "draft", owner, policyId: "personal", budget: "free / own hardware", endpoint: "localhost storage node", proofRef: "storage:home:draft" },
    { id: "compute-browser", label: "Browser compute", role: "compute", runtime: "browser-wasm", status: "running", owner, policyId: "personal", budget: "local only", endpoint: "browser:compute:local", proofRef: "compute:browser:local" },
    { id: "publishing-vps", label: "Publishing VPS", role: "publishing", runtime: "vps-native", status: "draft", owner, policyId: "business", budget: "hosting budget", endpoint: "https://publish.example", proofRef: "publishing:vps:draft" },
  ]
}

const defaultConnections: NetworkConnection[] = [
  { id: "browser-dao", from: "browser-primary", to: "admission-dao", kind: "submits-to", label: "default work requests" },
  { id: "browser-family", from: "browser-primary", to: "admission-family", kind: "submits-to", label: "family-scoped work" },
  { id: "family-inherits-dao", from: "admission-family", to: "admission-dao", kind: "inherits-policy", label: "inherits baseline policy" },
  { id: "family-relay", from: "admission-family", to: "relay-private", kind: "assigns-relay", label: "private relay path" },
  { id: "relay-storage", from: "relay-private", to: "storage-home", kind: "routes-to", label: "storage route" },
  { id: "relay-compute", from: "relay-private", to: "compute-browser", kind: "routes-to", label: "compute route" },
  { id: "relay-publish", from: "relay-private", to: "publishing-vps", kind: "routes-to", label: "publishing route" },
]

function connectionLabel(kind: ConnectionKind): string {
  switch (kind) {
    case "submits-to": return "submits to"
    case "assigns-relay": return "assigns relay"
    case "routes-to": return "routes to"
    case "stores-on": return "stores on"
    case "inherits-policy": return "inherits policy"
  }
}

function NodeCard({ node, selected, policy, onSelect }: { node: NodeInstance; selected: boolean; policy?: PolicyTemplate; onSelect: () => void }) {
  const Icon = roleIcon(node.role)
  return (
    <button onClick={onSelect} className={cn("rounded-xl border bg-card/60 p-3 text-left transition-colors hover:border-primary/40 hover:bg-primary/5", selected ? "border-primary/50 bg-primary/10" : "border-border")}>
      <div className="flex items-start justify-between gap-3">
        <div className={cn("flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border", statusClass(node.status))}><Icon className="h-4 w-4" /></div>
        <span className={cn("rounded border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(node.status))}>{node.status}</span>
      </div>
      <div className="mt-3 text-sm font-semibold text-foreground">{node.label}</div>
      <div className="mt-1 text-xs text-muted-foreground">{roleLabel(node.role)} · {runtimeLabel(node.runtime)}</div>
      <div className="mt-2 rounded-md border border-border bg-background/60 px-2 py-1.5 text-[11px] text-muted-foreground">{policy?.name ?? node.policyId} · {node.budget}</div>
    </button>
  )
}

function ConnectionRow({ connection, nodes }: { connection: NetworkConnection; nodes: NodeInstance[] }) {
  const from = nodes.find((node) => node.id === connection.from)
  const to = nodes.find((node) => node.id === connection.to)
  if (!from || !to) return null
  return (
    <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-2 rounded-md border border-border bg-background/60 px-2 py-2 text-xs">
      <span className="truncate font-medium text-foreground">{from.label}</span>
      <span className="rounded bg-secondary px-2 py-0.5 text-[10px] text-muted-foreground">{connectionLabel(connection.kind)}</span>
      <span className="truncate text-right font-medium text-foreground">{to.label}</span>
      <span className="col-span-3 text-[10px] text-muted-foreground">{connection.label}</span>
    </div>
  )
}

function Info({ label, value, copy = false }: { label: string; value: string; copy?: boolean }) {
  return (
    <button onClick={() => copy ? void navigator.clipboard?.writeText(value) : undefined} className="flex items-center justify-between gap-3 rounded-md border border-border bg-background/60 px-3 py-2 text-left" title={copy ? "Copy" : undefined}>
      <span className="text-[10px] uppercase tracking-wide text-muted-foreground">{label}</span>
      <span className="min-w-0 truncate font-mono text-[11px] text-foreground">{short(value)}</span>
      {copy ? <Copy className="h-3 w-3 shrink-0 text-muted-foreground" /> : null}
    </button>
  )
}

function Inspector({ node, policy }: { node: NodeInstance; policy?: PolicyTemplate }) {
  const Icon = roleIcon(node.role)
  return (
    <aside className="flex w-80 shrink-0 flex-col border-l border-border bg-[var(--window-header)]/25">
      <div className="border-b border-border p-4">
        <div className="mb-3 flex items-start justify-between gap-3">
          <div className={cn("flex h-11 w-11 items-center justify-center rounded-xl border", statusClass(node.status))}><Icon className="h-5 w-5" /></div>
          <span className={cn("rounded border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(node.status))}>{node.status}</span>
        </div>
        <h3 className="text-sm font-semibold text-foreground">{node.label}</h3>
        <p className="mt-1 text-xs text-muted-foreground">{roleLabel(node.role)} · {runtimeLabel(node.runtime)}</p>
      </div>
      <div className="min-h-0 flex-1 overflow-auto p-4">
        <div className="mb-2 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">Policy</div>
        <div className="rounded-lg border border-border bg-background/60 p-3 text-xs leading-5 text-muted-foreground">
          <div className="font-semibold text-foreground">{policy?.name ?? node.policyId}</div>
          <div className="mt-1">{policy?.summary ?? "Custom policy"}</div>
          <div className="mt-2 font-mono text-[10px] text-foreground">{policy?.policyHash ?? node.policyId}</div>
        </div>
        <div className="mt-4 grid gap-2 text-xs">
          <Info label="Owner" value={node.owner} />
          <Info label="Budget" value={node.budget} />
          <Info label="Endpoint" value={node.endpoint} />
          <Info label="Proof" value={node.proofRef} copy />
        </div>
        <div className="mt-4 rounded-lg border border-primary/20 bg-primary/5 p-3 text-xs leading-5 text-muted-foreground">
          {node.role === "admission"
            ? "Admission instances decide whether signed work requests may enter the network and which relay/channel/path they may use."
            : node.role === "relay"
              ? "Relay instances move admitted ordered packets without owning the content. They get paid only with proof-backed delivery."
              : node.role === "storage"
                ? "Storage instances accept admitted store/retrieve/pin/copy work and return verifiable object or shard evidence."
                : node.role === "browser"
                  ? "The browser node owns user intent, local approvals, app execution, and lightweight WASM role instances."
                  : "This node instance should be connected through admission and relay policy before it handles useful work."}
        </div>
      </div>
    </aside>
  )
}

function Metric({ icon: Icon, label, value }: { icon: LucideIcon; label: string; value: string }) {
  return <div className="rounded-md border border-border bg-background/60 px-2 py-1.5"><div className="flex items-center gap-1.5 text-[10px] text-muted-foreground"><Icon className="h-3 w-3" />{label}</div><div className="mt-0.5 font-mono text-sm font-semibold text-foreground">{value}</div></div>
}

function Action({ label, onClick }: { label: string; onClick: () => void }) {
  return <button onClick={onClick} className="rounded-md border border-border bg-secondary/35 px-3 py-2 text-left text-xs font-medium text-foreground hover:bg-secondary">{label}</button>
}

export function NodeNetworkBuilder() {
  const auth = useAuth()
  const runtimeEvents = useStore(runtimeEventLogStore)
  const profile = auth.unlockedProfile
  const owner = profile?.handle ?? "Local user"
  const initialNodes = useMemo(() => defaultNodes(owner, profile?.browserNode.identityIdHex), [owner, profile?.browserNode.identityIdHex])
  const [nodes, setNodes] = useState<NodeInstance[]>(initialNodes)
  const [connections, setConnections] = useState<NetworkConnection[]>(defaultConnections)
  const [selectedId, setSelectedId] = useState("browser-primary")
  const [selectedPolicyId, setSelectedPolicyId] = useState("personal")
  const selected = nodes.find((node) => node.id === selectedId) ?? nodes[0]
  const selectedPolicy = policies.find((policy) => policy.id === selected.policyId)
  const selectedPolicyTemplate = policies.find((policy) => policy.id === selectedPolicyId) ?? policies[1]
  const running = nodes.filter((node) => node.status === "running").length
  const admissionCount = nodes.filter((node) => node.role === "admission").length
  const relayCount = nodes.filter((node) => node.role === "relay").length
  const networkEvents = runtimeEvents.filter((event) => event.actor === "node-dashboard").length

  function addNode(role: NodeRole, runtime: RuntimeTarget) {
    const policy = selectedPolicyTemplate
    const id = `${role}-${Date.now()}`
    const node: NodeInstance = { id, label: `${policy.name} ${roleLabel(role)}`, role, runtime, status: "draft", owner, policyId: policy.id, budget: policy.budget, endpoint: runtime === "browser-wasm" ? `browser:${role}:${id}` : runtime === "localhost-native" ? `localhost:${role}` : `${runtime}:${role}`, proofRef: `${role}:${policy.id}:draft` }
    setNodes((current) => [...current, node])
    setSelectedId(id)
    runtimeEventLog.append({ kind: "capability_action_completed", actor: "node-dashboard", target: id, capabilityId: "node.instance.create", reason: `created ${role} node instance draft`, metadata: { role, runtime, policyId: policy.id, policyHash: policy.policyHash } })
  }

  function connectSelectedTo(targetRole: NodeRole, kind: ConnectionKind) {
    const target = nodes.find((node) => node.role === targetRole && node.id !== selected.id)
    if (!target) return
    const id = `${selected.id}-${target.id}-${Date.now()}`
    const connection: NetworkConnection = { id, from: selected.id, to: target.id, kind, label: `${selected.label} ${connectionLabel(kind)} ${target.label}` }
    setConnections((current) => [...current, connection])
    runtimeEventLog.append({ kind: "capability_action_completed", actor: "node-dashboard", target: id, capabilityId: "node.route.connect", reason: "connected node instances in visual network builder", metadata: { from: selected.id, to: target.id, kind } })
  }

  return (
    <div className="flex h-full min-h-0 bg-background text-foreground">
      <main className="flex min-w-0 flex-1 flex-col">
        <AppHeader title="Node Network" icon={<Network className="h-4 w-4" />}>
          Visual builder for admission, relay, storage, compute, and publishing node instances
        </AppHeader>
        <AppToolbar className="border-b border-border">
          <div className="grid grid-cols-4 gap-2 text-xs sm:w-[520px]">
            <Metric icon={Activity} label="Running" value={`${running}/${nodes.length}`} />
            <Metric icon={Shield} label="Admission" value={String(admissionCount)} />
            <Metric icon={Route} label="Relays" value={String(relayCount)} />
            <Metric icon={CheckCircle2} label="Events" value={String(networkEvents)} />
          </div>
          <div className="ml-auto flex items-center gap-2">
            <select value={selectedPolicyId} onChange={(event) => setSelectedPolicyId(event.target.value)} className="h-8 rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-primary">
              {policies.map((policy) => <option key={policy.id} value={policy.id}>{policy.name}</option>)}
            </select>
            <button onClick={() => addNode("admission", "browser-wasm")} className="inline-flex h-8 items-center gap-1.5 rounded-md bg-primary px-2 text-xs font-semibold text-primary-foreground hover:bg-primary/90"><Plus className="h-3.5 w-3.5" />Admission</button>
            <button onClick={() => addNode("relay", "browser-wasm")} className="inline-flex h-8 items-center gap-1.5 rounded-md bg-secondary px-2 text-xs font-semibold text-secondary-foreground hover:bg-secondary/80"><Plus className="h-3.5 w-3.5" />Relay</button>
            <button onClick={() => addNode("storage", "localhost-native")} className="inline-flex h-8 items-center gap-1.5 rounded-md bg-secondary px-2 text-xs font-semibold text-secondary-foreground hover:bg-secondary/80"><Plus className="h-3.5 w-3.5" />Storage</button>
          </div>
        </AppToolbar>
        <div className="grid min-h-0 flex-1 grid-cols-[minmax(420px,1fr)_320px]">
          <section className="min-w-0 overflow-auto p-4">
            <div className="mb-4 rounded-xl border border-primary/20 bg-primary/5 p-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <div className="text-sm font-semibold text-foreground">Build your own network</div>
                  <div className="mt-1 max-w-3xl text-xs leading-5 text-muted-foreground">Put together role instances visually. Browser/WASM instances are good for local policy and lightweight work. Native instances are good for ports, storage, publishing, and durable relays. All useful work should enter through admission and move through relays under policy.</div>
                </div>
                <div className="rounded-md border border-border bg-background/60 px-3 py-2 text-[11px] text-muted-foreground">node = identity + role + policy + budget + route</div>
              </div>
            </div>
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
              {nodes.map((node) => <NodeCard key={node.id} node={node} policy={policies.find((policy) => policy.id === node.policyId)} selected={node.id === selected.id} onSelect={() => setSelectedId(node.id)} />)}
            </div>
            <div className="mt-5 grid gap-4 lg:grid-cols-[1fr_0.9fr]">
              <div className="rounded-xl border border-border bg-card/55 p-4">
                <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground"><Link2 className="h-4 w-4 text-primary" />Connections</div>
                <div className="grid gap-2">{connections.map((connection) => <ConnectionRow key={connection.id} connection={connection} nodes={nodes} />)}</div>
              </div>
              <div className="rounded-xl border border-border bg-card/55 p-4">
                <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground"><Shield className="h-4 w-4 text-primary" />Connect selected node</div>
                <div className="grid gap-2">
                  <Action label="Submit selected to admission" onClick={() => connectSelectedTo("admission", "submits-to")} />
                  <Action label="Let selected admission assign relay" onClick={() => connectSelectedTo("relay", "assigns-relay")} />
                  <Action label="Route selected relay to storage" onClick={() => connectSelectedTo("storage", "routes-to")} />
                  <Action label="Route selected relay to compute" onClick={() => connectSelectedTo("compute", "routes-to")} />
                  <Action label="Inherit selected policy from DAO" onClick={() => connectSelectedTo("admission", "inherits-policy")} />
                </div>
                <div className="mt-3 rounded-md border border-border bg-background/60 p-2 text-xs leading-5 text-muted-foreground">This is currently a visual builder and audit event source. The next step is generating signed route/admission policy records from this graph.</div>
              </div>
            </div>
          </section>
          <Inspector node={selected} policy={selectedPolicy} />
        </div>
      </main>
    </div>
  )
}

export function NodeDashboard() {
  return <NodeNetworkBuilder />
}
