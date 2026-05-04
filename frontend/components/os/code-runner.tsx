"use client"

import { useCallback, useRef, useState } from "react"
import { Play, Loader2, Trash2, Copy, Check, Package } from "lucide-react"
import { cn } from "@/lib/utils"

const EXAMPLE_CODE = `// AssemblyScript compiled fully in the browser.
// Syntax is TypeScript-like, but types must be AssemblyScript types.

export function run(): i32 {
  return add(20, 22);
}

export function add(a: i32, b: i32): i32 {
  return a + b;
}`

type CompileResponse =
  | {
      id: number
      ok: true
      logs: string[]
      wasm: Uint8Array
      wasmSize: number
      compileMs: number
    }
  | {
      id: number
      ok: false
      logs: string[]
      error: string
      compileMs: number
    }

interface CodeRunnerProps {
  onOutput?: (output: string) => void
}

function stringifyExportResult(value: unknown): string {
  if (typeof value === "bigint") return value.toString()
  if (value === undefined) return "undefined"
  return String(value)
}

function createCompilerWorker(): Worker {
  return new Worker(new URL("../../workers/assemblyscript-compiler-worker.ts", import.meta.url), {
    type: "module",
  })
}

function compileAssemblyScript(source: string): Promise<CompileResponse> {
  const id = Date.now() + Math.floor(Math.random() * 1_000_000)
  const worker = createCompilerWorker()

  return new Promise((resolve) => {
    worker.onmessage = (event: MessageEvent<CompileResponse>) => {
      if (event.data.id !== id) return
      worker.terminate()
      resolve(event.data)
    }

    worker.onerror = (event) => {
      worker.terminate()
      resolve({
        id,
        ok: false,
        logs: [],
        error: event.message || "AssemblyScript compiler worker failed",
        compileMs: 0,
      })
    }

    worker.postMessage({ id, source })
  })
}

export function CodeRunner({ onOutput }: CodeRunnerProps) {
  const [code, setCode] = useState(EXAMPLE_CODE)
  const [output, setOutput] = useState<string[]>([])
  const [isRunning, setIsRunning] = useState(false)
  const [copied, setCopied] = useState(false)
  const [executionTime, setExecutionTime] = useState<number | null>(null)
  const [wasmSize, setWasmSize] = useState<number | null>(null)
  const runIdRef = useRef(0)

  const runCode = useCallback(async () => {
    const runId = ++runIdRef.current
    setIsRunning(true)
    setOutput([])
    setExecutionTime(null)
    setWasmSize(null)

    const startTime = performance.now()
    const logs: string[] = []

    try {
      logs.push("starting AssemblyScript compiler worker...")
      setOutput([...logs])

      const compiled = await compileAssemblyScript(code)
      if (runId !== runIdRef.current) return

      logs.push(...compiled.logs)

      if (!compiled.ok) {
        throw new Error(compiled.error)
      }

      setWasmSize(compiled.wasmSize)
      logs.push(`compiled ${compiled.wasmSize} byte wasm module in ${compiled.compileMs.toFixed(2)}ms`)

      const imports = {
        env: {
          abort(message: number, fileName: number, line: number, column: number) {
            throw new Error(`abort at ${line}:${column} message=${message} file=${fileName}`)
          },
        },
      }

      const { instance } = await WebAssembly.instantiate(compiled.wasm, imports)
      const exports = instance.exports as Record<string, unknown>
      const exportNames = Object.keys(exports)
      logs.push(`exports: ${exportNames.join(", ") || "none"}`)

      const callable = exports.run
      if (typeof callable === "function") {
        const value = callable()
        logs.push(`run() → ${stringifyExportResult(value)}`)
      } else {
        logs.push("no exported run() function found; module compiled successfully")
      }

      const endTime = performance.now()
      setExecutionTime(endTime - startTime)
      setOutput(logs)
      onOutput?.(logs.join("\n"))
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      const finalLogs = [...logs, `Error: ${errorMessage}`]
      setOutput(finalLogs)
      onOutput?.(finalLogs.join("\n"))
    } finally {
      if (runId === runIdRef.current) setIsRunning(false)
    }
  }, [code, onOutput])

  const clearOutput = () => {
    setOutput([])
    setExecutionTime(null)
    setWasmSize(null)
  }

  const copyCode = async () => {
    await navigator.clipboard.writeText(code)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-border bg-[var(--window-header)] px-3 py-2">
        <div className="flex items-center gap-2">
          <button
            onClick={runCode}
            disabled={isRunning}
            className={cn(
              "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
              isRunning
                ? "bg-primary/50 text-primary-foreground/70"
                : "bg-primary text-primary-foreground hover:bg-primary/90"
            )}
          >
            {isRunning ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <Play className="h-3.5 w-3.5" />
            )}
            {isRunning ? "Compiling..." : "Compile + Run"}
          </button>
          <button
            onClick={clearOutput}
            className="flex items-center gap-1.5 rounded-md bg-secondary px-3 py-1.5 text-xs font-medium text-secondary-foreground transition-colors hover:bg-secondary/80"
          >
            <Trash2 className="h-3.5 w-3.5" />
            Clear
          </button>
        </div>
        <div className="flex items-center gap-3">
          {wasmSize !== null && (
            <span className="flex items-center gap-1 font-mono text-xs text-muted-foreground">
              <Package className="h-3.5 w-3.5" />
              {wasmSize} B wasm
            </span>
          )}
          {executionTime !== null && (
            <span className="font-mono text-xs text-muted-foreground">
              {executionTime.toFixed(2)}ms
            </span>
          )}
          <button
            onClick={copyCode}
            className="flex items-center gap-1 rounded-md px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
          >
            {copied ? (
              <Check className="h-3.5 w-3.5 text-[var(--status-online)]" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
          </button>
        </div>
      </div>

      <div className="flex flex-1 divide-x divide-border overflow-hidden">
        <div className="flex flex-1 flex-col">
          <div className="border-b border-border/50 bg-muted/30 px-3 py-1">
            <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              AssemblyScript Editor
            </span>
          </div>
          <div className="relative flex-1">
            <textarea
              value={code}
              onChange={(e) => setCode(e.target.value)}
              className="absolute inset-0 resize-none bg-[var(--terminal-bg)] p-3 font-mono text-sm text-[var(--terminal-text)] outline-none placeholder:text-muted-foreground/50"
              placeholder="// export function run(): i32 { return 42 }"
              spellCheck={false}
            />
          </div>
        </div>

        <div className="flex w-2/5 flex-col">
          <div className="border-b border-border/50 bg-muted/30 px-3 py-1">
            <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Compiler Output
            </span>
          </div>
          <div className="flex-1 overflow-auto bg-[var(--terminal-bg)] p-3 font-mono text-sm">
            {output.length > 0 ? (
              output.map((line, i) => (
                <div
                  key={i}
                  className={cn(
                    "py-0.5 animate-terminal-line whitespace-pre-wrap",
                    line.startsWith("Error:") ? "text-[var(--status-error)]" : "text-foreground"
                  )}
                >
                  {line}
                </div>
              ))
            ) : (
              <span className="text-muted-foreground/50">
                {isRunning ? "Compiling..." : "AssemblyScript compiler output will appear here"}
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
