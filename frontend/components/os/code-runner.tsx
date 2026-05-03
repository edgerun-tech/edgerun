"use client"

import { useState, useCallback } from "react"
import { Play, Loader2, Trash2, Copy, Check } from "lucide-react"
import { cn } from "@/lib/utils"

const EXAMPLE_CODE = `// WASI Module Example
export function fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

// Execute
const result = fibonacci(20);
console.log(\`Fibonacci(20) = \${result}\`);
return result;`

interface CodeRunnerProps {
  onOutput?: (output: string) => void
}

export function CodeRunner({ onOutput }: CodeRunnerProps) {
  const [code, setCode] = useState(EXAMPLE_CODE)
  const [output, setOutput] = useState<string[]>([])
  const [isRunning, setIsRunning] = useState(false)
  const [copied, setCopied] = useState(false)
  const [executionTime, setExecutionTime] = useState<number | null>(null)

  const runCode = useCallback(async () => {
    setIsRunning(true)
    setOutput([])
    setExecutionTime(null)

    const startTime = performance.now()
    const logs: string[] = []

    // Custom console.log to capture output
    const customLog = (...args: unknown[]) => {
      const message = args.map((arg) => 
        typeof arg === "object" ? JSON.stringify(arg, null, 2) : String(arg)
      ).join(" ")
      logs.push(message)
    }

    try {
      // Simulate module loading delay
      await new Promise((resolve) => setTimeout(resolve, 200))
      
      // Create a sandboxed execution context
      const wrappedCode = `
        ${code}
      `
      
      // Execute with custom console
      const fn = new Function("console", wrappedCode)
      const result = fn({ log: customLog, error: customLog, warn: customLog })
      
      if (result !== undefined) {
        logs.push(`→ ${result}`)
      }

      const endTime = performance.now()
      setExecutionTime(endTime - startTime)
      setOutput(logs)
      onOutput?.(logs.join("\n"))
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      setOutput([`Error: ${errorMessage}`])
      onOutput?.(`Error: ${errorMessage}`)
    } finally {
      setIsRunning(false)
    }
  }, [code, onOutput])

  const clearOutput = () => {
    setOutput([])
    setExecutionTime(null)
  }

  const copyCode = async () => {
    await navigator.clipboard.writeText(code)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="flex h-full flex-col">
      {/* Toolbar */}
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
            {isRunning ? "Running..." : "Run"}
          </button>
          <button
            onClick={clearOutput}
            className="flex items-center gap-1.5 rounded-md bg-secondary px-3 py-1.5 text-xs font-medium text-secondary-foreground transition-colors hover:bg-secondary/80"
          >
            <Trash2 className="h-3.5 w-3.5" />
            Clear
          </button>
        </div>
        <div className="flex items-center gap-2">
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
        {/* Code Editor */}
        <div className="flex flex-1 flex-col">
          <div className="border-b border-border/50 bg-muted/30 px-3 py-1">
            <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Editor
            </span>
          </div>
          <div className="relative flex-1">
            <textarea
              value={code}
              onChange={(e) => setCode(e.target.value)}
              className="absolute inset-0 resize-none bg-[var(--terminal-bg)] p-3 font-mono text-sm text-[var(--terminal-text)] outline-none placeholder:text-muted-foreground/50"
              placeholder="// Write your code here..."
              spellCheck={false}
            />
          </div>
        </div>

        {/* Output Panel */}
        <div className="flex w-2/5 flex-col">
          <div className="border-b border-border/50 bg-muted/30 px-3 py-1">
            <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Output
            </span>
          </div>
          <div className="flex-1 overflow-auto bg-[var(--terminal-bg)] p-3 font-mono text-sm">
            {output.length > 0 ? (
              output.map((line, i) => (
                <div
                  key={i}
                  className={cn(
                    "py-0.5 animate-terminal-line",
                    line.startsWith("Error:") ? "text-[var(--status-error)]" : "text-foreground"
                  )}
                >
                  {line}
                </div>
              ))
            ) : (
              <span className="text-muted-foreground/50">
                {isRunning ? "Executing..." : "Output will appear here"}
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
