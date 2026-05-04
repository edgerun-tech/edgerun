"use client"

import { useMemo } from "react"
import { useStore } from "@nanostores/react"
import {
  Activity, AlertTriangle, ArrowRight, BarChart2, Bell, Check, CheckCircle,
  ChevronRight, Clock, Copy, Database, Download, Eye, FileCheck, Fingerprint,
  GitBranch, Globe, HardDrive, Key, Laptop, Layers, Lock, Menu, Network,
  Package, Plus, Route, ScrollText, Search, Server, Settings, Shield,
  Smartphone, Trash2, Upload, X, Zap,
  type LucideIcon,
} from "lucide-react"
import {
  trustManagerStore,
  setPage,
  toggleSidebar,
  setSelectedCapsule,
  toggleSelectedRoute,
  toggleSelectedCapability,
  setSelectedTrustMapNode,
  setRootStep,
  toggleSimulating,
  setInspectorResult,
  setImporting,
  enterDashboard,
  type TrustPage,
} from "@/stores/trust-manager-store"

type Risk = "low" | "medium" | "high"
type Status = "strong" | "verified" | "limited" | "imported" | "active" | "review" | "danger" | "unknown" | "signed" | Risk

type EventRow = {
  actor: string
  action: string
  target: string
  reason: string
  time: string
  status: "ok" | "warn" | "danger"
  icon: LucideIcon
}

type Capsule = {
  name: string
  type: string
  policy?: string
  status: Status
  storage: string
  trustedFor?: string
}

type Capability = {
  name: string
  risk: Risk
  usedBy: number
  desc: string
  includes: string[]
  excludes: string[]
}

type RouteDef = {
  name: string
  source: string
  dest: string
  executor: string
  cap: string
  risk: Risk
  status: "active" | "review"
  lastRun: string
  events: number
  approval: string
}

type Delegation = {
  name: string
  risk: Risk
  allowed: string[]
  denied: string[]
  by: string
  expires: string
}

const events: EventRow[] = [
  { actor: "Invoice Agent", action: "created", target: "accounting record #4821", reason: "Manage Invoices capability", time: "2 min ago", status: "ok", icon: FileCheck },
  { actor: "Build Authority", action: "signed", target: "edgerun-node v0.4.2", reason: "Build signing delegation", time: "18 min ago", status: "ok", icon: Package },
  { actor: "DNS Agent", action: "updated", target: "api.edgerun.tech", reason: "DNS Authority delegation", time: "1 hr ago", status: "ok", icon: Globe },
  { actor: "System", action: "revoked", target: "old phone key", reason: "Manual revocation by Ken Root", time: "3 hr ago", status: "warn", icon: X },
  { actor: "Unknown App", action: "denied", target: "private folder", reason: "No capability granted", time: "7 hr ago", status: "danger", icon: AlertTriangle },
]

const capsules: Capsule[] = [
  { name: "Ken Personal", type: "Personal root", policy: "2-of-3 devices", status: "strong", storage: "Local file + phone + encrypted backup" },
  { name: "EdgeRun Organization", type: "Organization", policy: "Board authority + build authority", status: "verified", storage: "Public URL" },
  { name: "Public Web PKI", type: "External trust root", status: "imported", storage: "Browser bundle", trustedFor: "Website certificates" },
  { name: "GitHub Releases", type: "External identity", status: "limited", storage: "Imported", trustedFor: "Software release metadata" },
]

const capabilities: Capability[] = [
  { name: "Manage Invoices", risk: "medium", usedBy: 2, desc: "Process invoices from email to accounting records safely.", includes: ["Read invoice emails", "Extract invoice fields", "Create accounting records", "Attach source hash"], excludes: ["Send emails", "Delete emails", "Approve payments"] },
  { name: "Sign Software Releases", risk: "medium", usedBy: 1, desc: "Authorize and sign versioned software artifacts.", includes: ["Sign release binary", "Publish manifest", "Revoke release"], excludes: ["Modify source code", "Access user data"] },
  { name: "Update DNS Records", risk: "medium", usedBy: 1, desc: "Manage DNS entries under delegated zones.", includes: ["Read DNS records", "Update A/AAAA", "Update CNAME"], excludes: ["Transfer domain", "Change registrar"] },
  { name: "Run Compute Jobs", risk: "low", usedBy: 3, desc: "Execute sandboxed WASM workloads on authorized nodes.", includes: ["Read job inputs", "Execute WASM", "Submit signed results"], excludes: ["Host filesystem", "Outbound network"] },
  { name: "Approve Payments", risk: "high", usedBy: 1, desc: "Authorize outbound payments within configured limits.", includes: ["Approve payout under $500", "Log payment event"], excludes: ["Change payout address", "Withdraw treasury"] },
]

