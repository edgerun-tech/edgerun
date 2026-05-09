"use client"

import { addLog, type LogEntry } from "@/stores/desktop-store"
import { launchAppById } from "@/stores/app-launcher"
import { getCatalogApp } from "@/platform/registries/app-catalog-registry"
import { listBuiltinApps } from "@/platform/registries/builtin-app-registry"
import { isRemovedAppId } from "@/platform/registries/app-id-policy"
import { sessionTracker } from "@/platform/auth/session-tracker"
import { nodeStore } from "@/platform/state/node-store"
import { clearAssistantMessages } from "@/stores/assistant-store"
import { normalizeAppId } from "@/stores/installed-apps-store"
import {
  addProjectChecklistItem,
  findProjectChecklistItem,
  formatProjectChecklist,
  removeProjectChecklistItem,
  setProjectChecklistItemDone,
} from "@/stores/project-checklist-store"

export type UiCommandSource = "user" | "assistant" | "dock" | "system"

export type UiAction = {
  type: string
  payload?: unknown
}

type UiCommandHandler = (context: {
  command: string
  source: UiCommandSource
}) => string | void | Promise<string | void>

type UiCommandDescriptor = {
  command: string
  aliases?: string[]
  description: string
  examples?: string[]
}

const commandHandlers = new Map<string, UiCommandHandler>()

const UI_COMMANDS: UiCommandDescriptor[] = [
  {
    command: "/open <app>",
    aliases: ["<app name>"],
    description: "Open an installed or published app.",
    examples: ["/open settings", "/settings"],
  },
  {
    command: "/node identity",
    aliases: ["/node", "node identity"],
    description: "Show the current browser node identity, health, sync state, and stream head.",
    examples: ["/node identity"],
  },
  {
    command: "/checklist",
    aliases: ["checklist"],
    description: "Show the project checklist.",
    examples: ["/checklist"],
  },
  {
    command: "/add checklist <item>",
    description: "Add a project checklist item.",
    examples: ["/add checklist Wire command help"],
  },
  {
    command: "/check <item>",
    aliases: ["/done <item>"],
    description: "Mark a matching checklist item done.",
    examples: ["/check demo cleanup"],
  },
  {
    command: "/uncheck <item>",
    description: "Reopen a matching checklist item.",
    examples: ["/uncheck demo cleanup"],
  },
  {
    command: "/remove checklist <item>",
    description: "Remove a matching checklist item.",
    examples: ["/remove checklist stale task"],
  },
  {
    command: "/clear chat",
    aliases: ["/clear assistant"],
    description: "Clear assistant history in the dock and assistant panel.",
    examples: ["/clear chat"],
  },
  {
    command: "/lock",
    description: "Lock the local profile session.",
    examples: ["/lock"],
  },
  {
    command: "~<message>",
    description: "Send a message to Codex from the prompt.",
    examples: ["~summarize current UI state"],
  },
  {
    command: "?<query>",
    aliases: ["?commands", "/help"],
    description: "Search available prompt commands.",
    examples: ["?commands", "?node"],
  },
]

