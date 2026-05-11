"use client"

import * as React from "react"
import { addEdge, Background, Controls, Handle, MarkerType, Position, ReactFlow, type Connection, type Edge, type Node, type NodeProps, useEdgesState, useNodesState } from "@xyflow/react"
import "@xyflow/react/dist/style.css"
import { BrainCircuit, HardDrive, Network, Plus, Route, Shield } from "lucide-react"
import { AppHeader, AppToolbar } from "@/components/os/app-chrome"
import { cn } from "@/lib/utils"

type Role = "admission" | "relay" | "storage" | "compute"
type Runtime = "wasm" | "native"

type Data = {
  label: string
  role: Role
  runtime: Runtime
  admissionNodeId: string
  budget: string
  capabilities: string[]
  status: "running" | "draft" | "available"
}

function icon(role: Role) {
  if (role === "admission") return Shield
  if (role === "relay") return Route
  if (role === "storage") return HardDrive
  return BrainCircuit
}

function color(role: Role) {
  if (role === "admission") return "#8b5cf6"
  if (role === "relay") return "#06b6d4"
  if (role === "storage") return "#f59e0b"
  return "#ef4444"
}

function statusClass(status: Data["status"]) {
  if (status === "running") return "border-[var(--status-online)]/40 bg-[var(--status-online)]/10 text-[var(--status-online)]"
  if (status === "available") return "border-primary/40 bg-primary/10 text-primary"
  return "border-[var(--status-warning)]/40 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
}

