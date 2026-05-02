/**
 * App definition types for the platform.
 * Canonical app metadata used by the app registry, launcher, and AppStore UI.
 * Replaces the old AppDefinition interface in components/os/app-store.tsx.
 *
 * No fake RAM/CPU/price as truth.
 * If unknown, show unknown.
 * If demo, label demo.
 * If measured, link to footprint evidence.
 */

import type { AppPackage } from "@/platform/protocol/apps"

export type AppKind = "builtin" | "installed" | "wasm" | "external" | "demo"

export type AppSource = "node" | "builtin" | "demo"

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
  status: "available" | "installed" | "running" | "blocked" | "demo"
  footprint?: AppFootprint
  displayMetadata?: Record<string, unknown>
}

/**
 * Convert a platform AppPackage into an AppDefinition.
 * Does NOT invent RAM/CPU/price.
 */
export function appPackageToDefinition(
  pkg: AppPackage,
  kind: AppKind = "installed",
  source: AppSource = "node",
): AppDefinition {
  return {
    appId: pkg.appId,
    name: pkg.name || pkg.appId,
    description: pkg.description || "",
    iconId: pkg.iconId || pkg.appId,
    kind,
    source,
    route: pkg.routes?.[0]?.path,
    componentKey: pkg.appId,
    wasmObjectRef: pkg.wasmObjectRef,
    requiredCapabilityIds: (pkg.requiredCapabilities || []).map((c) => c.capabilityId),
    optionalCapabilityIds: (pkg.optionalCapabilities || []).map((c) => c.capabilityId),
    status: "installed",
  }
}
