"use client"

import { useEffect, useState } from "react"
import { XrayViewport } from "./XrayViewport"
import { XrayInspector } from "./XrayInspector"
import { XrayCommandSurface } from "./XrayCommandSurface"
import { registerGlobalApi, initCodeAnalyzerConnection, xrayState } from "./graph/graph-store"
import { useStore } from "@nanostores/react"

type XrayWorkspaceMode = "full" | "bg"

interface XrayWorkspaceProps {
  mode?: XrayWorkspaceMode
}

export function XrayWorkspace({ mode = "full" }: XrayWorkspaceProps) {
  const state = useStore(xrayState)
  const [connected, setConnected] = useState(false)
  const [codeanalyzerUrl, setCodeanalyzerUrl] = useState("ws://localhost:13337/ws")

  useEffect(() => {
    registerGlobalApi()

    // Initialize connection to codeanalyzer
    initCodeAnalyzerConnection(codeanalyzerUrl)
    setConnected(true)

    return () => {
      // Cleanup on unmount
    }
  }, [codeanalyzerUrl])

  if (mode === "bg") {
    return (
      <div className="h-full w-full bg-zinc-950/60 text-zinc-100">
        <XrayViewport />
      </div>
    )
  }

  return (
    <div className="h-screen flex flex-col bg-zinc-950 text-zinc-100">
      {state.loading && (
        <div className="absolute inset-0 flex items-center justify-center bg-zinc-950/80 z-50">
          <div className="text-center">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-emerald-500 mx-auto mb-4"></div>
            <div className="text-sm text-zinc-400">Loading code analysis...</div>
          </div>
        </div>
      )}

      {state.error && (
        <div className="absolute top-14 left-1/2 -translate-x-1/2 z-50 bg-red-900/90 text-red-100 px-4 py-2 rounded text-sm">
          Error: {state.error}
        </div>
      )}

      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 relative">
          <XrayViewport />
          <div className="absolute top-3 left-3 pointer-events-none">
            <h1 className="text-lg font-mono text-zinc-400">xray</h1>
            {!connected && (
              <div className="text-xs text-zinc-500 mt-1">Not connected to codeanalyzer</div>
            )}
          </div>
        </div>
        <XrayInspector />
      </div>
      <XrayCommandSurface />
    </div>
  )
}