const routes: RouteDef[] = [
  { name: "Gmail → Invoice Agent → Accounting", source: "Gmail", dest: "Accounting DB", executor: "Ken Laptop Agent", cap: "Manage Invoices", risk: "medium", status: "active", lastRun: "2 min ago", events: 412, approval: "Auto under $500 · Ask above" },
  { name: "GitHub → Build Authority → Release Registry", source: "GitHub", dest: "Release Registry", executor: "Build Authority", cap: "Sign Software Releases", risk: "medium", status: "active", lastRun: "18 min ago", events: 87, approval: "Auto" },
  { name: "DNS Agent → Cloudflare → edgerun.tech", source: "DNS Manager", dest: "Cloudflare", executor: "DNS Agent", cap: "Update DNS Records", risk: "medium", status: "active", lastRun: "1 hr ago", events: 34, approval: "Auto" },
  { name: "Phone Presence → Home Assistant → Gate", source: "Phone GPS", dest: "Home Assistant", executor: "Local Agent", cap: "Control Smart Home", risk: "medium", status: "review", lastRun: "2 days ago", events: 22, approval: "Ask first" },
]

const delegations: Delegation[] = [
  { name: "Build Authority", risk: "medium", by: "Ken Root", expires: "Dec 31, 2026", allowed: ["Sign software releases", "Approve WASM runtime versions", "Revoke compromised releases"], denied: ["Access user data", "Change payment settings", "Add root devices"] },
  { name: "DNS Authority", risk: "medium", by: "Infrastructure Authority", expires: "Jun 1, 2026", allowed: ["Update records under *.edgerun.tech"], denied: ["Transfer domain", "Change payment settings"] },
  { name: "Payment Authority", risk: "high", by: "Ken Root", expires: "Mar 15, 2026", allowed: ["Approve payouts under $500"], denied: ["Change payout address", "Withdraw treasury funds"] },
]

const compat = [
  ["OpenID Connect / OAuth", Globe, "connected"],
  ["TLS / mTLS", Lock, "connected"],
  ["SSH", Key, "connected"],
  ["Git Signing", GitBranch, "connected"],
  ["Kubernetes", Layers, "available"],
  ["Cloudflare", Globe, "connected"],
  ["Hardware Keys", Fingerprint, "connected"],
  ["Public Web PKI", Shield, "imported"],
] as const

const nav = [
  ["dashboard", "Dashboard", BarChart2],
  ["capsules", "Capsules", Shield],
  ["root", "Root", Fingerprint],
  ["map", "Trust Map", Network],
  ["caps", "Capabilities", Layers],
  ["policy", "Policy", ScrollText],
  ["routes", "Routes", Route],
  ["delegations", "Delegations", GitBranch],
  ["inspector", "Inspector", Search],
  ["compat", "Compatibility", Globe],
  ["storage", "Storage", Settings],
] as const

function badgeClass(status: Status | "ok" | "warn") {
  const map: Record<string, string> = {
    strong: "bg-emerald-500/15 text-emerald-300 border-emerald-500/30",
    verified: "bg-blue-500/15 text-blue-300 border-blue-500/30",
    imported: "bg-slate-500/15 text-slate-300 border-slate-500/30",
    limited: "bg-amber-500/15 text-amber-300 border-amber-500/30",
    active: "bg-emerald-500/15 text-emerald-300 border-emerald-500/30",
    review: "bg-amber-500/15 text-amber-300 border-amber-500/30",
    danger: "bg-red-500/15 text-red-300 border-red-500/30",
    unknown: "bg-slate-600/30 text-slate-400 border-slate-600/40",
    signed: "bg-emerald-500/15 text-emerald-300 border-emerald-500/30",
    low: "bg-emerald-500/15 text-emerald-300 border-emerald-500/30",
    medium: "bg-amber-500/15 text-amber-300 border-amber-500/30",
    high: "bg-red-500/15 text-red-300 border-red-500/30",
    ok: "bg-emerald-500/15 text-emerald-300 border-emerald-500/30",
    warn: "bg-amber-500/15 text-amber-300 border-amber-500/30",
  }
  return map[status] ?? "bg-slate-700/50 text-slate-300 border-slate-600/40"
}

function Badge({ children, status }: { children: React.ReactNode; status: Status | "ok" | "warn" }) {
  return <span className={`inline-flex items-center rounded border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${badgeClass(status)}`}>{children}</span>
}

function Card({ children, className = "", onClick }: { children: React.ReactNode; className?: string; onClick?: () => void }) {
  return <div onClick={onClick} className={`rounded-2xl border border-slate-700/60 bg-slate-800/55 p-5 ${onClick ? "cursor-pointer transition hover:border-slate-600 hover:bg-slate-800/80" : ""} ${className}`}>{children}</div>
}

