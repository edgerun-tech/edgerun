import type {
  MCPServerConfig,
  MCPTool,
  MCPResource,
  MCPPrompt,
  JSONRPCRequest,
  JSONRPCNotification,
  InitializeRequest,
  InitializeResult,
  MCPToolCall,
  MCPToolResult,
  ToolResponse,
} from './types';
import { WebWorkerTransport } from './transports/webworker';
import { HTTPTransport } from './transports/http';
import { QwenMCPClient } from '../qwen/mcp-client';

/**
 * MCP Client Manager
 * Manages connections to multiple MCP servers
 */
export class MCPClientManager {
  private servers: Map<string, {
    config: MCPServerConfig;
    transport: WebWorkerTransport | HTTPTransport;
    tools: MCPTool[];
    resources: MCPResource[];
    prompts: MCPPrompt[];
    initialized: boolean;
  }> = new Map();

  private toolRegistry: Map<string, { serverId: string; tool: MCPTool }> = new Map();

  // Qwen SDK client for OAuth-based MCP
  private qwenClient: QwenMCPClient | null = null;

  constructor() {
    this.setupTokenRequestHandler();
  }

  private setupTokenRequestHandler(): void {
    if (typeof window === 'undefined') return;

    window.addEventListener('message', (event) => {
      if (event.data?.type === 'token:request') {
        const { requestId, key } = event.data;
        
        // Handle Qwen OAuth token
        let token: string | null = null;
        if (key === 'qwen_token') {
          const tokenData = localStorage.getItem('qwen_token');
          if (tokenData) {
            try {
              const parsed = JSON.parse(tokenData);
              token = parsed.access_token;
            } catch {
              token = null;
            }
          }
        } else {
          token = localStorage.getItem(key);
        }

        window.postMessage({
          type: 'token:response',
          requestId,
          token,
        }, '*');
      }
    });
  }

  /**
   * Connect to an MCP server
   */
  async connectServer(config: MCPServerConfig): Promise<void> {
    if (this.servers.has(config.id)) {
      console.warn(`Server ${config.id} already connected`);
      return;
    }

    if (!config.enabled) {
      console.log(`Server ${config.id} is disabled`);
      return;
    }

    try {
      // Handle Qwen SDK client specially
      if (config.id === 'qwen' && config.auth?.oauthProvider === 'qwen') {
        await this.connectQwenClient(config);
        return;
      }

      // Create transport for other servers
      let transport: WebWorkerTransport | HTTPTransport;

      if (config.type === 'builtin' && config.workerScript) {
        transport = new WebWorkerTransport(config.workerScript);
      } else if (config.type === 'external' && config.url) {
        // External server via HTTP
        transport = new HTTPTransport(config.url);
      } else {
        throw new Error(`Invalid server configuration for ${config.id}`);
      }

      // Connect transport
      await transport.connect();

      // Initialize MCP connection
      const initRequest: InitializeRequest = {
        protocolVersion: '2024-11-05',
        capabilities: {
          roots: { listChanged: true },
          sampling: {},
        },
        clientInfo: {
          name: 'browser-os',
          version: '1.0.0',
        },
      };

      const initResponse = await transport.send({
        jsonrpc: '2.0',
        id: transport.generateId(),
        method: 'initialize',
        params: initRequest,
      });

      const initResult = initResponse.result as InitializeResult;

      // Send initialized notification
      await transport.send({
        jsonrpc: '2.0',
        method: 'notifications/initialized',
      } as JSONRPCNotification);

      // Discover capabilities
      const tools: MCPTool[] = [];
      const resources: MCPResource[] = [];
      const prompts: MCPPrompt[] = [];

      if (initResult.capabilities.tools) {
        const toolsResponse = await transport.send({
          jsonrpc: '2.0',
          id: transport.generateId(),
          method: 'tools/list',
        });

        if (toolsResponse.result?.tools) {
          tools.push(...toolsResponse.result.tools);

          // Register tools
          tools.forEach(tool => {
            this.toolRegistry.set(tool.name, { serverId: config.id, tool });
          });
        }
      }

      if (initResult.capabilities.resources) {
        const resourcesResponse = await transport.send({
          jsonrpc: '2.0',
          id: transport.generateId(),
          method: 'resources/list',
        });

        if (resourcesResponse.result?.resources) {
          resources.push(...resourcesResponse.result.resources);
        }
      }

      if (initResult.capabilities.prompts) {
        const promptsResponse = await transport.send({
          jsonrpc: '2.0',
          id: transport.generateId(),
          method: 'prompts/list',
        });

        if (promptsResponse.result?.prompts) {
          prompts.push(...promptsResponse.result.prompts);
        }
      }

      // Store server info
      this.servers.set(config.id, {
        config,
        transport,
        tools,
        resources,
        prompts,
        initialized: true,
      });

      console.log(`Connected to MCP server: ${config.name} (${tools.length} tools, ${resources.length} resources, ${prompts.length} prompts)`);

    } catch (error) {
      console.error(`Failed to connect to MCP server ${config.id}:`, error);
      throw error;
    }
  }

