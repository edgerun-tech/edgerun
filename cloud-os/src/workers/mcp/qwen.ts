/**
 * Qwen Code MCP Server
 * Provides Qwen AI tools via MCP protocol
 */

import { MCPServerBase, setupWorkerServer } from './base';

interface QwenToken {
  access_token: string;
  token_type: string;
  refresh_token?: string;
  resource_url: string;
  expiry_date: number;
}

class QwenServer extends MCPServerBase {
  private token: QwenToken | null = null;

  constructor() {
    super('qwen', '1.0.0');
    this.requestToken();
  }

  /**
   * Request OAuth token from main thread
   */
  private requestToken(): void {
    const requestId = Math.random().toString(36).substring(2) + Date.now().toString(36);
    
    const handler = (event: MessageEvent) => {
      if (event.data?.type === 'token:response' && event.data.requestId === requestId) {
        if (event.data.token) {
          try {
            this.token = JSON.parse(event.data.token);
            console.log('[Qwen MCP] Token received');
          } catch {
            console.error('[Qwen MCP] Failed to parse token');
          }
        }
        self.removeEventListener('message', handler);
      }
    };

    self.addEventListener('message', handler);

    self.postMessage({
      type: 'token:request',
      requestId,
      key: 'qwen_token',
    });
  }

  /**
   * Call Qwen API
   */
  private async callQwenAPI(
    model: string,
    messages: Array<{ role: string; content: string }>
  ): Promise<any> {
    if (!this.token) {
      throw new Error('Qwen OAuth token not available');
    }

    const response = await fetch('https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.token.access_token}`,
      },
      body: JSON.stringify({
        model,
        messages,
        max_tokens: 2000,
        stream: false,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Qwen API error: ${error}`);
    }

    return await response.json();
  }

  setupHandlers(): void {
    // qwen_chat tool
    this.registerTool(
      {
        name: 'qwen_chat',
        description: 'Chat with Qwen AI for code assistance, explanations, and analysis',
        inputSchema: {
          type: 'object',
          properties: {
            prompt: {
              type: 'string',
              description: 'Your question or request for Qwen',
            },
            model: {
              type: 'string',
              description: 'Qwen model to use',
              enum: ['qwen-plus', 'qwen-turbo', 'qwen-max', 'qwen3.5-coder-plus'],
              default: 'qwen-plus',
            },
          },
          required: ['prompt'],
        },
      },
      async (args: Record<string, any>) => {
        try {
          const result = await this.callQwenAPI(args.model || 'qwen-plus', [
            { role: 'user', content: args.prompt },
          ]);

          const content = result.choices?.[0]?.message?.content || 'No response from Qwen';

          return {
            content: [{ type: 'text', text: content }],
          };
        } catch (error) {
          return {
            content: [{
              type: 'text',
              text: `Qwen error: ${error instanceof Error ? error.message : 'Unknown error'}`
            }],
            isError: true,
          };
        }
      },
    );

    // qwen_code_review tool
    this.registerTool(
      {
        name: 'qwen_code_review',
        description: 'Review code for issues, improvements, and best practices',
        inputSchema: {
          type: 'object',
          properties: {
            code: {
              type: 'string',
              description: 'Code to review',
            },
            language: {
              type: 'string',
              description: 'Programming language',
            },
            focus: {
              type: 'string',
              description: 'Specific aspect to focus on (performance, security, style, etc.)',
            },
          },
          required: ['code'],
        },
      },
      async (args: Record<string, any>) => {
        try {
          const prompt = [
            'Review this code for issues and improvements:',
            '',
            '```' + (args.language || ''),
            args.code,
            '```',
            '',
            args.focus ? `Focus on: ${args.focus}` : 'Provide general code review feedback.',
          ].join('\n');

          const result = await this.callQwenAPI('qwen-plus', [
            { role: 'user', content: prompt },
          ]);

          const content = result.choices?.[0]?.message?.content || 'No review provided';

          return {
            content: [{ type: 'text', text: content }],
          };
        } catch (error) {
          return {
            content: [{ 
              type: 'text', 
              text: `Code review error: ${error instanceof Error ? error.message : 'Unknown error'}` 
            }],
            isError: true,
          };
        }
      },
    );

    // qwen_explain_code tool
    this.registerTool(
      {
        name: 'qwen_explain_code',
        description: 'Explain what a piece of code does in simple terms',
        inputSchema: {
          type: 'object',
          properties: {
            code: {
              type: 'string',
              description: 'Code to explain',
            },
            language: {
              type: 'string',
              description: 'Programming language',
            },
            level: {
              type: 'string',
              description: 'Explanation level (beginner, intermediate, advanced)',
              enum: ['beginner', 'intermediate', 'advanced'],
              default: 'intermediate',
            },
          },
          required: ['code'],
        },
      },
      async (args: Record<string, any>) => {
        try {
          const prompt = [
            `Explain this code at ${args.level || 'intermediate'} level:`,
            '',
            '```' + (args.language || ''),
            args.code,
            '```',
            '',
            'Include:',
            '- What the code does',
            '- Key concepts used',
            '- How it works step by step',
          ].join('\n');

          const result = await this.callQwenAPI('qwen-plus', [
            { role: 'user', content: prompt },
          ]);

          const content = result.choices?.[0]?.message?.content || 'No explanation provided';

          return {
            content: [{ type: 'text', text: content }],
          };
        } catch (error) {
          return {
            content: [{ 
              type: 'text', 
              text: `Explanation error: ${error instanceof Error ? error.message : 'Unknown error'}` 
            }],
            isError: true,
          };
        }
      },
    );

    console.log('[Qwen MCP] Server initialized with 3 tools');
  }
}

// Start server
setupWorkerServer(QwenServer);
