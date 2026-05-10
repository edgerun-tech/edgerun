/**
 * App definition types for the platform.
 * Canonical app metadata used by the app registry, launcher, and AppStore UI.
 *
 * No fake RAM/CPU/price as truth.
 * If unknown, show unknown.
 * If preview, label preview.
 * If measured, link to footprint evidence.
 */

import { bytesToHex } from "@/platform/utils/bytes"
import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"

export type AppKind = "builtin" | "installed" | "wasm" | "external" | "preview"
export type AppSource = "node" | "builtin" | "preview" | "catalog"
export type AppStatus = "available" | "installed" | "running" | "blocked" | "preview"

export interface AppFootprint {
  ramBytes?: number
  cpuMillisPerSec?: number
  measuredAt?: number
  evidenceRef?: string
}

export interface AppSignatureInfo {
  developerId: string
  developerName?: string
  packageHash: string
  manifestHash?: string
  signature?: string
  verified: boolean
  verifiedAt?: string
  authorityRef?: string
  proofRef?: string
}

export interface ExternalAppBinding {
  provider: string
  externalAppId: string
  accountLabel?: string
  allowedActions: string[]
  authorityRef?: string
  proofRef?: string
}

export interface AppDefinition {
  appId: string
  name: string
  description: string
  iconId: string
  kind: AppKind
  source: AppSource
  route?: string
  componentKey?: string
  wasmUrl?: string
  wasmObjectRef?: { objectId: string; hash: string }
  requiredCapabilityIds: string[]
  optionalCapabilityIds: string[]
  status: AppStatus
  signature?: AppSignatureInfo
  externalBindings?: ExternalAppBinding[]
  footprint?: AppFootprint
  displayMetadata?: Record<string, unknown>
}

/**
 * Convert a platform AppPackage into an AppDefinition.
 * Does NOT invent RAM/CPU/price.
 */
export function appPackageToDefinition(
  pkg: edgerunStream.v0.stream.AppPackage,
  kind: AppKind = "installed",
  source: AppSource = "node",
): AppDefinition {
  const objectId = bytesToHex(pkg.wasm_object?.object_id)
  return {
    appId: objectId || pkg.name || "unknown-app",
    name: pkg.name || "Unnamed app",
    description: "Published EdgeRun app package",
    iconId: objectId || "wasm-generic",
    kind,
    source,
    route: Object.keys(pkg.routes || {})[0],
    componentKey: pkg.name,
    requiredCapabilityIds: [],
    optionalCapabilityIds: [],
    status: "installed",
    wasmObjectRef: pkg.wasm_object ? { objectId, hash: objectId } : undefined,
  }
}
