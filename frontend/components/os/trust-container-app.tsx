"use client"

import { useMemo, useState } from "react"
import { Contact, Copy, Inbox, KeyRound, Send, ShieldCheck, UserPlus, X } from "lucide-react"
import { useAuth, type LocalQueuedMessage, type UnlockedProfileContainer } from "@/hooks/use-auth"

function shortId(value: string, chars = 10) {
  if (!value) return "missing"
  return `${value.slice(0, chars)}...${value.slice(-4)}`
}

function EmptyState({ title, detail }: { title: string; detail: string }) {
  return (
    <div className="rounded-lg border border-dashed border-border bg-background/40 p-5">
      <div className="text-sm font-medium text-foreground">{title}</div>
      <div className="mt-1 text-xs text-muted-foreground">{detail}</div>
    </div>
  )
}

function Stat({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-lg border border-border bg-card p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 text-lg font-semibold text-foreground">{value}</div>
    </div>
  )
}

function MessageRow({ message, plaintext, onOpen }: { message: LocalQueuedMessage; plaintext?: string; onOpen: () => void }) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="flex items-start gap-3">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-primary/12 text-primary">
          <Inbox className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-semibold text-foreground">{message.sealedContainer.label}</div>
          <div className="mt-1 text-xs text-muted-foreground">{new Date(message.createdAtIso).toLocaleString()}</div>
        </div>
        {plaintext === undefined && (
          <button onClick={onOpen} className="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:opacity-90">
            Open
          </button>
        )}
      </div>
      {plaintext !== undefined && (
        <div className="mt-3 whitespace-pre-wrap rounded-md border border-border bg-background/70 p-3 text-sm text-foreground">{plaintext}</div>
      )}
      <div className="mt-3 text-[11px] text-muted-foreground">From {shortId(message.fromId)}</div>
    </div>
  )
}

