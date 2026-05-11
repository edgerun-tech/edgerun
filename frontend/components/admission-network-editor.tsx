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
import { BrainCircuit, Cloud, HardDrive, HelpCircle, Network, Plus, Route, Shield, Trash2, type LucideIcon } from "lucide-react"
import { cn } from "@/lib/utils"

type Role = "admission" | "relay" | "storage" | "compute" | "publishing"
type Runtime = "wasm" | "native"

type Port = { id: string; label: string; description: string }
type NodeDef = { role: Role; label: string; desc: string; detail: string; icon: LucideIcon; color: string; defaultRuntime: Runtime; inputs: Port[]; outputs: Port[]; capabilities: string[] }
type NodeData = { label: string; role: Role; runtime: Runtime; admissionNodeId: string; endpoint: string; budget: string; capabilities: string[]; discoverable: boolean; status: "running" | "available" | "draft" }

const DEFAULT_RENDEZVOUS = "admission.edgerun.tech"

const DEFS: Record<Role, NodeDef> = {
  admission: { role: "admission", label: "Admission", desc: "Policy authority and entry point", detail: "Receives work requests, enforces policy, selects relay paths, and signs admissions.", icon: Shield, color: "text-violet-400", defaultRuntime: "wasm", inputs: [{ id: "work_request", label: "work", description: "Signed WorkRequest" }, { id: "policy_source", label: "policy", description: "Policy or admission source" }], outputs: [{ id: "admission", label: "admit", description: "Signed WorkAdmission" }, { id: "relay_assignment", label: "route", description: "Relay/channel assignment" }], capabilities: ["sign_admissions", "enforce_policy", "select_relays"] },
  relay: { role: "relay", label: "Relay", desc: "Move admitted ordered packets", detail: "Carries admitted packets, hashes transit, and returns delivery evidence.", icon: Route, color: "text-cyan-400", defaultRuntime: "wasm", inputs: [{ id: "admitted_packet", label: "packet", description: "Admitted ordered packet" }], outputs: [{ id: "forwarded_packet", label: "forward", description: "Forwarded packet" }, { id: "transit_receipt", label: "receipt", description: "Transit receipt" }], capabilities: ["relay_packets", "hash_transit"] },
  storage: { role: "storage", label: "Storage", desc: "Store and retrieve data", detail: "Stores shards/objects and returns retrieval or status evidence.", icon: HardDrive, color: "text-amber-400", defaultRuntime: "native", inputs: [{ id: "store_request", label: "store", description: "Object/shard store request" }, { id: "retrieve_request", label: "read", description: "Object/shard retrieve request" }], outputs: [{ id: "store_receipt", label: "stored", description: "Store receipt/status" }, { id: "retrieve_response", label: "data", description: "Retrieved bytes/shard" }], capabilities: ["object_store", "object_retrieve", "object_pin"] },
  compute: { role: "compute", label: "Compute", desc: "Run admitted deterministic work", detail: "Runs compute jobs and returns output plus receipt/proof metadata.", icon: BrainCircuit, color: "text-red-400", defaultRuntime: "wasm", inputs: [{ id: "compute_request", label: "job", description: "Admitted compute job" }], outputs: [{ id: "compute_result", label: "result", description: "Compute result" }, { id: "compute_receipt", label: "receipt", description: "Compute receipt/proof" }], capabilities: ["compute_run"] },
  publishing: { role: "publishing", label: "Publishing", desc: "Expose approved services", detail: "Publishes sites, app packages, APIs, and compatibility endpoints under admission policy.", icon: Cloud, color: "text-blue-400", defaultRuntime: "native", inputs: [{ id: "publish_request", label: "publish", description: "Publish/deploy request" }, { id: "package_object", label: "object", description: "Package/content object" }], outputs: [{ id: "public_route", label: "route", description: "Published route" }, { id: "publish_receipt", label: "receipt", description: "Publish receipt/status" }], capabilities: ["publish_site", "serve_http", "serve_packages"] },
}

function roleColor(role: Role) { return role === "admission" ? "#8b5cf6" : role === "relay" ? "#06b6d4" : role === "storage" ? "#f59e0b" : role === "compute" ? "#ef4444" : "#3b82f6" }
function statusClass(status: NodeData["status"]) { return status === "running" ? "border-[var(--status-online)]/40 bg-[var(--status-online)]/10 text-[var(--status-online)]" : status === "available" ? "border-primary/40 bg-primary/10 text-primary" : "border-[var(--status-warning)]/40 bg-[var(--status-warning)]/10 text-[var(--status-warning)]" }

