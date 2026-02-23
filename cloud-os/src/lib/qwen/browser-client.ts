/**
 * Qwen Browser Client
 * Browser-compatible Qwen SDK client using OAuth and fetch API
 * Based on @qwen-code/sdk architecture but adapted for browser environment
 */

import type {
  SDKMessage,
  SDKUserMessage,
  SDKAssistantMessage,
  SDKSystemMessage,
  SDKResultMessage,
} from './types';

export interface QwenBrowserClientOptions {
  baseUrl: string;
  oauthToken: string;
  model?: string;
  sessionId?: string;
  debug?: boolean;
}

export interface QwenMessage {
  type: 'user' | 'assistant' | 'system' | 'result';
  content: string;
  timestamp: number;
}

export class QwenBrowserClient {
  private baseUrl: string;
  private oauthToken: string;
  private model?: string;
  private sessionId: string | null = null;
  private eventSource: EventSource | null = null;
  private messageQueue: SDKMessage[] = [];
  private resolveMessage: ((value: IteratorResult<SDKMessage>) => void) | null = null;
  private isReady = false;
  private closed = false;
  private debug: boolean;

  constructor(options: QwenBrowserClientOptions) {
    this.baseUrl = options.baseUrl;
    this.oauthToken = options.oauthToken;
    this.model = options.model;
    this.sessionId = options.sessionId || null;
    this.debug = options.debug || false;
  }

  /**
   * Initialize connection to Qwen API
   */
  async connect(): Promise<void> {
    try {
      this.log('Connecting to Qwen API...');

      // For browser, we just validate the token exists
      // Session management is handled per-request via /api/qwen/chat
      this.isReady = true;
      this.sessionId = this.sessionId || `browser-${Date.now()}`;
      
      this.log(`Connected with session: ${this.sessionId}`);
    } catch (error) {
      this.log('Connection error:', error);
      throw error;
    }
  }

  /**
   * Send a message to Qwen and get response
   */
  async send(message: string): Promise<void> {
    if (this.closed) {
      throw new Error('Client is closed');
    }

    if (!this.isReady) {
      throw new Error('Client not ready');
    }

    try {
      // Use existing /api/qwen/chat endpoint
      const response = await fetch(`${this.baseUrl}/api/qwen/chat`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          model: this.model || 'qwen-plus',
          messages: [{ role: 'user', content: message }],
          token: this.oauthToken, // Pass token directly
        }),
      });

      if (!response.ok) {
        throw new Error(`API error: ${response.statusText}`);
      }

      const data = await response.json();
      
      // Convert to SDK message format
      const assistantMessage: SDKAssistantMessage = {
        type: 'assistant',
        session_id: this.sessionId!,
        message: {
          role: 'assistant',
          content: data.choices?.[0]?.message?.content || '',
        },
        parent_tool_use_id: null,
      };

      this.messageQueue.push(assistantMessage);
      
      if (this.resolveMessage) {
        this.resolveMessage({ value: assistantMessage, done: false });
        this.resolveMessage = null;
      }

      this.log('Message sent and response received');
    } catch (error) {
      this.log('Send error:', error);
      throw error;
    }
  }

  private setupSSE(): void {
    // SSE not used - we use request/response via /api/qwen/chat
    console.warn('SSE not implemented - using /api/qwen/chat instead');
  }

  private async waitForConnection(): Promise<void> {
    // Connection is immediate for browser client
    return Promise.resolve();
  }

  /**
   * Send a message to Qwen
   */
  async send(message: string): Promise<void> {
    if (this.closed) {
      throw new Error('Client is closed');
    }

    if (!this.isReady) {
      throw new Error('Client not ready');
    }

    const userMessage: SDKUserMessage = {
      type: 'user',
      session_id: this.sessionId!,
      message: { role: 'user', content: message },
      parent_tool_use_id: null,
    };

    try {
      await fetch(`${this.baseUrl}/api/qwen/message`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.oauthToken}`,
        },
        body: JSON.stringify(userMessage),
      });
      this.log('Message sent:', message.substring(0, 50));
    } catch (error) {
      this.log('Send error:', error);
      throw error;
    }
  }

  /**
   * Read messages from Qwen (async iterator)
   */
  async *readMessages(): AsyncGenerator<SDKMessage, void, unknown> {
    while (!this.closed) {
      if (this.messageQueue.length > 0) {
        yield this.messageQueue.shift()!;
      } else {
        // Wait for next message
        const result = await new Promise<SDKMessage>((resolve) => {
          this.resolveMessage = (value) => {
            if (!value.done && value.value) {
              resolve(value.value);
            }
          };
        });
        yield result;
      }
    }
  }

  /**
   * Send a query and get streaming response
   */
  async *query(prompt: string): AsyncGenerator<SDKMessage, void, unknown> {
    await this.send(prompt);

    for await (const message of this.readMessages()) {
      yield message;
      
      // Stop at result message
      if (message.type === 'result') {
        break;
      }
    }
  }

  /**
   * Get the current session ID
   */
  getSessionId(): string | null {
    return this.sessionId;
  }

  /**
   * Check if client is connected
   */
  isConnected(): boolean {
    return this.isReady && !this.closed;
  }

  /**
   * Close the connection
   */
  async close(): Promise<void> {
    if (this.closed) return;

    this.log('Closing connection...');

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
        this.log('Session closed');
      } catch (e) {
        this.log('Close session error:', e);
      }
    }

    this.closed = true;
    this.isReady = false;

    // Resolve any pending reads
    if (this.resolveMessage) {
      this.resolveMessage({ value: undefined, done: true });
      this.resolveMessage = null;
    }
  }

  private log(...args: any[]): void {
    if (this.debug) {
      console.log('[QwenBrowserClient]', ...args);
    }
  }
}

// Type guards for message types
export function isSDKUserMessage(message: SDKMessage): message is SDKUserMessage {
  return message.type === 'user';
}

export function isSDKAssistantMessage(message: SDKMessage): message is SDKAssistantMessage {
  return message.type === 'assistant';
}

export function isSDKSystemMessage(message: SDKMessage): message is SDKSystemMessage {
  return message.type === 'system';
}

export function isSDKResultMessage(message: SDKMessage): message is SDKResultMessage {
  return message.type === 'result';
}
