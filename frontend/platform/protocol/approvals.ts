import { addApproval, approve, reject } from "@/stores/approval-tracker"
import { protocolClient } from "./client"
import type { PendingApproval, PermissionScope } from "@/stores/permission-store"

type ProtocolApproval = PendingApproval & {
  source?: string
}

type ProtocolApprovalsResponse = {
  approvals?: ProtocolApproval[]
}

const decoder = new TextDecoder()

function normalizeScope(scope: string | undefined): PermissionScope {
  switch (scope) {
    case "repo_text_edit":
    case "repo_rust_ast_edit":
    case "repo_read":
    case "xray_viewport_control":
    case "tool":
    case "filesystem":
    case "node_connection":
      return scope
    default:
      return "tool"
  }
}

function normalizeRisk(risk: string | undefined): PendingApproval["riskLevel"] {
  switch (risk) {
    case "low":
    case "medium":
    case "high":
    case "critical":
      return risk
    default:
      return "medium"
  }
}

export async function listProtocolApprovals(): Promise<PendingApproval[]> {
  const response = await protocolClient.send({
    method: "GET",
    path: "/protocol/approvals",
    headers: { Accept: "application/json" },
  })
  if (response.status !== 200) return []

  const parsed = JSON.parse(decoder.decode(response.body)) as ProtocolApprovalsResponse
  return (parsed.approvals || []).map((approval) => ({
    approvalId: approval.approvalId,
    operation: approval.operation,
    scope: normalizeScope(approval.scope),
    description: approval.description,
    requestedAt: approval.requestedAt || new Date().toISOString(),
    expiresAt: approval.expiresAt,
    riskLevel: normalizeRisk(approval.riskLevel),
  }))
}

export async function syncProtocolApprovals(): Promise<number> {
  const approvals = await listProtocolApprovals()
  for (const approval of approvals) addApproval(approval)
  return approvals.length
}

export async function approveProtocolApproval(approvalId: string): Promise<void> {
  const response = await protocolClient.send({
    method: "GET",
    path: `/protocol/approvals/${encodeURIComponent(approvalId)}/approve`,
    headers: { Accept: "application/json" },
  })
  if (response.status !== 200) {
    throw new Error(`Approval failed with status ${response.status}`)
  }
  approve(approvalId)
}

export async function rejectProtocolApproval(approvalId: string, reason = "Rejected by user"): Promise<void> {
  const response = await protocolClient.send({
    method: "GET",
    path: `/protocol/approvals/${encodeURIComponent(approvalId)}/reject`,
    headers: { Accept: "application/json" },
  })
  if (response.status !== 200) {
    throw new Error(`Rejection failed with status ${response.status}`)
  }
  reject(approvalId, reason)
}
