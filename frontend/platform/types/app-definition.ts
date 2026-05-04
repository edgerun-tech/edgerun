/**
 * App definition types for the platform.
 * Canonical app metadata used by the app registry, launcher, and AppStore UI.
 * Replaces the old AppDefinition interface in components/os/app-store.tsx.
 *
 * No fake RAM/CPU/price as truth.
 * If unknown, show unknown.
 * If preview, label preview.
 * If measured, link to footprint evidence.
 */

import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"
import { edgerun } from "@/gen/edgerun/v0/common"

export type AppKind = "builtin" | "installed" | "wasm" | "external" | "preview"
export type AppSource = "node" | "builtin" | "preview"
export type AppStatus = "available" | "installed" | "running" | "blocked" | "preview"

export interface AppFootprint {
  ramBytes?: number
  cpuMillisPerSec?: number
  measuredAt?: number
  evidenceRef?: string
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
  return {
    appId: Buffer.from(pkg.wasm_object?.object_id || new Uint8Array(0)).toString("hex"),
    name: pkg.name || "",
    description: "",
    iconId: Buffer.from(pkg.wasm_object?.object_id || new Uint8Array(0)).toString("hex"),
    kind,
    source,
    route: Object.keys(pkg.routes || {})[0],
    componentKey: pkg.name,
    requiredCapabilityIds: [],
    optionalCapabilityIds: [],
    status: "installed",
  }
}
