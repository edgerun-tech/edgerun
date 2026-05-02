/**
 * Builtin app definitions for the EdgeRun dashboard.
 *
 * Replaces the hardcoded `availableApps` array that was in components/os/app-store.tsx.
 * Uses platform `AppDefinition` type from `platform/types/app-definition.ts`.
 *
 * No fake RAM/CPU/price as truth.
 * If unknown, show unknown.
 * If demo, label demo.
 * If measured, link to footprint evidence.
 */

import type { AppDefinition, AppKind, AppSource } from "@/platform/types/app-definition"
import {
  Terminal,
  Code2,
  Database,
  Globe,
  FileText,
  GitBranch,
  Cpu,
  Network,
  HelpCircle,
  Users,
  Phone,
  MessageSquare,
  Wallet,
  Calculator,
  Activity,
  Sparkles,
  Workflow,
  Package,
} from "lucide-react"

/**
 * Icon ID to Lucide icon component mapping.
 * Centralized here; component-registry maps iconId → ReactNode when rendering.
 */
export const BUILTIN_ICON_MAP: Record<string, React.ReactNode> = {
  terminal: <Terminal className="h-5 w-5" />,
  "code-runner": <Code2 className="h-5 w-5" />,
  "db-explorer": <Database className="h-5 w-5" />,
  "network-monitor": <Network className="h-5 w-5" />,
  "resource-monitor": <Activity className="h-5 w-5" />,
  "file-browser": <FileText className="h-5 w-5" />,
  "git-sync": <GitBranch className="h-5 w-5" />,
  "web-server": <Globe className="h-5 w-5" />,
  "compute-node": <Cpu className="h-5 w-5" />,
  contacts: <Users className="h-5 w-5" />,
  calling: <Phone className="h-5 w-5" />,
  chat: <MessageSquare className="h-5 w-5" />,
  "ai-assistant": <Sparkles className="h-5 w-5" />,
  "workflow-builder": <Workflow className="h-5 w-5" />,
  wallet: <Wallet className="h-5 w-5" />,
  calculator: <Calculator className="h-5 w-5" />,
  help: <HelpCircle className="h-5 w-5" />,
  "wasm-generic": <Package className="h-5 w-5" />,
}

export function getIconById(iconId: string): React.ReactNode {
  return BUILTIN_ICON_MAP[iconId] || BUILTIN_ICON_MAP["wasm-generic"]
}

/**
 * Builtin app definitions.
 * No fake RAM/CPU/price.
 * `footprint` is only set when measured; otherwise undefined.
 * `source: "demo"` marks apps that are not real protocol-backed apps.
 */
