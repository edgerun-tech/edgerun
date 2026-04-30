"use client"

import { useCallback } from "react"
import { cn } from "@/lib/utils"

interface StageWindow {
  id: string
  appId: string
  title: string
  icon: React.ReactNode
}

interface StageManagerProps {
  windows: StageWindow[]
  focusedWindowId: string | null
  onFocus: (windowId: string) => void
  onClose: (windowId: string) => void
  enabled: boolean
}

export function StageManager({
  windows,
  focusedWindowId,
  onFocus,
  onClose,
  enabled,
}: StageManagerProps) {
  // Only show non-focused windows in the strip
  const stripWindows = windows.filter((w) => w.id !== focusedWindowId)

  const handleClick = useCallback(
    (e: React.MouseEvent, windowId: string) => {
      e.stopPropagation()
      onFocus(windowId)
    },
    [onFocus]
  )

  const handleClose = useCallback(
    (e: React.MouseEvent, windowId: string) => {
      e.stopPropagation()
      onClose(windowId)
    },
    [onClose]
  )

  if (!enabled || windows.length === 0) return null

  return (
    <div
      className="absolute left-0 top-0 bottom-16 z-30 flex w-[88px] flex-col gap-2 overflow-y-auto overflow-x-hidden py-3 pl-2 pr-1"
      style={{ scrollbarWidth: "none" }}
    >
      {stripWindows.map((win) => (
        <button
          key={win.id}
          onClick={(e) => handleClick(e, win.id)}
          className={cn(
            "group relative flex h-[68px] w-[76px] flex-shrink-0 flex-col items-center justify-center gap-1.5 rounded-xl border border-[var(--window-border)] bg-[var(--window-bg)]/70 backdrop-blur-md transition-all duration-200",
            "hover:border-primary/40 hover:bg-[var(--window-header)] hover:shadow-lg",
            "active:scale-95"
          )}
          title={win.title}
        >
          {/* App icon */}
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-secondary/60 text-muted-foreground transition-colors group-hover:text-foreground">
            {win.icon}
          </div>
          {/* App name */}
          <span className="max-w-[64px] truncate text-center font-mono text-[9px] leading-none text-muted-foreground group-hover:text-foreground">
            {win.title}
          </span>
          {/* Close button — appears on hover */}
          <button
            onClick={(e) => handleClose(e, win.id)}
            className="absolute -right-1.5 -top-1.5 hidden h-4 w-4 items-center justify-center rounded-full bg-destructive text-[8px] text-destructive-foreground group-hover:flex"
          >
            ×
          </button>
        </button>
      ))}

      {/* Add-window hint when strip is empty but stage mode on */}
      {stripWindows.length === 0 && windows.length > 0 && (
        <div className="flex h-[68px] w-[76px] items-center justify-center rounded-xl border border-dashed border-[var(--window-border)]/40">
          <span className="font-mono text-[9px] text-muted-foreground/40">stage</span>
        </div>
      )}
    </div>
  )
}
