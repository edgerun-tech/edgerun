"use client"

import { useEffect, useState } from "react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Kbd, KbdGroup } from "@/components/ui/kbd"

const relayExamples = [
  {
    label: "Frontend page relay",
    code: `window.edgerunCdpRelay.relay("hello frontend", "frontend")`,
  },
  {
    label: "Listen in this page",
    code: `window.edgerunCdpRelay.subscribe((message) => console.log(message))`,
  },
  {
    label: "Backend bridge",
    code: `window.edgerunCdpRelay.setBackendBridge(async (message) => fetch("/api/codex", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ prompt: message, resume: true, stream: false }),
}))`,
  },
  {
    label: "CDP destination (browser route)",
    code: `window.edgerunCdpRelay.setDestination("chatgpt")`,
  },
  {
    label: "Select chat session",
    code: `window.edgerunCdpRelay.setChatSession("chatgpt.com")`,
  },
  {
    label: "Direct CDP target",
    code: `window.edgerunCdpRelay.sendToChatGpt("hello", { webSocketUrl })`,
  },
]

export function UsageGuideModal() {
  const [open, setOpen] = useState(false)

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return
      if (event.key.toLowerCase() !== "h") return
      event.preventDefault()
      event.stopPropagation()
      setOpen((current) => !current)
    }

    window.addEventListener("keydown", onKeyDown, { capture: true })
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true })
  }, [])

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="max-h-[86vh] overflow-hidden p-0 sm:max-w-2xl">
        <DialogHeader className="border-b border-border px-5 py-4">
          <div className="flex items-center justify-between gap-4 pr-8">
            <div>
              <DialogTitle className="font-mono text-base">Usage Guide</DialogTitle>
              <DialogDescription className="mt-1">
                Frontend relay, backend bridge selection, and direct browser automation.
              </DialogDescription>
            </div>
            <KbdGroup className="shrink-0">
              <Kbd>Win</Kbd>
              <Kbd>H</Kbd>
            </KbdGroup>
          </div>
        </DialogHeader>

        <div className="min-h-0 space-y-5 overflow-auto px-5 py-4 text-sm">
          <section className="space-y-2">
            <h3 className="font-mono text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Choose The Path
            </h3>
            <div className="grid gap-2 sm:grid-cols-3">
              <div className="rounded-md border border-border bg-secondary/30 p-3">
                <div className="font-medium">Frontend</div>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">
                  Same-origin page messages through browser events and BroadcastChannel.
                </p>
              </div>
              <div className="rounded-md border border-border bg-secondary/30 p-3">
                <div className="font-medium">Backend</div>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">
                  Register your existing bridge with <span className="font-mono">setBackendBridge</span>.
                </p>
              </div>
              <div className="rounded-md border border-border bg-secondary/30 p-3">
                <div className="font-medium">CDP</div>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">
                  Use a DevTools WebSocket URL when the browser allows the page origin.
                </p>
              </div>
            </div>
          </section>

          <section className="space-y-2">
            <h3 className="font-mono text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Console API
            </h3>
            <div className="space-y-2">
              {relayExamples.map((example) => (
                <div key={example.label} className="rounded-md border border-border bg-black/30 p-3">
                  <div className="mb-2 text-xs font-medium text-muted-foreground">{example.label}</div>
                  <code className="block whitespace-pre-wrap break-words font-mono text-xs leading-5 text-foreground">
                    {example.code}
                  </code>
                </div>
              ))}
            </div>
          </section>

          <section className="rounded-md border border-border bg-secondary/20 p-3 text-xs leading-5 text-muted-foreground">
            Direct frontend CDP needs Brave launched with remote debugging and the allowed origins flag for this app:
            <code className="mt-2 block break-words rounded bg-background px-2 py-1 font-mono text-foreground">
              --remote-debugging-port=9222 --remote-allow-origins=http://localhost:3000,http://127.0.0.1:3000
            </code>
          </section>
        </div>
      </DialogContent>
    </Dialog>
  )
}