function Button({ children, icon: Icon, onClick, danger }: { children: React.ReactNode; icon?: LucideIcon; onClick?: () => void; danger?: boolean }) {
  return <button onClick={onClick} className={`inline-flex items-center gap-2 rounded-lg border px-3 py-2 text-xs font-medium transition ${danger ? "border-red-500/40 bg-red-600/15 text-red-300 hover:bg-red-600/25" : "border-slate-600/60 text-slate-300 hover:bg-slate-700/60 hover:text-slate-100"}`}>{Icon && <Icon size={13} />}{children}</button>
}

function Section({ icon: Icon, title, subtitle, children, actions }: { icon: LucideIcon; title: string; subtitle?: string; children: React.ReactNode; actions?: React.ReactNode }) {
  return <section className="space-y-5"><div className="flex items-start justify-between gap-4"><div><div className="flex items-center gap-2"><Icon size={18} className="text-slate-400" /><h1 className="text-xl font-semibold text-slate-100">{title}</h1></div>{subtitle && <p className="ml-7 mt-1 text-sm text-slate-500">{subtitle}</p>}</div>{actions}</div>{children}</section>
}

function TrustSentence({ children }: { children: React.ReactNode }) {
  return <div className="rounded-xl border border-slate-600/40 bg-slate-700/35 p-3 text-sm leading-relaxed text-slate-300"><span className="mb-1 block text-[10px] font-semibold uppercase tracking-widest text-slate-500">Why trusted</span>{children}</div>
}

function Landing({ enter }: { enter: () => void }) {
  return <div className="flex min-h-screen flex-col bg-slate-950 text-slate-200"><main className="flex flex-1 flex-col items-center justify-center px-8 text-center"><div className="mb-8 inline-flex items-center gap-2 rounded-full border border-emerald-500/20 bg-emerald-500/10 px-4 py-1.5"><Shield size={14} className="text-emerald-400" /><span className="text-xs font-medium text-emerald-300">Human-readable digital trust</span></div><h1 className="mb-4 font-mono text-6xl font-bold tracking-tighter text-slate-50">Trust Manager</h1><p className="mb-3 max-w-2xl text-xl text-slate-400">A human-readable policy layer for digital trust.</p><p className="mb-10 max-w-xl text-sm text-slate-500">Create portable Trust Capsules, delegate authority, route data between services, inspect trust chains, and keep signed proof of what happened.</p><div className="flex flex-wrap justify-center gap-3"><button onClick={enter} className="flex items-center gap-2 rounded-xl bg-emerald-600 px-6 py-3 text-sm font-medium text-white transition hover:bg-emerald-500"><Plus size={16} />Create Trust Capsule</button><button onClick={enter} className="flex items-center gap-2 rounded-xl border border-slate-700 bg-slate-800 px-6 py-3 text-sm font-medium text-slate-300 transition hover:bg-slate-700"><Eye size={16} />Inspect Existing Trust</button></div></main><div className="mx-auto grid w-full max-w-5xl grid-cols-2 gap-4 px-8 pb-16 md:grid-cols-4">{[[Shield,"Portable Capsules"],[Layers,"Capability Permissions"],[Network,"Visual Trust Chains"],[Route,"Policy Routes"],[ScrollText,"Signed Logs"],[Globe,"OIDC Compatible"],[Lock,"No Server Secrets"],[Fingerprint,"Root Wizard"]].map(([Icon,label]) => { const I = Icon as LucideIcon; return <div key={String(label)} className="rounded-xl border border-slate-800 bg-slate-900/60 p-4 text-center"><div className="mx-auto mb-3 flex h-9 w-9 items-center justify-center rounded-lg bg-slate-800"><I size={16} className="text-emerald-400" /></div><div className="text-sm font-medium text-slate-200">{label}</div></div> })}</div></div>
}

