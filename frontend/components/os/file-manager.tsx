"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import type React from "react"
import {
  ArrowLeft,
  ArrowUp,
  Cloud,
  Database,
  Download,
  Eye,
  FileArchive,
  FileAudio,
  FileCode,
  FileImage,
  FileText,
  FileVideo,
  Folder,
  HardDrive,
  Info,
  MemoryStick,
  Plus,
  RefreshCw,
  Save,
  Search,
  Upload,
} from "lucide-react"
import { cn } from "@/lib/utils"

type ProviderId = "drive" | "browser-fs" | "local-storage" | "memory"
type FileType = "folder" | "text" | "image" | "video" | "audio" | "code" | "archive" | "unknown"
type ViewMode = "grid" | "list"

type FSEntry = {
  id: string
  name: string
  type: FileType
  size?: string
  modified: string
  preview?: string
  mimeHint?: string
  webUrl?: string
  downloadUrl?: string
  handle?: FileSystemHandle
}

type StoredNode = {
  id: string
  name: string
  kind: "folder" | "file"
  updatedAtIso: string
  mime?: string
  content?: string
  children?: StoredNode[]
}

type DriveFile = {
  id: string
  name: string
  mimeType: string
  modifiedTime?: string
  size?: string
  webViewLink?: string
  webContentLink?: string
}

const LOCAL_STORAGE_KEY = "edgerun:file-manager:local-storage:v1"

const PROVIDERS: Array<{ id: ProviderId; label: string; icon: React.ReactNode; detail: string }> = [
  { id: "drive", label: "Google Drive", icon: <Cloud className="h-3.5 w-3.5" />, detail: "OAuth session" },
  { id: "browser-fs", label: "Browser FS", icon: <HardDrive className="h-3.5 w-3.5" />, detail: "picked folder" },
  { id: "local-storage", label: "Local Storage", icon: <Database className="h-3.5 w-3.5" />, detail: "persistent browser store" },
  { id: "memory", label: "Memory", icon: <MemoryStick className="h-3.5 w-3.5" />, detail: "temporary workspace" },
]

function nowIso() {
  return new Date().toISOString()
}

function defaultTree(): StoredNode {
  return {
    id: "root",
    name: "Workspace",
    kind: "folder",
    updatedAtIso: nowIso(),
    children: [
      {
        id: "readme",
        name: "README.md",
        kind: "file",
        updatedAtIso: nowIso(),
        mime: "text/markdown",
        content: "# EdgeRun File Manager\n\nUse the source picker to browse Drive, a local folder, localStorage, or memory.",
      },
      {
        id: "notes",
        name: "Notes",
        kind: "folder",
        updatedAtIso: nowIso(),
        children: [
          {
            id: "first-note",
            name: "first-note.txt",
            kind: "file",
            updatedAtIso: nowIso(),
            mime: "text/plain",
            content: "This file is stored by the selected provider.",
          },
        ],
      },
    ],
  }
}

function safeId(name: string) {
  return `${name.toLowerCase().replace(/[^a-z0-9._-]+/g, "-")}-${crypto.randomUUID().slice(0, 8)}`
}

function fileType(name: string, mime = ""): FileType {
  const lower = name.toLowerCase()
  if (mime.includes("folder")) return "folder"
  if (mime.startsWith("image/") || /\.(png|jpg|jpeg|gif|webp|svg)$/.test(lower)) return "image"
  if (mime.startsWith("video/") || /\.(mp4|mov|webm|mkv)$/.test(lower)) return "video"
  if (mime.startsWith("audio/") || /\.(mp3|wav|flac|ogg)$/.test(lower)) return "audio"
  if (/\.(ts|tsx|js|jsx|rs|go|py|json|toml|yaml|yml|css|html|md|sh)$/.test(lower)) return "code"
  if (/\.(txt|log|csv)$/.test(lower) || mime.startsWith("text/")) return "text"
  if (/\.(zip|tar|gz|wasm|bin)$/.test(lower)) return "archive"
  return "unknown"
}

function formatBytes(bytes?: number | string): string | undefined {
  const value = typeof bytes === "string" ? Number(bytes) : bytes
  if (!Number.isFinite(value ?? NaN)) return undefined
  if ((value ?? 0) < 1024) return `${value} B`
  if ((value ?? 0) < 1024 * 1024) return `${((value ?? 0) / 1024).toFixed(1)} KB`
  return `${((value ?? 0) / (1024 * 1024)).toFixed(1)} MB`
}

