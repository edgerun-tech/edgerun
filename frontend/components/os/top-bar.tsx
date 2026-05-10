"use client"

import { useEffect } from "react"
import { useStore } from "@nanostores/react"
import { HardDrive, Wifi, WifiOff, Activity, Server, Lock } from "lucide-react"
import { cn } from "@/lib/utils"
import { EdgerunLogo } from "./edgerun-logo"
import { clientMountedStore, markClientMounted } from "@/stores/ui-runtime-store"

interface TopBarProps {
  nodeCount: number
  activeSessions: number
  ramUsage: { used: number; total: number }
  isConnected: boolean
  username?: string
  isGuest: boolean
  onLock: () => void
  onSignIn: () => void
}

export function TopBar({
  nodeCount,
  activeSessions,
  ramUsage,
  isConnected,
  username,
  isGuest,
  onLock,
  onSignIn,
}: TopBarProps) {
  const mounted = useStore(clientMountedStore)

  useEffect(() => {
    markClientMounted()
  }, [])

  return (
    <div className="flex h-10 items-center gap-3 border-b border-[var(--window-border)] px-4">
      <EdgerunLogo className="text-primary" size="sm" />

      <div className="ml-4 flex items-center gap-3 text-[10px] text-muted-foreground">
        <span className="flex items-center gap-1">
          <Server className="h-3 w-3" />
          {nodeCount} nodes
        </span>
        <span className="flex items-center gap-1">
          <Activity className="h-3 w-3" />
          {activeSessions} sessions
        </span>
        <span className="flex items-center gap-1">
          <HardDrive className="h-3 w-3" />
          {ramUsage.used.toFixed(1)} / {ramUsage.total} GB
        </span>
        <span className={cn(
          "flex items-center gap-1",
          isConnected ? "text-green-500" : "text-red-500"
        )}>
          {isConnected ? (
            <Wifi className="h-3 w-3" />
          ) : (
            <WifiOff className="h-3 w-3" />
          )}
          {isConnected ? "Connected" : "Offline"}
        </span>
      </div>

      <div className="ml-auto flex items-center gap-2">
        {mounted && username && (
          <span className="text-xs text-muted-foreground">
            {isGuest ? "Guest" : username}
          </span>
        )}
        {mounted && username ? (
          <button
            onClick={onLock}
            aria-label="Lock desktop"
            className="flex items-center gap-1 rounded px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Lock className="h-3 w-3" />
            <span className="hidden sm:inline">Lock</span>
          </button>
        ) : (
          <button
            onClick={onSignIn}
            aria-label="Sign in"
            className="flex items-center gap-1 rounded bg-primary/10 px-2 py-1 text-xs font-medium text-primary hover:bg-primary/20"
          >
            Sign In
          </button>
        )}
      </div>
    </div>
  )
}
