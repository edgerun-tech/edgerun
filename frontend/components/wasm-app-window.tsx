import { useState, useEffect, useRef, useCallback } from "react"
import { Loader2, Terminal, AlertCircle, Play, Square, ChevronDown, ChevronUp } from "lucide-react"
import { cn } from "@/lib/utils"
import { WasmAdapter } from "@/lib/wasm/wasm-adapter"

export interface WasmAppProps {
  wasmUrl: string
  wasmBytes?: Uint8Array
  appName: string
  onLog?: (msg: string) => void
  onAction?: (action: string) => void
  children:
    | React.ReactNode
    | ((state: Record<string, unknown> | null, sendAction: (action: string) => void) => React.ReactNode)
}

type AppState = "loading" | "ready" | "running" | "stopped" | "error"

interface LogEntry {
  id: string
  timestamp: Date
  level: "info" | "warn" | "error" | "output"
  message: string
}

export function WasmAppWindow({ wasmUrl, wasmBytes, appName, onLog, onAction, children }: WasmAppProps) {
  const [state, setState] = useState<AppState>("loading")
  const [wasmState, setWasmState] = useState<Record<string, unknown> | null>(null)
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [error, setError] = useState<string | null>(null)
  const [showLogs, setShowLogs] = useState(false)

  const adapterRef = useRef<WasmAdapter | null>(null)
  const runLoopRef = useRef<ReturnType<typeof requestAnimationFrame> | null>(null)

  const addLog = useCallback((level: LogEntry["level"], message: string) => {
    setLogs((prev) => {
      const next = [...prev, {
        id: `log-${Date.now()}-${Math.random()}`,
        timestamp: new Date(),
        level,
        message,
      }]
      return next.slice(-200)
    })
    onLog?.(message)
  }, [onLog])

  const sendAction = useCallback((action: string) => {
    onAction?.(action)
  }, [onAction])

  const stop = useCallback(() => {
    if (runLoopRef.current) {
      cancelAnimationFrame(runLoopRef.current)
      runLoopRef.current = null
    }
    if (adapterRef.current) {
      adapterRef.current.dispose()
      adapterRef.current = null
    }
    setState("stopped")
    addLog("info", "App stopped")
  }, [addLog])

  const runLoop = useCallback((adapter: WasmAdapter) => {
    try {
      adapter.run(0, 0)
      runLoopRef.current = requestAnimationFrame(() => runLoop(adapter))
    } catch (e) {
      addLog("error", `Runtime error: ${e}`)
      setState("error")
    }
  }, [addLog])

  const start = useCallback(async () => {
    setState("loading")
    setError(null)

    try {
      let bytes: Uint8Array
      if (wasmBytes) {
        bytes = wasmBytes
      } else {
        const resp = await fetch(wasmUrl)
        if (!resp.ok) throw new Error(`Failed to fetch WASM: ${resp.status}`)
        bytes = new Uint8Array(await resp.arrayBuffer())
      }

      const adapter = new WasmAdapter({
        onOutput: (data) => {
          const text = new TextDecoder().decode(data)
          // Try to parse as JSON state message
          try {
            const parsed = JSON.parse(text)
            if (parsed && typeof parsed === "object" && parsed.type === "state") {
              setWasmState(parsed.payload as Record<string, unknown> | null)
              return
            }
          } catch {
            // not JSON, treat as log
          }
          addLog("output", text)
        },
        onMessage: (target, payload) => {
          addLog("info", `message -> ${target} (${payload.length}b)`)
        },
        onUIRender: () => {},
        onJSXRender: () => {},
        onUIAction: () => {},
        onLog: (level, msg) => {
          addLog(level, msg)
        },
      }, { verbose: false, appName })

      await adapter.load(bytes)
      adapterRef.current = adapter

      adapter.pushNetworkConnected(1)

      setState("running")
      addLog("info", `${appName} started`)

      runLoop(adapter)
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e)
      setError(errMsg)
      setState("error")
      addLog("error", errMsg)
    }
  }, [wasmUrl, wasmBytes, addLog, runLoop, appName])

  useEffect(() => {
    start()
    return () => {
      stop()
    }
  }, [start, stop])

  return (
    <div className="flex h-full flex-col">
      {/* Toolbar */}
      <div className="flex items-center justify-between border-b border-[var(--window-border)] bg-[var(--window-header)] px-3 py-2">
        <div className="flex items-center gap-2">
          <span className={cn(
            "h-2 w-2 rounded-full",
            state === "running" && "bg-[var(--status-online)]",
            state === "loading" && "bg-[var(--status-warning)] animate-pulse",
            state === "error" && "bg-[var(--status-error)]",
            state === "stopped" && "bg-muted-foreground",
            state === "ready" && "bg-[var(--status-online)]",
          )} />
          <span className="text-xs font-medium text-foreground">{appName}</span>
          {state === "running" && (
            <span className="text-[10px] text-muted-foreground">running</span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setShowLogs((v) => !v)}
            className={cn(
              "flex h-6 w-6 items-center justify-center rounded transition-colors",
              showLogs
                ? "bg-primary/15 text-primary"
                : "text-muted-foreground hover:bg-secondary hover:text-foreground"
            )}
            title="Toggle debug logs"
          >
            {showLogs ? <ChevronDown className="h-3.5 w-3.5" /> : <Terminal className="h-3.5 w-3.5" />}
          </button>

          {state === "running" ? (
            <button
              onClick={stop}
              className="flex items-center gap-1 rounded-md bg-red-500/10 px-2 py-1 text-[10px] font-medium text-red-400 hover:bg-red-500/20"
            >
              <Square className="h-3 w-3" />
              Stop
            </button>
          ) : state === "error" ? (
            <button
              onClick={start}
              className="flex items-center gap-1 rounded-md bg-primary/10 px-2 py-1 text-[10px] font-medium text-primary hover:bg-primary/20"
            >
              <Play className="h-3 w-3" />
              Retry
            </button>
          ) : (
            <button
              onClick={start}
              className="flex items-center gap-1 rounded-md bg-primary/10 px-2 py-1 text-[10px] font-medium text-primary hover:bg-primary/20"
            >
              <Play className="h-3 w-3" />
              Start
            </button>
          )}
        </div>
      </div>

      {/* Main content - React renders native components */}
      <div className="flex flex-1 flex-col overflow-hidden">
        <div className={cn(
          "flex flex-col overflow-hidden transition-all",
          showLogs ? "h-2/3" : "h-full"
        )}>
          <div className="flex-1 overflow-hidden bg-[var(--window-bg)]">
            {state === "loading" ? (
              <div className="flex h-full items-center justify-center">
                <div className="flex items-center gap-2 text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  <span className="text-sm">Loading...</span>
                </div>
              </div>
            ) : state === "running" || state === "ready" ? (
              typeof children === "function" ? (
                children(wasmState, sendAction)
              ) : (
                children
              )
            ) : state === "stopped" ? (
              <div className="flex h-full items-center justify-center">
                <span className="text-sm text-muted-foreground">App stopped</span>
              </div>
            ) : (
              <div className="flex h-full items-center justify-center">
                <span className="text-sm text-[var(--status-error)]">{error || "Error"}</span>
              </div>
            )}
          </div>
        </div>

        {/* Debug logs panel */}
        {showLogs && (
          <div className="flex h-1/3 flex-col border-t border-[var(--window-border)]">
            <div className="flex items-center gap-2 border-b border-[var(--window-border)]/50 bg-muted/20 px-3 py-1.5">
              <Terminal className="h-3 w-3 text-muted-foreground" />
              <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                Debug Logs
              </span>
              <span className="ml-auto text-[10px] text-muted-foreground/50">
                {logs.length} entries
              </span>
            </div>
            <div className="flex-1 overflow-auto bg-[var(--terminal-bg)] p-2 font-mono text-[11px]">
              {logs.length === 0 ? (
                <span className="text-muted-foreground/50">No logs yet</span>
              ) : (
                logs.map((entry) => (
                  <div key={entry.id} className="py-0.5 animate-terminal-line">
                    <span className="text-muted-foreground/60">
                      {entry.timestamp.toLocaleTimeString()}{" "}
                    </span>
                    <span className={cn(
                      entry.level === "error" && "text-[var(--status-error)]",
                      entry.level === "warn" && "text-[var(--status-warning)]",
                      entry.level === "info" && "text-[var(--status-online)]",
                      entry.level === "output" && "text-foreground",
                    )}>
                      {entry.message}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
