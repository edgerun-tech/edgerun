"use client"

import dynamic from "next/dynamic"
import type { AppDefinition } from "@/platform/types/app-definition"
import { getComponent } from "@/platform/registries/component-registry"

const LoadingApp = () => <div className="p-4 text-sm text-muted-foreground">Loading app...</div>

const TerminalApp = dynamic(
  () => import("@/components/os/terminal").then((mod) => mod.Terminal),
  { ssr: false, loading: LoadingApp },
)
const CodeRunnerApp = dynamic(
  () => import("@/components/os/code-runner").then((mod) => mod.CodeRunner),
  { ssr: false, loading: LoadingApp },
)
const ResourceMonitorApp = dynamic(
  () => import("@/components/os/resource-monitor").then((mod) => mod.ResourceMonitor),
  { ssr: false, loading: LoadingApp },
)
const AppStoreApp = dynamic(
  () => import("@/components/os/app-store").then((mod) => mod.AppStore),
  { ssr: false, loading: LoadingApp },
)
const PeopleApp = dynamic(
  () => import("@/components/os/people-app").then((mod) => mod.PeopleApp),
  { ssr: false, loading: LoadingApp },
)
const TrustManagerApp = dynamic(
  () => import("@/components/os/trust-manager-app").then((mod) => mod.TrustManagerApp),
  { ssr: false, loading: LoadingApp },
)
const FinancesApp = dynamic(
  () => import("@/components/os/finances-app").then((mod) => mod.FinancesApp),
  { ssr: false, loading: LoadingApp },
)
const CalculatorApp = dynamic(
  () => import("@/components/os/calculator-app").then((mod) => mod.CalculatorApp),
  { ssr: false, loading: LoadingApp },
)
const HelpApp = dynamic(
  () => import("@/components/os/help-app").then((mod) => mod.HelpApp),
  { ssr: false, loading: LoadingApp },
)
const AIAssistantApp = dynamic(
  () => import("@/components/os/ai-assistant").then((mod) => mod.AIAssistant),
  { ssr: false, loading: LoadingApp },
)
const WorkflowBuilderApp = dynamic(
  () => import("@/components/os/workflow-builder").then((mod) => mod.WorkflowBuilder),
  { ssr: false, loading: LoadingApp },
)
const FileManagerApp = dynamic(
  () => import("@/components/os/file-manager").then((mod) => mod.FileManager),
  { ssr: false, loading: LoadingApp },
)
const GmailApp = dynamic(
  () => import("@/components/os/gmail-app").then((mod) => mod.GmailApp),
  { ssr: false, loading: LoadingApp },
)
const SettingsApp = dynamic(
  () => import("@/components/os/settings-app").then((mod) => mod.SettingsApp),
  { ssr: false, loading: LoadingApp },
)

interface BuiltinAppHostProps {
  app: AppDefinition
  onLaunchApp?: (app: AppDefinition) => void
}

export function BuiltinAppHost({ app, onLaunchApp }: BuiltinAppHostProps) {
  switch (app.appId) {
    case "terminal":
      return <TerminalApp logs={[]} onCommand={() => {}} />
    case "code-runner":
      return <CodeRunnerApp />
    case "resource-monitor":
      return <ResourceMonitorApp />
    case "app-store":
      return <AppStoreApp onLaunchApp={(a: AppDefinition) => onLaunchApp?.(a)} />
    case "people":
    case "contacts":
    case "calling":
    case "chat":
      return <PeopleApp />
    case "trust-manager":
      return <TrustManagerApp />
    case "wallet":
      return <FinancesApp />
    case "calculator":
      return <CalculatorApp />
    case "help":
      return <HelpApp />
    case "ai-assistant":
      return <AIAssistantApp />
    case "workflow-builder":
      return <WorkflowBuilderApp onClose={() => {}} />
    case "file-browser":
      return <FileManagerApp />
    case "gmail":
      return <GmailApp />
    case "settings":
      return <SettingsApp />
    case "app-studio":
      return <div className="p-4 text-muted-foreground">App Studio coming soon</div>
    default: {
      const registered = getComponent(app.componentKey || app.appId)
      if (registered?.isSafe) {
        const Component = registered.component
        return <Component />
      }
      return <div className="p-4 text-muted-foreground">Builtin app not implemented: {app.name}</div>
    }
  }
}