export const BUILTIN_APPS: AppDefinition[] = [
  {
    appId: "terminal",
    name: "Terminal",
    description: "System shell & logs",
    iconId: "terminal",
    kind: "builtin",
    source: "builtin",
    componentKey: "terminal",
    requiredCapabilityIds: [],
    optionalCapabilityIds: [],
    status: "available",
  },
  {
    appId: "code-runner",
    name: "Code Runner",
    description: "Execute WASM modules",
    iconId: "code-runner",
    kind: "builtin",
    source: "builtin",
    componentKey: "code-runner",
    requiredCapabilityIds: [],
    optionalCapabilityIds: [],
    status: "available",
  },
  {
    appId: "db-explorer",
    name: "DB Explorer",
    description: "Query distributed state",
    iconId: "db-explorer",
    kind: "builtin",
    source: "builtin",
    componentKey: "db-explorer",
    requiredCapabilityIds: [],
    optionalCapabilityIds: ["node_connection"],
    status: "available",
  },
  {
    appId: "network-monitor",
    name: "Network",
    description: "P2P connection status",
    iconId: "network-monitor",
    kind: "builtin",
    source: "builtin",
    componentKey: "network-monitor",
    requiredCapabilityIds: [],
    optionalCapabilityIds: [],
    status: "available",
  },
  {
    appId: "resource-monitor",
    name: "Resource Monitor",
    description: "System metrics & performance",
    iconId: "resource-monitor",
    kind: "builtin",
    source: "builtin",
    componentKey: "resource-monitor",
    requiredCapabilityIds: [],
    optionalCapabilityIds: [],
    status: "available",
  },
  {
    appId: "file-browser",
    name: "Files",
    description: "Virtual filesystem",
    iconId: "file-browser",
    kind: "builtin",
    source: "builtin",
    componentKey: "file-browser",
    requiredCapabilityIds: [],
    optionalCapabilityIds: ["filesystem"],
    status: "available",
  },
  {
    appId: "git-sync",
    name: "Git Sync",
    description: "Decentralized repos",
    iconId: "git-sync",
    kind: "builtin",
    source: "builtin",
    componentKey: "git-sync",
    requiredCapabilityIds: [],
    optionalCapabilityIds: ["node_connection"],
    status: "available",
  },
  {
    appId: "web-server",
    name: "Web Server",
    description: "Serve static content",
    iconId: "web-server",
    kind: "builtin",
    source: "builtin",
    componentKey: "web-server",
    requiredCapabilityIds: [],
    optionalCapabilityIds: ["network_access"],
    status: "available",
  },
  {
    appId: "compute-node",
    name: "Compute",
    description: "Distributed processing",
    iconId: "compute-node",
    kind: "builtin",
    source: "builtin",
    componentKey: "compute-node",
    requiredCapabilityIds: [],
    optionalCapabilityIds: ["node_connection"],
    status: "available",
  },
  {
    appId: "contacts",
    name: "Contacts",
    description: "Manage your peer network",
    iconId: "contacts",
    kind: "builtin",
    source: "builtin",
    componentKey: "contacts",
    requiredCapabilityIds: ["identity"],
    optionalCapabilityIds: [],
    status: "available",
  },
  {
    appId: "calling",
    name: "Calling",
    description: "Encrypted P2P voice calls",
    iconId: "calling",
    kind: "builtin",
    source: "demo", // no real protocol-backed calling yet
    componentKey: "calling",
    requiredCapabilityIds: ["identity", "voice_call"],
    optionalCapabilityIds: [],
    status: "demo",
  },
  {
    appId: "chat",
    name: "Chat",
    description: "Messaging",
    iconId: "chat",
    kind: "builtin",
    source: "demo", // no real E2E protocol-backed chat yet
    componentKey: "chat-demo",
    requiredCapabilityIds: ["identity"],
    optionalCapabilityIds: [],
    status: "demo",
  },
  {
    appId: "ai-assistant",
    name: "AI Assistant",
    description: "LLM-powered helper",
    iconId: "ai-assistant",
    kind: "builtin",
    source: "builtin",
    componentKey: "ai-assistant",
    requiredCapabilityIds: [],
    optionalCapabilityIds: [],
    status: "available",
  },
  {
    appId: "workflow-builder",
    name: "Workflow Builder",
    description: "Create & manage automation workflows",
    iconId: "workflow-builder",
    kind: "builtin",
    source: "builtin",
    componentKey: "workflow-builder",
    requiredCapabilityIds: [],
    optionalCapabilityIds: [],
    status: "available",
  },
  {
    appId: "wallet",
    name: "Wallet",
    description: "EDGE token & payments",
    iconId: "wallet",
    kind: "builtin",
    source: "demo", // no real payment backend yet
    componentKey: "wallet",
    requiredCapabilityIds: ["identity", "payments"],
    optionalCapabilityIds: [],
    status: "demo",
  },
  {
    appId: "calculator",
    name: "Calculator",
    description: "System utility",
    iconId: "calculator",
    kind: "builtin",
    source: "builtin",
    componentKey: "calculator",
    requiredCapabilityIds: [],
    optionalCapabilityIds: [],
    status: "available",
  },
  {
    appId: "help",
    name: "Help & Onboarding",
    description: "Platform guide & setup",
    iconId: "help",
    kind: "builtin",
    source: "builtin",
    componentKey: "help",
    requiredCapabilityIds: [],
    optionalCapabilityIds: [],
    status: "available",
  },
]

/**
 * Get builtin app by ID.
 */
export function getBuiltinApp(appId: string): AppDefinition | undefined {
  return BUILTIN_APPS.find((a) => a.appId === appId)
}

/**
 * List all builtin apps, optionally filtered by source.
 */
export function listBuiltinApps(source?: AppSource): AppDefinition[] {
  if (!source) return [...BUILTIN_APPS]
  return BUILTIN_APPS.filter((a) => a.source === source)
}