function FlowNode({ data, selected }: NodeProps<Node<NodeData>>) {
  const def = DEFS[data.role]
  const Icon = def.icon
  return <div className={cn("min-w-48 rounded-lg border bg-card shadow-xl", selected ? "border-primary ring-2 ring-primary/35" : "border-border")}>
    <div className="flex items-center gap-2 rounded-t-lg border-b border-border bg-secondary/50 px-3 py-2">
      <div className={cn("flex h-7 w-7 items-center justify-center rounded-md border border-border bg-background", def.color)}><Icon className="h-4 w-4" /></div>
      <div className="min-w-0 flex-1"><div className="truncate text-xs font-semibold text-foreground">{data.label}</div><div className="text-[10px] uppercase tracking-wide text-muted-foreground">{def.label} · {data.runtime}</div></div>
      <span className={cn("rounded border px-1.5 py-0.5 text-[9px] font-semibold uppercase", statusClass(data.status))}>{data.status}</span>
    </div>
    <div className="grid grid-cols-2 gap-1 px-0 py-2 text-[10px] text-muted-foreground">
      <div className="space-y-1">{def.inputs.map((port, idx) => <div key={port.id} className="relative flex h-6 items-center pl-3" title={port.description}><Handle id={`in:${port.id}`} type="target" position={Position.Left} style={{ top: 48 + idx * 24, background: roleColor(data.role), width: 10, height: 10 }} /><span>{port.label}</span></div>)}</div>
      <div className="space-y-1">{def.outputs.map((port, idx) => <div key={port.id} className="relative flex h-6 items-center justify-end pr-3" title={port.description}><span>{port.label}</span><Handle id={`out:${port.id}`} type="source" position={Position.Right} style={{ top: 48 + idx * 24, background: roleColor(data.role), width: 10, height: 10 }} /></div>)}</div>
    </div>
    <div className="border-t border-border px-3 py-2 text-[10px] text-muted-foreground"><div className="truncate">admission: <span className="text-foreground">{data.admissionNodeId}</span></div><div className="truncate">caps: {data.capabilities.join(", ")}</div></div>
  </div>
}

const nodeTypes = { edgerun: FlowNode }

const initialNodes: Node<NodeData>[] = [
  { id: "admission-personal", type: "edgerun", position: { x: 80, y: 180 }, data: { label: "Personal admission", role: "admission", runtime: "wasm", admissionNodeId: "self", endpoint: "admission://personal", budget: "low daily spend", capabilities: DEFS.admission.capabilities, discoverable: true, status: "running" } },
  { id: "admission-dao", type: "edgerun", position: { x: 410, y: 60 }, data: { label: "DAO admission", role: "admission", runtime: "native", admissionNodeId: "self", endpoint: `https://${DEFAULT_RENDEZVOUS}`, budget: "network default", capabilities: ["baseline_policy", "rendezvous"], discoverable: true, status: "available" } },
  { id: "relay-private", type: "edgerun", position: { x: 740, y: 180 }, data: { label: "Private relay", role: "relay", runtime: "native", admissionNodeId: "admission://personal", endpoint: "relay://127.0.0.1:8787", budget: "assigned by admission", capabilities: DEFS.relay.capabilities, discoverable: false, status: "draft" } },
  { id: "storage-home", type: "edgerun", position: { x: 1060, y: 80 }, data: { label: "Home storage", role: "storage", runtime: "native", admissionNodeId: "admission://personal", endpoint: "storage://localhost", budget: "assigned by admission", capabilities: DEFS.storage.capabilities, discoverable: false, status: "draft" } },
  { id: "compute-local", type: "edgerun", position: { x: 1060, y: 320 }, data: { label: "Local compute", role: "compute", runtime: "wasm", admissionNodeId: "admission://personal", endpoint: "compute://local", budget: "assigned by admission", capabilities: DEFS.compute.capabilities, discoverable: false, status: "draft" } },
]

