import {
  createRuntimeId,
  utf8Encode,
  type CapabilityDescriptorView,
  type CapabilityInvocationRequest,
  type CapabilityInvocationResponse,
  type CapabilityProviderEndpoint,
  type CapabilitySessionRequest,
  type CapabilitySessionResponse,
  type RuntimeIdentity,
} from "../browser-capability-types"
import { runtimeEventLog } from "../runtime-event-log"

export interface BrowserNetworkProviderOptions {
  appId?: string
  allowedOrigins?: string[]
  deniedOrigins?: string[]
  allowByDefault?: boolean
}

export class BrowserNetworkCapabilityProvider implements CapabilityProviderEndpoint {
  readonly appId: string
  readonly identity: RuntimeIdentity
  readonly descriptor: CapabilityDescriptorView

  private readonly allowedOrigins: Set<string>
  private readonly deniedOrigins: Set<string>
  private readonly allowByDefault: boolean
  private readonly sessions = new Map<string, Set<string>>()

  constructor(options: BrowserNetworkProviderOptions = {}) {
    this.appId = options.appId ?? "cap.network.fetch"
    this.allowedOrigins = new Set(options.allowedOrigins ?? [])
    this.deniedOrigins = new Set(options.deniedOrigins ?? [])
    this.allowByDefault = options.allowByDefault ?? false
    this.identity = {
      appId: this.appId,
      assuranceClass: "software",
    }
    this.descriptor = {
      capabilityId: "network.fetch",
      providerName: "Browser Fetch Network",
      actions: ["network.fetch", "network.head"],
      scopes: ["origin"],
      risk: "high",
    }
  }

  async openSession(request: CapabilitySessionRequest): Promise<CapabilitySessionResponse> {
    const requested = new Set(request.requestedActions)
    const granted = new Set<string>()
    if (requested.size === 0 || requested.has("network.fetch")) granted.add("network.fetch")
    if (requested.has("network.head")) granted.add("network.head")

    if (granted.size === 0) {
      const event = runtimeEventLog.append({
        kind: "capability_grant_denied",
        actor: request.requester.appId,
        target: this.appId,
        requestId: request.id,
        capabilityId: this.descriptor.capabilityId,
        decision: "deny",
        reason: "network policy denied all requested actions",
      })
      return {
        id: createRuntimeId("grant"),
        requestId: request.id,
        providerAppId: this.appId,
        accepted: false,
        reason: "network policy denied all requested actions",
        eventId: event.id,
      }
    }

    const sessionId = createRuntimeId("network_session")
    this.sessions.set(sessionId, granted)
    const event = runtimeEventLog.append({
      kind: "capability_session_opened",
      actor: request.requester.appId,
      target: this.appId,
      requestId: request.id,
      sessionId,
      capabilityId: this.descriptor.capabilityId,
      decision: "allow",
      metadata: {
        grantedActions: [...granted],
        allowedOrigins: [...this.allowedOrigins],
        deniedOrigins: [...this.deniedOrigins],
        allowByDefault: this.allowByDefault,
      },
    })

    return {
      id: createRuntimeId("grant"),
      requestId: request.id,
      providerAppId: this.appId,
      accepted: true,
      sessionId,
      grantedActions: [...granted],
      eventId: event.id,
    }
  }