function GraphNode({ data, selected }: NodeProps<Node<Data>>) {
  const Icon = icon(data.role)
  return (
    <div className={cn("w-64 rounded-xl border bg-card/95 p-3 shadow-xl", selected ? "border-primary ring-2 ring-primary/30" : "border-border")}>
      <Handle type="target" position={Position.Left} className="!h-3 !w-3 !bg-primary" />
      <Handle type="source" position={Position.Right} className="!h-3 !w-3 !bg-primary" />
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <div className="flex h-9 w-9 items-center justify-center rounded-lg border border-border bg-background text-primary"><Icon className="h-4 w-4" /></div>
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold text-foreground">{data.label}</div>
            <div className="text-[10px] uppercase tracking-wide text-muted-foreground">{data.role} · {data.runtime}</div>
          </div>
        </div>
        <span className={cn("rounded border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(data.status))}>{data.status}</span>
      </div>
      <div className="mt-3 grid gap-1 text-[11px] text-muted-foreground">
        <div className="flex justify-between gap-2"><span>admission</span><span className="truncate text-foreground">{data.admissionNodeId}</span></div>
        <div className="flex justify-between gap-2"><span>budget</span><span className="truncate text-foreground">{data.budget}</span></div>
        <div className="truncate">caps: {data.capabilities.join(", ")}</div>
      </div>
    </div>
  )
}

const nodeTypes = { edge: GraphNode }

const initialNodes: Node<Data>[] = [
  { id: "admission-personal", type: "edge", position: { x: 80, y: 220 }, data: { label: "Personal admission", role: "admission", runtime: "wasm", admissionNodeId: "self", budget: "low daily spend", capabilities: ["sign", "policy", "route"], status: "running" } },
  { id: "admission-dao", type: "edge", position: { x: 420, y: 80 }, data: { label: "DAO admission", role: "admission", runtime: "native", admissionNodeId: "self", budget: "network default", capabilities: ["baseline"], status: "available" } },
  { id: "relay-private", type: "edge", position: { x: 760, y: 220 }, data: { label: "Private relay", role: "relay", runtime: "native", admissionNodeId: "admission-personal", budget: "assigned by admission", capabilities: ["relay", "hash"], status: "draft" } },
  { id: "storage-home", type: "edge", position: { x: 1120, y: 120 }, data: { label: "Home storage", role: "storage", runtime: "native", admissionNodeId: "admission-personal", budget: "assigned by admission", capabilities: ["store", "retrieve"], status: "draft" } },
  { id: "compute-local", type: "edge", position: { x: 1120, y: 380 }, data: { label: "Local compute", role: "compute", runtime: "wasm", admissionNodeId: "admission-personal", budget: "assigned by admission", capabilities: ["compute"], status: "draft" } },
]

const initialEdges: Edge[] = [
  { id: "personal-dao", source: "admission-personal", target: "admission-dao", label: "can ask baseline", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "relay-admission", source: "relay-private", target: "admission-personal", label: "follows admission", markerEnd: { type: MarkerType.ArrowClosed }, animated: true },
  { id: "storage-admission", source: "storage-home", target: "admission-personal", label: "follows admission", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "compute-admission", source: "compute-local", target: "admission-personal", label: "follows admission", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "admission-relay", source: "admission-personal", target: "relay-private", label: "assigns relay", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "relay-storage", source: "relay-private", target: "storage-home", label: "routes admitted storage", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "relay-compute", source: "relay-private", target: "compute-local", label: "routes admitted compute", markerEnd: { type: MarkerType.ArrowClosed } },
]

function Inspector({ node, setAuthority }: { node?: Node<Data>; setAuthority: (id: string) => void }) {
  if (!node) return <aside className="w-80 border-l border-border bg-background/80 p-4 text-sm text-muted-foreground">Select a node</aside>
  return (
    <aside className="w-80 shrink-0 overflow-auto border-l border-border bg-background/85 p-4">
      <h3 className="text-sm font-semibold text-foreground">{node.data.label}</h3>
      <p className="text-xs text-muted-foreground">{node.data.role} · {node.data.runtime}</p>
      <div className="mt-5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">Admission authority</div>
      <select value={node.data.admissionNodeId} disabled={node.data.role === "admission"} onChange={(event) => setAuthority(event.target.value)} className="mt-2 h-9 w-full rounded-md border border-border bg-card px-2 text-xs text-foreground disabled:opacity-50">
        <option value="admission-personal">Personal admission</option>
        <option value="admission-dao">DAO admission</option>
      </select>
      <div className="mt-4 grid gap-2 text-xs">
        <Info label="Admission" value={node.data.admissionNodeId} />
        <Info label="Budget" value={node.data.budget} />
        <Info label="Capabilities" value={node.data.capabilities.join(", ")} />
        <Info label="Node ID" value={node.id} />
      </div>
      <p className="mt-5 rounded-lg border border-primary/20 bg-primary/5 p-3 text-xs leading-5 text-muted-foreground">Policy authority is the admission node id. Nodes follow that admission address first; content-addressed policy documents can be attached later after storage exists.</p>
    </aside>
  )
}

function Info({ label, value }: { label: string; value: string }) {
  return <button onClick={() => void navigator.clipboard?.writeText(value)} className="flex items-center justify-between gap-3 rounded-md border border-border bg-card/70 px-3 py-2 text-left"><span className="text-[10px] uppercase tracking-wide text-muted-foreground">{label}</span><span className="truncate font-mono text-[11px] text-foreground">{value}</span></button>
}

export function AdmissionNetworkMinimal() {
  const [nodes, setNodes, onNodesChange] = useNodesState<Data>(initialNodes)
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges)
  const [selectedId, setSelectedId] = React.useState<string | null>("admission-personal")
  const selected = nodes.find((node) => node.id === selectedId)

  const onConnect = React.useCallback((connection: Connection) => setEdges((eds) => addEdge({ ...connection, markerEnd: { type: MarkerType.ArrowClosed }, label: "link" }, eds)), [setEdges])
  const addNode = (role: Role, runtime: Runtime) => setNodes((current) => [...current, { id: `${role}-${Date.now()}`, type: "edge", position: { x: 160, y: 160 }, data: { label: `New ${role}`, role, runtime, admissionNodeId: role === "admission" ? "self" : "admission-personal", budget: "assigned by admission", capabilities: [role], status: "draft" } }])
  const setAuthority = (id: string) => selected && setNodes((current) => current.map((node) => node.id === selected.id ? { ...node, data: { ...node.data, admissionNodeId: id } } : node))

  return (
    <div className="flex h-full min-h-0 flex-col bg-background text-foreground">
      <AppHeader title="Admission Network" icon={<Network className="h-4 w-4" />}>Drag standalone nodes, connect them with lines, and choose which admission node each one follows</AppHeader>
      <AppToolbar className="border-b border-border"><div className="flex items-center gap-2 text-xs text-muted-foreground"><span className="rounded border border-border bg-card px-2 py-1">{nodes.length} nodes</span><span className="rounded border border-border bg-card px-2 py-1">{edges.length} links</span></div><div className="ml-auto flex gap-2"><button onClick={() => addNode("admission", "wasm")} className="inline-flex h-8 items-center gap-1.5 rounded-md bg-primary px-2 text-xs font-semibold text-primary-foreground"><Plus className="h-3.5 w-3.5" />Admission</button><button onClick={() => addNode("relay", "wasm")} className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-card px-2 text-xs font-semibold text-foreground"><Plus className="h-3.5 w-3.5" />Relay</button><button onClick={() => addNode("storage", "native")} className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-card px-2 text-xs font-semibold text-foreground"><Plus className="h-3.5 w-3.5" />Storage</button><button onClick={() => addNode("compute", "wasm")} className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-card px-2 text-xs font-semibold text-foreground"><Plus className="h-3.5 w-3.5" />Compute</button></div></AppToolbar>
      <div className="min-h-0 flex-1 flex"><div className="min-w-0 flex-1"><ReactFlow nodes={nodes} edges={edges} onNodesChange={onNodesChange} onEdgesChange={onEdgesChange} onConnect={onConnect} nodeTypes={nodeTypes} onNodeClick={(_, node) => setSelectedId(node.id)} fitView colorMode="dark"><Background /><MiniMap nodeColor={(node) => color((node.data as Data).role)} /><Controls /></ReactFlow></div><Inspector node={selected} setAuthority={setAuthority} /></div>
    </div>
  )
}
