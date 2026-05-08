export type CapabilityRisk = "low" | "medium" | "high"

export interface CapabilityInfo {
  id: string
  label: string
  short: string
  why: string
  risk: CapabilityRisk
  examples?: string[]
}

const DEFAULT_CAPABILITY: CapabilityInfo = {
  id: "unknown",
  label: "Unknown capability",
  short: "Requested access",
  why: "This app is requesting a capability that is not yet described in the local catalog.",
  risk: "medium",
}

export const CAPABILITY_CATALOG: Record<string, CapabilityInfo> = {
  identity: {
    id: "identity",
    label: "Identity",
    short: "Use your Edgerun identity",
    why: "The app needs to know which local identity is acting so actions can be attributed, signed, and shown to the right user.",
    risk: "medium",
    examples: ["Contacts", "People", "Wallet"],
  },
  messaging: {
    id: "messaging",
    label: "Messaging",
    short: "Send and receive messages",
    why: "The app needs permission to create, read, and display message events for your selected identity.",
    risk: "medium",
    examples: ["People messages"],
  },
  voice_call: {
    id: "voice_call",
    label: "Voice calls",
    short: "Start and receive calls",
    why: "The app needs permission to initiate call sessions and expose call controls. Microphone access should still be requested separately by the browser or device runtime.",
    risk: "high",
    examples: ["People calls"],
  },
  payments: {
    id: "payments",
    label: "Payments",
    short: "Access wallet/payment actions",
    why: "The app needs permission to show balances, prepare payment commands, or request signed payment actions. Final spending should require explicit confirmation.",
    risk: "high",
    examples: ["Wallet", "EDGE token"],
  },
  filesystem: {
    id: "filesystem",
    label: "Files",
    short: "Read or manage local files",
    why: "The app needs access to the virtual filesystem or selected file objects. Access should be scoped to the chosen files or app workspace.",
    risk: "high",
    examples: ["Files", "App Studio"],
  },
  network_access: {
    id: "network_access",
    label: "Network",
    short: "Use network access",
    why: "The app needs to open network sessions or publish service endpoints. This can reveal metadata, so access should stay scoped.",
    risk: "high",
    examples: ["Web Server", "Network tools"],
  },
  node_connection: {
    id: "node_connection",
    label: "Node connection",
    short: "Talk to Edgerun nodes",
    why: "The app needs to query or control local/remote nodes through the protocol client. Commands still require local validation and policy checks.",
    risk: "medium",
    examples: ["Compute", "DB Explorer", "Git Sync"],
  },
}

export function getCapabilityInfo(capabilityId: string): CapabilityInfo {
  return CAPABILITY_CATALOG[capabilityId] ?? {
    ...DEFAULT_CAPABILITY,
    id: capabilityId,
    label: capabilityId.replaceAll("_", " "),
  }
}

export function describeCapabilityList(capabilityIds: string[]): CapabilityInfo[] {
  return capabilityIds.map(getCapabilityInfo)
}

export function riskTone(risk: CapabilityRisk): string {
  switch (risk) {
    case "low":
      return "text-[var(--status-online)] bg-[var(--status-online)]/10 border-[var(--status-online)]/20"
    case "medium":
      return "text-[var(--status-warning)] bg-[var(--status-warning)]/10 border-[var(--status-warning)]/20"
    case "high":
      return "text-[var(--status-error)] bg-[var(--status-error)]/10 border-[var(--status-error)]/20"
  }
}
