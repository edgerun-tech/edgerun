"use client"

import { useState, useEffect, useRef } from "react"
import { cn } from "@/lib/utils"

interface LogEntry {
  id: string
  timestamp: Date
  type: "info" | "success" | "warning" | "error" | "system"
  message: string
}

interface TerminalProps {
  logs: LogEntry[]
  onCommand?: (command: string) => void
}

export function Terminal({ logs, onCommand }: TerminalProps) {
  const [inputValue, setInputValue] = useState("")
  const [commandHistory, setCommandHistory] = useState<string[]>([])
  const [historyIndex, setHistoryIndex] = useState(-1)
  const scrollRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [logs])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (inputValue.trim()) {
      setCommandHistory((prev) => [...prev, inputValue])
      onCommand?.(inputValue)
      setInputValue("")
      setHistoryIndex(-1)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowUp") {
      e.preventDefault()
      if (historyIndex < commandHistory.length - 1) {
        const newIndex = historyIndex + 1
        setHistoryIndex(newIndex)
        setInputValue(commandHistory[commandHistory.length - 1 - newIndex])
      }
    } else if (e.key === "ArrowDown") {
      e.preventDefault()
      if (historyIndex > 0) {
        const newIndex = historyIndex - 1
        setHistoryIndex(newIndex)
        setInputValue(commandHistory[commandHistory.length - 1 - newIndex])
      } else if (historyIndex === 0) {
        setHistoryIndex(-1)
        setInputValue("")
      }
    }
  }

  const getTypeColor = (type: LogEntry["type"]) => {
    switch (type) {
      case "success":
        return "text-[var(--status-online)]"
      case "warning":
        return "text-[var(--status-warning)]"
      case "error":
        return "text-[var(--status-error)]"
      case "system":
        return "text-primary"
      default:
        return "text-muted-foreground"
    }
  }

  const getTypePrefix = (type: LogEntry["type"]) => {
    switch (type) {
      case "success":
        return "[OK]"
      case "warning":
        return "[WARN]"
      case "error":
        return "[ERR]"
      case "system":
        return "[SYS]"
      default:
        return "[INFO]"
    }
  }

  return (
    <div
      className="flex h-full flex-col bg-[var(--terminal-bg)] font-mono text-sm"
      onClick={() => inputRef.current?.focus()}
    >
      {/* Log Output */}
      <div ref={scrollRef} className="flex-1 overflow-auto p-3">
        {logs.map((log, index) => (
          <div
            key={log.id}
            className="animate-terminal-line flex gap-2 py-0.5"
            style={{ animationDelay: `${index * 20}ms` }}
          >
            <span className="flex-shrink-0 text-muted-foreground/50">
              {log.timestamp.toLocaleTimeString("en-US", {
                hour: "2-digit",
                minute: "2-digit",
                second: "2-digit",
                hour12: false,
              })}
            </span>
            <span className={cn("flex-shrink-0", getTypeColor(log.type))}>
              {getTypePrefix(log.type)}
            </span>
            <span className="text-foreground">{log.message}</span>
          </div>
        ))}
      </div>

      {/* Input Line */}
      <form onSubmit={handleSubmit} className="border-t border-border/50 p-3">
        <div className="flex items-center gap-2">
          <span className="text-primary">❯</span>
          <input
            ref={inputRef}
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            className="flex-1 bg-transparent text-[var(--terminal-text)] outline-none placeholder:text-muted-foreground/50"
            placeholder="Enter command..."
            autoComplete="off"
            spellCheck={false}
          />
        </div>
      </form>
    </div>
  )
}

// Helper to generate mock logs
export function generateMockLogs(): LogEntry[] {
  const messages: { type: LogEntry["type"]; message: string }[] = [
    { type: "system", message: "WASI Runtime v2.4.1 initialized" },
    { type: "info", message: "Connecting to peer network..." },
    { type: "success", message: "WebRTC signaling established" },
    { type: "info", message: "Discovered 12 active nodes" },
    { type: "success", message: "Joined cluster: us-east-1" },
    { type: "info", message: "Loading WASM module: runtime-core.wasm" },
    { type: "success", message: "Module loaded (2.3MB, 145ms)" },
    { type: "warning", message: "Memory usage at 68% capacity" },
    { type: "info", message: "Syncing distributed state..." },
    { type: "success", message: "State sync complete (1,247 entries)" },
  ]

  return messages.map((msg, i) => ({
    id: `log-${i}`,
    timestamp: new Date(Date.now() - (messages.length - i) * 2000),
    ...msg,
  }))
}
