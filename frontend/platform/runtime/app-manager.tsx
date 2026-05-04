import React from "react"
import type { AppDefinition } from "@/platform/types/app-definition"
import { getComponent } from "@/platform/registries/component-registry"
import { BuiltinAppHost } from "@/platform/runtime/builtin-app-host"
import { SandboxedWorkerAppHost } from "@/platform/runtime/sandboxed-worker-app-host"
import { WasmAppHost } from "@/platform/runtime/wasm-app-host"

export type AppRuntimeKind = "builtin" | "wasm" | "sandboxed-worker" | "blocked"

export interface AppHostContext {
  launchApp?: (app: AppDefinition) => void
}

export interface AppLaunchPlan {
  runtime: AppRuntimeKind
  reason: string
  component: React.ReactNode
}

function isSandboxedJavaScriptApp(app: AppDefinition): boolean {
  return app.kind === "external" || app.displayMetadata?.runtime === "sandboxed-worker"
}

export function createAppLaunchPlan(app: AppDefinition, context: AppHostContext = {}): AppLaunchPlan {
  if (app.kind === "wasm" || app.wasmUrl || app.wasmObjectRef) {
    return {
      runtime: "wasm",
      reason: "WASM apps run inside the WASM host boundary.",
      component: <WasmAppHost app={app} />,
    }
  }

  if (isSandboxedJavaScriptApp(app)) {
    return {
      runtime: "sandboxed-worker",
      reason: "JavaScript app logic is isolated in a worker and can only talk through host messages.",
      component: <SandboxedWorkerAppHost app={app} />,
    }
  }

  if (app.kind === "builtin" || app.kind === "demo" || app.source === "builtin" || app.source === "demo") {
    return {
      runtime: "builtin",
      reason: "Trusted dashboard builtin rendered by the host UI.",
      component: <BuiltinAppHost app={app} onLaunchApp={context.launchApp} />,
    }
  }

  const registered = app.componentKey ? getComponent(app.componentKey) : undefined
  if (registered?.isSafe) {
    const Component = registered.component
    return {
      runtime: "builtin",
      reason: "Safe registered host component.",
      component: <Component />,
    }
  }

  return {
    runtime: "blocked",
    reason: "No trusted runtime boundary is configured for this app.",
    component: (
      <div className="p-4 text-sm text-muted-foreground">
        App blocked: no sandbox/runtime boundary configured for {app.name}.
      </div>
    ),
  }
}
