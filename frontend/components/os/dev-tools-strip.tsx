"use client"

import { useCallback, useEffect, useState } from "react"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { cn } from "@/lib/utils"
import { bootstrapBrowserCdpRelay, type BrowserCdpRelayState } from "@/platform/dev/browser-cdp-relay"

type DesktopRelayDestination = "frontend" | "backend" | "chatgpt"
type CdpToolAction = "api" | "status" | "endpoint" | "targets" | "eval" | "focus" | "text" | "ws-eval" | "frontend" | "chatgpt"
type Health = "checking" | "ready" | "blocked" | "offline"

const DEV_TOOLS_ENABLED = process.env.NODE_ENV !== "production"

const initialRelayState: BrowserCdpRelayState = {
  destination: "backend",
  frontend: "offline",
  backend: "offline",
  backendBridgeRegistered: false,
  endpoint: "http://127.0.0.1:9222",
  chatSession: {
    query: "chatgpt.com",
    label: "chatgpt.com",
  },
  updatedAtIso: "",
}

const toolPlaceholders: Record<CdpToolAction, string> = {
  api: "",
  status: "",
  endpoint: "9222 or http://127.0.0.1:9222",
  targets: "",
  eval: "target | expression",
  focus: "target | locator",
  text: "target",
  "ws-eval": "websocket url | expression",
  frontend: "message",
  chatgpt: "message",
}

function formatToolResult(value: unknown) {
  if (typeof value === "string") return value
  return JSON.stringify(value, null, 2)
}

function splitToolArg(value: string) {
  const [first = "", ...rest] = value.split("|")
  return {
    target: first.trim(),
    value: rest.join("|").trim(),
  }
}

function StatusDot({ status }: { status: Health }) {
  return (
    <span
      className={cn(
        "h-2 w-2 rounded-full",
        status === "ready" && "bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.7)]",
        status === "checking" && "bg-amber-300",
        status === "blocked" && "bg-sky-400",
        status === "offline" && "bg-zinc-600",
      )}
      aria-hidden="true"
    />
  )
}

function RelayStatusDot({ online }: { online: boolean }) {
  return (
    <span
      className={cn(
        "h-2 w-2 rounded-full",
        online ? "bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.7)]" : "bg-zinc-600"
      )}
      aria-hidden="true"
    />
  )
}

function useRelayState() {
  const [relayState, setRelayState] = useState<BrowserCdpRelayState>(initialRelayState)

  useEffect(() => {
    const relay = bootstrapBrowserCdpRelay()
    if (!relay) return
    return relay.subscribeStatus(setRelayState)
  }, [])

  return relayState
}

function useDevHealth() {
  const [backendHealth, setBackendHealth] = useState<Health>("checking")
  const [cdpHealth, setCdpHealth] = useState<Health>("checking")
  const [cdpError, setCdpError] = useState("")
  const [backendError, setBackendError] = useState("")
  const [destination, setDestination] = useState<DesktopRelayDestination | null>(null)

  useEffect(() => {
    const relay = bootstrapBrowserCdpRelay()
    if (!relay) return

    setDestination(relay.status().destination)
    return relay.subscribeStatus((state) => setDestination(state.destination))
  }, [])

  const refresh = useCallback(async () => {
    const relay = bootstrapBrowserCdpRelay()
    const activeDestination = destination || relay?.status().destination || null

    setCdpHealth("checking")
    setCdpError("")
    if (!relay) {
      setCdpError("CDP relay unavailable.")
      setCdpHealth("offline")
    } else if (activeDestination !== "frontend") {
      setCdpHealth("blocked")
      const label = activeDestination ?? "unknown"
      setCdpError(`Direct CDP checks are disabled for destination "${label}".`)
    } else {
      try {
        await relay.targets()
        setCdpHealth("ready")
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error)
        setCdpError(message)
        setCdpHealth(/failed to fetch|cors|load failed/i.test(message) ? "blocked" : "offline")
      }
    }

    setBackendHealth("checking")
    setBackendError("")
    try {
      const response = await fetch("/api/codex", { cache: "no-store" })
      const data = await response.json().catch(() => null) as { ok?: boolean } | null
      setBackendHealth(response.ok && data?.ok === true ? "ready" : "offline")
      if (!response.ok || data?.ok !== true) setBackendError(`HTTP ${response.status}`)
    } catch (error) {
      setBackendError(error instanceof Error ? error.message : String(error))
      setBackendHealth("offline")
    }
  }, [destination])

  useEffect(() => {
    void refresh()
    const interval = window.setInterval(() => void refresh(), 10_000)
    return () => window.clearInterval(interval)
  }, [refresh])

  return { backendHealth, backendError, cdpHealth, cdpError, refresh }
}

