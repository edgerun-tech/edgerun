"use client"

import { useState, useRef, useCallback, useEffect } from "react"
import {
  Folder, FileText, FileImage, FileVideo, FileAudio, FileCode, FileArchive,
  ChevronRight, ChevronDown, Grid3x3, List, ArrowLeft, ArrowRight, ArrowUp,
  Search, Star, HardDrive, Home, Trash2, Copy, Scissors, Clipboard, Eye,
  MoreHorizontal, RefreshCw, SortAsc, Info, Download, Upload,
} from "lucide-react"
import { cn } from "@/lib/utils"

// ─── Types ──────────────────────────────────────────────────────────────────

type FileType = "folder" | "text" | "image" | "video" | "audio" | "code" | "archive" | "unknown"

interface FSEntry {
  id: string
  name: string
  type: FileType
  size?: string
  modified: string
  children?: FSEntry[]
  preview?: string   // data url or description for preview panel
  starred?: boolean
  mimeHint?: string
}

type ViewMode = "grid" | "list"
type SortKey = "name" | "modified" | "type" | "size"

// ─── Mock Filesystem ─────────────────────────────────────────────────────────

const FS_ROOT: FSEntry[] = [
  {
    id: "home", name: "Home", type: "folder", modified: "2026-04-28", children: [
      {
        id: "docs", name: "Documents", type: "folder", modified: "2026-04-27", children: [
          { id: "doc1", name: "whitepaper.md", type: "text", size: "14 KB", modified: "2026-04-26", preview: "# Edgerun Whitepaper\n\nEdgerun is a distributed WASI runtime for edge compute. Nodes are authenticated via WebAuthn biometrics and communicate over encrypted WebRTC channels...", mimeHint: "text/markdown" },
          { id: "doc2", name: "onboarding.txt", type: "text", size: "3 KB", modified: "2026-04-20", preview: "Welcome to Edgerun.\n\nThis guide walks you through your first deployment.", mimeHint: "text/plain" },
          { id: "doc3", name: "LICENSE", type: "text", size: "1 KB", modified: "2026-01-01", preview: "MIT License\n\nCopyright (c) 2026 Edgerun", mimeHint: "text/plain" },
        ]
      },
      {
        id: "images", name: "Images", type: "folder", modified: "2026-04-25", children: [
          { id: "img1", name: "globe-render.png", type: "image", size: "2.4 MB", modified: "2026-04-24", preview: "globe-render" },
          { id: "img2", name: "node-diagram.svg", type: "image", size: "48 KB", modified: "2026-04-22", preview: "node-diagram" },
          { id: "img3", name: "screenshot.jpg", type: "image", size: "1.1 MB", modified: "2026-04-18", preview: "screenshot" },
        ]
      },
      {
        id: "projects", name: "Projects", type: "folder", modified: "2026-04-29", children: [
          {
            id: "proj1", name: "edge-agent", type: "folder", modified: "2026-04-29", children: [
              { id: "main-rs", name: "main.rs", type: "code", size: "8 KB", modified: "2026-04-29", preview: 'use wasmtime::*;\n\nfn main() -> anyhow::Result<()> {\n    let engine = Engine::default();\n    let module = Module::from_file(&engine, "agent.wasm")?;\n    // ...\n    Ok(())\n}', mimeHint: "text/rust" },
              { id: "cargo", name: "Cargo.toml", type: "code", size: "1 KB", modified: "2026-04-28", preview: '[package]\nname = "edge-agent"\nversion = "0.1.0"\nedition = "2021"', mimeHint: "text/toml" },
              { id: "agent-wasm", name: "agent.wasm", type: "archive", size: "312 KB", modified: "2026-04-29" },
            ]
          },
          { id: "deploy-sh", name: "deploy.sh", type: "code", size: "2 KB", modified: "2026-04-27", preview: "#!/bin/bash\nset -e\necho 'Deploying edge agent...'\nedgerun push ./agent.wasm --nodes 12", mimeHint: "text/bash" },
        ]
      },
      {
        id: "media", name: "Media", type: "folder", modified: "2026-04-10", children: [
          { id: "vid1", name: "demo-call.mp4", type: "video", size: "48 MB", modified: "2026-04-10" },
          { id: "aud1", name: "ambient.wav", type: "audio", size: "22 MB", modified: "2026-03-30" },
        ]
      },
      { id: "readme", name: "README.md", type: "text", size: "2 KB", modified: "2026-04-29", preview: "# My Edgerun Workspace\n\nThis is my personal edge compute workspace.", starred: true },
    ]
  },
  {
    id: "system", name: "System", type: "folder", modified: "2026-04-01", children: [
      { id: "runtime", name: "runtime.wasm", type: "archive", size: "1.2 MB", modified: "2026-04-01" },
      { id: "config-json", name: "config.json", type: "code", size: "4 KB", modified: "2026-04-28", preview: '{\n  "nodeId": "edge-0xdeadbeef",\n  "region": "eu-west",\n  "maxRAM": "2GB",\n  "peers": 12\n}', mimeHint: "application/json" },
    ]
  },
  {
    id: "trash", name: "Trash", type: "folder", modified: "2026-04-15", children: [
      { id: "old-log", name: "old-log.txt", type: "text", size: "88 KB", modified: "2026-04-15" },
    ]
  },
]