  /**
   * Connect Qwen SDK client with OAuth
   */
  private async connectQwenClient(config: MCPServerConfig): Promise<void> {
    try {
      // Get OAuth token
      const tokenData = localStorage.getItem('qwen_token');
      if (!tokenData) {
        throw new Error('Qwen OAuth token not found');
      }

      const token = JSON.parse(tokenData);
      if (!token.access_token || Date.now() > token.expiry_date) {
        throw new Error('Qwen OAuth token expired');
      }

      // Create Qwen SDK client
      this.qwenClient = new QwenMCPClient({
        baseUrl: `https://${token.resource_url || 'dashscope.aliyuncs.com'}`,
        oauthToken: token.access_token,
        model: 'qwen-plus',
        debug: false,
      });

      await this.qwenClient.connect();

      // Register Qwen tools
      const qwenTools = this.qwenClient.getTools();
      qwenTools.forEach(tool => {
        this.toolRegistry.set(tool.name, { serverId: config.id, tool });
      });

      // Store as initialized server
      this.servers.set(config.id, {
        config,
        transport: null as any, // Qwen uses different transport
        tools: qwenTools,
        resources: [],
        prompts: [],
        initialized: true,
      });

      console.log(`Connected to Qwen MCP server (${qwenTools.length} tools)`);

    } catch (error) {
      console.error('Failed to connect Qwen MCP client:', error);
      throw error;
    }
  }

  /**
   * Disconnect from an MCP server
   */
  async disconnectServer(serverId: string): Promise<void> {
    const server = this.servers.get(serverId);
    if (!server) return;

    // Unregister tools
    server.tools.forEach(tool => {
      this.toolRegistry.delete(tool.name);
    });

    await server.transport.disconnect();
    this.servers.delete(serverId);
  }

  /**
   * Get all available tools from all connected servers
   */
  getAllTools(): MCPTool[] {
    return Array.from(this.toolRegistry.values()).map(t => t.tool);
  }

  /**
   * Get tools formatted for LLM context
   */
  getToolsForLLM(): Array<{
    name: string;
    description: string;
    parameters: any;
  }> {
    return this.getAllTools().map(tool => ({
      name: tool.name,
      description: tool.description,
      parameters: tool.inputSchema,
    }));
  }

  /**
   * Execute a tool by name
   */
  async executeTool(toolName: string, args: Record<string, any>): Promise<MCPToolResult & { ui?: ToolResponse['ui'] }> {
    const toolInfo = this.toolRegistry.get(toolName);
    if (!toolInfo) {
      throw new Error(`Tool ${toolName} not found`);
    }

    // Handle Qwen SDK client tools
    if (toolInfo.serverId === 'qwen' && this.qwenClient) {
      return await this.qwenClient.executeTool(toolName, args);
    }

    const server = this.servers.get(toolInfo.serverId);
    if (!server) {
      throw new Error(`Server ${toolInfo.serverId} not connected`);
    }

    // Check if server has transport (Qwen doesn't use standard transport)
    if (!server.transport) {
      throw new Error(`Server ${toolInfo.serverId} has no transport`);
    }

    const response = await server.transport.send({
      jsonrpc: '2.0',
      id: server.transport.generateId(),
      method: 'tools/call',
      params: {
        name: toolName,
        arguments: args,
      },
    });

    if (response.error) {
      throw new Error(response.error.message);
    }

    return response.result as MCPToolResult;
  }

  /**
   * Check if a server is connected
   */
  isConnected(serverId: string): boolean {
    return this.servers.has(serverId) && this.servers.get(serverId)!.initialized;
  }

  /**
   * Get server info
   */
  getServerInfo(serverId: string) {
    const server = this.servers.get(serverId);
    if (!server) return null;

    return {
      config: server.config,
      tools: server.tools,
      resources: server.resources,
      prompts: server.prompts,
    };
  }

  /**
   * Get all connected servers
   */
  getConnectedServers(): string[] {
    return Array.from(this.servers.keys());
  }

  /**
   * Disconnect all servers
   */
  async disconnectAll(): Promise<void> {
    await Promise.all(
      Array.from(this.servers.keys()).map(id => this.disconnectServer(id))
    );
  }
}

// Export singleton instance
export const mcpManager = new MCPClientManager();
