"use client"

import dynamic from "next/dynamic"

const TrustManagerPage = dynamic(() => import("@/app/trust-manager/page"), {
  ssr: false,
  loading: () => (
    <div className="flex h-full w-full items-center justify-center bg-slate-950 text-sm text-slate-500">
      Loading Trust Manager...
    </div>
  ),
})

export function TrustManagerApp() {
  return (
    <div className="h-full w-full overflow-auto bg-slate-950 text-slate-200">
      <TrustManagerPage />
    </div>
  )
}
