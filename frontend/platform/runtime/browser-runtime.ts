import { BrowserNetworkCapabilityProvider, type BrowserNetworkProviderOptions } from "./providers/network-provider"
import { BrowserStorageCapabilityProvider, type BrowserStorageProviderOptions } from "./providers/storage-provider"
import { browserMessageRouter, BrowserMessageRouter } from "./message-router"
import { browserWasmHost, BrowserWasmHost, type BrowserWasmAppDefinition } from "./browser-wasm-host"
import { runtimeEventLog } from "./runtime-event-log"

export interface BrowserRuntimeOptions {
  router?: BrowserMessageRouter
  wasmHost?: BrowserWasmHost
  storage?: BrowserStorageProviderOptions
  network?: BrowserNetworkProviderOptions
}

export class BrowserCapabilityRuntime {
  readonly router: BrowserMessageRouter
  readonly wasmHost: BrowserWasmHost
  readonly storageProvider: BrowserStorageCapabilityProvider
  readonly networkProvider: BrowserNetworkCapabilityProvider

  constructor(options: BrowserRuntimeOptions = {}) {
    this.router = options.router ?? browserMessageRouter
    this.wasmHost = options.wasmHost ?? browserWasmHost
    this.storageProvider = new BrowserStorageCapabilityProvider(options.storage)
    this.networkProvider = new BrowserNetworkCapabilityProvider(options.network)
  }

  bootstrap(): void {
    this.router.registerProvider(this.storageProvider)
    this.router.registerProvider(this.networkProvider)
    runtimeEventLog.append({
      kind: "app_package_verified",
      actor: "browser-capability-runtime",
      reason: "browser capability runtime bootstrapped",
      metadata: {
        providers: [this.storageProvider.appId, this.networkProvider.appId],
      },
    })
  }

  async launchWasmApp(def: BrowserWasmAppDefinition) {
    runtimeEventLog.append({
      kind: "app_launch_requested",
      actor: def.appId,
      metadata: {
        wasmBytes: def.wasmBytes.byteLength,
      },
    })
    return this.wasmHost.instantiate(def)
  }
}

export const browserCapabilityRuntime = new BrowserCapabilityRuntime({
  network: {
    allowByDefault: false,
    allowedOrigins: ["https://edgerun.tech", "https://api.edgerun.tech"],
    deniedOrigins: ["https://telemetry.example"],
  },
})

export function bootstrapBrowserCapabilityRuntime(): BrowserCapabilityRuntime {
  browserCapabilityRuntime.bootstrap()
  return browserCapabilityRuntime
}

export * from "./browser-capability-types"
export * from "./browser-wasm-host"
export * from "./message-router"
export * from "./runtime-event-log"
export * from "./providers/network-provider"
export * from "./providers/storage-provider"
