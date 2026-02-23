/**
 * Qwen Browser Client Types
 * Based on @qwen-code/sdk protocol types
 */

export interface SDKUserMessage {
  type: 'user';
  session_id: string;
  message: {
    role: 'user';
    content: string;
  };
  parent_tool_use_id: string | null;
}

export interface SDKAssistantMessage {
  type: 'assistant';
  session_id: string;
  message: {
    role: 'assistant';
    content: string;
    tool_calls?: Array<{
      id: string;
      type: 'function';
      function: {
        name: string;
        arguments: string;
      };
    }>;
  };
  parent_tool_use_id: string | null;
}

export interface SDKSystemMessage {
  type: 'system';
  session_id: string;
  message: {
    role: 'system';
    content: string;
  };
}

export interface SDKResultMessage {
  type: 'result';
  session_id: string;
  result: {
    success: boolean;
    message?: string;
    error?: string;
  };
}

export interface SDKPartialAssistantMessage {
  type: 'assistant_partial';
  session_id: string;
  delta: {
    content?: string;
    tool_calls?: Array<{
      id: string;
      type: 'function';
      function: {
        name: string;
        arguments: string;
      };
    }>;
  };
}

export type SDKMessage =
  | SDKUserMessage
  | SDKAssistantMessage
  | SDKSystemMessage
  | SDKResultMessage
  | SDKPartialAssistantMessage;

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
    uri?: string;
  }>;
  isError?: boolean;
}

export interface QwenOAuthToken {
  access_token: string;
  refresh_token?: string;
  token_type: string;
  resource_url: string;
  expiry_date: number;
}

export interface QwenSessionConfig {
  model?: string;
  sessionId?: string;
  permissionMode?: 'default' | 'plan' | 'auto-edit' | 'yolo';
  mcpServers?: Record<string, MCPServerConfig>;
}

export interface MCPServerConfig {
  type: 'stdio' | 'sse' | 'http' | 'sdk';
  command?: string;
  args?: string[];
  url?: string;
  httpUrl?: string;
  env?: Record<string, string>;
  // For SDK-embedded servers
  name?: string;
  instance?: any;
}
