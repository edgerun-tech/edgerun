/**
 * HTTP Transport for MCP
 * Connects to external MCP servers over HTTP/SSE
 */

import type {
  JSONRPCRequest,
  JSONRPCResponse,
  JSONRPCNotification,
  MCPTransport,
} from '../types';

export class HTTPTransport implements MCPTransport {
  private baseUrl: string;
  private sessionId: string | null = null;
  private messageCallbacks: Array<(message: JSONRPCResponse | JSONRPCNotification) => void> = [];
  private errorCallbacks: Array<(error: Error) => void> = [];
  private closeCallbacks: Array<() => void> = [];
  private messageId = 0;
  private eventSource: EventSource | null = null;
  private connected = false;

  constructor(url: string) {
    this.baseUrl = url;
  }

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        // Initialize session
        fetch(`${this.baseUrl}/initialize`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            protocolVersion: '2024-11-05',
            capabilities: {
              roots: { listChanged: true },
              sampling: {},
            },
            clientInfo: {
              name: 'browser-os',
              version: '1.0.0',
            },
          }),
        })
          .then(async (response) => {
            if (!response.ok) {
              throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const data = await response.json();
            this.sessionId = data.sessionId || null;

            // Set up SSE listener for server messages
            this.eventSource = new EventSource(`${this.baseUrl}/messages?sessionId=${this.sessionId}`);

            this.eventSource.onmessage = (event) => {
              try {
                const message = JSON.parse(event.data);
                this.handleMessage(message);
              } catch (e) {
                console.error('[HTTPTransport] Failed to parse message:', e);
              }
            };

            this.eventSource.onerror = (error) => {
              console.error('[HTTPTransport] SSE error:', error);
              this.errorCallbacks.forEach(cb => cb(new Error('SSE connection error')));
            };

            this.eventSource.onopen = () => {
              this.connected = true;
              resolve();
            };

            this.eventSource.addEventListener('close', () => {
              this.connected = false;
              this.closeCallbacks.forEach(cb => cb());
            });
          })
          .catch(reject);
      } catch (error) {
        reject(error);
      }
    });
  }

  async disconnect(): Promise<void> {
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }

    if (this.sessionId) {
      try {
        await fetch(`${this.baseUrl}/close?sessionId=${this.sessionId}`, {
          method: 'POST',
        });
      } catch {}
    }

    this.sessionId = null;
    this.connected = false;
    this.closeCallbacks.forEach(cb => cb());
  }

  async send(message: JSONRPCRequest | JSONRPCNotification): Promise<JSONRPCResponse> {
    if (!this.connected) {
      throw new Error('Transport not connected');
    }

    // Add session ID to message
    const messageWithSession = {
      ...message,
      sessionId: this.sessionId,
    };

    // If it's a notification, don't wait for response
    if (!('id' in message) || message.id === undefined) {
      await fetch(`${this.baseUrl}/message`, {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          ...(this.sessionId ? { 'X-Session-Id': this.sessionId } : {}),
        },
        body: JSON.stringify(messageWithSession),
      });
      return { jsonrpc: '2.0', id: -1, result: null };
    }

    // For requests, send and wait for response
    const response = await fetch(`${this.baseUrl}/message`, {
      method: 'POST',
      headers: { 
        'Content-Type': 'application/json',
        ...(this.sessionId ? { 'X-Session-Id': this.sessionId } : {}),
      },
      body: JSON.stringify(messageWithSession),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    return await response.json() as JSONRPCResponse;
  }

  onMessage(callback: (message: JSONRPCResponse | JSONRPCNotification) => void): void {
    this.messageCallbacks.push(callback);
  }

  onError(callback: (error: Error) => void): void {
    this.errorCallbacks.push(callback);
  }

  onClose(callback: () => void): void {
    this.closeCallbacks.push(callback);
  }

  private handleMessage(data: any): void {
    // Handle JSON-RPC messages from SSE
    if (data?.jsonrpc === '2.0') {
      this.messageCallbacks.forEach(cb => cb(data));
    }
  }

  generateId(): string {
    return `${Date.now()}-${++this.messageId}`;
  }

  isConnected(): boolean {
    return this.connected;
  }
}
