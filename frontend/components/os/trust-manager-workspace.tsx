"use client"

import * as React from "react"
import {
  CheckCircle2,
  Copy,
  ExternalLink,
  Github,
  KeyRound,
  Link2,
  Lock,
  Mail,
  PlugZap,
  RefreshCw,
  ShieldCheck,
  Trash2,
  type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { TrustManagerApp } from "@/components/os/trust-manager-app"

type WorkspaceTab = "connections" | "authority"
type ConnectionStatus = "connected" | "available" | "review" | "revoked"

type ExternalConnection = {
  id: string
  provider: string
  account: string
  app: string
  broker: string
  scope: string[]
  route: string
  status: ConnectionStatus
  icon: LucideIcon
  connectedAt?: string
  lastUsed?: string
  authorityRef: string
  proofRef: string
}

const initialConnections: ExternalConnection[] = [
  {
    id: "google-gmail",
    provider: "Google",
    account: "Google account",
    app: "Gmail",
    broker: "edgerun.tech",
    scope: ["gmail.read", "gmail.send", "gmail.metadata"],
    route: "Google OAuth → edgerun.tech callback → Gmail app capability",
    status: "connected",
    icon: Mail,
    connectedAt: "active session",
    lastUsed: "checked on app open",
    authorityRef: "external:google:gmail",
    proofRef: "oauth:google:gmail:cookie-session",
  },
  {
    id: "github",
    provider: "GitHub",
    account: "Not connected",
    app: "GitHub",
    broker: "edgerun.tech",
    scope: ["repo.metadata", "pull_requests.read", "issues.read"],
    route: "GitHub OAuth → edgerun.tech callback → repo/app capabilities",
    status: "available",
    icon: Github,
    connectedAt: undefined,
    lastUsed: "never",
    authorityRef: "external:github:pending",
    proofRef: "oauth:github:pending",
  },
]

function statusClass(status: ConnectionStatus) {
  switch (status) {
    case "connected":
      return "border-[var(--status-online)]/25 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    case "available":
      return "border-primary/25 bg-primary/10 text-primary"
    case "review":
      return "border-[var(--status-warning)]/25 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
    case "revoked":
      return "border-[var(--status-error)]/25 bg-[var(--status-error)]/10 text-[var(--status-error)]"
  }
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

function ProviderIcon({ connection }: { connection: ExternalConnection }) {
  const Icon = connection.icon
  if (connection.provider === "Google") {
    return <GoogleMark className="h-5 w-5" />
  }
  return <Icon className="h-5 w-5" />
}

function ConnectionCard({ connection, active, onSelect }: { connection: ExternalConnection; active: boolean; onSelect: () => void }) {
  return (
    <button
      onClick={onSelect}
      className={cn(
        "flex w-full items-start gap-3 border-b border-border px-3 py-3 text-left transition hover:bg-secondary/60",
        active && "bg-primary/10",
      )}
    >
      <div className={cn("flex h-10 w-10 items-center justify-center rounded-xl border", statusClass(connection.status))}>
        <ProviderIcon connection={connection} />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <div className="truncate text-sm font-semibold text-foreground">{connection.provider}</div>
          <span className={cn("rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(connection.status))}>{connection.status}</span>
        </div>
        <div className="mt-0.5 truncate text-xs text-muted-foreground">{connection.app} via {connection.broker}</div>
        <div className="mt-1 truncate text-[11px] text-muted-foreground/70">{connection.scope.join(" · ")}</div>
      </div>
    </button>
  )
}

function ConnectionsView() {
  const [connections, setConnections] = React.useState(initialConnections)
  const [selectedId, setSelectedId] = React.useState(connections[0]?.id ?? "")
  const [message, setMessage] = React.useState("External authority is local-first: providers authenticate, Edgerun scopes access, apps consume capabilities.")
  const selected = connections.find((item) => item.id === selectedId) ?? connections[0]

  function updateSelected(patch: Partial<ExternalConnection>) {
    setConnections((prev) => prev.map((item) => item.id === selected.id ? { ...item, ...patch } : item))
  }

  async function copy(value: string, label: string) {
    await navigator.clipboard.writeText(value).catch(() => {})
    setMessage(`Copied ${label}`)
  }

  function connectGithub() {
    if (selected.id !== "github") return
    updateSelected({
      account: "GitHub account",
      status: "connected",
      connectedAt: "local draft",
      lastUsed: "just now",
      authorityRef: "external:github:repo-metadata",
      proofRef: "oauth:github:pending-real-callback",
    })
    setMessage("GitHub connection drafted. OAuth route should be wired next to make this real.")
  }

  function disconnect() {
    updateSelected({ status: "revoked", lastUsed: "revoked locally" })
    setMessage(`${selected.provider} marked revoked locally.`)
  }

  function review() {
    updateSelected({ status: selected.status === "review" ? "connected" : "review", lastUsed: "reviewed just now" })
    setMessage(`${selected.provider} review state updated.`)
  }

  return (
    <div className="grid h-full min-h-0 grid-cols-[320px_minmax(360px,1fr)] bg-background text-foreground">
      <section className="flex min-w-0 flex-col border-r border-border">
        <div className="border-b border-border p-3">
          <div className="flex items-center gap-2">
            <PlugZap className="h-4 w-4 text-primary" />
            <div>
              <h2 className="text-sm font-semibold">External connections</h2>
              <p className="text-[11px] text-muted-foreground">Provider accounts mapped into Edgerun capabilities.</p>
            </div>
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-auto">
          {connections.map((connection) => (
            <ConnectionCard key={connection.id} connection={connection} active={selected.id === connection.id} onSelect={() => setSelectedId(connection.id)} />
          ))}
        </div>
      </section>

      <section className="flex min-w-0 flex-col">
        <header className="flex h-14 items-center justify-between border-b border-border bg-[var(--window-header)]/40 px-4">
          <div className="flex items-center gap-3">
            <div className={cn("flex h-9 w-9 items-center justify-center rounded-xl border", statusClass(selected.status))}>
              <ProviderIcon connection={selected} />
            </div>
            <div>
              <h3 className="text-sm font-semibold">{selected.provider} → {selected.app}</h3>
              <p className="text-[11px] text-muted-foreground">through <span className="font-mono text-foreground">{selected.broker}</span></p>
            </div>
          </div>
          <span className={cn("rounded-md border px-2 py-1 text-[10px] font-semibold uppercase", statusClass(selected.status))}>{selected.status}</span>
        </header>

        <div className="min-h-0 flex-1 overflow-auto p-4">
          <div className="grid gap-4 lg:grid-cols-2">
            <div className="rounded-xl border border-border bg-card p-4">
              <div className="mb-3 flex items-center gap-2 text-sm font-semibold"><Link2 className="h-4 w-4 text-primary" />Connection route</div>
              <div className="space-y-2 text-xs">
                <div className="flex justify-between gap-3"><span className="text-muted-foreground">Provider</span><span className="font-medium">{selected.provider}</span></div>
                <div className="flex justify-between gap-3"><span className="text-muted-foreground">Broker</span><span className="font-mono">{selected.broker}</span></div>
                <div className="flex justify-between gap-3"><span className="text-muted-foreground">App</span><span>{selected.app}</span></div>
                <div className="flex justify-between gap-3"><span className="text-muted-foreground">Account</span><span>{selected.account}</span></div>
              </div>
              <div className="mt-3 rounded-lg border border-border bg-background/70 p-2 text-[11px] text-muted-foreground">{selected.route}</div>
            </div>

            <div className="rounded-xl border border-border bg-card p-4">
              <div className="mb-3 flex items-center gap-2 text-sm font-semibold"><KeyRound className="h-4 w-4 text-primary" />Scoped access</div>
              <div className="flex flex-wrap gap-1.5">
                {selected.scope.map((scope) => <span key={scope} className="rounded-md border border-primary/20 bg-primary/10 px-2 py-1 text-[11px] text-primary">{scope}</span>)}
              </div>
              <div className="mt-3 space-y-1 text-[11px] text-muted-foreground">
                <div className="flex items-center gap-2"><ShieldCheck className="h-3.5 w-3.5 text-[var(--status-online)]" /> Apps receive scoped capabilities, not raw provider tokens.</div>
                <div className="flex items-center gap-2"><Lock className="h-3.5 w-3.5 text-[var(--status-warning)]" /> Revocation should invalidate local capability grants and provider tokens.</div>
              </div>
            </div>

            <div className="rounded-xl border border-border bg-card p-4 lg:col-span-2">
              <div className="mb-3 flex items-center gap-2 text-sm font-semibold"><CheckCircle2 className="h-4 w-4 text-primary" />Evidence</div>
              <div className="grid gap-2 font-mono text-[11px]">
                <button onClick={() => copy(selected.authorityRef, "authority ref")} className="flex items-center justify-between gap-3 rounded-lg bg-background p-2 text-left ring-1 ring-border hover:ring-primary/40"><span className="text-muted-foreground">authority</span><span className="truncate">{selected.authorityRef}</span><Copy className="h-3 w-3" /></button>
                <button onClick={() => copy(selected.proofRef, "proof ref")} className="flex items-center justify-between gap-3 rounded-lg bg-background p-2 text-left ring-1 ring-border hover:ring-primary/40"><span className="text-muted-foreground">proof</span><span className="truncate">{selected.proofRef}</span><Copy className="h-3 w-3" /></button>
              </div>
            </div>
          </div>
        </div>

        <footer className="flex h-12 items-center justify-between border-t border-border bg-[var(--window-header)]/40 px-4">
          <p className="min-w-0 truncate text-[11px] text-muted-foreground">{message}</p>
          <div className="flex gap-2">
            {selected.id === "github" && selected.status !== "connected" && selected.status !== "revoked" && <button onClick={connectGithub} className="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground">Connect GitHub</button>}
            <button onClick={review} className="rounded-md bg-secondary px-3 py-1.5 text-xs font-medium hover:bg-secondary/80">Review</button>
            <button onClick={disconnect} className="rounded-md bg-[var(--status-error)]/10 px-3 py-1.5 text-xs font-medium text-[var(--status-error)] hover:bg-[var(--status-error)]/20"><Trash2 className="mr-1 inline h-3 w-3" />Revoke</button>
          </div>
        </footer>
      </section>
    </div>
  )
}

export function TrustManagerWorkspace() {
  const [tab, setTab] = React.useState<WorkspaceTab>("connections")
  return (
    <div className="flex h-full min-h-0 flex-col bg-background text-foreground">
      <div className="flex h-10 flex-shrink-0 items-center gap-1 border-b border-border bg-[var(--window-header)]/70 px-2">
        <button onClick={() => setTab("connections")} className={cn("rounded-md px-3 py-1.5 text-xs font-medium", tab === "connections" ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-secondary hover:text-foreground")}>Connections</button>
        <button onClick={() => setTab("authority")} className={cn("rounded-md px-3 py-1.5 text-xs font-medium", tab === "authority" ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-secondary hover:text-foreground")}>Authority</button>
      </div>
      <div className="min-h-0 flex-1">
        {tab === "connections" ? <ConnectionsView /> : <TrustManagerApp />}
      </div>
    </div>
  )
}