function Dashboard() {
  return <Section icon={BarChart2} title="Dashboard" subtitle="See who can do what, why they are trusted, and what happened."><div className="grid grid-cols-2 gap-4 lg:grid-cols-4">{[["Trust Capsules","3","2 warnings",Shield],["Root Strength","2-of-3","Strong",Fingerprint],["Active Routes","12","3 need review",Route],["Events Today","143","All signed",ScrollText]].map(([label,value,sub,Icon]) => { const I = Icon as LucideIcon; return <Card key={String(label)}><I size={18} className="mb-3 text-emerald-400" /><div className="text-2xl font-bold text-slate-100">{value}</div><div className="text-xs text-slate-500">{label}</div><div className="text-xs text-slate-600">{sub}</div></Card> })}</div><div className="grid gap-6 lg:grid-cols-3"><Card><h3 className="mb-4 flex items-center gap-2 text-sm font-semibold text-slate-300"><Shield size={14} className="text-emerald-400" />Trust Health</h3>{[["Root policy","Strong","ok"],["Recovery","Configured","ok"],["Revoked keys","1","warn"],["Unknown services","2","danger"],["Expiring delegations","4","warn"]].map(([label,value,status]) => <div key={label} className="mb-3 flex justify-between text-sm"><span className="text-slate-400">{label}</span><span className="font-medium text-slate-200">{value}</span></div>)}</Card><Card><h3 className="mb-4 flex items-center gap-2 text-sm font-semibold text-slate-300"><Zap size={14} className="text-amber-400" />Quick Actions</h3>{["Review 3 routes", "4 delegations expiring", "2 unknown services", "Export Trust Capsule"].map(item => <div key={item} className="flex items-center gap-2 rounded-lg px-3 py-2 text-sm text-slate-400 hover:bg-slate-700/40 hover:text-slate-200"><ChevronRight size={12} />{item}</div>)}</Card><Card><h3 className="mb-4 flex items-center gap-2 text-sm font-semibold text-slate-300"><Activity size={14} className="text-blue-400" />Root Evolution</h3>{["Created root", "Added Android Phone", "Changed policy to 2-of-2", "Added YubiKey", "Changed policy to 2-of-3", "Removed old phone"].map(item => <div key={item} className="mb-2 flex gap-2 text-xs text-slate-400"><span className="mt-1.5 h-1.5 w-1.5 rounded-full bg-emerald-400" />{item}</div>)}</Card></div><div className="space-y-2">{events.map((ev, i) => { const I = ev.icon; return <div key={ev.target} onClick={() => setOpen(open === i ? null : i)} className="cursor-pointer rounded-xl border border-slate-700/50 bg-slate-800/50 px-4 py-3 transition hover:bg-slate-800"><div className="flex items-center gap-3"><div className="flex h-7 w-7 items-center justify-center rounded-lg bg-slate-700/60"><I size={13} className={ev.status === "danger" ? "text-red-400" : ev.status === "warn" ? "text-amber-400" : "text-emerald-400"} /></div><div className="min-w-0 flex-1 text-sm"><span className="font-medium text-slate-200">{ev.actor}</span><span className="text-slate-400"> {ev.action} </span><span className="text-slate-200">{ev.target}</span></div><span className="text-xs text-slate-600">{ev.time}</span><Badge status={ev.status === "ok" ? "ok" : ev.status === "warn" ? "warn" : "danger"}>{ev.status}</Badge></div>{open === i && <div className="ml-10 mt-3 border-t border-slate-700/50 pt-3 text-xs text-slate-400">Reason: {ev.reason}<br />Trust: Signed · Logged · Verifiable</div>}</div> })}</div></Section>
}

function Capsules() {
  const { importing } = useStore(trustManagerStore)
  return <Section icon={Shield} title="Trust Capsules" subtitle="Portable containers that describe your trust world." actions={<Button icon={Plus} onClick={() => setImporting(!importing)}>Load Capsule</Button>}>{importing && <Card><h3 className="mb-2 font-semibold text-slate-200">This capsule claims to be EdgeRun Organization</h3><p className="mb-4 text-sm text-slate-400">Verified by domain proof, GitHub proof, and public key fingerprint. Choose what to trust it for.</p><div className="mb-4 flex flex-wrap gap-2">{["View only","Software releases","Server certificates","Messages","Payments","Node jobs","Custom"].map(x => <Badge key={x} status="limited">{x}</Badge>)}</div><Button icon={Check} onClick={() => setImporting(false)}>Import selected trust</Button></Card>}<div className="grid gap-4 lg:grid-cols-2">{capsules.map(c => <Card key={c.name}><div className="mb-3 flex items-start justify-between"><div><h3 className="font-semibold text-slate-100">{c.name}</h3><p className="text-xs text-slate-500">{c.type}</p></div><Badge status={c.status}>{c.status}</Badge></div>{c.policy && <p className="text-xs text-slate-400"><span className="text-slate-600">Root policy:</span> {c.policy}</p>}{c.trustedFor && <p className="text-xs text-slate-400"><span className="text-slate-600">Trusted for:</span> {c.trustedFor}</p>}<p className="mb-4 text-xs text-slate-500"><span className="text-slate-600">Storage:</span> {c.storage}</p><div className="flex flex-wrap gap-2"><Button icon={Eye}>Open</Button><Button icon={Download}>Export</Button><Button icon={Globe}>Publish</Button><Button icon={Lock} danger>Revoke</Button></div></Card>)}</div></Section>
}

