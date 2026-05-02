/**
 * CapabilityList component - uses platform hooks only.
 */

"use client"

import { useCapabilities } from "@/platform/ui/useCapabilities"

export function CapabilityList() {
  const { capabilities, isLoading, error, loadCapabilities } = useCapabilities()

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
        {capabilities.map((cap) => (
          <div
            key={Buffer.from(cap.capability_id).toString("hex")}
            className="rounded-lg border border-border bg-secondary/50 p-3"
          >
            <h3 className="text-sm font-medium">
              {cap.provider_name || "Unnamed"}
            </h3>
            <p className="text-xs text-muted-foreground">
              Role: {cap.role}
            </p>
            <div className="mt-2 flex gap-2 text-[9px]">
              {cap.modalities && cap.modalities.length > 0 && (
                <span className="rounded bg-primary/10 px-1.5 py-0.5">
                  {cap.modalities.join(", ")}
                </span>
              )}
              {cap.operations && cap.operations.length > 0 && (
                <span className="rounded bg-yellow-500/10 px-1.5 py-0.5 text-yellow-600">
                  {cap.operations.length} operation(s)
                </span>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
