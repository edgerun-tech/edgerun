"use client"

import { useState, useEffect, useCallback } from "react"
import { Delete } from "lucide-react"
import { cn } from "@/lib/utils"

type Op = "+" | "-" | "×" | "÷" | null

interface CalcState {
  display: string
  prev: string
  op: Op
  justEvaled: boolean
}

const INIT: CalcState = { display: "0", prev: "", op: null, justEvaled: false }

const MAX_DIGITS = 12

function formatDisplay(val: string): string {
  if (val === "Error" || val === "Infinity") return val
  const num = parseFloat(val)
  if (isNaN(num)) return val
  // Limit display to 12 significant digits
  const formatted = parseFloat(num.toPrecision(10)).toString()
  return formatted.length > MAX_DIGITS ? num.toExponential(5) : formatted
}

export function CalculatorApp() {
  const [state, setState] = useState<CalcState>(INIT)
  const [history, setHistory] = useState<string[]>([])

  const evaluate = useCallback((prev: string, cur: string, op: Op): string => {
    const a = parseFloat(prev)
    const b = parseFloat(cur)
    if (isNaN(a) || isNaN(b)) return cur
    switch (op) {
      case "+": return String(a + b)
      case "-": return String(a - b)
      case "×": return String(a * b)
      case "÷": return b === 0 ? "Error" : String(a / b)
      default: return cur
    }
  }, [])

  const handleDigit = useCallback((d: string) => {
    setState((s) => {
      if (s.justEvaled) return { ...INIT, display: d === "." ? "0." : d }
      if (d === "." && s.display.includes(".")) return s
      if (d !== "." && s.display === "0") return { ...s, display: d, justEvaled: false }
      if (s.display.replace("-", "").replace(".", "").length >= MAX_DIGITS) return s
      return { ...s, display: s.display + d, justEvaled: false }
    })
  }, [])

  const handleOp = useCallback((op: Op) => {
    setState((s) => {
      if (s.op && !s.justEvaled && s.prev) {
        const result = evaluate(s.prev, s.display, s.op)
        return { display: result, prev: result, op, justEvaled: false }
      }
      return { ...s, prev: s.display, op, justEvaled: false }
    })
  }, [evaluate])

  const handleEquals = useCallback(() => {
    setState((s) => {
      if (!s.op || !s.prev) return { ...s, justEvaled: true }
      const result = evaluate(s.prev, s.display, s.op)
      const entry = `${s.prev} ${s.op} ${s.display} = ${formatDisplay(result)}`
      setHistory((h) => [entry, ...h].slice(0, 8))
      return { display: result, prev: "", op: null, justEvaled: true }
    })
  }, [evaluate])

  const handleClear = useCallback(() => setState(INIT), [])

  const handlePlusMinus = useCallback(() => {
    setState((s) => {
      if (s.display === "0" || s.display === "Error") return s
      return { ...s, display: s.display.startsWith("-") ? s.display.slice(1) : "-" + s.display }
    })
  }, [])

  const handlePercent = useCallback(() => {
    setState((s) => {
      const n = parseFloat(s.display)
      if (isNaN(n)) return s
      return { ...s, display: String(n / 100) }
    })
  }, [])

  const handleBackspace = useCallback(() => {
    setState((s) => {
      if (s.justEvaled || s.display === "Error") return { ...s, display: "0", justEvaled: false }
      const next = s.display.length > 1 ? s.display.slice(0, -1) : "0"
      return { ...s, display: next }
    })
  }, [])

  // Keyboard support
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ("0123456789".includes(e.key)) handleDigit(e.key)
      else if (e.key === ".") handleDigit(".")
      else if (e.key === "+") handleOp("+")
      else if (e.key === "-") handleOp("-")
      else if (e.key === "*") handleOp("×")
      else if (e.key === "/") { e.preventDefault(); handleOp("÷") }
      else if (e.key === "Enter" || e.key === "=") handleEquals()
      else if (e.key === "Escape") handleClear()
      else if (e.key === "Backspace") handleBackspace()
      else if (e.key === "%") handlePercent()
    }
    window.addEventListener("keydown", handler)
    return () => window.removeEventListener("keydown", handler)
  }, [handleDigit, handleOp, handleEquals, handleClear, handleBackspace, handlePercent])

  type BtnDef = { label: React.ReactNode; action: () => void; variant?: "op" | "func" | "eq" | "default" }

  const buttons: BtnDef[] = [
    { label: "AC", action: handleClear, variant: "func" },
    { label: "+/-", action: handlePlusMinus, variant: "func" },
    { label: "%", action: handlePercent, variant: "func" },
    { label: "÷", action: () => handleOp("÷"), variant: "op" },
    { label: "7", action: () => handleDigit("7") },
    { label: "8", action: () => handleDigit("8") },
    { label: "9", action: () => handleDigit("9") },
    { label: "×", action: () => handleOp("×"), variant: "op" },
    { label: "4", action: () => handleDigit("4") },
    { label: "5", action: () => handleDigit("5") },
    { label: "6", action: () => handleDigit("6") },
    { label: "-", action: () => handleOp("-"), variant: "op" },
    { label: "1", action: () => handleDigit("1") },
    { label: "2", action: () => handleDigit("2") },
    { label: "3", action: () => handleDigit("3") },
    { label: "+", action: () => handleOp("+"), variant: "op" },
    { label: "0", action: () => handleDigit("0") },
    { label: ".", action: () => handleDigit(".") },
    { label: <Delete className="h-4 w-4" />, action: handleBackspace, variant: "func" },
    { label: "=", action: handleEquals, variant: "eq" },
  ]

  const variantClass: Record<string, string> = {
    op: "bg-primary/20 text-primary hover:bg-primary/30 font-medium",
    func: "bg-secondary text-muted-foreground hover:bg-secondary/70 hover:text-foreground",
    eq: "bg-primary text-primary-foreground hover:opacity-90 font-semibold",
    default: "bg-[oklch(0.16_0.005_280)] text-foreground hover:bg-[oklch(0.2_0.005_280)]",
  }

  const activeOp = state.op

  return (
    <div className="flex h-full flex-col">
      {/* Display */}
      <div className="flex-1 flex flex-col justify-end bg-[oklch(0.07_0.005_280)] px-5 py-4 min-h-0">
        {/* History */}
        <div className="mb-2 overflow-hidden">
          {history.slice(0, 3).map((h, i) => (
            <p
              key={i}
              className="text-right font-mono text-[10px] text-muted-foreground/40 truncate"
              style={{ opacity: 1 - i * 0.3 }}
            >
              {h}
            </p>
          ))}
        </div>
        {/* Sub-expression */}
        {state.op && state.prev && (
          <p className="text-right font-mono text-sm text-muted-foreground truncate">
            {formatDisplay(state.prev)} {state.op}
          </p>
        )}
        {/* Main display */}
        <p
          className={cn(
            "text-right font-mono font-light tabular-nums text-foreground leading-none",
            state.display.length > 10 ? "text-2xl" : state.display.length > 7 ? "text-3xl" : "text-4xl"
          )}
        >
          {formatDisplay(state.display)}
        </p>
      </div>

      {/* Buttons grid */}
      <div className="grid grid-cols-4 gap-px bg-[var(--window-border)] border-t border-[var(--window-border)]">
        {buttons.map((btn, i) => (
          <button
            key={i}
            onClick={btn.action}
            className={cn(
              "flex h-14 items-center justify-center text-lg transition-all active:scale-95",
              variantClass[btn.variant || "default"],
              // Highlight active operator
              btn.variant === "op" && btn.label === activeOp && "bg-primary text-primary-foreground"
            )}
          >
            {btn.label}
          </button>
        ))}
      </div>
    </div>
  )
}
