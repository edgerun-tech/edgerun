/**
 * Canonical app surface exports.
 *
 * This folder represents full app-level UI. During migration, these exports may
 * point at mature implementations that still physically live under
 * `components/os`. New app surfaces should land here directly.
 */

export { SettingsApp } from "@/components/os/settings-app"
export { FinancesApp } from "@/components/os/finances-app"
export { TrustManagerSurface } from "@/components/os/trust-manager-surface"
export { PeopleApp } from "@/components/os/people-app"
export { Terminal } from "@/components/os/terminal"
export { FileManager } from "@/components/os/file-manager"
export { GmailApp } from "@/components/os/gmail-app"
export { AppStore } from "@/components/os/app-store"
export { WorkflowBuilder } from "@/components/os/workflow-builder"
export { CalculatorApp } from "@/components/os/calculator-app"
export { HelpApp } from "@/components/os/help-app"
