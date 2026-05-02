"use client"

import { useState, useEffect, useRef, useCallback, useCallback as useCallback2 } from "react"
import { useStore } from "@nanostores/react"
import { X, Save, FileCode, Check, Copy, ChevronDown } from "lucide-react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { fileSystemStore, saveFile, closeFile, setActiveFile, openDirectory, handleDroppedItems, openDroppedFiles } from "@/stores/file-system-store"

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
    if (fs.activeFileIndex >= 0) {
      closeFile(fs.activeFileIndex)
    }
  }

  if (!activeFile) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <div className="text-center">
          <FileCode className="mx-auto h-8 w-8 opacity-50" />
          <p className="mt-2 text-sm">No file open</p>
          <p className="text-xs">Ask to open a file or use the file browser</p>
        </div>
      </div>
    )
  }

  const lineCount = content.split("\n").length

  return (
    <div className="flex h-full flex-col">
      {/* Editor header */}
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <div className="flex items-center gap-2">
          <FileCode className="h-4 w-4 text-muted-foreground" />
          <span className="font-mono text-sm">{activeFile.name}</span>
          {hasChanges && <span className="text-xs text-yellow-500">●</span>}
        </div>
        <div className="flex items-center gap-1">
          <Button
            size="sm"
            variant="ghost"
            onClick={handleSave}
            disabled={!hasChanges}
            className="h-7 px-2"
          >
            <Save className="h-3.5 w-3.5" />
            <span className="ml-1 text-xs">Save</span>
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={handleClose}
            className="h-7 w-7 p-0"
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {/* Editor tabs */}
      {fs.openFiles.length > 1 && (
        <div className="flex gap-1 border-b border-border bg-muted/30 px-2 py-1">
          {fs.openFiles.map((file, i) => (
            <button
              key={file.path}
              onClick={() => setActiveFile(i)}
              className={cn(
                "flex items-center gap-1 rounded px-2 py-1 text-xs font-mono",
                i === fs.activeFileIndex
                  ? "bg-background text-foreground"
                  : "text-muted-foreground hover:text-foreground"
              )}
            >
              {file.name}
              {file.modified && <span className="text-yellow-500">●</span>}
            </button>
          ))}
        </div>
      )}

      {/* Code area */}
      <div className="flex flex-1 overflow-hidden">
        {/* Line numbers */}
        <div className="select-none border-r border-border bg-muted/30 py-2 text-right font-mono text-xs text-muted-foreground">
          <div className="flex flex-col items-end pr-2">
            {Array.from({ length: lineCount }, (_, i) => (
              <div key={i} className="leading-6">
                {i + 1}
              </div>
            ))}
          </div>
        </div>

        {/* Code textarea */}
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

      {/* Status bar */}
      <div className="flex items-center justify-between border-t border-border px-3 py-1 text-[10px] text-muted-foreground">
        <span>{activeFile.language}</span>
        <span>{content.length} chars · {lineCount} lines</span>
      </div>
    </div>
  )
}

export function FileTreeView() {
  const fs = useStore(fileSystemStore)
  const [isDragging, setIsDragging] = useState(false)

  const handleEntryClick = async (entry: { name: string; path: string; kind: string }) => {
    if (entry.kind === "directory") {
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
      if (e.dataTransfer.files.length > 0) {
        await openDroppedFiles(e.dataTransfer.files)
      }
      return
    }
    
    const result = await handleDroppedItems(items)
    if (result) {
      fileSystemStore.set(s => ({
        ...s,
        entries: result.entries,
        rootPath: result.rootName,
        rootHandle: null,
        error: null,
      }))
    } else if (e.dataTransfer.files.length > 0) {
      await openDroppedFiles(e.dataTransfer.files)
    }
  }

  if (!fs.rootHandle && fs.entries.length === 0) {
    return (
      <div 
        className={`flex h-full items-center justify-center text-muted-foreground ${isDragging ? 'bg-primary/10 border-2 border-dashed border-primary' : ''}`}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <div className="text-center p-4">
          {isDragging ? (
            <p className="text-sm text-primary">Drop files here...</p>
          ) : (
            <>
              <p className="text-sm">No folder open</p>
              <p className="text-xs mt-1">Drag & drop files or folders here</p>
              <Button
                onClick={() => openDirectory()}
                className="mt-2"
              >
                Open Folder
              </Button>
            </>
          )}
        </div>
      </div>
    )
  }

  const hasFiles = fs.rootHandle || fs.openFiles.length > 0
  if (!hasFiles && fs.entries.length > 0) {
    return (
      <div 
        className={`p-2 ${isDragging ? 'bg-primary/10 border-2 border-dashed border-primary' : ''}`}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <div className="mb-2 flex items-center justify-between">
          <span className="font-mono text-sm font-medium">{fs.rootPath}</span>
          <span className="text-xs text-muted-foreground">{fs.entries.length} items</span>
        </div>
        <div className="space-y-0.5">
          {fs.entries.map((entry) => (
            <button
              key={entry.path}
              onClick={() => handleEntryClick(entry)}
              className={cn(
                "flex w-full items-center gap-2 rounded px-2 py-1 text-left font-mono text-sm hover:bg-secondary",
                entry.kind === "directory" && "text-blue-400"
              )}
            >
              {entry.kind === "directory" ? "📁" : "📄"}
              <span>{entry.name}</span>
            </button>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div 
      className={`p-2 ${isDragging ? 'bg-primary/10' : ''}`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div className="mb-2 flex items-center justify-between">
        <span className="font-mono text-sm font-medium">{fs.rootPath}</span>
        {fs.gitRoot && (
          <span className="rounded bg-green-500/20 px-2 py-0.5 text-xs text-green-500">
            Git ✓
          </span>
        )}
      </div>
      <div className="space-y-0.5">
        {fs.entries.map((entry) => (
          <button
            key={entry.path}
            onClick={() => handleEntryClick(entry)}
            className={cn(
              "flex w-full items-center gap-2 rounded px-2 py-1 text-left font-mono text-sm hover:bg-secondary",
              entry.kind === "directory" && "text-blue-400"
            )}
          >
            {entry.kind === "directory" ? "📁" : "📄"}
            <span>{entry.name}</span>
          </button>
        ))}
      </div>
    </div>
  )
}

import { openFile } from "@/stores/file-system-store"