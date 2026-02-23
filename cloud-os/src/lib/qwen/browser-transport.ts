/**
 * Browser HTTP Transport for Qwen MCP
 * Uses fetch API with OAuth tokens instead of process spawning
 */

import type { Transport } from './Transport';

export interface BrowserTransportOptions {
  baseUrl: string;
  oauthToken: string;
  sessionId?: string;
  model?: string;
  onMessage?: (message: unknown) => void;
  onError?: (error: Error) => void;
}

export class BrowserTransport implements Transport {
  private baseUrl: string;
  private oauthToken: string;
  private sessionId: string | null = null;
  private model?: string;
  private eventSource: EventSource | null = null;
  private messageQueue: unknown[] = [];
  private resolveMessage: ((value: IteratorResult<unknown>) => void) | null = null;
  private isReady = false;
  private closed = false;
  private exitError: Error | null = null;
  private onMessage?: (message: unknown) => void;
  private onError?: (error: Error) => void;

  constructor(options: BrowserTransportOptions) {
    this.baseUrl = options.baseUrl;
    this.oauthToken = options.oauthToken;
    this.sessionId = options.sessionId || null;
    this.model = options.model;
    this.onMessage = options.onMessage;
    this.onError = options.onError;
  }

  async start(): Promise<void> {
    try {
      // Initialize session
      const initResponse = await fetch(`${this.baseUrl}/api/qwen/session`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.oauthToken}`,
        },
        body: JSON.stringify({
          model: this.model,
          sessionId: this.sessionId || undefined,
        }),
      });

      if (!initResponse.ok) {
        throw new Error(`Failed to initialize session: ${initResponse.statusText}`);
      }

      const data = await initResponse.json();
      this.sessionId = data.sessionId || this.sessionId;

      // Set up SSE for streaming messages
      this.eventSource = new EventSource(
        `${this.baseUrl}/api/qwen/stream?sessionId=${this.sessionId}`,
        {
          headers: {
            'Authorization': `Bearer ${this.oauthToken}`,
          },
        }
      );

      this.eventSource.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);
          this.messageQueue.push(message);
          
          // Resolve pending read if waiting
          if (this.resolveMessage) {
            this.resolveMessage({ value: message, done: false });
            this.resolveMessage = null;
          }

          // Callback for real-time handling
          this.onMessage?.(message);
        } catch (e) {
          const error = e instanceof Error ? e : new Error(String(e));
          this.exitError = error;
          this.onError?.(error);
        }
      };

      this.eventSource.onerror = (error) => {
        console.error('[BrowserTransport] SSE error:', error);
        const err = new Error('SSE connection error');
        this.exitError = err;
        this.onError?.(err);
      };

      this.eventSource.onopen = () => {
        this.isReady = true;
        console.log('[BrowserTransport] Connected to Qwen API');
      };

      // Wait for connection
      await new Promise<void>((resolve, reject) => {
        const timeout = setTimeout(() => {
          reject(new Error('Connection timeout'));
        }, 10000);

        this.eventSource!.onopen = () => {
          clearTimeout(timeout);
          this.isReady = true;
          resolve();
        };

        this.eventSource!.onerror = () => {
          clearTimeout(timeout);
          reject(new Error('Connection failed'));
        };
      });

    } catch (error) {
      this.exitError = error instanceof Error ? error : new Error(String(error));
      this.onError?.(this.exitError);
      throw error;
    }
  }

  async close(): Promise<void> {
    if (this.closed) return;

    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }

    // Close session on server
    if (this.sessionId) {
      try {
        await fetch(`${this.baseUrl}/api/qwen/session/${this.sessionId}`, {
          method: 'DELETE',
          headers: {
            'Authorization': `Bearer ${this.oauthToken}`,
          },
        });
      } catch {}
    }

    this.closed = true;
    this.isReady = false;

    // Resolve any pending reads
    if (this.resolveMessage) {
      this.resolveMessage({ value: undefined, done: true });
      this.resolveMessage = null;
    }
  }

  async waitForExit(): Promise<void> {
    if (this.exitError) {
      throw this.exitError;
    }
    
    if (this.closed) {
      return;
    }

    return new Promise<void>((resolve, reject) => {
      const checkExit = () => {
        if (this.closed) {
          if (this.exitError) {
            reject(this.exitError);
          } else {
            resolve();
          }
        } else {
          setTimeout(checkExit, 100);
        }
      };
      checkExit();
    });
  }

  write(message: string): void {
    if (this.closed) {
      throw new Error('Cannot write to closed transport');
    }

    if (!this.isReady) {
      throw new Error('Transport not ready');
    }

    // Send message to server
    fetch(`${this.baseUrl}/api/qwen/message`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.oauthToken}`,
      },
      body: message,
    }).catch(error => {
      console.error('[BrowserTransport] Write error:', error);
      this.exitError = error instanceof Error ? error : new Error(String(error));
      this.onError?.(this.exitError);
    });
  }

  async *readMessages(): AsyncGenerator<unknown, void, unknown> {
    while (!this.closed) {
      if (this.messageQueue.length > 0) {
        yield this.messageQueue.shift()!;
      } else {
        // Wait for next message
        yield await new Promise<unknown>((resolve) => {
          this.resolveMessage = resolve;
        });
      }
    }
  }

  get isReady(): boolean {
    return this.isReady;
  }

  get exitErrorValue(): Error | null {
    return this.exitError;
  }

  getSessionId(): string | null {
    return this.sessionId;
  }
}
