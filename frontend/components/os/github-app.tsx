"use client"

import { useCallback, useEffect, useState } from "react"
import { AlertCircle, CheckCircle2, ExternalLink, GitBranch, LogOut, RefreshCw, ShieldCheck, Star } from "lucide-react"
import { cn } from "@/lib/utils"
import { useAuth, type OAuthProfileSecret } from "@/hooks/use-auth"
import { deleteAppSession, storeAppSession } from "@/platform/runtime/app-session-broker"
import { readCookie, deleteCookie } from "@/lib/oauth-utils"

type PendingGitHubSecret = Omit<OAuthProfileSecret, "appId" | "kind" | "updatedAtIso">
const PENDING_GITHUB_STORAGE_KEY = "edgerun:oauth-pending:github"

type GitHubRepo = {
  id: number
  full_name: string
  private: boolean
  html_url: string
  description?: string | null
  updated_at?: string
  stargazers_count?: number
  default_branch?: string
}

function readPendingGitHubSecret(): PendingGitHubSecret | null {
  const stored = sessionStorage.getItem(PENDING_GITHUB_STORAGE_KEY)
  if (stored) {
    try {
      const parsed = JSON.parse(stored) as PendingGitHubSecret
      return parsed.accessToken && parsed.expiresAtIso ? { ...parsed, scopes: parsed.scopes ?? [] } : null
    } catch {
      sessionStorage.removeItem(PENDING_GITHUB_STORAGE_KEY)
    }
  }
  const raw = readCookie("github_profile_pending")
  if (!raw) return null
  try {
    const parsed = JSON.parse(atob(raw.replace(/-/g, "+").replace(/_/g, "/"))) as PendingGitHubSecret
    return parsed.accessToken && parsed.expiresAtIso ? { ...parsed, scopes: parsed.scopes ?? [] } : null
  } catch {
    deleteCookie("github_profile_pending", PENDING_GITHUB_STORAGE_KEY)
    return null
  }
}

async function restoreGitHubSession(secret: PendingGitHubSecret, profileId?: string): Promise<boolean> {
  return storeAppSession({
    appId: "github",
    accessToken: secret.accessToken,
    expiresAtIso: secret.expiresAtIso,
    profileId,
    grantId: "github.repos.read",
  })
}