function nodeToEntry(node: StoredNode): FSEntry {
  const size = node.kind === "file" ? new TextEncoder().encode(node.content ?? "").byteLength : undefined
  return {
    id: node.id,
    name: node.name,
    type: node.kind === "folder" ? "folder" : fileType(node.name, node.mime),
    size: formatBytes(size),
    modified: new Date(node.updatedAtIso).toLocaleString(),
    preview: node.content,
    mimeHint: node.mime,
  }
}

function childrenAt(root: StoredNode, path: string[]): StoredNode[] {
  let current = root
  for (const segment of path) {
    const next = current.children?.find((child) => child.id === segment)
    if (!next || next.kind !== "folder") return current.children ?? []
    current = next
  }
  return current.children ?? []
}

function mutateChildren(root: StoredNode, path: string[], fn: (children: StoredNode[]) => StoredNode[]): StoredNode {
  if (path.length === 0) return { ...root, children: fn(root.children ?? []), updatedAtIso: nowIso() }
  const [head, ...tail] = path
  return {
    ...root,
    children: (root.children ?? []).map((child) => {
      if (child.id !== head || child.kind !== "folder") return child
      return mutateChildren(child, tail, fn)
    }),
    updatedAtIso: nowIso(),
  }
}

function loadLocalTree(): StoredNode {
  if (typeof window === "undefined") return defaultTree()
  try {
    const parsed = JSON.parse(window.localStorage.getItem(LOCAL_STORAGE_KEY) ?? "null") as StoredNode | null
    return parsed?.kind === "folder" ? parsed : defaultTree()
  } catch {
    return defaultTree()
  }
}

function FileIcon({ type, className }: { type: FileType; className?: string }) {
  const cls = cn("shrink-0", className)
  if (type === "folder") return <Folder className={cn(cls, "text-[oklch(0.7_0.18_80)]")} />
  if (type === "image") return <FileImage className={cn(cls, "text-[oklch(0.6_0.2_270)]")} />
  if (type === "video") return <FileVideo className={cn(cls, "text-[oklch(0.6_0.2_310)]")} />
  if (type === "audio") return <FileAudio className={cn(cls, "text-[oklch(0.65_0.2_200)]")} />
  if (type === "code") return <FileCode className={cn(cls, "text-primary")} />
  if (type === "archive") return <FileArchive className={cn(cls, "text-[oklch(0.65_0.18_45)]")} />
  return <FileText className={cn(cls, "text-muted-foreground")} />
}

