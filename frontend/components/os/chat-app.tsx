"use client"

import { useState, useEffect, useCallback, useRef } from "react"
import { cn } from "@/lib/utils"
import { formatTime } from "@/lib/format"
import { AppAvatar } from "@/components/ui/app-avatar"
import {
  Send,
  Lock,
  Plus,
  Hash,
} from "lucide-react"
import { getDashboardMode, isDemoMode } from "@/platform/runtime/dashboard-mode"

interface Message {
  id: string
  sender: string
  text: string
  ts: Date
  self: boolean
}

interface Channel {
  id: string
  name: string
  type: "dm" | "channel"
  unread: number
  messages: Message[]
}

// Demo data — clearly isolated as demo
const DEMO_CHANNELS: Channel[] = [
  {
    id: "general",
    name: "general",
    type: "channel",
    unread: 0,
    messages: [
      { id: "m1", sender: "Elias Voss", text: "WASM module deployed to 8 nodes", ts: new Date(Date.now() - 120000), self: false },
      { id: "m2", sender: "Priya Mehta", text: "Latency looks great on the new routing", ts: new Date(Date.now() - 60000), self: false },
      { id: "m3", sender: "You", text: "Pushed the fingerprint auth update too", ts: new Date(Date.now() - 30000), self: true },
    ],
  },
  {
    id: "ops",
    name: "ops",
    type: "channel",
    unread: 2,
    messages: [
      { id: "m4", sender: "Mila Dube", text: "Node ed3f:m3n4 showing elevated CPU", ts: new Date(Date.now() - 300000), self: false },
      { id: "m5", sender: "Dax Okafor", text: "Rolled back compute-node v2.1", ts: new Date(Date.now() - 240000), self: false },
    ],
  },
  {
    id: "ara",
    name: "Ara Nakamura",
    type: "dm",
    unread: 1,
    messages: [
      { id: "m6", sender: "Ara Nakamura", text: "Did you see the new edge routing RFC?", ts: new Date(Date.now() - 600000), self: false },
    ],
  },
]

interface DemoChatAppProps {
  initialChannelId?: string
}

