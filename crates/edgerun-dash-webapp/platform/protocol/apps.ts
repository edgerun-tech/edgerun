/**
 * App protocol helpers.
 * Handles AppPackage, AppPrincipal, and app-related protocol messages.
 */

import { protocolClient } from "./client"
import type { ObjectRef } from "./refs"

export interface AppPackage {
  appId: string
  name: string
  version: string
  description: string
  wasmObjectRef: ObjectRef
  entryPoint: string
  routes: AppRoute[]
  actions: AppAction[]
  pipelines: AppPipeline[]
  requiredCapabilities: string[]
  optionalCapabilities: string[]
  isPublic: boolean
  packageHash: string
}

export interface AppRoute {
  path: string
  viewSpec?: ViewSpec
  requiredCapabilities: string[]
}

export interface AppAction {
  actionId: string
  description: string
  requiredCapabilities: string[]
  inputSpec?: unknown
  outputSpec?: unknown
}

export interface AppPipeline {
  pipelineId: string
  description: string
  steps: PipelineStep[]
  requiredCapabilities: string[]
}

export interface PipelineStep {
  stepId: string
  actionId: string
  inputMapping?: Record<string, string>
  outputMapping?: Record<string, string>
}

export interface ViewSpec {
  type: "react" | "html" | "canvas"
  componentTree?: ComponentSpec[]
  styles?: Record<string, string>
}

export interface ComponentSpec {
  type: string
  props?: Record<string, unknown>
  children?: ComponentSpec[]
}

export interface AppPrincipal {
  appId: string
  identityId: string
  grantedAt: string
  expiresAt?: string
  delegations: string[]
}

export interface AppInstallRequest {
  appPackageRef: ObjectRef
  approveCapabilities: string[]
}

export interface AppUninstallRequest {
  appId: string
}

export async function fetchAppPackage(
  appId: string,
): Promise<AppPackage | null> {
  const response = await protocolClient.send({
    method: "GET",
    path: `/protocol/app/${appId}/package`,
  })
  if (response.status !== 200) return null
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as AppPackage
}

export async function listInstalledApps(): Promise<AppPackage[]> {
  const response = await protocolClient.send({
    method: "GET",
    path: "/protocol/apps",
  })
  if (response.status !== 200) return []
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as AppPackage[]
}

export async function installApp(
  request: AppInstallRequest,
): Promise<AppPackage | null> {
  const response = await protocolClient.send({
    method: "POST",
    path: "/protocol/app/install",
    body: new TextEncoder().encode(JSON.stringify(request)),
  })
  if (response.status !== 200) return null
  const text = new TextDecoder().decode(response.body)
  return JSON.parse(text) as AppPackage
}

export async function uninstallApp(
  request: AppUninstallRequest,
): Promise<boolean> {
  const response = await protocolClient.send({
    method: "POST",
    path: "/protocol/app/uninstall",
    body: new TextEncoder().encode(JSON.stringify(request)),
  })
  return response.status === 200
}