function Root() {
  const { rootStep: step } = useStore(trustManagerStore)
  const steps = ["Start", "Devices", "Rule", "Delegations", "Review"]
  return <Section icon={Fingerprint} title="Root of Trust" subtitle="Start with one device, then upgrade to stronger roots over time."><div className="flex gap-2 overflow-x-auto">{steps.map((s,i) => <button key={s} onClick={() => setRootStep(i)} className={`rounded-xl border px-4 py-2 text-sm ${i === step ? "border-emerald-500/30 bg-emerald-600/15 text-emerald-300" : "border-slate-700 bg-slate-800/50 text-slate-500"}`}>{i + 1}. {s}</button>)}</div><Card>{step === 0 && <RootStep title="Create your Root" text="This is the source of authority for your digital life. It can approve devices, apps, servers, software, recovery, and delegations." items={["This device only", "This device + phone", "Hardware key", "Multiple devices", "Social recovery"]} />}{step === 1 && <RootStep title="Trusted devices" text="Choose devices or keys that can approve important changes." items={["Framework Laptop", "Android Phone", "YubiKey", "Recovery Phrase", "Guardian"]} />}{step === 2 && <RootStep title="Approval rule" text="Important changes require approval from any 2 of 3 trusted devices." items={["Any one device", "Two trusted devices", "Main device + recovery", "Custom rule"]} />}{step === 3 && <RootStep title="First delegations" text="Delegate limited authority instead of using root directly." items={["Daily Device Key", "App Approval Key", "Infrastructure Key", "Build Signing Key", "Recovery Key", "Payment Key"]} />}{step === 4 && <div><div className="mb-5 flex items-center gap-3"><Shield className="text-emerald-400" /><div><h3 className="font-semibold text-slate-100">Root created</h3><p className="text-sm text-slate-400">2-of-3 devices · Strong · Recovery configured</p></div></div><TrustSentence>Root is not one permanent key. It is the latest valid state of your identity according to its signed history.</TrustSentence><div className="mt-4 flex gap-2"><Button icon={Download}>Export Trust Capsule</Button><Button icon={Database}>Add backup</Button></div></div>}<div className="mt-5 flex justify-between"><Button onClick={() => setRootStep(Math.max(0, step - 1))}>Back</Button><Button onClick={() => setRootStep(Math.min(4, step + 1))}>Next</Button></div></Card></Section>
}

function RootStep({ title, text, items }: { title: string; text: string; items: string[] }) {
  return <div><h3 className="mb-1 font-semibold text-slate-100">{title}</h3><p className="mb-4 text-sm text-slate-400">{text}</p><div className="grid gap-3 sm:grid-cols-2">{items.map(item => <button key={item} className="rounded-xl border border-slate-700/50 bg-slate-700/25 p-4 text-left text-sm text-slate-300 transition hover:border-emerald-500/30 hover:bg-emerald-600/10"><Check className="mb-2 text-emerald-400" size={14} />{item}</button>)}</div></div>
}

function TrustMap() {
  const { selectedTrustMapNode: selected } = useStore(trustManagerStore)
  const nodes = [["root",260,40,"Ken Root","#10b981"],["infra",130,120,"Infrastructure","#3b82f6"],["build",390,120,"Build","#3b82f6"],["dns",70,215,"DNS Agent","#6ee7b7"],["server",210,215,"api.edgerun.tech","#6ee7b7"],["tls",160,315,"TLS Cert","#6ee7b7"],["release",430,250,"edgerun-node","#6ee7b7"],["runtime",430,345,"Runtime","#6ee7b7"],["unknown",300,390,"Unknown App","#ef4444"]] as const
  const edges = [["root","infra"],["root","build"],["infra","dns"],["infra","server"],["server","tls"],["build","release"],["release","runtime"]]
  const byId = Object.fromEntries(nodes.map(n => [n[0], n]))
  return <Section icon={Network} title="Trust Map" subtitle="Click any node to inspect why it is trusted."><div className="grid gap-4 lg:grid-cols-[1fr_320px]"><Card className="p-0"><svg viewBox="0 0 540 460" className="min-h-[430px] w-full rounded-2xl bg-slate-900/50">{edges.map(([a,b]) => { const from = byId[a], to = byId[b]; return <line key={`${a}-${b}`} x1={from[1]} y1={from[2]} x2={to[1]} y2={to[2]} stroke="#334155" strokeWidth="1.5" strokeDasharray="4 3" /> })}{nodes.map(([id,x,y,label,color]) => <g key={id} onClick={() => setSelectedTrustMapNode(id)} className="cursor-pointer"><circle cx={x} cy={y} r={selected === id ? 22 : 16} fill={selected === id ? color : "#1e293b"} stroke={color} strokeWidth="1.5" /><text x={x} y={y + 34} textAnchor="middle" fill="#94a3b8" fontSize="10" fontFamily="monospace">{label}</text></g>)}</svg></Card><Card><Badge status={selected === "unknown" ? "unknown" : "strong"}>{selected === "unknown" ? "unknown" : "trusted"}</Badge><h3 className="mt-3 font-semibold text-slate-100">{selected === "server" ? "api.edgerun.tech Server Key" : selected === "unknown" ? "Unknown App" : byId[selected]?.[3] ?? "Trust Node"}</h3><div className="mt-4"><TrustSentence>{selected === "unknown" ? "This app is not trusted because it was signed by a key you have never approved." : "This node is trusted because the chain from Ken Root to its delegated authority is valid and has not been revoked."}</TrustSentence></div><div className="mt-4 space-y-2 text-xs text-slate-400"><div><span className="text-slate-600">Authority:</span> scoped capability grant</div><div><span className="text-slate-600">Proof:</span> signed delegation chain</div><div><span className="text-slate-600">History:</span> all changes logged</div></div></Card></div></Section>
}

