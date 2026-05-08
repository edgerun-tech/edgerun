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

export interface InstalledBrowserApp {
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
  installedAt: string
  manifest: unknown
  eappBytes: ArrayBuffer
  verifiedAssets: BrowserCatalogAsset[]
}

export interface BrowserAppInstallState {
  catalog: BrowserAppCatalog | null
  installed: Map<string, InstalledBrowserApp>
  isLoading: boolean
  error: string | null
}

export const browserAppInstallStore = atom<BrowserAppInstallState>({
  catalog: null,
  installed: new Map(),
  isLoading: false,
  error: null,
})

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

async function idbGetAll(): Promise<InstalledBrowserApp[]> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, "readonly")
    const request = tx.objectStore(STORE_NAME).getAll()
    request.onerror = () => reject(request.error)
    request.onsuccess = () => resolve(request.result as InstalledBrowserApp[])
    tx.oncomplete = () => db.close()
  })
}

async function idbPut(app: InstalledBrowserApp): Promise<void> {
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
  browserAppInstallStore.set({ ...browserAppInstallStore.get(), isLoading: true, error: null })
  try {
    const [catalog, installed] = await Promise.all([loadCatalogThroughNode(), idbGetAll()])
    browserAppInstallStore.set({
      catalog,
      installed: new Map(installed.map((app) => [app.appId, app])),
      isLoading: false,
      error: null,
    })
  } catch (error) {
    browserAppInstallStore.set({
      ...browserAppInstallStore.get(),
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

export async function installBrowserCatalogApp(app: BrowserCatalogApp): Promise<InstalledBrowserApp> {
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

  const installed: InstalledBrowserApp = {
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
    installedAt: new Date().toISOString(),
    manifest,
    eappBytes: bytesToArrayBuffer(eappBytes),
    verifiedAssets,
  }

  await idbPut(installed)
  const state = browserAppInstallStore.get()
  const installedMap = new Map(state.installed)
  installedMap.set(installed.appId, installed)
  browserAppInstallStore.set({ ...state, installed: installedMap })
  runtimeEventLog.append({
    kind: "app_package_verified",
    actor: installed.appId,
    reason: "wasm node installed rkyv eapp package",
    inputHash: installed.eappSha256,
    metadata: {
      packageUrl: installed.packageUrl,
      manifestSha256: installed.manifestSha256,
      packageBytes: installed.packageBytes,
      runtimeAppId: installed.runtimeAppId,
      releaseId: installed.releaseId,
      verifiedAssets: installed.verifiedAssets.length,
    },
  })
  return installed
}

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

export async function uninstallBrowserApp(appId: string): Promise<void> {
  await idbDelete(appId)
  const state = browserAppInstallStore.get()
  const installed = new Map(state.installed)
  installed.delete(appId)
  browserAppInstallStore.set({ ...state, installed })
  runtimeEventLog.append({
    kind: "app_stopped",
    actor: appId,
    reason: "wasm node uninstalled app package",
  })
}