function PreviewPanel({ entry }: { entry: FSEntry | null }) {
  if (!entry) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground">
        <Eye className="h-10 w-10 opacity-20" />
        <span className="text-xs">Select a file</span>
      </div>
    )
  }
  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-[var(--window-border)] p-3">
        <FileIcon type={entry.type} className="h-5 w-5" />
        <div className="min-w-0">
          <p className="truncate text-sm font-medium text-foreground">{entry.name}</p>
          <p className="text-[10px] text-muted-foreground">{entry.size ?? "folder or remote file"} · {entry.modified}</p>
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {entry.preview ? (
          <pre className="whitespace-pre-wrap font-mono text-[11px] leading-relaxed text-muted-foreground">{entry.preview}</pre>
        ) : entry.webUrl ? (
          <a href={entry.webUrl} target="_blank" rel="noreferrer" className="inline-flex items-center gap-2 rounded-md border border-border px-3 py-2 text-xs text-primary hover:bg-secondary">
            Open remote file
          </a>
        ) : (
          <div className="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground">
            <FileIcon type={entry.type} className="h-12 w-12 opacity-30" />
            <span className="text-[10px]">No inline preview</span>
          </div>
        )}
      </div>
      <div className="space-y-1 border-t border-[var(--window-border)] p-3">
        {[
          ["Kind", entry.type],
          ["Size", entry.size ?? "-"],
          ["Modified", entry.modified],
          ["MIME", entry.mimeHint ?? "-"],
        ].map(([key, value]) => (
          <div key={key} className="flex justify-between gap-3 text-[10px]">
            <span className="text-muted-foreground">{key}</span>
            <span className="truncate font-mono text-foreground">{value}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

export function FileManager() {
  const [provider, setProvider] = useState<ProviderId>("local-storage")
  const [view, setView] = useState<ViewMode>("grid")
  const [search, setSearch] = useState("")
  const [selected, setSelected] = useState<string | null>(null)
  const [path, setPath] = useState<string[]>([])
  const [status, setStatus] = useState("")
  const [memoryTree, setMemoryTree] = useState<StoredNode>(() => defaultTree())
  const [localTree, setLocalTree] = useState<StoredNode>(() => loadLocalTree())
  const [browserRoot, setBrowserRoot] = useState<FileSystemDirectoryHandle | null>(null)
  const [browserPath, setBrowserPath] = useState<FileSystemDirectoryHandle[]>([])
  const [browserEntries, setBrowserEntries] = useState<FSEntry[]>([])
  const [driveFolderId, setDriveFolderId] = useState("root")
  const [driveCrumbs, setDriveCrumbs] = useState<Array<{ id: string; name: string }>>([{ id: "root", name: "My Drive" }])
  const [driveEntries, setDriveEntries] = useState<FSEntry[]>([])
  const [loading, setLoading] = useState(false)
  const fileInputRef = useRef<HTMLInputElement>(null)

  const tree = provider === "memory" ? memoryTree : localTree
  const treeEntries = useMemo(() => childrenAt(tree, path).map(nodeToEntry), [path, tree])
  const entries = provider === "drive" ? driveEntries : provider === "browser-fs" ? browserEntries : treeEntries
  const filtered = useMemo(() => {
    const term = search.trim().toLowerCase()
    return entries
      .filter((entry) => !term || entry.name.toLowerCase().includes(term))
      .sort((a, b) => (a.type === "folder" && b.type !== "folder" ? -1 : a.type !== "folder" && b.type === "folder" ? 1 : a.name.localeCompare(b.name)))
  }, [entries, search])
  const selectedEntry = filtered.find((entry) => entry.id === selected) ?? null

  useEffect(() => {
    window.localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(localTree))
  }, [localTree])

  const loadDrive = useCallback(async (folderId = driveFolderId) => {
    setLoading(true)
    setStatus("")
    try {
      const url = new URL("/api/google-drive/files", window.location.origin)
      url.searchParams.set("folderId", folderId)
      if (search.trim()) url.searchParams.set("q", search.trim())
      const res = await fetch(url)
      const data = await res.json()
      if (!res.ok) throw new Error(data.error || "Drive request failed")
      setDriveEntries((data.files ?? []).map((file: DriveFile) => ({
        id: file.id,
        name: file.name,
        type: file.mimeType === "application/vnd.google-apps.folder" ? "folder" : fileType(file.name, file.mimeType),
        size: formatBytes(file.size),
        modified: file.modifiedTime ? new Date(file.modifiedTime).toLocaleString() : "unknown",
        mimeHint: file.mimeType,
        webUrl: file.webViewLink,
        downloadUrl: file.webContentLink,
      })))
      setStatus(`Loaded ${data.files?.length ?? 0} Drive item${data.files?.length === 1 ? "" : "s"}`)
    } catch (error) {
      setDriveEntries([])
      setStatus(error instanceof Error ? error.message : String(error))
    } finally {
      setLoading(false)
    }
  }, [driveFolderId, search])

  const loadBrowserDirectory = useCallback(async (handle?: FileSystemDirectoryHandle | null) => {
    const dir = handle ?? browserPath.at(-1) ?? browserRoot
    if (!dir) {
      setBrowserEntries([])
      return
    }
    setLoading(true)
    try {
      const next: FSEntry[] = []
      for await (const child of dir.values()) {
        const file = child.kind === "file" ? await (child as FileSystemFileHandle).getFile() : null
        next.push({
          id: child.name,
          name: child.name,
          type: child.kind === "directory" ? "folder" : fileType(child.name, file?.type ?? ""),
          size: formatBytes(file?.size),
          modified: file ? new Date(file.lastModified).toLocaleString() : "folder",
          mimeHint: file?.type,
          handle: child,
        })
      }
      setBrowserEntries(next)
      setStatus(`Loaded ${next.length} browser filesystem item${next.length === 1 ? "" : "s"}`)
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error))
    } finally {
      setLoading(false)
    }
  }, [browserPath, browserRoot])

  useEffect(() => {
    if (provider === "drive") void loadDrive()
  }, [provider, loadDrive])

  useEffect(() => {
    if (provider === "browser-fs") void loadBrowserDirectory()
  }, [provider, loadBrowserDirectory])

  const switchProvider = (nextProvider: ProviderId) => {
    setProvider(nextProvider)
    setPath([])
    setSelected(null)
    setSearch("")
    setStatus("")
  }

  const openEntry = async (entry: FSEntry) => {
    setSelected(entry.id)
    if (provider === "drive") {
      if (entry.type === "folder") {
        setDriveFolderId(entry.id)
        setDriveCrumbs((crumbs) => [...crumbs, { id: entry.id, name: entry.name }])
        await loadDrive(entry.id)
      } else if (entry.webUrl) {
        window.open(entry.webUrl, "_blank", "noreferrer")
      }
      return
    }
    if (provider === "browser-fs") {
      if (entry.type === "folder" && entry.handle?.kind === "directory") {
        const dir = entry.handle as FileSystemDirectoryHandle
        setBrowserPath((current) => [...current, dir])
        await loadBrowserDirectory(dir)
      } else if (entry.handle?.kind === "file") {
        const file = await (entry.handle as FileSystemFileHandle).getFile()
        if (entry.type === "text" || entry.type === "code") {
          setBrowserEntries((current) => current.map((item) => item.id === entry.id ? { ...item, preview: file.size < 512_000 ? undefined : "File is too large for inline preview." } : item))
          const text = file.size < 512_000 ? await file.text() : "File is too large for inline preview."
          setBrowserEntries((current) => current.map((item) => item.id === entry.id ? { ...item, preview: text } : item))
        }
      }
      return
    }
    if (entry.type === "folder") setPath((current) => [...current, entry.id])
  }

  const goUp = async () => {
    setSelected(null)
    if (provider === "drive") {
      if (driveCrumbs.length <= 1) return
      const next = driveCrumbs.slice(0, -1)
      setDriveCrumbs(next)
      setDriveFolderId(next[next.length - 1].id)
      await loadDrive(next[next.length - 1].id)
      return
    }
    if (provider === "browser-fs") {
      const next = browserPath.slice(0, -1)
      setBrowserPath(next)
      await loadBrowserDirectory(next.at(-1) ?? browserRoot)
      return
    }
    if (path.length > 0) setPath((current) => current.slice(0, -1))
  }

  const chooseBrowserFolder = async () => {
    if (!("showDirectoryPicker" in window)) {
      setStatus("This browser does not expose the File System Access API.")
      return
    }
    const handle = await window.showDirectoryPicker({ mode: "readwrite" })
    setBrowserRoot(handle)
    setBrowserPath([])
    await loadBrowserDirectory(handle)
  }

  const updateTree = (fn: (root: StoredNode) => StoredNode) => {
    if (provider === "memory") setMemoryTree(fn)
    if (provider === "local-storage") setLocalTree(fn)
  }

  const createFolder = async () => {
    const name = window.prompt("Folder name")
    if (!name) return
    if (provider === "browser-fs") {
      const dir = browserPath.at(-1) ?? browserRoot
      if (!dir) return setStatus("Pick a browser folder first.")
      await dir.getDirectoryHandle(name, { create: true })
      await loadBrowserDirectory(dir)
      return
    }
    if (provider === "drive") {
      const form = new FormData()
      form.set("action", "create-folder")
      form.set("folderId", driveFolderId)
      form.set("name", name)
      const res = await fetch("/api/google-drive/files", { method: "POST", body: form })
      const data = await res.json().catch(() => null)
      if (!res.ok) return setStatus(data?.error || "Drive folder create failed")
      await loadDrive()
      return
    }
    updateTree((root) => mutateChildren(root, path, (children) => [...children, { id: safeId(name), name, kind: "folder", updatedAtIso: nowIso(), children: [] }]))
  }

  const createFile = async () => {
    const name = window.prompt("File name")
    if (!name) return
    const content = window.prompt("Initial content") ?? ""
    if (provider === "browser-fs") {
      const dir = browserPath.at(-1) ?? browserRoot
      if (!dir) return setStatus("Pick a browser folder first.")
      const handle = await dir.getFileHandle(name, { create: true })
      const writable = await handle.createWritable()
      await writable.write(content)
      await writable.close()
      await loadBrowserDirectory(dir)
      return
    }
    if (provider === "drive") {
      const file = new File([content], name, { type: "text/plain" })
      await uploadFileArray([file])
      return
    }
    updateTree((root) => mutateChildren(root, path, (children) => [...children, { id: safeId(name), name, kind: "file", mime: "text/plain", content, updatedAtIso: nowIso() }]))
  }

  const uploadFileArray = async (files: File[]) => {
    if (!files.length) return
    if (provider === "drive") {
      setLoading(true)
      try {
        for (const file of files) {
          const form = new FormData()
          form.set("folderId", driveFolderId)
          form.set("file", file)
          const res = await fetch("/api/google-drive/files", { method: "POST", body: form })
          const data = await res.json().catch(() => null)
          if (!res.ok) throw new Error(data?.error || `Drive upload failed for ${file.name}`)
        }
        await loadDrive()
        setStatus(`Uploaded ${files.length} file${files.length === 1 ? "" : "s"} to Drive`)
      } catch (error) {
        setStatus(error instanceof Error ? error.message : String(error))
      } finally {
        setLoading(false)
      }
      return
    }
    if (provider === "browser-fs") {
      const dir = browserPath.at(-1) ?? browserRoot
      if (!dir) return setStatus("Pick a browser folder first.")
      for (const file of Array.from(files)) {
        const handle = await dir.getFileHandle(file.name, { create: true })
        const writable = await handle.createWritable()
        await writable.write(file)
        await writable.close()
      }
      await loadBrowserDirectory(dir)
      return
    }
    const nodes = await Promise.all(files.map(async (file) => ({
      id: safeId(file.name),
      name: file.name,
      kind: "file" as const,
      mime: file.type || "application/octet-stream",
      content: file.size < 512_000 ? await file.text().catch(() => "") : `[binary file: ${file.name}]`,
      updatedAtIso: nowIso(),
    })))
    updateTree((root) => mutateChildren(root, path, (children) => [...children, ...nodes]))
  }

  const uploadFiles = async (files: FileList | null) => {
    if (!files?.length) return
    await uploadFileArray(Array.from(files))
  }

  const refresh = async () => {
    if (provider === "drive") await loadDrive()
    else if (provider === "browser-fs") await loadBrowserDirectory()
    else setStatus("Refreshed")
  }

  const breadcrumb = provider === "drive"
    ? driveCrumbs.map((crumb) => crumb.name)
    : provider === "browser-fs"
      ? [browserRoot?.name ?? "No folder", ...browserPath.map((handle) => handle.name)]
      : ["Workspace", ...path]

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex items-center gap-1.5 border-b border-[var(--window-border)] bg-[var(--window-header)] px-2 py-1.5">
        <button onClick={goUp} disabled={(provider === "drive" && driveCrumbs.length <= 1) || (provider === "browser-fs" && !browserPath.length) || (!["drive", "browser-fs"].includes(provider) && path.length === 0)} className="widget-icon-btn h-7 w-7 disabled:opacity-30" title="Up">
          <ArrowUp className="h-3.5 w-3.5" />
        </button>
        <button onClick={refresh} className="widget-icon-btn h-7 w-7" title="Refresh">
          <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
        </button>
        <div className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto px-2">
          {breadcrumb.map((segment, index) => (
            <span key={`${segment}-${index}`} className="whitespace-nowrap text-xs text-muted-foreground">
              {index > 0 ? " / " : ""}{segment}
            </span>
          ))}
        </div>
        <div className="relative">
          <Search className="absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-muted-foreground" />
          <input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Search" className="h-7 w-32 rounded-md bg-secondary/60 pl-6 pr-2 text-[11px] text-foreground outline-none focus:ring-1 focus:ring-primary/40" />
        </div>
        <button onClick={() => setView((current) => current === "grid" ? "list" : "grid")} className="h-7 rounded-md border border-border px-2 text-[11px] text-muted-foreground hover:text-foreground">
          {view}
        </button>
      </div>

      <div className="flex min-h-0 flex-1">
        <div className="flex w-44 shrink-0 flex-col border-r border-[var(--window-border)] py-2">
          <p className="px-3 pb-1 font-mono text-[9px] font-semibold uppercase tracking-widest text-muted-foreground/60">Sources</p>
          {PROVIDERS.map((item) => (
            <button key={item.id} onClick={() => switchProvider(item.id)} className={cn("flex w-full items-center gap-2 px-3 py-2 text-left text-xs", provider === item.id ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-secondary hover:text-foreground")}>
              {item.icon}
              <span className="min-w-0 flex-1 truncate">{item.label}</span>
            </button>
          ))}
          <div className="mt-2 border-t border-[var(--window-border)] p-2">
            {provider === "browser-fs" && (
              <button onClick={chooseBrowserFolder} className="flex w-full items-center justify-center gap-1.5 rounded-md border border-border px-2 py-1.5 text-[11px] text-foreground hover:bg-secondary">
                <HardDrive className="h-3 w-3" /> Pick folder
              </button>
            )}
            {provider === "drive" && (
              <div className="rounded-md border border-border bg-secondary/30 p-2 text-[11px] leading-4 text-muted-foreground">
                Uses the sealed Google Drive OAuth session when present.
              </div>
            )}
          </div>
        </div>

        <div className="flex min-w-0 flex-1 flex-col">
          <div className="flex shrink-0 items-center gap-1 border-b border-[var(--window-border)] px-3 py-2">
            <button onClick={createFolder} className="inline-flex h-7 items-center gap-1 rounded-md border border-border px-2 text-[11px] text-foreground hover:bg-secondary"><Folder className="h-3 w-3" />Folder</button>
            <button onClick={createFile} className="inline-flex h-7 items-center gap-1 rounded-md border border-border px-2 text-[11px] text-foreground hover:bg-secondary"><Plus className="h-3 w-3" />File</button>
            <button onClick={() => fileInputRef.current?.click()} className="inline-flex h-7 items-center gap-1 rounded-md border border-border px-2 text-[11px] text-foreground hover:bg-secondary"><Upload className="h-3 w-3" />Upload</button>
            <input ref={fileInputRef} type="file" multiple className="hidden" onChange={(event) => void uploadFiles(event.target.files)} />
            <div className="ml-auto flex items-center gap-1 text-[11px] text-muted-foreground">
              {PROVIDERS.find((item) => item.id === provider)?.detail}
            </div>
          </div>
          <div className="min-h-0 flex-1 overflow-auto p-3">
            {provider === "browser-fs" && !browserRoot ? (
              <div className="flex h-full items-center justify-center">
                <button onClick={chooseBrowserFolder} className="inline-flex items-center gap-2 rounded-md border border-border px-3 py-2 text-xs text-foreground hover:bg-secondary">
                  <HardDrive className="h-4 w-4" /> Pick a local folder
                </button>
              </div>
            ) : filtered.length === 0 ? (
              <div className="flex h-full items-center justify-center text-xs text-muted-foreground">{loading ? "Loading..." : "Empty folder"}</div>
            ) : view === "grid" ? (
              <div className="grid grid-cols-[repeat(auto-fill,minmax(96px,1fr))] gap-2">
                {filtered.map((entry) => (
                  <button key={entry.id} onClick={() => setSelected(entry.id)} onDoubleClick={() => void openEntry(entry)} className={cn("group flex min-h-24 flex-col items-center gap-1.5 rounded-lg p-2 text-center transition-all", selected === entry.id ? "bg-primary/15 ring-1 ring-primary/40" : "hover:bg-secondary/60")}>
                    <FileIcon type={entry.type} className="h-9 w-9" />
                    <span className="line-clamp-2 text-[10px] leading-tight text-foreground">{entry.name}</span>
                  </button>
                ))}
              </div>
            ) : (
              <div className="space-y-px">
                {filtered.map((entry) => (
                  <button key={entry.id} onClick={() => setSelected(entry.id)} onDoubleClick={() => void openEntry(entry)} className={cn("flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-left transition-all", selected === entry.id ? "bg-primary/15 ring-1 ring-primary/30" : "hover:bg-secondary/60")}>
                    <FileIcon type={entry.type} className="h-4 w-4" />
                    <span className="min-w-0 flex-1 truncate text-xs text-foreground">{entry.name}</span>
                    <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{entry.size ?? "-"}</span>
                    <span className="hidden shrink-0 font-mono text-[10px] text-muted-foreground md:inline">{entry.modified}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        <div className="w-52 shrink-0 border-l border-[var(--window-border)]">
          <PreviewPanel entry={selectedEntry} />
        </div>
      </div>

      <div className="flex items-center justify-between border-t border-[var(--window-border)] bg-[var(--window-header)] px-3 py-1">
        <span className="font-mono text-[10px] text-muted-foreground">{filtered.length} item{filtered.length === 1 ? "" : "s"}{selectedEntry ? ` · ${selectedEntry.name}` : ""}</span>
        {status && <span className="truncate px-3 font-mono text-[10px] text-primary">{status}</span>}
        <div className="flex items-center gap-2 text-muted-foreground/60">
          {selectedEntry?.downloadUrl && <a href={selectedEntry.downloadUrl} target="_blank" rel="noreferrer" title="Download"><Download className="h-3.5 w-3.5" /></a>}
          {provider !== "drive" && <Save className="h-3.5 w-3.5" />}
          <Info className="h-3.5 w-3.5" />
          <ArrowLeft className="hidden" />
        </div>
      </div>
    </div>
  )
}
