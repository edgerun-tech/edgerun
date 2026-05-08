"use client"

import { useEffect, useMemo, useRef, useState } from "react"
import { Lock, Send } from "lucide-react"
import { cn } from "@/lib/utils"
import { useAuth, type ContactRecord, type LocalQueuedMessage, type UnlockedProfileContainer } from "@/hooks/use-auth"

type MessagesAppProps = {
  onClose: () => void
  initialRecipientId?: string
}

type OpenedMessage = {
  message: LocalQueuedMessage
  text?: string
}

function Avatar({ name, size = "sm" }: { name: string; size?: "xs" | "sm" }) {
  const initials = name.split(" ").map((part) => part[0]).join("").slice(0, 2).toUpperCase() || "ID"
  const hue = name.split("").reduce((acc, char) => acc + char.charCodeAt(0), 0) % 360
  return (
    <div
      className={cn(
        "flex flex-shrink-0 items-center justify-center rounded-full font-mono font-bold",
        size === "xs" && "h-6 w-6 text-[9px]",
        size === "sm" && "h-8 w-8 text-xs",
      )}
      style={{ background: `oklch(0.25 0.1 ${hue})`, color: `oklch(0.85 0.1 ${hue})` }}
    >
      {initials}
    </div>
  )
}

function formatTime(date: Date) {
  return date.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit", hour12: false })
}

function senderName(profile: UnlockedProfileContainer, message: LocalQueuedMessage) {
  if (message.fromId === profile.ownerEncryption.identityIdHex) return "You"
  return profile.contacts.find((contact) => contact.identityIdHex === message.fromId)?.label ?? "Contact"
}

