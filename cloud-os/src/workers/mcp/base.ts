/**
 * Base MCP Server implementation for Web Workers
 * This runs inside a Web Worker and handles MCP protocol
 */

import type {
  JSONRPCRequest,
  JSONRPCResponse,
  JSONRPCNotification,
  MCPTool,
  MCPResource,
  MCPPrompt,
  InitializeRequest,
  InitializeResult,
  MCPServerCapabilities,
  MCPServerInfo,
  MCPToolResult,
  ToolResponse,
} from '../../lib/mcp/types';

interface ToolHandler {
  (args: Record<string, any>): Promise<MCPToolResult & { ui?: ToolResponse['ui'] }>;
}

interface ResourceHandler {
  (uri: string): Promise<{ contents: string; mimeType?: string }>;
}

export abstract class MCPServerBase {
  private tools: Map<string, { tool: MCPTool; handler: ToolHandler }> = new Map();
  private resources: Map<string, { resource: MCPResource; handler: ResourceHandler }> = new Map();
  private prompts: Map<string, MCPPrompt> = new Map();
  private initialized = false;

  constructor(
    private name: string,
    private version: string
  ) {
    this.setupHandlers();
  }

  /**
   * Register a tool
   */
  protected registerTool(tool: MCPTool, handler: ToolHandler): void {
    this.tools.set(tool.name, { tool, handler });
  }

  /**
   * Register a resource
   */
  protected registerResource(resource: MCPResource, handler: ResourceHandler): void {
    this.resources.set(resource.uri, { resource, handler });
  }

  /**
   * Register a prompt
   */
  protected registerPrompt(prompt: MCPPrompt): void {
    this.prompts.set(prompt.name, prompt);
  }

  /**
   * Abstract method to setup server-specific tools/resources
   */
  abstract setupHandlers(): void;

  /**
   * Get server capabilities
   */
  protected getCapabilities(): MCPServerCapabilities {
    return {
      tools: {
        listChanged: true,
      },
      resources: {
        subscribe: false,
        listChanged: false,
      },
      prompts: {
        listChanged: false,
      },
      logging: {},
    };
  }

  /**
   * Handle incoming messages
   */
  handleMessage(message: JSONRPCRequest | JSONRPCNotification): JSONRPCResponse | null {
    // Handle notifications (no response needed)
    if (!('id' in message) || message.id === undefined) {
      this.handleNotification(message as JSONRPCNotification);
      return null;
    }

    // Handle requests
    try {
      const result = this.handleRequest(message as JSONRPCRequest);
      return {
        jsonrpc: '2.0',
        id: message.id,
        result,
      };
    } catch (error) {
      return {
        jsonrpc: '2.0',
        id: message.id,
        error: {
          code: -32603,
          message: error instanceof Error ? error.message : 'Internal error',
        },
      };
    }
  }

  private handleRequest(request: JSONRPCRequest): any {
    switch (request.method) {
      case 'initialize':
        return this.handleInitialize(request.params as InitializeRequest);
      
      case 'tools/list':
        return this.handleToolsList();
      
      case 'tools/call':
        return this.handleToolCall(request.params);
      
      case 'resources/list':
        return this.handleResourcesList();
      
      case 'resources/read':
        return this.handleResourceRead(request.params);
      
      case 'prompts/list':
        return this.handlePromptsList();
      
      case 'ping':
        return {};
      
      default:
        throw new Error(`Method not found: ${request.method}`);
    }
  }

  private handleNotification(notification: JSONRPCNotification): void {
    switch (notification.method) {
      case 'notifications/initialized':
        this.initialized = true;
        console.log(`[${this.name}] Client initialized`);
        break;
      
      case 'notifications/cancelled':
        // Handle cancellation
        break;
      
      default:
        console.warn(`[${this.name}] Unknown notification: ${notification.method}`);
    }
  }

  private handleInitialize(params: InitializeRequest): InitializeResult {
    console.log(`[${this.name}] Initialize request from ${params.clientInfo.name}`);
    
    return {
      protocolVersion: '2024-11-05',
      capabilities: this.getCapabilities(),
      serverInfo: {
        name: this.name,
        version: this.version,
      },
    };
  }

  private handleToolsList() {
    return {
      tools: Array.from(this.tools.values()).map(t => t.tool),
    };
  }

  private async handleToolCall(params: any): Promise<MCPToolResult> {
    const { name, arguments: args } = params;
    const toolInfo = this.tools.get(name);
    
    if (!toolInfo) {
      throw new Error(`Tool not found: ${name}`);
    }

    return await toolInfo.handler(args || {});
  }

  private handleResourcesList() {
    return {
      resources: Array.from(this.resources.values()).map(r => r.resource),
    };
  }

  private async handleResourceRead(params: any) {
    const { uri } = params;
    const resourceInfo = this.resources.get(uri);
    
    if (!resourceInfo) {
      throw new Error(`Resource not found: ${uri}`);
    }

    return await resourceInfo.handler(uri);
  }

  private handlePromptsList() {
    return {
      prompts: Array.from(this.prompts.values()),
    };
  }
}

// Worker message handling
// This will be imported by specific server implementations
export function setupWorkerServer(ServerClass: new () => MCPServerBase) {
  const server = new ServerClass();

  // Notify main thread that worker is ready
  self.postMessage({ type: 'ready' });

  // Handle messages from main thread
  self.onmessage = async (event) => {
    const message = event.data;
    
    if (!message || typeof message !== 'object') return;
    
    // Handle JSON-RPC messages
    if (message.jsonrpc === '2.0') {
      const response = server.handleMessage(message);
      if (response) {
        self.postMessage(response);
      }
    }
  };
}
