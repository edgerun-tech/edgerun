"use client"

import { useEffect } from "react"
import { XrayViewport } from "./XrayViewport"
import { XrayInspector } from "./XrayInspector"
import { XrayCommandSurface } from "./XrayCommandSurface"
import { registerGlobalApi } from "./graph/graph-commands"

import { useEffect } from "react"
import { XrayViewport } from "./XrayViewport"
import { XrayInspector } from "./XrayInspector"
import { XrayCommandSurface } from "./XrayCommandSurface"
import { registerGlobalApi } from "./graph/graph-commands"

export function XrayWorkspace() {
  useEffect(() => {
    registerGlobalApi()
  }, [])

  return (
    <div className="h-screen flex flex-col bg-zinc-950 text-zinc-100">
      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 relative">
          <XrayViewport />
          <div className="absolute top-3 left-3 pointer-events-none">
            <h1 className="text-lg font-mono text-zinc-400">xray</h1>
          </div>
        </div>
        <XrayInspector />
      </div>
      <XrayCommandSurface />
    </div>
  )
}
