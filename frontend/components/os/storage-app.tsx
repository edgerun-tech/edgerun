"use client"

import { useMemo, useState } from "react"
import {
  Activity,
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  Code2,
  Database,
  FileCode,
  FolderGit2,
  Globe2,
  HardDrive,
  Network,
  Plus,
  RefreshCw,
  Search,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  UploadCloud,
  Workflow,
  XCircle,
  type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"

type StorageTab = "sources" | "pipelines" | "permissions" | "runtime" | "audit"
type ConnectorKind = "edgerun-vfs" | "google-drive" | "github" | "browser-storage" | "local-disk" | "custom"
type SourceStatus = "connected" | "limited" | "review" | "offline" | "blocked"
type PermissionMode = "none" | "read" | "read-write" | "ask-every-time"
type PipelineStatus = "draft" | "ready" | "running" | "blocked"

type StorageSource = {
  id: string
  name: string
  subtitle: string
  kind: ConnectorKind
  icon: LucideIcon
  status: SourceStatus
  permission: PermissionMode
  mount: string
  scope: string
  adapter: string
  sync: string
  lastSeen: string
  risk: "low" | "medium" | "high"
  details: string[]
}

type Pipeline = {
  id: string
  name: string
  status: PipelineStatus
  sourceIds: string[]
  transform: string
  sink: string
  permissionSummary: string
  runtime: string
  lastRun: string
}

type AuditEntry = {
  id: string
  time: string
  title: string
  detail: string
  status: SourceStatus | PipelineStatus
}

const tabs: { id: StorageTab; label: string; icon: LucideIcon }[] = [
  { id: "sources", label: "Sources", icon: Database },
  { id: "pipelines", label: "Pipelines", icon: Workflow },
  { id: "permissions", label: "Permissions", icon: ShieldCheck },
  { id: "runtime", label: "Runtime", icon: Code2 },
  { id: "audit", label: "Audit", icon: Activity },
]

const initialSources: StorageSource[] = [
  {
    id: "edgerun-vfs",
    name: "EdgeRun Virtual Filesystem",
    subtitle: "Unified mount table over node-backed storage",
    kind: "edgerun-vfs",
    icon: Network,
    status: "connected",
    permission: "read-write",
    mount: "edgerun://vfs/ken",
    scope: "/lifegraph,/projects,/datasets",
    adapter: "edgerun-virtual-filesystem",
    sync: "event stream + content objects",
    lastSeen: "just now",
    risk: "low",
    details: [
      "Primary abstraction layer. Other sources should mount through this instead of bypassing policy.",
      "Objects enter the node stream as signed source references and content hashes.",
      "Write access should still be scoped per pipeline and confirmed for destructive edits.",
    ],
  },
  {
    id: "local-disk",
    name: "Local Disk",
    subtitle: "Browser File System Access API routed through EdgeRun VFS",
    kind: "local-disk",
    icon: HardDrive,
    status: "review",
    permission: "ask-every-time",
    mount: "file://user-selected-directory",
    scope: "explicit folder handle only",
    adapter: "browser-file-system-access → edgerun-vfs",
    sync: "manual import/export",
    lastSeen: "waiting for folder grant",
    risk: "high",
    details: [
      "Browser cannot access arbitrary disk paths. User must pick a directory handle.",
      "This is the correct privacy model: selected folders become capabilities, not global disk access.",
      "Destructive writes should always go through the permission controller first.",
    ],
  },
  {
    id: "google-drive",
    name: "Google Drive",
    subtitle: "OAuth connector mounted as a scoped dataset",
    kind: "google-drive",
    icon: UploadCloud,
    status: "limited",
    permission: "read",
    mount: "gdrive://ken/root",
    scope: "Drive metadata + selected folders",
    adapter: "google-drive-connector",
    sync: "incremental changes feed",
    lastSeen: "12 min ago",
    risk: "medium",
    details: [
      "Expose Google Drive as a mounted source, not as a special app silo.",
      "OAuth scopes should be visible in Trust Manager and tied to edgerun.tech authority.",
      "Pipeline reads should emit source hashes so transformations are reproducible.",
    ],
  },
  {
    id: "github",
    name: "GitHub",
    subtitle: "Repos, issues, PRs, releases, and code snapshots",
    kind: "github",
    icon: FolderGit2,
    status: "connected",
    permission: "read",
    mount: "github://Sylchi/edgerun_reference_core",
    scope: "selected repositories",
    adapter: "github-connector",
    sync: "webhook + polling fallback",
    lastSeen: "3 min ago",
    risk: "medium",
    details: [
      "Treat repos as versioned datasets for agent coding, graphing, and explanation.",
      "Write operations should be explicit text-edit proposals first, then user-approved commits.",
      "Good default pipeline: repo snapshot -> codelyzer -> graph index -> local visualization projection.",
    ],
  },
  {
    id: "browser-storage",
    name: "Browser Storage",
    subtitle: "localStorage, IndexedDB, OPFS, cache entries",
    kind: "browser-storage",
    icon: Globe2,
    status: "connected",
    permission: "read-write",
    mount: "browser://origin/edgerun.tech",
    scope: "current origin only",
    adapter: "browser-storage-adapter",
    sync: "local first, optional node backup",
    lastSeen: "just now",
    risk: "low",
    details: [
      "Best place for UI state, local drafts, cached graph projections, and pipeline templates.",
      "Origin isolation is already browser-enforced, but EdgeRun policy should still log access.",
      "Large data should prefer OPFS/IndexedDB instead of localStorage.",
    ],
  },
  {
    id: "custom-source",
    name: "Custom Connector",
    subtitle: "Any user-defined storage with a manifest and capability contract",
    kind: "custom",
    icon: SlidersHorizontal,
    status: "blocked",
    permission: "none",
    mount: "connector://pending",
    scope: "not granted",
    adapter: "manifest-required",
    sync: "disabled",
    lastSeen: "not connected",
    risk: "high",
    details: [
      "Custom storage should be defined by a small manifest: identity, mount URI, auth method, operations, and proof rules.",
      "Do not let custom connectors call arbitrary browser APIs directly.",
      "The connector must expose read/write/list/watch as policy-checkable host calls.",
    ],
  },
]

const initialPipelines: Pipeline[] = [
  {
    id: "repo-visualization",
    name: "GitHub repo → graph visualization",
    status: "ready",
    sourceIds: ["github", "edgerun-vfs"],
    transform: "codelyzer.indexRepo() -> graph.pack()",
    sink: "browser://origin/xray/projections",
    permissionSummary: "read repo, write local graph cache, ask before code edits",
    runtime: "WASI module + scoped hostcalls",
    lastRun: "not run yet",
  },
  {
    id: "drive-ingest",
    name: "Google Drive → lifegraph dataset",
    status: "draft",
    sourceIds: ["google-drive", "edgerun-vfs"],
    transform: "normalizeDocs() -> extractMetadata() -> signSourceRefs()",
    sink: "edgerun://vfs/ken/lifegraph/imports/google-drive",
    permissionSummary: "read selected Drive folders, ask before exporting or deleting",
    runtime: "WASI module, deterministic mode",
    lastRun: "draft",
  },
  {
    id: "local-analysis",
    name: "Local folder → private analysis sandbox",
    status: "blocked",
    sourceIds: ["local-disk", "browser-storage"],
    transform: "scanFiles() -> classify() -> buildTimeline()",
    sink: "browser://origin/private/local-analysis",
    permissionSummary: "folder handle required, destructive writes disabled",
    runtime: "WASI module, no network hostcalls",
    lastRun: "blocked by folder permission",
  },
]

function statusClass(status: SourceStatus | PipelineStatus | PermissionMode | StorageSource["risk"]) {
  switch (status) {
    case "connected":
    case "ready":
    case "running":
    case "read":
    case "read-write":
    case "low":
      return "border-[var(--status-online)]/30 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    case "limited":
    case "review":
    case "draft":
    case "ask-every-time":
    case "medium":
      return "border-[var(--status-warning)]/30 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
    case "offline":
    case "blocked":
    case "none":
    case "high":
      return "border-[var(--status-error)]/30 bg-[var(--status-error)]/10 text-[var(--status-error)]"
  }
}

function StatusBadge({ value }: { value: SourceStatus | PipelineStatus | PermissionMode | StorageSource["risk"] }) {
  return <span className={cn("rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(value))}>{value}</span>
}

function SourceRow({ source, active, onSelect }: { source: StorageSource; active: boolean; onSelect: () => void }) {
  const Icon = source.icon
  return (
    <button
      onClick={onSelect}
      className={cn(
        "flex w-full items-start gap-3 border-b border-border/60 px-3 py-3 text-left transition-colors hover:bg-secondary/60",
        active && "bg-primary/10",
      )}
    >
      <div className={cn("mt-0.5 flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-xl border", statusClass(source.status))}>
        <Icon className="h-4.5 w-4.5" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <div className="truncate text-sm font-medium text-foreground">{source.name}</div>
          <StatusBadge value={source.status} />
        </div>
        <div className="mt-0.5 truncate text-xs text-muted-foreground">{source.subtitle}</div>
        <div className="mt-1 flex min-w-0 items-center gap-2 text-[11px] text-muted-foreground/70">
          <span className="truncate font-mono">{source.mount}</span>
          <span>·</span>
          <span className="truncate">{source.scope}</span>
        </div>
      </div>
    </button>
  )
}

function PipelineCard({ pipeline, sources, active, onSelect }: { pipeline: Pipeline; sources: StorageSource[]; active: boolean; onSelect: () => void }) {
  const linked = pipeline.sourceIds.map((id) => sources.find((source) => source.id === id)?.name ?? id)
  return (
    <button
      onClick={onSelect}
      className={cn(
        "rounded-xl border border-border bg-card p-3 text-left transition-colors hover:bg-secondary/50",
        active && "border-primary/40 bg-primary/5",
      )}
    >
      <div className="mb-2 flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="truncate text-sm font-semibold text-foreground">{pipeline.name}</div>
          <div className="mt-0.5 truncate text-[11px] text-muted-foreground">{linked.join(" + ")}</div>
        </div>
        <StatusBadge value={pipeline.status} />
      </div>
      <div className="grid grid-cols-[1fr_auto_1fr_auto_1fr] items-center gap-2 rounded-lg border border-border bg-background/60 p-2 font-mono text-[10px] text-muted-foreground">
        <span className="truncate">sources</span>
        <ArrowRight className="h-3 w-3" />
        <span className="truncate">WASI transform</span>
        <ArrowRight className="h-3 w-3" />
        <span className="truncate">sink</span>
      </div>
      <div className="mt-2 line-clamp-2 text-xs text-muted-foreground">{pipeline.permissionSummary}</div>
    </button>
  )
}

function Inspector({
  source,
  pipeline,
  onAction,
}: {
  source: StorageSource | null
  pipeline: Pipeline | null
  onAction: (action: string) => void
}) {
  if (pipeline) {
    return (
      <aside className="flex w-80 flex-shrink-0 flex-col border-l border-border bg-[var(--window-header)]/30">
        <div className="border-b border-border p-4">
          <div className="mb-3 flex items-center justify-between gap-2">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl border border-primary/30 bg-primary/10 text-primary"><Workflow className="h-5 w-5" /></div>
            <StatusBadge value={pipeline.status} />
          </div>
          <h3 className="text-sm font-semibold text-foreground">{pipeline.name}</h3>
          <p className="mt-1 text-xs text-muted-foreground">{pipeline.runtime}</p>
        </div>
        <div className="min-h-0 flex-1 overflow-auto p-4">
          <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Pipeline contract</div>
          <div className="space-y-2 rounded-lg border border-border bg-background/60 p-2 font-mono text-[10px] text-muted-foreground">
            <div className="flex justify-between gap-2"><span>sources</span><span className="truncate text-foreground">{pipeline.sourceIds.join(", ")}</span></div>
            <div className="flex justify-between gap-2"><span>transform</span><span className="truncate text-foreground">{pipeline.transform}</span></div>
            <div className="flex justify-between gap-2"><span>sink</span><span className="truncate text-foreground">{pipeline.sink}</span></div>
            <div className="flex justify-between gap-2"><span>last run</span><span className="truncate text-foreground">{pipeline.lastRun}</span></div>
          </div>
          <div className="mt-4 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Permission summary</div>
          <div className="mt-2 rounded-lg border border-border bg-background/60 p-2 text-xs leading-relaxed text-muted-foreground">{pipeline.permissionSummary}</div>
          <div className="mt-5 grid grid-cols-2 gap-2">
            <button onClick={() => onAction("run pipeline")} className="rounded-md bg-primary px-2 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90">Run</button>
            <button onClick={() => onAction("simulate pipeline")} className="rounded-md bg-secondary px-2 py-1.5 text-xs font-medium hover:bg-secondary/80">Simulate</button>
            <button onClick={() => onAction("edit pipeline")} className="rounded-md bg-secondary px-2 py-1.5 text-xs font-medium hover:bg-secondary/80">Edit</button>
            <button onClick={() => onAction("delete pipeline")} className="rounded-md bg-secondary px-2 py-1.5 text-xs font-medium text-destructive hover:bg-secondary/80">Delete</button>
          </div>
        </div>
      </aside>
    )
  }

  if (!source) {
    return (
      <aside className="flex w-80 flex-shrink-0 flex-col items-center justify-center border-l border-border bg-[var(--window-header)]/30 text-xs text-muted-foreground">
        Select a source or pipeline
      </aside>
    )
  }

  const Icon = source.icon
  return (
    <aside className="flex w-80 flex-shrink-0 flex-col border-l border-border bg-[var(--window-header)]/30">
      <div className="border-b border-border p-4">
        <div className="mb-3 flex items-start justify-between gap-3">
          <div className={cn("flex h-10 w-10 items-center justify-center rounded-xl border", statusClass(source.status))}><Icon className="h-5 w-5" /></div>
          <div className="flex gap-1"><StatusBadge value={source.status} /><StatusBadge value={source.risk} /></div>
        </div>
        <h3 className="text-sm font-semibold text-foreground">{source.name}</h3>
        <p className="mt-1 text-xs text-muted-foreground">{source.subtitle}</p>
      </div>
      <div className="min-h-0 flex-1 overflow-auto p-4">
        <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Mount contract</div>
        <div className="mb-4 space-y-2 rounded-lg border border-border bg-background/60 p-2 font-mono text-[10px] text-muted-foreground">
          <div className="flex justify-between gap-2"><span>mount</span><span className="truncate text-foreground">{source.mount}</span></div>
          <div className="flex justify-between gap-2"><span>adapter</span><span className="truncate text-foreground">{source.adapter}</span></div>
          <div className="flex justify-between gap-2"><span>scope</span><span className="truncate text-foreground">{source.scope}</span></div>
          <div className="flex justify-between gap-2"><span>sync</span><span className="truncate text-foreground">{source.sync}</span></div>
          <div className="flex justify-between gap-2"><span>permission</span><span className="truncate text-foreground">{source.permission}</span></div>
        </div>
        <div className="mb-3 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Policy notes</div>
        <div className="space-y-2">
          {source.details.map((detail) => <div key={detail} className="rounded-lg border border-border bg-background/60 p-2 text-xs leading-relaxed text-muted-foreground">{detail}</div>)}
        </div>
        <div className="mt-5 grid grid-cols-2 gap-2">
          <button onClick={() => onAction("connect source")} className="rounded-md bg-primary px-2 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90">Connect</button>
          <button onClick={() => onAction("request permission")} className="rounded-md bg-secondary px-2 py-1.5 text-xs font-medium hover:bg-secondary/80">Permission</button>
          <button onClick={() => onAction("test source")} className="rounded-md bg-secondary px-2 py-1.5 text-xs font-medium hover:bg-secondary/80">Test</button>
          <button onClick={() => onAction("block source")} className="rounded-md bg-secondary px-2 py-1.5 text-xs font-medium text-destructive hover:bg-secondary/80">Block</button>
        </div>
      </div>
    </aside>
  )
}

function RuntimePanel() {
  const hostcalls = [
    ["storage.list", "List mounted source paths", "read"],
    ["storage.read", "Read bytes/records from approved source", "read"],
    ["storage.write", "Write result objects through VFS", "ask/write"],
    ["storage.watch", "Subscribe to connector change events", "read"],
    ["graph.emit", "Emit visualization nodes and edges", "write-local"],
    ["permission.request", "Ask user before sensitive action", "system"],
  ]
  return (
    <div className="grid h-full min-h-0 grid-cols-[1fr_320px] gap-3 p-3">
      <section className="min-h-0 overflow-auto rounded-xl border border-border bg-card p-4">
        <div className="mb-1 flex items-center gap-2 text-sm font-semibold text-foreground"><FileCode className="h-4 w-4 text-primary" />WASI data runtime</div>
        <p className="mb-4 text-xs text-muted-foreground">Storage adapters should not feed raw credentials or unrestricted handles into modules. Each module receives capability-scoped hostcalls and signed source references.</p>
        <div className="space-y-2">
          {hostcalls.map(([name, description, mode]) => (
            <div key={name} className="grid grid-cols-[150px_1fr_88px] items-center gap-3 rounded-lg border border-border bg-background/60 p-2 text-xs">
              <span className="font-mono text-foreground">{name}</span>
              <span className="text-muted-foreground">{description}</span>
              <span className="rounded-md border border-border bg-secondary/60 px-1.5 py-0.5 text-center font-mono text-[10px] text-muted-foreground">{mode}</span>
            </div>
          ))}
        </div>
      </section>
      <section className="rounded-xl border border-border bg-card p-4">
        <div className="mb-3 text-sm font-semibold text-foreground">Correct boundary</div>
        <div className="space-y-2 text-xs leading-relaxed text-muted-foreground">
          <div className="rounded-lg border border-border bg-background/60 p-2">Connectors authenticate and mount data.</div>
          <div className="rounded-lg border border-border bg-background/60 p-2">Permission controller approves exact source, operation, sink, and transform.</div>
          <div className="rounded-lg border border-border bg-background/60 p-2">WASI modules receive only approved hostcalls.</div>
          <div className="rounded-lg border border-border bg-background/60 p-2">Xray/GL visualization consumes emitted graph projections, not raw secrets.</div>
        </div>
      </section>
    </div>
  )
}

export function StorageApp() {
  const [tab, setTab] = useState<StorageTab>("sources")
  const [sources, setSources] = useState(initialSources)
  const [pipelines, setPipelines] = useState(initialPipelines)
  const [query, setQuery] = useState("")
  const [selectedSourceId, setSelectedSourceId] = useState(initialSources[0]?.id ?? "")
  const [selectedPipelineId, setSelectedPipelineId] = useState(initialPipelines[0]?.id ?? "")
  const [audit, setAudit] = useState<AuditEntry[]>([
    { id: "boot", time: "boot", title: "Storage app loaded local connector snapshot", detail: "No external action executed", status: "connected" },
  ])

  const selectedSource = sources.find((source) => source.id === selectedSourceId) ?? sources[0] ?? null
  const selectedPipeline = pipelines.find((pipeline) => pipeline.id === selectedPipelineId) ?? pipelines[0] ?? null
  const filteredSources = useMemo(() => {
    const q = query.trim().toLowerCase()
    return sources.filter((source) => !q || `${source.name} ${source.subtitle} ${source.mount} ${source.scope}`.toLowerCase().includes(q))
  }, [query, sources])
  const filteredPipelines = useMemo(() => {
    const q = query.trim().toLowerCase()
    return pipelines.filter((pipeline) => !q || `${pipeline.name} ${pipeline.transform} ${pipeline.sink}`.toLowerCase().includes(q))
  }, [query, pipelines])

  function log(title: string, detail: string, status: AuditEntry["status"] = "connected") {
    setAudit((prev) => [{ id: `${Date.now()}-${Math.random()}`, time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" }), title, detail, status }, ...prev].slice(0, 12))
  }

  function handleAction(action: string) {
    if (action.includes("block source") && selectedSource) {
      setSources((prev) => prev.map((source) => source.id === selectedSource.id ? { ...source, status: "blocked", permission: "none", lastSeen: "blocked locally" } : source))
      log(`Blocked ${selectedSource.name}`, "Future access requires a new explicit grant", "blocked")
      return
    }
    if ((action.includes("connect") || action.includes("test")) && selectedSource) {
      setSources((prev) => prev.map((source) => source.id === selectedSource.id ? { ...source, status: "connected", lastSeen: "just now" } : source))
      log(`Verified ${selectedSource.name}`, selectedSource.mount, "connected")
      return
    }
    if (action.includes("permission") && selectedSource) {
      setSources((prev) => prev.map((source) => source.id === selectedSource.id ? { ...source, status: "review", permission: "ask-every-time" } : source))
      log(`Permission review requested for ${selectedSource.name}`, "Routed to permission controller", "review")
      return
    }
    if (action.includes("run pipeline") && selectedPipeline) {
      const blockedSource = selectedPipeline.sourceIds.map((id) => sources.find((source) => source.id === id)).find((source) => !source || source.status === "blocked" || source.status === "offline" || source.status === "review")
      if (blockedSource) {
        setPipelines((prev) => prev.map((pipeline) => pipeline.id === selectedPipeline.id ? { ...pipeline, status: "blocked", lastRun: "blocked by source permission" } : pipeline))
        log(`Blocked ${selectedPipeline.name}`, `Permission required for ${blockedSource.name}`, "blocked")
        return
      }
      setPipelines((prev) => prev.map((pipeline) => pipeline.id === selectedPipeline.id ? { ...pipeline, status: "running", lastRun: "just now" } : pipeline))
      log(`Started ${selectedPipeline.name}`, selectedPipeline.transform, "running")
      return
    }
    if (action.includes("simulate") && selectedPipeline) {
      log(`Simulated ${selectedPipeline.name}`, "Dry run: no source writes, no external mutation", "ready")
      return
    }
    if (action.includes("delete pipeline") && selectedPipeline) {
      const next = pipelines.filter((pipeline) => pipeline.id !== selectedPipeline.id)
      setPipelines(next)
      setSelectedPipelineId(next[0]?.id ?? "")
      log(`Deleted ${selectedPipeline.name}`, "Local draft removed", "draft")
      return
    }
    log(action, "UI action recorded", "review")
  }

  function addPipeline() {
    const id = `pipeline-${Date.now()}`
    const next: Pipeline = {
      id,
      name: "New configurable data pipeline",
      status: "draft",
      sourceIds: [selectedSource?.id ?? "edgerun-vfs"],
      transform: "input.records() -> transform() -> graph.emit()",
      sink: "browser://origin/pipelines/draft",
      permissionSummary: "draft: no permission granted yet",
      runtime: "WASI module",
      lastRun: "never",
    }
    setPipelines((prev) => [next, ...prev])
    setSelectedPipelineId(id)
    setTab("pipelines")
    log("Created pipeline draft", next.name, "draft")
  }

  const counts = useMemo(() => ({
    connected: sources.filter((source) => source.status === "connected").length,
    review: sources.filter((source) => source.status === "review" || source.status === "limited").length,
    blocked: sources.filter((source) => source.status === "blocked" || source.status === "offline").length,
  }), [sources])

  return (
    <div className="flex h-full min-h-0 bg-background text-foreground">
      <aside className="flex w-52 flex-shrink-0 flex-col border-r border-border bg-[var(--window-header)]/70">
        <div className="border-b border-border p-3">
          <div className="flex items-center gap-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/15 text-primary"><Database className="h-5 w-5" /></div>
            <div className="min-w-0"><div className="truncate text-sm font-semibold">Storage</div><div className="text-[10px] text-muted-foreground">connect · pipe · prove</div></div>
          </div>
        </div>
        <nav className="flex-1 space-y-1 p-2">
          {tabs.map((item) => {
            const Icon = item.icon
            return (
              <button key={item.id} onClick={() => setTab(item.id)} className={cn("flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-xs font-medium transition-colors", tab === item.id ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-secondary hover:text-foreground")}>
                <Icon className="h-3.5 w-3.5" /><span className="min-w-0 flex-1 truncate">{item.label}</span>
              </button>
            )
          })}
        </nav>
        <div className="border-t border-border p-3 text-[10px] text-muted-foreground">
          <div>{counts.connected} connected · {counts.review} review · {counts.blocked} blocked</div>
        </div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 flex-shrink-0 items-center justify-between border-b border-border bg-[var(--window-header)]/40 px-3">
          <div>
            <h2 className="text-sm font-semibold">Configurable data pipelines</h2>
            <p className="text-[11px] text-muted-foreground">Mount any source, scope it through permission control, then feed safe data into WASI transforms and visualization.</p>
          </div>
          <div className="flex items-center gap-2">
            <div className="relative"><Search className="absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-muted-foreground" /><input value={query} onChange={(event) => setQuery(event.target.value)} className="h-8 w-56 rounded-md border border-border bg-background pl-7 pr-2 text-xs outline-none focus:border-primary/50" placeholder="Search sources or pipelines..." /></div>
            <button onClick={() => log("Refreshed connector snapshot", "No external sync performed in UI mock", "connected")} className="flex h-8 items-center gap-1.5 rounded-md bg-secondary px-2 text-xs font-medium hover:bg-secondary/80"><RefreshCw className="h-3.5 w-3.5" />Refresh</button>
            <button onClick={addPipeline} className="flex h-8 items-center gap-1.5 rounded-md bg-primary px-2 text-xs font-medium text-primary-foreground hover:bg-primary/90"><Plus className="h-3.5 w-3.5" />Pipeline</button>
          </div>
        </header>

        <div className="flex min-h-0 flex-1">
          <section className="min-w-0 flex-1 overflow-hidden">
            {tab === "sources" && (
              <div className="grid h-full min-h-0 grid-cols-[minmax(320px,1fr)_minmax(340px,0.9fr)]">
                <div className="min-h-0 overflow-auto border-r border-border">
                  {filteredSources.map((source) => <SourceRow key={source.id} source={source} active={source.id === selectedSourceId} onSelect={() => setSelectedSourceId(source.id)} />)}
                </div>
                <div className="min-h-0 overflow-auto p-3">
                  <div className="mb-3 grid grid-cols-3 gap-3">
                    <div className="rounded-xl border border-border bg-card p-3"><CheckCircle2 className="mb-2 h-4 w-4 text-[var(--status-online)]" /><div className="text-lg font-semibold">{counts.connected}</div><div className="text-[10px] text-muted-foreground">connected</div></div>
                    <div className="rounded-xl border border-border bg-card p-3"><AlertTriangle className="mb-2 h-4 w-4 text-[var(--status-warning)]" /><div className="text-lg font-semibold">{counts.review}</div><div className="text-[10px] text-muted-foreground">limited/review</div></div>
                    <div className="rounded-xl border border-border bg-card p-3"><XCircle className="mb-2 h-4 w-4 text-[var(--status-error)]" /><div className="text-lg font-semibold">{counts.blocked}</div><div className="text-[10px] text-muted-foreground">blocked/offline</div></div>
                  </div>
                  <div className="rounded-xl border border-border bg-card p-4">
                    <div className="mb-2 flex items-center gap-2 text-sm font-semibold"><Sparkles className="h-4 w-4 text-primary" />Storage model</div>
                    <div className="space-y-2 text-xs leading-relaxed text-muted-foreground">
                      <p>Every connector becomes a mounted source with a capability contract. Pipelines never receive credentials; they receive scoped reads/writes from the host.</p>
                      <p>The strong move is to route Google Drive, GitHub, local disk handles, OPFS, and future connectors through the EdgeRun VFS namespace so policy, audit, hashes, and visualization all share one model.</p>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {tab === "pipelines" && (
              <div className="grid h-full min-h-0 grid-cols-2 gap-3 overflow-auto p-3">
                {filteredPipelines.map((pipeline) => <PipelineCard key={pipeline.id} pipeline={pipeline} sources={sources} active={pipeline.id === selectedPipelineId} onSelect={() => setSelectedPipelineId(pipeline.id)} />)}
              </div>
            )}

            {tab === "permissions" && (
              <div className="h-full overflow-auto p-3">
                <div className="grid gap-3">
                  {sources.map((source) => (
                    <div key={source.id} className="grid grid-cols-[44px_1fr_110px_110px_90px] items-center gap-3 rounded-xl border border-border bg-card p-3">
                      <div className={cn("flex h-10 w-10 items-center justify-center rounded-xl border", statusClass(source.risk))}><source.icon className="h-4 w-4" /></div>
                      <div className="min-w-0"><div className="truncate text-sm font-medium">{source.name}</div><div className="truncate font-mono text-[10px] text-muted-foreground">{source.mount}</div></div>
                      <StatusBadge value={source.permission} />
                      <StatusBadge value={source.risk} />
                      <button onClick={() => { setSelectedSourceId(source.id); handleAction("request permission") }} className="rounded-md bg-secondary px-2 py-1.5 text-xs font-medium hover:bg-secondary/80">Review</button>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {tab === "runtime" && <RuntimePanel />}

            {tab === "audit" && (
              <div className="h-full overflow-auto p-3">
                <div className="space-y-2">
                  {audit.map((entry) => (
                    <div key={entry.id} className="grid grid-cols-[86px_1fr_110px] items-start gap-3 rounded-xl border border-border bg-card p-3 text-xs">
                      <div className="font-mono text-[10px] text-muted-foreground">{entry.time}</div>
                      <div><div className="font-medium text-foreground">{entry.title}</div><div className="mt-1 text-muted-foreground">{entry.detail}</div></div>
                      <StatusBadge value={entry.status} />
                    </div>
                  ))}
                </div>
              </div>
            )}
          </section>

          <Inspector source={tab === "sources" || tab === "permissions" ? selectedSource : null} pipeline={tab === "pipelines" ? selectedPipeline : null} onAction={handleAction} />
        </div>
      </main>
    </div>
  )
}