function Capabilities() {
  const { selectedCapability: selected } = useStore(trustManagerStore)
  return <Section icon={Layers} title="Capabilities" subtitle="Named permission bundles. Internally these map to provider apps and concrete actions." actions={<Button icon={Plus}>Create capability</Button>}><div className="grid gap-4 lg:grid-cols-2">{capabilities.map((c,i) => <Card key={c.name} onClick={() => toggleSelectedCapability(i)} className={selected === i ? "border-emerald-500/40" : ""}><div className="mb-2 flex justify-between gap-3"><h3 className="font-semibold text-slate-200">{c.name}</h3><div className="flex gap-2"><Badge status={c.risk}>{c.risk}</Badge><span className="text-xs text-slate-500">{c.usedBy} routes</span></div></div><p className="text-xs text-slate-400">{c.desc}</p>{selected === i && <div className="mt-4 grid gap-3 border-t border-slate-700/50 pt-3 text-xs"><TagList title="Includes" items={c.includes} ok /><TagList title="Excludes" items={c.excludes} /></div>}</Card>)}</div></Section>
}

function TagList({ title, items, ok }: { title: string; items: string[]; ok?: boolean }) {
  return <div><div className="mb-1 text-[10px] font-semibold uppercase tracking-wider text-slate-500">{title}</div><div className="flex flex-wrap gap-1">{items.map(item => <span key={item} className={`rounded px-2 py-0.5 text-xs ${ok ? "bg-emerald-500/10 text-emerald-300" : "bg-red-500/10 text-red-300"}`}>{item}</span>)}</div></div>
}

function Policy() {
  const { simulating: sim } = useStore(trustManagerStore)
  return <Section icon={ScrollText} title="Policy Builder" subtitle="Plain-language if/then rules compiled into capability policy."><div className="grid gap-6 lg:grid-cols-2"><Card><h3 className="mb-4 text-sm font-semibold text-slate-300">Rule sentence</h3>{[["WHEN","a new email arrives"],["IF","it is from a billing address and has an invoice PDF"],["ALLOW","Invoice Agent to create an accounting record"],["USING","Ken Laptop Agent"],["THEN","save a signed log entry"]].map(([k,v]) => <div key={k} className="mb-3 flex gap-3"><span className="w-14 pt-2.5 font-mono text-xs text-slate-500">{k}</span><input defaultValue={v} className="flex-1 rounded-lg border border-slate-600/40 bg-slate-700/40 px-3 py-2.5 text-sm text-slate-200 outline-none focus:border-emerald-500/50" /></div>)}<Button icon={Check}>Save rule</Button><span className="ml-2"><Button icon={Activity} onClick={toggleSimulating}>Simulate</Button></span></Card><div className="space-y-4"><Card><h3 className="mb-3 text-sm font-semibold text-slate-300">Risk lanes</h3>{[["Auto-allow","Known vendor under $500","strong"],["Ask first","New vendor or $500-$2,000","limited"],["Block","Payment requested or over $2,000","danger"],["Log only","Simulation mode","verified"]].map(([a,b,s]) => <div key={a} className="mb-2 flex justify-between rounded-xl border border-slate-700/50 bg-slate-700/20 p-3 text-sm"><span className="text-slate-300">{a}</span><span className="text-xs text-slate-500">{b}</span><Badge status={s as Status}>{s}</Badge></div>)}</Card>{sim && <Card><h3 className="mb-3 text-sm font-semibold text-blue-300">Simulation: last 7 days</h3>{[["Emails processed",17],["Records created",15],["Approval requested",2],["Blocked",0],["Personal emails read",0]].map(([k,v]) => <div key={String(k)} className="mb-2 flex justify-between text-sm"><span className="text-slate-400">{k}</span><span className="font-semibold text-slate-200">{v}</span></div>)}</Card>}</div></div></Section>
}

