/**
 * CapabilityList component - uses platform hooks only.
 */

"use client"

import { useCapabilities } from "@/platform/ui/useCapabilities"

export function CapabilityList() {
  const { descriptors, isLoading, error, loadCapabilities } = useCapabilities()

  if (isLoading) return <div className="p-4">Loading capabilities...</div>
  if (error) return <div className="p-4 text-red-500">Error: {error}</div>

  return (
    <div className="p-4">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-lg font-semibold">Capabilities</h2>
        <button
          onClick={loadCapabilities}
          className="rounded-md bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary hover:bg-primary/20"
        >
          Refresh
        </button>
      </div>
      <div className="grid grid-cols-2 gap-3">
        {descriptors.map((cap) => (
          <div
            key={cap.capabilityId}
            className="rounded-lg border border-border bg-secondary/50 p-3"
          >
            <h3 className="text-sm font-medium">{cap.label}</h3>
            <p className="text-xs text-muted-foreground">{cap.description}</p>
            <div className="mt-2 flex gap-2 text-[9px]">
              <span className="rounded bg-primary/10 px-1.5 py-0.5">
                {cap.riskClass}
              </span>
              {cap.requiresUserPresence && (
                <span className="rounded bg-yellow-500/10 px-1.5 py-0.5 text-yellow-600">
                  Requires presence
                </span>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
