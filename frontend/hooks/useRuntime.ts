/**
 * Hook for accessing runtime state and actions.
 * WASM operations delegate to wasm-registry; app lifecycle delegates to app-runtime.
 */

import { useStore } from "@nanostores/react"
import {
  wasmRegistry,
  allWasmModules,
  resolveWasmObject,
  loadWasmForApp,
  getWasmUrl,
  getWasmBytes,
  removeWasm,
  validateWasmHash,
} from "@/platform/runtime/wasm-registry"
import {
  appRuntime,
  runningAppsList,
  startApp,
  stopApp,
  getRunningApp,
  isAppRunning,
  setRuntimeError,
} from "@/platform/runtime/app-runtime"

export function useRuntime() {
  const wasmRegistryState = useStore(wasmRegistry)
  const appRuntimeState = useStore(appRuntime)

  return {
    wasmModules: useStore(allWasmModules),
    resolveWasmObject,
    loadWasmForApp,
    getWasmUrl,
    getWasmBytes,
    removeWasm,
    validateWasmHash,
    runningApps: useStore(runningAppsList),
    startApp,
    stopApp,
    getRunningApp,
    isAppRunning,
    setRuntimeError,
    appRuntimeState,
  }
}
