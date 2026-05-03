"use client"

import { useEffect, useMemo, useRef, useState } from "react"
import { useStore } from "@nanostores/react"
import {
  ChevronDown,
  ChevronRight,
  FileCode,
  FileText,
  Folder,
  FolderOpen,
  RefreshCw,
  Save,
  Search,
  Trash2,
  X,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import {
  clearRepo,
  fileSystemStore,
  handleDroppedItems,
  openDirectory,
  openDroppedFiles,
  openFile,
  refreshEntries,
  saveFile,
  closeFile,
  setActiveFile,
  setRepoSearchQuery,
  toggleDirectory,
  type FileEntry,
} from "@/stores/file-system-store"

interface CodeEditorProps {
  onSave?: (content: string) => void
}

export function CodeEditor({ onSave }: CodeEditorProps) {
  const fs = useStore(fileSystemStore)
  const activeFile = fs.openFiles[fs.activeFileIndex]
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const [content, setContent] = useState("")
  const [hasChanges, setHasChanges] = useState(false)

  useEffect(() => {
    if (activeFile) {
      setContent(activeFile.content)
      setHasChanges(false)
    }
  }, [activeFile])

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setContent(e.target.value)
    setHasChanges(e.target.value !== activeFile?.content)
  }

  const handleSave = async () => {
    if (!activeFile || fs.activeFileIndex < 0) return
    const ok = await saveFile(fs.activeFileIndex, content)
    if (ok) {
      setHasChanges(false)
      onSave?.(content)
    }
  }

  const handleClose = () => {
    if (fs.activeFileIndex >= 0) closeFile(fs.activeFileIndex)
  }

  if (!activeFile) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <div className="text-center">
          <FileCode className="mx-auto h-8 w-8 opacity-50" />
          <p className="mt-2 text-sm">No file open</p>
          <p className="text-xs">Open a repo file from Xray</p>
        </div>
      </div>
    )
  }

  const lineCount = content.split("\n").length

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <div className="flex min-w-0 items-center gap-2">
          <FileCode className="h-4 w-4 shrink-0 text-muted-foreground" />
          <span className="truncate font-mono text-sm" title={activeFile.path}>{activeFile.path}</span>
          {hasChanges && <span className="text-xs text-yellow-500">●</span>}
        </div>
        <div className="flex items-center gap-1">
          <Button size="sm" variant="ghost" onClick={handleSave} disabled={!hasChanges || !activeFile.handle} className="h-7 px-2">
            <Save className="h-3.5 w-3.5" />
            <span className="ml-1 text-xs">Save</span>
          </Button>
          <Button size="sm" variant="ghost" onClick={handleClose} className="h-7 w-7 p-0">
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {fs.openFiles.length > 1 && (
        <div className="flex gap-1 overflow-x-auto border-b border-border bg-muted/30 px-2 py-1">
          {fs.openFiles.map((file, i) => (
            <button
              key={file.path}
              onClick={() => setActiveFile(i)}
              className={cn(
                "flex max-w-48 items-center gap-1 rounded px-2 py-1 text-xs font-mono",
                i === fs.activeFileIndex ? "bg-background text-foreground" : "text-muted-foreground hover:text-foreground",
              )}
              title={file.path}
            >
              <span className="truncate">{file.name}</span>
              {file.modified && <span className="text-yellow-500">●</span>}
            </button>
          ))}
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        <div className="select-none border-r border-border bg-muted/30 py-2 text-right font-mono text-xs text-muted-foreground">
          <div className="flex flex-col items-end pr-2">
            {Array.from({ length: lineCount }, (_, i) => (
              <div key={i} className="leading-6">{i + 1}</div>
            ))}
          </div>
        </div>

        <div className="relative flex-1">
          <textarea
            ref={textareaRef}
            value={content}
            onChange={handleChange}
            spellCheck={false}
            className="h-full w-full resize-none bg-background p-2 font-mono text-sm leading-6 text-foreground focus:outline-none"
            style={{ tabSize: 2 }}
          />
        </div>
      </div>

      <div className="flex items-center justify-between border-t border-border px-3 py-1 text-[10px] text-muted-foreground">
        <span>{activeFile.language}</span>
        <span>{content.length} chars · {lineCount} lines</span>
      </div>
    </div>
  )
}

function entryAncestorsExpanded(entry: FileEntry, expandedDirs: string[]): boolean {
  if (!entry.parentPath) return true
  const expanded = new Set(expandedDirs)
  const parts = entry.parentPath.split("/").filter(Boolean)
  for (let i = 0; i < parts.length; i++) {
    const ancestor = parts.slice(0, i + 1).join("/")
    if (!expanded.has(ancestor)) return false
  }
  return true
}

function entryMatchesSearch(entry: FileEntry, query: string): boolean {
  if (!query.trim()) return true
  return entry.path.toLowerCase().includes(query.trim().toLowerCase())
}

function makeEntriesByPath(entries: FileEntry[]): Record<string, FileEntry> {
  return Object.fromEntries(entries.map((entry) => [entry.path, entry]))
}

