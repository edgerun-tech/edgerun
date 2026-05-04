"use client"

import * as React from "react"
import {
  CheckCircle2,
  Copy,
  GitBranch,
  KeyRound,
  Link2,
  Mail,
  PlugZap,
  RefreshCw,
  Route,
  ScrollText,
  Shield,
  ShieldCheck,
  SlidersHorizontal,
  Trash2,
  type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"

type Section = "connections" | "capabilities" | "routes" | "audit"
type Status = "connected" | "available" | "review" | "revoked" | "active" | "verified"
type Risk = "low" | "medium" | "high"

type Connection = {
  id: string
  provider: string
  product: string
  account: string
  broker: string
  status: Status
  risk: Risk
  scopes: string[]
  route: string
  appAccess: string[]
  authorityRef: string
  proofRef: string
  lastUsed: string
  icon: LucideIcon
}

type TrustRecord = {
  id: string
  title: string
  subtitle: string
  status: Status
  risk: Risk
  capability: string
  detail: string
  authorityRef: string
  proofRef: string
  icon: LucideIcon
}

const sections: { id: Section; label: string; icon: LucideIcon }[] = [
  { id: "connections", label: "Connections", icon: PlugZap },
  { id: "capabilities", label: "Capabilities", icon: SlidersHorizontal },
  { id: "routes", label: "Routes", icon: Route },
  { id: "audit", label: "Audit", icon: ScrollText },
]

const initialConnections: Connection[] = [
  {
    id: "google-gmail",
    provider: "Google",
    product: "Gmail",
    account: "Google account",
    broker: "edgerun.tech",
    status: "connected",
    risk: "medium",
    scopes: ["gmail.read", "gmail.send", "gmail.metadata"],
    route: "Google OAuth → edgerun.tech callback → Gmail app capability",
    appAccess: ["Gmail app can read loaded messages", "Gmail app can send mail", "Other apps need separate grants"],
    authorityRef: "external:google:gmail",
    proofRef: "oauth:google:gmail:cookie-session",
    lastUsed: "checked on app open",
    icon: Mail,
  },
  {
    id: "github",
    provider: "GitHub",
    product: "GitHub",
    account: "Not connected",
    broker: "edgerun.tech",
    status: "available",
    risk: "medium",
    scopes: ["repo.metadata", "pull_requests.read", "issues.read"],
    route: "GitHub OAuth → edgerun.tech callback → repo/app capabilities",
    appAccess: ["Read repository metadata", "Read pull requests", "Read issues", "Write access must be granted separately"],
    authorityRef: "external:github:pending",
    proofRef: "oauth:github:pending",
    lastUsed: "never",
    icon: GitBranch,
  },
]

const initialRecords: TrustRecord[] = [
  {
    id: "gmail-capability",
    title: "Gmail capability",
    subtitle: "Google Mail through edgerun.tech",
    status: "active",
    risk: "medium",
    capability: "gmail.read + gmail.send",
    detail: "Gmail app receives scoped access. Raw Google tokens are not handed to arbitrary apps.",
    authorityRef: "cap:gmail-app:v1",
    proofRef: "external:google:gmail",
    icon: Mail,
  },
  {
    id: "github-route",
    title: "GitHub release route",
    subtitle: "GitHub → Build Authority → Release Registry",
    status: "review",
    risk: "medium",
    capability: "repo.metadata + release.sign",
    detail: "Route is ready conceptually, but GitHub connection still needs real OAuth wiring.",
    authorityRef: "route:release-signing:v2",
    proofRef: "external:github:pending",
    icon: GitBranch,
  },
  {
    id: "invoice-route",
    title: "Invoice automation route",
    subtitle: "Gmail → Invoice Agent → Accounting",
    status: "active",
    risk: "medium",
    capability: "gmail.read + accounting.write",
    detail: "Only invoice-like messages should be processed. Sending, deleting, and payment approval are excluded.",
    authorityRef: "route:invoice-agent:v5",
    proofRef: "event:invoice-route-commit",
    icon: Route,
  },
]

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

function statusClass(status: Status | Risk) {
  switch (status) {
    case "connected":
    case "active":
    case "verified":
    case "low":
      return "border-[var(--status-online)]/25 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    case "available":
      return "border-primary/25 bg-primary/10 text-primary"
    case "review":
    case "medium":
      return "border-[var(--status-warning)]/25 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
    case "revoked":
    case "high":
      return "border-[var(--status-error)]/25 bg-[var(--status-error)]/10 text-[var(--status-error)]"
  }
}

function Badge({ value }: { value: Status | Risk }) {
  return <span className={cn("rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(value))}>{value}</span>
}

function ProviderIcon({ connection }: { connection: Connection }) {
  const Icon = connection.icon
  if (connection.provider === "Google") return <GoogleMark className="h-5 w-5" />
  return <Icon className="h-5 w-5" />
}

function ConnectionRow({ connection, active, onSelect }: { connection: Connection; active: boolean; onSelect: () => void }) {
  return (
    <button onClick={onSelect} className={cn("flex w-full items-start gap-3 border-b border-border px-3 py-3 text-left transition hover:bg-secondary/60", active && "bg-primary/10")}>
      <div className={cn("flex h-10 w-10 items-center justify-center rounded-xl border", statusClass(connection.status))}>
        <ProviderIcon connection={connection} />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-semibold text-foreground">{connection.provider}</span>
          <Badge value={connection.status} />
        </div>
        <div className="mt-0.5 truncate text-xs text-muted-foreground">{connection.product} via {connection.broker}</div>
        <div className="mt-1 truncate text-[11px] text-muted-foreground/70">{connection.scopes.join(" · ")}</div>
      </div>
    </button>
  )
}

function RecordRow({ record }: { record: TrustRecord }) {
  const Icon = record.icon
  return (
    <div className="rounded-xl border border-border bg-card p-3">
      <div className="flex items-start gap-3">
        <div className={cn("flex h-9 w-9 items-center justify-center rounded-lg border", statusClass(record.status))}><Icon className="h-4 w-4" /></div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2"><h3 className="truncate text-sm font-semibold">{record.title}</h3><Badge value={record.status} /><Badge value={record.risk} /></div>
          <p className="mt-0.5 text-xs text-muted-foreground">{record.subtitle}</p>
          <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{record.detail}</p>
          <div className="mt-2 grid gap-1 font-mono text-[10px] text-muted-foreground">
            <div className="truncate">authority: <span className="text-foreground">{record.authorityRef}</span></div>
            <div className="truncate">proof: <span className="text-foreground">{record.proofRef}</span></div>
          </div>
        </div>
      </div>
    </div>
  )
}

export function TrustManagerSurface() {
  const [section, setSection] = React.useState<Section>("connections")
  const [connections, setConnections] = React.useState(initialConnections)
  const [selectedId, setSelectedId] = React.useState(connections[0]?.id ?? "")
  const [message, setMessage] = React.useState("External connections are provider accounts mapped into bounded Edgerun capabilities.")
  const selected = connections.find((item) => item.id === selectedId) ?? connections[0]

  function updateSelected(patch: Partial<Connection>) {
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
      lastUsed: "just now",
      authorityRef: "external:github:repo-metadata",
      proofRef: "oauth:github:pending-real-callback",
    })
    setMessage("GitHub connection drafted locally. Next step is wiring the real OAuth route.")
  }

  function revokeSelected() {
    updateSelected({ status: "revoked", lastUsed: "revoked locally" })
    setMessage(`${selected.provider} marked revoked locally.`)
  }

  function reviewSelected() {
    updateSelected({ status: selected.status === "review" ? "connected" : "review", lastUsed: "reviewed just now" })
    setMessage(`${selected.provider} review state updated.`)
  }

  return (
    <div className="flex h-full min-h-0 bg-background text-foreground">
      <aside className="flex w-56 flex-shrink-0 flex-col border-r border-border bg-[var(--window-header)]/70">
        <div className="border-b border-border p-3">
          <div className="flex items-center gap-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/15 text-primary"><Shield className="h-5 w-5" /></div>
            <div className="min-w-0"><div className="truncate text-sm font-semibold">Trust Manager</div><div className="text-[10px] text-muted-foreground">connections · authority</div></div>
          </div>
        </div>
        <nav className="flex-1 space-y-1 p-2">
          {sections.map((item) => {
            const Icon = item.icon
            return <button key={item.id} onClick={() => setSection(item.id)} className={cn("flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-xs font-medium transition", section === item.id ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-secondary hover:text-foreground")}><Icon className="h-3.5 w-3.5" />{item.label}</button>
          })}
        </nav>
        <div className="border-t border-border p-3 text-[10px] text-muted-foreground">Provider tokens should become scoped local capabilities, not app-global secrets.</div>
      </aside>

      {section === "connections" ? (
        <>
          <section className="flex w-80 flex-shrink-0 flex-col border-r border-border">
            <div className="border-b border-border p-3"><h2 className="text-sm font-semibold">External connections</h2><p className="text-[11px] text-muted-foreground">OAuth providers and their Edgerun scopes.</p></div>
            <div className="min-h-0 flex-1 overflow-auto">{connections.map((connection) => <ConnectionRow key={connection.id} connection={connection} active={selected.id === connection.id} onSelect={() => setSelectedId(connection.id)} />)}</div>
          </section>
          <main className="flex min-w-0 flex-1 flex-col">
            <header className="flex h-14 items-center justify-between border-b border-border bg-[var(--window-header)]/40 px-4">
              <div className="flex items-center gap-3"><div className={cn("flex h-9 w-9 items-center justify-center rounded-xl border", statusClass(selected.status))}><ProviderIcon connection={selected} /></div><div><h3 className="text-sm font-semibold">{selected.provider} → {selected.product}</h3><p className="text-[11px] text-muted-foreground">through <span className="font-mono text-foreground">{selected.broker}</span></p></div></div>
              <Badge value={selected.status} />
            </header>
            <div className="min-h-0 flex-1 overflow-auto p-4">
              <div className="grid gap-4 xl:grid-cols-2">
                <section className="rounded-xl border border-border bg-card p-4"><div className="mb-3 flex items-center gap-2 text-sm font-semibold"><Link2 className="h-4 w-4 text-primary" />Connection route</div><div className="space-y-2 text-xs"><div className="flex justify-between gap-3"><span className="text-muted-foreground">Provider</span><span className="font-medium">{selected.provider}</span></div><div className="flex justify-between gap-3"><span className="text-muted-foreground">Broker</span><span className="font-mono">{selected.broker}</span></div><div className="flex justify-between gap-3"><span className="text-muted-foreground">App</span><span>{selected.product}</span></div><div className="flex justify-between gap-3"><span className="text-muted-foreground">Account</span><span>{selected.account}</span></div></div><div className="mt-3 rounded-lg border border-border bg-background/70 p-2 text-[11px] text-muted-foreground">{selected.route}</div></section>
                <section className="rounded-xl border border-border bg-card p-4"><div className="mb-3 flex items-center gap-2 text-sm font-semibold"><KeyRound className="h-4 w-4 text-primary" />Scoped access</div><div className="flex flex-wrap gap-1.5">{selected.scopes.map((scope) => <span key={scope} className="rounded-md border border-primary/20 bg-primary/10 px-2 py-1 text-[11px] text-primary">{scope}</span>)}</div><div className="mt-3 space-y-1 text-[11px] text-muted-foreground">{selected.appAccess.map((line) => <div key={line} className="flex items-center gap-2"><CheckCircle2 className="h-3.5 w-3.5 text-[var(--status-online)]" />{line}</div>)}</div></section>
                <section className="rounded-xl border border-border bg-card p-4 xl:col-span-2"><div className="mb-3 flex items-center gap-2 text-sm font-semibold"><ShieldCheck className="h-4 w-4 text-primary" />Evidence</div><div className="grid gap-2 font-mono text-[11px]"><button onClick={() => copy(selected.authorityRef, "authority ref")} className="flex items-center justify-between gap-3 rounded-lg bg-background p-2 text-left ring-1 ring-border hover:ring-primary/40"><span className="text-muted-foreground">authority</span><span className="truncate">{selected.authorityRef}</span><Copy className="h-3 w-3" /></button><button onClick={() => copy(selected.proofRef, "proof ref")} className="flex items-center justify-between gap-3 rounded-lg bg-background p-2 text-left ring-1 ring-border hover:ring-primary/40"><span className="text-muted-foreground">proof</span><span className="truncate">{selected.proofRef}</span><Copy className="h-3 w-3" /></button></div></section>
              </div>
            </div>
            <footer className="flex h-12 items-center justify-between border-t border-border bg-[var(--window-header)]/40 px-4"><p className="min-w-0 truncate text-[11px] text-muted-foreground">{message}</p><div className="flex gap-2">{selected.id === "github" && selected.status !== "connected" && selected.status !== "revoked" && <button onClick={connectGithub} className="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground"><GitBranch className="mr-1 inline h-3 w-3" />Connect GitHub</button>}<button onClick={reviewSelected} className="rounded-md bg-secondary px-3 py-1.5 text-xs font-medium hover:bg-secondary/80">Review</button><button onClick={revokeSelected} className="rounded-md bg-[var(--status-error)]/10 px-3 py-1.5 text-xs font-medium text-[var(--status-error)] hover:bg-[var(--status-error)]/20"><Trash2 className="mr-1 inline h-3 w-3" />Revoke</button></div></footer>
          </main>
        </>
      ) : (
        <main className="min-w-0 flex-1 overflow-auto p-4">
          <div className="mb-4 flex items-center justify-between"><div><h2 className="text-sm font-semibold">{sections.find((item) => item.id === section)?.label}</h2><p className="text-xs text-muted-foreground">Trust records derived from connections, capabilities, routes, and audit events.</p></div><button className="rounded-md bg-secondary px-3 py-1.5 text-xs font-medium hover:bg-secondary/80"><RefreshCw className="mr-1 inline h-3 w-3" />Refresh</button></div>
          <div className="grid gap-3 xl:grid-cols-2">{initialRecords.map((record) => <RecordRow key={record.id} record={record} />)}</div>
        </main>
      )}
    </div>
  )
}
