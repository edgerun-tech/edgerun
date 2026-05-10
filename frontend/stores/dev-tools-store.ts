import { atom } from "nanostores"
import type { BrowserCdpRelayState } from "@/platform/dev/browser-cdp-relay"

export type DesktopRelayDestination = "frontend" | "backend" | "chatgpt"
export type CdpToolAction = "api" | "status" | "endpoint" | "targets" | "eval" | "focus" | "text" | "ws-eval" | "frontend" | "chatgpt"
export type Health = "checking" | "ready" | "blocked" | "offline"

export const initialRelayState: BrowserCdpRelayState = {
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

export interface DevHealthState {
  backendHealth: Health
  backendError: string
  cdpHealth: Health
  cdpError: string
  destination: DesktopRelayDestination | null
}

export interface CdpToolsState {
  action: CdpToolAction
  arg: string
  result: string
  resultUpdatedAt: number | null
  open: boolean
  running: boolean
}

export const relayStateStore = atom<BrowserCdpRelayState>(initialRelayState)

export const devHealthStore = atom<DevHealthState>({
  backendHealth: "checking",
  backendError: "",
  cdpHealth: "checking",
  cdpError: "",
  destination: null,
})

export const cdpToolsStore = atom<CdpToolsState>({
  action: "api",
  arg: "",
  result: "Select a tool and run it.",
  resultUpdatedAt: null,
  open: false,
  running: false,
})

export const chatSessionInputStore = atom(initialRelayState.chatSession.label || initialRelayState.chatSession.query)
export const usageGuideOpenStore = atom(false)

export function setRelayState(value: BrowserCdpRelayState) {
  relayStateStore.set(value)
}

export function patchDevHealth(patch: Partial<DevHealthState>) {
  devHealthStore.set({ ...devHealthStore.get(), ...patch })
}

export function patchCdpTools(patch: Partial<CdpToolsState>) {
  cdpToolsStore.set({ ...cdpToolsStore.get(), ...patch })
}

export function setCdpToolOutput(value: unknown) {
  patchCdpTools({
    result: typeof value === "string" ? value : JSON.stringify(value, null, 2),
    resultUpdatedAt: Date.now(),
  })
}

export function setChatSessionInput(value: string) {
  chatSessionInputStore.set(value)
}

export function setUsageGuideOpen(open: boolean) {
  usageGuideOpenStore.set(open)
}

export function toggleUsageGuideOpen() {
  usageGuideOpenStore.set(!usageGuideOpenStore.get())
}
