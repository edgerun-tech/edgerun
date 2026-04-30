"use client"

import { Terminal, Code2, Database, Globe, FileText, GitBranch, Cpu, Network, HelpCircle, Users, Phone, MessageSquare, Wallet, Calculator } from "lucide-react"
import { cn } from "@/lib/utils"

export interface AppDefinition {
  id: string
  name: string
  description: string
  icon: React.ReactNode
  ram: string
  cpu: string
  price: string | "Free"
}

export const availableApps: AppDefinition[] = [
  {
    id: "terminal",
    name: "Terminal",
    description: "System shell & logs",
    icon: <Terminal className="h-5 w-5" />,
    ram: "32MB",
    cpu: "0.1%",
    price: "Free",
  },
  {
    id: "code-runner",
    name: "Code Runner",
    description: "Execute WASM modules",
    icon: <Code2 className="h-5 w-5" />,
    ram: "128MB",
    cpu: "2.5%",
    price: "Free",
  },
  {
    id: "db-explorer",
    name: "DB Explorer",
    description: "Query distributed state",
    icon: <Database className="h-5 w-5" />,
    ram: "64MB",
    cpu: "0.5%",
    price: "$5/mo",
  },
  {
    id: "network-monitor",
    name: "Network",
    description: "P2P connection status",
    icon: <Network className="h-5 w-5" />,
    ram: "48MB",
    cpu: "0.3%",
    price: "Free",
  },
  {
    id: "file-browser",
    name: "Files",
    description: "Virtual filesystem",
    icon: <FileText className="h-5 w-5" />,
    ram: "24MB",
    cpu: "0.1%",
    price: "Free",
  },
  {
    id: "git-sync",
    name: "Git Sync",
    description: "Decentralized repos",
    icon: <GitBranch className="h-5 w-5" />,
    ram: "96MB",
    cpu: "1.2%",
    price: "$3/mo",
  },
  {
    id: "web-server",
    name: "Web Server",
    description: "Serve static content",
    icon: <Globe className="h-5 w-5" />,
    ram: "64MB",
    cpu: "0.8%",
    price: "Free",
  },
  {
    id: "compute-node",
    name: "Compute",
    description: "Distributed processing",
    icon: <Cpu className="h-5 w-5" />,
    ram: "256MB",
    cpu: "5.0%",
    price: "$10/mo",
  },
  {
    id: "contacts",
    name: "Contacts",
    description: "Manage your peer network",
    icon: <Users className="h-5 w-5" />,
    ram: "12MB",
    cpu: "0.0%",
    price: "Free",
  },
  {
    id: "calling",
    name: "Calling",
    description: "Encrypted P2P voice calls",
    icon: <Phone className="h-5 w-5" />,
    ram: "48MB",
    cpu: "1.0%",
    price: "Free",
  },
  {
    id: "chat",
    name: "Chat",
    description: "E2E encrypted messaging",
    icon: <MessageSquare className="h-5 w-5" />,
    ram: "24MB",
    cpu: "0.2%",
    price: "Free",
  },
  {
    id: "wallet",
    name: "Wallet",
    description: "EDGE token & payments",
    icon: <Wallet className="h-5 w-5" />,
    ram: "16MB",
    cpu: "0.1%",
    price: "Free",
  },
  {
    id: "calculator",
    name: "Calculator",
    description: "System utility",
    icon: <Calculator className="h-5 w-5" />,
    ram: "4MB",
    cpu: "0.0%",
    price: "Free",
  },
  {
    id: "help",
    name: "Help & Onboarding",
    description: "Platform guide & setup",
    icon: <HelpCircle className="h-5 w-5" />,
    ram: "2MB",
    cpu: "0.0%",
    price: "Free",
  },
]

interface AppStoreProps {
  onLaunchApp: (app: AppDefinition) => void
  runningApps: string[]
}

export function AppStore({ onLaunchApp, runningApps }: AppStoreProps) {
  return (
    <div className="flex h-full flex-col p-4">
      <div className="mb-4">
        <h2 className="text-lg font-semibold text-foreground">App Store</h2>
        <p className="text-xs text-muted-foreground">Launch distributed applications</p>
      </div>

      <div className="grid flex-1 grid-cols-2 gap-3 overflow-auto">
        {availableApps.map((app) => {
          const isRunning = runningApps.includes(app.id)
          return (
            <button
              key={app.id}
              onClick={() => onLaunchApp(app)}
              className={cn(
                "group flex flex-col rounded-lg border border-border bg-secondary/50 p-3 text-left transition-all hover:border-primary/50 hover:bg-secondary",
                isRunning && "border-primary/30 bg-primary/5"
              )}
            >
              <div className="flex items-start justify-between">
                <div className={cn(
                  "flex h-9 w-9 items-center justify-center rounded-lg bg-muted text-muted-foreground transition-colors group-hover:bg-primary group-hover:text-primary-foreground",
                  isRunning && "bg-primary/20 text-primary"
                )}>
                  {app.icon}
                </div>
                <span className={cn(
                  "rounded px-1.5 py-0.5 text-[10px] font-medium",
                  app.price === "Free" 
                    ? "bg-primary/20 text-primary" 
                    : "bg-[var(--status-warning)]/20 text-[var(--status-warning)]"
                )}>
                  {app.price}
                </span>
              </div>

              <div className="mt-2">
                <div className="flex items-center gap-1.5">
                  <h3 className="text-sm font-medium text-foreground">{app.name}</h3>
                  {isRunning && (
                    <span className="h-1.5 w-1.5 rounded-full bg-[var(--status-online)]" />
                  )}
                </div>
                <p className="text-xs text-muted-foreground">{app.description}</p>
              </div>

              <div className="mt-auto flex items-center gap-3 pt-2 text-[10px] text-muted-foreground">
                <span>RAM: {app.ram}</span>
                <span>CPU: {app.cpu}</span>
              </div>
            </button>
          )
        })}
      </div>
    </div>
  )
}
