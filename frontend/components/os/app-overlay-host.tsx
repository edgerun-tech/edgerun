"use client"

import { useEffect } from "react"
import type { AppSurfaceDef } from "@/stores/desktop-store"

type AppOverlayHostProps = {
  surfaces: AppSurfaceDef[]
  surfaceOrder: string[]
  focusedSurfaceId: string | null
  onFocus: (id: string) => void
  onClose: (id: string) => void
}

function getActiveOverlay(surfaces: AppSurfaceDef[], surfaceOrder: string[], focusedSurfaceId: string | null) {
  const overlays = surfaces.filter((surface) => surface.kind === "overlay")

  if (focusedSurfaceId) {
    const focused = overlays.find((surface) => surface.id === focusedSurfaceId)
    if (focused) return focused
  }

  for (let i = surfaceOrder.length - 1; i >= 0; i--) {
    const surface = overlays.find((candidate) => candidate.id === surfaceOrder[i])
    if (surface) return surface
  }

  return overlays[overlays.length - 1] ?? null
}

export function AppOverlayHost({
  surfaces,
  surfaceOrder,
  focusedSurfaceId,
  onFocus,
  onClose,
}: AppOverlayHostProps) {
  const activeSurface = getActiveOverlay(surfaces, surfaceOrder, focusedSurfaceId)

  useEffect(() => {
    if (!activeSurface) return

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return
      event.preventDefault()
      onClose(activeSurface.id)
    }

    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [activeSurface, onClose])

  if (!activeSurface) return null

  const closeOnOutsideClick = activeSurface.dismissOnOutsideClick

  return (
    <div
      className="absolute inset-0 z-40 flex items-center justify-center bg-black/35 p-6 backdrop-blur-[2px] animate-in fade-in-0 duration-150"
      onMouseDown={() => {
        if (closeOnOutsideClick) onClose(activeSurface.id)
      }}
    >
      <div
        data-app-overlay-id={activeSurface.id}
        data-app-id={activeSurface.appId}
        className="h-[calc(100vh-7rem)] max-h-[920px] w-[min(1280px,calc(100vw-3rem))] overflow-hidden rounded-[28px] border border-white/10 bg-background/92 shadow-[0_32px_120px_rgba(0,0,0,0.78)] ring-1 ring-white/5 animate-in zoom-in-95 duration-150"
        style={{
          width: activeSurface.defaultSize?.width
            ? `min(${activeSurface.defaultSize.width}px, calc(100vw - 3rem))`
            : undefined,
          height: activeSurface.defaultSize?.height
            ? `min(${activeSurface.defaultSize.height}px, calc(100vh - 7rem))`
            : undefined,
        }}
        onMouseDown={(event) => {
          event.stopPropagation()
          onFocus(activeSurface.id)
        }}
      >
        {activeSurface.component}
      </div>
    </div>
  )
}
