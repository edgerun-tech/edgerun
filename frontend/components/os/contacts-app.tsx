"use client"

import { useMemo, useState } from "react"
import { ChevronRight, Copy, Fingerprint, Mail, MessageSquare, Plus, Search, User, UserPlus, X } from "lucide-react"
import { cn } from "@/lib/utils"
import { useAuth, type ContactRecord, type UnlockedProfileContainer } from "@/hooks/use-auth"

type ContactsAppProps = {
  onClose?: () => void
  onMessage?: (contact: ContactRecord) => void
  onCall?: (contact: ContactRecord) => void
}

function contactCard(handle: string, publicKey: string) {
  return [
    "Edgerun contact",
    `Name: ${handle}`,
    `Public encryption key: ${publicKey}`,
  ].join("\n")
}

function emailContact(handle: string, publicKey: string) {
  const subject = encodeURIComponent(`${handle}'s Edgerun contact`)
  const body = encodeURIComponent(contactCard(handle, publicKey))
  window.location.href = `mailto:?subject=${subject}&body=${body}`
}

function Avatar({ name, size = "md" }: { name: string; size?: "sm" | "md" | "lg" }) {
  const initials = name.split(" ").map((part) => part[0]).join("").slice(0, 2).toUpperCase() || "ID"
  const hue = name.split("").reduce((acc, char) => acc + char.charCodeAt(0), 0) % 360
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

function routeLabel(contact: ContactRecord) {
  return contact.routeHint === "local-profile" ? "Local profile" : "Network contact"
}

function shortId(value: string) {
  return `${value.slice(0, 12)}...${value.slice(-6)}`
}

export function ContactsApp({ onClose, onMessage }: ContactsAppProps) {
  const auth = useAuth()
  const profile = auth.unlockedProfile as UnlockedProfileContainer | null
  const [query, setQuery] = useState("")
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [showAdd, setShowAdd] = useState(false)
  const [password, setPassword] = useState("")
  const [contactLabel, setContactLabel] = useState("")
  const [contactPublicKey, setContactPublicKey] = useState("")
  const [localProfileId, setLocalProfileId] = useState("")

  const contacts = useMemo(() => {
    if (!profile) return []
    const normalizedQuery = query.trim().toLowerCase()
    return profile.contacts
      .filter((contact) => contact.identityIdHex !== profile.ownerEncryption.identityIdHex)
      .filter((contact) => (
        !normalizedQuery
        || contact.label.toLowerCase().includes(normalizedQuery)
        || routeLabel(contact).toLowerCase().includes(normalizedQuery)
      ))
  }, [profile, query])

  const localProfileOptions = useMemo(() => {
    if (!profile) return []
    return auth.profileSummaries.filter((summary) => (
      summary.encryptionPublicKeyHint
      && summary.encryptionIdHint !== profile.ownerEncryption.identityIdHex
      && !profile.contacts.some((contact) => contact.identityIdHex === summary.encryptionIdHint)
    ))
  }, [auth.profileSummaries, profile])

  if (!profile) return null

  const selected = contacts.find((contact) => contact.identityIdHex === selectedId) ?? contacts[0] ?? null
  const canAddLocal = Boolean(password && localProfileOptions.length > 0)
  const canAddExternal = Boolean(password && contactPublicKey.trim())

  async function addLocalProfileContact() {
    const local = localProfileOptions.find((item) => item.profileId === localProfileId) ?? localProfileOptions[0]
    if (!local) return
    await auth.addContact({
      label: local.handle,
      publicKeyRawBase64: local.encryptionPublicKeyHint,
      routeHint: "local-profile",
      password,
    })
    setLocalProfileId("")
  }

  async function addExternalContact() {
    await auth.addContact({
      label: contactLabel.trim() || "Contact",
      publicKeyRawBase64: contactPublicKey,
      routeHint: "mesh",
      password,
    })
    setContactLabel("")
    setContactPublicKey("")
  }

  async function copyMyContact() {
    if (!profile) return
    await navigator.clipboard.writeText(contactCard(profile.handle, profile.ownerEncryption.publicKeyRawBase64))
  }

  const app = (
    <div className="flex h-full bg-background text-foreground">
      <div className="flex w-60 flex-shrink-0 flex-col border-r border-[var(--window-border)]">
        <div className="flex items-center gap-2 border-b border-[var(--window-border)] p-3">
          <div className="relative flex-1">
            <Search className="absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search..."
              className="h-8 w-full rounded-md bg-secondary pl-7 pr-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
            />
          </div>
          <button
            onClick={() => setShowAdd((value) => !value)}
            className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-md bg-secondary text-muted-foreground transition-colors hover:bg-primary hover:text-primary-foreground"
            aria-label={showAdd ? "Close add contact" : "Add contact"}
          >
            {showAdd ? <X className="h-3.5 w-3.5" /> : <Plus className="h-3.5 w-3.5" />}
          </button>
        </div>

        {showAdd && (
          <div className="space-y-2 border-b border-[var(--window-border)] p-3">
            <input
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              type="password"
              placeholder="Profile password"
              className="h-8 w-full rounded-md bg-secondary px-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
            />
            {localProfileOptions.length > 0 && (
              <div className="grid gap-2">
                <select
                  value={localProfileId}
                  onChange={(event) => setLocalProfileId(event.target.value)}
                  className="h-8 rounded-md bg-secondary px-2 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
                >
                  {localProfileOptions.map((summary) => <option key={summary.profileId} value={summary.profileId}>{summary.handle}</option>)}
                </select>
                <button
                  onClick={addLocalProfileContact}
                  disabled={!canAddLocal || auth.isLoading}
                  className="h-8 rounded-md bg-primary px-2 text-xs font-medium text-primary-foreground transition-opacity disabled:opacity-40"
                >
                  Add local profile
                </button>
              </div>
            )}
            <input
              value={contactLabel}
              onChange={(event) => setContactLabel(event.target.value)}
              placeholder="Contact name"
              className="h-8 w-full rounded-md bg-secondary px-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
            />
            <textarea
              value={contactPublicKey}
              onChange={(event) => setContactPublicKey(event.target.value)}
              placeholder="Public encryption key"
              className="min-h-20 w-full rounded-md bg-secondary px-2 py-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
            />
            <button
              onClick={addExternalContact}
              disabled={!canAddExternal || auth.isLoading}
              className="h-8 w-full rounded-md bg-primary px-2 text-xs font-medium text-primary-foreground transition-opacity disabled:opacity-40"
            >
              Add contact
            </button>
          </div>
        )}

        <div className="flex-1 overflow-y-auto">
          {contacts.length === 0 && (
            <div className="flex h-full flex-col items-center justify-center gap-2 p-5 text-center">
              <User className="h-8 w-8 text-muted-foreground/30" />
              <p className="text-xs text-muted-foreground">No contacts yet</p>
            </div>
          )}
          {contacts.map((contact) => (
            <button
              key={contact.identityIdHex}
              onClick={() => setSelectedId(contact.identityIdHex)}
              className={cn(
                "flex w-full items-center gap-2.5 px-3 py-2.5 text-left transition-colors hover:bg-secondary/70",
                selected?.identityIdHex === contact.identityIdHex && "bg-secondary",
              )}
            >
              <Avatar name={contact.label} size="sm" />
              <div className="min-w-0 flex-1">
                <p className="truncate text-xs font-medium text-foreground">{contact.label}</p>
                <p className="truncate text-[10px] text-muted-foreground">{routeLabel(contact)}</p>
              </div>
              {selected?.identityIdHex === contact.identityIdHex && <ChevronRight className="h-3 w-3 flex-shrink-0 text-primary" />}
            </button>
          ))}
        </div>

        <div className="border-t border-[var(--window-border)] p-3">
          <button
            onClick={copyMyContact}
            className="flex h-8 w-full items-center justify-center gap-1.5 rounded-md bg-secondary px-2 text-xs font-medium text-foreground transition-colors hover:bg-secondary/70"
          >
            <Copy className="h-3.5 w-3.5" />
            Copy my contact
          </button>
        </div>
      </div>

      <div className="flex flex-1 flex-col items-center justify-center p-6">
        {selected ? (
          <div className="flex w-full max-w-sm flex-col items-center gap-4">
            <Avatar name={selected.label} size="lg" />
            <div className="text-center">
              <h2 className="text-lg font-semibold text-foreground">{selected.label}</h2>
              <p className="text-sm text-muted-foreground">{routeLabel(selected)}</p>
            </div>

            <div className="flex w-full items-center gap-2 rounded-lg bg-secondary/50 px-3 py-2">
              <Fingerprint className="h-4 w-4 flex-shrink-0 text-muted-foreground" />
              <div className="min-w-0">
                <p className="text-[10px] text-muted-foreground">Identity</p>
                <p className="font-mono text-xs text-foreground">{shortId(selected.identityIdHex)}</p>
              </div>
            </div>

            <div className="flex w-full gap-2">
              <button
                onClick={() => onMessage?.(selected)}
                className="flex flex-1 items-center justify-center gap-1.5 rounded-lg bg-primary px-4 py-2 text-xs font-medium text-primary-foreground transition-colors hover:opacity-90"
              >
                <MessageSquare className="h-3.5 w-3.5" />
                Message
              </button>
              <button
                onClick={() => emailContact(profile.handle, profile.ownerEncryption.publicKeyRawBase64)}
                className="flex items-center justify-center gap-1.5 rounded-lg bg-secondary px-4 py-2 text-xs font-medium text-foreground transition-colors hover:bg-secondary/70"
              >
                <Mail className="h-3.5 w-3.5" />
                Email me
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

  if (!onClose) return app

  return (
    <div className="fixed left-1/2 top-1/2 z-40 flex h-[min(640px,calc(100vh-7rem))] w-[min(960px,calc(100vw-3rem))] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-lg border border-border bg-background/96 shadow-2xl backdrop-blur-xl">
      <div className="flex h-12 flex-shrink-0 items-center justify-between border-b border-[var(--window-border)] px-4">
        <div className="flex items-center gap-2">
          <UserPlus className="h-4 w-4 text-primary" />
          <span className="text-sm font-medium text-foreground">Contacts</span>
        </div>
        <button onClick={onClose} className="rounded-md p-2 text-muted-foreground hover:bg-secondary hover:text-foreground" aria-label="Close Contacts">
          <X className="h-4 w-4" />
        </button>
      </div>
      <div className="min-h-0 flex-1">{app}</div>
    </div>
  )
}
