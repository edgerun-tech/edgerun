import { sha256Hex as _sha256Hex } from "@/platform/utils/bytes"

export type AppAddress = string
export type HexString = string

export type CapabilityDecision = "allow" | "ask" | "deny" | "partial" | "metadata_only"
export type CapabilityTransportKind = "local-worker" | "remote-websocket" | "remote-webrtc" | "in-process"
export type RuntimeEventKind =
  | "app_launch_requested"
  | "app_package_verified"
  | "app_instance_created"
  | "capability_resolution_started"
  | "capability_grant_requested"
  | "capability_grant_accepted"
  | "capability_grant_denied"
  | "capability_session_opened"
  | "capability_invocation_requested"
  | "capability_invocation_allowed"
  | "capability_invocation_denied"
  | "capability_action_completed"
  | "capability_action_failed"
  | "capability_session_closed"
  | "app_stopped"
  | "router_message_delivered"
  | "router_message_denied"

export interface RuntimeIdentity {
  appId: AppAddress
  publicKey?: Uint8Array
  packageHash?: HexString
  instanceId?: string
  runtimeHash?: HexString
  assuranceClass?: "software" | "hardware_backed" | "attested_runtime"
}

export interface EvidenceBundle {
  caller: RuntimeIdentity
  target?: RuntimeIdentity
  capabilityId?: string
  action?: string
  resource?: string
  delegationChain?: Uint8Array[]
  grant?: Uint8Array
  runtimeEvidence?: Uint8Array
  revocationHints?: Uint8Array[]
  context?: Record<string, unknown>
}

export interface RuntimeMessage {
  id: string
  from: AppAddress
  to: AppAddress
  payload: Uint8Array
  evidence?: EvidenceBundle
  createdAt: number
  expiresAt?: number
  signature?: Uint8Array
}

export interface RuntimeResponse {
  id: string
  requestId: string
  from: AppAddress
  to: AppAddress
  decision: CapabilityDecision
  payload?: Uint8Array
  reason?: string
  eventId?: string
  signature?: Uint8Array
}

export interface RuntimeEvent {
  id: string
  kind: RuntimeEventKind
  actor?: AppAddress
  target?: AppAddress
  requestId?: string
  sessionId?: string
  capabilityId?: string
  decision?: CapabilityDecision
  reason?: string
  inputHash?: HexString
  outputHash?: HexString
  createdAt: number
  metadata?: Record<string, unknown>
}

export interface AppEndpoint {
  appId: AppAddress
  identity: RuntimeIdentity
  post(message: RuntimeMessage): Promise<RuntimeResponse>
}

export interface CapabilityProviderEndpoint {
  appId: AppAddress
  identity: RuntimeIdentity
  descriptor: CapabilityDescriptorView
  openSession(request: CapabilitySessionRequest): Promise<CapabilitySessionResponse>
  invoke(request: CapabilityInvocationRequest): Promise<CapabilityInvocationResponse>
  closeSession(sessionId: string): Promise<void>
}

export interface CapabilityDescriptorView {
  capabilityId: string
  providerName: string
  actions: string[]
  scopes?: string[]
  risk?: "low" | "medium" | "high" | "critical"
}

export interface CapabilitySessionRequest {
  id: string
  requester: RuntimeIdentity
  providerAppId: AppAddress
  capabilityId: string
  requestedActions: string[]
  evidence: EvidenceBundle
  createdAt: number
}

export interface CapabilitySessionResponse {
  id: string
  requestId: string
  providerAppId: AppAddress
  accepted: boolean
  sessionId?: string
  grantedActions?: string[]
  reason?: string
  eventId?: string
}

export interface CapabilityInvocationRequest {
  id: string
  sessionId: string
  requester: RuntimeIdentity
  providerAppId: AppAddress
  capabilityId: string
  action: string
  resource?: string
  payload?: Uint8Array
  evidence: EvidenceBundle
  createdAt: number
}

export interface CapabilityInvocationResponse {
  id: string
  requestId: string
  providerAppId: AppAddress
  decision: CapabilityDecision
  payload?: Uint8Array
  reason?: string
  eventId?: string
}

export interface CapabilityRoutePlan {
  providerAppId: AppAddress
  providerIdentity: RuntimeIdentity
  transport: CapabilityTransportKind
  descriptor: CapabilityDescriptorView
  requiresApproval: boolean
  evidence: EvidenceBundle
  explanation: string
}

export function createRuntimeId(prefix = "rt"): string {
  const random = new Uint8Array(16)
  crypto.getRandomValues(random)
  return `${prefix}_${Array.from(random, b => b.toString(16).padStart(2, "0")).join("")}`
}

export async function sha256Hex(data: Uint8Array | string): Promise<HexString> {
  return _sha256Hex(data)
}

export function utf8Encode(value: string): Uint8Array {
  return new TextEncoder().encode(value)
}

export function utf8Decode(value: Uint8Array): string {
  return new TextDecoder().decode(value)
}
