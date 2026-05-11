"use client"

import { useMemo } from "react"
import { useStore } from "@nanostores/react"
import {
  BadgeCheck,
  Boxes,
  BrainCircuit,
  Cloud,
  Fingerprint,
  Globe,
  HardDrive,
  KeyRound,
  MessageCircle,
  Network,
  Package,
  PhoneCall,
  Route,
  Shield,
  Users,
  Wallet,
  type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { AppHeader } from "@/components/os/app-chrome"
import { useAuth } from "@/hooks/use-auth"
import { browserAppInstallStore } from "@/platform/runtime/browser-app-install-store"
import { localCapabilityGrantsStore } from "@/stores/local-capability-grants-store"

type NodeCard = {
  id: string
  title: string
  responsibility: string
  userBenefit: string
  status: "ready" | "optional" | "network"
  icon: LucideIcon
}

const nodeCards: NodeCard[] = [
  {
    id: "browser-node",
    title: "Browser node",
    responsibility: "Receives its own identity, follows your local policy, signs actions, verifies packages, runs apps, creates work orders, caches bytes, records proofs, and can participate in network work when you allow it.",
    userBenefit: "Everyone starts included. You do not need tokens first; leave the browser node available and it can earn from useful work.",
    status: "ready",
    icon: Fingerprint,
  },
  {
    id: "admission-instance",
    title: "Admission instance",
    responsibility: "A WASM node instance that enforces one policy and budget before work enters the network. You can run separate instances for apps, family members, projects, or organizations.",
    userBenefit: "Use EdgeRun DAO admission by default, or run your own admission instances when you want stricter personal rules.",
    status: "ready",
    icon: Shield,
  },
  {
    id: "relay-instance",
    title: "Relay instance",
    responsibility: "A WASM or native node instance that moves admitted ordered packets without owning the content. Multiple relay instances can enforce different route policies.",
    userBenefit: "Pick your own relay rules: personal devices, family traffic, paid public traffic, or app-specific paths.",
    status: "optional",
    icon: Route,
  },
  {
    id: "storage-node",
    title: "Storage node",
    responsibility: "Accepts admitted work orders to store, retrieve, copy, pin, and move content-addressed data between EdgeRun storage, your cloud host, and your other machines.",
    userBenefit: "Run one on a VPS, home server, or spare machine so your data has a reliable place to live and can earn from useful storage/retrieval work.",
    status: "optional",
    icon: HardDrive,
  },
  {
    id: "cdn-node",
    title: "Storage / CDN node",
    responsibility: "Stores content-addressed app/site/package objects and serves verified retrievals for other users.",
    userBenefit: "Lets users run apps from network storage and lets providers earn from useful delivery.",
    status: "network",
    icon: Cloud,
  },
  {
    id: "compute-node",
    title: "Compute node",
    responsibility: "Runs deterministic work and returns verifiable outputs for admitted jobs. The browser node is already a small local compute node for app execution.",
    userBenefit: "Turns idle hardware into useful paid work once compute proofs are wired.",
    status: "optional",
    icon: BrainCircuit,
  },
  {
    id: "settlement-node",
    title: "Settlement rail",
    responsibility: "Settles proof-backed receipts and pays publishers, storage, relay, browser, and compute providers.",
    userBenefit: "A user can earn first, then spend inside the network. App sales, hosting, caching, and node earnings feel automatic.",
    status: "network",
    icon: Wallet,
  },
]

function statusLabel(status: NodeCard["status"]) {
  if (status === "ready") return "ready now"
  if (status === "network") return "network service"
  return "optional"
}

function statusClass(status: NodeCard["status"]) {
  if (status === "ready") return "border-[var(--status-online)]/25 bg-[var(--status-online)]/10 text-[var(--status-online)]"
  if (status === "network") return "border-primary/25 bg-primary/10 text-primary"
  return "border-[var(--status-warning)]/25 bg-[var(--status-warning)]/10 text-[var(--status-warning)]"
}

export function OnboardingApp() {
  const auth = useAuth()
  const appState = useStore(browserAppInstallStore)
  const capabilityGrants = useStore(localCapabilityGrantsStore)
  const profile = auth.unlockedProfile
  const contactCount = Math.max((profile?.contacts.length ?? 0) - 1, 0)
  const capabilityGrantCount = useMemo(
    () => Object.values(capabilityGrants).reduce((sum, grants) => sum + grants.length, 0),
    [capabilityGrants],
  )

  const checklist = [
    {
      title: "Own your identity",
      done: Boolean(profile),
      body: profile
        ? `Trust Container unlocked for ${profile.handle}.`
        : "Create or unlock a local Trust Container. Your identity keys stay in your browser unless you export them.",
      icon: Fingerprint,
    },
    {
      title: "Set admission policy",
      done: Boolean(profile?.browserNode),
      body: profile?.browserNode
        ? "Your browser node can use EdgeRun DAO admission or your own WASM admission instances for app, family, or project budgets."
        : "Choose who can submit work, which relays/workers are allowed, and which budget/policy applies.",
      icon: Shield,
    },
    {
      title: "Add contacts",
      done: contactCount > 0,
      body: contactCount > 0
        ? `${contactCount} direct contact${contactCount === 1 ? "" : "s"} in your contact book.`
        : "Your contact book lets you message and call people directly by identity instead of platform account.",
      icon: Users,
    },
    {
      title: "Send storage work orders",
      done: appState.installed.size > 0,
      body: appState.installed.size > 0
        ? "You already have verified cached package data. Storage work orders are the same idea for your files."
        : "Ask your own host, another machine, or paid network storage to store, copy, pin, retrieve, or move data for you.",
      icon: HardDrive,
    },
    {
      title: "Inspect proof",
      done: capabilityGrantCount > 0 || appState.installed.size > 0,
      body: "Trust Manager shows identity, admission policy, packages, grants, routes, work orders, and audit events from the same local state.",
      icon: BadgeCheck,
    },
  ]

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <AppHeader title="Help & Onboarding" icon={<Globe className="h-4 w-4" />}>
        Learn how Edgerun turns your browser into a user-owned node
      </AppHeader>
      <div className="min-h-0 flex-1 overflow-auto p-4 sm:p-5">
        <section className="rounded-xl border border-border bg-card/60 p-5">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
            <div className="max-w-2xl">
              <div className="inline-flex rounded-full border border-primary/25 bg-primary/10 px-2.5 py-1 text-[11px] font-semibold uppercase tracking-wide text-primary">
                Own identity · run policy nodes · earn first
              </div>
              <h2 className="mt-3 text-2xl font-semibold tracking-tight text-foreground">Your browser can run more than one Edgerun node.</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                Admission nodes, relay nodes, compute nodes, and app runtimes are WASM-capable node instances. You can use the default EdgeRun DAO admission node, or run your own admission instances with different rules and budgets for apps, family members, projects, or organizations. Every workload still enters through admission, then moves through relays under policy.
              </p>
            </div>
            <div className="grid min-w-60 gap-2 rounded-xl border border-border bg-background/60 p-3 text-xs">
              <Metric label="Trust Container" value={profile ? "unlocked" : "locked"} icon={Shield} />
              <Metric label="Browser node" value={profile?.browserNode ? "ready" : "not started"} icon={Network} />
              <Metric label="Contacts" value={String(contactCount)} icon={Users} />
              <Metric label="Cached apps" value={String(appState.installed.size)} icon={Package} />
            </div>
          </div>
        </section>

        <section className="mt-4 grid gap-3 lg:grid-cols-5">
          {checklist.map((item, index) => {
            const Icon = item.icon
            return (
              <div key={item.title} className={cn("rounded-xl border p-4", item.done ? "border-[var(--status-online)]/25 bg-[var(--status-online)]/5" : "border-border bg-card/55")}>
                <div className="flex items-center justify-between gap-2">
                  <div className="flex h-9 w-9 items-center justify-center rounded-lg border border-border bg-background/60 text-primary">
                    <Icon className="h-4 w-4" />
                  </div>
                  <span className={cn("rounded px-1.5 py-0.5 text-[10px] font-medium", item.done ? "bg-[var(--status-online)]/15 text-[var(--status-online)]" : "bg-secondary text-muted-foreground")}>{item.done ? "done" : `step ${index + 1}`}</span>
                </div>
                <div className="mt-3 text-sm font-semibold text-foreground">{item.title}</div>
                <div className="mt-1 text-xs leading-5 text-muted-foreground">{item.body}</div>
              </div>
            )
          })}
        </section>

        <section className="mt-5 grid gap-3 lg:grid-cols-3">
          <FlowCard icon={Shield} title="Run scoped admission" body="Create separate admission instances for an app, a family member, a project, or a business budget. Each instance can enforce its own policy hash." />
          <FlowCard icon={Route} title="Run scoped relays" body="Relay instances can carry only the traffic you allow: family routes, app routes, paid public routes, or private machine-to-machine routes." />
          <FlowCard icon={Wallet} title="Everyone can start" body="A new user can leave browser node instances available and earn before buying. Usage funds the network instead of forcing token purchase upfront." />
        </section>

        <section className="mt-5 grid gap-3 lg:grid-cols-3">
          <FlowCard icon={HardDrive} title="Give storage work orders" body="Your browser node can ask admitted storage nodes to store, retrieve, copy, pin, or move data. The worker returns proof, and payment follows useful work." />
          <FlowCard icon={Cloud} title="Use cloud or your own machine" body="Run storage, relay, admission, or compute nodes on a VPS, home server, NAS, spare computer, or browser tab. Your policy decides what each instance may do." />
          <FlowCard icon={Package} title="Apps run from storage" body="Publishers use the CLI to publish signed SDK packages. Users run them by hash from network storage, or cache locally." />
        </section>

        <section className="mt-5 grid gap-3 lg:grid-cols-3">
          <FlowCard icon={MessageCircle} title="Message directly" body="Your contact book stores identities, not platform handles. Messages are sealed and routed by policy, without needing Meta, Apple, or Google as the social graph." />
          <FlowCard icon={PhoneCall} title="Call directly" body="Calls can use the same identity and route model: the network helps connect peers, but your node and your contacts remain the authority." />
          <FlowCard icon={Network} title="Move data with proof" body="Data movement is an admitted route/work-order flow. The transport can be cloud, home machine, network storage, or another EdgeRun node." />
        </section>

        <section className="mt-5">
          <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground">
            <Boxes className="h-4 w-4 text-primary" />
            Node responsibilities
          </div>
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {nodeCards.map((node) => {
              const Icon = node.icon
              return (
                <div key={node.id} className="rounded-xl border border-border bg-card/55 p-4">
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex h-10 w-10 items-center justify-center rounded-lg border border-border bg-background/60 text-primary">
                      <Icon className="h-4 w-4" />
                    </div>
                    <span className={cn("rounded border px-1.5 py-0.5 text-[10px] font-semibold uppercase", statusClass(node.status))}>{statusLabel(node.status)}</span>
                  </div>
                  <div className="mt-3 text-sm font-semibold text-foreground">{node.title}</div>
                  <div className="mt-1 text-xs leading-5 text-muted-foreground">{node.responsibility}</div>
                  <div className="mt-3 rounded-md border border-border bg-background/60 p-2 text-xs leading-5 text-muted-foreground">
                    {node.userBenefit}
                  </div>
                </div>
              )
            })}
          </div>
        </section>
      </div>
    </div>
  )
}

function Metric({ label, value, icon: Icon }: { label: string; value: string; icon: LucideIcon }) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-md border border-border bg-secondary/25 px-3 py-2">
      <div className="flex items-center gap-2 text-muted-foreground"><Icon className="h-3.5 w-3.5" />{label}</div>
      <div className="font-semibold text-foreground">{value}</div>
    </div>
  )
}

function FlowCard({ icon: Icon, title, body }: { icon: LucideIcon; title: string; body: string }) {
  return (
    <div className="rounded-xl border border-primary/20 bg-primary/5 p-4">
      <div className="flex items-center gap-2 text-sm font-semibold text-foreground"><Icon className="h-4 w-4 text-primary" />{title}</div>
      <div className="mt-2 text-xs leading-5 text-muted-foreground">{body}</div>
    </div>
  )
}
