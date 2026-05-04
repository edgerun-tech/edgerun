"use client"

import { useEffect, useRef, useState } from "react"
import type { AppDefinition } from "@/platform/types/app-definition"

interface WorkerViewState {
  title?: string
  lines: string[]
}

type WorkerAppMessage =
  | { type: "READY" }
  | { type: "LOG"; payload: string }
  | { type: "VIEW"; payload: WorkerViewState }
  | { type: "ERROR"; payload: string }

function createSandboxedWorker(app: AppDefinition): Worker {
  return new Worker(new URL("../../workers/sandboxed-js-app-worker.ts", import.meta.url), {
    type: "module",
    name: `edgerun-app-${app.appId}`,
  })
}

export function SandboxedWorkerAppHost({ app }: { app: AppDefinition }) {
  const workerRef = useRef<Worker | null>(null)
  const [logs, setLogs] = useState<string[]>([])
  const [view, setView] = useState<WorkerViewState>({ lines: [] })
  const [status, setStatus] = useState<"starting" | "ready" | "error">("starting")

  useEffect(() => {
    const worker = createSandboxedWorker(app)
    workerRef.current = worker

    worker.onmessage = (event: MessageEvent<WorkerAppMessage>) => {
      const msg = event.data
      switch (msg.type) {
        case "READY":
          setStatus("ready")
          setLogs((prev) => [...prev, "worker ready"])
          break
        case "LOG":
          setLogs((prev) => [...prev.slice(-100), msg.payload])
          break
        case "VIEW":
          setView(msg.payload)
          break
        case "ERROR":
          setStatus("error")
          setLogs((prev) => [...prev.slice(-100), `error: ${msg.payload}`])
          break
      }
    }

    worker.onerror = (event) => {
      setStatus("error")
      setLogs((prev) => [...prev.slice(-100), `worker error: ${event.message}`])
    }

    worker.postMessage({
      type: "BOOT",
      payload: {
        appId: app.appId,
        name: app.name,
        description: app.description,
        capabilities: app.requiredCapabilityIds,
        source: app.displayMetadata?.source ?? null,
      },
    })

    return () => {
      worker.postMessage({ type: "STOP" })
      worker.terminate()
      workerRef.current = null
    }
  }, [app])

  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <div className="border-b border-border px-3 py-2 text-xs text-muted-foreground">
        Sandboxed JS Worker · {status}
      </div>
      <div className="flex-1 overflow-auto p-4">
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="text-sm font-semibold">{view.title || app.name}</div>
          <div className="mt-3 space-y-1 font-mono text-xs text-muted-foreground">
            {(view.lines.length ? view.lines : [app.description || "No view published yet."]).map((line, index) => (
              <div key={index}>{line}</div>
            ))}
          </div>
        </div>
        {logs.length > 0 && (
          <div className="mt-4 rounded-lg border border-border bg-[var(--terminal-bg)] p-3 font-mono text-xs">
            {logs.map((line, index) => (
              <div key={index} className="whitespace-pre-wrap text-[var(--terminal-text)]">
                {line}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