function Routes() {
  const [selected, setSelected] = useState(0)
  return <Section icon={Route} title="Routes" subtitle="Visible policy paths: source → executor → destination." actions={<Button icon={Plus}>New route</Button>}><div className="space-y-3">{routes.map((r,i) => <Card key={r.name} onClick={() => setSelected(selected === i ? -1 : i)}><div className="flex items-center gap-4"><div className={`h-10 w-1.5 rounded-full ${r.status === "active" ? "bg-emerald-500" : "bg-amber-500"}`} /><div className="min-w-0 flex-1"><div className="mb-1 flex flex-wrap items-center gap-2 text-sm"><span className="font-medium text-slate-200">{r.source}</span><ArrowRight size={12} className="text-slate-600" /><span className="text-slate-400">{r.executor}</span><ArrowRight size={12} className="text-slate-600" /><span className="text-slate-300">{r.dest}</span></div><div className="flex flex-wrap gap-2"><Badge status={r.status}>{r.status}</Badge><Badge status={r.risk}>{r.risk}</Badge><span className="text-xs text-slate-500">{r.cap}</span></div></div><div className="text-right text-xs text-slate-600"><div>{r.events} events</div><div>{r.lastRun}</div></div></div>{selected === i && <div className="mt-4 border-t border-slate-700/50 pt-4"><TrustSentence>This route allows {r.executor} to process data from {r.source} and write to {r.dest}. Approval mode: {r.approval}.</TrustSentence></div>}</Card>)}</div></Section>
}

function Delegations() {
  return <Section icon={GitBranch} title="Delegations" subtitle="Bounded authority transferred from one principal to another." actions={<Button icon={Plus}>Create delegation</Button>}><div className="grid gap-4 lg:grid-cols-2">{delegations.map(d => <Card key={d.name}><div className="mb-3 flex justify-between"><h3 className="font-semibold text-slate-200">{d.name}</h3><Badge status={d.risk}>{d.risk}</Badge></div><p className="mb-3 text-xs text-slate-500">Approved by <span className="text-slate-300">{d.by}</span> · Expires <span className="text-slate-300">{d.expires}</span></p><TagList title="Allowed" items={d.allowed} ok /><div className="mt-3"><TagList title="Not allowed" items={d.denied} /></div></Card>)}</div></Section>
}

