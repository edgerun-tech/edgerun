/**
 * @deprecated WASM state → wasm-registry.ts (loadWasmForApp, getWasmUrl, removeWasm)
 * @deprecated Running apps → app-runtime.ts (startApp, stopApp, getRunningApp, isAppRunning)
 */
import { wasmRegistry, loadWasmForApp, removeWasm, getWasmUrl, getWasmBytes } from "../runtime/wasm-registry"
import { appRuntime, startApp, stopApp, getRunningApp, isAppRunning } from "../runtime/app-runtime"

export {
  wasmRegistry,
  loadWasmForApp,
  removeWasm,
  getWasmUrl,
  getWasmBytes,
  appRuntime,
  startApp,
  stopApp,
  getRunningApp,
  isAppRunning,
}
