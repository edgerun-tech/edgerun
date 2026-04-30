"use client"

import { useState, useMemo } from "react"
import { Search, Plus, User, Phone, MessageSquare, X, ChevronRight, Fingerprint } from "lucide-react"
import { cn } from "@/lib/utils"

interface Contact {
  id: string
  name: string
  handle: string
  nodeId: string
  status: "online" | "offline" | "idle"
  lastSeen?: string
  avatar?: string
  tags: string[]
}

const DEMO_CONTACTS: Contact[] = [
  { id: "1", name: "Ara Nakamura", handle: "@ara.run", nodeId: "ed3f:a1b2", status: "online", tags: ["team"] },
  { id: "2", name: "Elias Voss", handle: "@voss.edge", nodeId: "ed3f:c3d4", status: "online", tags: ["team", "dev"] },
  { id: "3", name: "Seren Aydın", handle: "@seren", nodeId: "ed3f:e5f6", status: "idle", lastSeen: "2m ago", tags: ["external"] },
  { id: "4", name: "Tomás Reyes", handle: "@t.reyes", nodeId: "ed3f:g7h8", status: "offline", lastSeen: "1h ago", tags: ["dev"] },
  { id: "5", name: "Priya Mehta", handle: "@priya.m", nodeId: "ed3f:i9j0", status: "online", tags: ["team"] },
  { id: "6", name: "Dax Okafor", handle: "@dax.ok", nodeId: "ed3f:k1l2", status: "offline", lastSeen: "3d ago", tags: ["external"] },
  { id: "7", name: "Mila Dube", handle: "@mila", nodeId: "ed3f:m3n4", status: "online", tags: ["team", "dev"] },
  { id: "8", name: "Rémi Laval", handle: "@r.laval", nodeId: "ed3f:o5p6", status: "idle", lastSeen: "15m ago", tags: ["external"] },
]

const STATUS_COLOR: Record<Contact["status"], string> = {
  online: "bg-[var(--status-online)]",
  idle: "bg-[var(--status-warning)]",
  offline: "bg-muted-foreground",
}

function Avatar({ name, size = "md" }: { name: string; size?: "sm" | "md" | "lg" }) {
  const initials = name.split(" ").map((n) => n[0]).join("").slice(0, 2).toUpperCase()
  const hue = name.split("").reduce((acc, c) => acc + c.charCodeAt(0), 0) % 360
  return (
    <div
      className={cn(
        "flex flex-shrink-0 items-center justify-center rounded-full font-mono font-bold",
        size === "sm" && "h-8 w-8 text-xs",
        size === "md" && "h-10 w-10 text-sm",
        size === "lg" && "h-14 w-14 text-base",
      )}
      style={{ background: `oklch(0.3 0.1 ${hue})`, color: `oklch(0.85 0.1 ${hue})` }}
    >
      {initials}
    </div>
  )
}

interface ContactsAppProps {
  onCall?: (contact: Contact) => void
  onMessage?: (contact: Contact) => void
}

