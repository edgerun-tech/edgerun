/*
 * Register platform tools.
 * All state-changing actions are registered here, not in the chat route.
 * The chat route is read-only and delegates to these tools.
 */

import { registerTool } from "@/platform/registries/tool-registry"
import { invokeNodeTool } from "@/platform/protocol/tools"
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

function toolInput(input: unknown): Record<string, unknown> {
  return (input || {}) as Record<string, unknown>
}

function registerCodelyzerTools(): void {
  registerTool({
    toolId: "codelyzer.graph.summary",
    name: "Codelyzer Graph Summary",
    description: "Read the codelyzer graph summary from the local node.",
    riskClass: "none",
    requiredCapabilities: ["repo_read"],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: {},
    commandPreview: () => "Read codelyzer graph summary",
    handler: async (input) => invokeNodeTool("codelyzer.graph.summary", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "codelyzer.graph.search_symbols",
    name: "Search Code Symbols",
    description: "Search codelyzer symbols by name, file, language, or tag.",
    riskClass: "none",
    requiredCapabilities: ["repo_read"],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: { query: "required" },
    commandPreview: (input) => `Search symbols for ${String(input.query || "")}`,
    handler: async (input) => invokeNodeTool("codelyzer.graph.search_symbols", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "codelyzer.graph.related_nodes",
    name: "Show Related Code Nodes",
    description: "List callers/callees for a codelyzer node.",
    riskClass: "none",
    requiredCapabilities: ["repo_read"],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: { id: "required" },
    commandPreview: (input) => `Read related nodes for ${String(input.id || "")}`,
    handler: async (input) => invokeNodeTool("codelyzer.graph.related_nodes", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "codelyzer.graph.read_file",
    name: "Read Repo File",
    description: "Read a repository file through the node/codelyzer protocol.",
    riskClass: "low",
    requiredCapabilities: ["repo_read"],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: { path: "required" },
    commandPreview: (input) => `Read file ${String(input.path || "")}`,
    evidenceExtractor: (result) => [`repo_read:${JSON.stringify(result).slice(0, 120)}`],
    handler: async (input) => invokeNodeTool("codelyzer.graph.read_file", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "codelyzer.rust_ast.replace_fn_body",
    name: "Replace Rust Function Body",
    description: "AST-safe Rust function body replacement through edgerun-edit.",
    riskClass: "high",
    requiredCapabilities: ["repo_rust_ast_edit"],
    requiresApproval: true,
    requiresUserPresence: true,
    inputSchema: { path: "required", name: "required", body: "required" },
    commandPreview: (input) => `Replace Rust function body ${String(input.name || "")} in ${String(input.path || "")}`,
    auditRecord: (input) => `rust_ast.replace_fn_body ${String(input.name || "")} ${String(input.path || "")}`,
    handler: async (input) => invokeNodeTool("codelyzer.rust_ast.replace_fn_body", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "codelyzer.rust_ast.add_fn",
    name: "Add Rust Function",
    description: "AST-safe Rust function insertion through edgerun-edit.",
    riskClass: "high",
    requiredCapabilities: ["repo_rust_ast_edit"],
    requiresApproval: true,
    requiresUserPresence: true,
    inputSchema: { path: "required", name: "required", body: "required" },
    commandPreview: (input) => `Add Rust function ${String(input.name || "")} to ${String(input.path || "")}`,
    auditRecord: (input) => `rust_ast.add_fn ${String(input.name || "")} ${String(input.path || "")}`,
    handler: async (input) => invokeNodeTool("codelyzer.rust_ast.add_fn", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "codelyzer.rust_ast.add_use",
    name: "Add Rust Use",
    description: "AST-safe Rust import insertion through edgerun-edit.",
    riskClass: "high",
    requiredCapabilities: ["repo_rust_ast_edit"],
    requiresApproval: true,
    requiresUserPresence: true,
    inputSchema: { path: "required", use_path: "required" },
    commandPreview: (input) => `Add use ${String(input.use_path || "")} to ${String(input.path || "")}`,
    auditRecord: (input) => `rust_ast.add_use ${String(input.use_path || "")} ${String(input.path || "")}`,
    handler: async (input) => invokeNodeTool("codelyzer.rust_ast.add_use", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "codelyzer.rust_ast.add_derive",
    name: "Add Rust Derive",
    description: "AST-safe derive insertion for Rust structs/enums.",
    riskClass: "high",
    requiredCapabilities: ["repo_rust_ast_edit"],
    requiresApproval: true,
    requiresUserPresence: true,
    inputSchema: { path: "required", name: "required", derive: "required" },
    commandPreview: (input) => `Add derive ${String(input.derive || "")} to ${String(input.name || "")} in ${String(input.path || "")}`,
    auditRecord: (input) => `rust_ast.add_derive ${String(input.derive || "")} ${String(input.name || "")} ${String(input.path || "")}`,
    handler: async (input) => invokeNodeTool("codelyzer.rust_ast.add_derive", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "codelyzer.rust_ast.rename_type",
    name: "Rename Rust Type",
    description: "AST-safe Rust type rename through edgerun-edit.",
    riskClass: "high",
    requiredCapabilities: ["repo_rust_ast_edit"],
    requiresApproval: true,
    requiresUserPresence: true,
    inputSchema: { old: "required", new: "required" },
    commandPreview: (input) => `Rename Rust type ${String(input.old || "")} to ${String(input.new || "")}`,
    auditRecord: (input) => `rust_ast.rename_type ${String(input.old || "")} ${String(input.new || "")}`,
    handler: async (input) => invokeNodeTool("codelyzer.rust_ast.rename_type", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "codelyzer.text.replace_text",
    name: "Replace Text",
    description: "High-risk text replacement for non-Rust files. Rust should use rust_ast.",
    riskClass: "critical",
    requiredCapabilities: ["repo_text_edit"],
    requiresApproval: true,
    requiresUserPresence: true,
    inputSchema: { path: "required", old: "required", new: "required" },
    commandPreview: (input) => `Replace text in ${String(input.path || "")}`,
    auditRecord: (input) => `text.replace_text ${String(input.path || "")}`,
    handler: async (input) => invokeNodeTool("codelyzer.text.replace_text", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "xray.viewport.focus_node",
    name: "Focus Xray Node",
    description: "Focus/select a node in the Xray viewport.",
    riskClass: "low",
    requiredCapabilities: ["xray_viewport_control"],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: { id: "required" },
    commandPreview: (input) => `Focus Xray node ${String(input.id || "")}`,
    handler: async (input) => invokeNodeTool("xray.viewport.focus_node", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "xray.viewport.show_related",
    name: "Show Related Xray Nodes",
    description: "Focus a node and show its related graph neighborhood.",
    riskClass: "low",
    requiredCapabilities: ["xray_viewport_control"],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: { id: "required" },
    commandPreview: (input) => `Show related Xray nodes for ${String(input.id || "")}`,
    handler: async (input) => invokeNodeTool("xray.viewport.show_related", toolInput(input)),
    isEnabled: true,
  })

  registerTool({
    toolId: "xray.viewport.set_camera",
    name: "Set Xray Camera",
    description: "Move the Xray camera.",
    riskClass: "low",
    requiredCapabilities: ["xray_viewport_control"],
    requiresApproval: false,
    requiresUserPresence: false,
    inputSchema: {},
    commandPreview: () => "Move Xray camera",
    handler: async (input) => invokeNodeTool("xray.viewport.set_camera", toolInput(input)),
    isEnabled: true,
  })
}

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

  registerCodelyzerTools()
}