const initialEdges: Edge[] = [
  { id: "personal-rendezvous", source: "admission-personal", target: "admission-dao", sourceHandle: "out:admission", targetHandle: "in:policy_source", label: "optional rendezvous", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "relay-follows", source: "relay-private", target: "admission-personal", label: "follows admission", animated: true, markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "storage-follows", source: "storage-home", target: "admission-personal", label: "follows admission", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "compute-follows", source: "compute-local", target: "admission-personal", label: "follows admission", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "admission-relay", source: "admission-personal", target: "relay-private", label: "assigns relay", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "relay-storage", source: "relay-private", target: "storage-home", label: "admitted storage", markerEnd: { type: MarkerType.ArrowClosed } },
  { id: "relay-compute", source: "relay-private", target: "compute-local", label: "admitted compute", markerEnd: { type: MarkerType.ArrowClosed } },
]

function Palette({ onAdd }: { onAdd: (role: Role) => void }) { return <aside className="flex w-56 shrink-0 flex-col border-r border-border bg-secondary/35"><div className="border-b border-border px-3 py-3 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">Node Palette</div><div className="flex-1 space-y-2 overflow-auto p-2">{Object.values(DEFS).map((def) => { const Icon = def.icon; return <button key={def.role} onClick={() => onAdd(def.role)} className="w-full rounded-lg border border-border bg-card p-2 text-left hover:bg-secondary"><div className="flex items-center gap-2"><div className={cn("flex h-7 w-7 items-center justify-center rounded-md border border-border bg-background", def.color)}><Icon className="h-4 w-4" /></div><div className="text-xs font-semibold text-foreground">{def.label}</div></div><div className="mt-1 pl-9 text-[11px] leading-4 text-muted-foreground">{def.desc}</div></button> })}</div><div className="border-t border-border p-3 text-[11px] leading-5 text-muted-foreground">Admission nodes rendezvous at <span className="font-mono text-foreground">{DEFAULT_RENDEZVOUS}</span> by default. Discovery can be disabled. Admission nodes share other admission nodes only if policy permits it.</div></aside> }

function Field({ label, children }: { label: string; children: React.ReactNode }) { return <label className="block"><div className="mb-1 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">{label}</div>{children}</label> }

function Config({ node, onUpdate, onDelete }: { node?: Node<NodeData>; onUpdate: (patch: Partial<NodeData>) => void; onDelete: () => void }) {
  if (!node) return <aside className="w-72 shrink-0 border-l border-border bg-secondary/35 p-3 text-sm text-muted-foreground">Select a node to configure it.</aside>
  const data = node.data
  const def = DEFS[data.role]
  return <aside className="flex w-72 shrink-0 flex-col border-l border-border bg-secondary/35"><div className="border-b border-border p-3"><div className="text-sm font-semibold text-foreground">{data.label}</div><div className="text-xs text-muted-foreground">{def.label} node</div></div><div className="flex-1 space-y-3 overflow-auto p-3"><Field label="Name"><input value={data.label} onChange={(e) => onUpdate({ label: e.target.value })} className="h-8 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground" /></Field><Field label="Runtime"><select value={data.runtime} onChange={(e) => onUpdate({ runtime: e.target.value as Runtime })} className="h-8 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground"><option value="wasm">WASM</option><option value="native">Native</option></select></Field><Field label="Admission node id/address"><input value={data.admissionNodeId} disabled={data.role === "admission"} onChange={(e) => onUpdate({ admissionNodeId: e.target.value })} className="h-8 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground disabled:opacity-60" /></Field><Field label="Endpoint"><input value={data.endpoint} onChange={(e) => onUpdate({ endpoint: e.target.value })} className="h-8 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground" /></Field><Field label="Budget"><input value={data.budget} onChange={(e) => onUpdate({ budget: e.target.value })} className="h-8 w-full rounded-md border border-border bg-background px-2 text-xs text-foreground" /></Field><label className="flex items-center gap-2 rounded-md border border-border bg-background px-2 py-2 text-xs text-foreground"><input type="checkbox" checked={data.discoverable} onChange={(e) => onUpdate({ discoverable: e.target.checked })} /> Share through rendezvous if policy permits</label><div className="rounded-lg border border-border bg-background p-3 text-xs leading-5 text-muted-foreground">{def.detail}</div><div className="rounded-lg border border-primary/20 bg-primary/5 p-3 text-xs leading-5 text-muted-foreground">Policy authority is the admission node id/address. Content-addressed policy documents can be attached later after storage is available.</div></div><div className="border-t border-border p-3"><button onClick={onDelete} className="inline-flex w-full items-center justify-center gap-2 rounded-md border border-destructive/30 px-3 py-2 text-xs text-destructive hover:bg-destructive/10"><Trash2 className="h-3.5 w-3.5" /> Delete node</button></div></aside>
}