export function GitHubApp({ className }: { className?: string }) {
  const auth = useAuth()
  const savedSecret = auth.unlockedProfile?.appSecrets.find((secret) => secret.appId === "github")
  const profileId = auth.unlockedProfile?.ownerEncryption.identityIdHex
  const [connected, setConnected] = useState<boolean | null>(null)
  const [login, setLogin] = useState("")
  const [repos, setRepos] = useState<GitHubRepo[]>([])
  const [pendingSecret, setPendingSecret] = useState<PendingGitHubSecret | null>(null)
  const [profilePassword, setProfilePassword] = useState("")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchRepos = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await fetch("/api/github/repos")
      const data = await res.json()
      if (res.status === 401 && data.needReauth) {
        setConnected(false)
        setError("GitHub token expired or was revoked. Reconnect GitHub.")
        return
      }
      if (!res.ok) throw new Error(data.error || "Failed to fetch GitHub repositories")
      setRepos(data.repos ?? [])
      setConnected(true)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    const pending = readPendingGitHubSecret()
    if (pending) {
      setPendingSecret(pending)
      setLogin(pending.email)
    }
    const params = new URLSearchParams(window.location.search)
    if (params.get("github_connected") === "true") {
      setConnected(true)
      window.history.replaceState({}, "", "/")
      const pendingAfterRedirect = pending ?? readPendingGitHubSecret()
      if (pendingAfterRedirect) void restoreGitHubSession(pendingAfterRedirect, profileId).then(() => fetchRepos())
      else void fetchRepos()
    } else if (params.get("github_error")) {
      setError(`OAuth error: ${params.get("github_error")}`)
      window.history.replaceState({}, "", "/")
    }
  }, [fetchRepos, profileId])

  useEffect(() => {
    if (connected !== false || !savedSecret) return
    let cancelled = false
    async function restore() {
      if (!savedSecret) return
      setLoading(true)
      try {
        const ok = await restoreGitHubSession(savedSecret, profileId)
        if (cancelled) return
        if (ok) {
          setLogin(savedSecret.email)
          setConnected(true)
          await fetchRepos()
        }
      } catch (err) {
        if (!cancelled) setError(`Saved GitHub session restore failed: ${err}`)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void restore()
    return () => {
      cancelled = true
    }
  }, [connected, fetchRepos, profileId, savedSecret])

  useEffect(() => {
    if (connected !== null) return
    const loginCookie = readCookie("github_login")
    if (loginCookie) setLogin(decodeURIComponent(loginCookie))
    setConnected(Boolean(loginCookie || savedSecret || pendingSecret))
  }, [connected, pendingSecret, savedSecret])

  const connect = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await fetch("/api/github/auth")
      const data = await res.json()
      if (data.authUrl) window.location.href = data.authUrl
      else setError(data.error || "Failed to start GitHub connection")
    } catch (err) {
      setError(`Connection failed: ${err}`)
    } finally {
      setLoading(false)
    }
  }, [])

  const savePendingSecret = useCallback(async () => {
    if (!pendingSecret || !profilePassword) return
    const ok = await auth.saveOAuthSecret({ appId: "github", password: profilePassword, secret: pendingSecret })
    if (ok) {
      await restoreGitHubSession(pendingSecret, profileId)
      deleteCookie("github_profile_pending", PENDING_GITHUB_STORAGE_KEY)
      setPendingSecret(null)
      setProfilePassword("")
      setConnected(true)
      setLogin(pendingSecret.email)
      await fetchRepos()
    }
  }, [auth, fetchRepos, pendingSecret, profileId, profilePassword])

  const removeSavedSecret = useCallback(async () => {
    if (!profilePassword) return
    const ok = await auth.removeOAuthSecret("github", profilePassword)
    if (ok) setProfilePassword("")
  }, [auth, profilePassword])

  const disconnect = useCallback(() => {
    void deleteAppSession("github")
    void fetch("/api/github/session", { method: "DELETE" })
    deleteCookie("github_profile_pending", PENDING_GITHUB_STORAGE_KEY)
    setConnected(false)
    setLogin("")
    setRepos([])
    setPendingSecret(null)
  }, [])

  if (connected === null) {
    return <div className="flex h-full items-center justify-center"><RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" /></div>
  }

  if (!connected) {
    return (
      <div className={cn("flex h-full items-center justify-center bg-background p-6", className)}>
        <div className="w-full max-w-md rounded-lg border border-border bg-card p-5 shadow-sm">
          <div className="mb-4 flex items-center gap-3">
            <div className="flex h-11 w-11 items-center justify-center rounded-md bg-background ring-1 ring-border"><GitBranch className="h-6 w-6" /></div>
            <div>
              <h2 className="text-base font-semibold text-foreground">Connect GitHub</h2>
              <p className="text-xs text-muted-foreground">The access token can be sealed into your Trust Container after OAuth consent.</p>
            </div>
          </div>
          <div className="mb-4 space-y-2 rounded-md border border-border bg-background/70 p-3 text-xs">
            <div className="flex items-center gap-2"><ShieldCheck className="h-3.5 w-3.5 text-primary" /> EdgeRun requests repository and profile read access.</div>
            <div className="flex items-center gap-2"><CheckCircle2 className="h-3.5 w-3.5 text-[var(--status-online)]" /> Saving stores the token only inside your encrypted profile container.</div>
            <div className="flex items-center gap-2 text-muted-foreground"><ExternalLink className="h-3.5 w-3.5" /> GitHub hosts the final consent screen.</div>
          </div>
          {error && <div className="mb-4 flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive"><AlertCircle className="h-3.5 w-3.5" />{error}</div>}
          <button onClick={connect} disabled={loading} className="flex h-11 w-full items-center justify-center gap-2 rounded-md border border-border bg-background text-sm font-medium text-foreground hover:bg-secondary disabled:opacity-50">
            {loading ? <RefreshCw className="h-4 w-4 animate-spin" /> : <GitBranch className="h-4 w-4" />}
            Continue with GitHub
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className={cn("flex h-full flex-col bg-background", className)}>
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex min-w-0 items-center gap-2">
          <GitBranch className="h-4 w-4 shrink-0" />
          <span className="truncate text-sm font-medium text-foreground">{login || "GitHub"}</span>
        </div>
        <button onClick={disconnect} className="inline-flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-xs text-muted-foreground hover:bg-secondary hover:text-foreground" title="Disconnect GitHub session">
          <LogOut className="h-3.5 w-3.5" />
          Disconnect
        </button>
      </div>
      <div className="border-b border-border px-4 py-2 text-[11px] text-muted-foreground">
        {savedSecret ? "Access token saved in sealed profile" : "Connected for this browser session"} · repository list
      </div>
      {pendingSecret && (
        <div className="border-b border-border bg-primary/5 p-3">
          <div className="mb-1 text-xs font-medium text-foreground">Save GitHub secret to Trust Container</div>
          <p className="mb-2 text-[11px] leading-4 text-muted-foreground">Your profile password decrypts and reseals the local container. The temporary browser cookie is removed after saving.</p>
          <input value={profilePassword} onChange={(event) => setProfilePassword(event.target.value)} type="password" placeholder="Profile password" className="mb-2 h-8 w-full rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-primary" />
          <button onClick={savePendingSecret} disabled={!profilePassword || auth.isLoading} className="w-full rounded-md bg-primary px-2 py-1.5 text-xs font-medium text-primary-foreground disabled:opacity-45">Save to Trust Container</button>
        </div>
      )}
      {savedSecret && !pendingSecret && (
        <div className="border-b border-border p-3">
          <div className="mb-2 text-xs font-medium text-foreground">Saved GitHub secret</div>
          <p className="mb-2 text-[11px] leading-4 text-muted-foreground">Disconnect clears this browser session. Removing the saved secret also deletes the token copy sealed in your Trust Container.</p>
          <input value={profilePassword} onChange={(event) => setProfilePassword(event.target.value)} type="password" placeholder="Profile password to remove saved GitHub secret" className="mb-2 h-8 w-full rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-primary" />
          <button onClick={removeSavedSecret} disabled={!profilePassword || auth.isLoading} className="w-full rounded-md border border-border bg-secondary/50 px-2 py-1.5 text-xs text-muted-foreground hover:text-foreground disabled:opacity-45">Remove saved GitHub secret</button>
        </div>
      )}
      <div className="flex items-center gap-2 border-b border-border px-4 py-2">
        <button onClick={fetchRepos} disabled={loading} className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-secondary hover:text-foreground disabled:opacity-50">
          <RefreshCw className={cn("h-3 w-3", loading && "animate-spin")} />{loading ? "Loading..." : "Refresh"}
        </button>
      </div>
      {error && <div className="m-3 flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive"><AlertCircle className="h-3.5 w-3.5" />{error}</div>}
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {repos.length === 0 && !loading ? <p className="p-6 text-center text-xs text-muted-foreground">No repositories loaded yet</p> : null}
        <div className="grid gap-2">
          {repos.map((repo) => (
            <a key={repo.id} href={repo.html_url} target="_blank" rel="noreferrer" className="flex items-center gap-3 rounded-md border border-border bg-card p-3 hover:bg-secondary/45">
              <GitBranch className="h-4 w-4 shrink-0 text-primary" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium text-foreground">{repo.full_name}</div>
                <div className="mt-0.5 truncate text-[11px] text-muted-foreground">{repo.description || "No description"} · {repo.private ? "private" : "public"} · {repo.default_branch ?? "main"}</div>
              </div>
              <div className="flex shrink-0 items-center gap-1 text-[11px] text-muted-foreground"><Star className="h-3 w-3" />{repo.stargazers_count ?? 0}</div>
              <ExternalLink className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            </a>
          ))}
        </div>
      </div>
    </div>
  )
}
