"use client"

import { useMemo, useState } from "react"
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Fingerprint,
  GitBranch,
  KeyRound,
  Lock,
  Network,
  Route,
  ScrollText,
  Search,
  Shield,
  SlidersHorizontal,
  XCircle,
  type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"

type TrustTab = "overview" | "capsules" | "capabilities" | "routes" | "delegations" | "events"
type Risk = "low" | "medium" | "high"
type Status = "strong" | "verified" | "review" | "danger" | "active" | "limited"

const tabs: { id: TrustTab; label: string; icon: LucideIcon }[] = [
  { id: "overview", label: "Overview", icon: Activity },
  { id: "capsules", label: "Capsules", icon: Shield },
  { id: "capabilities", label: "Capabilities", icon: SlidersHorizontal },
  { id: "routes", label: "Routes", icon: Route },
  { id: "delegations", label: "Delegations", icon: GitBranch },
  { id: "events", label: "Audit", icon: ScrollText },
]

const trustCards = [
  { label: "Root policy", value: "2-of-3", detail: "Framework · Phone · YubiKey", status: "strong" as Status, icon: Fingerprint },
  { label: "Active delegations", value: "7", detail: "2 expire soon", status: "review" as Status, icon: GitBranch },
  { label: "Policy routes", value: "12", detail: "3 need review", status: "review" as Status, icon: Route },
  { label: "Denied actions", value: "4", detail: "last 24h", status: "danger" as Status, icon: XCircle },
]

const capabilities = [
  { name: "Manage Invoices", risk: "medium" as Risk, usedBy: 2, includes: ["Read invoice emails", "Create accounting records"], excludes: ["Send email", "Approve payments"] },
  { name: "Sign Software Releases", risk: "medium" as Risk, usedBy: 1, includes: ["Sign binary", "Publish manifest"], excludes: ["Modify source", "Access user data"] },
  { name: "Update DNS Records", risk: "medium" as Risk, usedBy: 1, includes: ["Read DNS", "Update A/AAAA/CNAME"], excludes: ["Transfer domain"] },
  { name: "Approve Payments", risk: "high" as Risk, usedBy: 1, includes: ["Approve under limit", "Log payment event"], excludes: ["Change payout address"] },
]

const routes = [
  { name: "Gmail → Invoice Agent → Accounting", cap: "Manage Invoices", status: "active" as Status, approval: "Auto under $500 · Ask above", lastRun: "2 min ago" },
  { name: "GitHub → Build Authority → Release Registry", cap: "Sign Software Releases", status: "active" as Status, approval: "Auto", lastRun: "18 min ago" },
  { name: "DNS Agent → Cloudflare → edgerun.tech", cap: "Update DNS Records", status: "active" as Status, approval: "Auto", lastRun: "1 hr ago" },
  { name: "Phone Presence → Home Assistant → Gate", cap: "Control Smart Home", status: "review" as Status, approval: "Ask first", lastRun: "2 days ago" },
]

const delegations = [
  { name: "Build Authority", by: "Ken Root", expires: "Dec 31, 2026", risk: "medium" as Risk, allowed: "Sign releases · revoke bad releases" },
  { name: "DNS Authority", by: "Infrastructure Authority", expires: "Jun 1, 2026", risk: "medium" as Risk, allowed: "Update *.edgerun.tech records" },
  { name: "Payment Authority", by: "Ken Root", expires: "Mar 15, 2026", risk: "high" as Risk, allowed: "Approve payouts under $500" },
]

const events = [
  { actor: "Invoice Agent", action: "created accounting record #4821", reason: "Manage Invoices capability", status: "verified" as Status, time: "2 min ago" },
  { actor: "Build Authority", action: "signed edgerun-node v0.4.2", reason: "Build signing delegation", status: "verified" as Status, time: "18 min ago" },
  { actor: "DNS Agent", action: "updated api.edgerun.tech", reason: "DNS Authority delegation", status: "verified" as Status, time: "1 hr ago" },
  { actor: "System", action: "revoked old phone key", reason: "Manual revocation", status: "review" as Status, time: "3 hr ago" },
  { actor: "Unknown App", action: "denied private folder", reason: "No capability grant", status: "danger" as Status, time: "7 hr ago" },
]

function statusClass(status: Status | Risk) {
  switch (status) {
    case "strong":
    case "verified":
    case "active":
    case "low":
      return "border-[var(--status-online)]/25 bg-[var(--status-online)]/10 text-[var(--status-online)]"
    case "review":
    case "limited":
    case "medium":
      return "border-[var(--status-warning)]/25 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
    case "danger":
    case "high":
      return "border-[var(--status-error)]/25 bg-[var(--status-error)]/10 text-[var(--status-error)]"
  }
}

