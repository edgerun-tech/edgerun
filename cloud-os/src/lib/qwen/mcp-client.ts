/**
 * Qwen MCP Client for Browser
 * Connects to Qwen MCP server using OAuth authentication
 */

import { QwenBrowserClient } from './browser-client';
import type { MCPTool, MCPServerConfig } from '../mcp/types';

export interface QwenMCPClientOptions {
  baseUrl: string;
  oauthToken: string;
  model?: string;
  debug?: boolean;
}

export class QwenMCPClient {
  private client: QwenBrowserClient | null = null;
  private tools: Map<string, MCPTool> = new Map();
  private options: QwenMCPClientOptions;
  private connected = false;

  constructor(options: QwenMCPClientOptions) {
    this.options = options;
  }

  /**
   * Connect to Qwen MCP server
   */
  async connect(): Promise<void> {
    if (this.connected) return;

    this.client = new QwenBrowserClient({
      baseUrl: this.options.baseUrl,
      oauthToken: this.options.oauthToken,
      model: this.options.model,
      debug: this.options.debug,
    });

    await this.client.connect();
    this.connected = true;

    // Discover available tools
    await this.discoverTools();
  }

  /**
   * Discover available MCP tools from Qwen
   */
  private async discoverTools(): Promise<void> {
    if (!this.client) return;

    try {
      // Request tools list via MCP protocol
      await this.client.send(JSON.stringify({
        jsonrpc: '2.0',
        id: 'tools-list-1',
        method: 'tools/list',
        params: {},
      }));

      // Wait for response (simplified - in production use proper message handling)
      setTimeout(() => {
        // Register default Qwen tools
        this.registerDefaultTools();
      }, 1000);
    } catch (error) {
      console.error('[QwenMCPClient] Tool discovery error:', error);
    }
  }

  private registerDefaultTools(): void {
    // Qwen Code tools
    const qwenTools: MCPTool[] = [
      {
        name: 'qwen_chat',
        description: 'Chat with Qwen AI assistant',
        parameters: {
          type: 'object',
          properties: {
            message: {
              type: 'string',
              description: 'Message to send to Qwen',
            },
          },
          required: ['message'],
        },
      },
      {
        name: 'qwen_code',
        description: 'Get code assistance from Qwen',
        parameters: {
          type: 'object',
          properties: {
            code: {
              type: 'string',
              description: 'Code to analyze',
            },
            task: {
              type: 'string',
              description: 'Task to perform (explain, refactor, debug, etc)',
            },
          },
          required: ['code', 'task'],
        },
      },
      {
        name: 'qwen_analyze',
        description: 'Analyze files or code with Qwen',
        parameters: {
          type: 'object',
          properties: {
            path: {
              type: 'string',
              description: 'File or directory path',
            },
            analysis: {
              type: 'string',
              description: 'Type of analysis (security, performance, style)',
            },
          },
          required: ['path', 'analysis'],
        },
      },
    ];

    qwenTools.forEach(tool => {
      this.tools.set(tool.name, tool);
    });

    console.log('[QwenMCPClient] Registered tools:', qwenTools.length);
  }

  /**
   * Execute a tool via Qwen MCP
   */
  async executeTool(toolName: string, args: Record<string, any>): Promise<any> {
    if (!this.connected || !this.client) {
      throw new Error('Not connected to Qwen MCP server');
    }

    try {
      // Send tool call via MCP protocol
      const toolCall = {
        jsonrpc: '2.0' as const,
        id: `tool-${toolName}-${Date.now()}`,
        method: 'tools/call',
        params: {
          name: toolName,
          arguments: args,
        },
      };

      await this.client.send(JSON.stringify(toolCall));

      // Wait for response (simplified)
      return {
        success: true,
        content: [{ type: 'text' as const, text: `Tool ${toolName} executed` }],
      };
    } catch (error) {
      console.error('[QwenMCPClient] Tool execution error:', error);
      throw error;
    }
  }

  /**
   * Send a chat message to Qwen
   */
  async chat(message: string): Promise<string> {
    if (!this.client) {
      throw new Error('Not connected');
    }

    let response = '';
    
    for await (const msg of this.client.query(message)) {
      if (msg.type === 'assistant') {
        response += msg.message.content;
      }
    }

    return response;
  }

  /**
   * Get available tools
   */
  getTools(): MCPTool[] {
    return Array.from(this.tools.values());
  }

  /**
   * Check connection status
   */
  isConnected(): boolean {
    return this.connected && this.client?.isConnected() === true;
  }

  /**
   * Close connection
   */
  async close(): Promise<void> {
    if (this.client) {
      await this.client.close();
      this.client = null;
    }
    this.connected = false;
    this.tools.clear();
  }

  /**
   * Get session ID
   */
  getSessionId(): string | null {
    return this.client?.getSessionId() || null;
  }
}

/**
 * Create and configure Qwen MCP client from OAuth token
 */
export async function createQwenMCPClient(
  tokenData: { access_token: string; resource_url: string },
  options?: Partial<QwenMCPClientOptions>
): Promise<QwenMCPClient> {
  const client = new QwenMCPClient({
    baseUrl: `https://${tokenData.resource_url}`,
    oauthToken: tokenData.access_token,
    model: 'qwen-plus',
    debug: false,
    ...options,
  });

  await client.connect();
  return client;
}
