import type { AppDefinition } from "@/platform/types/app-definition"
import { getComponent } from "@/platform/registries/component-registry"
import { Terminal } from "@/components/os/terminal"
import { CodeRunner } from "@/components/os/code-runner"
import { ResourceMonitor } from "@/components/os/resource-monitor"
import { AppStore as AppStoreComponent } from "@/components/os/app-store"
import { ContactsApp } from "@/components/os/contacts-app"
import { CallingApp } from "@/components/os/calling-app"
import { DemoChatApp as ChatApp } from "@/components/os/chat-app"
import { WalletApp } from "@/components/os/wallet-app"
import { CalculatorApp } from "@/components/os/calculator-app"
import { HelpApp } from "@/components/os/help-app"
import { AIAssistant } from "@/components/os/ai-assistant"
import { WorkflowBuilder } from "@/components/os/workflow-builder"
import { FileManager } from "@/components/os/file-manager"
import { GmailApp } from "@/components/os/gmail-app"
import { SettingsApp } from "@/components/os/settings-app"

interface BuiltinAppHostProps {
  app: AppDefinition
  onLaunchApp?: (app: AppDefinition) => void
}

export function BuiltinAppHost({ app, onLaunchApp }: BuiltinAppHostProps) {
  switch (app.appId) {
    case "terminal":
      return <Terminal logs={[]} onCommand={() => {}} />
    case "code-runner":
      return <CodeRunner />
    case "resource-monitor":
      return <ResourceMonitor />
    case "app-store":
      return (
        <AppStoreComponent
          onLaunchApp={(a) => onLaunchApp?.(a)}
          onAppBlocked={() => {}}
          runningApps={[]}
        />
      )
    case "contacts":
      return <ContactsApp />
    case "calling":
      return <CallingApp />
    case "chat":
      return <ChatApp />
    case "wallet":
      return <WalletApp />
    case "calculator":
      return <CalculatorApp />
    case "help":
      return <HelpApp />
    case "ai-assistant":
      return <AIAssistant />
    case "workflow-builder":
      return <WorkflowBuilder onClose={() => {}} />
    case "file-browser":
      return <FileManager />
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