function Badge({ value }: { value: Status | Risk }) {
  return <span className={cn("rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(value))}>{value}</span>
}

function Panel({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={cn("rounded-xl border border-border bg-card/80 p-3 shadow-sm", className)}>{children}</div>
}

function Overview() {
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-4 gap-3">
        {trustCards.map((card) => {
          const Icon = card.icon
          return (
            <Panel key={card.label}>
              <div className="mb-3 flex items-center justify-between">
                <Icon className="h-4 w-4 text-primary" />
                <Badge value={card.status} />
              </div>
              <div className="text-xl font-semibold text-foreground">{card.value}</div>
              <div className="text-[11px] text-muted-foreground">{card.label}</div>
              <div className="mt-1 truncate text-[10px] text-muted-foreground/70">{card.detail}</div>
            </Panel>
          )
        })}
      </div>

      <div className="grid grid-cols-[1.2fr_0.8fr] gap-3">
        <Panel>
          <div className="mb-3 flex items-center justify-between">
            <div>
              <h3 className="text-sm font-semibold text-foreground">Trust map</h3>
              <p className="text-xs text-muted-foreground">Root → authority → app/action chain</p>
            </div>
            <Network className="h-4 w-4 text-muted-foreground" />
          </div>
          <div className="relative h-56 rounded-lg border border-border bg-background/70">
            <svg viewBox="0 0 520 220" className="h-full w-full">
              <line x1="260" y1="38" x2="150" y2="110" stroke="currentColor" className="text-border" strokeDasharray="4 4" />
              <line x1="260" y1="38" x2="370" y2="110" stroke="currentColor" className="text-border" strokeDasharray="4 4" />
              <line x1="150" y1="110" x2="100" y2="178" stroke="currentColor" className="text-border" strokeDasharray="4 4" />
              <line x1="150" y1="110" x2="210" y2="178" stroke="currentColor" className="text-border" strokeDasharray="4 4" />
              <line x1="370" y1="110" x2="430" y2="178" stroke="currentColor" className="text-border" strokeDasharray="4 4" />
              {[
                [260, 38, "Ken Root", "var(--status-online)"],
                [150, 110, "Infra", "var(--primary)"],
                [370, 110, "Build", "var(--primary)"],
                [100, 178, "DNS", "var(--status-online)"],
                [210, 178, "Server", "var(--status-online)"],
                [430, 178, "Release", "var(--status-online)"],
              ].map(([x, y, label, color]) => (
                <g key={String(label)}>
                  <circle cx={Number(x)} cy={Number(y)} r="14" fill={String(color)} opacity="0.18" stroke={String(color)} />
                  <text x={Number(x)} y={Number(y) + 32} textAnchor="middle" className="fill-muted-foreground text-[10px] font-mono">{label}</text>
                </g>
              ))}
            </svg>
          </div>
        </Panel>

        <Panel>
          <h3 className="mb-3 text-sm font-semibold text-foreground">Recent decisions</h3>
          <div className="space-y-2">
            {events.slice(0, 4).map((event) => (
              <div key={`${event.actor}-${event.time}`} className="rounded-lg border border-border/70 bg-background/50 p-2">
                <div className="flex items-center justify-between gap-2">
                  <div className="truncate text-xs font-medium text-foreground">{event.actor}</div>
                  <Badge value={event.status} />
                </div>
                <div className="mt-1 truncate text-[11px] text-muted-foreground">{event.action}</div>
                <div className="mt-1 text-[10px] text-muted-foreground/70">{event.time}</div>
              </div>
            ))}
          </div>
        </Panel>
      </div>
    </div>
  )
}

function CapabilityList() {
  return (
    <div className="grid grid-cols-2 gap-3">
      {capabilities.map((capability) => (
        <Panel key={capability.name}>
          <div className="mb-2 flex items-start justify-between gap-2">
            <div>
              <h3 className="text-sm font-semibold text-foreground">{capability.name}</h3>
              <p className="text-xs text-muted-foreground">Used by {capability.usedBy} route{capability.usedBy === 1 ? "" : "s"}</p>
            </div>
            <Badge value={capability.risk} />
          </div>
          <div className="grid gap-2 text-[11px]">
            <div>
              <div className="mb-1 text-muted-foreground">Allows</div>
              <div className="flex flex-wrap gap-1">{capability.includes.map((item) => <span key={item} className="rounded bg-[var(--status-online)]/10 px-1.5 py-0.5 text-[var(--status-online)]">{item}</span>)}</div>
            </div>
            <div>
              <div className="mb-1 text-muted-foreground">Blocks</div>
              <div className="flex flex-wrap gap-1">{capability.excludes.map((item) => <span key={item} className="rounded bg-[var(--status-error)]/10 px-1.5 py-0.5 text-[var(--status-error)]">{item}</span>)}</div>
            </div>
          </div>
        </Panel>
      ))}
    </div>
  )
}

function RoutesList() {
  return <div className="space-y-2">{routes.map((route) => <Panel key={route.name}><div className="flex items-center justify-between gap-3"><div className="min-w-0"><div className="truncate text-sm font-semibold text-foreground">{route.name}</div><div className="mt-1 flex flex-wrap gap-2 text-xs text-muted-foreground"><span>{route.cap}</span><span>·</span><span>{route.approval}</span><span>·</span><span>{route.lastRun}</span></div></div><Badge value={route.status} /></div></Panel>)}</div>
}

function DelegationsList() {
  return <div className="space-y-2">{delegations.map((delegation) => <Panel key={delegation.name}><div className="flex items-start justify-between gap-3"><div><div className="text-sm font-semibold text-foreground">{delegation.name}</div><div className="mt-1 text-xs text-muted-foreground">By {delegation.by} · Expires {delegation.expires}</div><div className="mt-2 text-[11px] text-muted-foreground">{delegation.allowed}</div></div><Badge value={delegation.risk} /></div></Panel>)}</div>
}

function EventsList() {
  return <div className="space-y-2">{events.map((event) => <Panel key={`${event.actor}-${event.action}`}><div className="flex items-start justify-between gap-3"><div><div className="text-sm font-semibold text-foreground">{event.actor}</div><div className="mt-1 text-xs text-muted-foreground">{event.action}</div><div className="mt-1 text-[11px] text-muted-foreground/70">Reason: {event.reason}</div></div><div className="text-right"><Badge value={event.status} /><div className="mt-2 text-[10px] text-muted-foreground">{event.time}</div></div></div></Panel>)}</div>
}

export function TrustManagerApp() {
  const [tab, setTab] = useState<TrustTab>("overview")
  const activeTab = useMemo(() => tabs.find((item) => item.id === tab) ?? tabs[0], [tab])

  return (
    <div className="flex h-full bg-background text-foreground">
      <aside className="flex w-52 flex-shrink-0 flex-col border-r border-border bg-[var(--window-header)]/60">
        <div className="border-b border-border p-3">
          <div className="flex items-center gap-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/15 text-primary">
              <Shield className="h-5 w-5" />
            </div>
            <div className="min-w-0">
              <div className="truncate text-sm font-semibold">Trust Manager</div>
              <div className="text-[10px] text-muted-foreground">policy · grants · audit</div>
            </div>
          </div>
        </div>
        <nav className="flex-1 space-y-1 p-2">
          {tabs.map((item) => {
            const Icon = item.icon
            return (
              <button
                key={item.id}
                onClick={() => setTab(item.id)}
                className={cn(
                  "flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-xs font-medium transition-colors",
                  tab === item.id ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-secondary hover:text-foreground",
                )}
              >
                <Icon className="h-3.5 w-3.5" />
                {item.label}
              </button>
            )
          })}
        </nav>
        <div className="border-t border-border p-3 text-[10px] text-muted-foreground">
          Local-first policy view. Stream commits become authority after validation.
        </div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-border bg-[var(--window-header)]/40 px-4 py-3">
          <div>
            <div className="flex items-center gap-2">
              <activeTab.icon className="h-4 w-4 text-primary" />
              <h2 className="text-sm font-semibold">{activeTab.label}</h2>
            </div>
            <p className="mt-0.5 text-xs text-muted-foreground">Human-readable trust, delegation, and capability policy.</p>
          </div>
          <div className="flex items-center gap-2">
            <div className="relative">
              <Search className="absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-muted-foreground" />
              <input className="h-8 w-44 rounded-md border border-border bg-background pl-7 pr-2 text-xs outline-none focus:border-primary/50" placeholder="Search trust..." />
            </div>
            <button className="flex h-8 items-center gap-1.5 rounded-md bg-secondary px-2 text-xs font-medium hover:bg-secondary/80">
              <Lock className="h-3.5 w-3.5" />
              Export
            </button>
          </div>
        </header>

        <div className="min-h-0 flex-1 overflow-auto p-4">
          {tab === "overview" && <Overview />}
          {tab === "capsules" && <Overview />}
          {tab === "capabilities" && <CapabilityList />}
          {tab === "routes" && <RoutesList />}
          {tab === "delegations" && <DelegationsList />}
          {tab === "events" && <EventsList />}
        </div>
      </main>
    </div>
  )
}