function formatCommandHelp(query = "") {
  const term = query.trim().toLowerCase()
  const commands = UI_COMMANDS.filter((item) => {
    if (!term || term === "commands" || term === "help") return true
    return [
      item.command,
      item.description,
      ...(item.aliases ?? []),
      ...(item.examples ?? []),
    ].some((value) => value.toLowerCase().includes(term))
  })

  if (commands.length === 0) {
    return [
      `No prompt commands matched "${query}".`,
      "",
      "Try `?commands`, `?node`, `?checklist`, or `?open`.",
    ].join("\n")
  }

  return [
    term && term !== "commands" && term !== "help" ? `Command help for "${query}"` : "Prompt commands",
    "",
    ...commands.flatMap((item) => {
      const lines = [`- \`${item.command}\` — ${item.description}`]
      if (item.aliases?.length) lines.push(`  aliases: ${item.aliases.map((alias) => `\`${alias}\``).join(", ")}`)
      if (item.examples?.length) lines.push(`  examples: ${item.examples.map((example) => `\`${example}\``).join(", ")}`)
      return lines
    }),
  ].join("\n")
}

function sourceLabel(source: UiCommandSource) {
  if (source === "assistant") return "assistant"
  if (source === "dock") return "dock"
  if (source === "system") return "system"
  return "user"
}

function appendTerminalLog(type: LogEntry["type"], message: string) {
  addLog(type, message)
}

function resolveCommandAppId(value: string): string | null {
  const normalized = normalizeAppId(value)
  if (isRemovedAppId(normalized)) return null
  if (listBuiltinApps().some((app) => app.appId === normalized) || getCatalogApp(normalized)) return normalized

  const term = value.trim().toLowerCase()
  const builtin = listBuiltinApps().find((app) => (
    !isRemovedAppId(app.appId) && (
      app.name.toLowerCase() === term ||
      app.name.toLowerCase().startsWith(term) ||
      app.appId.toLowerCase() === term
    )
  ))
  return builtin?.appId ?? null
}

function shortId(value?: string | null) {
  if (!value) return "unknown"
  return value.length > 24 ? `${value.slice(0, 14)}...${value.slice(-8)}` : value
}

function formatNodeIdentity() {
  const nodeState = nodeStore.get()
  const session = sessionTracker.get()
  const currentNode = nodeState.currentNode
  const registration = session.nodeRegistration

  if (!currentNode && !registration && nodeState.nodes.length === 0) {
    return [
      "Node identity unavailable.",
      "",
      nodeState.error ? `Error: ${nodeState.error}` : "No browser node is connected yet.",
      "Open Identity to create or unlock a local profile.",
    ].join("\n")
  }

  const lines = ["Node identity"]

  if (currentNode) {
    lines.push(`- Node ID: ${shortId(currentNode.nodeId)}`)
    lines.push(`- Identity: ${shortId(currentNode.identity)}`)
    lines.push(`- Health: ${currentNode.health}`)
    lines.push(`- Runtime: ${currentNode.runtimeVersion || "unknown"}`)
    lines.push(`- Sync: ${currentNode.syncStatus}`)
    if (currentNode.streamHead) {
      lines.push(`- Stream: ${shortId(currentNode.streamHead.streamId)} #${currentNode.streamHead.lastSeq}`)
    }
    lines.push(`- Last refresh: ${currentNode.lastRefresh || "unknown"}`)
  } else if (registration) {
    lines.push(`- Registered node: ${shortId(registration.nodeId)}`)
    lines.push(`- Target: ${registration.nodeTarget}`)
    lines.push(`- User: ${registration.username || session.username || "unknown"}`)
    lines.push("- Health: offline")
  }

  if (nodeState.nodes.length > 0) {
    lines.push("")
    lines.push(`Reachable nodes: ${nodeState.nodes.length}`)
    for (const node of nodeState.nodes.slice(0, 5)) {
      lines.push(`- ${shortId(node.nodeId)} · ${node.relationship} · ${node.health}`)
    }
  }

  return lines.join("\n")
}

export function registerUiCommandHandler(name: string, handler: UiCommandHandler) {
  const key = name.trim().toLowerCase()
  commandHandlers.set(key, handler)
  return () => {
    if (commandHandlers.get(key) === handler) commandHandlers.delete(key)
  }
}

export function focusDockInput(prefix: "/" | "?" | "~" = "~", value?: string) {
  if (typeof window === "undefined") return
  window.dispatchEvent(new CustomEvent("edgerun:dock-command-input", {
    detail: { prefix, value },
  }))
}

export function focusAssistantInput(message?: string, submit = false) {
  if (typeof window === "undefined") return
  window.dispatchEvent(new CustomEvent("edgerun:assistant-input", {
    detail: { message, submit },
  }))
}

export function getUiCommandHelp(query = "") {
  return formatCommandHelp(query)
}

export function executeUiAction(action: UiAction, source: UiCommandSource = "assistant") {
  switch (action.type) {
    case "open-app": {
      const appId = typeof action.payload === "string" ? action.payload : ""
      const surface = appId ? launchAppById(normalizeAppId(appId)) : null
      return surface ? `Opened ${surface.title}` : `Could not open app: ${appId || "unknown"}`
    }
    case "show-stats": {
      const surface = launchAppById("compute-node")
      return surface ? `Opened ${surface.title}` : "Compute app is not available"
    }
    case "open-files": {
      const surface = launchAppById("file-browser")
      return surface ? `Opened ${surface.title}` : "File Manager is not available"
    }
    case "focus-dock":
      focusDockInput("~")
      return "Focused dock input"
    default:
      appendTerminalLog("warning", `Unknown UI action from ${sourceLabel(source)}: ${action.type}`)
      return `Unknown UI action: ${action.type}`
  }
}

