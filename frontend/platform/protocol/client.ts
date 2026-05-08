/**
 * Single low-level client for communicating with edgerun-node in this browser.
 * The node owns peer reachability; UI code does not probe native endpoints directly.
 */

import { registerNodeLocator, sendEdgerunNodeProtocol } from "@/platform/runtime/edgerun-node"

export interface NodeRegistration {
  nodeId: string
  nodeTarget: string
  username: string
  registeredAtIso: string
}

export interface ProtocolRequest {
  method: string
  path: string
  body?: Uint8Array
  headers?: Record<string, string>
}

export interface ProtocolResponse {
  status: number
  body: Uint8Array
  headers: Record<string, string>
}

export class ProtocolClient {
  private nodeRegistration: NodeRegistration | null = null
  private peerRegistrationPromise: Promise<void> | null = null

  setNodeRegistration(reg: NodeRegistration | null) {
    this.nodeRegistration = reg
    if (reg) {
      this.peerRegistrationPromise = registerNodeLocator(reg).catch(() => undefined)
    } else {
      this.peerRegistrationPromise = null
    }
  }

  getNodeId(): string | null {
    return this.nodeRegistration?.nodeId ?? null
  }

  getNodeTarget(): string {
    return this.nodeRegistration?.nodeTarget ?? "wasm"
  }

  async send(request: ProtocolRequest): Promise<ProtocolResponse> {
    try {
      if (this.peerRegistrationPromise) {
        await this.peerRegistrationPromise
      }
      return await sendEdgerunNodeProtocol(request)
    } catch (err) {
      throw new ProtocolError("Browser node transport error", err)
    }
  }

  async fetchProto<T>(
    request: ProtocolRequest,
    decode: (bytes: Uint8Array) => T,
  ): Promise<T> {
    const response = await this.send(request)
    if (response.status !== 200) {
      throw new ProtocolError(`Request failed with status ${response.status}`)
    }
    return decode(response.body)
  }

  async postProto<T>(
    request: ProtocolRequest,
    encode: (data: unknown) => Uint8Array,
    decode: (bytes: Uint8Array) => T,
    data: unknown,
  ): Promise<T> {
    const encoded = encode(data)
    const response = await this.send({
      ...request,
      body: encoded,
    })
    if (response.status !== 200) {
      throw new ProtocolError(`Request failed with status ${response.status}`)
    }
    return decode(response.body)
  }
}

export class ProtocolError extends Error {
  constructor(
    message: string,
    public cause?: unknown,
  ) {
    super(message)
    this.name = "ProtocolError"
  }
}

export const protocolClient = new ProtocolClient()
