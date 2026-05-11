"use client"

import { atom, computed } from "nanostores"
import { runtimeEventLog } from "./runtime-event-log"
import { sha256Hex } from "./browser-capability-types"
import { decodeAppStoreCatalogWithEdgerunNode, installEappWithEdgerunNode } from "./edgerun-node"

const APP_STORE_CATALOG_URL = "/apps/catalog.ecat"
const DB_NAME = "edgerun-browser-apps"
const DB_VERSION = 1
const STORE_NAME = "packages"

export interface BrowserCatalogAsset {
  path: string
  sha256: string
  bytes: number
}

export interface BrowserCatalogApp {
  appId: string
  runtimeAppId?: string
  releaseId?: string
  developerId?: string
  slug: string
  name: string
  version: string
  description: string
  runtime: "browser-iframe"
  packageUrl: string
  manifestUrl: string
  launchUrl: string
  eappSha256: string
  manifestSha256: string
  packageBytes: number
  developer: string
  requiredCapabilityIds: string[]
  optionalCapabilityIds: string[]
  assets: BrowserCatalogAsset[]
}

export interface BrowserAppCatalog {
  format: "edgerun-app-store-catalog-rkyv-v1"
  generatedAt: number
  sequence: number
  storeId: string
  signatureVerified: boolean
  apps: BrowserCatalogApp[]
}

export interface CachedBrowserApp {
  appId: string
  slug: string
  name: string
  version: string
  description: string
  developer: string
  packageUrl: string
  manifestUrl: string
  launchUrl: string
  eappSha256: string
  manifestSha256: string
  packageBytes: number
  runtimeAppId: string
  releaseId: string
  developerId: string
  cachedAt: string
  manifest: unknown
  eappBytes: ArrayBuffer
  verifiedAssets: BrowserCatalogAsset[]
}

/** @deprecated Use CachedBrowserApp. */
export type InstalledBrowserApp = CachedBrowserApp & { installedAt?: string }

export interface BrowserAppCacheState {
  catalog: BrowserAppCatalog | null
  cached: Map<string, CachedBrowserApp>
  isLoading: boolean
  error: string | null
}

/** @deprecated Use BrowserAppCacheState. */
export interface BrowserAppInstallState {
  catalog: BrowserAppCatalog | null
  installed: Map<string, InstalledBrowserApp>
  isLoading: boolean
  error: string | null
}

export const browserAppCacheStore = atom<BrowserAppCacheState>({
  catalog: null,
  cached: new Map(),
  isLoading: false,
  error: null,
})

export const cachedBrowserApps = computed(browserAppCacheStore, (state) =>
  Array.from(state.cached.values()),
)

/** @deprecated Use browserAppCacheStore. */
export const browserAppInstallStore = computed(browserAppCacheStore, (state): BrowserAppInstallState => ({
  catalog: state.catalog,
  installed: new Map(Array.from(state.cached.entries()).map(([id, app]) => [id, { ...app, installedAt: app.cachedAt }])),
  isLoading: state.isLoading,
  error: state.error,
}))

/** @deprecated Use cachedBrowserApps. */
export const installedBrowserApps = computed(browserAppInstallStore, (state) =>
  Array.from(state.installed.values()),
)

function normalizeStoreId(value: string): string {
  const normalized = value.trim().toLowerCase()
  if (!/^[0-9a-f]{64}$/.test(normalized)) {
    throw new Error("NEXT_PUBLIC_EDGERUN_APP_STORE_ID must be 32 hex bytes")
  }
  return normalized
}

function expectedAppStoreId(): string {
  const configured = process.env.NEXT_PUBLIC_EDGERUN_APP_STORE_ID
  if (!configured) {
    throw new Error("NEXT_PUBLIC_EDGERUN_APP_STORE_ID must be configured")
  }
  return normalizeStoreId(configured)
}

function bytesToArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength)
  copy.set(bytes)
  return copy.buffer
}

function normalizeCachedApp(app: CachedBrowserApp | InstalledBrowserApp): CachedBrowserApp {
  return {
    ...app,
    cachedAt: app.cachedAt || app.installedAt || new Date().toISOString(),
  }
}