export function MessagesApp({ onClose, initialRecipientId }: MessagesAppProps) {
  const auth = useAuth()
  const profile = auth.unlockedProfile as UnlockedProfileContainer | null
  const [activeContactId, setActiveContactId] = useState(initialRecipientId ?? "")
  const [subject, setSubject] = useState("")
  const [input, setInput] = useState("")
  const [password, setPassword] = useState("")
  const [openedMessages, setOpenedMessages] = useState<Record<string, string>>({})
  const bottomRef = useRef<HTMLDivElement>(null)

  const contacts = useMemo<ContactRecord[]>(() => {
    if (!profile) return []
    return profile.contacts.filter((contact) => contact.identityIdHex !== profile.ownerEncryption.identityIdHex)
  }, [profile])

  const activeContact = contacts.find((contact) => contact.identityIdHex === activeContactId) ?? contacts[0] ?? null

  useEffect(() => {
    if (!activeContactId && activeContact) setActiveContactId(activeContact.identityIdHex)
  }, [activeContact, activeContactId])

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" })
  }, [activeContactId, auth.localMessages.length, openedMessages])

  if (!profile) return null

  const threadMessages: OpenedMessage[] = auth.localMessages
    .filter((message) => !activeContact || message.fromId === activeContact.identityIdHex || message.toId === activeContact.identityIdHex)
    .map((message) => ({ message, text: openedMessages[message.id] }))

  const canSend = Boolean(password && input.trim() && activeContact)

  async function openMessage(messageId: string) {
    const opened = await auth.openLocalMessage(messageId)
    if (opened !== null) setOpenedMessages((current) => ({ ...current, [messageId]: opened }))
  }

  async function sendMessage() {
    if (!canSend || !activeContact) return
    await auth.createSealedContainer({
      kind: "message",
      label: subject.trim() || "Message",
      recipientId: activeContact.identityIdHex,
      plaintext: input,
      password,
    })
    setInput("")
    setSubject("")
  }

  return (
    <div className="fixed left-1/2 top-1/2 z-40 flex h-[min(680px,calc(100vh-6rem))] w-[min(980px,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-xl border border-border bg-background/96 shadow-2xl backdrop-blur-xl">
      <div className="flex h-12 flex-shrink-0 items-center justify-between border-b border-[var(--window-border)] px-4 sm:h-14 sm:px-5">
        <div className="flex items-center gap-2">
          <Send className="h-4 w-4 text-primary" />
          <span className="text-sm font-medium text-foreground">Messages</span>
        </div>
        <button onClick={onClose} className="rounded-md border border-border bg-background/80 px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-secondary hover:text-foreground" aria-label="Close Messages">
          Close
        </button>
      </div>

      <div className="flex min-h-0 flex-1 flex-col md:flex-row">
        <div className="flex h-32 flex-shrink-0 flex-col border-b border-[var(--window-border)] py-2 md:h-auto md:w-48 md:border-b-0 md:border-r">
          <div className="mb-1 px-3">
            <p className="text-[9px] font-semibold uppercase tracking-widest text-muted-foreground/70">Direct</p>
          </div>
          {contacts.length === 0 && (
            <p className="px-3 py-4 text-center text-xs text-muted-foreground">Add a contact first.</p>
          )}
          {contacts.map((contact) => {
            const unread = auth.localMessages.filter((message) => message.fromId === contact.identityIdHex && openedMessages[message.id] === undefined).length
            return (
              <button
                key={contact.identityIdHex}
                onClick={() => setActiveContactId(contact.identityIdHex)}
                className={cn(
                  "flex items-center gap-2 px-3 py-1.5 text-left transition-colors",
                  contact.identityIdHex === activeContact?.identityIdHex ? "bg-secondary text-foreground" : "text-muted-foreground hover:text-foreground",
                )}
              >
                <Avatar name={contact.label} size="xs" />
                <span className="flex-1 truncate text-xs">{contact.label.split(" ")[0]}</span>
                {unread > 0 && (
                  <span className="flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[9px] font-bold text-primary-foreground">
                    {unread}
                  </span>
                )}
              </button>
            )
          })}
          <div className="mt-auto border-t border-[var(--window-border)] px-3 pt-3">
            <div className="flex items-center gap-1 text-[10px] text-muted-foreground/60">
              <Lock className="h-3 w-3" />
              <span>Sealed locally</span>
            </div>
          </div>
        </div>

        <div className="flex min-w-0 flex-1 flex-col">
          <div className="flex h-10 flex-shrink-0 items-center gap-2 border-b border-[var(--window-border)] px-4">
            {activeContact ? <Avatar name={activeContact.label} size="xs" /> : null}
            <span className="text-sm font-medium text-foreground">{activeContact?.label ?? "No contact selected"}</span>
            <span className="ml-auto text-[10px] text-muted-foreground">{threadMessages.length} messages</span>
          </div>

          <div className="flex-1 space-y-3 overflow-y-auto p-4">
            {threadMessages.length === 0 && (
              <p className="pt-8 text-center text-xs text-muted-foreground">No messages yet. Send one to create a sealed local queue item.</p>
            )}
            {threadMessages.map(({ message, text }, index) => {
              const self = message.fromId === profile.ownerEncryption.identityIdHex
              const name = senderName(profile, message)
              const previous = threadMessages[index - 1]?.message
              const previousName = previous ? senderName(profile, previous) : ""
              const showSender = index === 0 || previousName !== name
              return (
                <div key={message.id} className={cn("flex gap-2.5", self && "flex-row-reverse")}>
                  {showSender && !self && <Avatar name={name} size="sm" />}
                  {!showSender && !self && <div className="w-8 flex-shrink-0" />}
                  <div className={cn("flex max-w-[75%] flex-col gap-0.5", self && "items-end")}>
                    {showSender && (
                      <span className={cn("text-[10px] text-muted-foreground", self && "text-right")}>
                        {self ? "You" : name.split(" ")[0]} · {formatTime(new Date(message.createdAtIso))}
                      </span>
                    )}
                    <div
                      className={cn(
                        "rounded-xl px-3 py-2 text-sm leading-relaxed",
                        self
                          ? "rounded-tr-sm bg-primary text-primary-foreground"
                          : "rounded-tl-sm bg-secondary text-foreground",
                      )}
                    >
                      {text ?? (
                        <button onClick={() => openMessage(message.id)} className="text-xs font-medium underline underline-offset-2">
                          Open sealed message
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              )
            })}
            <div ref={bottomRef} />
          </div>

          <div className="border-t border-[var(--window-border)] p-3 sm:p-4">
            <div className="mb-2 grid gap-2 sm:grid-cols-[1fr_180px]">
              <input
                value={subject}
                onChange={(event) => setSubject(event.target.value)}
                placeholder="Subject"
                className="h-8 rounded-md bg-secondary px-3 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
              />
              <input
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                type="password"
                placeholder="Profile password"
                className="h-8 rounded-md bg-secondary px-3 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </div>
            <div className="flex items-center gap-2 rounded-lg bg-secondary px-3 py-2">
              <input
                value={input}
                onChange={(event) => setInput(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && !event.shiftKey) {
                    event.preventDefault()
                    sendMessage()
                  }
                }}
                placeholder={activeContact ? `Message ${activeContact.label.split(" ")[0]}...` : "Add a contact first..."}
                className="flex-1 bg-transparent text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
              />
              <button
                onClick={sendMessage}
                disabled={!canSend}
                className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground transition-opacity disabled:opacity-40"
                aria-label="Send message"
              >
                <Send className="h-3.5 w-3.5" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
