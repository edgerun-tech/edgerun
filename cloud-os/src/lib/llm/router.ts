/**
 * LLM Provider Router
 * Routes requests to appropriate LLM provider based on configuration and rules
 */

import type {
  LLMProvider,
  LLMRequest,
  LLMResponse,
  LLMStreamChunk,
  LLMTool,
  RoutingRule,
} from './types';

export class LLMRouter {
  private providers: Map<string, LLMProvider> = new Map();
  private rules: RoutingRule[] = [];

  /**
   * Add a provider
   */
  addProvider(provider: LLMProvider): void {
    this.providers.set(provider.id, provider);
    console.log(`[LLMRouter] Added provider: ${provider.name}`);
  }

  /**
   * Remove a provider
   */
  removeProvider(providerId: string): void {
    this.providers.delete(providerId);
    this.rules = this.rules.filter(r => r.providerId !== providerId);
  }

  /**
   * Add a routing rule
   */
  addRule(rule: RoutingRule): void {
    this.rules.push(rule);
    this.rules.sort((a, b) => a.priority - b.priority);
  }

  /**
   * Get all providers
   */
  getProviders(): LLMProvider[] {
    return Array.from(this.providers.values());
  }

  /**
   * Get enabled providers
   */
  getEnabledProviders(): LLMProvider[] {
    return this.getProviders().filter(p => p.enabled);
  }

  /**
   * Route a request to the appropriate provider
   */
  async route(request: LLMRequest): Promise<LLMResponse> {
    const provider = this.selectProvider(request);
    
    if (!provider) {
      throw new Error('No LLM provider available. Please configure a provider in settings.');
    }

    console.log(`[LLMRouter] Routing to ${provider.name}`);

    return this.callProvider(provider, request);
  }

  /**
   * Stream a request
   */
  async *stream(request: LLMRequest): AsyncGenerator<LLMStreamChunk> {
    const provider = this.selectProvider(request);
    
    if (!provider) {
      throw new Error('No LLM provider available');
    }

    yield* this.streamProvider(provider, request);
  }

  /**
   * Select the best provider for a request
   */
  private selectProvider(request: LLMRequest): LLMProvider | null {
    const enabledProviders = this.getEnabledProviders();
    
    if (enabledProviders.length === 0) return null;
    if (enabledProviders.length === 1) return enabledProviders[0];

    // Apply routing rules
    for (const rule of this.rules) {
      const provider = this.providers.get(rule.providerId);
      if (!provider || !provider.enabled) continue;

      if (this.matchesRule(rule, request)) {
        return provider;
      }
    }

    // Default: use provider with highest priority (lowest priority number)
    return enabledProviders.sort((a, b) => a.priority - b.priority)[0];
  }

  /**
   * Check if a request matches a routing rule
   */
  private matchesRule(rule: RoutingRule, request: LLMRequest): boolean {
    switch (rule.condition) {
      case 'always':
        return true;
      
      case 'tool_use':
        return !!request.tools && request.tools.length > 0;
      
      case 'simple_query':
        // Simple query = no tools, short message
        return (!request.tools || request.tools.length === 0) && 
               request.messages[request.messages.length - 1]?.content.length < 100;
      
      case 'privacy_mode':
        // Privacy mode = prefer local providers
        return false; // This would check a global privacy setting
      
      default:
        return false;
    }
  }

  /**
   * Call a provider
   */
  private async callProvider(provider: LLMProvider, request: LLMRequest): Promise<LLMResponse> {
    const model = request.model || provider.defaultModel;

    switch (provider.type) {
      case 'openai':
      case 'qwen':
        return this.callQwen(provider, request, model);
      case 'custom':
        return this.callOpenAICompatible(provider, request, model);

      case 'anthropic':
        return this.callAnthropic(provider, request, model);

      case 'ollama':
        return this.callOllama(provider, request, model);

      default:
        throw new Error(`Unsupported provider type: ${provider.type}`);
    }
  }

