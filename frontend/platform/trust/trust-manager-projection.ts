import type { RuntimeEvent } from "@/platform/runtime/browser-capability-types"
import type { BrowserAppInstallState } from "@/platform/runtime/browser-app-install-store"
import type { LocalCapabilityGrant } from "@/stores/local-capability-grants-store"
import type { UnlockedProfileContainer } from "@/stores/auth-types"

export type TrustProjectionStatus = "strong" | "verified" | "review" | "danger" | "active" | "limited" | "revoked"
export type TrustProjectionRisk = "low" | "medium" | "high"
export type TrustProjectionTab = "overview" | "capsules" | "capabilities" | "routes" | "packages" | "events"

export interface TrustProjectionItem {
  id: string
  tab: TrustProjectionTab
  title: string
  subtitle: string
  meta: string
  status: TrustProjectionStatus
  risk?: TrustProjectionRisk
  authorityRef: string
  proofRef: string
  lastChecked: string
  details: string[]
  actions?: string[]
}

export interface TrustProjectionInput {
  profile: UnlockedProfileContainer | null
  appState: BrowserAppInstallState
  capabilityGrants: Record<string, LocalCapabilityGrant[]>
  runtimeEvents: RuntimeEvent[]
  now?: number
}

export interface TrustProjection {
  overview: TrustProjectionItem[]
  capsules: TrustProjectionItem[]
  capabilities: TrustProjectionItem[]
  routes: TrustProjectionItem[]
  packages: TrustProjectionItem[]
  events: TrustProjectionItem[]
}

export function buildTrustProjection(input: TrustProjectionInput): TrustProjection {
  return {
    overview: buildOverviewItems(input),
    capsules: buildCapsuleItems(input),
    capabilities: buildCapabilityItems(input),
    routes: buildRouteItems(input),
    packages: buildPackageItems(input),
    events: buildEventItems(input),
  }
}

function buildOverviewItems(input: TrustProjectionInput): TrustProjectionItem[] {
  const profile = input.profile
  if (!profile) {
    return [{
      id: "profile-locked",
      tab: "overview",
      title: "Trust Container locked",
      subtitle: "Unlock identity to inspect authority",
      meta: "no profile state loaded",
      status: "review",
      risk: "medium",
      authorityRef: "profile:locked",
      proofRef: "proof:none",
      lastChecked: "now",
      details: [
        "The sealed Trust Container exists outside this projection until unlocked.",
        "Unlocking exposes local identity, browser node, event log, app secrets, and sealed containers to the UI.",
      ],
      actions: ["Unlock"],
    }]
  }

  const cachedApps = input.appState.installed.size
  const grantCount = Object.values(input.capabilityGrants).reduce((sum, grants) => sum + grants.length, 0)
  return [
    {
      id: "profile-root",
      tab: "overview",
      title: `${profile.handle} Trust Container`,
      subtitle: "Local sealed browser identity",
      meta: `${profile.nodes.length} node identities · ${profile.contacts.length} contacts · ${profile.eventLog.length} profile events`,
      status: profile.webAuthnBinding ? "strong" : "verified",
      authorityRef: `profile:${profile.owner.identityIdHex}`,
      proofRef: profile.eventLog.at(-1)?.eventHash ?? "proof:profile-genesis",
      lastChecked: "now",
      details: [
        "This profile is the user's local authority root for browser-node actions.",
        "It contains owner identity, browser node identity, contacts, app secrets, sealed containers, and profile events.",
        profile.webAuthnBinding ? "Passkey unlock is bound through the local WebAuthn vault." : "Passkey unlock is not bound yet.",
      ],
      actions: ["Open Identity", "Export", "Bind passkey"],
    },
    {
      id: "browser-node",
      tab: "overview",
      title: "Browser node",
      subtitle: "User-owned runtime node",
      meta: short(profile.browserNode.identityIdHex),
      status: profile.profilePreferences.shareResources ? "active" : "limited",
      authorityRef: `node:${profile.browserNode.identityIdHex}`,
      proofRef: profile.eventLog.find((event) => event.kind === "NODE_GENESIS")?.eventHash ?? "proof:node-genesis",
      lastChecked: "now",
      details: [
        "The browser node can sign local runtime events, cache verified packages, and participate in work flows when enabled.",
        profile.profilePreferences.shareResources ? "Resource sharing is enabled in profile preferences." : "Resource sharing is disabled in profile preferences.",
      ],
      actions: ["Open routes", "Toggle earning"],
    },
    {
      id: "package-cache",
      tab: "overview",
      title: "Verified package cache",
      subtitle: "Network-run apps cached locally",
      meta: `${cachedApps} cached packages · ${grantCount} capability grants`,
      status: cachedApps > 0 ? "verified" : "limited",
      authorityRef: "cache:indexeddb:edgerun-browser-apps",
      proofRef: `runtime-events:app_package_verified:${input.runtimeEvents.filter((event) => event.kind === "app_package_verified").length}`,
      lastChecked: "now",
      details: [
        "Apps run from network storage by content hash.",
        "Local cache avoids repeated retrieval payments and records verification events.",
      ],
      actions: ["Open App Store", "Inspect packages"],
    },
  ]
}

