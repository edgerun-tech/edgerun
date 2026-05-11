"use client"

import * as React from "react"
import {
  addEdge,
  Background,
  Controls,
  Handle,
  MarkerType,
  MiniMap,
  Position,
  ReactFlow,
  type Connection,
  type Edge,
  type Node,
  type NodeProps,
  useEdgesState,
  useNodesState,
} from "@xyflow/react"
import "@xyflow/react/dist/style.css"
import {
  BrainCircuit,
  Cloud,
  Fingerprint,
  HardDrive,
  Network,
  Plus,
  Route,
  Shield,
  Wallet,
  type LucideIcon,
} from "lucide-react"
import { useStore } from "@nanostores/react"
import { cn } from "@/lib/utils"
import { AppHeader, AppToolbar } from "@/components/os/app-chrome"
import { useAuth } from "@/hooks/use-auth"
import { runtimeEventLog, runtimeEventLogStore } from "@/platform/runtime/runtime-event-log"

type NodeRole = "browser" | "admission" | "relay" | "storage" | "compute" | "publishing" | "settlement"
type RuntimeTarget = "browser-wasm" | "localhost-native" | "vps-native" | "remote-native"
type PolicyId = "dao-default" | "personal" | "family" | "business"

type PolicyTemplate = {
  id: PolicyId
  name: string
  summary: string
  budget: string
  hash: string
}

type GraphNodeData = {
  label: string
  role: NodeRole
  runtime: RuntimeTarget
  policyId: PolicyId
  owner: string
  budget: string
  endpoint: string
  status: "running" | "available" | "draft" | "offline"
}

const policies: Record<PolicyId, PolicyTemplate> = {
  "dao-default": {
    id: "dao-default",
    name: "EdgeRun DAO default",
    summary: "Default shared network admission and baseline policy inheritance.",
    budget: "network default",
    hash: "policy:dao:default",
  },
  personal: {
    id: "personal",
    name: "Personal strict",
    summary: "Only your browser identity, approved relays, and your own machines.",
    budget: "low daily spend",
    hash: "policy:personal:strict",
  },
  family: {
    id: "family",
    name: "Family budget",
    summary: "Separate rules and spend limits for family members and shared devices.",
    budget: "family cap",
    hash: "policy:family:budget",
  },
  business: {
    id: "business",
    name: "Business apps",
    summary: "Higher budget for publishing, storage sync, app sales, and agents.",
    budget: "business cap",
    hash: "policy:business:apps",
  },
}

function roleIcon(role: NodeRole): LucideIcon {
  switch (role) {
    case "browser": return Fingerprint
    case "admission": return Shield
    case "relay": return Route
    case "storage": return HardDrive
    case "compute": return BrainCircuit
    case "publishing": return Cloud
    case "settlement": return Wallet
  }
}

function roleTitle(role: NodeRole): string {
  switch (role) {
    case "browser": return "Browser"
    case "admission": return "Admission"
    case "relay": return "Relay"
    case "storage": return "Storage"
    case "compute": return "Compute"
    case "publishing": return "Publishing"
    case "settlement": return "Settlement"
  }
}

function runtimeTitle(runtime: RuntimeTarget): string {
  switch (runtime) {
    case "browser-wasm": return "Browser WASM"
    case "localhost-native": return "Localhost native"
    case "vps-native": return "VPS native"
    case "remote-native": return "Remote native"
  }
}

function statusClass(status: GraphNodeData["status"]) {
  switch (status) {
    case "running": return "border-[var(--status-online)]/40 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    case "available": return "border-primary/40 bg-primary/10 text-primary"
    case "draft": return "border-[var(--status-warning)]/40 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
    case "offline": return "border-[var(--status-error)]/40 bg-[var(--status-error)]/10 text-[var(--status-error)]"
  }
}

function nodeRoleColor(role: NodeRole) {
  switch (role) {
    case "browser": return "#22c55e"
    case "admission": return "#8b5cf6"
    case "relay": return "#06b6d4"
    case "storage": return "#f59e0b"
    case "compute": return "#ef4444"
    case "publishing": return "#3b82f6"
    case "settlement": return "#a855f7"
  }
}

