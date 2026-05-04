"use client"

import { useState, type ReactNode } from "react"
import { useStore } from "@nanostores/react"
import { Globe2, Layers3, Network, RotateCcw, Zap } from "lucide-react"
import { cn } from "@/lib/utils"
import { xrayState, resetView, setLayout, setRuntimeMode } from "./graph/graph-store"
import { setRuntimeOverlayEnabled } from "./runtime/runtime-store"
import type { LayoutType } from "./graph/types"

type XrayCommandSurfaceProps = {
  className?: string
  surface?: "vertical" | "inline"
}

type IconActionProps = {
  active?: boolean
  label: string
  children: ReactNode
  onClick: () => void
}

function IconAction({ active, label, children, onClick }: IconActionProps) {
  const [showTip, setShowTip] = useState(false)

  return (
    <button
      type="button"
      onClick={onClick}
      onMouseEnter={() => {
        window.setTimeout(() => setShowTip(true), 650)
      }}
      onMouseLeave={() => setShowTip(false)}
      className={cn(
        "group relative flex h-8 w-8 items-center justify-center rounded-full transition-colors",
        active ? "text-emerald-400" : "text-zinc-500 hover:text-zinc-300",
      )}
      aria-label={label}
      title={label}
    >
      {children}
      {active && <span className="absolute -right-1 top-1/2 h-1.5 w-1.5 -translate-y-1/2 rounded-full bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.7)]" />}
      {showTip && (
        <span className="pointer-events-none absolute right-full mr-2 whitespace-nowrap rounded-md border border-border bg-card px-2 py-1 text-[10px] text-card-foreground shadow-xl">
          {label}
        </span>
      )}
    </button>
  )
}

export function XrayCommandSurface({ className, surface = "vertical" }: XrayCommandSurfaceProps) {
  const state = useStore(xrayState)
  const vertical = surface === "vertical"

  const controls = [
    {
      key: "force",
      label: "Force layout",
      active: state.layout === "force",
      icon: <Network className="h-4 w-4" />,
      onClick: () => setLayout("force" as LayoutType),
    },
    {
      key: "globe",
      label: "Globe layout",
      active: state.layout === "globe",
      icon: <Globe2 className="h-4 w-4" />,
      onClick: () => setLayout("globe" as LayoutType),
    },
    {
      key: "layers",
      label: "Layer layout",
      active: state.layout === "layers",
      icon: <Layers3 className="h-4 w-4" />,
      onClick: () => setLayout("layers" as LayoutType),
    },
    {
      key: "runtime",
      label: state.runtimeMode ? "Runtime overlay on" : "Runtime overlay off",
      active: state.runtimeMode,
      icon: <Zap className="h-4 w-4" />,
      onClick: () => {
        setRuntimeMode(!state.runtimeMode)
        setRuntimeOverlayEnabled(!state.runtimeMode)
      },
    },
    {
      key: "reset",
      label: `Reset view · ${state.nodes.size} nodes · ${state.edges.length} edges`,
      active: false,
      icon: <RotateCcw className="h-4 w-4" />,
      onClick: resetView,
    },
  ]

  return (
    <div
      className={cn(
        "pointer-events-auto flex gap-1",
        vertical ? "flex-col items-center" : "items-center",
        className,
      )}
    >
      {controls.map((control) => (
        <IconAction key={control.key} active={control.active} label={control.label} onClick={control.onClick}>
          {control.icon}
        </IconAction>
      ))}
    </div>
  )
}