function buildCapsuleItems(input: TrustProjectionInput): TrustProjectionItem[] {
  const profile = input.profile
  if (!profile) return []
  return [
    {
      id: "personal-container",
      tab: "capsules",
      title: "Personal Trust Container",
      subtitle: "AES-GCM sealed profile",
      meta: `created ${profile.createdAtIso}`,
      status: "strong",
      authorityRef: `profile:${profile.owner.identityIdHex}`,
      proofRef: profile.eventLog.at(-1)?.eventHash ?? "proof:profile",
      lastChecked: "now",
      details: [
        "Contains identity keys, encryption keys, browser node keys, contacts, OAuth secrets, and sealed containers.",
        `${profile.appSecrets.length} app secrets and ${profile.sealedContainers.length} sealed containers are currently stored.`,
      ],
      actions: ["Export", "Backup", "Rotate"],
    },
    ...profile.nodes.map((node, index) => ({
      id: `node-${node.identityIdHex}`,
      tab: "capsules" as const,
      title: index === 0 ? "Primary browser node" : `Device node ${index + 1}`,
      subtitle: node.algorithm,
      meta: short(node.identityIdHex),
      status: "verified" as const,
      authorityRef: `node:${node.identityIdHex}`,
      proofRef: profile.eventLog.find((event) => event.kind === "NODE_GENESIS" && event.payload.nodeId === node.identityIdHex)?.eventHash ?? "proof:node",
      lastChecked: "now",
      details: ["Node identity is part of the local profile event log.", "Additional devices can be added from Identity."],
      actions: ["Inspect", "Rotate"],
    })),
  ]
}

function buildCapabilityItems(input: TrustProjectionInput): TrustProjectionItem[] {
  return Object.entries(input.capabilityGrants).flatMap(([appId, grants]) =>
    grants.map((grant) => ({
      id: `grant-${appId}-${grant.capabilityId}`,
      tab: "capabilities" as const,
      title: grant.capabilityId,
      subtitle: `Granted to ${appId}`,
      meta: grant.expiresAt ? `expires ${new Date(grant.expiresAt).toISOString()}` : "no expiry",
      status: grant.expiresAt && grant.expiresAt <= (input.now ?? Date.now()) ? "review" as const : "active" as const,
      risk: capabilityRisk(grant.capabilityId),
      authorityRef: `capability:${appId}:${grant.capabilityId}`,
      proofRef: `grant:${grant.grantedAt}`,
      lastChecked: "now",
      details: [
        `Scope: ${grant.scope}`,
        `Reason: ${grant.reason}`,
        "Capability grants are local browser policy and should map to Trust Manager approvals.",
      ],
      actions: ["Inspect", "Revoke"],
    })),
  )
}

