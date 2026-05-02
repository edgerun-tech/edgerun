/**
 * AppList component - uses platform hooks only.
 * No direct protocol access, no manual fetching.
 */

"use client"

import { useApps, useCapabilities } from "@/platform/ui/useApps"
import { edgerun as stream } from "@/gen/edgerun/v0/stream"

interface AppListProps {
  apps: stream.AppPackage[]
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
                {listGrantsForApp(appId).map((grant: never) => (
                  <span
                    key={(grant as { grant_id: Uint8Array }).grant_id?.toString()}
                    className="rounded bg-primary/10 px-1.5 py-0.5 text-[9px]"
                  >
                    {Buffer.from((grant as { grant_id: Uint8Array }).grant_id || new Uint8Array(0)).toString("hex")}
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
