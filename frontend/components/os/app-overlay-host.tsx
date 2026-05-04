"use client"

import { useEffect } from "react"
import type { OpenWindowDef } from "@/stores/desktop-store"

type AppOverlayHostProps = {
  windows: OpenWindowDef[]
  windowOrder: string[]
  focusedWindowId: string | null
  onFocus: (id: string) => void
  onClose: (id: string) => void
}

function getActiveWindow(windows: OpenWindowDef[], windowOrder: string[], focusedWindowId: string | null) {
  if (focusedWindowId) {
    const focused = windows.find((win) => win.id === focusedWindowId)
    if (focused) return focused
  }

  for (let i = windowOrder.length - 1; i >= 0; i--) {
    const win = windows.find((candidate) => candidate.id === windowOrder[i])
    if (win) return win
  }

  return windows[windows.length - 1] ?? null
}

export function AppOverlayHost({
  windows,
  windowOrder,
  focusedWindowId,
  onFocus,
  onClose,
}: AppOverlayHostProps) {
  const activeWindow = getActiveWindow(windows, windowOrder, focusedWindowId)

  useEffect(() => {
    if (!activeWindow) return

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return
      event.preventDefault()
      onClose(activeWindow.id)
    }

    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [activeWindow, onClose])

  if (!activeWindow) return null

  return (
    <div
      className="absolute inset-0 z-40 flex items-center justify-center bg-black/35 p-6 backdrop-blur-[2px] animate-in fade-in-0 duration-150"
      onMouseDown={() => onClose(activeWindow.id)}
    >
      <div
        data-app-overlay-id={activeWindow.id}
        className="h-[calc(100vh-7rem)] max-h-[920px] w-[min(1280px,calc(100vw-3rem))] overflow-hidden rounded-[28px] border border-white/10 bg-background/92 shadow-[0_32px_120px_rgba(0,0,0,0.78)] ring-1 ring-white/5 animate-in zoom-in-95 duration-150"
        onMouseDown={(event) => {
          event.stopPropagation()
          onFocus(activeWindow.id)
        }}
      >
        {activeWindow.component}
      </div>
    </div>
  )
}