function buildRouteItems(input: TrustProjectionInput): TrustProjectionItem[] {
  const profile = input.profile
  if (!profile) return []
  const outboxRoutes = new Set(profile.outbox.map((item) => item.routeHint).filter(Boolean))
  return [
    ...Array.from(outboxRoutes).map((route) => ({
      id: `route-${route}`,
      tab: "routes" as const,
      title: route,
      subtitle: "Queued sealed-message route",
      meta: `${profile.outbox.filter((item) => item.routeHint === route).length} queued envelopes`,
      status: "active" as const,
      risk: "low" as const,
      authorityRef: `route:${route}`,
      proofRef: `outbox:${route}`,
      lastChecked: "now",
      details: ["Route was inferred from local sealed-message outbox state.", "Protocol route advertisements should replace this projection when connected."],
      actions: ["Inspect", "Pause"],
    })),
    {
      id: "browser-node-route",
      tab: "routes",
      title: "Browser node network route",
      subtitle: profile.profilePreferences.connectToNetwork ? "Network connection enabled" : "Network connection disabled",
      meta: short(profile.browserNode.identityIdHex),
      status: profile.profilePreferences.connectToNetwork ? "active" : "limited",
      risk: "medium",
      authorityRef: `node-route:${profile.browserNode.identityIdHex}`,
      proofRef: "profile-preferences:connectToNetwork",
      lastChecked: "now",
      details: ["This is derived from profile preferences.", "Signed route advertisements should be shown here once connected to edgerun-work route state."],
      actions: ["Publish route", "Disable"],
    },
  ]
}

function buildPackageItems(input: TrustProjectionInput): TrustProjectionItem[] {
  return Array.from(input.appState.installed.values()).map((app) => ({
    id: `package-${app.appId}`,
    tab: "packages" as const,
    title: app.name,
    subtitle: `${app.version} · ${app.developer}`,
    meta: `${formatBytes(app.packageBytes)} · ${app.verifiedAssets.length} verified assets`,
    status: "verified" as const,
    risk: app.verifiedAssets.length > 0 ? "low" as const : "medium" as const,
    authorityRef: `developer:${app.developerId || app.developer}`,
    proofRef: `package:${app.eappSha256}`,
    lastChecked: app.installedAt,
    details: [
      `Manifest hash: ${app.manifestSha256}`,
      `Release id: ${app.releaseId || "unknown"}`,
      `Runtime app id: ${app.runtimeAppId || "unknown"}`,
      "This package was verified through the browser node and cached locally.",
    ],
    actions: ["Open App Store", "Remove cache"],
  }))
}

function buildEventItems(input: TrustProjectionInput): TrustProjectionItem[] {
  const profileEvents = (input.profile?.eventLog ?? []).map((event) => ({
    id: `profile-event-${event.eventHash}`,
    tab: "events" as const,
    title: event.kind,
    subtitle: "Profile event log",
    meta: `seq ${event.seq}`,
    status: "verified" as const,
    authorityRef: `profile-event:${event.seq}`,
    proofRef: event.eventHash,
    lastChecked: "now",
    details: [
      `Payload hash: ${event.payloadSha256}`,
      `Previous event: ${event.previousEventHash}`,
      "Profile events are signed by the browser node identity.",
    ],
    actions: ["Copy proof"],
  }))

  const runtime = input.runtimeEvents.slice(-25).reverse().map((event) => ({
    id: `runtime-event-${event.id}`,
    tab: "events" as const,
    title: event.kind,
    subtitle: event.reason ?? event.actor ?? "Runtime event",
    meta: new Date(event.createdAt).toISOString(),
    status: event.decision === "deny" ? "danger" as const : "verified" as const,
    authorityRef: event.actor ? `actor:${event.actor}` : "runtime:local",
    proofRef: event.outputHash ?? event.inputHash ?? event.id,
    lastChecked: "now",
    details: [
      event.inputHash ? `Input hash: ${event.inputHash}` : "No input hash recorded.",
      event.outputHash ? `Output hash: ${event.outputHash}` : "No output hash recorded.",
      event.metadata ? `Metadata keys: ${Object.keys(event.metadata).join(", ") || "none"}` : "No metadata.",
    ],
    actions: ["Copy proof"],
  }))

  return [...runtime, ...profileEvents]
}

function capabilityRisk(capabilityId: string): TrustProjectionRisk {
  if (capabilityId.includes("payment") || capabilityId.includes("wallet")) return "high"
  if (capabilityId.includes("filesystem") || capabilityId.includes("network") || capabilityId.includes("messaging")) return "medium"
  return "low"
}

function short(value: string): string {
  return value.length > 18 ? `${value.slice(0, 12)}…${value.slice(-6)}` : value
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B"
  const units = ["B", "KB", "MB", "GB"]
  let value = bytes
  let index = 0
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024
    index += 1
  }
  return `${value.toFixed(index === 0 ? 0 : 1)} ${units[index]}`
}
