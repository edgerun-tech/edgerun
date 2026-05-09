"use client"

import { useCallback, useEffect, useRef } from "react"
import { useStore } from "@nanostores/react"
import { appSurfaceOrderStore, appSurfacesStore, closeAppSurface, focusedAppSurfaceStore, focusAppSurface } from "@/stores/desktop-store"
import { launchAppById } from "@/stores/app-launcher"
import { executeUiAction, executeUiCommand, focusDockInput, type UiAction } from "@/stores/ui-command-center"
import { useAuth } from "@/hooks/use-auth"

type BridgeCommand = {
  id: string
  type: string
  payload?: unknown
  createdAt: string
}

type BridgePayload = {
  command?: string
  action?: UiAction
  appId?: string
  surfaceId?: string
  selector?: string
  value?: string
  submit?: boolean
  prefix?: "/" | "?" | "~"
}

function payloadOf(command: BridgeCommand): BridgePayload {
  return command.payload && typeof command.payload === "object" ? command.payload as BridgePayload : {}
}

function setNativeValue(element: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const prototype = element instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype
  const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set
  setter?.call(element, value)
  element.dispatchEvent(new Event("input", { bubbles: true }))
  element.dispatchEvent(new Event("change", { bubbles: true }))
}

async function consumeCommand(commandId: string) {
  await fetch("/api/agent-ui-bridge", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    cache: "no-store",
    body: JSON.stringify({ consumeCommandId: commandId }),
  }).catch(() => null)
}

export function AgentUiBridge() {
  const auth = useAuth()
  const appSurfaces = useStore(appSurfacesStore)
  const appSurfaceOrder = useStore(appSurfaceOrderStore)
  const focusedSurfaceId = useStore(focusedAppSurfaceStore)
  const seenCommandsRef = useRef(new Set<string>())

  const publishState = useCallback(() => {
    const activeElement = document.activeElement instanceof HTMLElement
      ? {
        tagName: document.activeElement.tagName.toLowerCase(),
        id: document.activeElement.id || null,
        role: document.activeElement.getAttribute("role"),
        ariaLabel: document.activeElement.getAttribute("aria-label"),
        title: document.activeElement.getAttribute("title"),
      }
      : null

    void fetch("/api/agent-ui-bridge", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      cache: "no-store",
      body: JSON.stringify({
        state: {
          updatedAt: new Date().toISOString(),
          route: window.location.pathname,
          visibility: document.visibilityState,
          viewport: { width: window.innerWidth, height: window.innerHeight },
          auth: {
            state: auth.authState,
            username: auth.username,
            activeProfileId: auth.activeProfileId,
          },
          dockPage: document.querySelector("[data-dock-page]")?.getAttribute("data-dock-page") ?? null,
          activeElement,
          focusedSurfaceId,
          surfaceOrder: appSurfaceOrder,
          surfaces: appSurfaces.map((surface) => ({
            id: surface.id,
            appId: surface.appId,
            title: surface.title,
            kind: surface.kind,
            variant: surface.variant,
            focused: surface.id === focusedSurfaceId,
          })),
        },
      }),
    }).catch(() => null)
  }, [appSurfaceOrder, appSurfaces, auth.activeProfileId, auth.authState, auth.username, focusedSurfaceId])

  const executeBridgeCommand = useCallback(async (command: BridgeCommand) => {
    const payload = payloadOf(command)

    switch (command.type) {
      case "ui-command":
        await executeUiCommand(payload.command || "", "system")
        break
      case "ui-action":
        if (payload.action) executeUiAction(payload.action, "system")
        break
      case "focus-dock":
        focusDockInput(payload.prefix || "~", payload.value)
        break
      case "launch-app":
        if (payload.appId) launchAppById(payload.appId)
        break
      case "focus-surface":
        if (payload.surfaceId) focusAppSurface(payload.surfaceId)
        break
      case "close-surface":
        if (payload.surfaceId) closeAppSurface(payload.surfaceId)
        break
      case "dom-click": {
        const target = payload.selector ? document.querySelector<HTMLElement>(payload.selector) : null
        target?.click()
        break
      }
      case "dom-input": {
        const target = payload.selector ? document.querySelector<HTMLInputElement | HTMLTextAreaElement>(payload.selector) : null
        if (target && ("value" in target)) {
          target.focus()
          setNativeValue(target, payload.value || "")
          if (payload.submit) target.closest("form")?.requestSubmit()
        }
        break
      }
    }

    await consumeCommand(command.id)
    publishState()
  }, [publishState])

  useEffect(() => {
    publishState()
  }, [publishState])

  useEffect(() => {
    const interval = window.setInterval(publishState, 1_000)
    window.addEventListener("focus", publishState)
    window.addEventListener("resize", publishState)
    return () => {
      window.clearInterval(interval)
      window.removeEventListener("focus", publishState)
      window.removeEventListener("resize", publishState)
    }
  }, [publishState])

  useEffect(() => {
    let cancelled = false

    async function pollCommands() {
      try {
        const res = await fetch("/api/agent-ui-bridge", { cache: "no-store" })
        const data = await res.json().catch(() => ({})) as { commands?: BridgeCommand[] }
        if (cancelled) return
        for (const command of data.commands || []) {
          if (seenCommandsRef.current.has(command.id)) continue
          seenCommandsRef.current.add(command.id)
          await executeBridgeCommand(command)
        }
      } catch {
        return
      }
    }

    void pollCommands()
    const interval = window.setInterval(() => void pollCommands(), 750)
    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [executeBridgeCommand])

  return null
}
