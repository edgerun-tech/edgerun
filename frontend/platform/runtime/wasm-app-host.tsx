"use client"

import { useEffect, useRef, useState } from "react"
import type { AppDefinition } from "@/platform/types/app-definition"

type WasmWorkerMessage =
  | { type: "STATUS"; payload: string }
  | { type: "BLOCKED"; payload: string[] }
  | { type: "READY"; payload: { lines: string[] } }
  | { type: "LOG"; payload: string }
  | { type: "ERROR"; payload: string }

function createWasmWorker(app: AppDefinition): Worker {
  return new Worker(new URL("../../workers/wasm-app-worker.ts", import.meta.url), {
    type: "module",
    name: `edgerun-wasm-${app.appId}`,
  })
}

export function WasmAppHost({ app }: { app: AppDefinition }) {
  const workerRef = useRef<Worker | null>(null)
  const [status, setStatus] = useState("idle")
  const [details, setDetails] = useState<string[]>([])

  useEffect(() => {
    const worker = createWasmWorker(app)
    workerRef.current = worker
    setStatus("starting")
    setDetails([])

    worker.onmessage = (event: MessageEvent<WasmWorkerMessage>) => {
      const msg = event.data
      switch (msg.type) {
        case "STATUS":
          setStatus(msg.payload)
          break
        case "BLOCKED":
          setStatus("blocked")
          setDetails(msg.payload)
          break
        case "READY":
          setStatus("ready")
          setDetails(msg.payload.lines)
          break
        case "LOG":
          setDetails((prev) => [...prev.slice(-100), msg.payload])
          break
        case "ERROR":
          setStatus("error")
          setDetails([msg.payload])
          break
      }
    }

    worker.onerror = (event) => {
      setStatus("error")
      setDetails([event.message || "WASM worker failed"])
    }

    worker.postMessage({ type: "BOOT", payload: { wasmUrl: app.wasmUrl ?? null } })

    return () => {
      worker.postMessage({ type: "STOP" })
      worker.terminate()
      workerRef.current = null
    }
  }, [app])

  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <div className="border-b border-border px-3 py-2 text-xs text-muted-foreground">
        WASM App Worker · {status}
      </div>
      <div className="flex-1 overflow-auto p-4 font-mono text-xs text-muted-foreground">
        {(details.length ? details : [app.description || "Waiting for WASM app..."]).map((line, index) => (
          <div key={index}>{line}</div>
        ))}
      </div>
    </div>
  )
}
