import { generateText } from "ai"
import { createOpenAI } from "@ai-sdk/openai"
import { systemStatsStore, windowsStore } from "@/stores/desktop-store"
import { appStore } from "@/platform/state/app-store"
import { capabilityStore } from "@/platform/state/capability-store"
import { nodeStore } from "@/platform/state/node-store"
import { fileSystemStore, isFileSystemAccessSupported } from "@/stores/file-system-store"
import { getCodebaseContext, scanCodebase } from "@/stores/codebase-context"
import {
  getWorkflowsForAI,
  getWorkflowDetailsForAI,
  createWorkflowFromAI,
  updateWorkflowFromAI,
  executeWorkflow,
  getExecutionHistory,
  type Workflow,
} from "@/stores/workflow-store"
import { getSystemPrompt } from "./system-prompt"

const opencode = createOpenAI({
  baseURL: "https://opencode.ai/zen/v1",
  apiKey: process.env.OPENCODE_API_KEY || "",
})

const FREE_MODEL = "minimax-m2.5-free"

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
    cpuUsage: stats.cpuUsage,
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

function getWorkflowContext(): string {
  const workflows = getWorkflowsForAI()
  return `
## Workflows

${workflows}

You can help users:
- Create new workflows with stages and actions
- Edit existing workflows
- Execute workflows manually
- View workflow execution history

To work with workflows, tell me what you want to do (create/edit/run) and I'll help you manage them.`
}

export async function POST(req: Request) {
  const body = await req.json()
  const { messages, action, workflowAction } = body

  // Handle special actions
  if (action === "scanCodebase") {
    await scanCodebase()
    return Response.json({ success: true, message: "Codebase scanned" })
  }

  if (action === "openFolder") {
    const { openDirectory } = await import("@/stores/file-system-store")
    const success = await openDirectory()
    if (success) {
      await scanCodebase()
    }
    return Response.json({ success, message: success ? "Folder opened" : "Failed" })
  }

  // Handle workflow actions
  if (workflowAction) {
    try {
      switch (workflowAction.type) {
        case "list": {
          const workflows = getWorkflowsForAI()
          return Response.json({ text: `Here are your workflows:\n\n${workflows}` })
        }

        case "details": {
          const details = getWorkflowDetailsForAI(workflowAction.workflowId)
          return Response.json({ text: details })
        }

        case "create": {
          const workflow = createWorkflowFromAI(workflowAction.spec)
          if (workflow) {
            return Response.json({ 
              text: `Created workflow "${workflow.name}" successfully! You can now open the Workflow Builder to edit it.`,
            })
          }
          return Response.json({ text: "Failed to create workflow. Check the JSON format." }, { status: 400 })
        }

        case "update": {
          updateWorkflowFromAI(workflowAction.workflowId, workflowAction.updates)
          return Response.json({ text: "Workflow updated successfully!" })
        }

        case "execute": {
          const exec = await executeWorkflow(workflowAction.workflowId)
          return Response.json({ 
            text: `Workflow execution ${exec.status}: ${exec.error || "Completed in " + (exec.completedAt ? Math.round((exec.completedAt - exec.startedAt) / 1000) + "s" : "running...")}`,
          })
        }

        case "history": {
          const history = getExecutionHistory(workflowAction.workflowId)
          if (history.length === 0) {
            return Response.json({ text: "No execution history for this workflow." })
          }
          const summary = history.map(e => 
            `- ${new Date(e.startedAt).toLocaleString()}: ${e.status} ${e.error ? `(error: ${e.error})` : ""}`
          ).join("\n")
          return Response.json({ text: `Execution history:\n${summary}` })
        }

        default:
          return Response.json({ text: "Unknown workflow action" }, { status: 400 })
      }
    } catch (error) {
      return Response.json({ 
        text: `Workflow error: ${error instanceof Error ? error.message : "Unknown error"}` 
      }, { status: 500 })
    }
  }

  const systemData = getRealSystemData()
  const codebaseContext = getCodebaseContext()
  const workflowContext = getWorkflowContext()
  const fsSupported = isFileSystemAccessSupported()

  const systemPrompt = getSystemPrompt()
    .replace("{{SYSTEM_DATA}}", JSON.stringify(systemData, null, 2))
    .replace("{{CODEBASE_CONTEXT}}", codebaseContext)
    .replace("{{WORKFLOW_CONTEXT}}", workflowContext)
    .replace("{{FS_SUPPORTED}}", fsSupported ? "yes" : "no")

  try {
    const result = await generateText({
      model: opencode(FREE_MODEL),
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