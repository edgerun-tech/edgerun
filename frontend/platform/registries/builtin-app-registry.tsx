import type React from "react"
import type { AppDefinition, AppSource } from "@/platform/types/app-definition"
import {
  Store,
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
  Workflow,
  Package,
  Mail,
  Settings,
  Shield,
} from "lucide-react"

const APP_ID_ALIASES: Record<string, string> = {
  wallet: "finances",
  contacts: "people",
  calling: "people",
  chat: "people",
}

export const BUILTIN_ICON_MAP: Record<string, React.ReactNode> = {
  "app-store": <Store className="h-5 w-5" />,
  terminal: <Terminal className="h-5 w-5" />,
  "code-runner": <Code2 className="h-5 w-5" />,
  "db-explorer": <Database className="h-5 w-5" />,
  "network-monitor": <Network className="h-5 w-5" />,
  "file-browser": <FileText className="h-5 w-5" />,
  "git-sync": <GitBranch className="h-5 w-5" />,
  "web-server": <Globe className="h-5 w-5" />,
  "compute-node": <Cpu className="h-5 w-5" />,
  people: <Users className="h-5 w-5" />,
  contacts: <Users className="h-5 w-5" />,
  calling: <Phone className="h-5 w-5" />,
  chat: <MessageSquare className="h-5 w-5" />,
  "trust-manager": <Shield className="h-5 w-5" />,
  "workflow-builder": <Workflow className="h-5 w-5" />,
  finances: <Wallet className="h-5 w-5" />,
  wallet: <Wallet className="h-5 w-5" />,
  calculator: <Calculator className="h-5 w-5" />,
  help: <HelpCircle className="h-5 w-5" />,
  "wasm-generic": <Package className="h-5 w-5" />,
  gmail: <Mail className="h-5 w-5" />,
  settings: <Settings className="h-5 w-5" />,
}

export function normalizeBuiltinAppId(appId: string): string {
  return APP_ID_ALIASES[appId] || appId
}

export function getIconById(iconId: string): React.ReactNode {
  return BUILTIN_ICON_MAP[iconId] || BUILTIN_ICON_MAP[normalizeBuiltinAppId(iconId)] || BUILTIN_ICON_MAP["wasm-generic"]
}

export const BUILTIN_APPS: AppDefinition[] = [
  { appId: "app-store", name: "App Store", description: "Install, open, and uninstall apps", iconId: "app-store", kind: "builtin", source: "builtin", componentKey: "app-store", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "terminal", name: "Terminal", description: "System shell & logs", iconId: "terminal", kind: "builtin", source: "builtin", componentKey: "terminal", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "code-runner", name: "AS Compiler", description: "Compile AssemblyScript to WASM in-browser", iconId: "code-runner", kind: "builtin", source: "builtin", componentKey: "code-runner", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "db-explorer", name: "DB Explorer", description: "Query distributed state", iconId: "db-explorer", kind: "builtin", source: "builtin", componentKey: "db-explorer", requiredCapabilityIds: [], optionalCapabilityIds: ["node_connection"], status: "available" },
  { appId: "network-monitor", name: "Network", description: "Codelyzer bridge and active graph connections", iconId: "network-monitor", kind: "builtin", source: "builtin", componentKey: "network-monitor", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "file-browser", name: "Files", description: "Virtual filesystem", iconId: "file-browser", kind: "builtin", source: "builtin", componentKey: "file-browser", requiredCapabilityIds: [], optionalCapabilityIds: ["filesystem"], status: "available" },
  { appId: "git-sync", name: "Git Sync", description: "Decentralized repos", iconId: "git-sync", kind: "builtin", source: "builtin", componentKey: "git-sync", requiredCapabilityIds: [], optionalCapabilityIds: ["node_connection"], status: "available" },
  { appId: "web-server", name: "Web Server", description: "Serve static content", iconId: "web-server", kind: "builtin", source: "builtin", componentKey: "web-server", requiredCapabilityIds: [], optionalCapabilityIds: ["network_access"], status: "available" },
  { appId: "compute-node", name: "Compute", description: "Distributed processing", iconId: "compute-node", kind: "builtin", source: "builtin", componentKey: "compute-node", requiredCapabilityIds: [], optionalCapabilityIds: ["node_connection"], status: "available" },
  { appId: "people", name: "People", description: "Contacts, messages, and calls", iconId: "people", kind: "builtin", source: "builtin", componentKey: "people", requiredCapabilityIds: ["identity"], optionalCapabilityIds: ["voice_call", "messaging"], status: "available" },
  { appId: "trust-manager", name: "Trust Manager", description: "Inspect trust roots, delegations, capabilities, and audit trails", iconId: "trust-manager", kind: "builtin", source: "builtin", componentKey: "trust-manager", requiredCapabilityIds: ["identity"], optionalCapabilityIds: ["node_connection"], status: "available" },
  { appId: "workflow-builder", name: "Workflow Builder", description: "Create & manage automation workflows", iconId: "workflow-builder", kind: "builtin", source: "builtin", componentKey: "workflow-builder", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "finances", name: "Finances", description: "Finance hub for portfolio, wallet transfers, exchange, rewards, and settlement", iconId: "finances", kind: "builtin", source: "builtin", componentKey: "finances", requiredCapabilityIds: ["identity", "payments"], optionalCapabilityIds: ["node_connection"], status: "available" },
  { appId: "calculator", name: "Calculator", description: "System utility", iconId: "calculator", kind: "builtin", source: "builtin", componentKey: "calculator", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "help", name: "Help & Onboarding", description: "Platform guide & setup", iconId: "help", kind: "builtin", source: "builtin", componentKey: "help", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "gmail", name: "Gmail", description: "Read and send emails via Google", iconId: "gmail", kind: "builtin", source: "builtin", componentKey: "gmail", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "settings", name: "Settings", description: "System preferences & configuration", iconId: "settings", kind: "builtin", source: "builtin", componentKey: "settings", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
]

export function getBuiltinApp(appId: string): AppDefinition | undefined {
  const normalizedAppId = normalizeBuiltinAppId(appId)
  return BUILTIN_APPS.find((a) => a.appId === normalizedAppId)
}

export function listBuiltinApps(source?: AppSource): AppDefinition[] {
  if (!source) return [...BUILTIN_APPS]
  return BUILTIN_APPS.filter((a) => a.source === source)
}