// ─── Icon helper ─────────────────────────────────────────────────────────────

function FileIcon({ type, className }: { type: FileType; className?: string }) {
  const cls = cn("shrink-0", className)
  switch (type) {
    case "folder": return <Folder className={cn(cls, "text-[oklch(0.7_0.18_80)]")} />
    case "image":  return <FileImage className={cn(cls, "text-[oklch(0.6_0.2_270)]")} />
    case "video":  return <FileVideo className={cn(cls, "text-[oklch(0.6_0.2_310)]")} />
    case "audio":  return <FileAudio className={cn(cls, "text-[oklch(0.65_0.2_200)]")} />
    case "code":   return <FileCode className={cn(cls, "text-primary")} />
    case "archive":return <FileArchive className={cn(cls, "text-[oklch(0.65_0.18_45)]")} />
    default:       return <FileText className={cn(cls, "text-muted-foreground")} />
  }
}

// ─── Preview Panel ────────────────────────────────────────────────────────────

function PreviewPanel({ entry }: { entry: FSEntry | null }) {
  if (!entry) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground">
        <Eye className="h-10 w-10 opacity-20" />
        <span className="text-xs">Select a file to preview</span>
      </div>
    )
  }

  const isText = ["text", "code"].includes(entry.type) && entry.preview
  const isImage = entry.type === "image"
  const isMedia = ["video", "audio"].includes(entry.type)

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center gap-2 border-b border-[var(--window-border)] p-3">
        <FileIcon type={entry.type} className="h-5 w-5" />
        <div className="min-w-0">
          <p className="truncate text-sm font-medium text-foreground">{entry.name}</p>
          <p className="text-[10px] text-muted-foreground">{entry.size ?? "—"} · {entry.modified}</p>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-3">
        {isText && (
          <pre className="whitespace-pre-wrap font-mono text-[11px] leading-relaxed text-muted-foreground">
            {entry.preview}
          </pre>
        )}
        {isImage && (
          <div className="flex h-full flex-col items-center justify-center gap-3">
            <div className="flex h-32 w-full items-center justify-center rounded-lg bg-secondary/30">
              <FileImage className="h-12 w-12 text-muted-foreground/30" />
            </div>
            <span className="text-[10px] text-muted-foreground">{entry.name}</span>
          </div>
        )}
        {isMedia && (
          <div className="flex h-full flex-col items-center justify-center gap-2">
            <FileIcon type={entry.type} className="h-12 w-12 opacity-30" />
            <span className="text-[10px] text-muted-foreground">{entry.type === "video" ? "Video file" : "Audio file"}</span>
            <span className="text-[10px] text-muted-foreground">{entry.size}</span>
          </div>
        )}
        {entry.type === "folder" && (
          <div className="flex h-full flex-col items-center justify-center gap-2">
            <Folder className="h-12 w-12 text-[oklch(0.7_0.18_80)]/30" />
            <span className="text-[10px] text-muted-foreground">{entry.children?.length ?? 0} items</span>
          </div>
        )}
        {entry.type === "archive" && (
          <div className="flex h-full flex-col items-center justify-center gap-2">
            <FileArchive className="h-12 w-12 opacity-30" />
            <span className="text-[10px] text-muted-foreground">{entry.size}</span>
          </div>
        )}
      </div>

      {/* Info rows */}
      <div className="border-t border-[var(--window-border)] p-3 space-y-1">
        {[
          ["Kind", entry.type],
          ["Size", entry.size ?? "—"],
          ["Modified", entry.modified],
        ].map(([k, v]) => (
          <div key={k} className="flex justify-between text-[10px]">
            <span className="text-muted-foreground">{k}</span>
            <span className="text-foreground font-mono">{v}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

// ─── Context Menu ─────────────────────────────────────────────────────────────

interface ContextMenuState { x: number; y: number; entry: FSEntry | null }

function ContextMenu({
  state, onClose, onAction,
}: {
  state: ContextMenuState
  onClose: () => void
  onAction: (action: string, entry: FSEntry | null) => void
}) {
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose()
    }
    document.addEventListener("mousedown", handler)
    return () => document.removeEventListener("mousedown", handler)
  }, [onClose])

  const isFolder = state.entry?.type === "folder"

  const items = state.entry ? [
    { label: "Open", icon: <Eye className="h-3.5 w-3.5" />, action: "open" },
    { label: "Get Info", icon: <Info className="h-3.5 w-3.5" />, action: "info" },
    null,
    ...(state.entry.starred
      ? [{ label: "Unstar", icon: <Star className="h-3.5 w-3.5" />, action: "unstar" }]
      : [{ label: "Star", icon: <Star className="h-3.5 w-3.5" />, action: "star" }]
    ),
    null,
    { label: "Copy", icon: <Copy className="h-3.5 w-3.5" />, action: "copy" },
    { label: "Cut", icon: <Scissors className="h-3.5 w-3.5" />, action: "cut" },
    { label: isFolder ? "Compress Folder" : "Download", icon: <Download className="h-3.5 w-3.5" />, action: "download" },
    null,
    { label: "Move to Trash", icon: <Trash2 className="h-3.5 w-3.5" />, action: "trash", danger: true },
  ] : [
    { label: "New Folder", icon: <Folder className="h-3.5 w-3.5" />, action: "new-folder" },
    { label: "New File", icon: <FileText className="h-3.5 w-3.5" />, action: "new-file" },
    null,
    { label: "Paste", icon: <Clipboard className="h-3.5 w-3.5" />, action: "paste" },
    { label: "Upload Here", icon: <Upload className="h-3.5 w-3.5" />, action: "upload" },
    null,
    { label: "Refresh", icon: <RefreshCw className="h-3.5 w-3.5" />, action: "refresh" },
    { label: "Sort By", icon: <SortAsc className="h-3.5 w-3.5" />, action: "sort" },
  ]

  return (
    <div
      ref={ref}
      className="fixed z-[999] min-w-44 overflow-hidden rounded-lg border border-[var(--window-border)] bg-[var(--window-bg)]/95 py-1 shadow-2xl backdrop-blur-md"
      style={{ left: state.x, top: state.y }}
    >
      {items.map((item, i) =>
        item === null ? (
          <div key={i} className="my-1 border-t border-[var(--window-border)]" />
        ) : (
          <button
            key={item.action}
            onClick={() => { onAction(item.action, state.entry); onClose() }}
            className={cn(
              "flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-xs transition-colors",
              (item as any).danger
                ? "text-destructive hover:bg-destructive/10"
                : "text-foreground hover:bg-secondary"
            )}
          >
            <span className="text-muted-foreground">{item.icon}</span>
            {item.label}
          </button>
        )
      )}
    </div>
  )
}

