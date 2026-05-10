"use client"

import { useEffect, useMemo, useState } from "react"
import { ChevronRight, Copy, Fingerprint, Mail, MessageSquare, Plus, Save, Search, User, X } from "lucide-react"
import { cn } from "@/lib/utils"
import { useAuth, type ContactRecord, type UnlockedProfileContainer } from "@/hooks/use-auth"
import { shortHex } from "@/lib/format"

type ContactsAppProps = {
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
  if (contact.routeHint === "address-book") return "Address book"
  return contact.routeHint === "local-profile" ? "Local profile" : "Network contact"
}
function shortId(value: string) {
  return shortHex(value, 12, 6)
}

export function ContactsApp({ onMessage }: ContactsAppProps) {
  const auth = useAuth()
  const profile = auth.unlockedProfile as UnlockedProfileContainer | null
  const [query, setQuery] = useState("")
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [showAdd, setShowAdd] = useState(false)
  const [password, setPassword] = useState("")
  const [contactLabel, setContactLabel] = useState("")
  const [contactPublicKey, setContactPublicKey] = useState("")
  const [localProfileId, setLocalProfileId] = useState("")
  const [editPassword, setEditPassword] = useState("")
  const [editLabel, setEditLabel] = useState("")
  const [editEmail, setEditEmail] = useState("")
  const [editPhone, setEditPhone] = useState("")
  const [editNodeIds, setEditNodeIds] = useState("")
  const [editNotes, setEditNotes] = useState("")

  const contacts = useMemo(() => {
    if (!profile) return []
    const normalizedQuery = query.trim().toLowerCase()
    return profile.contacts
      .filter((contact) => contact.identityIdHex !== profile.ownerEncryption.identityIdHex)
      .filter((contact) => (
        !normalizedQuery
        || contact.label.toLowerCase().includes(normalizedQuery)
        || contact.email?.toLowerCase().includes(normalizedQuery)
        || contact.phone?.toLowerCase().includes(normalizedQuery)
        || contact.knownNodeIds?.some((nodeId) => nodeId.toLowerCase().includes(normalizedQuery))
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

  const selected = contacts.find((contact) => contact.identityIdHex === selectedId) ?? contacts[0] ?? null
  const selectedCanMessage = Boolean(selected?.publicKeyRawBase64)
  const canAddLocal = Boolean(password && localProfileOptions.length > 0)
  const canAddExternal = Boolean(password && contactPublicKey.trim())
  const canSaveSelected = Boolean(selected && editPassword && editLabel.trim())
  const selectedContactId = selected?.id
  const selectedLabel = selected?.label ?? ""
  const selectedEmail = selected?.email ?? ""
  const selectedPhone = selected?.phone ?? ""
  const selectedNodeIds = selected?.knownNodeIds?.join("\n") ?? ""
  const selectedNotes = selected?.notes ?? ""

  useEffect(() => {
    if (!selectedContactId) return
    setEditLabel(selectedLabel)
    setEditEmail(selectedEmail)
    setEditPhone(selectedPhone)
    setEditNodeIds(selectedNodeIds)
    setEditNotes(selectedNotes)
  }, [selectedContactId, selectedLabel, selectedEmail, selectedPhone, selectedNotes, selectedNodeIds])

  if (!profile) return null

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

  async function saveSelectedContact() {
    if (!selected) return
    const ok = await auth.saveContact({
      ...selected,
      label: editLabel,
      email: editEmail,
      phone: editPhone,
      knownNodeIds: editNodeIds.split(/\r?\n|,/).map((item) => item.trim()).filter(Boolean),
      notes: editNotes,
      password: editPassword,
    })
    if (ok) setEditPassword("")
  }

  async function copyMyContact() {
    if (!profile) return
    await navigator.clipboard.writeText(contactCard(profile.handle, profile.ownerEncryption.publicKeyRawBase64))
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-background text-foreground md:flex-row">
      <div className="flex h-[42%] min-h-0 flex-shrink-0 flex-col border-b border-[var(--window-border)] md:h-auto md:w-72 md:border-b-0 md:border-r">
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
                "flex w-full items-center gap-2 px-2.5 py-1.5 text-left transition-colors hover:bg-secondary/70",
                selected?.identityIdHex === contact.identityIdHex && "bg-secondary",
              )}
            >
              <Avatar name={contact.label} size="sm" />
              <div className="min-w-0 flex-1">
                <p className="truncate text-xs font-medium text-foreground">{contact.label}</p>
                <p className="truncate text-[10px] text-muted-foreground">
                  {contact.email || contact.knownNodeIds?.[0] || routeLabel(contact)}
                </p>
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

      <div className="flex min-h-0 flex-1 flex-col items-center justify-center overflow-auto p-4 sm:p-6">
        {selected ? (
          <div className="grid w-full max-w-2xl gap-4 lg:grid-cols-[220px_1fr]">
            <div className="flex flex-col items-center gap-4 rounded-lg border border-[var(--window-border)] bg-card/55 p-4">
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
            {selected.knownNodeIds?.length ? (
              <div className="w-full rounded-lg bg-secondary/50 px-3 py-2">
                <p className="mb-1 text-[10px] text-muted-foreground">Known node IDs</p>
                <div className="space-y-1">
                  {selected.knownNodeIds.map((nodeId) => <p key={nodeId} className="truncate font-mono text-[10px] text-foreground">{nodeId}</p>)}
                </div>
              </div>
            ) : null}

            <div className="flex w-full gap-2">
              <button
                onClick={() => onMessage?.(selected)}
                disabled={!selectedCanMessage}
                className="flex flex-1 items-center justify-center gap-1.5 rounded-lg bg-primary px-4 py-2 text-xs font-medium text-primary-foreground transition-colors hover:opacity-90 disabled:opacity-40"
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

            {!selectedCanMessage && <p className="text-center text-[11px] text-muted-foreground">Add an Edgerun public key before encrypted messaging.</p>}
            </div>

            <div className="space-y-2 rounded-lg border border-[var(--window-border)] bg-card/55 p-4">
              <div className="mb-3 text-xs font-semibold text-foreground">Edit contact</div>
              <input value={editLabel} onChange={(event) => setEditLabel(event.target.value)} placeholder="Name" className="h-8 w-full rounded-md bg-secondary px-2 text-xs text-foreground outline-none focus:ring-1 focus:ring-primary" />
              <input value={editEmail} onChange={(event) => setEditEmail(event.target.value)} placeholder="Email" className="h-8 w-full rounded-md bg-secondary px-2 text-xs text-foreground outline-none focus:ring-1 focus:ring-primary" />
              <input value={editPhone} onChange={(event) => setEditPhone(event.target.value)} placeholder="Phone" className="h-8 w-full rounded-md bg-secondary px-2 text-xs text-foreground outline-none focus:ring-1 focus:ring-primary" />
              <textarea value={editNodeIds} onChange={(event) => setEditNodeIds(event.target.value)} placeholder="Known node IDs, one per line" className="min-h-20 w-full rounded-md bg-secondary px-2 py-2 font-mono text-[11px] text-foreground outline-none focus:ring-1 focus:ring-primary" />
              <textarea value={editNotes} onChange={(event) => setEditNotes(event.target.value)} placeholder="Notes" className="min-h-16 w-full rounded-md bg-secondary px-2 py-2 text-xs text-foreground outline-none focus:ring-1 focus:ring-primary" />
              <div className="flex gap-2">
                <input value={editPassword} onChange={(event) => setEditPassword(event.target.value)} type="password" placeholder="Profile password" className="h-8 min-w-0 flex-1 rounded-md bg-secondary px-2 text-xs text-foreground outline-none focus:ring-1 focus:ring-primary" />
                <button onClick={saveSelectedContact} disabled={!canSaveSelected || auth.isLoading} className="flex h-8 items-center gap-1.5 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground disabled:opacity-40">
                  <Save className="h-3.5 w-3.5" />
                  Save
                </button>
              </div>
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
