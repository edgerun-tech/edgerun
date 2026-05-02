"use client"

import { useState, useEffect, useCallback } from "react"
import {
  Mail,
  RefreshCw,
  LogOut,
  Send,
  Inbox,
  AlertCircle,
  Circle,
  User,
  Clock,
} from "lucide-react"
import { cn } from "@/lib/utils"

interface Email {
  id: string
  threadId: string
  snippet: string
  from: string
  to: string
  subject: string
  date: string
  unread: boolean
}

interface GmailAppProps {
  className?: string
}

function parseFromField(from: string): { name: string; email: string } {
  const match = from.match(/^(?:"?([^"]+)"?\s*)?<?([^>]+@[^>]+)>?$/)
  if (match) {
    return { name: match[1] || match[2], email: match[2] }
  }
  return { name: from, email: from }
}

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr)
    const now = new Date()
    if (d.toDateString() === now.toDateString()) {
      return d.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit", hour12: false })
    }
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" })
  } catch {
    return dateStr
  }
}

function Avatar({ name, size = "sm" }: { name: string; size?: "sm" | "md" }) {
  const { name: displayName } = parseFromField(name)
  const initials = displayName
    .split(" ")
    .map((n) => n[0])
    .join("")
    .slice(0, 2)
    .toUpperCase()
  const hue = displayName.split("").reduce((a, c) => a + c.charCodeAt(0), 0) % 360
  return (
    <div
      className={cn(
        "flex flex-shrink-0 items-center justify-center rounded-full font-mono font-bold",
        size === "sm" && "h-8 w-8 text-[10px]",
        size === "md" && "h-10 w-10 text-xs",
      )}
      style={{ background: `oklch(0.3 0.1 ${hue})`, color: `oklch(0.85 0.1 ${hue})` }}
    >
      {initials}
    </div>
  )
}

