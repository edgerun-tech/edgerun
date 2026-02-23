/**
 * Qwen Code Browser Client
 * Browser-compatible client for Qwen Code API
 * Uses the same OAuth flow and API as @qwen-code/qwen-code CLI
 * 
 * Usage:
 *   const client = new QwenCodeClient({
 *     accessToken: 'your-oauth-token',
 *     baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1'
 *   });
 *   
 *   const response = await client.chat({
 *     model: 'qwen-plus',
 *     messages: [{ role: 'user', content: 'Hello!' }]
 *   });
 */

export interface QwenMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface QwenChatRequest {
  model: string;
  messages: QwenMessage[];
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
  tools?: QwenTool[];
  tool_choice?: 'auto' | 'none' | { type: 'function'; function: { name: string } };
}

export interface QwenTool {
  type: 'function';
  function: {
    name: string;
    description: string;
    parameters: Record<string, unknown>;
  };
}

export interface QwenChatResponse {
  id: string;
  model: string;
  choices: Array<{
    index: number;
    message: QwenMessage;
    finish_reason: string;
  }>;
  usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

export interface QwenToken {
  access_token: string;
  token_type: string;
  refresh_token?: string;
  resource_url: string;
  expiry_date: number;
}

export interface QwenClientConfig {
  accessToken: string;
  baseUrl?: string;
  model?: string;
  timeout?: number;
}

// Qwen OAuth models (from qwen-code CLI constants)
export const QWEN_OAUTH_MODELS = [
  'qwen3.5-coder-plus',
  'qwen3.5-coder-turbo',
  'qwen3-coder-plus',
  'qwen-plus',
  'qwen-turbo',
  'qwen-max',
];

export const DEFAULT_QWEN_MODEL = 'qwen3.5-coder-plus';

export class QwenCodeClient {
  private baseUrl: string;
  private accessToken: string;
  private model: string;
  private timeout: number;

  constructor(config: QwenClientConfig) {
    this.baseUrl = config.baseUrl || 'https://dashscope.aliyuncs.com/compatible-mode/v1';
    this.accessToken = config.accessToken;
    this.model = config.model || 'qwen-plus';
    this.timeout = config.timeout || 60000;
  }

  /**
   * Create a new client from OAuth token object
   */
  static fromToken(token: QwenToken, model?: string): QwenCodeClient {
    return new QwenCodeClient({
      accessToken: token.access_token,
      model: model || 'qwen-plus'
    });
  }

  /**
   * Check if token is valid
   */
  isTokenValid(): boolean {
    return !!this.accessToken && Date.now() < this.getTokenExpiry();
  }

  /**
   * Get token expiry date
   */
  private getTokenExpiry(): number {
    // Default to 1 hour if not specified
    return Date.now() + 3600000;
  }

  /**
   * Make a chat completion request
   */
  async chat(request: QwenChatRequest): Promise<QwenChatResponse> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(`${this.baseUrl}/chat/completions`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.accessToken}`,
        },
        body: JSON.stringify({
          model: request.model || this.model,
          messages: request.messages,
          temperature: request.temperature ?? 0.7,
          max_tokens: request.max_tokens,
          stream: false,
          tools: request.tools,
          tool_choice: request.tool_choice,
        }),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: response.statusText }));
        throw new QwenAPIError(
          `API request failed: ${response.status}`,
          response.status,
          error
        );
      }

      return await response.json();
    } catch (error) {
      clearTimeout(timeoutId);
      if (error instanceof QwenAPIError) throw error;
      if (error instanceof Error && error.name === 'AbortError') {
        throw new QwenAPIError('Request timeout', 408);
      }
      throw new QwenAPIError(
        error instanceof Error ? error.message : 'Unknown error',
        0,
        error
      );
    }
  }

  /**
   * Stream a chat completion request
   */
  async *chatStream(request: QwenChatRequest): AsyncGenerator<string> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(`${this.baseUrl}/chat/completions`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.accessToken}`,
        },
        body: JSON.stringify({
          model: request.model || this.model,
          messages: request.messages,
          temperature: request.temperature ?? 0.7,
          max_tokens: request.max_tokens,
          stream: true,
        }),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        const error = await response.json().catch(() => ({ error: response.statusText }));
        throw new QwenAPIError(
          `API request failed: ${response.status}`,
          response.status,
          error
        );
      }

      if (!response.body) {
        throw new QwenAPIError('No response body', 500);
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();

      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const chunk = decoder.decode(value);
          const lines = chunk.split('\n');

          for (const line of lines) {
            if (line.startsWith('data: ')) {
              const data = line.slice(6);
              if (data === '[DONE]') continue;

              try {
                const parsed = JSON.parse(data);
                const content = parsed.choices?.[0]?.delta?.content || '';
                if (content) {
                  yield content;
                }
              } catch {
                // Skip invalid JSON
              }
            }
          }
        }
      } finally {
        reader.releaseLock();
      }
    } catch (error) {
      clearTimeout(timeoutId);
      throw error;
    }
  }

  /**
   * List available models
   */
  async listModels(): Promise<string[]> {
    try {
      const response = await fetch(`${this.baseUrl}/models`, {
        headers: {
          'Authorization': `Bearer ${this.accessToken}`,
        },
      });

      if (!response.ok) {
        return ['qwen-plus', 'qwen-turbo', 'qwen-max'];
      }

      const data = await response.json();
      return data.data?.map((m: any) => m.id) || ['qwen-plus', 'qwen-turbo', 'qwen-max'];
    } catch {
      return ['qwen-plus', 'qwen-turbo', 'qwen-max'];
    }
  }
}

