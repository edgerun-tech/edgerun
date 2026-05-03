/**
 * Hook for accessing runtime state and actions.
 */

import { useStore } from "@nanostores/react"
import {
  runtimeStore,
  runningAppsList,
  startRuntimeApp,
  stopRuntimeApp,
  cacheWasm,
  getCachedWasm,
  getWasmUrl,
  removeWasmCache,
} from "@/platform/state/runtime-store"
import {
  wasmRegistry,
  allWasmModules,
  resolveWasmObject,
  loadWasmForApp,
  getWasmUrl as registryGetWasmUrl,
  getWasmBytes,
  removeWasm,
  validateWasmHash,
} from "@/platform/runtime/wasm-registry"
import {
  appRuntime,
  runningAppsList as appRuntimeRunning,
  startApp,
  stopApp,
  getRunningApp,
  isAppRunning,
  setRuntimeError,
} from "@/platform/runtime/app-runtime"

export function useRuntime() {
  const store = useStore(runtimeStore)
  const wasmRegistryState = useStore(wasmRegistry)
  const appRuntimeState = useStore(appRuntime)

  return {
    // Runtime store
    runningApps: store.runningApps,
    wasmCache: store.wasmCache,
    startRuntimeApp,
    stopRuntimeApp,
    cacheWasm,
    getCachedWasm,
    getWasmUrl,
    removeWasmCache,
    // WASM registry
    wasmModules: useStore(allWasmModules),
    resolveWasmObject,
    loadWasmForApp,
    registryGetWasmUrl,
    getWasmBytes,
    removeWasm,
    validateWasmHash,
    // App runtime
    appRuntimeRunning,
    startApp,
    stopApp,
    getRunningApp,
    isAppRunning,
    setRuntimeError,
  }
}
