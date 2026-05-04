import { atom, computed } from "nanostores"
import type { AppDefinition } from "@/platform/types/app-definition"

export interface AppCatalogState {
  apps: Map<string, AppDefinition>
}

const initialState: AppCatalogState = {
  apps: new Map(),
}

export const appCatalogRegistry = atom<AppCatalogState>(initialState)

export const catalogApps = computed(appCatalogRegistry, (state) =>
  Array.from(state.apps.values()),
)

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

export function installCatalogApp(appId: string): AppDefinition | undefined {
  const app = verifyCatalogApp(appId)
  if (!app) return undefined
  const installed: AppDefinition = {
    ...app,
    status: "installed",
    kind: app.kind === "preview" ? "installed" : app.kind,
  }
  registerCatalogApp(installed)
  return installed
}

const BUILTIN_CATALOG_APPS: AppDefinition[] = [
  {
    appId: "graph-studio-wasm",
    name: "Graph Studio",
    description: "WASM graph visualization app that can read graph data and draw overlays only when delegated by the user.",
    iconId: "wasm-generic",
    kind: "wasm",
    source: "catalog",
    wasmUrl: "/edgerun-app.wasm",
    requiredCapabilityIds: ["canvas.draw", "graph.read"],
    optionalCapabilityIds: ["graph.overlay.write", "xray_viewport_control"],
    status: "available",
    signature: {
      developerId: "developer:edgerun",
      developerName: "EdgeRun",
      packageHash: "demo:edgerun-app.wasm",
      manifestHash: "demo:graph-studio-manifest",
      verified: false,
      authorityRef: "developer:edgerun",
      proofRef: "catalog:graph-studio-wasm",
    },
    displayMetadata: {
      runtime: "wasm",
      showcase: true,
    },
  },
  {
    appId: "xray-lens-wasm",
    name: "Xray Lens",
    description: "WASM app that filters and annotates the Xray graph through capability-scoped host calls.",
    iconId: "wasm-generic",
    kind: "wasm",
    source: "catalog",
    wasmUrl: "/counter.wasm",
    requiredCapabilityIds: ["graph.read", "xray_viewport_control"],
    optionalCapabilityIds: ["canvas.draw"],
    status: "available",
    signature: {
      developerId: "developer:edgerun",
      developerName: "EdgeRun",
      packageHash: "demo:counter.wasm",
      manifestHash: "demo:xray-lens-manifest",
      verified: false,
      authorityRef: "developer:edgerun",
      proofRef: "catalog:xray-lens-wasm",
    },
    displayMetadata: {
      runtime: "wasm",
      showcase: true,
    },
  },
]

let seeded = false

export function seedBuiltinCatalogApps(): void {
  if (seeded) return
  seeded = true
  for (const app of BUILTIN_CATALOG_APPS) registerCatalogApp(app)
}
