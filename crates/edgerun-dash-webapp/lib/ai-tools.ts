import { generateText, tool } from "ai"
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

const getSystemStats = tool({
  description: "Get current system statistics including node count, CPU, memory, sessions, and connection status",
  parameters: {
    type: "object",
    properties: {},
  },
  execute: async () => {
    const stats = systemStatsStore.get()
    return {
      nodeCount: stats.nodeCount,
      activeSessions: stats.activeSessions,
      cpuUsage: stats.cpuUsage,
      ramUsed: stats.ramUsage.used,
      ramTotal: stats.ramUsage.total,
      ramPercent: Math.round((stats.ramUsage.used / stats.ramUsage.total) * 100),
      isConnected: stats.isConnected,
    }
  },
})

const getRunningApps = tool({
  description: "List all currently running applications",
  parameters: {
    type: "object",
    properties: {},
  },
  execute: async () => {
    const windows = windowsStore.get()
    const running = windows
      .filter(w => w.appId !== "app-store" && w.appId !== "app-studio")
      .map(w => ({ id: w.appId, title: w.title }))
    return { apps: running, count: running.length }
  },
})

const getInstalledApps = tool({
  description: "List all installed applications on the system",
  parameters: {
    type: "object",
    properties: {},
  },
  execute: async () => {
    const apps = appStore.get()
    const appList = Array.from(apps.apps.values()).map(a => ({
      id: a.appId,
      name: a.name || a.appId,
      version: a.version || "unknown",
    }))
    return { apps: appList, count: appList.length }
  },
})

const getCapabilities = tool({
  description: "List all available capabilities and their status",
  parameters: {
    type: "object",
    properties: {},
  },
  execute: async () => {
    const caps = capabilityStore.get()
    const capList = Array.from(caps.descriptors.values()).map(c => ({
      id: c.capabilityId,
      type: c.capabilityType,
      description: c.description || "",
      riskClass: c.riskClass || "unknown",
    }))
    return { capabilities: capList, count: capList.length }
  },
})

const getNodeStatus = tool({
  description: "Get the local node registration and health status",
  parameters: {
    type: "object",
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
})

const getUserInfo = tool({
  description: "Get current user authentication status and info",
  parameters: {
    type: "object",
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
})

const runTerminalCommand = tool({
  description: "Execute a terminal command and return the output",
  parameters: {
    type: "object",
    properties: {
      command: { type: "string", description: "The command to execute" },
    },
    required: ["command"],
  },
  execute: async ({ command }) => {
    try {
      const response = await fetch("/api/terminal", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ command }),
      })
      if (!response.ok) throw new Error(`HTTP ${response.status}`)
      const data = await response.json()
      return { success: true, output: data.output || data.result, error: null }
    } catch (error) {
      return { success: false, output: "", error: String(error) }
    }
  },
})

const generateSystemGraph = tool({
  description: "Generate resource usage graph data for visualization",
  parameters: {
    type: "object",
    properties: {
      type: { 
        type: "string", 
        enum: ["cpu", "memory", "network", "sessions"],
        description: "Type of graph data to generate" 
      },
    },
    required: ["type"],
  },
  execute: async ({ type }) => {
    const stats = systemStatsStore.get()
    const now = Date.now()
    
    const generateHistory = (base: number, variance: number) => {
      return Array.from({ length: 12 }, (_, i) => ({
        time: new Date(now - (11 - i) * 60000).toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit", hour12: false }),
        value: Math.max(0, Math.min(100, base + (Math.random() - 0.5) * variance)),
      }))
    }

    switch (type) {
      case "cpu":
        return { 
          type: "cpu", 
          data: generateHistory(stats.cpuUsage, 20),
          current: stats.cpuUsage,
          unit: "%"
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
          data: generateHistory(50, 40),
          current: Math.floor(Math.random() * 100),
          unit: "Mbps"
        }
      case "sessions":
        return {
          type: "sessions",
          data: generateHistory(stats.activeSessions * 10, 10),
          current: stats.activeSessions,
          unit: "sessions"
        }
      default:
        return { type: "unknown", data: [], current: 0, unit: "" }
    }
  },
})

export async function POST(req: Request) {
  const { messages, systemContext } = await req.json()

  const baseSystemPrompt = `You are an AI assistant for Edgerun, a distributed WASM runtime dashboard called "EdgeRun". 

You have access to tools to query and interact with the system. Use them proactively to provide real data.

Available tools:
- getSystemStats: Get CPU, memory, node count, sessions
- getRunningApps: List running applications  
- getInstalledApps: List installed apps
- getCapabilities: List system capabilities
- getNodeStatus: Get local node health
- getUserInfo: Get current user
- runTerminalCommand: Execute shell commands
- generateSystemGraph: Get graph data for charts

Guidelines:
- Use tools to get real data instead of guessing
- When showing data, format it nicely
- For graphs, include the data but acknowledge rendering depends on client
- Be technical and concise`

  const systemPrompt = systemContext 
    ? `${baseSystemPrompt}\n\n${systemContext}`
    : baseSystemPrompt

  try {
    const result = await generateText({
      model: opencode(FREE_MODEL),
      messages: [
        { role: "system", content: systemPrompt },
        ...messages,
      ],
      tools: {
        getSystemStats,
        getRunningApps,
        getInstalledApps,
        getCapabilities,
        getNodeStatus,
        getUserInfo,
        runTerminalCommand,
        generateSystemGraph,
      },
    })

    const toolCalls = result.toolCalls || []
    const toolResults = result.toolResults || []

    return Response.json({ 
      text: result.text,
      finishReason: result.finishReason,
      toolCalls: toolCalls.map((tc, i) => ({
        name: tc.toolName,
        args: tc.args,
        result: toolResults[i]?.result,
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