function Inspector() {
  const { inspectorResult: result } = useStore(trustManagerStore)
  const risky = result === "risky"
  return <Section icon={Search} title="Inspector" subtitle="Drop or paste any artifact to explain its trust chain.">{!result ? <Card className="border-dashed py-12 text-center"><Upload className="mx-auto mb-4 text-slate-400" /><h3 className="mb-2 font-semibold text-slate-200">Inspect anything</h3><p className="mx-auto mb-6 max-w-md text-sm text-slate-400">Certificate, JWT, binary, signed release, Trust Capsule, SSH key, TLS chain, app manifest, or event log.</p><div className="flex justify-center gap-2"><Button icon={Upload} onClick={() => setInspectorResult("safe")}>Demo signed release</Button><Button icon={AlertTriangle} onClick={() => setInspectorResult("risky")}>Demo risky update</Button></div></Card> : <div className="space-y-4"><div className="flex justify-between"><div><h3 className="font-semibold text-slate-100">{risky ? "Unknown Publisher v2.1" : "edgerun-node v0.4.2"}</h3><p className="text-xs text-slate-500">{risky ? "Authority drift detected" : "Signed software release"}</p></div><div className="flex gap-2"><Badge status={risky ? "high" : "low"}>{risky ? "high risk" : "low risk"}</Badge><Button icon={X} onClick={() => setInspectorResult(null)}>Clear</Button></div></div><TrustSentence>{risky ? "This update is signed by a key you have never approved and asks for network access plus local file writes." : "This app was approved by Build Authority, which was approved by EdgeRun Controller, which was approved by Ken Root."}</TrustSentence><Card><h4 className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500">Chain</h4>{(risky ? ["Unknown Signer"] : ["Ken Root", "EdgeRun Controller", "Build Authority", "edgerun-node v0.4.2"]).map(x => <div key={x} className="mb-2 flex items-center gap-2 text-sm text-slate-300"><Key size={12} className="text-emerald-400" />{x}</div>)}</Card></div>}</Section>
}

function Compatibility() {
  return <Section icon={Globe} title="Compatibility" subtitle="Bridge existing identity and infrastructure standards."><TrustSentence>OpenID tells apps who logged in. Trust Capsules tell people and machines why that identity or authority should be trusted.</TrustSentence><div className="grid grid-cols-2 gap-3 lg:grid-cols-4">{compat.map(([name,Icon,status]) => <Card key={name}><Icon size={17} className="mb-2 text-slate-400" /><Badge status={status as Status}>{status}</Badge><div className="mt-2 text-sm font-medium text-slate-200">{name}</div></Card>)}</div><Card><h3 className="mb-3 text-sm font-semibold text-slate-300">OIDC / OAuth flows</h3><div className="flex flex-wrap gap-2">{["Authorization Code + PKCE","Device Authorization","Client Credentials","Token Exchange","Refresh Tokens","OpenID Federation"].map(x => <span key={x} className="rounded-lg border border-blue-500/20 bg-blue-500/10 px-3 py-1.5 text-xs text-blue-300">{x}</span>)}</div></Card></Section>
}

function Storage() {
  return <Section icon={Settings} title="Storage & Privacy" subtitle="Server hosts UI assets only. You control the capsule."><div className="grid gap-6 lg:grid-cols-2"><Card><h3 className="mb-4 text-sm font-semibold text-slate-300">Storage locations</h3>{[["Local browser",Globe,true],["Local file",HardDrive,true],["Phone",Smartphone,true],["Hardware key",Key,true],["Encrypted cloud backup",Database,true],["Git repository",GitBranch,false]].map(([label,Icon,active]) => { const I = Icon as LucideIcon; return <div key={String(label)} className="flex items-center justify-between border-b border-slate-700/30 py-2 text-sm"><span className="flex items-center gap-2 text-slate-300"><I size={14} className="text-slate-500" />{label}</span><Badge status={active ? "strong" : "imported"}>{active ? "active" : "add"}</Badge></div> })}</Card><Card><h3 className="mb-4 text-sm font-semibold text-slate-300">Capsule operations</h3><div className="space-y-2"><Button icon={Download}>Export public capsule</Button><Button icon={Lock}>Export encrypted backup</Button><Button icon={Globe}>Publish capsule</Button><Button icon={CheckCircle}>Verify backup</Button><Button icon={Trash2} danger>Remove local copy</Button></div></Card></div></Section>
}

export default function TrustManagerDemo() {
  const { page, sidebarOpen: open } = useStore(trustManagerStore)
  const current = useMemo(() => ({ dashboard: <Dashboard />, capsules: <Capsules />, root: <Root />, map: <TrustMap />, caps: <Capabilities />, policy: <Policy />, routes: <Routes />, delegations: <Delegations />, inspector: <Inspector />, compat: <Compatibility />, storage: <Storage /> } as Record<string, React.ReactNode>), [])
  if (page === "landing") return <Landing enter={() => setPage("dashboard")} />
  return <div className="flex min-h-screen bg-slate-950 text-slate-200"><aside className={`${open ? "w-56" : "w-14"} flex shrink-0 flex-col border-r border-slate-800/60 bg-slate-900/80 transition-all`}><div className="flex items-center gap-2 border-b border-slate-800/60 px-3 py-4"><div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-emerald-500/30 bg-emerald-600/20"><Shield size={14} className="text-emerald-400" /></div>{open && <span className="font-mono text-sm font-bold text-slate-100">Trust Manager</span>}<button onClick={toggleSidebar} className="ml-auto text-slate-600 hover:text-slate-400"><Menu size={14} /></button></div><nav className="flex-1 overflow-y-auto py-3">{nav.map(([id,label,Icon]) => <button key={id} onClick={() => setPage(id as TrustPage)} className={`flex w-full items-center gap-2.5 px-3 py-2 text-sm font-medium transition ${page === id ? "bg-emerald-500/10 text-emerald-300" : "text-slate-500 hover:bg-slate-800/60 hover:text-slate-300"}`}><Icon size={15} />{open && <span className="truncate">{label}</span>}{open && id === "routes" && <span className="ml-auto rounded-full bg-amber-500/20 px-1.5 py-0.5 text-xs text-amber-400">3</span>}</button>)}</nav><div className="border-t border-slate-800/60 px-3 py-3"><div className="flex items-center gap-2"><div className="flex h-7 w-7 items-center justify-center rounded-full bg-emerald-600/20 text-xs font-bold text-emerald-400">K</div>{open && <div><div className="text-xs font-medium text-slate-300">Ken Personal</div><div className="text-xs text-slate-600">2-of-3 · Strong</div></div>}</div></div></aside><main className="flex-1 overflow-auto"><header className="sticky top-0 z-10 flex items-center gap-4 border-b border-slate-800/60 bg-slate-950/90 px-6 py-3 backdrop-blur"><div className="relative max-w-sm flex-1"><Search size={13} className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-600" /><input placeholder="Search capsules, routes, events…" className="w-full rounded-lg border border-slate-700/50 bg-slate-800/60 py-2 pl-9 pr-4 text-sm text-slate-300 outline-none placeholder:text-slate-600" /></div><button className="relative p-2 text-slate-500 hover:text-slate-300"><Bell size={16} /><span className="absolute right-1 top-1 h-2 w-2 rounded-full bg-amber-500" /></button><button onClick={() => setPage("landing" as TrustPage)} className="px-2 py-1 text-xs text-slate-600 hover:text-slate-400">← Home</button></header><div className="max-w-6xl p-6">{current[page] ?? <Dashboard />}</div></main></div>
}
