export type AppRuntimeKind = "builtin" | "wasm" | "sandboxed-worker"

export interface AppRuntimeManifest {
  appId: string
  name: string
  runtime: AppRuntimeKind
  entry?: string
  source?: string
  permissions: string[]
  optionalPermissions?: string[]
}

export function readRuntimeManifest(value: unknown): Partial<AppRuntimeManifest> {
  if (!value || typeof value !== "object") return {}
  const record = value as Record<string, unknown>
  const manifest = record.runtimeManifest
  if (!manifest || typeof manifest !== "object") return {}
  return manifest as Partial<AppRuntimeManifest>
}
