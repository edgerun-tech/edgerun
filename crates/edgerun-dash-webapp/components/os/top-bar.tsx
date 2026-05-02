"use client"

import { useState, useEffect } from "react"
import { Cpu, HardDrive, Wifi, WifiOff, Activity, Server, Lock, Sparkles } from "lucide-react"
import { cn } from "@/lib/utils"
import { EdgerunLogo } from "./edgerun-logo"
import { launchApp } from "@/stores/app-launcher"
import { availableApps } from "./app-store"

interface TopBarProps {
  nodeCount: number
  activeSessions: number
  ramUsage: { used: number; total: number }
  isConnected: boolean
  username?: string
  isGuest?: boolean
  onLock?: () => void
  onSignIn?: () => void
}

export function TopBar({ nodeCount, activeSessions, ramUsage, isConnected, username, isGuest, onLock, onSignIn }: TopBarProps) {
  const [time, setTime] = useState<Date | null>(null)

  useEffect(() => {
    setTime(new Date())
    const interval = setInterval(() => setTime(new Date()), 1000)
    return () => clearInterval(interval)
  }, [])

  const ramPercentage = Math.round((ramUsage.used / ramUsage.total) * 100)

  return (
    <div className="fixed left-0 right-0 top-0 z-50 flex h-10 items-center justify-between border-b border-border bg-[var(--window-header)] px-4 backdrop-blur-sm">
      {/* Left Section - Logo & Status */}
      <div className="flex items-center gap-6">
        <EdgerunLogo variant="full" size="sm" className="text-primary" />

        <div className="flex items-center gap-4 text-xs">
          {/* Node Count */}
          <div className="flex items-center gap-1.5">
            <Server className="h-3.5 w-3.5 text-primary" />
            <span className="text-muted-foreground">Nodes:</span>
            <span className="font-mono font-medium text-foreground">{nodeCount}</span>
          </div>

          {/* Active Sessions */}
          <div className="flex items-center gap-1.5">
            <Activity className="h-3.5 w-3.5 text-[var(--status-online)]" />
            <span className="text-muted-foreground">Sessions:</span>
            <span className="font-mono font-medium text-foreground">{activeSessions}</span>
          </div>
        </div>
      </div>

      {/* Center Section - RAM Usage */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <Cpu className="h-3.5 w-3.5 text-muted-foreground" />
          <div className="flex items-center gap-2">
            <div className="h-1.5 w-24 overflow-hidden rounded-full bg-secondary">
              <div
                className={cn(
                  "h-full rounded-full transition-all duration-500",
                  ramPercentage > 80
                    ? "bg-[var(--status-error)]"
                    : ramPercentage > 60
                    ? "bg-[var(--status-warning)]"
                    : "bg-primary"
                )}
                style={{ width: `${ramPercentage}%` }}
              />
            </div>
            <span className="font-mono text-xs text-muted-foreground">
              {ramUsage.used.toFixed(1)}GB / {ramUsage.total}GB
            </span>
          </div>
        </div>

        <div className="h-4 w-px bg-border" />

        <div className="flex items-center gap-1.5">
          <HardDrive className="h-3.5 w-3.5 text-muted-foreground" />
          <span className="font-mono text-xs text-muted-foreground">SSD</span>
        </div>
      </div>

      {/* Right Section - Connection, User & Time */}
      <div className="flex items-center gap-4">
        {/* WebRTC Status */}
        <div className="flex items-center gap-1.5">
          {isConnected ? (
            <>
              <Wifi className="h-3.5 w-3.5 text-[var(--status-online)]" />
              <span className="text-xs text-[var(--status-online)]">Connected</span>
            </>
          ) : (
            <>
              <WifiOff className="h-3.5 w-3.5 text-[var(--status-error)]" />
              <span className="text-xs text-[var(--status-error)]">Disconnected</span>
            </>
          )}
        </div>

        <div className="h-4 w-px bg-border" />

        {/* Time — null until mounted to avoid SSR hydration mismatch */}
        <div className="font-mono text-xs text-muted-foreground tabular-nums w-16">
          {time
            ? time.toLocaleTimeString("en-US", {
                hour: "2-digit",
                minute: "2-digit",
                second: "2-digit",
                hour12: false,
              })
            : "--:--:--"}
        </div>

        {/* AI Assistant Button */}
        <button
          onClick={() => {
            const aiApp = availableApps.find(a => a.id === "ai-assistant")
            if (aiApp) launchApp(aiApp)
          }}
          title="AI Assistant (Ctrl+Shift+A)"
          className="flex items-center gap-1.5 rounded-md bg-primary/10 px-2 py-1 text-xs text-primary transition-colors hover:bg-primary/20"
        >
          <Sparkles className="h-3.5 w-3.5" />
          <span className="font-medium">AI</span>
          <span className="text-[10px] opacity-60">⌘⇧A</span>
        </button>

        {/* User + Lock */}
        {isGuest && onSignIn && (
          <>
            <div className="h-4 w-px bg-border" />
            <button
              onClick={onSignIn}
              title="Set up identity to unlock all features"
              className="flex items-center gap-1.5 rounded px-1.5 py-1 text-xs text-[var(--status-warning)] transition-colors hover:bg-secondary"
            >
              <span className="font-mono">Guest</span>
              <Lock className="h-3 w-3" />
            </button>
          </>
        )}
        {username && !isGuest && onLock && (
          <>
            <div className="h-4 w-px bg-border" />
            <button
              onClick={onLock}
              title="Lock session"
              className="flex items-center gap-1.5 rounded px-1.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
            >
              <span className="font-mono">{username}</span>
              <Lock className="h-3 w-3" />
            </button>
          </>
        )}
      </div>
    </div>
  )
}
