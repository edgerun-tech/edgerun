/**
 * Application runtime management.
 * Handles app lifecycle, WASM execution, runtime status.
 */

import { atom, computed } from "nanostores"
import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"
import { edgerun } from "@/gen/edgerun/v0/common"
import { wasmRegistry, loadWasmForApp } from "./wasm-registry"

export type RuntimeStatus = "stopped" | "starting" | "running" | "stopping" | "error"

export interface RunningApp {
  appId: string
  appPackage: edgerunStream.v0.stream.AppPackage
  status: RuntimeStatus
  startedAt?: string
  pid?: number
  wasmUrl?: string
}

export interface AppRuntimeState {
  runningApps: Map<string, RunningApp>
  runtimeErrors: Map<string, string>
}

const initialState: AppRuntimeState = {
  runningApps: new Map(),
  runtimeErrors: new Map(),
}

export const appRuntime = atom<AppRuntimeState>(initialState)

export const runningAppsList = computed(appRuntime, (s) =>
  Array.from(s.runningApps.values()),
)

export async function startApp(
  app: edgerunStream.v0.stream.AppPackage,
): Promise<RunningApp | null> {
  const state = appRuntime.get()
  const appId = Buffer.from(app.wasm_object?.object_id || new Uint8Array(0)).toString("hex")

  // Load WASM if needed
  const wasmModule = await loadWasmForApp(
    appId,
    app.wasm_object || new edgerun.v0.common.ObjectRef(),
  )

  if (!wasmModule) {
    const newErrors = new Map(state.runtimeErrors)
    newErrors.set(appId, "Failed to load WASM module")
    appRuntime.set({ ...state, runtimeErrors: newErrors })
    return null
  }

  const runningApp: RunningApp = {
    appId,
    appPackage: app,
    status: "running",
    startedAt: new Date().toISOString(),
    wasmUrl: wasmModule.wasmUrl,
  }

  const newRunning = new Map(state.runningApps)
  newRunning.set(appId, runningApp)
  appRuntime.set({ ...state, runningApps: newRunning })

  return runningApp
}

export function stopApp(appId: string): void {
  const state = appRuntime.get()
  const running = state.runningApps.get(appId)
  if (running) {
    const newRunning = new Map(state.runningApps)
    newRunning.set(appId, { ...running, status: "stopped" })
    appRuntime.set({ ...state, runningApps: newRunning })
  }
}

export function getRunningApp(appId: string): RunningApp | undefined {
  return appRuntime.get().runningApps.get(appId)
}

export function isAppRunning(appId: string): boolean {
  return (
    appRuntime.get().runningApps.get(appId)?.status === "running"
  )
}

export function setRuntimeError(appId: string, error: string): void {
  const state = appRuntime.get()
  const newErrors = new Map(state.runtimeErrors)
  newErrors.set(appId, error)
  appRuntime.set({ ...state, runtimeErrors: newErrors })
}
