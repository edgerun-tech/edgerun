"use client"

import { useCallback, useEffect, useState } from "react"
import { AlertCircle, CheckCircle2, CloudUpload, ExternalLink, FileText, Folder, LogOut, RefreshCw, ShieldCheck } from "lucide-react"
import { cn } from "@/lib/utils"
import { useAuth, type OAuthProfileSecret } from "@/hooks/use-auth"
import { deleteAppSession, storeAppSession } from "@/platform/runtime/app-session-broker"
import { runtimeEventLog } from "@/platform/runtime/runtime-event-log"
import { formatBytes } from "@/lib/format"
import { readCookie, deleteCookie, readPendingOAuthSecret, GoogleMark, type PendingOAuthSecret } from "@/lib/oauth-utils"

const PENDING_DRIVE_STORAGE_KEY = "edgerun:oauth-pending:google-drive"

type DriveFile = {
  id: string
  name: string
  mimeType: string
  modifiedTime?: string
  size?: string
  webViewLink?: string
}

async function restoreDriveSession(secret: PendingOAuthSecret | OAuthProfileSecret, profileId?: string): Promise<boolean> {
  return storeAppSession({
    appId: "google-drive",
    accessToken: secret.accessToken,
    expiresAtIso: secret.expiresAtIso,
    profileId,
    grantId: "google.drive.import.sync",
  })
}

