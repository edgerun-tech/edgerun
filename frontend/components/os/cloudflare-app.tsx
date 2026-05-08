"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { AlertCircle, CheckCircle2, Cloud, Edit3, LogOut, Plus, RefreshCw, Save, ShieldCheck, Trash2 } from "lucide-react"
import { cn } from "@/lib/utils"
import { useAuth, type OAuthProfileSecret } from "@/hooks/use-auth"

type PendingCloudflareSecret = Omit<OAuthProfileSecret, "appId" | "kind" | "updatedAtIso">

type CloudflareZone = {
  id: string
  name: string
  status?: string
  account?: { name?: string }
}

type CloudflareDnsRecord = {
  id: string
  type: string
  name: string
  content: string
  ttl: number
  proxied?: boolean
  comment?: string
}

const DNS_TYPES = ["A", "AAAA", "CNAME", "TXT", "MX", "SRV", "CAA"] as const

function readCookie(name: string): string | null {
  const prefix = `${name}=`
  return document.cookie.split("; ").find((cookie) => cookie.startsWith(prefix))?.slice(prefix.length) ?? null
}

function deleteCookie(name: string) {
  document.cookie = `${name}=; Max-Age=0; path=/`
}

async function restoreCloudflareSession(secret: PendingCloudflareSecret): Promise<boolean> {
  const res = await fetch("/api/cloudflare/session", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(secret),
  })
  return res.ok
}

function emptyRecord(zoneName = "") {
  return { type: "A", name: zoneName, content: "", ttl: 1, proxied: true }
}

