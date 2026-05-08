import { protocolClient } from "./client"

const encoder = new TextEncoder()
const decoder = new TextDecoder()

export type NodeToolResult = {
  status: "executed" | "pending_approval" | "blocked" | "failed"
  result?: unknown
  approvalId?: string
  error?: string
  evidenceRefs?: string[]
}

export async function invokeNodeTool(
  toolId: string,
  input: Record<string, unknown>,
): Promise<NodeToolResult> {
  const response = await protocolClient.send({
    method: "POST",
    path: "/protocol/tools/invoke",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
    },
    body: encoder.encode(JSON.stringify({ toolId, input })),
  })

  const bodyText = decoder.decode(response.body)
  const parsed = bodyText ? JSON.parse(bodyText) : {}

  if (response.status !== 200) {
    return {
      status: "failed",
      error: parsed.error || `Tool invocation failed with status ${response.status}`,
    }
  }

  return parsed as NodeToolResult
}

export async function invokeCodelyzerTool(
  name: string,
  input: Record<string, unknown>,
): Promise<NodeToolResult> {
  return invokeNodeTool(`codelyzer.${name}`, input)
}

export async function invokeXrayCommand(
  command: string,
  input: Record<string, unknown>,
): Promise<NodeToolResult> {
  return invokeNodeTool(`xray.viewport.${command}`, input)
}
