"use client"

import { useStore } from "@nanostores/react"
import { assistantSession, addMessage, setLoading, clearSession } from "@/platform/assitant/assitant-session"
import { toolRegistry, invokeToolCall } from "@/platform/registries/tool-registry"
import { getDashboardMode } from "@/platform/runtime/dashboard-mode"
import { buildSystemPrompt } from "@/platform/assitant/assitant-prompts"
import { cn } from "@/lib/utils"
import {
  Send,
  Sparkles,
  Loader2,
  Bot,
  User,
  Trash2,
  Copy,
  Check,
} from "lucide-react"
import { useState, useRef, useEffect, useCallback } from "react"

export function AssistantApp() {
  const session = useStore(assistantSession)
  const tools = useStore(toolRegistry)
  const [inputValue, setInputValue] = useState("")
  const [copiedId, setCopiedId] = useState<string | null>(null)
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const bottomRef = useRef<HTMLDivElement>(null)
  const mode = getDashboardMode()

  const scrollToBottom = useCallback(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" })
  }, [])

  useEffect(() => {
    scrollToBottom()
  }, [session.messages, scrollToBottom])

  const handleCopyMessage = async (content: string, id: string) => {
    await navigator.clipboard.writeText(content)
    setCopiedId(id)
    setTimeout(() => setCopiedId(null), 2000)
  }

  const handleSend = async () => {
    const text = inputValue.trim()
    if (!text || session.isLoading) return

    addMessage("user", text)
    setInputValue("")
    setLoading(true)

    try {
      const messages = session.messages.map((m) => ({
        role: m.role,
        content: m.content,
      }))
      messages.push({ role: "user", content: text })

      const systemPrompt = buildSystemPrompt()

      const response = await fetch("/api/assistant", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          messages,
          systemPrompt,
        }),
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`)
      }

      const data = await response.json()
      addMessage("assitant", data.text, data.toolCalls)
    } catch (error) {
      addMessage("assitant", `Error: ${error instanceof Error ? error.message : "Failed to get response"}`)
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
    clearSession()
  }

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex h-10 flex-shrink-0 items-center gap-2 border-b border-[var(--window-border)] px-4">
        <Sparkles className="h-4 w-4 text-primary" />
        <span className="text-sm font-medium text-foreground">AI Assistant</span>

        {mode === "demo" && (
          <span className="rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
            Demo
          </span>
        )}
        {mode === "offline" && (
          <span className="rounded bg-red-500/20 px-1.5 py-0.5 text-[9px] font-medium text-red-400">
            Offline
          </span>
        )}

        <div className="ml-auto flex items-center gap-1">
          <button
            onClick={handleClear}
            className="flex items-center gap-1 rounded px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
            title="Clear chat"
          >
            <Trash2 className="h-3 w-3" />
          </button>
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4">
        <div className="space-y-4">
          {session.messages.map((msg) => (
            <div
              key={msg.id}
              className={cn(
                "flex gap-3",
                msg.role === "user" && "flex-row-reverse"
              )}
            >
              <div
                className={cn(
                  "flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full",
                  msg.role === "assitant"
                    ? "bg-primary/20 text-primary"
                    : "bg-secondary text-muted-foreground"
                )}
              >
                {msg.role === "assitant" ? (
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
                  {msg.role === "assitant" ? "AI" : "You"} ·{" "}
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
                  {msg.role === "assitant" && (
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

                {/* Tool calls display */}
                {msg.toolCalls && msg.toolCalls.length > 0 && (
                  <div className="mt-2 space-y-1">
                    {msg.toolCalls.map((tc, i) => (
                      <div
                        key={i}
                        className={cn(
                          "rounded border px-2 py-1 text-xs",
                          tc.status === "executed" && "border-green-500/30 bg-green-500/10 text-green-400",
                          tc.status === "failed" && "border-red-500/30 bg-red-500/10 text-red-400",
                          tc.status === "pending" && "border-yellow-500/30 bg-yellow-500/10 text-yellow-400",
                        )}
                      >
                        <span className="font-medium">{tc.toolId}</span>
                        {tc.error && <span className="ml-1">— {tc.error}</span>}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          ))}

          {session.isLoading && (
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
        <div className="flex flex-1 items-end gap-2 rounded-lg bg-secondary px-3 py-2">
          <textarea
            ref={inputRef}
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Ask me something..."
            className="flex-1 resize-none bg-transparent text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
            rows={1}
            style={{ minHeight: "24px", maxHeight: "100px" }}
          />
          <button
            onClick={handleSend}
            disabled={session.isLoading || !inputValue.trim()}
            className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground transition-opacity disabled:opacity-40"
          >
            <Send className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
    </div>
  )
}