export function GmailApp({ className }: GmailAppProps) {
  const [connected, setConnected] = useState<boolean | null>(null)
  const [email, setEmail] = useState<string>("")
  const [emails, setEmails] = useState<Email[]>([])
  const [selected, setSelected] = useState<Email | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [nextPage, setNextPage] = useState<string | null>(null)

  // Check if already connected on mount
  useEffect(() => {
    const checkConnection = async () => {
      try {
        const res = await fetch("/api/gmail/emails", { method: "GET" })
        if (res.ok) {
          setConnected(true)
          // Get email from cookie via a simple endpoint or use a dedicated one
          const emailCookie = document.cookie
            .split("; ")
            .find((c) => c.startsWith("gmail_email="))
          if (emailCookie) {
            setEmail(decodeURIComponent(emailCookie.split("=")[1]))
          }
        } else {
          setConnected(false)
        }
      } catch {
        setConnected(false)
      }
    }
    checkConnection()
  }, [])

  const connect = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await fetch("/api/gmail/auth")
      const data = await res.json()
      if (data.authUrl) {
        window.location.href = data.authUrl
      } else {
        setError(data.error || "Failed to start OAuth")
      }
    } catch (err) {
      setError(`Connection failed: ${err}`)
    } finally {
      setLoading(false)
    }
  }, [])

  const fetchEmails = useCallback(async (pageToken?: string) => {
    setLoading(true)
    setError(null)
    try {
      const url = new URL("/api/gmail/emails", window.location.origin)
      if (pageToken) url.searchParams.set("pageToken", pageToken)
      const res = await fetch(url.toString())
      const data = await res.json()

      if (res.status === 401 && data.needReauth) {
        setConnected(false)
        setError("Session expired. Please reconnect.")
        return
      }

      if (!res.ok) {
        setError(data.error || "Failed to fetch emails")
        return
      }

      setEmails((prev) => (pageToken ? [...prev, ...data.emails] : data.emails))
      setNextPage(data.nextPageToken || null)
      setConnected(true)
    } catch (err) {
      setError(`Failed to fetch emails: ${err}`)
    } finally {
      setLoading(false)
    }
  }, [])

  const disconnect = useCallback(() => {
    document.cookie = "gmail_access_token=; Max-Age=0; path=/"
    document.cookie = "gmail_refresh_token=; Max-Age=0; path=/"
    document.cookie = "gmail_email=; Max-Age=0; path=/"
    setConnected(false)
    setEmail("")
    setEmails([])
    setSelected(null)
  }, [])

  // Check for OAuth callback success
  useEffect(() => {
    const params = new URLSearchParams(window.location.search)
    if (params.get("gmail_connected") === "true") {
      setConnected(true)
      const emailCookie = document.cookie
        .split("; ")
        .find((c) => c.startsWith("gmail_email="))
      if (emailCookie) {
        setEmail(decodeURIComponent(emailCookie.split("=")[1]))
      }
      // Clean URL
      window.history.replaceState({}, "", "/")
      fetchEmails()
    }
    if (params.get("gmail_error")) {
      setError(`OAuth error: ${params.get("gmail_error")}`)
      window.history.replaceState({}, "", "/")
    }
  }, [fetchEmails])

  // Loading state
  if (connected === null) {
    return (
      <div className="flex h-full items-center justify-center">
        <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    )
  }

  // Not connected - show connect screen
  if (!connected) {
    return (
      <div className={cn("flex h-full flex-col items-center justify-center gap-6 p-8", className)}>
        <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-[#EA4335]/10">
          <Mail className="h-8 w-8 text-[#EA4335]" />
        </div>
        <div className="text-center">
          <h2 className="text-lg font-semibold text-foreground">Gmail</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Connect your Google account to read and send emails
          </p>
        </div>
        {error && (
          <div className="flex items-center gap-2 rounded-lg bg-destructive/10 px-3 py-2 text-xs text-destructive">
            <AlertCircle className="h-3.5 w-3.5 flex-shrink-0" />
            {error}
          </div>
        )}
        <button
          onClick={connect}
          disabled={loading}
          className="flex items-center gap-2 rounded-lg bg-[#EA4335] px-5 py-2.5 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50"
        >
          {loading ? (
            <RefreshCw className="h-4 w-4 animate-spin" />
          ) : (
            <svg className="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
              <path d="M22 12c0-5.52-4.48-10-10-10S2 6.48 2 12c0 4.84 3.44 8.87 8 9.8V15H8v-3h2V9.5C10 7.57 11.57 6 13.5 6H16v3h-2c-.55 0-1 .45-1 1v2h3v3h-3v6.95c5.05-.5 9-4.76 9-9.95z" />
            </svg>
          )}
          Connect with Google
        </button>
      </div>
    )
  }

  // Connected - show email list
  return (
    <div className={cn("flex h-full", className)}>
      {/* Email list sidebar */}
      <div className="flex w-72 flex-shrink-0 flex-col border-r border-[var(--window-border)]">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[var(--window-border)] px-3 py-2.5">
          <div className="flex items-center gap-2 min-w-0">
            <Mail className="h-4 w-4 flex-shrink-0 text-[#EA4335]" />
            <span className="truncate text-xs font-medium text-foreground">{email || "Gmail"}</span>
          </div>
          <button
            onClick={disconnect}
            className="rounded p-1 text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
            title="Disconnect"
          >
            <LogOut className="h-3.5 w-3.5" />
          </button>
        </div>

        {/* Refresh */}
        <div className="flex items-center gap-2 border-b border-[var(--window-border)] px-3 py-2">
          <button
            onClick={() => fetchEmails()}
            disabled={loading}
            className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground disabled:opacity-50"
          >
            <RefreshCw className={cn("h-3 w-3", loading && "animate-spin")} />
            {loading ? "Loading..." : "Refresh"}
          </button>
        </div>

        {/* Email list */}
        <div className="flex-1 overflow-y-auto">
          {emails.length === 0 && !loading && (
            <p className="p-4 text-center text-xs text-muted-foreground">No emails yet</p>
          )}
          {emails.map((email) => {
            const { name } = parseFromField(email.from)
            return (
              <button
                key={email.id}
                onClick={() => setSelected(email)}
                className={cn(
                  "flex w-full gap-2.5 border-b border-[var(--window-border)]/50 px-3 py-2.5 text-left transition-colors hover:bg-secondary/50",
                  selected?.id === email.id && "bg-secondary",
                  email.unread && "bg-primary/5"
                )}
              >
                <Avatar name={email.from} size="sm" />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-1">
                    {email.unread && (
                      <Circle className="h-2 w-2 flex-shrink-0 fill-[#EA4335] text-[#EA4335]" />
                    )}
                    <span className={cn("truncate text-xs", email.unread ? "font-semibold" : "font-medium", "text-foreground")}>
                      {name}
                    </span>
                  </div>
                  <p className={cn("truncate text-xs", email.unread ? "font-medium" : "font-normal", "text-foreground")}>
                    {email.subject}
                  </p>
                  <p className="truncate text-[10px] text-muted-foreground">
                    {email.snippet.replace(/</g, "&lt;").replace(/>/g, "&gt;").slice(0, 60)}
                  </p>
                </div>
                <span className="flex-shrink-0 self-start pt-0.5 text-[10px] text-muted-foreground">
                  {formatDate(email.date)}
                </span>
              </button>
            )
          })}
          {nextPage && (
            <button
              onClick={() => fetchEmails(nextPage)}
              disabled={loading}
              className="w-full py-2 text-center text-xs text-primary hover:underline disabled:opacity-50"
            >
              Load more
            </button>
          )}
        </div>
      </div>

      {/* Email detail */}
      <div className="flex flex-1 flex-col">
        {selected ? (
          <>
            <div className="border-b border-[var(--window-border)] px-4 py-3">
              <h2 className="text-sm font-semibold text-foreground">{selected.subject}</h2>
              <div className="mt-2 flex items-center gap-2">
                <Avatar name={selected.from} size="sm" />
                <div className="min-w-0 flex-1">
                  <p className="text-xs font-medium text-foreground">
                    {parseFromField(selected.from).name}
                  </p>
                  <p className="text-[10px] text-muted-foreground">
                    {parseFromField(selected.from).email}
                  </p>
                </div>
                <span className="flex items-center gap-1 text-[10px] text-muted-foreground">
                  <Clock className="h-3 w-3" />
                  {formatDate(selected.date)}
                </span>
              </div>
            </div>
            <div className="flex-1 overflow-y-auto p-4">
              <p className="whitespace-pre-wrap text-xs leading-relaxed text-foreground">
                {selected.snippet}
              </p>
            </div>
          </>
        ) : (
          <div className="flex flex-1 items-center justify-center">
            <div className="text-center">
              <Inbox className="mx-auto h-10 w-10 text-muted-foreground/30" />
              <p className="mt-2 text-sm text-muted-foreground">Select an email to read</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
