/**
 * MCP Integration for Qwen Code Browser Client
 * 
 * Supports:
 * - Remote MCP servers (SSE/HTTP)
 * - Browser-based tool registration
 * - Tool execution with proper error handling
 * 
 * Based on @qwen-code/qwen-code MCP architecture
 */

import type { QwenTool } from './client';

/**
 * MCP Tool Definition
 */
export interface BrowserMcpTool {
  name: string;
  description: string;
  inputSchema: {
    type: 'object';
    properties: Record<string, {
      type: string;
      description?: string;
      items?: { type: string };
    }>;
    required?: string[];
  };
  handler: (args: Record<string, unknown>) => Promise<McpToolResult>;
}

/**
 * MCP Tool Result
 */
export interface McpToolResult {
  content: Array<{
    type: 'text' | 'image' | 'resource';
    text?: string;
    data?: string;
    mimeType?: string;
    uri?: string;
  }>;
  isError?: boolean;
}

/**
 * Remote MCP Server Configuration
 */
export interface RemoteMcpServerConfig {
  type: 'sse' | 'http';
  url: string;
  headers?: Record<string, string>;
  timeout?: number;
}

/**
 * Browser MCP Server Configuration
 */
export interface BrowserMcpServerConfig {
  type: 'browser';
  name: string;
  version?: string;
  tools: BrowserMcpTool[];
}

/**
 * Unified MCP Server Config
 */
export type McpServerConfig = RemoteMcpServerConfig | BrowserMcpServerConfig;

/**
 * MCP Server Connection
 */
export class McpServerConnection {
  private config: McpServerConfig;
  private connected: boolean = false;
  private tools: QwenTool[] = [];

  constructor(config: McpServerConfig) {
    this.config = config;
  }

  /**
   * Connect to MCP server
   */
  async connect(): Promise<void> {
    if (this.config.type === 'browser') {
      // Browser-embedded server - tools already available
      this.tools = this.config.tools.map(tool => ({
        type: 'function' as const,
        function: {
          name: tool.name,
          description: tool.description,
          parameters: tool.inputSchema,
        },
      }));
      this.connected = true;
    } else if (this.config.type === 'sse' || this.config.type === 'http') {
      // Remote server - fetch tools list
      await this.connectRemote();
    }
  }

  /**
   * Connect to remote MCP server
   */
  private async connectRemote(): Promise<void> {
    if (this.config.type === 'browser') return;
    
    try {
      const url = this.config.type === 'sse'
        ? `${this.config.url}/sse`
        : this.config.url;

      // Initialize connection
      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(this.config.headers || {}),
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'initialize',
          params: {
            protocolVersion: '2024-11-05',
            capabilities: {},
            clientInfo: {
              name: 'qwen-browser-client',
              version: '1.0.0',
            },
          },
        }),
      });

      if (!response.ok) {
        throw new Error(`MCP server connection failed: ${response.status}`);
      }

      const result = await response.json();
      
      // Get available tools
      const toolsResponse = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...this.config.headers,
        },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 2,
          method: 'tools/list',
          params: {},
        }),
      });

      if (toolsResponse.ok) {
        const toolsResult = await toolsResponse.json();
        this.tools = (toolsResult.result?.tools || []).map((tool: any) => ({
          type: 'function' as const,
          function: {
            name: tool.name,
            description: tool.description,
            parameters: tool.inputSchema,
          },
        }));
      }

      this.connected = true;
    } catch (error) {
      console.error('[MCP] Connection error:', error);
      throw error;
    }
  }

  /**
   * Get available tools
   */
  getTools(): QwenTool[] {
    return this.tools;
  }

  /**
   * Execute a tool
   */
  async executeTool(name: string, args: Record<string, unknown>): Promise<McpToolResult> {
    if (this.config.type === 'browser') {
      // Execute browser-embedded tool
      const tool = this.config.tools.find(t => t.name === name);
      if (!tool) {
        throw new Error(`Tool not found: ${name}`);
      }
      return await tool.handler(args);
    } else {
      // Execute remote tool
      return await this.executeRemoteTool(name, args);
    }
  }

  /**
   * Execute remote tool
   */
  private async executeRemoteTool(
    name: string,
    args: Record<string, unknown>
  ): Promise<McpToolResult> {
    if (this.config.type === 'browser') {
      throw new Error('Cannot call remote method on browser MCP server');
    }
    
    const url = this.config.type === 'sse'
      ? `${this.config.url}/sse`
      : this.config.url;

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(this.config.headers || {}),
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: Date.now(),
        method: 'tools/call',
        params: {
          name,
          arguments: args,
        },
      }),
      signal: AbortSignal.timeout(this.config.timeout || 60000),
    });

    if (!response.ok) {
      throw new Error(`Tool execution failed: ${response.status}`);
    }

    const result = await response.json();
    return result.result as McpToolResult;
  }

  /**
   * Disconnect from server
   */
  async disconnect(): Promise<void> {
    this.connected = false;
    this.tools = [];
  }
}

/**
 * MCP Manager for Qwen Browser Client
 * Manages multiple MCP server connections
 */
export class McpManager {
  private connections: Map<string, McpServerConnection> = new Map();

  /**
   * Add MCP server
   */
  async addServer(name: string, config: McpServerConfig): Promise<void> {
    const connection = new McpServerConnection(config);
    await connection.connect();
    this.connections.set(name, connection);
  }

  /**
   * Remove MCP server
   */
  async removeServer(name: string): Promise<void> {
    const connection = this.connections.get(name);
    if (connection) {
      await connection.disconnect();
      this.connections.delete(name);
    }
  }

  /**
   * Get all available tools from all servers
   */
  getAllTools(): QwenTool[] {
    const tools: QwenTool[] = [];
    for (const [name, connection] of this.connections) {
      tools.push(...connection.getTools().map(tool => ({
        ...tool,
        function: {
          ...tool.function,
          name: `${name}_${tool.function.name}`,
        },
      })));
    }
    return tools;
  }

  /**
   * Execute tool by name
   */
  async executeTool(
    qualifiedName: string,
    args: Record<string, unknown>
  ): Promise<McpToolResult> {
    const [serverName, toolName] = qualifiedName.split('_');
    const connection = this.connections.get(serverName);
    
    if (!connection) {
      throw new Error(`MCP server not found: ${serverName}`);
    }

    return await connection.executeTool(toolName || qualifiedName, args);
  }

  /**
   * Get connected servers
   */
  getConnectedServers(): string[] {
    return Array.from(this.connections.keys());
  }

  /**
   * Clear all connections
   */
  clear(): void {
    this.connections.clear();
  }
}

/**
 * Helper to create browser-embedded MCP tools
 */
export function createBrowserTool<Schema extends Record<string, { type: string }>>(
  name: string,
  description: string,
  inputSchema: {
    type: 'object';
    properties: Schema;
    required?: string[];
  },
  handler: (
    args: { [K in keyof Schema]: Schema[K]['type'] extends 'string' ? string : Schema[K]['type'] extends 'number' ? number : Schema[K]['type'] extends 'boolean' ? boolean : unknown }
  ) => Promise<McpToolResult>
): BrowserMcpTool {
  return {
    name,
    description,
    inputSchema,
    handler: handler as (args: Record<string, unknown>) => Promise<McpToolResult>,
  };
}
