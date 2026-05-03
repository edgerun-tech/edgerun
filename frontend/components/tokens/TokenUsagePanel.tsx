"use client"

import { useStore } from "@nanostores/react"
import { getDashboardMode } from "@/platform/runtime/dashboard-mode"
import { cn } from "@/lib/utils"
import { Progress } from "@/components/ui/progress"
import {
  Coins,
  TrendingUp,
  TrendingDown,
  AlertTriangle,
} from "lucide-react"

// Token usage derived from real API responses, not estimated
export interface TokenUsage {
  promptTokens: number
  completionTokens: number
  totalTokens: number
  estimatedCost?: number // only if pricing is known from real API
  model: string
  provider: string
  timestamp: number
  source: "real" | "demo"
  requestId?: string
}

export interface TokenUsageState {
  entries: TokenUsage[]
  totalPromptTokens: number
  totalCompletionTokens: number
  totalTokens: number
  estimatedTotalCost: number
  todayTokens: number
  todayCost: number
  topModels: Array<{ model: string; tokens: number; cost: number }>
  source: "real" | "demo"
}

import { atom, computed } from "nanostores"

const initialTokenState: TokenUsageState = {
  entries: [],
  totalPromptTokens: 0,
  totalCompletionTokens: 0,
  totalTokens: 0,
  estimatedTotalCost: 0,
  todayTokens: 0,
  todayCost: 0,
  topModels: [],
  source: "real",
}

export const tokenStore = atom<TokenUsageState>(initialTokenState)

export const recentTokens = computed(tokenStore, (s) =>
  s.entries.slice(-50).reverse(),
)

export function recordTokenUsage(usage: TokenUsage): void {
  const state = tokenStore.get()
  const entries = [...state.entries, usage]
  const today = new Date().toDateString()
  const todayEntries = entries.filter(e => new Date(e.timestamp).toDateString() === today)
  
  const topModels = computeTopModels(entries)
  
  tokenStore.set({
    entries,
    totalPromptTokens: entries.reduce((a, e) => a + e.promptTokens, 0),
    totalCompletionTokens: entries.reduce((a, e) => a + e.completionTokens, 0),
    totalTokens: entries.reduce((a, e) => a + e.totalTokens, 0),
    estimatedTotalCost: entries.reduce((a, e) => a + (e.estimatedCost || 0), 0),
    todayTokens: todayEntries.reduce((a, e) => a + e.totalTokens, 0),
    todayCost: todayEntries.reduce((a, e) => a + (e.estimatedCost || 0), 0),
    topModels,
    source: usage.source,
  })
}

function computeTopModels(entries: TokenUsage[]): Array<{ model: string; tokens: number; cost: number }> {
  const modelMap = new Map<string, { tokens: number; cost: number }>()
  entries.forEach(e => {
    const existing = modelMap.get(e.model) || { tokens: 0, cost: 0 }
    modelMap.set(e.model, {
      tokens: existing.tokens + e.totalTokens,
      cost: existing.cost + (e.estimatedCost || 0),
    })
  })
  return Array.from(modelMap.entries())
    .map(([model, data]) => ({ model, ...data }))
    .sort((a, b) => b.tokens - a.tokens)
    .slice(0, 5)
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return n.toString()
}

function formatCost(c: number): string {
  if (c >= 1) return `$${c.toFixed(2)}`
  return `$${(c * 100).toFixed(1)}¢`
}

export function TokenUsagePanel() {
  const state = useStore(tokenStore)
  const recent = useStore(recentTokens)
  const mode = getDashboardMode()

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-foreground">Token Usage</h3>
        <div className="flex items-center gap-2">
          {mode === "demo" && (
            <span className="rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
              Demo
            </span>
          )}
          <span className="text-xs text-muted-foreground">
            {formatTokens(state.totalTokens)} total
          </span>
        </div>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <div className="rounded-lg border border-border bg-card p-3">
          <p className="text-[10px] text-muted-foreground">Today</p>
          <p className="text-lg font-bold">{formatTokens(state.todayTokens)}</p>
          <p className="text-[10px] text-muted-foreground">{formatCost(state.todayCost)}</p>
        </div>
        <div className="rounded-lg border border-border bg-card p-3">
          <p className="text-[10px] text-muted-foreground">Prompt</p>
          <p className="text-lg font-bold">{formatTokens(state.totalPromptTokens)}</p>
        </div>
        <div className="rounded-lg border border-border bg-card p-3">
          <p className="text-[10px] text-muted-foreground">Completion</p>
          <p className="text-lg font-bold">{formatTokens(state.totalCompletionTokens)}</p>
        </div>
        <div className="rounded-lg border border-border bg-card p-3">
          <p className="text-[10px] text-muted-foreground">Est. Cost</p>
          <p className="text-lg font-bold">{formatCost(state.estimatedTotalCost)}</p>
          {state.source === "demo" && (
            <p className="text-[9px] text-yellow-400">Demo pricing</p>
          )}
        </div>
      </div>

      {state.topModels.length > 0 && (
        <div className="rounded-lg border border-border bg-card p-3">
          <h4 className="text-xs font-medium mb-2">Top Models</h4>
          <div className="space-y-2">
            {state.topModels.map(m => (
              <div key={m.model} className="space-y-1">
                <div className="flex items-center justify-between text-xs">
                  <span className="truncate flex-1">{m.model}</span>
                  <span className="text-muted-foreground">{formatTokens(m.tokens)}</span>
                </div>
                <Progress 
                  value={state.totalTokens > 0 ? (m.tokens / state.totalTokens) * 100 : 0} 
                  className="h-1" 
                />
              </div>
            ))}
          </div>
        </div>
      )}

      {recent.length === 0 && (
        <div className="rounded-lg border border-border bg-card p-4 text-center">
          <p className="text-sm text-muted-foreground">
            {mode === "demo" ? "No real token data (demo mode)" : "No token usage recorded"}
          </p>
        </div>
      )}

      {recent.length > 0 && (
        <div className="rounded-lg border border-border bg-card p-3">
          <h4 className="text-xs font-medium mb-2">Recent Requests</h4>
          <div className="space-y-1 max-h-32 overflow-y-auto">
            {recent.slice(0, 10).map((entry, i) => (
              <div key={i} className="flex items-center justify-between text-[10px]">
                <span className="truncate flex-1">{entry.model}</span>
                <span className="text-muted-foreground">{formatTokens(entry.totalTokens)}</span>
                {entry.source === "demo" && (
                  <AlertTriangle className="h-3 w-3 text-yellow-500 ml-1" />
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
