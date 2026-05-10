"use client"

import { getDashboardMode } from "@/platform/runtime/dashboard-mode"
import { cn } from "@/lib/utils"
import type { ReactNode } from "react"

interface DashboardPanelProps {
  title: string
  summary?: string
  children: ReactNode
  mode?: ReturnType<typeof getDashboardMode>
  empty?: boolean
  emptyMessage?: string
  demoMessage?: string
  className?: string
}

export function DashboardPanel({
  title,
  summary,
  children,
  mode: modeProp,
  empty,
  emptyMessage,
  demoMessage,
  className,
}: DashboardPanelProps) {
  const mode = modeProp ?? getDashboardMode()
  const isDemo = mode === "demo"

  return (
    <div className={cn("space-y-3", className)}>
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-foreground">{title}</h3>
        <div className="flex items-center gap-2">
          {isDemo && (
            <span className="rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
              Demo
            </span>
          )}
          {summary && <span className="text-xs text-muted-foreground">{summary}</span>}
        </div>
      </div>

      {empty ? (
        <div className="rounded-lg border border-border bg-card p-4 text-center">
          <p className="text-sm text-muted-foreground">
            {isDemo ? demoMessage ?? "No real data (demo mode)" : emptyMessage ?? "No data"}
          </p>
        </div>
      ) : (
        children
      )}
    </div>
  )
}

export function DemoBadge() {
  return (
    <span className="rounded bg-yellow-500/20 px-1.5 py-0.5 text-[9px] font-medium text-yellow-400">
      Demo
    </span>
  )
}
