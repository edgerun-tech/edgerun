export enum Capability {
  Identity = "identity",
  NodeConnection = "node_connection",
  NetworkAccess = "network_access",
  Filesystem = "filesystem",
  HardwareSigning = "hardware_signing",
  Payments = "payments",
  VoiceCall = "voice_call",
}

export interface CapabilityInfo {
  id: Capability
  label: string
  description: string
  icon: string
  requiresAuth: boolean
}

export const CAPABILITY_REGISTRY: Record<Capability, CapabilityInfo> = {
  [Capability.Identity]: {
    id: Capability.Identity,
    label: "Identity",
    description: "Requires a verified user identity (passkey/biometric)",
    icon: "fingerprint",
    requiresAuth: true,
  },
  [Capability.NodeConnection]: {
    id: Capability.NodeConnection,
    label: "Node",
    description: "Connects to an Edgerun node",
    icon: "server",
    requiresAuth: false,
  },
  [Capability.NetworkAccess]: {
    id: Capability.NetworkAccess,
    label: "Network",
    description: "Outbound network requests",
    icon: "globe",
    requiresAuth: false,
  },
  [Capability.Filesystem]: {
    id: Capability.Filesystem,
    label: "Storage",
    description: "Read/write app-scoped storage",
    icon: "folder",
    requiresAuth: false,
  },
  [Capability.HardwareSigning]: {
    id: Capability.HardwareSigning,
    label: "Hardware Signing",
    description: "Uses hardware-backed key for signing",
    icon: "key",
    requiresAuth: true,
  },
  [Capability.Payments]: {
    id: Capability.Payments,
    label: "Payments",
    description: "Wallet access and transactions",
    icon: "credit-card",
    requiresAuth: true,
  },
  [Capability.VoiceCall]: {
    id: Capability.VoiceCall,
    label: "Voice Call",
    description: "P2P encrypted voice calls",
    icon: "phone",
    requiresAuth: true,
  },
}

export interface CapabilityCheckResult {
  capability: Capability
  available: boolean
  info: CapabilityInfo
}

export interface GuestContext {
  isGuest: boolean
  hasIdentity: boolean
  hasNode: boolean
}

export function checkCapabilities(
  required: Capability[],
  optional: Capability[],
  ctx: GuestContext,
): { all: CapabilityCheckResult[]; blocked: CapabilityCheckResult[]; granted: CapabilityCheckResult[] } {
  const all = [...new Set([...required, ...optional])]

  const results = all.map((cap) => {
    const info = CAPABILITY_REGISTRY[cap]
    let available = true

    if (info.requiresAuth) {
      if (cap === Capability.Identity || cap === Capability.HardwareSigning) {
        available = ctx.hasIdentity
      } else if (cap === Capability.Payments || cap === Capability.VoiceCall) {
        available = ctx.hasIdentity && ctx.hasNode
      }
    }

    return { capability: cap, available, info }
  })

  return {
    all: results,
    blocked: results.filter((r) => !r.available),
    granted: results.filter((r) => r.available),
  }
}

export function isCapabilityAvailable(cap: Capability, ctx: GuestContext): boolean {
  const info = CAPABILITY_REGISTRY[cap]
  if (!info.requiresAuth) return true
  if (cap === Capability.Identity || cap === Capability.HardwareSigning) return ctx.hasIdentity
  if (cap === Capability.Payments || cap === Capability.VoiceCall) return ctx.hasIdentity && ctx.hasNode
  return true
}
