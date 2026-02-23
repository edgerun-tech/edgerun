/**
 * Qwen Code Browser Integration
 * 
 * Provides browser-compatible access to Qwen Code with:
 * - OAuth authentication
 * - Session management with localStorage
 * - Chat recording/history
 * - Streaming responses
 * - MCP server support
 * 
 * Based on @qwen-code/qwen-code architecture but adapted for browser context
 */

import { QwenCodeClient, QwenOAuth, type QwenToken, type QwenMessage, type QwenChatRequest, type QwenChatResponse, type QwenTool } from './client';
import { McpManager, type McpServerConfig } from './mcp';

/**
 * Session data stored in localStorage
 */
export interface QwenSession {
  id: string;
  createdAt: number;
  updatedAt: number;
  messages: QwenMessage[];
  token: QwenToken;
  model: string;
  title?: string;
}

/**
 * Session manager for Qwen Code browser sessions
 * Handles persistence, retrieval, and cleanup
 */
export class QwenSessionManager {
  private static readonly STORAGE_KEY = 'qwen_sessions';
  private static readonly CURRENT_SESSION_KEY = 'qwen_current_session';

  /**
   * Get all sessions
   */
  static getSessions(): QwenSession[] {
    try {
      const data = localStorage.getItem(this.STORAGE_KEY);
      if (!data) return [];
      return JSON.parse(data);
    } catch {
      return [];
    }
  }

  /**
   * Get current active session
   */
  static getCurrentSession(): QwenSession | null {
    try {
      const id = localStorage.getItem(this.CURRENT_SESSION_KEY);
      if (!id) return null;
      return this.getSessionById(id);
    } catch {
      return null;
    }
  }

  /**
   * Get session by ID
   */
  static getSessionById(id: string): QwenSession | null {
    const sessions = this.getSessions();
    return sessions.find(s => s.id === id) || null;
  }

  /**
   * Create a new session
   */
  static createSession(token: QwenToken, model: string = 'qwen-plus'): QwenSession {
    const session: QwenSession = {
      id: crypto.randomUUID(),
      createdAt: Date.now(),
      updatedAt: Date.now(),
      messages: [],
      token,
      model,
    };

    this.saveSession(session);
    this.setCurrentSession(session.id);
    return session;
  }

  /**
   * Save session to storage
   */
  static saveSession(session: QwenSession): void {
    const sessions = this.getSessions().filter(s => s.id !== session.id);
    sessions.push(session);
    localStorage.setItem(this.STORAGE_KEY, JSON.stringify(sessions));
  }

  /**
   * Set current session
   */
  static setCurrentSession(id: string): void {
    localStorage.setItem(this.CURRENT_SESSION_KEY, id);
  }

  /**
   * Add message to session
   */
  static addMessage(sessionId: string, message: QwenMessage): QwenSession | null {
    const session = this.getSessionById(sessionId);
    if (!session) return null;

    session.messages.push(message);
    session.updatedAt = Date.now();
    this.saveSession(session);
    return session;
  }

  /**
   * Delete session
   */
  static deleteSession(id: string): boolean {
    const sessions = this.getSessions().filter(s => s.id !== id);
    localStorage.setItem(this.STORAGE_KEY, JSON.stringify(sessions));
    
    const currentId = localStorage.getItem(this.CURRENT_SESSION_KEY);
    if (currentId === id) {
      localStorage.removeItem(this.CURRENT_SESSION_KEY);
    }
    
    return true;
  }

  /**
   * Clear all sessions
   */
  static clearAll(): void {
    localStorage.removeItem(this.STORAGE_KEY);
    localStorage.removeItem(this.CURRENT_SESSION_KEY);
  }
}

/**
 * Browser-compatible Qwen Code client with session management and MCP support
 */
export class QwenBrowserClient {
  private client: QwenCodeClient;
  private session: QwenSession;
  private mcpManager: McpManager;

  constructor(session: QwenSession) {
    this.session = session;
    this.client = QwenCodeClient.fromToken(session.token, session.model);
    this.mcpManager = new McpManager();
  }

  /**
   * Get current session
   */
  getSession(): QwenSession {
    return this.session;
  }

  /**
   * Get MCP manager
   */
  getMcpManager(): McpManager {
    return this.mcpManager;
  }

  /**
   * Add MCP server
   */
  async addMcpServer(name: string, config: McpServerConfig): Promise<void> {
    await this.mcpManager.addServer(name, config);
  }

