/**
 * Application runtime management.
 * Handles app lifecycle, WASM execution, runtime status.
 */

import { atom, computed } from "nanostores"
import type { AppPackage } from "@/platform/protocol/apps"
import { wasmRegistry } from "./wasm-registry"

export type RuntimeStatus = "stopped" | "starting" | "running" | "stopping" | "error"

export interface RunningApp {
  appId: string
  appPackage: AppPackage
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
  app: AppPackage,
): Promise<RunningApp | null> {
  const state = appRuntime.get()

  // Load WASM if needed
  const wasmModule = await wasmRegistry.loadWasmForApp(
    app.appId,
    app.wasmObjectRef,
  )

  if (!wasmModule) {
    const newErrors = new Map(state.runtimeErrors)
    newErrors.set(app.appId, "Failed to load WASM module")
    appRuntime.set({ ...state, runtimeErrors: newErrors })
    return null
  }

  const runningApp: RunningApp = {
    appId: app.appId,
    appPackage: app,
    status: "running",
    startedAt: new Date().toISOString(),
    wasmUrl: wasmModule.wasmUrl,
  }

  const newRunning = new Map(state.runningApps)
  newRunning.set(app.appId, runningApp)
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
