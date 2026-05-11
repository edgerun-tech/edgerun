"use client"

import dynamic from "next/dynamic"
import type { AppDefinition } from "@/platform/types/app-definition"
import { getComponent } from "@/platform/registries/component-registry"

const LoadingApp = () => <div className="p-4 text-sm text-muted-foreground">Loading app...</div>

const AppStoreApp = dynamic(
  () => import("@/components/os/app-store").then((mod) => mod.AppStore),
  { ssr: false, loading: LoadingApp },
)
const TrustManagerSurface = dynamic(
  () => import("@/components/os/trust-manager-real-app").then((mod) => mod.TrustManagerSurface),
  { ssr: false, loading: LoadingApp },
)
const OnboardingApp = dynamic(
  () => import("@/components/os/onboarding-app").then((mod) => mod.OnboardingApp),
  { ssr: false, loading: LoadingApp },
)
const FileManagerApp = dynamic(
  () => import("@/components/os/file-manager").then((mod) => mod.FileManager),
  { ssr: false, loading: LoadingApp },
)
const StorageDashboard = dynamic(
  () => import("@/components/os/storage-dashboard").then((mod) => mod.StorageDashboard),
  { ssr: false, loading: LoadingApp },
)
const GmailApp = dynamic(
  () => import("@/components/os/gmail-app").then((mod) => mod.GmailApp),
  { ssr: false, loading: LoadingApp },
)
const GoogleDriveApp = dynamic(
  () => import("@/components/os/google-drive-app").then((mod) => mod.GoogleDriveApp),
  { ssr: false, loading: LoadingApp },
)
const GooglePhotosApp = dynamic(
  () => import("@/components/os/google-photos-app").then((mod) => mod.GooglePhotosApp),
  { ssr: false, loading: LoadingApp },
)
const GoogleContactsApp = dynamic(
  () => import("@/components/os/google-contacts-app").then((mod) => mod.GoogleContactsApp),
  { ssr: false, loading: LoadingApp },
)
const GitHubApp = dynamic(
  () => import("@/components/os/github-app").then((mod) => mod.GitHubApp),
  { ssr: false, loading: LoadingApp },
)
const CloudflareApp = dynamic(
  () => import("@/components/os/cloudflare-app").then((mod) => mod.CloudflareApp),
  { ssr: false, loading: LoadingApp },
)
const SettingsApp = dynamic(
  () => import("@/components/os/settings-app").then((mod) => mod.SettingsApp),
  { ssr: false, loading: LoadingApp },
)
const IdentityApp = dynamic(
  () => import("@/components/os/identity-app").then((mod) => mod.IdentityApp),
  { ssr: false, loading: LoadingApp },
)
const ComputeNodeApp = dynamic(
  () => import("@/components/os/compute-node").then((mod) => mod.ComputeNode),
  { ssr: false, loading: LoadingApp },
)

interface BuiltinAppHostProps {
  app: AppDefinition
  onLaunchApp?: (app: AppDefinition) => void
}

export function BuiltinAppHost({ app, onLaunchApp }: BuiltinAppHostProps) {
  switch (app.appId) {
    case "identity":
      return <IdentityApp />
    case "app-store":
      return <AppStoreApp onLaunchApp={(a: AppDefinition) => onLaunchApp?.(a)} />
    case "help":
      return <OnboardingApp />
    case "people":
    case "contacts":
    case "calling":
    case "chat":
    case "finances":
    case "wallet":
    case "calculator":
    case "workflow-builder":
      return <div className="p-4 text-muted-foreground">Desktop demo app disabled: {app.name}</div>
    case "trust-manager":
      return <TrustManagerSurface onLaunchApp={(a: AppDefinition) => onLaunchApp?.(a)} />
    case "file-browser":
      return <FileManagerApp />
    case "storage":
      return <StorageDashboard />
    case "gmail":
      return <GmailApp />
    case "google-drive":
      return <GoogleDriveApp />
    case "google-photos":
      return <GooglePhotosApp />
    case "google-contacts":
      return <GoogleContactsApp />
    case "github":
      return <GitHubApp />
    case "cloudflare":
      return <CloudflareApp />
    case "settings":
      return <SettingsApp />
    case "compute-node":
      return <ComputeNodeApp />
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