export function CloudflareApp({ className }: { className?: string }) {
  const auth = useAuth()
  const savedSecret = auth.unlockedProfile?.appSecrets.find((secret) => secret.appId === "cloudflare")
  const [connected, setConnected] = useState<boolean | null>(null)
  const [label, setLabel] = useState("")
  const [apiToken, setApiToken] = useState("")
  const [profilePassword, setProfilePassword] = useState("")
  const [zones, setZones] = useState<CloudflareZone[]>([])
  const [selectedZoneId, setSelectedZoneId] = useState("")
  const [records, setRecords] = useState<CloudflareDnsRecord[]>([])
  const [draft, setDraft] = useState(emptyRecord())
  const [editingId, setEditingId] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const selectedZone = useMemo(() => zones.find((zone) => zone.id === selectedZoneId) ?? zones[0], [selectedZoneId, zones])

  const fetchZones = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await fetch("/api/cloudflare/zones")
      const data = await res.json()
      if (res.status === 401 && data.needReauth) {
        setConnected(false)
        setError("Cloudflare token expired or lacks access. Reconnect with a DNS-scoped API token.")
        return
      }
      if (!res.ok) throw new Error(data.error || "Failed to fetch Cloudflare zones")
      const nextZones = data.result ?? []
      setZones(nextZones)
      setSelectedZoneId((current) => current || nextZones[0]?.id || "")
      setConnected(true)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  const fetchRecords = useCallback(async (zoneId = selectedZone?.id) => {
    if (!zoneId) return
    setLoading(true)
    setError(null)
    try {
      const res = await fetch(`/api/cloudflare/dns?zoneId=${encodeURIComponent(zoneId)}`)
      const data = await res.json()
      if (!res.ok) throw new Error(data.error || "Failed to fetch DNS records")
      setRecords(data.result ?? [])
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [selectedZone?.id])

  useEffect(() => {
    const labelCookie = readCookie("cloudflare_label")
    if (labelCookie) setLabel(decodeURIComponent(labelCookie))
    setConnected(Boolean(labelCookie || savedSecret))
  }, [savedSecret])

  useEffect(() => {
    if (connected !== false || !savedSecret) return
    let cancelled = false
    async function restore() {
      if (!savedSecret) return
      setLoading(true)
      try {
        const ok = await restoreCloudflareSession(savedSecret)
        if (cancelled) return
        if (ok) {
          setLabel(savedSecret.email)
          setConnected(true)
          await fetchZones()
        }
      } catch (err) {
        if (!cancelled) setError(`Saved Cloudflare session restore failed: ${err}`)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void restore()
    return () => {
      cancelled = true
    }
  }, [connected, fetchZones, savedSecret])

  useEffect(() => {
    if (connected) void fetchZones()
  }, [connected, fetchZones])

  useEffect(() => {
    if (selectedZone?.id) {
      setDraft((current) => ({ ...current, name: current.name || selectedZone.name }))
      void fetchRecords(selectedZone.id)
    }
  }, [fetchRecords, selectedZone?.id, selectedZone?.name])

  const connect = useCallback(async () => {
    if (!apiToken.trim()) return
    setLoading(true)
    setError(null)
    try {
      const secret: PendingCloudflareSecret = {
        email: label.trim() || "Cloudflare API token",
        accessToken: apiToken.trim(),
        expiresAtIso: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString(),
        scopes: ["Zone:Read", "DNS:Edit"],
      }
      const ok = await restoreCloudflareSession(secret)
      if (!ok) throw new Error("Cloudflare token verification failed")
      setConnected(true)
      setApiToken("")
      await fetchZones()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [apiToken, fetchZones, label])

  const saveSecret = useCallback(async () => {
    if (!apiToken.trim() || !profilePassword) return
    const secret: PendingCloudflareSecret = {
      email: label.trim() || "Cloudflare API token",
      accessToken: apiToken.trim(),
      expiresAtIso: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString(),
      scopes: ["Zone:Read", "DNS:Edit"],
    }
    const ok = await auth.saveOAuthSecret({ appId: "cloudflare", password: profilePassword, secret })
    if (ok) {
      await restoreCloudflareSession(secret)
      setProfilePassword("")
      setApiToken("")
      setConnected(true)
      await fetchZones()
    }
  }, [apiToken, auth, fetchZones, label, profilePassword])

  const removeSavedSecret = useCallback(async () => {
    if (!profilePassword) return
    const ok = await auth.removeOAuthSecret("cloudflare", profilePassword)
    if (ok) setProfilePassword("")
  }, [auth, profilePassword])

  const disconnect = useCallback(() => {
    void fetch("/api/cloudflare/session", { method: "DELETE" })
    deleteCookie("cloudflare_label")
    setConnected(false)
    setZones([])
    setRecords([])
    setSelectedZoneId("")
  }, [])

  const saveRecord = useCallback(async () => {
    if (!selectedZone?.id || !draft.name || !draft.content) return
    setLoading(true)
    setError(null)
    try {
      const method = editingId ? "PATCH" : "POST"
      const url = `/api/cloudflare/dns?zoneId=${encodeURIComponent(selectedZone.id)}${editingId ? `&recordId=${encodeURIComponent(editingId)}` : ""}`
      const res = await fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          type: draft.type,
          name: draft.name,
          content: draft.content,
          ttl: Number(draft.ttl) || 1,
          proxied: ["A", "AAAA", "CNAME"].includes(draft.type) ? Boolean(draft.proxied) : undefined,
        }),
      })
      const data = await res.json()
      if (!res.ok) throw new Error(data.error || "DNS record save failed")
      setDraft(emptyRecord(selectedZone.name))
      setEditingId(null)
      await fetchRecords(selectedZone.id)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [draft, editingId, fetchRecords, selectedZone])

  const deleteRecord = useCallback(async (record: CloudflareDnsRecord) => {
    if (!selectedZone?.id || !window.confirm(`Delete ${record.type} ${record.name}?`)) return
    setLoading(true)
    setError(null)
    try {
      const res = await fetch(`/api/cloudflare/dns?zoneId=${encodeURIComponent(selectedZone.id)}&recordId=${encodeURIComponent(record.id)}`, { method: "DELETE" })
      const data = await res.json()
      if (!res.ok) throw new Error(data.error || "DNS record delete failed")
      await fetchRecords(selectedZone.id)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [fetchRecords, selectedZone?.id])

  if (connected === null) {
    return <div className="flex h-full items-center justify-center"><RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" /></div>
  }

  if (!connected) {
    return (
      <div className={cn("flex h-full items-center justify-center bg-background p-6", className)}>
        <div className="w-full max-w-md rounded-lg border border-border bg-card p-5 shadow-sm">
          <div className="mb-4 flex items-center gap-3">
            <div className="flex h-11 w-11 items-center justify-center rounded-md bg-background ring-1 ring-border"><Cloud className="h-6 w-6" /></div>
            <div>
              <h2 className="text-base font-semibold text-foreground">Connect Cloudflare</h2>
              <p className="text-xs text-muted-foreground">Use a scoped API token for zone read and DNS edit access.</p>
            </div>
          </div>
          <div className="mb-4 space-y-2 rounded-md border border-border bg-background/70 p-3 text-xs">
            <div className="flex items-center gap-2"><ShieldCheck className="h-3.5 w-3.5 text-primary" /> Create a Cloudflare token with Zone:Read and DNS:Edit for selected zones.</div>
            <div className="flex items-center gap-2"><CheckCircle2 className="h-3.5 w-3.5 text-[var(--status-online)]" /> Saving stores the token only inside your encrypted profile container.</div>
          </div>
          <input value={label} onChange={(event) => setLabel(event.target.value)} placeholder="Label, e.g. Edgerun DNS" className="mb-2 h-10 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary" />
          <input value={apiToken} onChange={(event) => setApiToken(event.target.value)} type="password" placeholder="Cloudflare API token" className="mb-2 h-10 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary" />
          <input value={profilePassword} onChange={(event) => setProfilePassword(event.target.value)} type="password" placeholder="Profile password to save token" className="mb-3 h-10 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary" />
          {error && <div className="mb-4 flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive"><AlertCircle className="h-3.5 w-3.5" />{error}</div>}
          <div className="grid grid-cols-2 gap-2">
            <button onClick={connect} disabled={!apiToken || loading} className="flex h-10 items-center justify-center gap-2 rounded-md border border-border bg-background text-sm font-medium text-foreground hover:bg-secondary disabled:opacity-50">
              {loading ? <RefreshCw className="h-4 w-4 animate-spin" /> : <Cloud className="h-4 w-4" />}
              Session only
            </button>
            <button onClick={saveSecret} disabled={!apiToken || !profilePassword || loading || auth.isLoading} className="flex h-10 items-center justify-center gap-2 rounded-md bg-primary text-sm font-medium text-primary-foreground disabled:opacity-45">
              <Save className="h-4 w-4" />
              Save sealed
            </button>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className={cn("flex h-full flex-col bg-background", className)}>
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex min-w-0 items-center gap-2">
          <Cloud className="h-4 w-4 shrink-0" />
          <span className="truncate text-sm font-medium text-foreground">{label || "Cloudflare DNS"}</span>
        </div>
        <button onClick={disconnect} className="inline-flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-xs text-muted-foreground hover:bg-secondary hover:text-foreground">
          <LogOut className="h-3.5 w-3.5" />
          Disconnect
        </button>
      </div>
      <div className="border-b border-border px-4 py-2 text-[11px] text-muted-foreground">
        {savedSecret ? "API token saved in sealed profile" : "Connected for this browser session"} · DNS records
      </div>
      {savedSecret && (
        <div className="border-b border-border p-3">
          <div className="mb-2 text-xs font-medium text-foreground">Saved Cloudflare secret</div>
          <div className="grid grid-cols-[1fr_auto] gap-2">
            <input value={profilePassword} onChange={(event) => setProfilePassword(event.target.value)} type="password" placeholder="Profile password to remove saved token" className="h-8 rounded-md border border-border bg-background px-2 text-xs outline-none focus:border-primary" />
            <button onClick={removeSavedSecret} disabled={!profilePassword || auth.isLoading} className="rounded-md border border-border bg-secondary/50 px-2 text-xs text-muted-foreground hover:text-foreground disabled:opacity-45">Remove sealed token</button>
          </div>
        </div>
      )}
      <div className="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[280px_minmax(0,1fr)]">
        <aside className="min-h-0 border-b border-border p-3 md:border-b-0 md:border-r">
          <button onClick={fetchZones} disabled={loading} className="mb-3 flex w-full items-center justify-center gap-1.5 rounded-md border border-border bg-secondary/50 px-3 py-2 text-xs text-foreground hover:bg-secondary disabled:opacity-50">
            <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
            Refresh zones
          </button>
          <div className="grid gap-2">
            {zones.map((zone) => (
              <button key={zone.id} onClick={() => setSelectedZoneId(zone.id)} className={cn("rounded-md border p-3 text-left text-xs", selectedZone?.id === zone.id ? "border-primary bg-primary/10" : "border-border bg-card hover:bg-secondary/45")}>
                <div className="font-medium text-foreground">{zone.name}</div>
                <div className="mt-1 text-muted-foreground">{zone.account?.name || zone.status || zone.id.slice(0, 8)}</div>
              </button>
            ))}
          </div>
        </aside>
        <main className="min-h-0 overflow-auto p-3">
          <div className="mb-3 rounded-md border border-border bg-card p-3">
            <div className="mb-2 flex items-center gap-2 text-xs font-medium text-foreground">
              {editingId ? <Edit3 className="h-3.5 w-3.5" /> : <Plus className="h-3.5 w-3.5" />}
              {editingId ? "Edit DNS record" : "Create DNS record"}
            </div>
            <div className="grid gap-2 md:grid-cols-[90px_1fr_1fr_90px_90px_auto]">
              <select value={draft.type} onChange={(event) => setDraft((current) => ({ ...current, type: event.target.value }))} className="rounded-md border border-border bg-background px-2 py-2 text-xs">
                {DNS_TYPES.map((type) => <option key={type} value={type}>{type}</option>)}
              </select>
              <input value={draft.name} onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))} placeholder="name" className="rounded-md border border-border bg-background px-2 py-2 text-xs outline-none focus:border-primary" />
              <input value={draft.content} onChange={(event) => setDraft((current) => ({ ...current, content: event.target.value }))} placeholder="content" className="rounded-md border border-border bg-background px-2 py-2 text-xs outline-none focus:border-primary" />
              <input value={draft.ttl} onChange={(event) => setDraft((current) => ({ ...current, ttl: Number(event.target.value) || 1 }))} type="number" min={1} placeholder="ttl" className="rounded-md border border-border bg-background px-2 py-2 text-xs outline-none focus:border-primary" />
              <label className="flex items-center justify-center gap-1 rounded-md border border-border bg-background px-2 py-2 text-xs text-muted-foreground">
                <input type="checkbox" checked={Boolean(draft.proxied)} disabled={!["A", "AAAA", "CNAME"].includes(draft.type)} onChange={(event) => setDraft((current) => ({ ...current, proxied: event.target.checked }))} />
                proxy
              </label>
              <button onClick={saveRecord} disabled={!selectedZone || !draft.name || !draft.content || loading} className="rounded-md bg-primary px-3 py-2 text-xs font-medium text-primary-foreground disabled:opacity-45">
                {editingId ? "Update" : "Create"}
              </button>
            </div>
          </div>
          {error && <div className="mb-3 flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive"><AlertCircle className="h-3.5 w-3.5" />{error}</div>}
          <div className="grid gap-2">
            {records.map((record) => (
              <div key={record.id} className="grid gap-2 rounded-md border border-border bg-card p-3 text-xs md:grid-cols-[70px_minmax(0,1fr)_minmax(0,1fr)_80px_auto] md:items-center">
                <span className="font-mono font-semibold text-primary">{record.type}</span>
                <span className="min-w-0 truncate font-medium text-foreground">{record.name}</span>
                <span className="min-w-0 truncate font-mono text-muted-foreground">{record.content}</span>
                <span className="text-muted-foreground">{record.proxied ? "proxied" : `TTL ${record.ttl}`}</span>
                <div className="flex justify-end gap-1">
                  <button onClick={() => { setEditingId(record.id); setDraft({ type: record.type, name: record.name, content: record.content, ttl: record.ttl, proxied: Boolean(record.proxied) }) }} className="rounded-md border border-border p-2 text-muted-foreground hover:bg-secondary hover:text-foreground" aria-label="Edit DNS record">
                    <Edit3 className="h-3.5 w-3.5" />
                  </button>
                  <button onClick={() => deleteRecord(record)} className="rounded-md border border-border p-2 text-muted-foreground hover:bg-destructive/10 hover:text-destructive" aria-label="Delete DNS record">
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>
            ))}
            {records.length === 0 && !loading && <div className="rounded-md border border-dashed border-border p-6 text-center text-xs text-muted-foreground">No DNS records loaded.</div>}
          </div>
        </main>
      </div>
    </div>
  )
}
