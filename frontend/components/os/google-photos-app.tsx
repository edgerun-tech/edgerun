"use client"

import { useCallback, useState } from "react"
import { AlertCircle, ExternalLink, ImageIcon, Images, LogOut, RefreshCw } from "lucide-react"
import { cn } from "@/lib/utils"

type PickerSession = {
  id?: string
  pickerUri?: string
  mediaItemsSet?: boolean
}

type PickedMediaItem = {
  id?: string
  mediaFile?: {
    baseUrl?: string
    mimeType?: string
    filename?: string
  }
  createTime?: string
}

export function GooglePhotosApp({ className }: { className?: string }) {
  const [session, setSession] = useState<PickerSession | null>(null)
  const [items, setItems] = useState<PickedMediaItem[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const connect = useCallback(async () => {
    const res = await fetch("/api/google-drive/auth")
    const data = await res.json()
    if (data.authUrl) window.location.href = data.authUrl
    else setError(data.error || "Failed to start Google connection")
  }, [])

  const createSession = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await fetch("/api/google-photos/session", { method: "POST" })
      const data = await res.json()
      if (res.status === 401) {
        setError("Google session is missing or needs the Photos Picker scope. Reconnect Google.")
        return
      }
      if (!res.ok) throw new Error(data.error || "Failed to create Photos Picker session")
      setSession(data)
      setItems([])
      if (data.pickerUri) window.open(data.pickerUri, "_blank", "noreferrer")
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  const fetchSelection = useCallback(async () => {
    if (!session?.id) return
    setLoading(true)
    setError(null)
    try {
      const statusRes = await fetch(`/api/google-photos/session?sessionId=${encodeURIComponent(session.id)}`)
      const status = await statusRes.json()
      if (!statusRes.ok) throw new Error(status.error || "Failed to check Photos Picker session")
      setSession(status)
      const itemsRes = await fetch(`/api/google-photos/session?sessionId=${encodeURIComponent(session.id)}&mediaItems=true`)
      const data = await itemsRes.json()
      if (!itemsRes.ok) throw new Error(data.error || "Failed to load selected photos")
      setItems(data.mediaItems ?? [])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [session])

  const disconnect = useCallback(() => {
    void fetch("/api/google-drive/session", { method: "DELETE" })
    setSession(null)
    setItems([])
    setError("Google session disconnected. Reconnect Google to pick photos.")
  }, [])

  return (
    <div className={cn("flex h-full flex-col bg-background", className)}>
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex items-center gap-2">
          <Images className="h-4 w-4 text-primary" />
          <div>
            <div className="text-sm font-semibold text-foreground">Google Photos</div>
            <div className="text-[11px] text-muted-foreground">Picker API for user-selected photos and videos</div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={fetchSelection} disabled={loading || !session?.id} className="rounded p-1.5 text-muted-foreground hover:bg-secondary hover:text-foreground disabled:opacity-40" title="Refresh selected media">
            <RefreshCw className={cn("h-4 w-4", loading && "animate-spin")} />
          </button>
          <button onClick={disconnect} className="inline-flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-xs text-muted-foreground hover:bg-secondary hover:text-foreground" title="Disconnect Google session">
            <LogOut className="h-3.5 w-3.5" />
            Disconnect
          </button>
        </div>
      </div>
      <div className="border-b border-border px-4 py-2 text-[11px] text-muted-foreground">Uses the same Google session and sealed Trust Container secret as Drive and Contacts.</div>
      <div className="border-b border-border p-3">
        <div className="mb-2 rounded-md border border-border bg-secondary/30 p-3 text-[11px] leading-4 text-muted-foreground">
          Google no longer grants general third-party reads of the full Photos library for normal apps. This uses the official Picker flow: you choose photos in Google, then EdgeRun can list only that selected session.
        </div>
        <div className="flex gap-2">
          <button onClick={createSession} disabled={loading} className="inline-flex items-center gap-2 rounded-md border border-border px-3 py-1.5 text-xs text-foreground hover:bg-secondary disabled:opacity-50">
            <ExternalLink className="h-3.5 w-3.5" /> Pick from Google Photos
          </button>
          <button onClick={connect} className="inline-flex items-center gap-2 rounded-md border border-border px-3 py-1.5 text-xs text-muted-foreground hover:bg-secondary hover:text-foreground">
            Reconnect Google
          </button>
        </div>
      </div>
      {error && <div className="m-3 flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive"><AlertCircle className="h-3.5 w-3.5" />{error}</div>}
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {items.length === 0 ? <p className="p-6 text-center text-xs text-muted-foreground">No selected Photos media loaded yet</p> : null}
        <div className="grid grid-cols-[repeat(auto-fill,minmax(132px,1fr))] gap-2">
          {items.map((item, index) => {
            const file = item.mediaFile
            const src = file?.baseUrl ? `${file.baseUrl}=w360-h240` : ""
            return (
              <a key={item.id ?? index} href={file?.baseUrl} target="_blank" rel="noreferrer" className="overflow-hidden rounded-md border border-border bg-card hover:bg-secondary/45">
                <div className="flex aspect-[4/3] items-center justify-center bg-secondary">
                  {src ? <img src={src} alt="" className="h-full w-full object-cover" /> : <ImageIcon className="h-8 w-8 text-muted-foreground" />}
                </div>
                <div className="p-2">
                  <div className="truncate text-xs font-medium text-foreground">{file?.filename || item.id || "Selected media"}</div>
                  <div className="mt-0.5 truncate text-[10px] text-muted-foreground">{file?.mimeType || "Google Photos media"}</div>
                </div>
              </a>
            )
          })}
        </div>
      </div>
    </div>
  )
}