async function fetchBytes(url: string): Promise<Uint8Array> {
  const response = await fetch(url, { cache: "no-store" })
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`)
  return new Uint8Array(await response.arrayBuffer())
}

async function expectHash(url: string, expectedSha256: string): Promise<Uint8Array> {
  const bytes = await fetchBytes(url)
  const actual = await sha256Hex(bytes)
  if (actual !== expectedSha256) {
    throw new Error(`${url} hash mismatch: expected ${expectedSha256}, got ${actual}`)
  }
  return bytes
}

function appAssetUrl(app: BrowserCatalogApp, assetPath: string): string {
  return `/apps/${app.slug}/${assetPath}`
}

async function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION)
    request.onupgradeneeded = () => {
      const db = request.result
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: "appId" })
      }
    }
    request.onerror = () => reject(request.error)
    request.onsuccess = () => resolve(request.result)
  })
}

async function idbGetAll(): Promise<CachedBrowserApp[]> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, "readonly")
    const request = tx.objectStore(STORE_NAME).getAll()
    request.onerror = () => reject(request.error)
    request.onsuccess = () => resolve((request.result as Array<CachedBrowserApp | InstalledBrowserApp>).map(normalizeCachedApp))
    tx.oncomplete = () => db.close()
  })
}

async function idbPut(app: CachedBrowserApp): Promise<void> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, "readwrite")
    tx.objectStore(STORE_NAME).put(app)
    tx.onerror = () => reject(tx.error)
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
  })
}

async function idbDelete(appId: string): Promise<void> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, "readwrite")
    tx.objectStore(STORE_NAME).delete(appId)
    tx.onerror = () => reject(tx.error)
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
  })
}

export async function loadBrowserAppCatalog(): Promise<void> {
  browserAppCacheStore.set({ ...browserAppCacheStore.get(), isLoading: true, error: null })
  try {
    const [catalog, cached] = await Promise.all([loadCatalogThroughNode(), idbGetAll()])
    browserAppCacheStore.set({
      catalog,
      cached: new Map(cached.map((app) => [app.appId, app])),
      isLoading: false,
      error: null,
    })
  } catch (error) {
    browserAppCacheStore.set({
      ...browserAppCacheStore.get(),
      isLoading: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}

async function loadCatalogThroughNode(): Promise<BrowserAppCatalog> {
  const response = await fetch(APP_STORE_CATALOG_URL, { cache: "no-store" })
  if (!response.ok) {
    throw new Error(`${APP_STORE_CATALOG_URL}: HTTP ${response.status}`)
  }
  const decoded = await decodeAppStoreCatalogWithEdgerunNode(new Uint8Array(await response.arrayBuffer()))
  if (!decoded.signatureVerified) {
    throw new Error("app store catalog signature was not verified")
  }
  if (decoded.storeId !== expectedAppStoreId()) {
    throw new Error("app store catalog was signed by an unexpected store")
  }
  return decoded
}

export async function cacheBrowserCatalogApp(app: BrowserCatalogApp): Promise<CachedBrowserApp> {
  const [eappBytes] = await Promise.all([
    expectHash(app.packageUrl, app.eappSha256),
    expectHash(app.manifestUrl, app.manifestSha256),
  ])
  await installEappWithEdgerunNode(eappBytes)
  const manifest = catalogManifestMetadata(app)

  const verifiedAssets: BrowserCatalogAsset[] = []
  for (const asset of app.assets) {
    await expectHash(appAssetUrl(app, asset.path), asset.sha256)
    verifiedAssets.push(asset)
  }

  const cached: CachedBrowserApp = {
    appId: app.appId,
    slug: app.slug,
    name: app.name,
    version: app.version,
    description: app.description,
    developer: app.developer,
    packageUrl: app.packageUrl,
    manifestUrl: app.manifestUrl,
    launchUrl: app.launchUrl,
    eappSha256: app.eappSha256,
    manifestSha256: app.manifestSha256,
    packageBytes: eappBytes.byteLength,
    runtimeAppId: app.runtimeAppId || "",
    releaseId: app.releaseId || "",
    developerId: app.developerId || "",
    cachedAt: new Date().toISOString(),
    manifest,
    eappBytes: bytesToArrayBuffer(eappBytes),
    verifiedAssets,
  }

  await idbPut(cached)
  const state = browserAppCacheStore.get()
  const cachedMap = new Map(state.cached)
  cachedMap.set(cached.appId, cached)
  browserAppCacheStore.set({ ...state, cached: cachedMap })
  runtimeEventLog.append({
    kind: "app_package_verified",
    actor: cached.appId,
    reason: "browser node verified and cached rkyv eapp package",
    inputHash: cached.eappSha256,
    metadata: {
      packageUrl: cached.packageUrl,
      manifestSha256: cached.manifestSha256,
      packageBytes: cached.packageBytes,
      runtimeAppId: cached.runtimeAppId,
      releaseId: cached.releaseId,
      verifiedAssets: cached.verifiedAssets.length,
      cacheState: "cached",
    },
  })
  return cached
}

/** @deprecated Use cacheBrowserCatalogApp. */
export const installBrowserCatalogApp = cacheBrowserCatalogApp

function catalogManifestMetadata(app: BrowserCatalogApp): Record<string, unknown> & { name: string; version: string } {
  return {
    format: "edgerun-app-manifest-from-signed-catalog-v1",
    appId: app.runtimeAppId,
    developerId: app.developerId,
    slug: app.slug,
    name: app.name,
    version: app.version,
    summary: app.description,
  }
}

export async function removeCachedBrowserApp(appId: string): Promise<void> {
  await idbDelete(appId)
  const state = browserAppCacheStore.get()
  const cached = new Map(state.cached)
  cached.delete(appId)
  browserAppCacheStore.set({ ...state, cached })
  runtimeEventLog.append({
    kind: "app_stopped",
    actor: appId,
    reason: "browser node removed cached app package",
  })
}

/** @deprecated Use removeCachedBrowserApp. */
export const uninstallBrowserApp = removeCachedBrowserApp
