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

export interface BrowserStorageProviderOptions {
  appId?: string
  allowWrite?: boolean
  allowRead?: boolean
}

export class BrowserStorageCapabilityProvider implements CapabilityProviderEndpoint {
  readonly appId: string
  readonly identity: RuntimeIdentity
  readonly descriptor: CapabilityDescriptorView

  private readonly allowWrite: boolean
  private readonly allowRead: boolean
  private readonly objects = new Map<string, Uint8Array>()
  private readonly sessions = new Map<string, Set<string>>()

  constructor(options: BrowserStorageProviderOptions = {}) {
    this.appId = options.appId ?? "cap.storage.object"
    this.allowWrite = options.allowWrite ?? true
    this.allowRead = options.allowRead ?? true
    this.identity = {
      appId: this.appId,
      assuranceClass: "software",
    }
    this.descriptor = {
      capabilityId: "object.storage",
      providerName: "Browser Object Storage",
      actions: ["object.read", "object.write", "object.delete"],
      scopes: ["browser.local"],
      risk: "medium",
    }
  }

  async openSession(request: CapabilitySessionRequest): Promise<CapabilitySessionResponse> {
    const requested = new Set(request.requestedActions)
    const allowed = new Set<string>()

    if (this.allowRead && (requested.size === 0 || requested.has("object.read"))) {
      allowed.add("object.read")
    }
    if (this.allowWrite && (requested.size === 0 || requested.has("object.write"))) {
      allowed.add("object.write")
    }
    if (requested.has("object.delete")) {
      allowed.add("object.delete")
    }

    if (allowed.size === 0) {
      const event = runtimeEventLog.append({
        kind: "capability_grant_denied",
        actor: request.requester.appId,
        target: this.appId,
        requestId: request.id,
        capabilityId: this.descriptor.capabilityId,
        decision: "deny",
        reason: "storage policy denied all requested actions",
      })
      return {
        id: createRuntimeId("grant"),
        requestId: request.id,
        providerAppId: this.appId,
        accepted: false,
        reason: "storage policy denied all requested actions",
        eventId: event.id,
      }
    }

    const sessionId = createRuntimeId("storage_session")
    this.sessions.set(sessionId, allowed)
    const event = runtimeEventLog.append({
      kind: "capability_session_opened",
      actor: request.requester.appId,
      target: this.appId,
      requestId: request.id,
      sessionId,
      capabilityId: this.descriptor.capabilityId,
      decision: "allow",
      metadata: {
        grantedActions: [...allowed],
      },
    })

    return {
      id: createRuntimeId("grant"),
      requestId: request.id,
      providerAppId: this.appId,
      accepted: true,
      sessionId,
      grantedActions: [...allowed],
      eventId: event.id,
    }
  }

  async invoke(request: CapabilityInvocationRequest): Promise<CapabilityInvocationResponse> {
    const allowed = this.sessions.get(request.sessionId)
    if (!allowed || !allowed.has(request.action)) {
      const event = runtimeEventLog.append({
        kind: "capability_invocation_denied",
        actor: request.requester.appId,
        target: this.appId,
        requestId: request.id,
        sessionId: request.sessionId,
        capabilityId: this.descriptor.capabilityId,
        decision: "deny",
        reason: `storage action ${request.action} is not granted`,
      })
      return {
        id: createRuntimeId("storage_rsp"),
        requestId: request.id,
        providerAppId: this.appId,
        decision: "deny",
        reason: `storage action ${request.action} is not granted`,
        eventId: event.id,
      }
    }

    if (request.action === "object.write") {
      const key = request.resource || createRuntimeId("object")
      this.objects.set(key, request.payload ?? new Uint8Array())
      const event = runtimeEventLog.append({
        kind: "capability_action_completed",
        actor: request.requester.appId,
        target: this.appId,
        requestId: request.id,
        sessionId: request.sessionId,
        capabilityId: this.descriptor.capabilityId,
        decision: "allow",
        metadata: {
          resource: key,
          bytes: request.payload?.byteLength ?? 0,
        },
      })
      return {
        id: createRuntimeId("storage_rsp"),
        requestId: request.id,
        providerAppId: this.appId,
        decision: "allow",
        payload: utf8Encode(JSON.stringify({ objectId: key })),
        eventId: event.id,
      }
    }

    if (request.action === "object.read") {
      const key = request.resource || ""
      const object = this.objects.get(key)
      if (!object) {
        const event = runtimeEventLog.append({
          kind: "capability_action_failed",
          actor: request.requester.appId,
          target: this.appId,
          requestId: request.id,
          sessionId: request.sessionId,
          capabilityId: this.descriptor.capabilityId,
          decision: "deny",
          reason: `object ${key} not found`,
        })
        return {
          id: createRuntimeId("storage_rsp"),
          requestId: request.id,
          providerAppId: this.appId,
          decision: "deny",
          reason: `object ${key} not found`,
          eventId: event.id,
        }
      }
      const event = runtimeEventLog.append({
        kind: "capability_action_completed",
        actor: request.requester.appId,
        target: this.appId,
        requestId: request.id,
        sessionId: request.sessionId,
        capabilityId: this.descriptor.capabilityId,
        decision: "allow",
        metadata: {
          resource: key,
          bytes: object.byteLength,
        },
      })
      return {
        id: createRuntimeId("storage_rsp"),
        requestId: request.id,
        providerAppId: this.appId,
        decision: "allow",
        payload: object,
        eventId: event.id,
      }
    }

    const event = runtimeEventLog.append({
      kind: "capability_invocation_denied",
      actor: request.requester.appId,
      target: this.appId,
      requestId: request.id,
      sessionId: request.sessionId,
      capabilityId: this.descriptor.capabilityId,
      decision: "deny",
      reason: `unsupported storage action ${request.action}`,
    })
    return {
      id: createRuntimeId("storage_rsp"),
      requestId: request.id,
      providerAppId: this.appId,
      decision: "deny",
      reason: `unsupported storage action ${request.action}`,
      eventId: event.id,
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
}
