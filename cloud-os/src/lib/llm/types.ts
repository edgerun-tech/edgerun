/**
 * LLM Provider Types and Interfaces
 */

export type LLMProviderType = 'openai' | 'anthropic' | 'ollama' | 'qwen' | 'custom';

export interface LLMProvider {
  id: string;
  name: string;
  type: LLMProviderType;
  baseUrl: string;
  apiKey?: string;
  defaultModel: string;
  availableModels: string[];
  enabled: boolean;
  priority: number; // Lower = higher priority
}

export interface LLMMessage {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string;
  tool_calls?: LLMToolCall[];
  tool_call_id?: string;
}

export interface LLMToolCall {
  id: string;
  type: 'function';
  function: {
    name: string;
    arguments: string;
  };
}

export interface LLMTool {
  type: 'function';
  function: {
    name: string;
    description: string;
    parameters: any;
  };
}

export interface LLMRequest {
  messages: LLMMessage[];
  model?: string;
  tools?: LLMTool[];
  tool_choice?: 'auto' | 'none' | { type: 'function'; function: { name: string } };
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
}

export interface LLMResponse {
  id: string;
  model: string;
  content: string;
  tool_calls?: LLMToolCall[];
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

export interface LLMStreamChunk {
  id: string;
  model: string;
  delta: {
    content?: string;
    tool_calls?: LLMToolCall[];
  };
  finish_reason?: 'stop' | 'length' | 'tool_calls' | null;
}

// Routing rules
export interface RoutingRule {
  id: string;
  condition: 'always' | 'tool_use' | 'simple_query' | 'privacy_mode';
  providerId: string;
  priority: number;
}