  /**
   * Send a chat message and get response
   */
  async chat(
    content: string,
    options?: {
      onChunk?: (chunk: string) => void;
      useTools?: boolean;
    }
  ): Promise<string> {
    // Add user message to history
    const userMessage: QwenMessage = { role: 'user', content };
    QwenSessionManager.addMessage(this.session.id, userMessage);

    const messages: QwenMessage[] = [
      ...this.session.messages,
      userMessage,
    ];

    // Get MCP tools if enabled
    let tools: QwenTool[] | undefined;
    if (options?.useTools) {
      tools = this.mcpManager.getAllTools();
    }

    let fullResponse = '';

    if (options?.onChunk) {
      // Streaming
      for await (const chunk of this.client.chatStream({
        model: this.session.model,
        messages,
        ...(tools && tools.length > 0 ? { tools } : {}),
      })) {
        fullResponse += chunk;
        options.onChunk(chunk);
      }
    } else {
      // Non-streaming
      const response = await this.client.chat({
        model: this.session.model,
        messages,
        ...(tools && tools.length > 0 ? { tools } : {}),
      });
      fullResponse = response.choices[0]?.message?.content || '';

      // Handle tool calls if any
      const message = response.choices[0]?.message as any;
      const toolCalls = message?.tool_calls;
      if (toolCalls && Array.isArray(toolCalls) && toolCalls.length > 0) {
        for (const toolCall of toolCalls) {
          const result = await this.mcpManager.executeTool(
            toolCall.function.name,
            JSON.parse(toolCall.function.arguments)
          );

          // Add tool result to messages and continue
          const toolMessage: QwenMessage = {
            role: 'user',
            content: JSON.stringify(result.content),
          };
          QwenSessionManager.addMessage(this.session.id, toolMessage);
        }
      }
    }

    // Add assistant response to history
    const assistantMessage: QwenMessage = { role: 'assistant', content: fullResponse };
    QwenSessionManager.addMessage(this.session.id, assistantMessage);

    // Update local session reference
    this.session = QwenSessionManager.getSessionById(this.session.id)!;

    return fullResponse;
  }

  /**
   * Clear conversation history
   */
  clearHistory(): void {
    this.session.messages = [];
    this.session.updatedAt = Date.now();
    QwenSessionManager.saveSession(this.session);
    this.client = QwenCodeClient.fromToken(this.session.token, this.session.model);
  }

  /**
   * Refresh OAuth token
   */
  async refreshToken(): Promise<boolean> {
    if (!this.session.token.refresh_token) return false;

    try {
      const response = await fetch('/api/qwen/token', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          grant_type: 'refresh_token',
          refresh_token: this.session.token.refresh_token,
        }),
      });

      if (!response.ok) return false;

      const newToken: QwenToken = await response.json();
      this.session.token = newToken;
      QwenSessionManager.saveSession(this.session);
      this.client = QwenCodeClient.fromToken(newToken, this.session.model);
      
      return true;
    } catch {
      return false;
    }
  }
}

/**
 * Initialize Qwen Code browser client
 * Creates new session or uses existing one
 */
export async function createQwenBrowserClient(
  options?: {
    model?: string;
    newSession?: boolean;
  }
): Promise<QwenBrowserClient> {
  // Check for existing session
  if (!options?.newSession) {
    const current = QwenSessionManager.getCurrentSession();
    if (current) {
      // Check if token is still valid
      if (Date.now() < current.token.expiry_date) {
        return new QwenBrowserClient(current);
      }
      
      // Try to refresh token
      const refreshed = await new QwenBrowserClient(current).refreshToken();
      if (refreshed) {
        const refreshedSession = QwenSessionManager.getCurrentSession()!;
        return new QwenBrowserClient(refreshedSession);
      }
    }
  }

  // Start OAuth flow for new session
  const token = await QwenOAuth.openPopup(
    import.meta.env.QWEN_CLIENT_ID || '',
    `${window.location.origin}/api/qwen/callback`
  );

  if (!token) {
    throw new Error('OAuth authentication failed or was cancelled');
  }

  const session = QwenSessionManager.createSession(token, options?.model || 'qwen-plus');
  return new QwenBrowserClient(session);
}

// Re-export types and utilities
export { QwenCodeClient, QwenOAuth };
export type { QwenToken, QwenMessage, QwenChatRequest, QwenChatResponse };
