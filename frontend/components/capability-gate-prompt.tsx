"use client"

import { Shield, X, Check, AlertTriangle } from "lucide-react"
import { cn } from "@/lib/utils"
import { describeCapabilityList, riskTone } from "@/platform/capabilities/capability-catalog"

interface CapabilityGatePromptProps {
  appName: string
  blockedCapabilities: string[]
  onGrant: () => void
  onDismiss: () => void
}

export function CapabilityGatePrompt({ appName, blockedCapabilities, onGrant, onDismiss }: CapabilityGatePromptProps) {
  const capabilities = describeCapabilityList(blockedCapabilities)
  const hasHighRisk = capabilities.some((cap) => cap.risk === "high")

  return (
    <div className="flex h-full flex-col p-4">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="text-sm font-semibold text-foreground">Permission request</h2>
          <p className="text-xs text-muted-foreground">{appName} is asking for local capabilities.</p>
        </div>
        <button onClick={onDismiss} className="rounded p-1 hover:bg-secondary">
          <X className="h-4 w-4 text-muted-foreground" />
        </button>
      </div>

      <div className="mb-3 rounded-lg border border-border bg-secondary/30 p-3">
        <div className="flex items-start gap-2 text-sm text-foreground">
          <Shield className="mt-0.5 h-4 w-4 text-primary" />
          <div>
            <div className="font-medium">Why this appears</div>
            <p className="mt-1 text-xs text-muted-foreground">
              Edgerun capabilities are bounded local permissions. Granting lets this app use only the listed actions for this app context. It does not give global control or permanent node authority.
            </p>
          </div>
        </div>
      </div>

      {hasHighRisk && (
        <div className="mb-3 flex gap-2 rounded-lg border border-[var(--status-warning)]/20 bg-[var(--status-warning)]/10 p-3 text-xs text-[var(--status-warning)]">
          <AlertTriangle className="h-4 w-4 flex-shrink-0" />
          <span>One or more requested capabilities can affect private data, network behavior, payments, files, or live device access.</span>
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-auto space-y-2 pr-1">
        {capabilities.map((cap) => (
          <div key={cap.id} className="rounded-lg border border-border bg-card p-3">
            <div className="flex items-start justify-between gap-2">
              <div>
                <div className="text-sm font-medium text-foreground">{cap.label}</div>
                <div className="mt-0.5 text-xs text-muted-foreground">{cap.short}</div>
              </div>
              <span className={cn("rounded border px-1.5 py-0.5 text-[10px] font-medium uppercase", riskTone(cap.risk))}>
                {cap.risk}
              </span>
            </div>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{cap.why}</p>
          </div>
        ))}
      </div>

      <div className="mt-4 grid grid-cols-2 gap-2">
        <button
          onClick={onDismiss}
          className="rounded-md bg-secondary px-3 py-2 text-xs font-medium text-secondary-foreground hover:bg-secondary/80"
        >
          Not now
        </button>
        <button
          onClick={onGrant}
          className="flex items-center justify-center gap-1.5 rounded-md bg-primary px-3 py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90"
        >
          <Check className="h-3.5 w-3.5" />
          Grant and open
        </button>
      </div>
    </div>
  )
}