export class QwenAPIError extends Error {
  constructor(
    message: string,
    public status: number,
    public data?: unknown
  ) {
    super(message);
    this.name = 'QwenAPIError';
  }
}

/**
 * OAuth utilities for Qwen Code
 */
export class QwenOAuth {
  private static readonly AUTH_URL = 'https://portal.qwen.ai/oauth/authorize';
  private static readonly TOKEN_URL = 'https://portal.qwen.ai/oauth/token';

  /**
   * Get OAuth authorization URL
   */
  static getAuthorizationUrl(clientId: string, redirectUri: string, state: string): string {
    const url = new URL(QwenOAuth.AUTH_URL);
    url.searchParams.set('client_id', clientId);
    url.searchParams.set('redirect_uri', redirectUri);
    url.searchParams.set('response_type', 'code');
    url.searchParams.set('scope', 'model:invoke');
    url.searchParams.set('state', state);
    return url.toString();
  }

  /**
   * Exchange authorization code for tokens
   * Note: This should be called from a backend to protect client_secret
   */
  static async exchangeCode(
    clientId: string,
    clientSecret: string,
    code: string,
    redirectUri: string
  ): Promise<QwenToken> {
    const response = await fetch(QwenOAuth.TOKEN_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Accept': 'application/json',
      },
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        client_id: clientId,
        client_secret: clientSecret,
        code: code,
        redirect_uri: redirectUri,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Token exchange failed: ${error}`);
    }

    const data = await response.json();
    return {
      access_token: data.access_token,
      token_type: data.token_type || 'Bearer',
      refresh_token: data.refresh_token,
      resource_url: data.resource_url || 'portal.qwen.ai',
      expiry_date: Date.now() + (data.expires_in || 3600) * 1000,
    };
  }

  /**
   * Refresh access token
   */
  static async refreshAccessToken(
    clientId: string,
    clientSecret: string,
    refreshToken: string
  ): Promise<QwenToken> {
    const response = await fetch(QwenOAuth.TOKEN_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Accept': 'application/json',
      },
      body: new URLSearchParams({
        grant_type: 'refresh_token',
        client_id: clientId,
        client_secret: clientSecret,
        refresh_token: refreshToken,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Token refresh failed: ${error}`);
    }

    const data = await response.json();
    return {
      access_token: data.access_token,
      token_type: data.token_type || 'Bearer',
      refresh_token: data.refresh_token || refreshToken,
      resource_url: data.resource_url || 'portal.qwen.ai',
      expiry_date: Date.now() + (data.expires_in || 3600) * 1000,
    };
  }

  /**
   * Open OAuth popup window
   */
  static openPopup(clientId: string, redirectUri: string): Promise<QwenToken | null> {
    return new Promise((resolve) => {
      const state = Math.random().toString(36).substring(2);
      const width = 500;
      const height = 600;
      const left = window.screenX + (window.outerWidth - width) / 2;
      const top = window.screenY + (window.outerHeight - height) / 2;

      const authUrl = QwenOAuth.getAuthorizationUrl(clientId, redirectUri, state);
      const popup = window.open(
        authUrl,
        'qwen-oauth',
        `width=${width},height=${height},left=${left},top=${top},toolbar=no,menubar=no`
      );

      // Listen for OAuth completion via postMessage
      const handleMessage = (event: MessageEvent) => {
        if (event.origin !== window.location.origin) return;
        if (event.data.type !== 'qwen-oauth-success') return;

        window.removeEventListener('message', handleMessage);
        resolve(event.data.token as QwenToken);
      };

      window.addEventListener('message', handleMessage);

      // Check if popup was closed
      const checkClosed = setInterval(() => {
        if (popup?.closed) {
          clearInterval(checkClosed);
          window.removeEventListener('message', handleMessage);
          resolve(null);
        }
      }, 1000);
    });
  }
}
