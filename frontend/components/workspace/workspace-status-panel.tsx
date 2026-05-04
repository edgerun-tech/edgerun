"use client"

import { useState, useCallback } from "react"
import { cn } from "@/lib/utils"
import {
  Circle,
  Brain,
  ListTodo,
  Clock,
} from "lucide-react"

interface WorkspaceStatusPanelProps {
  className?: string
  surface?: "floating" | "embedded"
}

export function WorkspaceStatusPanel({ className, surface = "floating" }: WorkspaceStatusPanelProps) {
  const [isExpanded, setIsExpanded] = useState(true)
  const [isHovered, setIsHovered] = useState(false)
  const embedded = surface === "embedded"

  return (
    <div
      className={cn(
        "pointer-events-auto flex flex-col gap-2 rounded-lg border border-[var(--border)] bg-[var(--bg)]/90 backdrop-blur-sm transition-opacity duration-200",
        embedded ? "h-full w-full overflow-hidden" : "fixed bottom-4 left-4 z-40",
        isHovered ? "opacity-100" : "opacity-80",
        className
      )}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between gap-2 px-3 py-2 text-xs font-medium hover:bg-[var(--bg-hover)]"
      >
        <div className="flex items-center gap-2">
          <Brain className="h-4 w-4 text-primary" />
          <span>Workspace Status</span>
        </div>
      </button>

      {isExpanded && (
        <div className="min-h-0 flex-1 overflow-auto px-3 pb-3">
          <div className={cn("grid gap-x-4 gap-y-1 text-[10px]", embedded ? "grid-cols-1" : "grid-cols-2")}>
            <div className="flex items-center gap-1.5">
              <Circle className="h-3 w-3 text-muted-foreground" />
              <span className="text-muted-foreground">Git:</span>
              <span className="font-mono">unknown</span>
            </div>

            <div className="flex items-center gap-1.5">
              <Circle className="h-3 w-3 text-muted-foreground" />
              <span className="text-muted-foreground">Agents:</span>
              <span className="font-mono">unknown</span>
            </div>

            <div className="flex items-center gap-1.5">
              <Circle className="h-3 w-3 text-muted-foreground" />
              <span className="text-muted-foreground">Verify:</span>
              <span className="font-mono">not_run</span>
            </div>

            <div className="flex items-center gap-1.5">
              <Circle className="h-3 w-3 text-muted-foreground" />
              <span className="text-muted-foreground">Deps:</span>
              <span className="font-mono">approval required</span>
            </div>

            <div className="flex items-center gap-1.5">
              <Clock className="h-3 w-3 text-muted-foreground" />
              <span className="text-muted-foreground">Ctx:</span>
              <span className="font-mono text-green-500">0s</span>
            </div>

            <div className="flex items-center gap-1.5">
              <ListTodo className="h-3 w-3 text-muted-foreground" />
              <span className="text-muted-foreground">Turn:</span>
              <span className="font-mono">18</span>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

export function useWorkspaceAwareness() {
  const refreshGitStatus = useCallback(() => {
    if (typeof window === "undefined") return
  }, [])

  return { refreshGitStatus }
}