function EdgeRunNode({ data, selected }: NodeProps<Node<GraphNodeData>>) {
  const Icon = roleIcon(data.role)
  const policy = policies[data.policyId]
  return (
    <div className={cn(
      "w-60 rounded-xl border bg-card/95 p-3 shadow-xl backdrop-blur",
      selected ? "border-primary ring-2 ring-primary/30" : "border-border",
    )}>
      <Handle type="target" position={Position.Left} className="!h-3 !w-3 !border-background !bg-primary" />
      <Handle type="source" position={Position.Right} className="!h-3 !w-3 !border-background !bg-primary" />
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2">
          <div className="flex h-9 w-9 items-center justify-center rounded-lg border border-border bg-background/80 text-primary">
            <Icon className="h-4 w-4" />
          </div>
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold text-foreground">{data.label}</div>
            <div className="text-[10px] uppercase tracking-wide text-muted-foreground">{roleTitle(data.role)}</div>
          </div>
        </div>
        <span className={cn("rounded border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(data.status))}>{data.status}</span>
      </div>
      <div className="mt-3 grid gap-1.5 text-[11px] text-muted-foreground">
        <div className="flex justify-between gap-2"><span>runtime</span><span className="text-foreground">{runtimeTitle(data.runtime)}</span></div>
        <div className="flex justify-between gap-2"><span>policy</span><span className="truncate text-foreground">{policy.name}</span></div>
        <div className="flex justify-between gap-2"><span>budget</span><span className="text-foreground">{data.budget}</span></div>
      </div>
    </div>
  )
}

const nodeTypes = { edgerun: EdgeRunNode }

function initialNodes(owner: string, browserNodeId?: string): Node<GraphNodeData>[] {
  return [
    {
      id: "browser",
      type: "edgerun",
      position: { x: 40, y: 220 },
      data: { label: "My browser", role: "browser", runtime: "browser-wasm", policyId: "personal", owner, budget: "local approvals", endpoint: browserNodeId ? `node:${browserNodeId}` : "browser:local", status: "running" },
    },
    {
      id: "dao-admission",
      type: "edgerun",
      position: { x: 400, y: 60 },
      data: { label: "DAO admission", role: "admission", runtime: "remote-native", policyId: "dao-default", owner: "EdgeRun DAO", budget: "network default", endpoint: "admission.edgerun.network", status: "available" },
    },
    {
      id: "family-admission",
      type: "edgerun",
      position: { x: 400, y: 280 },
      data: { label: "Family admission", role: "admission", runtime: "browser-wasm", policyId: "family", owner, budget: "family cap", endpoint: "browser:admission:family", status: "draft" },
    },
    {
      id: "private-relay",
      type: "edgerun",
      position: { x: 760, y: 280 },
      data: { label: "Private relay", role: "relay", runtime: "localhost-native", policyId: "personal", owner, budget: "private traffic", endpoint: "ws://127.0.0.1:8787", status: "draft" },
    },
    {
      id: "home-storage",
      type: "edgerun",
      position: { x: 1120, y: 140 },
      data: { label: "Home storage", role: "storage", runtime: "localhost-native", policyId: "personal", owner, budget: "own hardware", endpoint: "localhost:storage", status: "draft" },
    },
    {
      id: "browser-compute",
      type: "edgerun",
      position: { x: 1120, y: 380 },
      data: { label: "Browser compute", role: "compute", runtime: "browser-wasm", policyId: "personal", owner, budget: "local only", endpoint: "browser:compute", status: "running" },
    },
  ]
}

const initialEdges: Edge[] = [
  { id: "browser-dao", source: "browser", target: "dao-admission", label: "default WorkRequest", animated: true, markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "browser-family", source: "browser", target: "family-admission", label: "family WorkRequest", animated: true, markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "family-dao", source: "family-admission", target: "dao-admission", label: "inherits policy", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "family-relay", source: "family-admission", target: "private-relay", label: "assigns relay", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "relay-storage", source: "private-relay", target: "home-storage", label: "routes storage", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "relay-compute", source: "private-relay", target: "browser-compute", label: "routes compute", markerEnd: { type: MarkerType.ArrowClosed } },
]

function Inspector({ node, onUpdatePolicy }: { node?: Node<GraphNodeData>; onUpdatePolicy: (policyId: PolicyId) => void }) {
  if (!node) {
    return <aside className="w-80 border-l border-border bg-background/80 p-4 text-sm text-muted-foreground">Select a node</aside>
  }
  const data = node.data
  const policy = policies[data.policyId]
  const Icon = roleIcon(data.role)
  return (
    <aside className="w-80 shrink-0 overflow-auto border-l border-border bg-background/85 p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex h-11 w-11 items-center justify-center rounded-xl border border-border bg-card text-primary"><Icon className="h-5 w-5" /></div>
        <span className={cn("rounded border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(data.status))}>{data.status}</span>
      </div>
      <h3 className="mt-3 text-sm font-semibold text-foreground">{data.label}</h3>
      <p className="text-xs text-muted-foreground">{roleTitle(data.role)} · {runtimeTitle(data.runtime)}</p>

      <div className="mt-5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">Policy</div>
      <select value={data.policyId} onChange={(event) => onUpdatePolicy(event.target.value as PolicyId)} className="mt-2 h-9 w-full rounded-md border border-border bg-card px-2 text-xs text-foreground outline-none focus:border-primary">
        {Object.values(policies).map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
      </select>
      <div className="mt-2 rounded-lg border border-border bg-card/70 p-3 text-xs leading-5 text-muted-foreground">
        <div className="font-semibold text-foreground">{policy.name}</div>
        <div className="mt-1">{policy.summary}</div>
        <div className="mt-2 font-mono text-[10px] text-foreground">{policy.hash}</div>
      </div>

      <div className="mt-5 grid gap-2 text-xs">
        <Info label="Owner" value={data.owner} />
        <Info label="Budget" value={data.budget} />
        <Info label="Endpoint" value={data.endpoint} />
        <Info label="Node ID" value={node.id} />
      </div>

      <div className="mt-5 rounded-lg border border-primary/20 bg-primary/5 p-3 text-xs leading-5 text-muted-foreground">
        Drag nodes around the canvas. Connect handles to draw new policy/route lines. Admission nodes decide which workloads may enter the network; relays carry admitted ordered packets; workers/storage only receive admitted relay traffic.
      </div>
    </aside>
  )
}

function Info({ label, value }: { label: string; value: string }) {
  return (
    <button onClick={() => void navigator.clipboard?.writeText(value)} className="flex items-center justify-between gap-3 rounded-md border border-border bg-card/70 px-3 py-2 text-left">
      <span className="text-[10px] uppercase tracking-wide text-muted-foreground">{label}</span>
      <span className="min-w-0 truncate font-mono text-[11px] text-foreground">{value}</span>
    </button>
  )
}

function nextPosition(index: number) {
  return { x: 120 + (index % 3) * 320, y: 120 + Math.floor(index / 3) * 220 }
}

export function NodePolicyGraph() {
  const auth = useAuth()
  const owner = auth.unlockedProfile?.handle ?? "Local user"
  const browserNodeId = auth.unlockedProfile?.browserNode.identityIdHex
  const [nodes, setNodes, onNodesChange] = useNodesState<GraphNodeData>(initialNodes(owner, browserNodeId))
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges)
  const [selectedId, setSelectedId] = React.useState<string | null>("browser")
  const selected = nodes.find((node) => node.id === selectedId)

  const onConnect = React.useCallback((connection: Connection) => {
    setEdges((eds) => addEdge({ ...connection, animated: true, markerEnd: { type: MarkerType.ArrowClosed }, label: "policy route" }, eds))
    runtimeEventLog.append({
      kind: "capability_action_completed",
      actor: "node-dashboard",
      target: `${connection.source}->${connection.target}`,
      capabilityId: "node.graph.connect",
      reason: "connected nodes in policy graph",
      metadata: { source: connection.source, target: connection.target },
    })
  }, [setEdges])

  function addNode(role: NodeRole, runtime: RuntimeTarget, policyId: PolicyId) {
    const id = `${role}-${Date.now()}`
    setNodes((current) => [
      ...current,
      {
        id,
        type: "edgerun",
        position: nextPosition(current.length),
        data: {
          label: `${policies[policyId].name} ${roleTitle(role)}`,
          role,
          runtime,
          policyId,
          owner,
          budget: policies[policyId].budget,
          endpoint: runtime === "browser-wasm" ? `browser:${role}:${id}` : runtime === "localhost-native" ? `localhost:${role}` : `${runtime}:${role}`,
          status: "draft",
        },
      },
    ])
    setSelectedId(id)
  }

  function updateSelectedPolicy(policyId: PolicyId) {
    if (!selected) return
    setNodes((current) => current.map((node) => node.id === selected.id ? { ...node, data: { ...node.data, policyId, budget: policies[policyId].budget } } : node))
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-background text-foreground">
      <AppHeader title="Node Network" icon={<Network className="h-4 w-4" />}>
        Drag node instances, connect them with lines, and assign policies/budgets
      </AppHeader>
      <AppToolbar className="border-b border-border">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span className="rounded border border-border bg-card px-2 py-1">{nodes.length} nodes</span>
          <span className="rounded border border-border bg-card px-2 py-1">{edges.length} links</span>
        </div>
        <div className="ml-auto flex flex-wrap items-center gap-2">
          <button onClick={() => addNode("admission", "browser-wasm", "personal")} className="inline-flex h-8 items-center gap-1.5 rounded-md bg-primary px-2 text-xs font-semibold text-primary-foreground hover:bg-primary/90"><Plus className="h-3.5 w-3.5" />Admission</button>
          <button onClick={() => addNode("relay", "browser-wasm", "personal")} className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-card px-2 text-xs font-semibold text-foreground hover:bg-secondary"><Plus className="h-3.5 w-3.5" />Relay</button>
          <button onClick={() => addNode("storage", "localhost-native", "personal")} className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-card px-2 text-xs font-semibold text-foreground hover:bg-secondary"><Plus className="h-3.5 w-3.5" />Storage</button>
          <button onClick={() => addNode("compute", "browser-wasm", "personal")} className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-card px-2 text-xs font-semibold text-foreground hover:bg-secondary"><Plus className="h-3.5 w-3.5" />Compute</button>
        </div>
      </AppToolbar>
      <div className="min-h-0 flex-1 flex">
        <div className="min-w-0 flex-1">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            nodeTypes={nodeTypes}
            onNodeClick={(_, node) => setSelectedId(node.id)}
            fitView
            colorMode="dark"
          >
            <Background />
            <MiniMap nodeColor={(node) => nodeRoleColor((node.data as GraphNodeData).role)} pannable zoomable />
            <Controls />
          </ReactFlow>
        </div>
        <Inspector node={selected} onUpdatePolicy={updateSelectedPolicy} />
      </div>
    </div>
  )
}

export function NodeDashboard() {
  return <NodePolicyGraph />
}
