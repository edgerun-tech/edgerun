import { generateText } from "ai"
import { createOpenAI } from "@ai-sdk/openai"
import { systemStatsStore, windowsStore } from "@/stores/desktop-store"
import { appStore } from "@/platform/state/app-store"
import { capabilityStore } from "@/platform/state/capability-store"
import { nodeStore } from "@/platform/state/node-store"
import { authStore } from "@/stores/auth-store"

const opencode = createOpenAI({
  baseURL: "https://opencode.ai/zen/v1",
  apiKey: process.env.OPENCODE_API_KEY || "",
})

const FREE_MODEL = "minimax-m2.5-free"

export const maxDuration = 60

const tools = {
  getSystemStats: {
    description: "Get current system statistics including node count, CPU, memory, sessions, and connection status",
    parameters: {
      type: "object" as const,
      properties: {},
    },
    execute: async () => {
      const stats = systemStatsStore.get()
      return {
        nodeCount: stats.nodeCount,
        activeSessions: stats.activeSessions,
        ramUsed: stats.ramUsage.used,
        ramTotal: stats.ramUsage.total,
        ramPercent: Math.round((stats.ramUsage.used / stats.ramUsage.total) * 100),
        isConnected: stats.isConnected,
      }
    },
  },
  getRunningApps: {
    description: "List all currently running applications",
    parameters: {
      type: "object" as const,
      properties: {},
    },
    execute: async () => {
      const windows = windowsStore.get()
      const running = windows
        .filter(w => w.appId !== "app-store" && w.appId !== "app-studio")
        .map(w => ({ id: w.appId, title: w.title }))
      return { apps: running, count: running.length }
    },
  },
  getInstalledApps: {
    description: "List all installed applications on the system",
    parameters: {
      type: "object" as const,
      properties: {},
    },
    execute: async () => {
      const apps = appStore.get()
      const appList = Array.from(apps.apps.entries()).map(([id, a]) => ({
        id,
        name: a.name || id,
        version: a.version || "unknown",
      }))
      return { apps: appList, count: appList.length }
    },
  },
  getCapabilities: {
    description: "List all available capabilities and their status",
    parameters: {
      type: "object" as const,
      properties: {},
    },
    execute: async () => {
      const caps = capabilityStore.get()
      const capList = Array.from(caps.descriptors.values()).map(c => ({
        id: c.capability_id ? Buffer.from(c.capability_id).toString("hex") : "unknown",
        role: c.role || "unspecified",
        providerName: c.provider_name || "unknown",
      }))
      return { capabilities: capList, count: capList.length }
    },
  },
  getNodeStatus: {
    description: "Get the local node registration and health status",
    parameters: {
      type: "object" as const,
      properties: {},
    },
    execute: async () => {
      const node = nodeStore.get()
      return {
        nodeId: node.currentNode?.nodeId || null,
        health: node.currentNode?.health || "unknown",
        runtimeVersion: node.currentNode?.runtimeVersion || "unknown",
        syncStatus: node.currentNode?.syncStatus || "unknown",
        isConnected: node.isConnected,
      }
    },
  },
  getUserInfo: {
    description: "Get current user authentication status and info",
    parameters: {
      type: "object" as const,
      properties: {},
    },
    execute: async () => {
      const auth = authStore.get()
      return {
        username: auth.username || null,
        authState: auth.authState,
        hasIdentity: auth.authState === "authenticated",
      }
    },
  },
  runTerminalCommand: {
    description: "Execute a terminal command on the system",
    parameters: {
      type: "object" as const,
      properties: {
        command: { type: "string" as const, description: "The command to execute" },
      },
      required: ["command"],
    },
    execute: async ({ command }: { command: string }) => {
      return { output: `Simulated: ${command}`, exitCode: 0 }
    },
  },
  generateSystemGraph: {
    description: "Generate resource usage graph data for visualization",
    parameters: {
      type: "object" as const,
      properties: {
        type: {
          type: "string" as const,
          enum: ["cpu", "memory", "network", "sessions"],
          description: "Type of graph data to generate",
        },
      },
      required: ["type"],
    },
    execute: async ({ type }: { type: "cpu" | "memory" | "network" | "sessions" }) => {
      const stats = systemStatsStore.get()
      const now = Date.now()

      const generateHistory = (base: number, variance: number) => {
        return Array.from({ length: 12 }, (_, i) => ({
          time: new Date(now - (11 - i) * 60000).toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit", hour12: false }),
          value: Math.max(0, Math.min(100, base + (Math.random() - 0.5) * variance)),
        }))
      }

      switch (type) {
        case "cpu": {
          const cpuBase = 35
          return {
            type: "cpu",
            data: generateHistory(cpuBase, 20),
            current: cpuBase,
            unit: "%"
          }
        }
        case "memory":
          return {
            type: "memory",
            data: generateHistory((stats.ramUsage.used / stats.ramUsage.total) * 100, 15),
            current: Math.round((stats.ramUsage.used / stats.ramUsage.total) * 100),
            used: stats.ramUsage.used,
            total: stats.ramUsage.total,
            unit: "%"
          }
        case "network":
          return {
            type: "network",
            data: generateHistory(50, 30),
            current: 50,
            unit: "Mbps"
          }
        case "sessions":
          return {
            type: "sessions",
            data: generateHistory(stats.activeSessions, 2),
            current: stats.activeSessions,
            unit: "sessions"
          }
        default:
          return { error: "Unknown graph type" }
      }
    },
  },
}

export async function handleAIChat(messages: any[], systemContext?: string) {
  const baseSystemPrompt = `You are an AI assistant for the EdgeRun platform. You can query system stats, list apps, check capabilities, and more.`

  const systemPrompt = systemContext
    ? `${baseSystemPrompt}\n\n${systemContext}`
    : baseSystemPrompt

  try {
    const result = await generateText({
      model: opencode(FREE_MODEL) as any,
      messages: [
        { role: "system", content: systemPrompt },
        ...messages,
      ],
      tools: tools as any,
    })

    const toolCalls = result.toolCalls || []
    const toolResults = result.toolResults || []

    return Response.json({
      text: result.text,
      finishReason: result.finishReason,
      toolCalls: toolCalls.map((tc: any, i: number) => ({
        name: tc.toolName,
        args: tc.input || {},
        result: toolResults[i]?.output,
      })),
    })
  } catch (error) {
    console.error("OpenCode API error:", error)

    const errorStr = String(error)
    if (errorStr.includes("401") || errorStr.includes("authentication")) {
      return Response.json({
        text: "Please configure OPENCODE_API_KEY in your environment. Get one at https://opencode.ai/auth",
        error: "Missing API key"
      }, { status: 401 })
    }

    return Response.json({
      text: "Sorry, the AI service is currently unavailable. Please try again later.",
      error: String(error)
    }, { status: 503 })
  }
}
