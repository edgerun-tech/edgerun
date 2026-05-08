"use client"

import type React from "react"
import { cn } from "@/lib/utils"

export function AppHeader({
  icon,
  title,
  children,
  aside,
}: {
  icon?: React.ReactNode
  title: string
  children?: React.ReactNode
  aside?: React.ReactNode
}) {
  return (
    <div className="flex shrink-0 flex-col gap-3 border-b border-border bg-[var(--window-header)]/55 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
      <div className="flex min-w-0 items-center gap-3">
        {icon ? (
          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-primary/20 bg-primary/15 text-primary">
            {icon}
          </div>
        ) : null}
        <div className="min-w-0">
          <h2 className="truncate text-sm font-semibold text-foreground">{title}</h2>
          {children ? <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground">{children}</div> : null}
        </div>
      </div>
      {aside ? <div className="sm:mr-11">{aside}</div> : null}
    </div>
  )
}

export function AppToolbar({ className, children }: { className?: string; children: React.ReactNode }) {
  return (
    <div className={cn("flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between", className)}>
      {children}
    </div>
  )
}

export function AppEmptyState({ icon, children }: { icon?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="flex h-40 flex-col items-center justify-center gap-2 rounded-md border border-dashed border-border text-center text-xs text-muted-foreground">
      {icon}
      {children}
    </div>
  )
}