export function ContactsApp({ onCall, onMessage }: ContactsAppProps) {
  const [query, setQuery] = useState("")
  const [selected, setSelected] = useState<Contact | null>(null)
  const [showAdd, setShowAdd] = useState(false)
  const [newName, setNewName] = useState("")
  const [newHandle, setNewHandle] = useState("")
  const [contacts, setContacts] = useState<Contact[]>(DEMO_CONTACTS)

  const filtered = useMemo(() =>
    contacts.filter((c) =>
      c.name.toLowerCase().includes(query.toLowerCase()) ||
      c.handle.toLowerCase().includes(query.toLowerCase())
    ), [contacts, query])

  const handleAdd = () => {
    if (!newName.trim()) return
    const contact: Contact = {
      id: `c-${Date.now()}`,
      name: newName.trim(),
      handle: newHandle.trim() || `@${newName.trim().toLowerCase().replace(/\s+/, ".")}`,
      nodeId: `ed3f:${Math.random().toString(16).slice(2, 6)}`,
      status: "offline",
      tags: [],
    }
    setContacts((prev) => [contact, ...prev])
    setNewName("")
    setNewHandle("")
    setShowAdd(false)
  }

  return (
    <div className="flex h-full">
      {/* Sidebar */}
      <div className="flex w-56 flex-shrink-0 flex-col border-r border-[var(--window-border)]">
        {/* Search + Add */}
        <div className="flex items-center gap-2 border-b border-[var(--window-border)] p-3">
          <div className="relative flex-1">
            <Search className="absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search..."
              className="h-7 w-full rounded-md bg-secondary pl-7 pr-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
            />
          </div>
          <button
            onClick={() => setShowAdd((v) => !v)}
            className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-md bg-secondary text-muted-foreground transition-colors hover:bg-primary hover:text-primary-foreground"
          >
            {showAdd ? <X className="h-3.5 w-3.5" /> : <Plus className="h-3.5 w-3.5" />}
          </button>
        </div>

        {/* Add form */}
        {showAdd && (
          <div className="border-b border-[var(--window-border)] p-3 space-y-2">
            <input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="Display name"
              className="h-7 w-full rounded-md bg-secondary px-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
            />
            <input
              value={newHandle}
              onChange={(e) => setNewHandle(e.target.value)}
              placeholder="@handle (optional)"
              className="h-7 w-full rounded-md bg-secondary px-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
            />
            <button
              onClick={handleAdd}
              className="h-7 w-full rounded-md bg-primary px-2 text-xs font-medium text-primary-foreground transition-colors hover:opacity-90"
            >
              Add contact
            </button>
          </div>
        )}

        {/* List */}
        <div className="flex-1 overflow-y-auto">
          {filtered.length === 0 && (
            <p className="p-4 text-center text-xs text-muted-foreground">No contacts found</p>
          )}
          {filtered.map((c) => (
            <button
              key={c.id}
              onClick={() => setSelected(c)}
              className={cn(
                "flex w-full items-center gap-2.5 px-3 py-2.5 text-left transition-colors hover:bg-secondary/70",
                selected?.id === c.id && "bg-secondary"
              )}
            >
              <div className="relative">
                <Avatar name={c.name} size="sm" />
                <span className={cn("absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full border-2 border-[var(--window-bg)]", STATUS_COLOR[c.status])} />
              </div>
              <div className="min-w-0 flex-1">
                <p className="truncate text-xs font-medium text-foreground">{c.name}</p>
                <p className="truncate text-[10px] text-muted-foreground">{c.handle}</p>
              </div>
              {selected?.id === c.id && <ChevronRight className="h-3 w-3 flex-shrink-0 text-primary" />}
            </button>
          ))}
        </div>
      </div>

      {/* Detail panel */}
      <div className="flex flex-1 flex-col items-center justify-center p-6">
        {selected ? (
          <div className="flex w-full max-w-xs flex-col items-center gap-4">
            <div className="relative">
              <Avatar name={selected.name} size="lg" />
              <span className={cn("absolute -bottom-1 -right-1 h-4 w-4 rounded-full border-2 border-[var(--window-bg)]", STATUS_COLOR[selected.status])} />
            </div>
            <div className="text-center">
              <h2 className="text-lg font-semibold text-foreground">{selected.name}</h2>
              <p className="text-sm text-muted-foreground">{selected.handle}</p>
            </div>

            {/* Node ID */}
            <div className="flex w-full items-center gap-2 rounded-lg bg-secondary/50 px-3 py-2">
              <Fingerprint className="h-4 w-4 flex-shrink-0 text-muted-foreground" />
              <div className="min-w-0">
                <p className="text-[10px] text-muted-foreground">Node ID</p>
                <p className="font-mono text-xs text-foreground">{selected.nodeId}</p>
              </div>
            </div>

            {/* Status */}
            <div className="flex w-full items-center justify-between rounded-lg bg-secondary/50 px-3 py-2">
              <span className="text-xs text-muted-foreground">Status</span>
              <div className="flex items-center gap-1.5">
                <span className={cn("h-2 w-2 rounded-full", STATUS_COLOR[selected.status])} />
                <span className="text-xs font-medium text-foreground capitalize">
                  {selected.status === "offline" && selected.lastSeen
                    ? `Offline · ${selected.lastSeen}`
                    : selected.status}
                </span>
              </div>
            </div>

            {/* Tags */}
            {selected.tags.length > 0 && (
              <div className="flex flex-wrap justify-center gap-1.5">
                {selected.tags.map((tag) => (
                  <span key={tag} className="rounded-full bg-primary/15 px-2 py-0.5 text-[10px] font-medium text-primary">
                    {tag}
                  </span>
                ))}
              </div>
            )}

            {/* Actions */}
            <div className="flex gap-2 pt-1">
              <button
                onClick={() => onCall?.(selected)}
                className="flex items-center gap-1.5 rounded-lg bg-[var(--status-online)]/15 px-4 py-2 text-xs font-medium text-[var(--status-online)] transition-colors hover:bg-[var(--status-online)]/25"
              >
                <Phone className="h-3.5 w-3.5" />
                Call
              </button>
              <button
                onClick={() => onMessage?.(selected)}
                className="flex items-center gap-1.5 rounded-lg bg-secondary px-4 py-2 text-xs font-medium text-foreground transition-colors hover:bg-secondary/70"
              >
                <MessageSquare className="h-3.5 w-3.5" />
                Message
              </button>
            </div>
          </div>
        ) : (
          <div className="flex flex-col items-center gap-3 text-center">
            <User className="h-10 w-10 text-muted-foreground/30" />
            <p className="text-sm text-muted-foreground">Select a contact</p>
          </div>
        )}
      </div>
    </div>
  )
}
