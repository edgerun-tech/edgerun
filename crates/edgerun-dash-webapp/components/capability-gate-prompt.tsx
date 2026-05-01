"use client"

import { useState, useCallback } from "react"
import { Shield, X, KeyRound, Loader2 } from "lucide-react"
import { cn } from "@/lib/utils"

interface CapabilityGatePromptProps {
  appName: string
  blockedCapabilities: string[]
  onSetupIdentity: (name: string) => Promise<boolean>
  onDismiss: () => void
}

export function CapabilityGatePrompt({ appName, blockedCapabilities, onSetupIdentity, onDismiss }: CapabilityGatePromptProps) {
  const [name, setName] = useState("")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleSetup = useCallback(async () => {
    if (!name.trim()) {
      setError("Enter a display name")
      return
    }
    setLoading(true)
    setError(null)
    try {
      const ok = await onSetupIdentity(name.trim())
      if (!ok) {
        setError("Setup cancelled or failed")
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Setup failed")
    } finally {
      setLoading(false)
    }
  }, [name, onSetupIdentity])

  return (
    <div className="flex h-full flex-col p-4">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-foreground">Capability Required</h2>
        <button onClick={onDismiss} className="rounded p-1 hover:bg-secondary">
          <X className="h-4 w-4 text-muted-foreground" />
        </button>
      </div>

      <div className="mb-4 rounded-lg border border-border bg-secondary/30 p-3">
        <div className="flex items-center gap-2 text-sm text-foreground">
          <Shield className="h-4 w-4 text-[var(--status-warning)]" />
          <span><strong>{appName}</strong> requires capabilities you don't have yet.</span>
        </div>
        <div className="mt-2 flex flex-wrap gap-1">
          {blockedCapabilities.map((cap) => (
            <span key={cap} className="rounded bg-[var(--status-error)]/10 px-2 py-0.5 text-[10px] font-medium text-[var(--status-error)]">
              {cap}
            </span>
          ))}
        </div>
      </div>

      <div className="mb-4">
        <p className="mb-2 text-xs text-muted-foreground">
          Set up your Edgerun identity (passkey/biometric) to unlock these features.
        </p>
        <label className="mb-1 block text-xs font-medium text-muted-foreground">Display Name</label>
        <input
          type="text"
          value={name}
          onChange={(e) => { setName(e.target.value); setError(null) }}
          placeholder="Your name"
          className="w-full rounded-md border border-border bg-secondary px-2 py-1.5 text-xs text-foreground placeholder:text-muted-foreground/50 outline-none focus:border-primary/50"
          onKeyDown={(e) => { if (e.key === "Enter") handleSetup() }}
        />
      </div>

      {error && (
        <div className="mb-3 rounded-md bg-[var(--status-error)]/10 px-3 py-2 text-xs text-[var(--status-error)]">
          {error}
        </div>
      )}

      <button
        onClick={handleSetup}
        disabled={loading}
        className={cn(
          "flex w-full items-center justify-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition-colors",
          loading
            ? "bg-primary/50 text-primary-foreground/70"
            : "bg-primary text-primary-foreground hover:bg-primary/90"
        )}
      >
        {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <KeyRound className="h-4 w-4" />}
        {loading ? "Setting up..." : "Set up Identity"}
      </button>

      <button
        onClick={onDismiss}
        className="mt-2 w-full rounded-md bg-secondary px-3 py-1.5 text-xs font-medium text-secondary-foreground hover:bg-secondary/80"
      >
        Maybe later
      </button>
    </div>
  )
}
