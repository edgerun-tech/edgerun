"use client"

import { useEffect, useMemo, useRef, useState } from "react"
import { Lock, Send } from "lucide-react"
import { cn } from "@/lib/utils"
import { formatTime } from "@/lib/format"
import { AppAvatar } from "@/components/ui/app-avatar"
import { useAuth, type ContactRecord, type LocalQueuedMessage, type UnlockedProfileContainer } from "@/hooks/use-auth"

type MessagesAppProps = {
  initialRecipientId?: string
}

type OpenedMessage = {
  message: LocalQueuedMessage
  text?: string
}

function senderName(profile: UnlockedProfileContainer, message: LocalQueuedMessage) {
  if (message.fromId === profile.ownerEncryption.identityIdHex) return "You"
  return profile.contacts.find((contact) => contact.identityIdHex === message.fromId)?.label ?? "Contact"
}

export function MessagesApp({ initialRecipientId }: MessagesAppProps) {
  const auth = useAuth()
  const profile = auth.unlockedProfile as UnlockedProfileContainer | null
  const [activeContactId, setActiveContactId] = useState(initialRecipientId ?? "")
  const [subject, setSubject] = useState("")
  const [input, setInput] = useState("")
  const [password, setPassword] = useState("")
  const [authorizedPassword, setAuthorizedPassword] = useState("")
  const [openedMessages, setOpenedMessages] = useState<Record<string, string>>({})
  const bottomRef = useRef<HTMLDivElement>(null)

  const contacts = useMemo<ContactRecord[]>(() => {
    if (!profile) return []
    return profile.contacts.filter((contact) => contact.identityIdHex !== profile.ownerEncryption.identityIdHex)
  }, [profile])

  const activeContact = contacts.find((contact) => contact.identityIdHex === activeContactId) ?? contacts[0] ?? null

  useEffect(() => {
    if (initialRecipientId) setActiveContactId(initialRecipientId)
  }, [initialRecipientId])

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

  const unreadByContactId = auth.localMessages.reduce<Record<string, number>>((acc, message) => {
    if (openedMessages[message.id] !== undefined) return acc
    acc[message.fromId] = (acc[message.fromId] ?? 0) + 1
    return acc
  }, {})
  const canAuthorize = Boolean(password)
  const canSend = Boolean((authorizedPassword || password) && input.trim() && activeContact)

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
      password: authorizedPassword || password,
    })
    if (!authorizedPassword && password) setAuthorizedPassword(password)
    setPassword("")
    setInput("")
    setSubject("")
  }

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-background/96">
      <div className="flex min-h-0 flex-1 flex-col md:flex-row">
        <div className="flex h-32 flex-shrink-0 flex-col border-b border-[var(--window-border)] py-2 md:h-auto md:w-48 md:border-b-0 md:border-r">
          <div className="mb-1 px-3">
            <p className="text-[9px] font-semibold uppercase tracking-widest text-muted-foreground/70">Direct</p>
          </div>
          {contacts.length === 0 && (
            <p className="px-3 py-4 text-center text-xs text-muted-foreground">Add a contact first.</p>
          )}
          {contacts.map((contact) => {
            const unread = unreadByContactId[contact.identityIdHex] ?? 0
            return (
              <button
                key={contact.identityIdHex}
                onClick={() => setActiveContactId(contact.identityIdHex)}
                className={cn(
                  "flex items-center gap-2 px-3 py-1.5 text-left transition-colors",
                  contact.identityIdHex === activeContact?.identityIdHex ? "bg-secondary text-foreground" : "text-muted-foreground hover:text-foreground",
                )}
              >
                <AppAvatar name={contact.label} size="xs" />
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
            {activeContact ? <AppAvatar name={activeContact.label} size="xs" /> : null}
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
                  {showSender && !self && <AppAvatar name={name} size="sm" />}
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
                placeholder={authorizedPassword ? "Authorized" : "Profile password"}
                className="h-8 rounded-md bg-secondary px-3 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
              />
            </div>
            {!authorizedPassword && (
              <button
                onClick={() => {
                  if (canAuthorize) {
                    setAuthorizedPassword(password)
                    setPassword("")
                  }
                }}
                disabled={!canAuthorize}
                className="mb-2 h-7 rounded-md border border-border bg-secondary/35 px-2 text-[11px] font-medium text-muted-foreground hover:text-foreground disabled:opacity-40"
              >
                Authorize sending for this session
              </button>
            )}
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