export function TrustContainerApp({ onClose }: { onClose: () => void }) {
  const auth = useAuth()
  const profile = auth.unlockedProfile as UnlockedProfileContainer | null
  const [recipientId, setRecipientId] = useState("")
  const [messageLabel, setMessageLabel] = useState("")
  const [messageBody, setMessageBody] = useState("")
  const [contactLabel, setContactLabel] = useState("")
  const [contactPublicKey, setContactPublicKey] = useState("")
  const [localProfileId, setLocalProfileId] = useState("")
  const [nodeLabel, setNodeLabel] = useState("")
  const [password, setPassword] = useState("")
  const [openedMessages, setOpenedMessages] = useState<Record<string, string>>({})

  const localProfileOptions = useMemo(() => {
    if (!profile) return []
    return auth.profileSummaries.filter((summary) => (
      summary.encryptionPublicKeyHint
      && summary.encryptionIdHint !== profile.ownerEncryption.identityIdHex
      && !profile.contacts.some((contact) => contact.identityIdHex === summary.encryptionIdHint)
    ))
  }, [auth.profileSummaries, profile])

  const contactOptions = useMemo(() => {
    if (!profile) return []
    return profile.contacts.filter((contact) => contact.identityIdHex !== profile.ownerEncryption.identityIdHex)
  }, [profile])

  if (!profile) return null

  const selectedRecipient = recipientId || contactOptions[0]?.identityIdHex || profile.contacts[0]?.identityIdHex || ""
  const canSend = Boolean(password && messageBody.trim() && selectedRecipient)
  const canAddExternal = Boolean(password && contactPublicKey.trim())
  const canAddLocal = Boolean(password && localProfileOptions.length > 0)

  async function sendMessage() {
    if (!canSend) return
    await auth.createSealedContainer({
      kind: "message",
      label: messageLabel.trim() || "Message",
      recipientId: selectedRecipient,
      plaintext: messageBody,
      password,
    })
    setMessageLabel("")
    setMessageBody("")
    setRecipientId("")
  }

  async function addLocalProfileContact() {
    const selected = localProfileOptions.find((item) => item.profileId === localProfileId) ?? localProfileOptions[0]
    if (!selected) return
    await auth.addContact({
      label: selected.handle,
      publicKeyRawBase64: selected.encryptionPublicKeyHint,
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

  async function createNode() {
    await auth.createNode(nodeLabel, password)
    setNodeLabel("")
  }

  async function copyOwnPublicKey() {
    if (!profile) return
    await navigator.clipboard.writeText(profile.ownerEncryption.publicKeyRawBase64)
  }

  async function openMessage(messageId: string) {
    const opened = await auth.openLocalMessage(messageId)
    if (opened !== null) setOpenedMessages((current) => ({ ...current, [messageId]: opened }))
  }

  return (
    <div className="fixed inset-3 z-40 mx-auto flex max-w-6xl flex-col overflow-hidden rounded-lg border border-border bg-background/96 shadow-2xl backdrop-blur-xl md:inset-x-8 md:inset-y-6">
      <div className="flex h-14 shrink-0 items-center justify-between border-b border-border px-4">
        <div className="flex min-w-0 items-center gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-primary/12 text-primary">
            <ShieldCheck className="h-4 w-4" />
          </div>
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold text-foreground">{profile.handle}</div>
            <div className="text-xs text-muted-foreground">Profile is unlocked on this browser node</div>
          </div>
        </div>
        <button onClick={onClose} className="rounded-md p-2 text-muted-foreground hover:bg-secondary hover:text-foreground" aria-label="Close Trust Container">
          <X className="h-4 w-4" />
        </button>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 overflow-hidden md:grid-cols-[minmax(0,1fr)_360px]">
        <main className="min-h-0 overflow-auto p-4">
          <div className="grid gap-3 sm:grid-cols-3">
            <Stat label="Messages" value={auth.localMessages.length} />
            <Stat label="Contacts" value={Math.max(profile.contacts.length - 1, 0)} />
            <Stat label="Nodes" value={profile.nodes.length} />
          </div>

          <section className="mt-5">
            <div className="mb-3 flex items-center gap-2">
              <Inbox className="h-4 w-4 text-primary" />
              <h2 className="text-sm font-semibold text-foreground">Inbox</h2>
            </div>
            {auth.localMessages.length === 0 ? (
              <EmptyState title="No messages yet" detail="Messages sent to this profile will appear here after you unlock it." />
            ) : (
              <div className="grid gap-3">
                {auth.localMessages.map((message) => (
                  <MessageRow
                    key={message.id}
                    message={message}
                    plaintext={openedMessages[message.id]}
                    onOpen={() => openMessage(message.id)}
                  />
                ))}
              </div>
            )}
          </section>

          <section className="mt-5">
            <div className="mb-3 flex items-center gap-2">
              <Contact className="h-4 w-4 text-primary" />
              <h2 className="text-sm font-semibold text-foreground">Contacts</h2>
            </div>
            {contactOptions.length === 0 ? (
              <EmptyState title="No contacts added" detail="Add another local profile or paste a public key to send messages." />
            ) : (
              <div className="grid gap-3 sm:grid-cols-2">
                {contactOptions.map((contact) => (
                  <div key={contact.identityIdHex} className="rounded-lg border border-border bg-card p-3">
                    <div className="truncate text-sm font-medium text-foreground">{contact.label}</div>
                    <div className="mt-1 text-xs text-muted-foreground">{contact.routeHint === "local-profile" ? "Local profile" : "Mesh contact"}</div>
                  </div>
                ))}
              </div>
            )}
          </section>

          <details className="mt-5 rounded-lg border border-border bg-card">
            <summary className="cursor-pointer px-4 py-3 text-sm font-medium text-foreground">Advanced details</summary>
            <div className="grid gap-2 border-t border-border p-4 text-xs">
              <div className="grid gap-1">
                <span className="text-muted-foreground">Controller</span>
                <span className="break-all font-mono text-foreground">{profile.owner.identityIdHex}</span>
              </div>
              <div className="grid gap-1">
                <span className="text-muted-foreground">Primary node</span>
                <span className="break-all font-mono text-foreground">{profile.browserNode.identityIdHex}</span>
              </div>
              <div className="grid gap-1">
                <span className="text-muted-foreground">Genesis</span>
                <span className="break-all font-mono text-foreground">{profile.eventLog[0]?.eventHash ?? "missing"}</span>
              </div>
              <button onClick={copyOwnPublicKey} className="mt-2 inline-flex w-fit items-center gap-2 rounded-md border border-border bg-secondary/50 px-3 py-1.5 text-xs text-foreground hover:bg-secondary">
                <Copy className="h-3.5 w-3.5" />
                Copy public key
              </button>
            </div>
          </details>
        </main>

        <aside className="min-h-0 overflow-auto border-t border-border bg-card/65 p-4 md:border-l md:border-t-0">
          <label className="grid gap-1 text-xs text-muted-foreground">
            Profile password
            <input value={password} onChange={(event) => setPassword(event.target.value)} type="password" placeholder="Required to save changes" className="rounded-md border border-border bg-background px-3 py-2 text-sm text-foreground outline-none focus:border-primary" />
          </label>

          <section className="mt-5">
            <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground">
              <Send className="h-4 w-4 text-primary" />
              Send message
            </div>
            <div className="grid gap-2">
              <select value={recipientId} onChange={(event) => setRecipientId(event.target.value)} className="rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary">
                {contactOptions.length === 0 ? (
                  <option value="">Add a contact first</option>
                ) : (
                  contactOptions.map((contact) => <option key={contact.identityIdHex} value={contact.identityIdHex}>{contact.label}</option>)
                )}
              </select>
              <input value={messageLabel} onChange={(event) => setMessageLabel(event.target.value)} placeholder="Subject" className="rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary" />
              <textarea value={messageBody} onChange={(event) => setMessageBody(event.target.value)} placeholder="Message" className="min-h-28 rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary" />
              <button disabled={!canSend || auth.isLoading} onClick={sendMessage} className="flex items-center justify-center gap-2 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground disabled:opacity-45">
                <Send className="h-4 w-4" />
                Send
              </button>
            </div>
          </section>

          <section className="mt-6 border-t border-border pt-5">
            <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground">
              <UserPlus className="h-4 w-4 text-primary" />
              Add contact
            </div>
            <div className="grid gap-2">
              {localProfileOptions.length > 0 && (
                <>
                  <select value={localProfileId} onChange={(event) => setLocalProfileId(event.target.value)} className="rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary">
                    {localProfileOptions.map((summary) => <option key={summary.profileId} value={summary.profileId}>{summary.handle}</option>)}
                  </select>
                  <button disabled={!canAddLocal || auth.isLoading} onClick={addLocalProfileContact} className="flex items-center justify-center gap-2 rounded-md border border-border bg-secondary/50 px-3 py-2 text-sm font-medium text-foreground hover:bg-secondary disabled:opacity-45">
                    Add local profile
                  </button>
                </>
              )}
              <input value={contactLabel} onChange={(event) => setContactLabel(event.target.value)} placeholder="Contact name" className="rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary" />
              <textarea value={contactPublicKey} onChange={(event) => setContactPublicKey(event.target.value)} placeholder="Public encryption key" className="min-h-20 rounded-md border border-border bg-background px-3 py-2 text-xs outline-none focus:border-primary" />
              <button disabled={!canAddExternal || auth.isLoading} onClick={addExternalContact} className="flex items-center justify-center gap-2 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground disabled:opacity-45">
                <UserPlus className="h-4 w-4" />
                Add contact
              </button>
            </div>
          </section>

          <details className="mt-6 border-t border-border pt-5">
            <summary className="cursor-pointer text-sm font-semibold text-foreground">Node tools</summary>
            <div className="mt-3 grid gap-2">
              <input value={nodeLabel} onChange={(event) => setNodeLabel(event.target.value)} placeholder="Node label" className="rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary" />
              <button disabled={!password || auth.isLoading} onClick={createNode} className="flex items-center justify-center gap-2 rounded-md border border-border bg-secondary/50 px-3 py-2 text-sm font-medium text-foreground hover:bg-secondary disabled:opacity-45">
                <KeyRound className="h-4 w-4" />
                Add node
              </button>
            </div>
          </details>
        </aside>
      </div>
    </div>
  )
}
