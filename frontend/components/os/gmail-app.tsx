"use client"

import { useState, useEffect, useCallback } from "react"
import {
  RefreshCw,
  LogOut,
  Inbox,
  AlertCircle,
  Circle,
  Clock,
  ShieldCheck,
  ExternalLink,
  CheckCircle2,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { useAuth, type GmailProfileSecret } from "@/hooks/use-auth"
import { deleteAppSession, storeAppSession } from "@/platform/runtime/app-session-broker"

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

type PendingGmailSecret = Omit<GmailProfileSecret, "appId" | "kind" | "updatedAtIso">
const PENDING_GMAIL_STORAGE_KEY = "edgerun:oauth-pending:gmail"

function readCookie(name: string): string | null {
  const prefix = `${name}=`
  return document.cookie.split("; ").find((cookie) => cookie.startsWith(prefix))?.slice(prefix.length) ?? null
}

function deleteCookie(name: string) {
  document.cookie = `${name}=; Max-Age=0; path=/`
  if (name === "gmail_profile_pending") sessionStorage.removeItem(PENDING_GMAIL_STORAGE_KEY)
}

function readPendingGmailSecret(): PendingGmailSecret | null {
  const stored = sessionStorage.getItem(PENDING_GMAIL_STORAGE_KEY)
  if (stored) {
    try {
      const parsed = JSON.parse(stored) as PendingGmailSecret
      return parsed.accessToken && parsed.expiresAtIso ? { ...parsed, scopes: parsed.scopes ?? [] } : null
    } catch {
      sessionStorage.removeItem(PENDING_GMAIL_STORAGE_KEY)
    }
  }
  const raw = readCookie("gmail_profile_pending")
  if (!raw) return null
  try {
    const parsed = JSON.parse(atob(raw.replace(/-/g, "+").replace(/_/g, "/"))) as PendingGmailSecret
    return parsed.accessToken && parsed.expiresAtIso ? { ...parsed, scopes: parsed.scopes ?? [] } : null
  } catch {
    deleteCookie("gmail_profile_pending")
    return null
  }
}

async function restoreGmailSession(secret: PendingGmailSecret, profileId?: string): Promise<boolean> {
  return storeAppSession({
    appId: "gmail",
    accessToken: secret.accessToken,
    expiresAtIso: secret.expiresAtIso,
    profileId,
    grantId: "gmail.readonly",
  })
}

function GoogleMark({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" />
      <path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" />
      <path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" />
      <path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" />
    </svg>
  )
}

function parseFromField(from: string): { name: string; email: string } {
  const match = from.match(/^(?:"?([^"]+)"?\s*)?<?([^>]+@[^>]+)>?$/)
  if (match) return { name: match[1] || match[2], email: match[2] }
  return { name: from, email: from }
}

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr)
    const now = new Date()
    if (d.toDateString() === now.toDateString()) return d.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit", hour12: false })
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" })
  } catch {
    return dateStr
  }
}

function Avatar({ name, size = "sm" }: { name: string; size?: "sm" | "md" }) {
  const { name: displayName } = parseFromField(name)
  const initials = displayName.split(" ").map((n) => n[0]).join("").slice(0, 2).toUpperCase()
  const hue = displayName.split("").reduce((a, c) => a + c.charCodeAt(0), 0) % 360
  return (
    <div
      className={cn("flex flex-shrink-0 items-center justify-center rounded-full font-mono font-bold", size === "sm" && "h-8 w-8 text-[10px]", size === "md" && "h-10 w-10 text-xs")}
      style={{ background: `oklch(0.3 0.1 ${hue})`, color: `oklch(0.85 0.1 ${hue})` }}
    >
      {initials}
    </div>
  )
}

