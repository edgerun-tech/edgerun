"use client"

import { useEffect, useRef } from "react"
import type { AppDefinition } from "@/platform/types/app-definition"
import { sendEdgerunNodeProtocol } from "@/platform/runtime/edgerun-node"

export function BrowserIframeAppHost({ app }: { app: AppDefinition }) {
  const iframeRef = useRef<HTMLIFrameElement | null>(null)
  const launchUrl = typeof app.displayMetadata?.launchUrl === "string" ? app.displayMetadata.launchUrl : undefined

  useEffect(() => {
    function postToApp(message: unknown) {
      iframeRef.current?.contentWindow?.postMessage(message, window.location.origin)
    }

    function handleMessage(event: MessageEvent) {
      if (event.source !== iframeRef.current?.contentWindow) return
      if (event.origin !== window.location.origin) return
      const data = event.data
      if (!data || typeof data !== "object") return

      if (data.type === "edgerun:node.request") {
        const id = typeof data.id === "string" ? data.id : ""
        const method = typeof data.method === "string" ? data.method : "GET"
        const path = typeof data.path === "string" ? data.path : ""
        if (!id || !path.startsWith("/protocol/")) {
          postToApp({ type: "edgerun:node.response", id, ok: false, error: "invalid node protocol request" })
          return
        }
        void sendEdgerunNodeProtocol({ method, path }).then((response) => {
          postToApp({
            type: "edgerun:node.response",
            id,
            ok: response.status >= 200 && response.status < 300,
            status: response.status,
            body: new TextDecoder().decode(response.body),
          })
        }).catch((error) => {
          postToApp({
            type: "edgerun:node.response",
            id,
            ok: false,
            error: error instanceof Error ? error.message : String(error),
          })
        })
      }
    }

    window.addEventListener("message", handleMessage)
    return () => window.removeEventListener("message", handleMessage)
  }, [])

  if (!launchUrl) {
    return (
      <div className="flex h-full items-center justify-center bg-background p-4 text-sm text-muted-foreground">
        Installed app has no browser launch URL.
      </div>
    )
  }

  return (
    <iframe
      ref={iframeRef}
      title={app.name}
      src={launchUrl}
      onLoad={() => {
        iframeRef.current?.contentWindow?.postMessage({
          type: "edgerun:host.ready",
          app: {
            appId: app.appId,
            name: app.name,
            version: app.displayMetadata?.version,
            runtimeAppId: app.displayMetadata?.runtimeAppId,
            releaseId: app.displayMetadata?.releaseId,
            developerId: app.displayMetadata?.developerId,
            manifestSha256: app.signature?.manifestHash,
            packageHash: app.signature?.packageHash,
            installedAt: app.displayMetadata?.installedAt,
            verifiedAssets: app.displayMetadata?.verifiedAssets,
          },
        }, window.location.origin)
      }}
      className="h-full w-full border-0 bg-background"
      sandbox="allow-scripts allow-forms allow-same-origin"
      referrerPolicy="no-referrer"
    />
  )
}