  async invoke(request: CapabilityInvocationRequest): Promise<CapabilityInvocationResponse> {
    const granted = this.sessions.get(request.sessionId)
    if (!granted || !granted.has(request.action)) {
      const event = runtimeEventLog.append({
        kind: "capability_invocation_denied",
        actor: request.requester.appId,
        target: this.appId,
        requestId: request.id,
        sessionId: request.sessionId,
        capabilityId: this.descriptor.capabilityId,
        decision: "deny",
        reason: `network action ${request.action} is not granted`,
      })
      return {
        id: createRuntimeId("network_rsp"),
        requestId: request.id,
        providerAppId: this.appId,
        decision: "deny",
        reason: `network action ${request.action} is not granted`,
        eventId: event.id,
      }
    }

    if (request.action !== "network.fetch" && request.action !== "network.head") {
      const event = runtimeEventLog.append({
        kind: "capability_invocation_denied",
        actor: request.requester.appId,
        target: this.appId,
        requestId: request.id,
        sessionId: request.sessionId,
        capabilityId: this.descriptor.capabilityId,
        decision: "deny",
        reason: `unsupported network action ${request.action}`,
      })
      return {
        id: createRuntimeId("network_rsp"),
        requestId: request.id,
        providerAppId: this.appId,
        decision: "deny",
        reason: `unsupported network action ${request.action}`,
        eventId: event.id,
      }
    }

    const url = request.resource || ""
    const originDecision = this.evaluateOrigin(url)
    if (!originDecision.allowed) {
      const event = runtimeEventLog.append({
        kind: "capability_invocation_denied",
        actor: request.requester.appId,
        target: this.appId,
        requestId: request.id,
        sessionId: request.sessionId,
        capabilityId: this.descriptor.capabilityId,
        decision: "deny",
        reason: originDecision.reason,
        metadata: { url },
      })
      return {
        id: createRuntimeId("network_rsp"),
        requestId: request.id,
        providerAppId: this.appId,
        decision: "deny",
        reason: originDecision.reason,
        eventId: event.id,
      }
    }

    try {
      const response = await fetch(url, {
        method: request.action === "network.head" ? "HEAD" : "GET",
      })
      const body = request.action === "network.head" ? "" : await response.text()
      const event = runtimeEventLog.append({
        kind: "capability_action_completed",
        actor: request.requester.appId,
        target: this.appId,
        requestId: request.id,
        sessionId: request.sessionId,
        capabilityId: this.descriptor.capabilityId,
        decision: "allow",
        metadata: {
          url,
          status: response.status,
          bytes: body.length,
        },
      })
      return {
        id: createRuntimeId("network_rsp"),
        requestId: request.id,
        providerAppId: this.appId,
        decision: "allow",
        payload: utf8Encode(JSON.stringify({
          status: response.status,
          ok: response.ok,
          headers: Object.fromEntries(response.headers.entries()),
          body,
        })),
        eventId: event.id,
      }
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error)
      const event = runtimeEventLog.append({
        kind: "capability_action_failed",
        actor: request.requester.appId,
        target: this.appId,
        requestId: request.id,
        sessionId: request.sessionId,
        capabilityId: this.descriptor.capabilityId,
        decision: "deny",
        reason,
        metadata: { url },
      })
      return {
        id: createRuntimeId("network_rsp"),
        requestId: request.id,
        providerAppId: this.appId,
        decision: "deny",
        reason,
        eventId: event.id,
      }
    }
  }

  async closeSession(sessionId: string): Promise<void> {
    this.sessions.delete(sessionId)
    runtimeEventLog.append({
      kind: "capability_session_closed",
      actor: this.appId,
      target: this.appId,
      sessionId,
      capabilityId: this.descriptor.capabilityId,
    })
  }

  private evaluateOrigin(url: string): { allowed: boolean; reason?: string } {
    let parsed: URL
    try {
      parsed = new URL(url)
    } catch {
      return { allowed: false, reason: "invalid URL" }
    }

    if (this.deniedOrigins.has(parsed.origin) || this.deniedOrigins.has(parsed.hostname)) {
      return { allowed: false, reason: `origin ${parsed.origin} is denied` }
    }

    if (this.allowedOrigins.has(parsed.origin) || this.allowedOrigins.has(parsed.hostname)) {
      return { allowed: true }
    }

    if (this.allowByDefault) {
      return { allowed: true }
    }

    return { allowed: false, reason: `origin ${parsed.origin} is not delegated` }
  }
}