// ─── Sidebar ──────────────────────────────────────────────────────────────────

function Sidebar({
  starred, currentPath, onNavigate,
}: {
  starred: FSEntry[]
  currentPath: string[]
  onNavigate: (path: string[]) => void
}) {
  const sections = [
    { label: "Locations", items: [
      { icon: <Home className="h-3.5 w-3.5" />, label: "Home", path: ["home"] },
      { icon: <HardDrive className="h-3.5 w-3.5" />, label: "System", path: ["system"] },
      { icon: <Trash2 className="h-3.5 w-3.5" />, label: "Trash", path: ["trash"] },
    ]},
    ...(starred.length > 0 ? [{ label: "Starred", items: starred.map(e => ({
      icon: <Star className="h-3.5 w-3.5 fill-current text-[oklch(0.75_0.18_80)]" />,
      label: e.name,
      path: ["home"],
    }))}] : []),
  ]

  return (
    <div className="flex h-full w-36 flex-col border-r border-[var(--window-border)] py-2">
      {sections.map((sec) => (
        <div key={sec.label} className="mb-3">
          <p className="px-3 pb-1 font-mono text-[9px] font-semibold uppercase tracking-widest text-muted-foreground/60">
            {sec.label}
          </p>
          {sec.items.map((item) => {
            const active = JSON.stringify(currentPath) === JSON.stringify(item.path)
            return (
              <button
                key={item.label}
                onClick={() => onNavigate(item.path)}
                className={cn(
                  "flex w-full items-center gap-2 px-3 py-1.5 text-xs transition-colors",
                  active ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-secondary hover:text-foreground"
                )}
              >
                {item.icon}
                <span className="truncate">{item.label}</span>
              </button>
            )
          })}
        </div>
      ))}
    </div>
  )
}

