import {
  createRuntimeId,
  type AppAddress,
  type AppEndpoint,
  type CapabilityProviderEndpoint,
  type CapabilityRoutePlan,
  type RuntimeIdentity,
  type RuntimeMessage,
  type RuntimeResponse,
} from "./browser-capability-types"
import { runtimeEventLog } from "./runtime-event-log"

export class BrowserMessageRouter {
  private apps = new Map<AppAddress, AppEndpoint>()
  private providers = new Map<AppAddress, CapabilityProviderEndpoint>()

  registerApp(endpoint: AppEndpoint): void {
    this.apps.set(endpoint.appId, endpoint)
  }

  unregisterApp(appId: AppAddress): void {
    this.apps.delete(appId)
    this.providers.delete(appId)
  }

  registerProvider(endpoint: CapabilityProviderEndpoint): void {
    this.providers.set(endpoint.appId, endpoint)
    this.apps.set(endpoint.appId, {
      appId: endpoint.appId,
      identity: endpoint.identity,
      post: async (message) => this.invokeProviderMessage(endpoint, message),
    })
  }

  getIdentity(appId: AppAddress): RuntimeIdentity | undefined {
    return this.apps.get(appId)?.identity ?? this.providers.get(appId)?.identity
  }

  listApps(): RuntimeIdentity[] {
    return [...this.apps.values()].map(app => app.identity)
  }

  listProviders(): CapabilityProviderEndpoint[] {
    return [...this.providers.values()]
  }

  resolveCapability(capabilityId: string, caller: RuntimeIdentity): CapabilityRoutePlan | null {
    for (const provider of this.providers.values()) {
      if (provider.descriptor.capabilityId !== capabilityId) continue
      return {
        providerAppId: provider.appId,
        providerIdentity: provider.identity,
        transport: "in-process",
        descriptor: provider.descriptor,
        requiresApproval: false,
        evidence: {
          caller,
          target: provider.identity,
          capabilityId,
          context: {
            router: "browser-message-router",
          },
        },
        explanation: `${caller.appId} can request ${capabilityId} from ${provider.appId}`,
      }
    }
    return null
  }

  async send(message: RuntimeMessage): Promise<RuntimeResponse> {
    const target = this.apps.get(message.to)
    if (!target) {
      const event = runtimeEventLog.append({
        kind: "router_message_denied",
        actor: message.from,
        target: message.to,
        requestId: message.id,
        decision: "deny",
        reason: `target ${message.to} not found`,
      })
      return {
        id: createRuntimeId("rsp"),
        requestId: message.id,
        from: message.to,
        to: message.from,
        decision: "deny",
        reason: `target ${message.to} not found`,
        eventId: event.id,
      }
    }

    runtimeEventLog.append({
      kind: "router_message_delivered",
      actor: message.from,
      target: message.to,
      requestId: message.id,
      metadata: {
        payloadBytes: message.payload.byteLength,
      },
    })

    return target.post(message)
  }

  private async invokeProviderMessage(
    provider: CapabilityProviderEndpoint,
    message: RuntimeMessage,
  ): Promise<RuntimeResponse> {
    let request: any
    try {
      request = JSON.parse(new TextDecoder().decode(message.payload))
    } catch {
      const event = runtimeEventLog.append({
        kind: "capability_invocation_denied",
        actor: message.from,
        target: provider.appId,
        requestId: message.id,
        decision: "deny",
        reason: "provider message payload is not JSON",
      })
      return {
        id: createRuntimeId("rsp"),
        requestId: message.id,
        from: provider.appId,
        to: message.from,
        decision: "deny",
        reason: "provider message payload is not JSON",
        eventId: event.id,
      }
    }

    if (request.type === "open_session") {
      const opened = await provider.openSession({
        id: message.id,
        requester: message.evidence?.caller ?? { appId: message.from },
        providerAppId: provider.appId,
        capabilityId: provider.descriptor.capabilityId,
        requestedActions: Array.isArray(request.actions) ? request.actions : [],
        evidence: message.evidence ?? { caller: { appId: message.from } },
        createdAt: Date.now(),
      })
      return {
        id: createRuntimeId("rsp"),
        requestId: message.id,
        from: provider.appId,
        to: message.from,
        decision: opened.accepted ? "allow" : "deny",
        reason: opened.reason,
        eventId: opened.eventId,
        payload: new TextEncoder().encode(JSON.stringify(opened)),
      }
    }

    if (request.type === "invoke") {
      const invoked = await provider.invoke({
        id: message.id,
        sessionId: String(request.sessionId ?? ""),
        requester: message.evidence?.caller ?? { appId: message.from },
        providerAppId: provider.appId,
        capabilityId: provider.descriptor.capabilityId,
        action: String(request.action ?? ""),
        resource: request.resource,
        payload: request.payload ? new TextEncoder().encode(JSON.stringify(request.payload)) : undefined,
        evidence: message.evidence ?? { caller: { appId: message.from } },
        createdAt: Date.now(),
      })
      return {
        id: createRuntimeId("rsp"),
        requestId: message.id,
        from: provider.appId,
        to: message.from,
        decision: invoked.decision,
        reason: invoked.reason,
        eventId: invoked.eventId,
        payload: invoked.payload,
      }
    }

    const event = runtimeEventLog.append({
      kind: "capability_invocation_denied",
      actor: message.from,
      target: provider.appId,
      requestId: message.id,
      decision: "deny",
      reason: `unknown provider message type ${request.type}`,
    })
    return {
      id: createRuntimeId("rsp"),
      requestId: message.id,
      from: provider.appId,
      to: message.from,
      decision: "deny",
      reason: `unknown provider message type ${request.type}`,
      eventId: event.id,
    }
  }
}

export const browserMessageRouter = new BrowserMessageRouter()
