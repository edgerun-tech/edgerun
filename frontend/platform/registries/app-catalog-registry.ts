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
import {
  browserCatalogAppToPackageProjection,
  packageProjectionToAppDefinition,
} from "@/platform/runtime/verified-app-package"

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
  return packageProjectionToAppDefinition(browserCatalogAppToPackageProjection(app, installed))
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
    displayMetadata: {
      ...app.displayMetadata,
      verifiedAt: new Date().toISOString(),
    },
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
