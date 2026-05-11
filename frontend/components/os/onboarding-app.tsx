"use client"

import { useMemo } from "react"
import { useStore } from "@nanostores/react"
import {
  BadgeCheck,
  Boxes,
  BrainCircuit,
  Cloud,
  Database,
  Fingerprint,
  Globe,
  HardDrive,
  KeyRound,
  Network,
  Package,
  Route,
  Shield,
  Wallet,
  Zap,
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
    responsibility: "Signs local actions, verifies app packages, runs network apps, caches bytes, and records proof events.",
    userBenefit: "This is the default node every user gets immediately in the browser.",
    status: "ready",
    icon: Fingerprint,
  },
  {
    id: "storage-node",
    title: "Storage / CDN node",
    responsibility: "Stores content-addressed app/site/package objects and serves verified retrievals.",
    userBenefit: "Lets users run apps from network storage and lets providers earn from useful delivery.",
    status: "network",
    icon: HardDrive,
  },
  {
    id: "relay-node",
    title: "Relay node",
    responsibility: "Moves ordered encrypted messages and work packets between identities without owning the content.",
    userBenefit: "Enables private messaging, app delivery, and proof-backed routing.",
    status: "network",
    icon: Route,
  },
  {
    id: "compute-node",
    title: "Compute node",
    responsibility: "Runs deterministic work and returns verifiable outputs for admitted jobs.",
    userBenefit: "Turns idle hardware into useful paid work once compute proofs are wired.",
    status: "optional",
    icon: BrainCircuit,
  },
  {
    id: "admission-node",
    title: "Admission node",
    responsibility: "Checks signed requests, policy, funding, budget, and route before work enters the network.",
    userBenefit: "Prevents unpaid work and makes users/developers accountable by policy.",
    status: "network",
    icon: Shield,
  },
  {
    id: "settlement-node",
    title: "Settlement rail",
    responsibility: "Settles proof-backed receipts and pays publishers, storage, relay, and compute providers.",
    userBenefit: "Makes app sales, hosting, caching, and node earnings feel automatic.",
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
      title: "Bind passkey",
      done: Boolean(profile?.webAuthnBinding),
      body: profile?.webAuthnBinding
        ? "Passkey unlock is bound to this profile."
        : "Bind a passkey so signing and unlocks feel like normal web actions.",
      icon: KeyRound,
    },
    {
      title: "Start your browser node",
      done: Boolean(profile?.browserNode),
      body: profile?.browserNode
        ? "Your browser node exists and can verify, cache, sign, and eventually earn."
        : "The browser node is created with your profile and acts as your local runtime node.",
      icon: Network,
    },
    {
      title: "Run a network app",
      done: appState.installed.size > 0,
      body: appState.installed.size > 0
        ? `${appState.installed.size} app package${appState.installed.size === 1 ? "" : "s"} cached locally.`
        : "Open App Store, run an app from network storage, and optionally cache it locally.",
      icon: Package,
    },
    {
      title: "Inspect proof",
      done: capabilityGrantCount > 0 || appState.installed.size > 0,
      body: "Trust Manager shows identity, packages, capability grants, routes, and audit events from the same local state.",
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
                Own identity · own data · run by proof
              </div>
              <h2 className="mt-3 text-2xl font-semibold tracking-tight text-foreground">Your browser is your first Edgerun node.</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                Edgerun does not start with a cloud account. It starts with a local Trust Container in your browser. You own your identity, app keys, package cache, messages, and proof history. Network apps run from content-addressed storage, and you can cache verified bytes locally to avoid repeated retrieval payments.
              </p>
            </div>
            <div className="grid min-w-60 gap-2 rounded-xl border border-border bg-background/60 p-3 text-xs">
              <Metric label="Trust Container" value={profile ? "unlocked" : "locked"} icon={Shield} />
              <Metric label="Browser node" value={profile?.browserNode ? "ready" : "not started"} icon={Network} />
              <Metric label="Cached apps" value={String(appState.installed.size)} icon={Package} />
              <Metric label="Capability grants" value={String(capabilityGrantCount)} icon={KeyRound} />
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

        <section className="mt-5 grid gap-3 lg:grid-cols-3">
          <FlowCard icon={Package} title="Apps run from storage" body="Publishers use the CLI to publish signed SDK packages. Users run them by hash from network storage, or cache locally." />
          <FlowCard icon={Wallet} title="Usage pays the network" body="Storage, relay, compute, and publishers are paid through proof-backed work receipts instead of platform gatekeeping." />
          <FlowCard icon={Cloud} title="You can take full advantage" body="Leave your browser node running, cache packages, share resources when allowed, and inspect every proof in Trust Manager." />
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
