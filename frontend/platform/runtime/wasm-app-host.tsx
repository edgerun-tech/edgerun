"use client"

import { useEffect, useState } from "react"
import type { AppDefinition } from "@/platform/types/app-definition"

export function WasmAppHost({ app }: { app: AppDefinition }) {
  const [status, setStatus] = useState("idle")
  const [details, setDetails] = useState<string[]>([])

  useEffect(() => {
    let cancelled = false

    async function boot() {
      setStatus("starting")
      setDetails([])

      if (!app.wasmUrl) {
        setStatus("blocked")
        setDetails(["No wasmUrl configured yet. Object-ref loading will be wired through the node/object store."])
        return
      }

      try {
        setStatus("fetching")
        const response = await fetch(app.wasmUrl)
        if (!response.ok) throw new Error(`failed to fetch wasm: ${response.status}`)
        const bytes = new Uint8Array(await response.arrayBuffer())
        if (cancelled) return

        setStatus("instantiating")
        const { instance } = await WebAssembly.instantiate(bytes, {
          env: {
            abort(message: number, fileName: number, line: number, column: number) {
              throw new Error(`abort at ${line}:${column} message=${message} file=${fileName}`)
            },
          },
        })

        if (cancelled) return
        const exports = Object.keys(instance.exports)
        setStatus("ready")
        setDetails([`loaded ${bytes.byteLength} bytes`, `exports: ${exports.join(", ") || "none"}`])
      } catch (error) {
        if (cancelled) return
        setStatus("error")
        setDetails([error instanceof Error ? error.message : String(error)])
      }
    }

    boot()
    return () => {
      cancelled = true
    }
  }, [app])

  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <div className="border-b border-border px-3 py-2 text-xs text-muted-foreground">
        WASM App Host · {status}
      </div>
      <div className="flex-1 overflow-auto p-4 font-mono text-xs text-muted-foreground">
        {(details.length ? details : [app.description || "Waiting for WASM app..."]).map((line, index) => (
          <div key={index}>{line}</div>
        ))}
      </div>
    </div>
  )
}
