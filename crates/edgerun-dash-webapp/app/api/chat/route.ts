import { generateText } from "ai"
import { createOpenAI } from "@ai-sdk/openai"
import { systemStatsStore, windowsStore } from "@/stores/desktop-store"
import { appStore } from "@/platform/state/app-store"
import { capabilityStore } from "@/platform/state/capability-store"
import { nodeStore } from "@/platform/state/node-store"
import { fileSystemStore } from "@/stores/file-system-store"
import { getCodebaseContext } from "@/stores/codebase-context"
import {
  getWorkflowsForAI,
  getWorkflowDetailsForAI,
  getExecutionHistory,
} from "@/stores/workflow-store"
import { getSystemPrompt } from "./system-prompt"
import { explainToolAvailability } from "@/platform/registries/tool-registry"

const opencode = createOpenAI({
  baseURL: "https://opencode.ai/zen/v1",
  apiKey: process.env.OPENCODE_API_KEY || "",
})

const FREE_MODEL = "minimax-m2-free"

export const maxDuration = 60

function getRealSystemData() {
  const stats = systemStatsStore.get()
  const windows = windowsStore.get()
  const apps = appStore.get()
  const caps = capabilityStore.get()
  const node = nodeStore.get()
  const fs = fileSystemStore.get()

  const runningApps = windows
    .filter(w => w.appId !== "app-store" && w.appId !== "app-studio")
    .map(w => ({ id: w.appId, title: w.title }))

  return {
    nodeCount: stats.nodeCount,
    activeSessions: stats.activeSessions,
    ramUsed: stats.ramUsage.used,
    ramTotal: stats.ramUsage.total,
    ramPercent: Math.round((stats.ramUsage.used / stats.ramUsage.total) * 100),
    isConnected: stats.isConnected,
    runningApps,
    nodeId: node.currentNode?.nodeId || null,
    nodeHealth: node.currentNode?.health || "unknown",
    filesystem: {
      isOpen: !!fs.rootHandle,
      rootPath: fs.rootPath,
      entryCount: fs.entries.length,
      openFiles: fs.openFiles.map(f => ({ name: f.name, path: f.path, modified: f.modified })),
    },
  }
}

function getReadonlyContext(): string {
  const workflows = getWorkflowsForAI()
  const toolAvailability = explainToolAvailability()

  return `
## Workflows (${workflows.length} total)

Available workflows:
${workflows.map(w => `- ${w.name} (${w.workflowId}): ${w.trigger}`).join("\n") || "No workflows"}

## Registered Tools

${toolAvailability}

To perform actions, reference tools by their toolId. The chat route is read-only — all mutations go through the tool registry.`
}

export async function POST(req: Request) {
  const body = await req.json()
  const { messages } = body

  const systemData = getRealSystemData()
  const codebaseContext = getCodebaseContext()
  const readonlyCtx = getReadonlyContext()

  const systemPrompt = getSystemPrompt()
    .replace("{{SYSTEM_DATA}}", JSON.stringify(systemData, null, 2))
    .replace("{{CODEBASE_CONTEXT}}", codebaseContext)
    .replace("{{WORKFLOW_CONTEXT}}", readonlyCtx)

  try {
    const result = await generateText({
      model: opencode(FREE_MODEL) as any,
      messages: [
        { role: "system", content: systemPrompt },
        ...messages,
      ],
    })

    return Response.json({ 
      text: result.text,
      finishReason: result.finishReason,
    })
  } catch (error) {
    return Response.json({ 
      text: "AI unavailable. Check OPENCODE_API_KEY in .env.local",
    }, { status: 503 })
  }
}
