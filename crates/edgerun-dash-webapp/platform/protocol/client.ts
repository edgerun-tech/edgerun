/**
 * Single low-level client for communicating with the local EdgeRun node.
 * Handles transport only - no React state, no UI formatting.
 */

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
  private baseUrl: string
  private nodeRegistration: NodeRegistration | null = null

  constructor(baseUrl = "http://127.0.0.1:35630") {
    this.baseUrl = baseUrl
  }

  setNodeRegistration(reg: NodeRegistration | null) {
    this.nodeRegistration = reg
    if (reg) {
      this.baseUrl = `http://${reg.nodeTarget}`
    }
  }

  getNodeId(): string | null {
    return this.nodeRegistration?.nodeId ?? null
  }

  getNodeTarget(): string {
    return this.nodeRegistration?.nodeTarget ?? this.baseUrl.replace("http://", "")
  }

  async send(request: ProtocolRequest): Promise<ProtocolResponse> {
    const url = `${this.baseUrl}${request.path}`
    const headers: Record<string, string> = {
      "Content-Type": "application/octet-stream",
      ...request.headers,
    }

    if (this.nodeRegistration) {
      headers["X-EdgeRun-Node"] = this.nodeRegistration.nodeId
      headers["X-EdgeRun-User"] = this.nodeRegistration.username
    }

    try {
      const response = await fetch(url, {
        method: request.method,
        headers,
        body: request.body ? request.body : undefined,
      })

      const responseBody = new Uint8Array(await response.arrayBuffer())

      return {
        status: response.status,
        body: responseBody,
        headers: Object.fromEntries(response.headers.entries()),
      }
    } catch (err) {
      throw new ProtocolError("Transport error", err)
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
