/**
 * Register platform tools.
 * All state-changing actions are registered here, not in the chat route.
 * The chat route is read-only and delegates to these tools.
 */

import { registerTool } from "@/platform/registries/tool-registry"
import { scanCodebase } from "@/stores/codebase-context"
import { openDirectory } from "@/stores/file-system-store"
import {
  createWorkflow,
  updateWorkflow,
  executeWorkflow,
  getWorkflowsForAI,
  getWorkflowDetailsForAI,
  getExecutionHistory,
} from "@/stores/workflow-store"

export function registerPlatformTools(): void {
  registerTool({
    toolId: "scan_codebase",
    name: "Scan Codebase",
    description: "Scan the currently opened project directory and build a file index for context.",
    riskClass: "none",
    requiredCapabilities: [],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: {},
    handler: async () => {
      await scanCodebase()
      return { success: true, message: "Codebase scanned" }
    },
    isEnabled: true,
  })

  registerTool({
    toolId: "open_directory",
    name: "Open Directory",
    description: "Open a folder picker and load the selected directory as the project root.",
    riskClass: "low",
    requiredCapabilities: [],
    requiresApproval: false,
    requiresUserPresence: true,
    inputSchema: {},
    handler: async () => {
      const success = await openDirectory()
      if (success) {
        await scanCodebase()
      }
      return { success, message: success ? "Directory opened" : "Failed to open directory" }
    },
    isEnabled: true,
  })

  registerTool({
    toolId: "create_workflow",
    name: "Create Workflow",
    description: "Create a new workflow with a name, description, and trigger type.",
    riskClass: "low",
    requiredCapabilities: [],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: {
      name: "required",
      description: "required",
      trigger: "required",
    },
    handler: async (input: unknown) => {
      const { name, description, trigger } = input as { name: string; description: string; trigger: string }
      const id = createWorkflow(name, description, trigger as any)
      return { success: true, workflowId: id }
    },
    isEnabled: true,
  })

  registerTool({
    toolId: "update_workflow",
    name: "Update Workflow",
    description: "Update an existing workflow's fields.",
    riskClass: "low",
    requiredCapabilities: [],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: {
      workflowId: "required",
      updates: "required",
    },
    handler: async (input: unknown) => {
      const { workflowId, updates } = input as { workflowId: string; updates: Record<string, unknown> }
      updateWorkflow(workflowId, updates)
      return { success: true, workflowId }
    },
    isEnabled: true,
  })

  registerTool({
    toolId: "execute_workflow",
    name: "Execute Workflow",
    description: "Start execution of a workflow by ID.",
    riskClass: "medium",
    requiredCapabilities: [],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: {
      workflowId: "required",
    },
    handler: async (input: unknown) => {
      const { workflowId } = input as { workflowId: string }
      const execId = await executeWorkflow(workflowId)
      return { success: true, executionId: execId }
    },
    isEnabled: true,
  })

  registerTool({
    toolId: "list_workflows",
    name: "List Workflows",
    description: "List all registered workflows.",
    riskClass: "none",
    requiredCapabilities: [],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: {},
    handler: async () => {
      const workflows = getWorkflowsForAI()
      return { workflows }
    },
    isEnabled: true,
  })

  registerTool({
    toolId: "get_workflow_details",
    name: "Get Workflow Details",
    description: "Get details for a specific workflow by ID.",
    riskClass: "none",
    requiredCapabilities: [],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: {
      workflowId: "required",
    },
    handler: async (input: unknown) => {
      const { workflowId } = input as { workflowId: string }
      const workflow = getWorkflowDetailsForAI(workflowId)
      if (!workflow) return { error: "Workflow not found" }
      return { workflow }
    },
    isEnabled: true,
  })

  registerTool({
    toolId: "get_workflow_history",
    name: "Get Workflow History",
    description: "Get execution history for a workflow.",
    riskClass: "none",
    requiredCapabilities: [],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: {
      workflowId: "required",
    },
    handler: async (input: unknown) => {
      const { workflowId } = input as { workflowId: string }
      const history = getExecutionHistory(workflowId)
      return { history }
    },
    isEnabled: true,
  })
}
