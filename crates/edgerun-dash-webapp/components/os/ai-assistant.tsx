"use client"

import { useRef, useEffect, useCallback, useState } from "react"
import { useStore } from "@nanostores/react"
import { Send, Sparkles, Loader2, Bot, User, Trash2, Activity, Wifi, WifiOff, Code2, FolderOpen, Copy, Check, Workflow, Play, Settings } from "lucide-react"
import { cn } from "@/lib/utils"
import { aiChatStore, addMessage, setLoading, clearChat } from "@/stores/ai-chat-store"
import { getSystemContext } from "@/stores/ai-chat-store"
import { systemStatsStore, windowsStore } from "@/stores/desktop-store"
import { fileSystemStore, openDirectory, isFileSystemAccessSupported, openDroppedFiles, handleDroppedItems } from "@/stores/file-system-store"
import { scanCodebase } from "@/stores/codebase-context"
import { launchApp } from "@/stores/app-launcher"
import { CodeEditor, FileTreeView } from "./code-editor"
import { workflowStore, executeWorkflow } from "@/stores/workflow-store"
import { getSystemPrompt, saveSystemPrompt, resetSystemPrompt } from "@/app/api/chat/system-prompt"

type ViewMode = "chat" | "editor" | "split"

export function AIAssistant() {
  const chat = useStore(aiChatStore)
  const stats = useStore(systemStatsStore)
  const windows = useStore(windowsStore)
  const fs = useStore(fileSystemStore)
  const [inputValue, setInputValue] = useState("")
  const [viewMode, setViewMode] = useState<ViewMode>("split")
  const [copiedId, setCopiedId] = useState<string | null>(null)
  const [isDragging, setIsDragging] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [systemPrompt, setSystemPrompt] = useState("")
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const bottomRef = useRef<HTMLDivElement>(null)

  const handleCopyMessage = async (content: string, id: string) => {
    await navigator.clipboard.writeText(content)
    setCopiedId(id)
    setTimeout(() => setCopiedId(null), 2000)
  }

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(true)
  }

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
  }

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
    
    const files = e.dataTransfer.files
    if (files.length > 0) {
      await openDroppedFiles(files)
      addMessage("assistant", `📁 Added ${files.length} file(s)! You can now browse and edit them in the Editor tab.`)
    }
  }

  const scrollToBottom = useCallback(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" })
  }, [])

  useEffect(() => {
    scrollToBottom()
  }, [chat.messages, scrollToBottom])

  useEffect(() => {
    if (chat.messages.length === 0) {
      addMessage("assistant", "Hello! I'm your Edgerun AI assistant with an integrated code editor and workflow automation.\n\n**Features:**\n- 📊 System monitoring (nodes, CPU, memory)\n- 📁 **Open local folders** (or drag & drop)\n- ✏️ **Edit files** directly in the browser\n- ⚡ **Workflows** - automation pipelines\n- ⚙️ Click the gear icon to edit my system prompt\n\nTry asking me about the system or drag a file here!")
    }
    setSystemPrompt(getSystemPrompt())
  }, [])

  const handleSend = async () => {
    const text = inputValue.trim()
    if (!text || chat.isLoading) return

    const isFolderCommand = text.toLowerCase().includes("open folder") || text.toLowerCase().includes("open directory")
    
    if (isFolderCommand) {
      addMessage("user", text)
      setInputValue("")
      
      if (!isFileSystemAccessSupported()) {
        addMessage("assistant", "❌ **Can't open folder picker** - your browser doesn't support it.\n\n**But drag & drop works!** Try:\n1. Drag a file or folder from your desktop directly onto this window\n2. I'll show it in the file tree so you can edit it\n\nAlternatively, just paste the code here and I'll help you work with it.")
        return
      }
      
      addMessage("assistant", "Opening folder picker... Select a folder to analyze your codebase.")
      
      const success = await openDirectory()
      const fsState = fileSystemStore.get()
      
      if (success) {
        await scanCodebase()
        const ctx = await import("@/stores/codebase-context")
        const context = ctx.getCodebaseContext()
        addMessage("assistant", `✅ Opened! I've scanned ${context.split("Total Files")[1]?.split("\n")[0] || "your project"}. You can now ask me about specific files, components, or the project structure.`)
      } else if (fsState.error) {
        addMessage("assistant", `❌ ${fsState.error}`)
      } else {
        addMessage("assistant", "❌ Could not open folder. No folder selected or permission denied.")
      }
      return
    }

    addMessage("user", text)
    setInputValue("")
    setLoading(true)

    try {
      const messages = chat.messages.map((m) => ({
        role: m.role,
        content: m.content,
      }))
      messages.push({ role: "user", content: text })

      const context = getSystemContext()

      const response = await fetch("/api/chat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          messages,
          systemContext: context,
        }),
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`)
      }

      const data = await response.json()
      addMessage("assistant", data.text)
    } catch (error) {
      addMessage("assistant", `Error: ${error instanceof Error ? error.message : "Failed to get response"}`)
    } finally {
      setLoading(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  const handleClear = () => {
    clearChat()
    addMessage("assistant", "Chat cleared. How can I help you?")
  }

  const runningCount = windows.filter(w => w.appId !== "app-store" && w.appId !== "app-studio").length
  const hasOpenFile = fs.openFiles?.length > 0

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex h-10 flex-shrink-0 items-center gap-2 border-b border-[var(--window-border)] px-4">
        <Sparkles className="h-4 w-4 text-primary" />
        <span className="text-sm font-medium text-foreground">AI Assistant</span>
        
        <div className="ml-auto flex items-center gap-1">
          <button
            onClick={() => {
              const apps = require("@/components/os/app-store").availableApps
              const wfApp = apps.find((a: any) => a.id === "workflow-builder")
              if (wfApp) launchApp(wfApp)
            }}
            className="flex items-center gap-1 rounded px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
            title="Open Workflow Builder"
          >
            <Workflow className="h-3 w-3" />
          </button>
          <button
            onClick={() => setShowSettings(true)}
            className="flex items-center gap-1 rounded px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
            title="Edit System Prompt"
          >
            <Settings className="h-3 w-3" />
          </button>
          <button
            onClick={() => setViewMode("chat")}
            className={cn(
              "rounded px-2 py-1 text-xs",
              viewMode === "chat" ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:text-foreground"
            )}
          >
            Chat
          </button>
          <button
            onClick={() => setViewMode("editor")}
            className={cn(
              "rounded px-2 py-1 text-xs",
              viewMode === "editor" ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:text-foreground"
            )}
          >
            Editor
          </button>
          <button
            onClick={() => setViewMode("split")}
            className={cn(
              "rounded px-2 py-1 text-xs",
              viewMode === "split" ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:text-foreground"
            )}
          >
            Split
          </button>
        </div>

        <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
          <span className="flex items-center gap-1">
            {stats.isConnected ? (
              <Wifi className="h-3 w-3 text-green-500" />
            ) : (
              <WifiOff className="h-3 w-3 text-red-500" />
            )}
          </span>
          {fs.rootHandle && (
            <span className="flex items-center gap-1 text-green-500">
              📁 {fs.rootPath}
            </span>
          )}
          {hasOpenFile && (
            <span className="flex items-center gap-1 text-blue-400">
              📄 {fs.openFiles[fs.activeFileIndex]?.name}
            </span>
          )}
        </div>
      </div>

      {/* Main content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Chat panel */}
        {(viewMode === "chat" || viewMode === "split") && (
          <div className={cn("flex flex-col", viewMode === "split" ? "w-1/2 border-r border-[var(--window-border)]" : "w-full")}>
            {/* Messages */}
            <div className="flex-1 overflow-y-auto p-4">
              <div className="space-y-4">
                {chat.messages.map((msg) => (
                  <div
                    key={msg.id}
                    data-message
                    className={cn(
                      "flex gap-3",
                      msg.role === "user" && "flex-row-reverse"
                    )}
                  >
                    <div
                      className={cn(
                        "flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full",
                        msg.role === "assistant"
                          ? "bg-primary/20 text-primary"
                          : "bg-secondary text-muted-foreground"
                      )}
                    >
                      {msg.role === "assistant" ? (
                        <Bot className="h-4 w-4" />
                      ) : (
                        <User className="h-4 w-4" />
                      )}
                    </div>
                    <div
                      className={cn(
                        "flex max-w-[85%] flex-col gap-1",
                        msg.role === "user" && "items-end"
                      )}
                    >
<span
                      className={cn(
                        "text-[10px] text-muted-foreground",
                        msg.role === "user" && "text-right"
                      )}
                    >
                      {msg.role === "assistant" ? "AI" : "You"} ·{" "}
                      {new Date(msg.timestamp).toLocaleTimeString("en-US", {
                        hour: "2-digit",
                        minute: "2-digit",
                        hour12: false,
                      })}
                    </span>
                    <div className="group relative flex items-start gap-2">
                      <div
                        className={cn(
                          "rounded-xl px-3 py-2 text-sm leading-relaxed whitespace-pre-wrap",
                          msg.role === "user"
                            ? "rounded-tr-sm bg-primary text-primary-foreground"
                            : "rounded-tl-sm bg-secondary text-foreground"
                        )}
                      >
                        {msg.content}
                      </div>
                      {msg.role === "assistant" && (
                        <button
                          onClick={() => handleCopyMessage(msg.content, msg.id)}
                          className="absolute -top-2 -right-2 opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded bg-secondary hover:bg-primary/20"
                          title="Copy"
                        >
                          {copiedId === msg.id ? (
                            <Check className="h-3 w-3 text-green-500" />
                          ) : (
                            <Copy className="h-3 w-3 text-muted-foreground" />
                          )}
                        </button>
                      )}
                    </div>
                    </div>
                  </div>
                ))}
                {chat.isLoading && (
                  <div className="flex gap-3">
                    <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary/20 text-primary">
                      <Bot className="h-4 w-4" />
                    </div>
                    <div className="flex items-center gap-2 rounded-xl bg-secondary px-3 py-2">
                      <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                      <span className="text-sm text-muted-foreground">Processing...</span>
                    </div>
                  </div>
                )}
                <div ref={bottomRef} />
              </div>
            </div>

            {/* Input */}
            <div className="flex items-center gap-2 border-t border-[var(--window-border)] p-2">
              <button
                onClick={handleClear}
                className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-secondary hover:text-foreground transition-colors"
                title="Clear chat"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
              <button
                onClick={() => openDirectory()}
                className="flex h-7 items-center gap-1 rounded-md px-2 text-muted-foreground hover:bg-secondary hover:text-foreground transition-colors"
                title="Open folder"
              >
                <FolderOpen className="h-3.5 w-3.5" />
                <span className="text-xs">Open Folder</span>
              </button>
              <div className="flex flex-1 items-end gap-2 rounded-lg bg-secondary px-3 py-2">
                <textarea
                  ref={inputRef}
                  value={inputValue}
                  onChange={(e) => setInputValue(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="Ask or say 'open folder'..."
                  className="flex-1 resize-none bg-transparent text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
                  rows={1}
                  style={{ minHeight: "24px", maxHeight: "100px" }}
                />
                <button
                  onClick={handleSend}
                  disabled={chat.isLoading || !inputValue.trim()}
                  className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground transition-opacity disabled:opacity-40"
                >
                  <Send className="h-3.5 w-3.5" />
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Editor panel */}
        {(viewMode === "editor" || viewMode === "split") && (
          <div className={cn("flex flex-col", viewMode === "split" ? "w-1/2" : "w-full")}>
            {/* File tree */}
            <div className="flex h-32 flex-shrink-0 border-b border-[var(--window-border)] overflow-auto">
              <FileTreeView />
            </div>
            
            {/* Code editor */}
            <div className="flex-1 overflow-hidden">
              <CodeEditor />
            </div>
          </div>
        )}
      </div>

      {/* Settings Modal */}
      {showSettings && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="flex h-[80vh] w-[90vw] max-w-4xl flex-col rounded-lg border border-[var(--border)] bg-[var(--bg)]">
            <div className="flex items-center justify-between border-b border-[var(--border)] px-4 py-2">
              <h2 className="font-medium">Edit System Prompt</h2>
              <button
                onClick={() => setShowSettings(false)}
                className="text-muted-foreground hover:text-foreground"
              >
                ✕
              </button>
            </div>
            <div className="flex-1 p-4">
              <textarea
                value={systemPrompt}
                onChange={(e) => setSystemPrompt(e.target.value)}
                className="h-full w-full resize-none font-mono text-sm bg-[var(--bg-hover)] p-3 rounded outline-none"
                spellCheck={false}
              />
            </div>
            <div className="flex items-center justify-between border-t border-[var(--border)] px-4 py-2">
              <button
                onClick={() => {
                  resetSystemPrompt()
                  setSystemPrompt(getSystemPrompt())
                }}
                className="text-sm text-muted-foreground hover:text-foreground"
              >
                Reset to Default
              </button>
              <div className="flex gap-2">
                <button
                  onClick={() => setShowSettings(false)}
                  className="px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground"
                >
                  Cancel
                </button>
                <button
                  onClick={() => {
                    saveSystemPrompt(systemPrompt)
                    setShowSettings(false)
                    addMessage("assistant", "✅ System prompt saved! I'll use the new instructions now.")
                  }}
                  className="px-3 py-1.5 text-sm bg-[var(--accent)] text-[var(--accent-fg)] rounded"
                >
                  Save
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}