function Step({ n, title, body }: { n: string; title: string; body: string }) { return <div className="border border-border p-4"><div className="mb-2 flex h-6 w-6 items-center justify-center rounded-full bg-primary/10 text-xs font-semibold text-primary">{n}</div><div className="text-xs font-semibold text-foreground">{title}</div><p className="mt-1 text-[11px] leading-5 text-muted-foreground">{body}</p></div> }

export function AdmissionNetworkEditor() {
  const [nodes, setNodes, onNodesChange] = useNodesState<NodeData>(initialNodes)
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges)
  const [selectedId, setSelectedId] = React.useState<string | null>("admission-personal")
  const [showHelp, setShowHelp] = React.useState(true)
  const selected = nodes.find((node) => node.id === selectedId)
  const onConnect = React.useCallback((connection: Connection) => setEdges((eds) => addEdge({ ...connection, markerEnd: { type: MarkerType.ArrowClosed }, label: "connected" }, eds)), [setEdges])
  const addNode = (role: Role) => { const def = DEFS[role]; const id = `${role}-${Date.now()}`; setNodes((current) => [...current, { id, type: "edgerun", position: { x: 180 + current.length * 28, y: 160 + current.length * 18 }, data: { label: `New ${def.label}`, role, runtime: def.defaultRuntime, admissionNodeId: role === "admission" ? "self" : "admission://personal", endpoint: `${role}://${id}`, budget: role === "admission" ? "policy defined" : "assigned by admission", capabilities: def.capabilities, discoverable: role === "admission", status: "draft" } }]); setSelectedId(id) }
  const updateSelected = (patch: Partial<NodeData>) => selected && setNodes((current) => current.map((node) => node.id === selected.id ? { ...node, data: { ...node.data, ...patch } } : node))
  const deleteSelected = () => { if (!selected) return; setNodes((current) => current.filter((node) => node.id !== selected.id)); setEdges((current) => current.filter((edge) => edge.source !== selected.id && edge.target !== selected.id)); setSelectedId(null) }

  return <div className="relative flex h-full overflow-hidden rounded-xl border border-border bg-background text-foreground"><Palette onAdd={addNode} /><main className="relative min-w-0 flex-1"><div className="absolute right-3 top-3 z-10"><button onClick={() => setShowHelp(true)} className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-card px-2 text-xs hover:bg-secondary"><HelpCircle className="h-3.5 w-3.5" />Help</button></div><ReactFlow nodes={nodes} edges={edges} nodeTypes={nodeTypes} onNodesChange={onNodesChange} onEdgesChange={onEdgesChange} onConnect={onConnect} onNodeClick={(_, node) => setSelectedId(node.id)} fitView colorMode="dark"><Background /><MiniMap nodeColor={(node) => roleColor((node.data as NodeData).role)} pannable zoomable /><Controls /></ReactFlow></main><Config node={selected} onUpdate={updateSelected} onDelete={deleteSelected} />{showHelp ? <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/55 p-4"><div className="max-w-lg rounded-xl border border-border bg-background shadow-2xl"><div className="p-5"><h2 className="text-lg font-semibold text-foreground">Admission Network Builder</h2><p className="mt-1 text-sm text-muted-foreground">Build the network visually. Add standalone nodes, wire ports, choose which admission node each node follows, and decide whether admission nodes may share discovery through the default rendezvous.</p></div><div className="grid grid-cols-2 border-y border-border text-xs"><Step n="1" title="Add nodes" body="Click a node in the palette." /><Step n="2" title="Wire ports" body="Drag from output handles to input handles." /><Step n="3" title="Configure" body="Edit admission address, endpoint, budget, and capabilities." /><Step n="4" title="Rendezvous" body="admission.edgerun.tech is only default discovery. It can be disabled." /></div><div className="flex items-center justify-between p-4"><p className="text-xs text-muted-foreground">Nodes share other admission nodes only if policy permits it.</p><button onClick={() => setShowHelp(false)} className="rounded-md bg-foreground px-4 py-2 text-xs font-semibold text-background">Get started</button></div></div></div> : null}</div>
}
