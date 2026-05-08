"use client"

import { useEffect, useRef } from "react"
import { X } from "lucide-react"
import type { AppSurfaceDef } from "@/stores/desktop-store"
import { cn } from "@/lib/utils"

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
  const dialogRef = useRef<HTMLDivElement>(null)
  const previousFocusRef = useRef<HTMLElement | null>(null)

  useEffect(() => {
    if (!activeSurface) return
    previousFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null
    dialogRef.current?.focus()

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault()
        onClose(activeSurface.id)
        return
      }

      if (event.key !== "Tab" || !dialogRef.current) return
      const focusable = Array.from(dialogRef.current.querySelectorAll<HTMLElement>(
        "a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex='-1'])",
      )).filter((element) => !element.hasAttribute("disabled") && element.offsetParent !== null)
      if (focusable.length === 0) {
        event.preventDefault()
        dialogRef.current.focus()
        return
      }
      const first = focusable[0]
      const last = focusable[focusable.length - 1]
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault()
        last.focus()
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault()
        first.focus()
      }
    }

    window.addEventListener("keydown", onKeyDown)
    return () => {
      window.removeEventListener("keydown", onKeyDown)
      previousFocusRef.current?.focus()
      previousFocusRef.current = null
    }
  }, [activeSurface, onClose])

  if (!activeSurface) return null

  const closeOnOutsideClick = activeSurface.dismissOnOutsideClick

  return (
    <div
      className="absolute inset-0 z-40 flex items-center justify-center bg-black/35 p-3 backdrop-blur-[2px] animate-in fade-in-0 duration-150 sm:p-6"
      onPointerDown={() => {
        if (closeOnOutsideClick) onClose(activeSurface.id)
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={activeSurface.title}
        tabIndex={-1}
        data-app-overlay-id={activeSurface.id}
        data-app-id={activeSurface.appId}
        data-surface-variant={activeSurface.variant}
        className={cn(
          "relative overflow-hidden border border-white/10 bg-background/92 shadow-[0_32px_120px_rgba(0,0,0,0.78)] outline-none ring-1 ring-white/5 animate-in zoom-in-95 duration-150",
          "h-[calc(100vh-2rem)] w-[calc(100vw-1.5rem)] rounded-2xl sm:h-[calc(100vh-7rem)] sm:w-[calc(100vw-3rem)] sm:rounded-[28px]",
          activeSurface.variant === "compact" && "sm:h-[min(620px,calc(100vh-7rem))] sm:max-w-[560px]",
          activeSurface.variant === "standard" && "sm:max-w-[980px]",
          activeSurface.variant === "wide" && "sm:max-w-[1180px]",
          activeSurface.variant === "full" && "sm:max-w-[1440px]",
        )}
        onPointerDown={(event) => {
          event.stopPropagation()
          onFocus(activeSurface.id)
        }}
      >
        <button
          type="button"
          onClick={() => onClose(activeSurface.id)}
          aria-label={`Close ${activeSurface.title}`}
          className="absolute right-3 top-3 z-20 flex h-8 w-8 items-center justify-center rounded-md border border-border bg-background/85 text-muted-foreground shadow-sm backdrop-blur hover:bg-secondary hover:text-foreground"
        >
          <X className="h-4 w-4" />
        </button>
        {activeSurface.component}
      </div>
    </div>
  )
}
