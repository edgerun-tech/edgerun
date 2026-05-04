"use client"

import { useMemo, useState } from "react"
import {
  ArrowRight,
  CheckCircle2,
  ChevronRight,
  Cloud,
  Database,
  FolderOpen,
  Github,
  Globe2,
  HardDrive,
  Lock,
  RefreshCw,
  ShieldCheck,
  Sparkles,
  Zap,
  type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"

type ConnectionState = "connected" | "recommended" | "available" | "needs-attention"

type StorageConnection = {
  id: string
  name: string
  plainName: string
  description: string
  icon: LucideIcon
  state: ConnectionState
  usedGb: number
  availableGb: number
  totalGb: number
  defaultAction: string
  nextStep: string
  safeDefault: string
  canDo: string[]
  willAsk: string[]
}

const connections: StorageConnection[] = [
  {
    id: "edgerun-storage",
    name: "EdgeRun Storage",
    plainName: "Private EdgeRun storage",
    description: "Your private storage. Apps use this first unless you choose another source.",
    icon: Database,
    state: "connected",
    usedGb: 18,
    availableGb: 238,
    totalGb: 256,
    defaultAction: "Open files",
    nextStep: "Ready to use",
    safeDefault: "Use for private app data, imported files, graph previews, and saved results.",
    canDo: ["Save private app data", "Store imported files", "Keep graph previews and generated results"],
    willAsk: ["Before deleting saved files", "Before sharing files outside EdgeRun", "Before giving another app write access"],
  },
  {
    id: "github",
    name: "GitHub",
    plainName: "Code and repositories",
    description: "Use your repositories for code search, project maps, and safe coding help.",
    icon: Github,
    state: "connected",
    usedGb: 2,
    availableGb: 48,
    totalGb: 50,
    defaultAction: "View repositories",
    nextStep: "Ready for code maps",
    safeDefault: "Read selected repositories. Ask before creating branches, commits, or pull requests.",
    canDo: ["Read selected repositories", "Build code maps", "Help explain files and relationships"],
    willAsk: ["Before creating a branch", "Before committing code", "Before opening or updating a pull request"],
  },
  {
    id: "google-drive",
    name: "Google Drive",
    plainName: "Documents and folders",
    description: "Connect selected folders so EdgeRun can organize, search, and visualize your documents.",
    icon: Cloud,
    state: "recommended",
    usedGb: 0,
    availableGb: 15,
    totalGb: 15,
    defaultAction: "Connect Google Drive",
    nextStep: "Recommended next",
    safeDefault: "Read selected folders only. Ask before editing, deleting, sharing, or exporting files.",
    canDo: ["See files in folders you choose", "Import names, text, and metadata", "Save private indexes to EdgeRun storage"],
    willAsk: ["Before editing Drive files", "Before deleting files", "Before sharing or exporting files"],
  },
  {
    id: "this-computer",
    name: "This computer",
    plainName: "Local files",
    description: "Choose one folder from this browser. EdgeRun cannot see the rest of your disk.",
    icon: HardDrive,
    state: "available",
    usedGb: 0,
    availableGb: 0,
    totalGb: 0,
    defaultAction: "Choose folder",
    nextStep: "Pick a folder",
    safeDefault: "Only the folder you choose becomes available. Full disk access is never granted.",
    canDo: ["Read one folder you choose", "Analyze files inside that folder", "Copy approved results into EdgeRun storage"],
    willAsk: ["Before choosing a folder", "Before editing local files", "Before deleting or moving anything"],
  },
  {
    id: "browser-cache",
    name: "Browser cache",
    plainName: "Local browser data",
    description: "Fast local space for drafts, previews, and temporary graph data on this device.",
    icon: Globe2,
    state: "connected",
    usedGb: 1.4,
    availableGb: 8.6,
    totalGb: 10,
    defaultAction: "Use cache",
    nextStep: "Ready for previews",
    safeDefault: "Good for temporary data. Important results should be saved to EdgeRun Storage.",
    canDo: ["Store temporary previews", "Keep local drafts", "Speed up visualizations"],
    willAsk: ["Before clearing important drafts", "Before copying cached data elsewhere", "Before another app reads cached data"],
  },
]

const tasks = [
  {
    title: "Map my codebase",
    text: "EdgeRun reads your selected repository and builds a visual map of files, functions, and relationships.",
    action: "Create code map",
    ready: true,
  },
  {
    title: "Organize my documents",
    text: "Connect selected Drive folders and build a searchable private index in your EdgeRun storage.",
    action: "Connect Drive first",
    ready: false,
  },
  {
    title: "Analyze a local folder",
    text: "Choose one folder from this computer. EdgeRun can inspect that folder only.",
    action: "Choose folder",
    ready: true,
  },
]

function stateLabel(state: ConnectionState) {
  switch (state) {
    case "connected": return "Connected"
    case "recommended": return "Recommended"
    case "available": return "Available"
    case "needs-attention": return "Needs attention"
  }
}

function stateClass(state: ConnectionState) {
  switch (state) {
    case "connected": return "border-[var(--status-online)]/30 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    case "recommended": return "border-primary/30 bg-primary/10 text-primary"
    case "available": return "border-border bg-secondary/60 text-foreground"
    case "needs-attention": return "border-[var(--status-warning)]/30 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
  }
}

function formatGb(value: number) {
  if (value === 0) return "Available after connection"
  if (value >= 10) return `${Math.round(value)} GB`
  return `${value.toFixed(1)} GB`
}

function StorageMeter({ source }: { source: StorageConnection }) {
  const percent = source.totalGb > 0 ? Math.min(100, Math.round((source.usedGb / source.totalGb) * 100)) : 0
  return (
    <div>
      <div className="mb-1 flex items-center justify-between text-[10px] text-muted-foreground">
        <span>{source.totalGb > 0 ? `${formatGb(source.availableGb)} available` : "Available after folder selection"}</span>
        <span>{source.totalGb > 0 ? `${percent}% used` : "not connected"}</span>
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-secondary">
        <div className="h-full rounded-full bg-primary" style={{ width: `${percent}%` }} />
      </div>
    </div>
  )
}

function ConnectionCard({ source, active, onSelect, onAction }: { source: StorageConnection; active: boolean; onSelect: () => void; onAction: () => void }) {
  const Icon = source.icon
  return (
    <div className={cn("rounded-2xl border bg-card p-4 transition-colors", active ? "border-primary/40 bg-primary/5" : "border-border")}> 
      <button onClick={onSelect} className="flex w-full items-start gap-3 text-left">
        <div className={cn("flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border", stateClass(source.state))}>
          <Icon className="h-5 w-5" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <div className="truncate text-sm font-semibold text-foreground">{source.name}</div>
            <span className={cn("rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase", stateClass(source.state))}>{stateLabel(source.state)}</span>
          </div>
          <div className="mt-1 line-clamp-2 text-xs leading-relaxed text-muted-foreground">{source.description}</div>
        </div>
        <ChevronRight className="mt-1 h-4 w-4 shrink-0 text-muted-foreground" />
      </button>
      <div className="mt-4"><StorageMeter source={source} /></div>
      <div className="mt-4 flex items-center gap-2">
        <button onClick={onAction} className="rounded-lg bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground hover:bg-primary/90">{source.defaultAction}</button>
        <button onClick={onSelect} className="rounded-lg bg-secondary px-3 py-1.5 text-xs font-medium text-secondary-foreground hover:bg-secondary/80">Details</button>
      </div>
    </div>
  )
}

function DetailPanel({ source, onAction }: { source: StorageConnection; onAction: () => void }) {
  const Icon = source.icon
  return (
    <aside className="flex w-80 shrink-0 flex-col border-l border-border bg-[var(--window-header)]/30">
      <div className="border-b border-border p-4">
        <div className="mb-3 flex items-start justify-between gap-3">
          <div className={cn("flex h-11 w-11 items-center justify-center rounded-xl border", stateClass(source.state))}><Icon className="h-5 w-5" /></div>
          <span className={cn("rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase", stateClass(source.state))}>{stateLabel(source.state)}</span>
        </div>
        <h3 className="text-sm font-semibold text-foreground">{source.plainName}</h3>
        <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{source.description}</p>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-4">
        <div className="mb-4 rounded-2xl border border-border bg-background/60 p-3">
          <div className="mb-2 flex items-center gap-2 text-xs font-semibold text-foreground"><HardDrive className="h-3.5 w-3.5" /> Available to use</div>
          <div className="text-lg font-semibold text-foreground">{source.totalGb > 0 ? `${formatGb(source.availableGb)} available` : "Choose a folder first"}</div>
          <div className="mt-2"><StorageMeter source={source} /></div>
        </div>

        <div className="mb-4">
          <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">What this lets EdgeRun do</div>
          <div className="space-y-2">
            {source.canDo.map((item) => (
              <div key={item} className="flex gap-2 rounded-lg border border-border bg-background/60 p-2 text-xs text-muted-foreground">
                <CheckCircle2 className="mt-0.5 h-3.5 w-3.5 shrink-0 text-[var(--status-online)]" />
                <span>{item}</span>
              </div>
            ))}
          </div>
        </div>

        <div className="mb-4">
          <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">EdgeRun will ask before</div>
          <div className="space-y-2">
            {source.willAsk.map((item) => (
              <div key={item} className="flex gap-2 rounded-lg border border-border bg-background/60 p-2 text-xs text-muted-foreground">
                <Lock className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" />
                <span>{item}</span>
              </div>
            ))}
          </div>
        </div>

        <div className="rounded-2xl border border-primary/25 bg-primary/10 p-3">
          <div className="mb-1 flex items-center gap-2 text-xs font-semibold text-primary"><ShieldCheck className="h-3.5 w-3.5" /> Recommended setting</div>
          <div className="text-xs leading-relaxed text-muted-foreground">{source.safeDefault}</div>
        </div>

        <div className="mt-4 grid grid-cols-1 gap-2">
          <button onClick={onAction} className="rounded-lg bg-primary px-3 py-2 text-xs font-semibold text-primary-foreground hover:bg-primary/90">{source.defaultAction}</button>
          <button className="rounded-lg bg-secondary px-3 py-2 text-xs font-medium text-secondary-foreground hover:bg-secondary/80">Use recommended setting</button>
          <button className="text-left text-[11px] text-muted-foreground hover:text-foreground">Show technical details</button>
        </div>
      </div>
    </aside>
  )
}

export function StorageDashboard() {
  const [selectedId, setSelectedId] = useState("google-drive")
  const [activity, setActivity] = useState("Google Drive is the recommended next connection.")
  const selected = connections.find((source) => source.id === selectedId) ?? connections[0]

  const connectedCount = connections.filter((source) => source.state === "connected").length
  const attentionCount = connections.filter((source) => source.state === "recommended" || source.state === "needs-attention").length
  const availableGb = useMemo(() => connections.reduce((sum, source) => sum + source.availableGb, 0), [])
  const recommended = connections.find((source) => source.state === "recommended") ?? connections.find((source) => source.state === "available") ?? connections[0]

  function runAction(source: StorageConnection) {
    setSelectedId(source.id)
    setActivity(`${source.defaultAction}: ${source.nextStep}. Safe defaults will be used.`)
  }

  return (
    <div className="flex h-full min-h-0 bg-background text-foreground">
      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex shrink-0 items-center justify-between border-b border-border bg-[var(--window-header)]/40 px-5 py-4">
          <div>
            <div className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wider text-primary"><Sparkles className="h-3.5 w-3.5" /> Storage setup</div>
            <h2 className="mt-1 text-xl font-semibold">Your data sources</h2>
            <p className="mt-1 max-w-2xl text-sm text-muted-foreground">Connect the places where your files, code, and documents live. EdgeRun will only use what you approve.</p>
          </div>
          <div className="flex items-center gap-2">
            <button onClick={() => runAction(recommended)} className="flex items-center gap-2 rounded-xl bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground hover:bg-primary/90">
              {recommended.defaultAction}
              <ArrowRight className="h-4 w-4" />
            </button>
            <button className="rounded-xl bg-secondary px-4 py-2 text-sm font-medium text-secondary-foreground hover:bg-secondary/80">See app access</button>
          </div>
        </header>

        <div className="grid shrink-0 grid-cols-3 gap-3 border-b border-border bg-background/70 p-4">
          <div className="rounded-2xl border border-border bg-card p-4">
            <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground"><CheckCircle2 className="h-4 w-4 text-[var(--status-online)]" /> Connected</div>
            <div className="text-2xl font-semibold">{connectedCount}</div>
            <div className="text-xs text-muted-foreground">sources ready</div>
          </div>
          <div className="rounded-2xl border border-border bg-card p-4">
            <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground"><HardDrive className="h-4 w-4 text-primary" /> Available to use</div>
            <div className="text-2xl font-semibold">{Math.round(availableGb)} GB</div>
            <div className="text-xs text-muted-foreground">known available space</div>
          </div>
          <div className="rounded-2xl border border-border bg-card p-4">
            <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground"><Zap className="h-4 w-4 text-primary" /> Next best action</div>
            <div className="truncate text-lg font-semibold">{recommended.defaultAction}</div>
            <div className="text-xs text-muted-foreground">{recommended.nextStep}</div>
          </div>
        </div>

        <div className="flex min-h-0 flex-1">
          <section className="min-w-0 flex-1 overflow-auto p-4">
            <div className="mb-4 rounded-2xl border border-primary/25 bg-primary/10 p-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <div className="mb-1 text-sm font-semibold text-foreground">Recommended setup</div>
                  <div className="max-w-2xl text-sm leading-relaxed text-muted-foreground">Start with safe defaults. EdgeRun can read selected data for search, organization, and visualization. It asks before editing, deleting, sharing, or sending anything.</div>
                </div>
                <button onClick={() => runAction(recommended)} className="shrink-0 rounded-lg bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground hover:bg-primary/90">{recommended.defaultAction}</button>
              </div>
            </div>

            <div className="mb-3 flex items-center justify-between">
              <h3 className="text-sm font-semibold">Connected and available sources</h3>
              <button onClick={() => setActivity("Refreshed source status. No permissions changed.")} className="flex items-center gap-1.5 rounded-lg bg-secondary px-2.5 py-1.5 text-xs font-medium hover:bg-secondary/80"><RefreshCw className="h-3.5 w-3.5" /> Refresh</button>
            </div>
            <div className="grid grid-cols-2 gap-3">
              {connections.map((source) => <ConnectionCard key={source.id} source={source} active={source.id === selectedId} onSelect={() => setSelectedId(source.id)} onAction={() => runAction(source)} />)}
            </div>

            <div className="mt-5">
              <h3 className="mb-3 text-sm font-semibold">What you can do next</h3>
              <div className="grid grid-cols-3 gap-3">
                {tasks.map((task) => (
                  <button key={task.title} onClick={() => setActivity(`${task.title}: ${task.ready ? "ready" : "connect required"}.`)} className="rounded-2xl border border-border bg-card p-4 text-left hover:bg-secondary/40">
                    <div className="mb-2 flex items-center justify-between gap-2">
                      <div className="text-sm font-semibold text-foreground">{task.title}</div>
                      <FolderOpen className="h-4 w-4 text-muted-foreground" />
                    </div>
                    <div className="min-h-[48px] text-xs leading-relaxed text-muted-foreground">{task.text}</div>
                    <div className={cn("mt-3 inline-flex rounded-lg px-2.5 py-1.5 text-xs font-semibold", task.ready ? "bg-primary text-primary-foreground" : "bg-secondary text-secondary-foreground")}>{task.action}</div>
                  </button>
                ))}
              </div>
            </div>
          </section>

          <DetailPanel source={selected} onAction={() => runAction(selected)} />
        </div>

        <footer className="flex h-9 shrink-0 items-center justify-between border-t border-border bg-[var(--window-header)]/50 px-4 text-[11px] text-muted-foreground">
          <span>{activity}</span>
          <span>Advanced details are hidden until needed.</span>
        </footer>
      </main>
    </div>
  )
}