export function executeUiActions(actions: UiAction[], source: UiCommandSource = "assistant") {
  return actions.map((action) => executeUiAction(action, source))
}

export async function executeUiCommand(rawCommand: string, source: UiCommandSource = "user") {
  const command = rawCommand.trim()
  if (!command) return "No command provided"

  if (command.startsWith("~")) {
    const message = command.slice(1).trim()
    focusAssistantInput(message, true)
    appendTerminalLog("info", `${sourceLabel(source)}> assistant ${message}`)
    return message ? "Sent to assistant" : "Focused assistant"
  }

  const lower = command.toLowerCase()

  if (command.startsWith("?") || lower === "/help" || lower.startsWith("/help ")) {
    const query = command.startsWith("?")
      ? command.slice(1).trim()
      : command.replace(/^\/help\s*/i, "").trim()
    return formatCommandHelp(query)
  }

  const handlerKey = lower.startsWith("/") ? lower.split(/\s+/)[0] : lower.split(/\s+/)[0]
  const handler = commandHandlers.get(handlerKey)
  if (handler) {
    const result = await handler({ command, source })
    return typeof result === "string" && result.trim() ? result.trim() : `Handled ${command}`
  }

  if (lower.startsWith("/open ")) {
    const targetName = command.slice("/open ".length).trim()
    const appId = resolveCommandAppId(targetName)
    if (appId) {
      const surface = launchAppById(appId)
      if (surface) return `Opened ${surface.title}`
    }
    appendTerminalLog("warning", `Could not open app: ${targetName}`)
    return `Could not open app: ${targetName}`
  }

  if (
    lower === "/node" ||
    lower === "node" ||
    lower === "/node identity" ||
    lower === "node identity" ||
    lower === "/show node identity" ||
    lower === "show node identity"
  ) {
    return formatNodeIdentity()
  }

  if (lower === "/clear chat" || lower === "clear chat" || lower === "/clear assistant" || lower === "clear assistant") {
    clearAssistantMessages()
    return "Cleared assistant history"
  }

  if (lower === "/checklist" || lower === "checklist") {
    return formatProjectChecklist()
  }

  if (lower.startsWith("/add checklist ") || lower.startsWith("add checklist ")) {
    const label = command.replace(/^\/?add checklist\s+/i, "").trim()
    addProjectChecklistItem(label)
    return label ? `Added checklist item: ${label}` : "No checklist item provided"
  }

  if (lower.startsWith("/check ") || lower.startsWith("check ") || lower.startsWith("/done ") || lower.startsWith("done ")) {
    const query = command.replace(/^\/?(check|done)\s+/i, "").trim()
    const item = findProjectChecklistItem(query)
    if (!item) return `No checklist item matched: ${query}`
    setProjectChecklistItemDone(item.id, true)
    return `Marked done: ${item.label}`
  }

  if (lower.startsWith("/uncheck ") || lower.startsWith("uncheck ")) {
    const query = command.replace(/^\/?uncheck\s+/i, "").trim()
    const item = findProjectChecklistItem(query)
    if (!item) return `No checklist item matched: ${query}`
    setProjectChecklistItemDone(item.id, false)
    return `Marked open: ${item.label}`
  }

  if (lower.startsWith("/remove checklist ") || lower.startsWith("remove checklist ")) {
    const query = command.replace(/^\/?remove checklist\s+/i, "").trim()
    const item = findProjectChecklistItem(query)
    if (!item) return `No checklist item matched: ${query}`
    removeProjectChecklistItem(item.id)
    return `Removed checklist item: ${item.label}`
  }

  if (lower === "/terminal" || lower === "terminal") {
    return "Terminal app is no longer available"
  }

  if (lower === "/settings" || lower === "settings") {
    return executeUiAction({ type: "open-app", payload: "settings" }, source)
  }

  const launchTarget = command.startsWith("/") ? command.slice(1).trim() : command
  const launchAppId = resolveCommandAppId(launchTarget)
  if (launchAppId) {
    const surface = launchAppById(launchAppId)
    if (surface) return `Opened ${surface.title}`
  }

  appendTerminalLog("system", `${sourceLabel(source)}> ${command}`)
  return `Logged ${command}`
}
