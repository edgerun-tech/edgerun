/**
 * AppList component - uses platform hooks only.
 * No direct protocol access, no manual fetching.
 */

"use client"

import { useApps } from "@/platform/ui/useApps"
import { useCapabilities } from "@/platform/ui/useCapabilities"
import { edgerun as streamTypes } from "@/gen/edgerun/v0/stream"
import { edgerun as capTypes } from "@/gen/edgerun/v0/capability"

interface AppListProps {
  apps: streamTypes.v0.stream.AppPackage[]
  getGrants: (appId: string) => unknown[]
  onRefresh: () => void
}

export function AppList({ apps, getGrants, onRefresh }: AppListProps) {
  const { listGrantsForApp } = useCapabilities()

  return (
    <div className="p-4">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-lg font-semibold">Apps</h2>
        <button
          onClick={onRefresh}
          className="rounded-md bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary hover:bg-primary/20"
        >
          Refresh
        </button>
      </div>
      <div className="grid grid-cols-2 gap-3">
        {apps.map((app) => {
          const appId = Buffer.from(app.wasm_object?.object_id || new Uint8Array(0)).toString("hex")
          return (
            <div
              key={appId}
              className="rounded-lg border border-border bg-secondary/50 p-3"
            >
              <h3 className="text-sm font-medium">{app.name}</h3>
              <div className="mt-2 flex flex-wrap gap-1">
                {listGrantsForApp(appId).map((grant: capTypes.v0.capability.CapabilityGrant) => (
                  <span
                    key={Buffer.from(grant.grant_id || new Uint8Array(0)).toString("hex")}
                    className="rounded bg-primary/10 px-1.5 py-0.5 text-[9px]"
                  >
                    {Buffer.from(grant.grant_id || new Uint8Array(0)).toString("hex").slice(0, 8)}...
                  </span>
                ))}
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
