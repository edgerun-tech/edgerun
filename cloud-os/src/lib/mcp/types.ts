// MCP Protocol Types
// Based on Model Context Protocol specification

export interface JSONRPCRequest {
  jsonrpc: '2.0';
  id: string | number;
  method: string;
  params?: any;
}

export interface JSONRPCResponse {
  jsonrpc: '2.0';
  id: string | number;
  result?: any;
  error?: {
    code: number;
    message: string;
    data?: any;
  };
}

export interface JSONRPCNotification {
  jsonrpc: '2.0';
  method: string;
  params?: any;
}

// MCP Tool Types
export interface MCPTool {
  name: string;
  description: string;
  inputSchema: JSONSchema;
}

export interface MCPToolCall {
  name: string;
  arguments: Record<string, any>;
}

export interface MCPToolResult {
  content: Array<{
    type: 'text' | 'image' | 'resource';
    text?: string;
    data?: string;
    mimeType?: string;
    resource?: MCPResource;
  }>;
  isError?: boolean;
}

// MCP Resource Types
export interface MCPResource {
  uri: string;
  mimeType?: string;
  text?: string;
  blob?: string;
}

export interface MCPResourceTemplate {
  uriTemplate: string;
  name: string;
  mimeType?: string;
  description?: string;
}

// MCP Prompt Types
export interface MCPPrompt {
  name: string;
  description?: string;
  arguments?: Array<{
    name: string;
    description?: string;
    required?: boolean;
  }>;
}

export interface MCPPromptMessage {
  role: 'user' | 'assistant';
  content: {
    type: 'text' | 'image' | 'resource';
    text?: string;
    data?: string;
    mimeType?: string;
    resource?: MCPResource;
  };
}

// MCP Server Capabilities
export interface MCPServerCapabilities {
  tools?: {
    listChanged?: boolean;
  };
  resources?: {
    subscribe?: boolean;
    listChanged?: boolean;
  };
  prompts?: {
    listChanged?: boolean;
  };
  logging?: {};
}

// MCP Client Capabilities
export interface MCPClientCapabilities {
  roots?: {
    listChanged?: boolean;
  };
  sampling?: {};
  experimental?: {};
}

// MCP Server Info
export interface MCPServerInfo {
  name: string;
  version: string;
}

export interface MCPClientInfo {
  name: string;
  version: string;
}

// Initialization
export interface InitializeRequest {
  protocolVersion: string;
  capabilities: MCPClientCapabilities;
  clientInfo: MCPClientInfo;
}

export interface InitializeResult {
  protocolVersion: string;
  capabilities: MCPServerCapabilities;
  serverInfo: MCPServerInfo;
}

// JSON Schema Type
export interface JSONSchema {
  type: string;
  properties?: Record<string, JSONSchema>;
  required?: string[];
  enum?: any[];
  description?: string;
  default?: any;
}

// Transport Types
export interface MCPTransport {
  connect(): Promise<void>;
  disconnect(): Promise<void>;
  send(message: JSONRPCRequest | JSONRPCNotification): Promise<JSONRPCResponse>;
  onMessage(callback: (message: JSONRPCResponse | JSONRPCNotification) => void): void;
  onError(callback: (error: Error) => void): void;
  onClose(callback: () => void): void;
}

// Server Configuration
export interface MCPServerConfig {
  id: string;
  name: string;
  type: 'builtin' | 'external';
  workerScript?: string;
  url?: string;
  auth?: {
    type: 'none' | 'apiKey' | 'oauth';
    apiKey?: string;
    oauthProvider?: 'qwen' | 'google' | 'github';
    tokenKey?: string;
  };
  enabled: boolean;
}

// Error Codes
export const MCPErrorCodes = {
  PARSE_ERROR: -32700,
  INVALID_REQUEST: -32600,
  METHOD_NOT_FOUND: -32601,
  INVALID_PARAMS: -32602,
  INTERNAL_ERROR: -32603,
  SERVER_ERROR_START: -32000,
  SERVER_ERROR_END: -32099,
} as const;

// ============================================
// UI Hints & Morphable Result Types
// For AI-centric CloudOS architecture
// ============================================

export type ViewType = 
  | 'preview'        // Default, simple summaries
  | 'json-tree'      // JSON data, API responses
  | 'table'          // Tabular data, lists
  | 'code-diff'      // Git changes, PR reviews
  | 'file-grid'      // File search results
  | 'log-viewer'     // Terminal output, error logs
  | 'timeline'       // Event sequences, history
  | 'email-reader'   // Gmail conversations
  | 'doc-viewer'     // Documentation, README
  | 'media-gallery'; // Images, videos

export type LayoutType = 
  | 'full'     // Takes full width
  | 'panel'    // Side panel
  | 'inline'   // Embedded in results flow
  | 'modal';   // Modal dialog

export type ActionVariant = 'primary' | 'secondary' | 'danger' | 'ghost';

export interface ToolAction {
  label: string;
  intent: string;  // Natural language for next action
  icon?: string;
  variant?: ActionVariant;
}

export interface UIHints {
  viewType: ViewType;
  layout?: LayoutType;
  title?: string;
  description?: string;
  actions?: ToolAction[];
  metadata?: {
    itemCount?: number;
    duration?: string;
    source?: string;
    timestamp?: string;
    [key: string]: any;
  };
}

export interface ToolResponse {
  // Standard fields
  success: boolean;
  data: any;
  error?: string;
  
  // UI rendering hints
  ui?: UIHints;
}
