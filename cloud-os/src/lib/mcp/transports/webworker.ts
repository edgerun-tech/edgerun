import type { JSONRPCNotification, JSONRPCRequest, JSONRPCResponse } from '../types';

export class WebWorkerTransport {
  private workerScript: string;
  private worker: Worker | null = null;
  private pending = new Map<string | number, (value: JSONRPCResponse) => void>();
  private messageCallback: ((message: JSONRPCResponse | JSONRPCNotification) => void) | null = null;
  private errorCallback: ((error: Error) => void) | null = null;
  private closeCallback: (() => void) | null = null;
  private counter = 1;

  constructor(workerScript: string) {
    this.workerScript = workerScript;
  }

  async connect(): Promise<void> {
    if (this.worker) return;
    this.worker = new Worker(this.workerScript, { type: 'module' });
    this.worker.onmessage = (event: MessageEvent<JSONRPCResponse | JSONRPCNotification>) => {
      const message = event.data;
      if (!message || typeof message !== 'object') return;

      const maybeResponse = message as JSONRPCResponse;
      if (maybeResponse.id !== undefined && this.pending.has(maybeResponse.id)) {
        const resolve = this.pending.get(maybeResponse.id)!;
        this.pending.delete(maybeResponse.id);
        resolve(maybeResponse);
        return;
      }

      this.messageCallback?.(message);
    };
    this.worker.onerror = (event) => {
      this.errorCallback?.(new Error(event.message || 'worker error'));
    };
  }

  async disconnect(): Promise<void> {
    if (!this.worker) return;
    this.worker.terminate();
    this.worker = null;
    this.pending.clear();
    this.closeCallback?.();
  }

  async send(message: JSONRPCRequest | JSONRPCNotification): Promise<JSONRPCResponse> {
    if (!this.worker) throw new Error('worker not connected');

    const request = message as JSONRPCRequest;
    if (request.id === undefined) {
      this.worker.postMessage(message);
      return { jsonrpc: '2.0', id: -1, result: { ok: true } };
    }

    return new Promise<JSONRPCResponse>((resolve) => {
      this.pending.set(request.id, resolve);
      this.worker!.postMessage(message);
      setTimeout(() => {
        if (this.pending.has(request.id)) {
          this.pending.delete(request.id);
          resolve({
            jsonrpc: '2.0',
            id: request.id,
            error: { code: -32000, message: 'request timeout' },
          });
        }
      }, 20000);
    });
  }

  onMessage(callback: (message: JSONRPCResponse | JSONRPCNotification) => void): void {
    this.messageCallback = callback;
  }

  onError(callback: (error: Error) => void): void {
    this.errorCallback = callback;
  }

  onClose(callback: () => void): void {
    this.closeCallback = callback;
  }

  generateId(): number {
    const id = this.counter;
    this.counter += 1;
    return id;
  }
}