function CdpToolsControl({ backendHealth, backendError, cdpHealth, cdpError }: { backendHealth: Health; backendError: string; cdpHealth: Health; cdpError: string }) {
  const [action, setAction] = useState<CdpToolAction>("api")
  const [arg, setArg] = useState("")
  const [result, setResult] = useState("Select a tool and run it.")
  const [resultUpdatedAt, setResultUpdatedAt] = useState<number | null>(null)
  const [open, setOpen] = useState(false)
  const [running, setRunning] = useState(false)

  const placeholder = toolPlaceholders[action]

  const setOutput = useCallback((value: unknown) => {
    setResult(formatToolResult(value))
    setResultUpdatedAt(Date.now())
  }, [])

  const runTool = useCallback(async () => {
    const relay = bootstrapBrowserCdpRelay()
    if (!relay) return

    setRunning(true)
    setOpen(true)
    try {
      if (action === "api") {
        const keys = Object.keys(relay).filter((key) => typeof (relay as unknown as Record<string, unknown>)[key] === "function")
        setOutput({
          endpoint: relay.endpoint,
          eventName: relay.eventName,
          destination: relay.destination,
          health: { cdp: cdpHealth, backend: backendHealth },
          errors: { cdp: cdpError || null, backend: backendError || null },
          methods: keys,
          examples: {
            endpoint: "9222",
            eval: "localhost:3000 | document.title",
            focus: "chatgpt.com | role=textbox",
            text: "localhost:3000",
            "ws-eval": "ws://127.0.0.1:9222/devtools/page/... | document.title",
          },
        })
        return
      }

      if (action === "status") {
        setOutput({
          relay: relay.status(),
          health: { cdp: cdpHealth, backend: backendHealth },
          errors: { cdp: cdpError || null, backend: backendError || null },
        })
        return
      }

      if (action === "endpoint") {
        setOutput({ endpoint: relay.setEndpoint(arg.trim() || "9222") })
        return
      }

      if (action === "targets") {
        const targets = await relay.targets()
        setOutput(targets.map((target) => ({
          id: target.id,
          type: target.type,
          title: target.title,
          url: target.url,
          webSocketDebuggerUrl: target.webSocketDebuggerUrl,
        })))
        return
      }

      if (action === "frontend") {
        setOutput(relay.sendToFrontend(arg.trim() || "hello frontend", { source: "cdp-tools" }))
        return
      }

      if (action === "chatgpt") {
        setOutput(await relay.relay(arg.trim() || "hello from edgerun", "chatgpt", { waitMs: 6000 }))
        return
      }

      const { target, value } = splitToolArg(arg)
      if (action === "ws-eval") {
        if (!target) throw new Error("ws-eval requires: websocket url | expression")
        const page = await relay.connectWebSocket(target)
        try {
          setOutput(await page.evaluate(value || "({ title: document.title, url: location.href })"))
        } finally {
          page.close()
        }
        return
      }

      const page = await relay.connect(target || undefined)
      try {
        if (action === "eval") {
          setOutput(await page.evaluate(value || "({ title: document.title, url: location.href })"))
        } else if (action === "focus") {
          setOutput(await page.focus(value || "role=textbox"))
        } else {
          setOutput(await page.evaluate("(document.body?.innerText || '').slice(0, 4000)"))
        }
      } finally {
        page.close()
      }
    } catch (error) {
      setOutput(error instanceof Error ? error.message : String(error))
    } finally {
      setRunning(false)
    }
  }, [action, arg, backendError, backendHealth, cdpError, cdpHealth, setOutput])

  const copyResult = useCallback(async () => {
    await navigator.clipboard.writeText(result)
  }, [result])

  return (
    <div data-cdp-tools-control className="flex items-center gap-2">
      <span className="flex items-center gap-1 font-mono text-[10px] uppercase text-muted-foreground" title={cdpError ? `CDP ${cdpHealth}: ${cdpError}` : `CDP ${cdpHealth}`}>
        <StatusDot status={cdpHealth} />
        CDP
      </span>
      <Select value={action} onValueChange={(value) => setAction(value as CdpToolAction)}>
        <SelectTrigger
          size="sm"
          className="h-6 w-[82px] border-0 bg-transparent px-0 py-0 font-mono text-[11px] text-muted-foreground shadow-none hover:text-foreground focus-visible:ring-0 focus-visible:ring-offset-0 dark:bg-transparent dark:hover:bg-transparent [&_svg]:h-3 [&_svg]:w-3"
          aria-label="CDP tool"
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent align="center" className="min-w-[120px]">
          <SelectItem value="api" className="font-mono text-xs">api</SelectItem>
          <SelectItem value="status" className="font-mono text-xs">status</SelectItem>
          <SelectItem value="endpoint" className="font-mono text-xs">endpoint</SelectItem>
          <SelectItem value="targets" className="font-mono text-xs">targets</SelectItem>
          <SelectItem value="text" className="font-mono text-xs">text</SelectItem>
          <SelectItem value="eval" className="font-mono text-xs">eval</SelectItem>
          <SelectItem value="focus" className="font-mono text-xs">focus</SelectItem>
          <SelectItem value="ws-eval" className="font-mono text-xs">ws-eval</SelectItem>
          <SelectItem value="frontend" className="font-mono text-xs">frontend</SelectItem>
          <SelectItem value="chatgpt" className="font-mono text-xs">chatgpt</SelectItem>
        </SelectContent>
      </Select>
      <input
        value={arg}
        onChange={(event) => setArg(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") void runTool()
        }}
        placeholder={placeholder}
        className="h-6 w-[176px] bg-transparent font-mono text-[11px] text-muted-foreground outline-none placeholder:text-muted-foreground/45 hover:text-foreground focus:text-foreground"
        aria-label="CDP tool argument"
      />
      <button
        type="button"
        onClick={() => void runTool()}
        disabled={running}
        className="h-6 font-mono text-[11px] text-muted-foreground hover:text-foreground disabled:opacity-45"
      >
        {running ? "..." : "run"}
      </button>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <button type="button" className="h-6 font-mono text-[11px] text-muted-foreground hover:text-foreground">
            out
          </button>
        </PopoverTrigger>
        <PopoverContent align="center" side="top" className="mb-2 w-[min(640px,calc(100vw-2rem))] rounded-md border-border bg-background/95 p-3 shadow-xl backdrop-blur">
          <div className="mb-2 flex items-center justify-between gap-3 font-mono text-[10px] text-muted-foreground">
            <span>{resultUpdatedAt ? new Date(resultUpdatedAt).toLocaleTimeString() : "no result yet"}</span>
            <button type="button" onClick={() => void copyResult()} className="hover:text-foreground">copy</button>
          </div>
          <pre className="max-h-[320px] overflow-auto whitespace-pre-wrap break-words font-mono text-[11px] leading-5 text-muted-foreground">
            {result}
          </pre>
        </PopoverContent>
      </Popover>
    </div>
  )
}