  /**
   * Call Qwen Portal API (OAuth authenticated) via our proxy
   */
  private async callQwen(
    provider: LLMProvider,
    request: LLMRequest,
    model: string
  ): Promise<LLMResponse> {
    // Get token from localStorage (browser only)
    const tokenStr = typeof localStorage !== 'undefined' ? localStorage.getItem('qwen_token') : null;
    
    // Use our server-side proxy to avoid CORS
    const response = await fetch('/api/qwen/chat', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model,
        messages: request.messages,
        tools: request.tools,
        temperature: request.temperature ?? 0.7,
        max_tokens: request.max_tokens,
        token: tokenStr, // Pass token from localStorage
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Qwen API error: ${error.error?.message || response.statusText}`);
    }

    const data = await response.json();

    return {
      id: data.id,
      model: data.model,
      content: data.choices[0]?.message?.content || '',
      usage: {
        prompt_tokens: data.usage?.prompt_tokens,
        completion_tokens: data.usage?.completion_tokens,
        total_tokens: data.usage?.total_tokens,
      },
    };
  }

  /**
   * Call OpenAI-compatible API
   */
  private async callOpenAICompatible(
    provider: LLMProvider, 
    request: LLMRequest,
    model: string
  ): Promise<LLMResponse> {
    const response = await fetch(`${provider.baseUrl}/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${provider.apiKey}`,
      },
      body: JSON.stringify({
        model,
        messages: request.messages,
        tools: request.tools,
        tool_choice: request.tool_choice,
        temperature: request.temperature ?? 0.7,
        max_tokens: request.max_tokens,
        stream: false,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`LLM API error: ${error}`);
    }

    const data = await response.json();
    
    return {
      id: data.id,
      model: data.model,
      content: data.choices[0]?.message?.content || '',
      tool_calls: data.choices[0]?.message?.tool_calls,
      usage: data.usage,
    };
  }

  /**
   * Call Anthropic API
   */
  private async callAnthropic(
    provider: LLMProvider,
    request: LLMRequest,
    model: string
  ): Promise<LLMResponse> {
    // Convert OpenAI-style messages to Anthropic format
    const systemMessage = request.messages.find(m => m.role === 'system')?.content || '';
    const otherMessages = request.messages.filter(m => m.role !== 'system');

    const response = await fetch(`${provider.baseUrl}/messages`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-api-key': provider.apiKey || '',
        'anthropic-version': '2023-06-01',
      },
      body: JSON.stringify({
        model,
        system: systemMessage,
        messages: otherMessages.map(m => ({
          role: m.role === 'assistant' ? 'assistant' : 'user',
          content: m.content,
        })),
        tools: request.tools?.map(t => ({
          name: t.function.name,
          description: t.function.description,
          input_schema: t.function.parameters,
        })),
        temperature: request.temperature ?? 0.7,
        max_tokens: request.max_tokens || 4096,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Anthropic API error: ${error}`);
    }

    const data = await response.json();

    return {
      id: data.id,
      model: data.model,
      content: data.content[0]?.text || '',
      usage: {
        prompt_tokens: data.usage.input_tokens,
        completion_tokens: data.usage.output_tokens,
        total_tokens: data.usage.input_tokens + data.usage.output_tokens,
      },
    };
  }

  /**
   * Call Ollama API (local)
   */
  private async callOllama(
    provider: LLMProvider,
    request: LLMRequest,
    model: string
  ): Promise<LLMResponse> {
    const response = await fetch(`${provider.baseUrl}/api/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model,
        messages: request.messages,
        tools: request.tools,
        stream: false,
        options: {
          temperature: request.temperature ?? 0.7,
        },
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Ollama API error: ${error}`);
    }

    const data = await response.json();

    return {
      id: `ollama-${Date.now()}`,
      model: data.model,
      content: data.message?.content || '',
      tool_calls: data.message?.tool_calls,
    };
  }

  /**
   * Stream from provider
   */
  private async *streamProvider(
    provider: LLMProvider,
    request: LLMRequest
  ): AsyncGenerator<LLMStreamChunk> {
    // Implementation for streaming
    // This would use ReadableStream and yield chunks as they arrive
    // For now, just yield the full response as a single chunk
    const response = await this.callProvider(provider, request);
    
    yield {
      id: response.id,
      model: response.model,
      delta: {
        content: response.content,
        tool_calls: response.tool_calls,
      },
      finish_reason: 'stop',
    };
  }

  /**
   * Test a provider connection
   */
  async testProvider(providerId: string): Promise<boolean> {
    const provider = this.providers.get(providerId);
    if (!provider) return false;

    try {
      const response = await fetch(`${provider.baseUrl}/models`, {
        headers: provider.apiKey ? {
          'Authorization': `Bearer ${provider.apiKey}`,
        } : {},
      });
      
      return response.ok;
    } catch {
      return false;
    }
  }
}

// Export singleton
export const llmRouter = new LLMRouter();

// Default providers configuration
export const defaultProviders: LLMProvider[] = [
  {
    id: 'ollama-local',
    name: 'Ollama (Local)',
    type: 'ollama',
    baseUrl: 'http://localhost:11434',
    defaultModel: 'llama3.2',
    availableModels: ['llama3.2', 'mistral', 'codellama'],
    enabled: false,
    priority: 1, // Highest priority for local
  },
];
