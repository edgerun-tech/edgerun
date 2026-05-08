import { atom, computed } from "nanostores"
import type { AppDefinition } from "@/platform/types/app-definition"
import {
  browserAppInstallStore,
  installBrowserCatalogApp,
  loadBrowserAppCatalog,
  uninstallBrowserApp,
  type BrowserCatalogApp,
  type InstalledBrowserApp,
} from "@/platform/runtime/browser-app-install-store"

export interface AppCatalogState {
  apps: Map<string, AppDefinition>
}

export const appCatalogRegistry = atom<AppCatalogState>({
  apps: new Map(),
})

export const catalogApps = computed(appCatalogRegistry, (state) =>
  Array.from(state.apps.values()),
)

function browserCatalogAppToDefinition(app: BrowserCatalogApp, installed?: InstalledBrowserApp): AppDefinition {
  return {
    appId: app.appId,
    name: app.name,
    description: app.description,
    iconId: "wasm-generic",
    kind: "wasm",
    source: "catalog",
    requiredCapabilityIds: app.requiredCapabilityIds,
    optionalCapabilityIds: app.optionalCapabilityIds,
    status: installed ? "installed" : "available",
    signature: {
      developerId: app.developer,
      developerName: app.developer,
      packageHash: app.eappSha256,
      manifestHash: app.manifestSha256,
      verified: Boolean(installed),
      verifiedAt: installed?.installedAt,
      authorityRef: "catalog:/apps/catalog.ecat",
      proofRef: app.packageUrl,
    },
    displayMetadata: {
      runtimeManifest: {
        runtime: "browser-iframe",
        permissions: app.requiredCapabilityIds,
        optionalPermissions: app.optionalCapabilityIds,
      },
      runtime: "browser-iframe",
      packageUrl: app.packageUrl,
      manifestUrl: app.manifestUrl,
      launchUrl: app.launchUrl,
      eappSha256: app.eappSha256,
      packageBytes: app.packageBytes,
      runtimeAppId: installed?.runtimeAppId ?? app.runtimeAppId,
      releaseId: installed?.releaseId ?? app.releaseId,
      developerId: installed?.developerId ?? app.developerId,
      installedAt: installed?.installedAt,
      verifiedAssets: installed?.verifiedAssets.length ?? 0,
    },
  }
}

export function registerCatalogApp(app: AppDefinition): void {
  const state = appCatalogRegistry.get()
  const apps = new Map(state.apps)
  apps.set(app.appId, app)
  appCatalogRegistry.set({ apps })
}

export function getCatalogApp(appId: string): AppDefinition | undefined {
  return appCatalogRegistry.get().apps.get(appId)
}

export function listCatalogApps(): AppDefinition[] {
  return Array.from(appCatalogRegistry.get().apps.values())
}

export function syncBrowserCatalogRegistry(): void {
  const state = browserAppInstallStore.get()
  const apps = new Map<string, AppDefinition>()
  for (const app of state.catalog?.apps ?? []) {
    apps.set(app.appId, browserCatalogAppToDefinition(app, state.installed.get(app.appId)))
  }
  appCatalogRegistry.set({ apps })
}

export async function seedBuiltinCatalogApps(): Promise<void> {
  await loadBrowserAppCatalog()
  syncBrowserCatalogRegistry()
}

export function verifyCatalogApp(appId: string): AppDefinition | undefined {
  const app = getCatalogApp(appId)
  if (!app) return undefined
  const verified: AppDefinition = {
    ...app,
    signature: app.signature ? {
      ...app.signature,
      verified: true,
      verifiedAt: new Date().toISOString(),
    } : undefined,
  }
  registerCatalogApp(verified)
  return verified
}

export async function installCatalogApp(appId: string): Promise<AppDefinition | undefined> {
  const source = browserAppInstallStore.get().catalog?.apps.find((app) => app.appId === appId)
  if (!source) return undefined
  const installed = await installBrowserCatalogApp(source)
  const app = browserCatalogAppToDefinition(source, installed)
  registerCatalogApp(app)
  return app
}

export async function uninstallCatalogApp(appId: string): Promise<void> {
  await uninstallBrowserApp(appId)
  syncBrowserCatalogRegistry()
}
