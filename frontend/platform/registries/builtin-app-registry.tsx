import type React from "react"
import type { AppDefinition, AppSource } from "@/platform/types/app-definition"
import { normalizeAppId } from "./app-id-policy"
import {
  Store,
  Terminal,
  Database,
  Globe,
  FileText,
  GitBranch,
  Cpu,
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
  HardDrive,
  Cloud,
  Images,
} from "lucide-react"

export const BUILTIN_ICON_MAP: Record<string, React.ReactNode> = {
  "app-store": <Store className="h-5 w-5" />,
  terminal: <Terminal className="h-5 w-5" />,
  "db-explorer": <Database className="h-5 w-5" />,
  "file-browser": <FileText className="h-5 w-5" />,
  storage: <HardDrive className="h-5 w-5" />,
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
  "google-drive": <Cloud className="h-5 w-5" />,
  "google-photos": <Images className="h-5 w-5" />,
  "google-contacts": <Users className="h-5 w-5" />,
  github: <GitBranch className="h-5 w-5" />,
  cloudflare: <Cloud className="h-5 w-5" />,
  settings: <Settings className="h-5 w-5" />,
}

export function normalizeBuiltinAppId(appId: string): string {
  return normalizeAppId(appId)
}

export function getIconById(iconId: string): React.ReactNode {
  return BUILTIN_ICON_MAP[iconId] || BUILTIN_ICON_MAP[normalizeBuiltinAppId(iconId)] || BUILTIN_ICON_MAP["wasm-generic"]
}

export const BUILTIN_APPS: AppDefinition[] = [
  { appId: "app-store", name: "App Store", description: "Install, open, and uninstall apps", iconId: "app-store", kind: "builtin", source: "builtin", componentKey: "app-store", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "terminal", name: "Terminal", description: "System shell & logs", iconId: "terminal", kind: "builtin", source: "builtin", componentKey: "terminal", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "db-explorer", name: "DB Explorer", description: "Query distributed state", iconId: "db-explorer", kind: "builtin", source: "builtin", componentKey: "db-explorer", requiredCapabilityIds: [], optionalCapabilityIds: ["node_connection"], status: "available" },
  { appId: "file-browser", name: "File Manager", description: "Open selected local folders through the browser filesystem boundary", iconId: "file-browser", kind: "builtin", source: "builtin", componentKey: "file-browser", requiredCapabilityIds: ["filesystem"], optionalCapabilityIds: [], status: "available" },
  { appId: "storage", name: "Storage", description: "Connect storage sources and build permission-scoped data pipelines", iconId: "storage", kind: "builtin", source: "builtin", componentKey: "storage", requiredCapabilityIds: [], optionalCapabilityIds: ["filesystem", "node_connection"], status: "available" },
  { appId: "git-sync", name: "Git Sync", description: "Decentralized repos", iconId: "git-sync", kind: "builtin", source: "builtin", componentKey: "git-sync", requiredCapabilityIds: [], optionalCapabilityIds: ["node_connection"], status: "available" },
  { appId: "web-server", name: "Web Server", description: "Serve static content", iconId: "web-server", kind: "builtin", source: "builtin", componentKey: "web-server", requiredCapabilityIds: [], optionalCapabilityIds: ["network_access"], status: "available" },
  { appId: "compute-node", name: "Compute", description: "Distributed processing", iconId: "compute-node", kind: "builtin", source: "builtin", componentKey: "compute-node", requiredCapabilityIds: [], optionalCapabilityIds: ["node_connection"], status: "available" },
  { appId: "people", name: "People", description: "Contacts, messages, and calls", iconId: "people", kind: "builtin", source: "builtin", componentKey: "people", requiredCapabilityIds: ["identity"], optionalCapabilityIds: ["voice_call", "messaging"], status: "available" },
  { appId: "trust-manager", name: "Trust Manager", description: "Inspect trust roots, delegations, capabilities, and audit trails", iconId: "trust-manager", kind: "builtin", source: "builtin", componentKey: "trust-manager", requiredCapabilityIds: ["identity"], optionalCapabilityIds: ["node_connection"], status: "available" },
  { appId: "workflow-builder", name: "Workflow Builder", description: "Create & manage automation workflows", iconId: "workflow-builder", kind: "builtin", source: "builtin", componentKey: "workflow-builder", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "finances", name: "Finances", description: "Finance hub for portfolio, wallet transfers, exchange, rewards, and settlement", iconId: "finances", kind: "builtin", source: "builtin", componentKey: "finances", requiredCapabilityIds: ["identity", "payments"], optionalCapabilityIds: ["node_connection"], status: "available" },
  { appId: "calculator", name: "Calculator", description: "System utility", iconId: "calculator", kind: "builtin", source: "builtin", componentKey: "calculator", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "help", name: "Help & Onboarding", description: "Platform guide & setup", iconId: "help", kind: "builtin", source: "builtin", componentKey: "help", requiredCapabilityIds: [], optionalCapabilityIds: [], status: "available" },
  { appId: "gmail", name: "Gmail", description: "Read mail with OAuth keys sealed into your Trust Container", iconId: "gmail", kind: "builtin", source: "builtin", componentKey: "gmail", requiredCapabilityIds: ["identity"], optionalCapabilityIds: ["messaging"], status: "available" },
  { appId: "google-drive", name: "Google Drive", description: "Browse Drive files with OAuth keys sealed into your Trust Container", iconId: "google-drive", kind: "builtin", source: "builtin", componentKey: "google-drive", requiredCapabilityIds: ["identity", "filesystem"], optionalCapabilityIds: [], status: "available" },
  { appId: "google-photos", name: "Google Photos", description: "Use Google Photos Picker with OAuth keys sealed into your Trust Container", iconId: "google-photos", kind: "builtin", source: "builtin", componentKey: "google-photos", requiredCapabilityIds: ["identity"], optionalCapabilityIds: ["filesystem"], status: "available" },
  { appId: "google-contacts", name: "Google Contacts", description: "Read Google contacts through the People API with sealed OAuth keys", iconId: "google-contacts", kind: "builtin", source: "builtin", componentKey: "google-contacts", requiredCapabilityIds: ["identity"], optionalCapabilityIds: [], status: "available" },
  { appId: "github", name: "GitHub", description: "Browse repositories with OAuth keys sealed into your Trust Container", iconId: "github", kind: "builtin", source: "builtin", componentKey: "github", requiredCapabilityIds: ["identity"], optionalCapabilityIds: ["network_access"], status: "available" },
  { appId: "cloudflare", name: "Cloudflare", description: "Manage zones and DNS records with an API token sealed into your Trust Container", iconId: "cloudflare", kind: "builtin", source: "builtin", componentKey: "cloudflare", requiredCapabilityIds: ["identity"], optionalCapabilityIds: ["network_access"], status: "available" },
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