// ─── Main Component ───────────────────────────────────────────────────────────

export function FileManager() {
  const [view, setView] = useState<ViewMode>("grid")
  const [path, setPath] = useState<string[]>(["home"])
  const [history, setHistory] = useState<string[][]>([["home"]])
  const [historyIdx, setHistoryIdx] = useState(0)
  const [selected, setSelected] = useState<string | null>(null)
  const [search, setSearch] = useState("")
  const [sortKey, setSortKey] = useState<SortKey>("name")
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null)
  const [starred, setStarred] = useState<FSEntry[]>([])
  const [statusMsg, setStatusMsg] = useState("")
  const containerRef = useRef<HTMLDivElement>(null)

  // Resolve current folder from path
  const resolve = useCallback((p: string[]): FSEntry[] => {
    let cur: FSEntry[] = FS_ROOT
    for (const seg of p) {
      const found = cur.find(e => e.id === seg)
      if (!found || found.type !== "folder") return cur
      cur = found.children ?? []
    }
    return cur
  }, [])

  const entries = resolve(path)

  const filtered = entries
    .filter(e => e.name.toLowerCase().includes(search.toLowerCase()))
    .sort((a, b) => {
      if (sortKey === "name") return a.name.localeCompare(b.name)
      if (sortKey === "modified") return b.modified.localeCompare(a.modified)
      if (sortKey === "type") return a.type.localeCompare(b.type)
      return 0
    })

  const selectedEntry = filtered.find(e => e.id === selected) ?? null

  const navigate = useCallback((newPath: string[]) => {
    setPath(newPath)
    setSelected(null)
    const newHistory = history.slice(0, historyIdx + 1)
    newHistory.push(newPath)
    setHistory(newHistory)
    setHistoryIdx(newHistory.length - 1)
  }, [history, historyIdx])

  const goBack = () => {
    if (historyIdx > 0) { setHistoryIdx(i => i - 1); setPath(history[historyIdx - 1]); setSelected(null) }
  }
  const goForward = () => {
    if (historyIdx < history.length - 1) { setHistoryIdx(i => i + 1); setPath(history[historyIdx + 1]); setSelected(null) }
  }
  const goUp = () => { if (path.length > 1) navigate(path.slice(0, -1)) }

  const handleOpen = (entry: FSEntry) => {
    if (entry.type === "folder") navigate([...path, entry.id])
    else setSelected(entry.id)
  }

  const handleContextMenu = useCallback((e: React.MouseEvent, entry: FSEntry | null) => {
    e.preventDefault()
    e.stopPropagation()
    setContextMenu({ x: e.clientX, y: e.clientY, entry })
  }, [])

  const handleContextAction = (action: string, entry: FSEntry | null) => {
    if (!entry) {
      if (action === "refresh") setStatusMsg("Refreshed")
      return
    }
    if (action === "open") handleOpen(entry)
    else if (action === "star") {
      setStarred(s => [...s.filter(e => e.id !== entry.id), { ...entry, starred: true }])
      setStatusMsg(`Starred "${entry.name}"`)
    } else if (action === "unstar") {
      setStarred(s => s.filter(e => e.id !== entry.id))
      setStatusMsg(`Unstarred "${entry.name}"`)
    } else if (action === "trash") {
      setStatusMsg(`"${entry.name}" moved to Trash`)
    } else if (action === "copy") {
      setStatusMsg(`Copied "${entry.name}"`)
    } else if (action === "download") {
      setStatusMsg(`Downloading "${entry.name}"`)
    }
  }

  // Breadcrumb labels
  const breadcrumb = path.map(seg => {
    const find = (list: FSEntry[]): string => {
      for (const e of list) { if (e.id === seg) return e.name; if (e.children) { const r = find(e.children); if (r) return r } }
      return seg
    }
    return find(FS_ROOT)
  })

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Toolbar */}
      <div className="flex items-center gap-1.5 border-b border-[var(--window-border)] bg-[var(--window-header)] px-2 py-1.5">
        <button onClick={goBack} disabled={historyIdx === 0} className="widget-icon-btn h-7 w-7 disabled:opacity-30"><ArrowLeft className="h-3.5 w-3.5" /></button>
        <button onClick={goForward} disabled={historyIdx >= history.length - 1} className="widget-icon-btn h-7 w-7 disabled:opacity-30"><ArrowRight className="h-3.5 w-3.5" /></button>
        <button onClick={goUp} disabled={path.length <= 1} className="widget-icon-btn h-7 w-7 disabled:opacity-30"><ArrowUp className="h-3.5 w-3.5" /></button>

        {/* Breadcrumb */}
        <div className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto px-2">
          {breadcrumb.map((seg, i) => (
            <span key={i} className="flex items-center gap-1">
              {i > 0 && <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground/40" />}
              <button
                onClick={() => navigate(path.slice(0, i + 1))}
                className={cn("whitespace-nowrap text-xs transition-colors hover:text-foreground",
                  i === breadcrumb.length - 1 ? "text-foreground font-medium" : "text-muted-foreground"
                )}
              >
                {seg}
              </button>
            </span>
          ))}
        </div>

        {/* Search */}
        <div className="relative">
          <Search className="absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-muted-foreground" />
          <input
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder="Search..."
            className="h-6 w-32 rounded-md bg-secondary/60 pl-6 pr-2 font-mono text-[11px] text-foreground placeholder:text-muted-foreground/50 outline-none focus:ring-1 focus:ring-primary/40"
          />
        </div>

        {/* View + Sort */}
        <button onClick={() => setView(v => v === "grid" ? "list" : "grid")} className="widget-icon-btn h-7 w-7">
          {view === "grid" ? <List className="h-3.5 w-3.5" /> : <Grid3x3 className="h-3.5 w-3.5" />}
        </button>
        <button onClick={() => setSortKey(k => k === "name" ? "modified" : k === "modified" ? "type" : "name")} className="widget-icon-btn h-7 w-7" title={`Sort: ${sortKey}`}>
          <SortAsc className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Body */}
      <div className="flex min-h-0 flex-1">
        {/* Sidebar */}
        <Sidebar starred={starred} currentPath={path} onNavigate={navigate} />

        {/* File grid/list */}
        <div
          ref={containerRef}
          className="flex-1 overflow-auto p-3"
          onContextMenu={e => handleContextMenu(e, null)}
          onClick={() => setSelected(null)}
        >
          {filtered.length === 0 && (
            <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
              {search ? "No results" : "Empty folder"}
            </div>
          )}

          {view === "grid" ? (
            <div className="grid grid-cols-[repeat(auto-fill,minmax(88px,1fr))] gap-2">
              {filtered.map(entry => (
                <button
                  key={entry.id}
                  onClick={e => { e.stopPropagation(); setSelected(entry.id) }}
                  onDoubleClick={() => handleOpen(entry)}
                  onContextMenu={e => handleContextMenu(e, entry)}
                  className={cn(
                    "group flex flex-col items-center gap-1.5 rounded-lg p-2 text-center transition-all",
                    selected === entry.id ? "bg-primary/15 ring-1 ring-primary/40" : "hover:bg-secondary/60"
                  )}
                >
                  <FileIcon type={entry.type} className="h-9 w-9" />
                  <span className="line-clamp-2 text-[10px] leading-tight text-foreground">{entry.name}</span>
                  {entry.starred && <Star className="h-2.5 w-2.5 fill-current text-[oklch(0.75_0.18_80)]" />}
                </button>
              ))}
            </div>
          ) : (
            <div className="space-y-px">
              {filtered.map(entry => (
                <button
                  key={entry.id}
                  onClick={e => { e.stopPropagation(); setSelected(entry.id) }}
                  onDoubleClick={() => handleOpen(entry)}
                  onContextMenu={e => handleContextMenu(e, entry)}
                  className={cn(
                    "flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-left transition-all",
                    selected === entry.id ? "bg-primary/15 ring-1 ring-primary/30" : "hover:bg-secondary/60"
                  )}
                >
                  <FileIcon type={entry.type} className="h-4 w-4" />
                  <span className="flex-1 truncate text-xs text-foreground">{entry.name}</span>
                  {entry.starred && <Star className="h-3 w-3 shrink-0 fill-current text-[oklch(0.75_0.18_80)]" />}
                  {entry.type !== "folder" && <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{entry.size}</span>}
                  <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{entry.modified}</span>
                  <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground/30" />
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Preview Panel */}
        <div className="w-44 shrink-0 border-l border-[var(--window-border)]">
          <PreviewPanel entry={selectedEntry} />
        </div>
      </div>

      {/* Status bar */}
      <div className="flex items-center justify-between border-t border-[var(--window-border)] bg-[var(--window-header)] px-3 py-1">
        <span className="font-mono text-[10px] text-muted-foreground">
          {filtered.length} item{filtered.length !== 1 ? "s" : ""}
          {selected && selectedEntry ? ` · "${selectedEntry.name}" selected` : ""}
        </span>
        {statusMsg && <span className="font-mono text-[10px] text-primary">{statusMsg}</span>}
        <span className="font-mono text-[10px] text-muted-foreground/40">{path.join(" / ")}</span>
      </div>

      {contextMenu && (
        <ContextMenu
          state={contextMenu}
          onClose={() => setContextMenu(null)}
          onAction={handleContextAction}
        />
      )}
    </div>
  )
}