export function DemoChatApp({ initialChannelId }: DemoChatAppProps) {
  const [channels, setChannels] = useState<Channel[]>(DEMO_CHANNELS)
  const [activeId, setActiveId] = useState(initialChannelId || "general")
  const [input, setInput] = useState("")
  const bottomRef = useRef<HTMLDivElement>(null)
  const mode = getDashboardMode()

  const active = channels.find((c) => c.id === activeId) || channels[0]

  const scrollToBottom = useCallback(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" })
  }, [])

  useEffect(() => { scrollToBottom() }, [activeId, scrollToBottom])
  useEffect(() => {
    if (active.messages.length) scrollToBottom()
  }, [channels])

  const handleSelect = (id: string) => {
    setActiveId(id)
    setChannels((prev) => prev.map((c) => c.id === id ? { ...c, unread: 0 } : c))
  }

  const handleSend = () => {
    const text = input.trim()
    if (!text) return
    setInput("")
    const msg: Message = {
      id: `m-${Date.now()}-${Math.random()}`,
      sender: "You",
      text,
      ts: new Date(),
      self: true,
    }
    setChannels((prev) =>
      prev.map((c) => c.id === activeId ? { ...c, messages: [...c.messages, msg] } : c)
    )
  }

  const dmChannels = channels.filter((c) => c.type === "dm")
  const groupChannels = channels.filter((c) => c.type === "channel")

  return (
    <div className="flex h-full">
      {/* Sidebar */}
      <div className="flex w-44 flex-shrink-0 flex-col border-r border-[var(--window-border)] py-2">
        <div className="mb-1 px-3">
          <p className="text-[9px] font-semibold uppercase tracking-widest text-muted-foreground/70">
            Channels
            {mode === "demo" && (
              <span className="ml-1 rounded bg-yellow-500/20 px-1 text-[8px] text-yellow-400">Demo</span>
            )}
          </p>
        </div>
        {groupChannels.map((c) => (
          <button
            key={c.id}
            onClick={() => handleSelect(c.id)}
            className={cn(
              "flex items-center gap-1.5 px-3 py-1.5 text-left text-sm transition-colors",
              c.id === activeId ? "bg-secondary text-foreground" : "text-muted-foreground hover:text-foreground"
            )}
          >
            <Hash className="h-3.5 w-3.5 flex-shrink-0" />
            <span className="truncate flex-1 text-xs">{c.name}</span>
            {c.unread > 0 && (
              <span className="flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[9px] font-bold text-primary-foreground">
                {c.unread}
              </span>
            )}
          </button>
        ))}

        <div className="mb-1 mt-4 px-3">
          <p className="text-[9px] font-semibold uppercase tracking-widest text-muted-foreground/70">
            Direct
            {mode === "demo" && (
              <span className="ml-1 rounded bg-yellow-500/20 px-1 text-[8px] text-yellow-400">Demo</span>
            )}
          </p>
        </div>
        {dmChannels.map((c) => (
          <button
            key={c.id}
            onClick={() => handleSelect(c.id)}
            className={cn(
              "flex items-center gap-2 px-3 py-1.5 text-left transition-colors",
              c.id === activeId ? "bg-secondary text-foreground" : "text-muted-foreground hover:text-foreground"
            )}
          >
            <AppAvatar name={c.name} size="xs" />
            <span className="truncate flex-1 text-xs">{c.name.split(" ")[0]}</span>
            {c.unread > 0 && (
              <span className="flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[9px] font-bold text-primary-foreground">
                {c.unread}
              </span>
            )}
          </button>
        ))}

        <div className="mt-auto border-t border-[var(--window-border)] px-3 pt-3">
          <div className="flex items-center gap-1 text-[10px] text-muted-foreground/60">
            <Lock className="h-3 w-3" />
            <span>{mode === "demo" ? "Demo — not real E2E" : "E2E encrypted"}</span>
          </div>
        </div>
      </div>

      {/* Main chat */}
      <div className="flex flex-1 flex-col">
        <div className="flex h-10 flex-shrink-0 items-center gap-2 border-b border-[var(--window-border)] px-4">
          {active.type === "channel" ? (
            <Hash className="h-4 w-4 text-muted-foreground" />
          ) : (
            <AppAvatar name={active.name} size="xs" />
          )}
          <span className="text-sm font-medium text-foreground">
            {active.type === "channel" ? active.name : active.name.split(" ")[0]}
          </span>
          {mode === "demo" && (
            <span className="rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
              Demo — not real messaging
            </span>
          )}
          <span className="ml-auto text-[10px] text-muted-foreground">{active.messages.length} messages</span>
        </div>

        <div className="flex-1 space-y-3 overflow-y-auto p-4">
          {active.messages.length === 0 && (
            <p className="text-center text-xs text-muted-foreground pt-8">No messages yet. Say hello.</p>
          )}
          {active.messages.map((msg, i) => {
            const showSender = i === 0 || active.messages[i - 1].sender !== msg.sender
            return (
              <div key={msg.id} className={cn("flex gap-2.5", msg.self && "flex-row-reverse")}>
                {showSender && !msg.self && <AppAvatar name={msg.sender} size="sm" />}
                {!showSender && !msg.self && <div className="w-8 flex-shrink-0" />}
                <div className={cn("flex max-w-[75%] flex-col gap-0.5", msg.self && "items-end")}>
                  {showSender && (
                    <span className={cn("text-[10px] text-muted-foreground", msg.self && "text-right")}>
                      {msg.self ? "You" : msg.sender.split(" ")[0]} · {formatTime(msg.ts)}
                    </span>
                  )}
                  <div
                    className={cn(
                      "rounded-xl px-3 py-2 text-sm leading-relaxed",
                      msg.self
                        ? "rounded-tr-sm bg-primary text-primary-foreground"
                        : "rounded-tl-sm bg-secondary text-foreground"
                    )}
                  >
                    {msg.text}
                  </div>
                </div>
              </div>
            )
          })}
          <div ref={bottomRef} />
        </div>

        <div className="border-t border-[var(--window-border)] p-3">
          <div className="flex items-center gap-2 rounded-lg bg-secondary px-3 py-2">
            <input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); handleSend() } }}
              placeholder={mode === "demo" ? "Demo chat — messages are not real..." : `Message ${active.type === "channel" ? "#" + active.name : active.name.split(" ")[0]}...`}
              className="flex-1 bg-transparent text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
            />
            <button
              onClick={handleSend}
              disabled={!input.trim()}
              className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground transition-opacity disabled:opacity-40"
            >
              <Send className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
