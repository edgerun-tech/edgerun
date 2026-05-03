import { atom } from "nanostores"
import { createRuntimeId, type RuntimeEvent, type RuntimeEventKind } from "./browser-capability-types"

export const runtimeEventLogStore = atom<RuntimeEvent[]>([])

export interface AppendRuntimeEventInput {
  kind: RuntimeEventKind
  actor?: string
  target?: string
  requestId?: string
  sessionId?: string
  capabilityId?: string
  decision?: RuntimeEvent["decision"]
  reason?: string
  inputHash?: string
  outputHash?: string
  metadata?: Record<string, unknown>
}

export class RuntimeEventLog {
  append(input: AppendRuntimeEventInput): RuntimeEvent {
    const event: RuntimeEvent = {
      id: createRuntimeId("evt"),
      createdAt: Date.now(),
      ...input,
    }
    runtimeEventLogStore.set([...runtimeEventLogStore.get(), event])
    return event
  }

  list(): RuntimeEvent[] {
    return runtimeEventLogStore.get()
  }

  clear(): void {
    runtimeEventLogStore.set([])
  }
}

export const runtimeEventLog = new RuntimeEventLog()