function RelayRoutingControl({ relayState, backendHealth }: { relayState: BrowserCdpRelayState; backendHealth: Health }) {
  const selectedDestination: DesktopRelayDestination = relayState.destination
  const backendOnline = relayState.backendBridgeRegistered && backendHealth === "ready"
  const [chatSessionInput, setChatSessionInput] = useState(relayState.chatSession.label || relayState.chatSession.query)

  useEffect(() => {
    if (selectedDestination === "chatgpt") {
      setChatSessionInput(relayState.chatSession.label || relayState.chatSession.query)
    }
  }, [relayState.chatSession.label, relayState.chatSession.query, selectedDestination])

  const updateDestination = useCallback((destination: DesktopRelayDestination) => {
    const relay = bootstrapBrowserCdpRelay()
    relay?.setDestination(destination)
  }, [])

  const setSession = useCallback(() => {
    const relay = bootstrapBrowserCdpRelay()
    const next = relay?.setChatSession(chatSessionInput.trim() || "chatgpt.com")
    if (next) setChatSessionInput(next)
  }, [chatSessionInput])

  return (
    <div data-relay-routing-control className="flex items-center gap-3">
      <Select value={selectedDestination} onValueChange={(value) => updateDestination(value as DesktopRelayDestination)}>
        <SelectTrigger
          size="sm"
          className="h-6 w-[86px] border-0 bg-transparent px-0 py-0 font-mono text-[11px] text-muted-foreground shadow-none hover:text-foreground focus-visible:ring-0 focus-visible:ring-offset-0 dark:bg-transparent dark:hover:bg-transparent [&_svg]:h-3 [&_svg]:w-3"
          aria-label="Relay route"
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent align="center" className="min-w-[96px]">
          <SelectItem value="frontend" className="font-mono text-xs">frontend</SelectItem>
          <SelectItem value="backend" className="font-mono text-xs">backend</SelectItem>
          <SelectItem value="chatgpt" className="font-mono text-xs">chatgpt</SelectItem>
        </SelectContent>
      </Select>
      {selectedDestination === "chatgpt" ? (
        <div className="flex items-center gap-2">
          <input
            value={chatSessionInput}
            onChange={(event) => setChatSessionInput(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") setSession()
            }}
            placeholder="chatgpt.com"
            className="h-6 w-40 bg-transparent font-mono text-[10px] text-muted-foreground outline-none placeholder:text-muted-foreground/45 hover:text-foreground focus:text-foreground"
            aria-label="ChatGPT session"
          />
          <button
            type="button"
            onClick={() => void setSession()}
            className="h-6 font-mono text-[10px] text-muted-foreground hover:text-foreground"
          >
            set
          </button>
        </div>
      ) : null}
      <div className="flex items-center gap-2 font-mono text-[10px] uppercase text-muted-foreground">
        <span className={cn("flex items-center gap-1", relayState.frontend === "ready" && "text-emerald-300")}>
          <RelayStatusDot online={relayState.frontend === "ready"} />
          FE
        </span>
        <span className={cn("flex items-center gap-1", backendOnline && "text-emerald-300")}>
          <RelayStatusDot online={backendOnline} />
          BE
        </span>
      </div>
    </div>
  )
}

export function DevToolsStrip() {
  const relayState = useRelayState()
  const { backendHealth, backendError, cdpHealth, cdpError } = useDevHealth()

  if (!DEV_TOOLS_ENABLED) return null

  return (
    <div className="z-50 flex max-w-[calc(100vw-2rem)] items-center gap-4 overflow-x-auto px-2">
      <CdpToolsControl backendHealth={backendHealth} backendError={backendError} cdpHealth={cdpHealth} cdpError={cdpError} />
      <RelayRoutingControl relayState={relayState} backendHealth={backendHealth} />
    </div>
  )
}