export function GoogleDriveApp({ className }: { className?: string }) {
  const auth = useAuth()
  const savedSecret = auth.unlockedProfile?.appSecrets.find((secret) => secret.appId === "google-drive")
  const profileId = auth.unlockedProfile?.ownerEncryption.identityIdHex
  const [connected, setConnected] = useState<boolean | null>(null)
  const [email, setEmail] = useState("")
  const [files, setFiles] = useState<DriveFile[]>([])
  const [pendingSecret, setPendingSecret] = useState<PendingOAuthSecret | null>(null)
  const [profilePassword, setProfilePassword] = useState("")
  const [nextPage, setNextPage] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [syncing, setSyncing] = useState(false)
  const [activity, setActivity] = useState("Connect Drive, then import selected metadata/files into EdgeRun storage.")
  const [error, setError] = useState<string | null>(null)

  const fetchFiles = useCallback(async (pageToken?: string) => {
    setLoading(true)
    setError(null)
    try {
      const url = new URL("/api/google-drive/files", window.location.origin)
      if (pageToken) url.searchParams.set("pageToken", pageToken)
      const res = await fetch(url.toString())
      const data = await res.json()
      if (res.status === 401 && data.needReauth) {
        setConnected(false)
        setError("Google Drive session expired. Reconnect or save a fresh secret to your Trust Container.")
        return
      }
      if (!res.ok) {
        setError(data.error || "Failed to fetch Google Drive files")
        return
      }
      setFiles((current) => pageToken ? [...current, ...(data.files ?? [])] : (data.files ?? []))
      setNextPage(data.nextPageToken ?? null)
      setConnected(true)
      setActivity("Drive file list refreshed. Nothing is imported until you choose sync.")
    } catch (err) {
      setError(`Failed to fetch Google Drive files: ${err}`)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    const pending = readPendingOAuthSecret(PENDING_DRIVE_STORAGE_KEY, "google_drive_profile_pending")
    if (pending) {
      setPendingSecret(pending)
      setEmail(pending.email)
    }
    const connectedParam = new URLSearchParams(window.location.search)
    if (connectedParam.get("google_drive_connected") === "true") {
      setConnected(true)
      window.history.replaceState({}, "", "/")
      const pendingAfterRedirect = pending ?? readPendingOAuthSecret(PENDING_DRIVE_STORAGE_KEY, "google_drive_profile_pending")
      if (pendingAfterRedirect) void restoreDriveSession(pendingAfterRedirect, profileId).then(() => fetchFiles())
      else void fetchFiles()
    } else if (connectedParam.get("google_drive_error")) {
      setError(`OAuth error: ${connectedParam.get("google_drive_error")}`)
      window.history.replaceState({}, "", "/")
    }
  }, [fetchFiles, profileId])

  useEffect(() => {
    if (connected !== false || !savedSecret) return
    let cancelled = false
    async function restore() {
      if (!savedSecret) return
      setLoading(true)
      setError(null)
      try {
        const ok = await restoreDriveSession(savedSecret, profileId)
        if (cancelled) return
        if (ok) {
          setEmail(savedSecret.email)
          setConnected(true)
          await fetchFiles()
        }
      } catch (err) {
        if (!cancelled) setError(`Saved Drive session restore failed: ${err}`)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void restore()
    return () => {
      cancelled = true
    }
  }, [connected, fetchFiles, profileId, savedSecret])

  useEffect(() => {
    if (connected !== null) return
    const emailCookie = readCookie("google_drive_email")
    if (emailCookie) setEmail(decodeURIComponent(emailCookie))
    setConnected(Boolean(emailCookie || savedSecret || pendingSecret))
  }, [connected, pendingSecret, savedSecret])

  const connect = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await fetch("/api/google-drive/auth")
      const data = await res.json()
      if (data.authUrl) window.location.href = data.authUrl
      else setError(data.error || "Failed to start Google Drive connection")
    } catch (err) {
      setError(`Connection failed: ${err}`)
    } finally {
      setLoading(false)
    }
  }, [])

  const savePendingSecret = useCallback(async () => {
    if (!pendingSecret || !profilePassword) return
    const ok = await auth.saveOAuthSecret({ appId: "google-drive", password: profilePassword, secret: pendingSecret })
    if (ok) {
      await restoreDriveSession(pendingSecret, profileId)
      deleteCookie("google_drive_profile_pending", PENDING_DRIVE_STORAGE_KEY)
      setPendingSecret(null)
      setProfilePassword("")
      setConnected(true)
      setEmail(pendingSecret.email)
      setActivity("Drive refresh token sealed into your Trust Container. You can now sync into EdgeRun storage.")
      await fetchFiles()
    }
  }, [auth, fetchFiles, pendingSecret, profileId, profilePassword])

  const removeSavedSecret = useCallback(async () => {
    if (!profilePassword) return
    const ok = await auth.removeOAuthSecret("google-drive", profilePassword)
    if (ok) setProfilePassword("")
  }, [auth, profilePassword])

  const disconnect = useCallback(() => {
    void deleteAppSession("google-drive")
    void fetch("/api/google-drive/session", { method: "DELETE" })
    deleteCookie("google_drive_profile_pending", PENDING_DRIVE_STORAGE_KEY)
    setConnected(false)
    setEmail("")
    setFiles([])
    setPendingSecret(null)
    setActivity("Drive session disconnected. Imported EdgeRun copies remain under your storage policy.")
  }, [])

  const syncVisibleFiles = useCallback(async () => {
    if (files.length === 0) return
    setSyncing(true)
    setError(null)
    try {
      const fileCount = files.length
      const totalBytes = files.reduce((sum, file) => sum + Number(file.size ?? 0), 0)
      runtimeEventLog.append({
        kind: "capability_action_completed",
        actor: "google-drive",
        target: "edgerun-storage",
        capabilityId: "storage.import.google-drive",
        reason: "queued visible Drive files for EdgeRun storage sync",
        metadata: {
          fileCount,
          totalBytes,
          source: "google-drive",
          destination: "edgerun-storage",
          mode: "metadata-first-sync",
        },
      })
      setActivity(`Queued ${fileCount} visible Drive item${fileCount === 1 ? "" : "s"} for EdgeRun storage sync. Real byte upload will use the storage pipeline.`)
    } catch (err) {
      setError(`Sync queue failed: ${err}`)
    } finally {
      setSyncing(false)
    }
  }, [files])

  if (connected === null) {
    return <div className="flex h-full items-center justify-center"><RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" /></div>
  }

  if (!connected) {
    return (
      <div className={cn("flex h-full items-center justify-center bg-background p-6", className)}>
        <div className="w-full max-w-md rounded-lg border border-border bg-card p-5 shadow-sm">
          <div className="mb-4 flex items-center gap-3">
            <div className="flex h-11 w-11 items-center justify-center rounded-md bg-background ring-1 ring-border"><GoogleMark className="h-6 w-6" /></div>
            <div>
              <h2 className="text-base font-semibold text-foreground">Connect Google Drive</h2>
              <p className="text-xs text-muted-foreground">Use Drive as an import source. EdgeRun storage becomes the user-owned destination.</p>
            </div>
          </div>
          <div className="mb-4 space-y-2 rounded-md border border-border bg-background/70 p-3 text-xs">
            <div className="flex items-center gap-2"><ShieldCheck className="h-3.5 w-3.5 text-primary" /> EdgeRun asks Google for Drive browse access and app-file write access.</div>
            <div className="flex items-center gap-2"><CheckCircle2 className="h-3.5 w-3.5 text-[var(--status-online)]" /> Save the refresh token only inside your encrypted Trust Container.</div>
            <div className="flex items-center gap-2 text-muted-foreground"><CloudUpload className="h-3.5 w-3.5" /> Sync selected metadata/files into EdgeRun storage, then slowly rely less on Google.</div>
            <div className="flex items-center gap-2 text-muted-foreground"><ExternalLink className="h-3.5 w-3.5" /> Google hosts the final consent screen.</div>
          </div>
          {error && <div className="mb-4 flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive"><AlertCircle className="h-3.5 w-3.5" />{error}</div>}
          <button onClick={connect} disabled={loading} className="flex h-11 w-full items-center justify-center gap-2 rounded-md border border-border bg-background text-sm font-medium text-foreground hover:bg-secondary disabled:opacity-50">
            {loading ? <RefreshCw className="h-4 w-4 animate-spin" /> : <GoogleMark className="h-4 w-4" />}
            Continue with Google
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className={cn("flex h-full flex-col bg-background", className)}>
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex min-w-0 items-center gap-2">
          <GoogleMark className="h-4 w-4 shrink-0" />
          <span className="truncate text-sm font-medium text-foreground">{email || "Google Drive"}</span>
        </div>
        <button onClick={disconnect} className="inline-flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-xs text-muted-foreground hover:bg-secondary hover:text-foreground" title="Disconnect Google session">
          <LogOut className="h-3.5 w-3.5" />
          Disconnect
        </button>
      </div>
      <div className="border-b border-border px-4 py-2 text-[11px] text-muted-foreground">
        {savedSecret ? "Refresh token saved in sealed Trust Container" : "Connected for this browser session"} · Google Drive is a source; EdgeRun storage is the destination
      </div>
      {pendingSecret && (
        <div className="border-b border-border bg-primary/5 p-3">
          <div className="mb-1 text-xs font-medium text-foreground">Save Google Drive secret to Trust Container</div>
          <p className="mb-2 text-[11px] leading-4 text-muted-foreground">Your profile password decrypts and reseals the local container. The refresh token is removed from the temporary browser cookie after saving.</p>
          <input value={profilePassword} onChange={(event) => setProfilePassword(event.target.value)} type="password" placeholder="Profile password" className="mb-2 h-8 w-full rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-primary" />
          <button onClick={savePendingSecret} disabled={!profilePassword || auth.isLoading} className="w-full rounded-md bg-primary px-2 py-1.5 text-xs font-medium text-primary-foreground disabled:opacity-45">Save to Trust Container</button>
        </div>
      )}
      {savedSecret && !pendingSecret && (
        <div className="border-b border-border p-3">
          <div className="mb-2 text-xs font-medium text-foreground">Saved Google secret</div>
          <p className="mb-2 text-[11px] leading-4 text-muted-foreground">Disconnect clears this browser session. Removing the saved secret also deletes the refresh token copy sealed in your Trust Container. Imported EdgeRun copies remain yours.</p>
          <input value={profilePassword} onChange={(event) => setProfilePassword(event.target.value)} type="password" placeholder="Profile password to remove saved Google secret" className="mb-2 h-8 w-full rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-primary" />
          <button onClick={removeSavedSecret} disabled={!profilePassword || auth.isLoading} className="w-full rounded-md border border-border bg-secondary/50 px-2 py-1.5 text-xs text-muted-foreground hover:text-foreground disabled:opacity-45">Remove saved Drive secret</button>
        </div>
      )}
      <div className="flex items-center gap-2 border-b border-border px-4 py-2">
        <button onClick={() => fetchFiles()} disabled={loading} className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-secondary hover:text-foreground disabled:opacity-50">
          <RefreshCw className={cn("h-3 w-3", loading && "animate-spin")} />{loading ? "Loading..." : "Refresh"}
        </button>
        <button onClick={syncVisibleFiles} disabled={syncing || files.length === 0} className="flex items-center gap-1.5 rounded-md bg-primary px-2 py-1 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50">
          <CloudUpload className={cn("h-3 w-3", syncing && "animate-pulse")} />{syncing ? "Queueing..." : "Sync visible to EdgeRun"}
        </button>
      </div>
      <div className="border-b border-border bg-primary/5 px-4 py-2 text-[11px] leading-4 text-muted-foreground">
        {activity}
      </div>
      {error && <div className="m-3 flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive"><AlertCircle className="h-3.5 w-3.5" />{error}</div>}
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {files.length === 0 && !loading ? <p className="p-6 text-center text-xs text-muted-foreground">No Drive files loaded yet</p> : null}
        <div className="grid gap-2">
          {files.map((file) => {
            const folder = file.mimeType === "application/vnd.google-apps.folder"
            const Icon = folder ? Folder : FileText
            return (
              <a key={file.id} href={file.webViewLink} target="_blank" rel="noreferrer" className="flex items-center gap-3 rounded-md border border-border bg-card p-3 hover:bg-secondary/45">
                <Icon className="h-4 w-4 shrink-0 text-primary" />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium text-foreground">{file.name}</div>
                  <div className="mt-0.5 text-[11px] text-muted-foreground">{formatBytes(file.size, "folder or Google doc")} · {file.modifiedTime ? new Date(file.modifiedTime).toLocaleString() : "modified time unavailable"}</div>
                </div>
                <ExternalLink className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
              </a>
            )
          })}
        </div>
        {nextPage && <button onClick={() => fetchFiles(nextPage)} disabled={loading} className="mt-3 w-full rounded-md border border-border py-2 text-center text-xs text-primary hover:bg-secondary disabled:opacity-50">Load more</button>}
      </div>
    </div>
  )
}