export function FileTreeView() {
  const fs = useStore(fileSystemStore)
  const [isDragging, setIsDragging] = useState(false)

  const visibleEntries = useMemo(() => {
    const query = fs.searchQuery.trim()
    if (query) return fs.entries.filter((entry) => entryMatchesSearch(entry, query))
    return fs.entries.filter((entry) => entryAncestorsExpanded(entry, fs.expandedDirs))
  }, [fs.entries, fs.expandedDirs, fs.searchQuery])

  const handleEntryClick = async (entry: FileEntry) => {
    if (entry.kind === "directory") {
      await toggleDirectory(entry.path)
      return
    }
    await openFile(entry.path)
  }

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(true)
  }

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(false)
  }

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(false)

    const items = e.dataTransfer.items
    if (!items || items.length === 0) {
      if (e.dataTransfer.files.length > 0) await openDroppedFiles(e.dataTransfer.files)
      return
    }

    const result = await handleDroppedItems(items)
    if (result) {
      fileSystemStore.set({
        ...fileSystemStore.get(),
        entries: result.entries,
        entriesByPath: makeEntriesByPath(result.entries),
        rootPath: result.rootName,
        repoName: result.rootName,
        rootHandle: null,
        repoRootHandle: null,
        repoIndexStatus: "ready",
        error: null,
      })
    } else if (e.dataTransfer.files.length > 0) {
      await openDroppedFiles(e.dataTransfer.files)
    }
  }

  if (!fs.rootHandle && fs.entries.length === 0) {
    return (
      <div
        className={cn(
          "flex h-full items-center justify-center text-muted-foreground",
          isDragging && "border-2 border-dashed border-primary bg-primary/10",
        )}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <div className="p-4 text-center">
          {isDragging ? (
            <p className="text-sm text-primary">Drop files here...</p>
          ) : (
            <>
              <p className="text-sm">No repo open</p>
              <p className="mt-1 text-xs">Open a local repository through browser capability storage</p>
              <Button onClick={() => openDirectory()} className="mt-2">
                <FolderOpen className="mr-2 h-4 w-4" />
                Open Folder
              </Button>
            </>
          )}
        </div>
      </div>
    )
  }

  return (
    <div
      className={cn("flex h-full flex-col overflow-hidden", isDragging && "bg-primary/10")}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div className="border-b border-border p-2">
        <div className="mb-2 flex items-center gap-2">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <span className="truncate font-mono text-sm font-medium" title={fs.rootPath}>{fs.repoName || fs.rootPath || "Repo Xray"}</span>
              {fs.isGitRepo && <span className="rounded bg-green-500/20 px-1.5 py-0.5 text-[10px] text-green-500">Git</span>}
            </div>
            <div className="text-[10px] text-muted-foreground">
              {fs.repoIndexStatus === "indexing" ? "Indexing..." : `${fs.indexedFileCount} files · ${fs.indexedDirCount} dirs`}
            </div>
          </div>
          <Button size="sm" variant="ghost" onClick={() => refreshEntries()} disabled={fs.repoIndexStatus === "indexing"} className="h-7 w-7 p-0" title="Refresh index">
            <RefreshCw className={cn("h-3.5 w-3.5", fs.repoIndexStatus === "indexing" && "animate-spin")} />
          </Button>
          <Button size="sm" variant="ghost" onClick={() => clearRepo()} className="h-7 w-7 p-0" title="Clear repo">
            <Trash2 className="h-3.5 w-3.5" />
          </Button>
        </div>

        <div className="flex items-center gap-2 rounded-md border border-border bg-background px-2 py-1">
          <Search className="h-3.5 w-3.5 text-muted-foreground" />
          <input
            value={fs.searchQuery}
            onChange={(e) => setRepoSearchQuery(e.target.value)}
            placeholder="Search paths..."
            className="min-w-0 flex-1 bg-transparent font-mono text-xs outline-none placeholder:text-muted-foreground"
          />
        </div>

        <div className="mt-2 flex gap-2">
          <Button size="sm" variant="secondary" onClick={() => openDirectory()} className="h-7 flex-1 text-xs">
            <FolderOpen className="mr-1 h-3.5 w-3.5" />
            Open Folder
          </Button>
        </div>

        {(fs.error || fs.repoIndexError) && (
          <div className="mt-2 rounded border border-yellow-500/30 bg-yellow-500/10 px-2 py-1 text-[10px] text-yellow-400">
            {fs.error || fs.repoIndexError}
          </div>
        )}
      </div>

      <div className="flex-1 overflow-auto p-1">
        {visibleEntries.length === 0 ? (
          <div className="p-4 text-center text-xs text-muted-foreground">No matching files</div>
        ) : (
          <div className="space-y-0.5">
            {visibleEntries.map((entry) => {
              const selected = fs.selectedPath === entry.path
              const isDirectory = entry.kind === "directory"
              const expanded = fs.expandedDirs.includes(entry.path) || fs.searchQuery.trim().length > 0
              return (
                <button
                  key={entry.path}
                  onClick={() => handleEntryClick(entry)}
                  className={cn(
                    "flex w-full items-center gap-1 rounded px-1 py-1 text-left font-mono text-xs hover:bg-secondary",
                    selected && "bg-secondary text-foreground",
                    entry.ignored && "text-muted-foreground/60",
                    entry.binary && "text-muted-foreground",
                  )}
                  style={{ paddingLeft: `${4 + entry.depth * 12}px` }}
                  title={entry.path}
                >
                  {isDirectory ? (
                    expanded ? <ChevronDown className="h-3.5 w-3.5 shrink-0" /> : <ChevronRight className="h-3.5 w-3.5 shrink-0" />
                  ) : (
                    <span className="w-3.5 shrink-0" />
                  )}
                  {isDirectory ? (
                    expanded ? <FolderOpen className="h-3.5 w-3.5 shrink-0 text-blue-400" /> : <Folder className="h-3.5 w-3.5 shrink-0 text-blue-400" />
                  ) : entry.binary ? (
                    <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                  ) : (
                    <FileCode className="h-3.5 w-3.5 shrink-0 text-primary" />
                  )}
                  <span className="truncate">{entry.name}</span>
                  {entry.ignored && <span className="ml-auto shrink-0 text-[9px] text-muted-foreground">skip</span>}
                  {entry.binary && <span className="ml-auto shrink-0 text-[9px] text-muted-foreground">bin</span>}
                </button>
              )
            })}
          </div>
        )}
      </div>
    </div>
  )
}