export function GmailApp({ className }: GmailAppProps) {
  const auth = useAuth()
  const savedSecret = auth.unlockedProfile?.appSecrets.find((secret) => secret.appId === "gmail")
  const profileId = auth.unlockedProfile?.ownerEncryption.identityIdHex
  const [connected, setConnected] = useState<boolean | null>(null)
  const [email, setEmail] = useState<string>("")
  const [emails, setEmails] = useState<Email[]>([])
  const [selected, setSelected] = useState<Email | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [nextPage, setNextPage] = useState<string | null>(null)
  const [pendingSecret, setPendingSecret] = useState<PendingGmailSecret | null>(null)
  const [profilePassword, setProfilePassword] = useState("")

  useEffect(() => {
    const checkConnection = async () => {
      try {
        const res = await fetch("/api/gmail/emails", { method: "GET" })
        if (res.ok) {
          setConnected(true)
          const emailCookie = document.cookie.split("; ").find((c) => c.startsWith("gmail_email="))
          if (emailCookie) setEmail(decodeURIComponent(emailCookie.split("=")[1]))
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
      if (data.authUrl) window.location.href = data.authUrl
      else setError(data.error || "Failed to start Google connection")
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
        setError("Session expired. Please reconnect Google Mail.")
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

  useEffect(() => {
    const pending = readPendingGmailSecret()
    if (pending) {
      setPendingSecret(pending)
      setEmail(pending.email)
    }
  }, [])

  useEffect(() => {
    if (connected !== false || !savedSecret) return
    let cancelled = false
    async function restore() {
      if (!savedSecret) return
      setLoading(true)
      setError(null)
      try {
        const secret = savedSecret
        const ok = await restoreGmailSession(secret, profileId)
        if (cancelled) return
        if (ok) {
          setEmail(secret.email)
          setConnected(true)
          await fetchEmails()
        }
      } catch (err) {
        if (!cancelled) setError(`Saved Gmail session restore failed: ${err}`)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void restore()
    return () => {
      cancelled = true
    }
  }, [connected, fetchEmails, profileId, savedSecret])

  const savePendingSecret = useCallback(async () => {
    if (!pendingSecret || !profilePassword) return
    const ok = await auth.saveGmailSecret({ password: profilePassword, secret: pendingSecret })
    if (ok) {
      await restoreGmailSession(pendingSecret, profileId)
      deleteCookie("gmail_profile_pending")
      setPendingSecret(null)
      setProfilePassword("")
      setConnected(true)
      setEmail(pendingSecret.email)
    }
  }, [auth, pendingSecret, profileId, profilePassword])

  const removeSavedSecret = useCallback(async () => {
    if (!profilePassword) return
    const ok = await auth.removeGmailSecret(profilePassword)
    if (ok) setProfilePassword("")
  }, [auth, profilePassword])

  const disconnect = useCallback(() => {
    document.cookie = "gmail_access_token=; Max-Age=0; path=/"
    document.cookie = "gmail_refresh_token=; Max-Age=0; path=/"
    document.cookie = "gmail_email=; Max-Age=0; path=/"
    deleteCookie("gmail_profile_pending")
    void deleteAppSession("gmail")
    void fetch("/api/gmail/session", { method: "DELETE" })
    setConnected(false)
    setEmail("")
    setEmails([])
    setSelected(null)
    setPendingSecret(null)
  }, [])

  useEffect(() => {
    const params = new URLSearchParams(window.location.search)
    if (params.get("gmail_connected") === "true") {
      setConnected(true)
      const pending = readPendingGmailSecret()
      if (pending) {
        setPendingSecret(pending)
        setEmail(pending.email)
        void restoreGmailSession(pending, profileId).then(() => fetchEmails())
      } else {
        fetchEmails()
      }
      const emailCookie = document.cookie.split("; ").find((c) => c.startsWith("gmail_email="))
      if (emailCookie) setEmail(decodeURIComponent(emailCookie.split("=")[1]))
      window.history.replaceState({}, "", "/")
    }
    if (params.get("gmail_error")) {
      setError(`OAuth error: ${params.get("gmail_error")}`)
      window.history.replaceState({}, "", "/")
    }
  }, [fetchEmails, profileId])

  if (connected === null) {
    return <div className="flex h-full items-center justify-center"><RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" /></div>
  }

  if (!connected) {
    return (
      <div className={cn("flex h-full items-center justify-center bg-background p-6", className)}>
        <div className="w-full max-w-md rounded-2xl border border-border bg-card p-5 shadow-sm">
          <div className="mb-4 flex items-center gap-3">
            <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-background ring-1 ring-border">
              <GoogleMark className="h-6 w-6" />
            </div>
            <div>
              <h2 className="text-base font-semibold text-foreground">Connect Google Mail</h2>
              <p className="text-xs text-muted-foreground">Gmail tokens can be sealed into your Trust Container after Google returns consent.</p>
            </div>
          </div>

          <div className="mb-4 rounded-xl border border-border bg-background/70 p-3">
            <div className="mb-2 flex items-center gap-2 text-xs font-medium text-foreground">
              <ShieldCheck className="h-4 w-4 text-primary" />
              Connection route
            </div>
            <div className="grid gap-2 text-xs text-muted-foreground">
              <div className="flex items-center justify-between gap-3"><span>Provider</span><span className="font-medium text-foreground">Google</span></div>
              <div className="flex items-center justify-between gap-3"><span>Broker</span><span className="font-mono text-foreground">edgerun.tech</span></div>
              <div className="flex items-center justify-between gap-3"><span>App access</span><span className="text-foreground">Gmail only</span></div>
            </div>
          </div>

          <div className="mb-4 space-y-2 rounded-xl border border-border bg-background/70 p-3 text-xs">
            <div className="flex items-center gap-2"><CheckCircle2 className="h-3.5 w-3.5 text-[var(--status-online)]" /> Read mailbox messages you choose to load</div>
            <div className="flex items-center gap-2"><CheckCircle2 className="h-3.5 w-3.5 text-[var(--status-online)]" /> Save the refresh token only inside your encrypted profile container</div>
            <div className="flex items-center gap-2 text-muted-foreground"><ExternalLink className="h-3.5 w-3.5" /> Google will show the final OAuth consent screen</div>
          </div>

          {error && <div className="mb-4 flex items-center gap-2 rounded-lg bg-destructive/10 px-3 py-2 text-xs text-destructive"><AlertCircle className="h-3.5 w-3.5 flex-shrink-0" />{error}</div>}

          <button
            onClick={connect}
            disabled={loading}
            className="flex h-11 w-full items-center justify-center gap-2 rounded-lg border border-border bg-background text-sm font-medium text-foreground transition hover:bg-secondary disabled:opacity-50"
          >
            {loading ? <RefreshCw className="h-4 w-4 animate-spin" /> : <GoogleMark className="h-4 w-4" />}
            Continue with Google
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className={cn("flex h-full", className)}>
      <div className="flex w-72 flex-shrink-0 flex-col border-r border-[var(--window-border)]">
        <div className="flex items-center justify-between border-b border-[var(--window-border)] px-3 py-2.5">
          <div className="flex min-w-0 items-center gap-2">
            <GoogleMark className="h-4 w-4 flex-shrink-0" />
            <span className="truncate text-xs font-medium text-foreground">{email || "Google Mail"}</span>
          </div>
          <button onClick={disconnect} className="inline-flex items-center gap-1.5 rounded-md border border-border px-2 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground" title="Disconnect Gmail session">
            <LogOut className="h-3.5 w-3.5" />
            Disconnect
          </button>
        </div>
        <div className="border-b border-[var(--window-border)] px-3 py-2 text-[10px] text-muted-foreground">
          {savedSecret ? "Refresh token saved in sealed profile" : "Connected for this browser session"} · Gmail scope
        </div>
        {pendingSecret && (
          <div className="border-b border-[var(--window-border)] bg-primary/5 p-3">
            <div className="mb-2 text-xs font-medium text-foreground">Save Gmail secret to Trust Container</div>
            <p className="mb-2 text-[10px] leading-4 text-muted-foreground">Your profile password decrypts and reseals the local container. The refresh token is removed from the temporary browser cookie after saving.</p>
            <input
              value={profilePassword}
              onChange={(event) => setProfilePassword(event.target.value)}
              type="password"
              placeholder="Profile password"
              className="mb-2 h-8 w-full rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-primary"
            />
            <button onClick={savePendingSecret} disabled={!profilePassword || auth.isLoading} className="w-full rounded-md bg-primary px-2 py-1.5 text-xs font-medium text-primary-foreground disabled:opacity-45">
              Save to Trust Container
            </button>
          </div>
        )}
        {savedSecret && !pendingSecret && (
          <div className="border-b border-[var(--window-border)] p-3">
            <div className="mb-2 text-xs font-medium text-foreground">Saved Gmail secret</div>
            <p className="mb-2 text-[10px] leading-4 text-muted-foreground">Disconnect clears this browser session. Removing the saved secret also deletes the refresh token copy sealed in your Trust Container.</p>
            <input
              value={profilePassword}
              onChange={(event) => setProfilePassword(event.target.value)}
              type="password"
              placeholder="Profile password to remove saved Gmail secret"
              className="mb-2 h-8 w-full rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-primary"
            />
            <button onClick={removeSavedSecret} disabled={!profilePassword || auth.isLoading} className="w-full rounded-md border border-border bg-secondary/50 px-2 py-1.5 text-xs text-muted-foreground hover:text-foreground disabled:opacity-45">
              Remove saved Gmail secret
            </button>
          </div>
        )}
        <div className="flex items-center gap-2 border-b border-[var(--window-border)] px-3 py-2">
          <button onClick={() => fetchEmails()} disabled={loading} className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground disabled:opacity-50">
            <RefreshCw className={cn("h-3 w-3", loading && "animate-spin")} />{loading ? "Loading..." : "Refresh"}
          </button>
        </div>
        <div className="flex-1 overflow-y-auto">
          {emails.length === 0 && !loading && <p className="p-4 text-center text-xs text-muted-foreground">No emails yet</p>}
          {emails.map((email) => {
            const { name } = parseFromField(email.from)
            return (
              <button key={email.id} onClick={() => setSelected(email)} className={cn("flex w-full gap-2.5 border-b border-[var(--window-border)]/50 px-3 py-2.5 text-left transition-colors hover:bg-secondary/50", selected?.id === email.id && "bg-secondary", email.unread && "bg-primary/5")}> 
                <Avatar name={email.from} size="sm" />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-1">{email.unread && <Circle className="h-2 w-2 flex-shrink-0 fill-primary text-primary" />}<span className={cn("truncate text-xs", email.unread ? "font-semibold" : "font-medium", "text-foreground")}>{name}</span></div>
                  <p className={cn("truncate text-xs", email.unread ? "font-medium" : "font-normal", "text-foreground")}>{email.subject}</p>
                  <p className="truncate text-[10px] text-muted-foreground">{email.snippet.replace(/</g, "&lt;").replace(/>/g, "&gt;").slice(0, 60)}</p>
                </div>
                <span className="flex-shrink-0 self-start pt-0.5 text-[10px] text-muted-foreground">{formatDate(email.date)}</span>
              </button>
            )
          })}
          {nextPage && <button onClick={() => fetchEmails(nextPage)} disabled={loading} className="w-full py-2 text-center text-xs text-primary hover:underline disabled:opacity-50">Load more</button>}
        </div>
      </div>

      <div className="flex flex-1 flex-col">
        {selected ? (
          <>
            <div className="border-b border-[var(--window-border)] px-4 py-3">
              <h2 className="text-sm font-semibold text-foreground">{selected.subject}</h2>
              <div className="mt-2 flex items-center gap-2">
                <Avatar name={selected.from} size="sm" />
                <div className="min-w-0 flex-1"><p className="text-xs font-medium text-foreground">{parseFromField(selected.from).name}</p><p className="text-[10px] text-muted-foreground">{parseFromField(selected.from).email}</p></div>
                <span className="flex items-center gap-1 text-[10px] text-muted-foreground"><Clock className="h-3 w-3" />{formatDate(selected.date)}</span>
              </div>
            </div>
            <div className="flex-1 overflow-y-auto p-4"><p className="whitespace-pre-wrap text-xs leading-relaxed text-foreground">{selected.snippet}</p></div>
          </>
        ) : (
          <div className="flex flex-1 items-center justify-center"><div className="text-center"><Inbox className="mx-auto h-10 w-10 text-muted-foreground/30" /><p className="mt-2 text-sm text-muted-foreground">Select an email to read</p></div></div>
        )}
      </div>
    </div>
  )
}
