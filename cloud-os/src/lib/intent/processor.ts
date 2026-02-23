/**
 * Intent Processing System
 * Parses natural language into structured intents and execution plans
 */

import type { LLMRequest, LLMMessage, LLMTool } from '../llm/types';
import type { ToolResponse, ViewType } from '../mcp/types';
import { llmRouter } from '../llm/router';
import { mcpManager } from '../mcp/client';

// Intent Types
export interface ParsedIntent {
  raw: string;
  verb: string;
  target: string;
  modifiers: string[];
  context: AppContext;
  confidence: number;
}

export interface AppContext {
  currentRepo?: string;
  currentBranch?: string;
  currentHost?: string;
  currentProject?: string;
  recentFiles: string[];
  recentCommands: string[];
  activeIntegrations: string[];
  environment: 'dev' | 'staging' | 'prod' | 'unknown';
  openWindows: string[];
}

export interface ExecutionStep {
  tool: string;
  args: Record<string, any>;
  description: string;
}

export interface ExecutionPlan {
  id: string;
  intent: ParsedIntent;
  steps: ExecutionStep[];
  risk: 'low' | 'medium' | 'high' | 'critical';
  preview: PreviewItem[];
  requiresAuth: boolean;
  authProvider?: string;
  estimatedTime?: string;
  predictedResult: string;
}

export interface PreviewItem {
  label: string;
  value: string;
  type: 'info' | 'warning' | 'danger';
}

export interface ExecutionResult {
  success: boolean;
  message: string;
  steps: Array<{
    tool: string;
    success: boolean;
    result: any;
    response?: ToolResponse;  // NEW: Structured response with UI hints
  }>;
  responses: ToolResponse[];  // NEW: All tool responses for rendering
}

/**
 * Intent Processor
 * Uses LLM to parse intents and generate execution plans
 */
export class IntentProcessor {
  /**
   * Process user input into an execution plan
   */
  async process(input: string, context: AppContext): Promise<ExecutionPlan | null> {
    try {
      // Get available tools from MCP
      const mcpTools = mcpManager.getToolsForLLM();
      
      // Convert MCP tools to LLM format
      const llmTools: LLMTool[] = mcpTools.map(tool => ({
        type: 'function',
        function: {
          name: tool.name,
          description: tool.description,
          parameters: tool.parameters,
        },
      }));

      // Build system prompt
      const systemPrompt = this.buildSystemPrompt(context, mcpTools);

      // Create LLM request
      const request: LLMRequest = {
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: input },
        ],
        tools: llmTools,
        tool_choice: 'auto',
        temperature: 0.2, // Lower temperature for more deterministic parsing
      };

      // Call LLM
      const response = await llmRouter.route(request);

      // Parse response
      if (response.tool_calls && response.tool_calls.length > 0) {
        // LLM wants to use tools directly
        return this.createPlanFromToolCalls(input, context, response.tool_calls, llmTools);
      } else {
        // LLM provided natural language response - parse it
        return this.createPlanFromText(input, context, response.content, llmTools);
      }
    } catch (error) {
      console.error('Intent processing error:', error);
      return null;
    }
  }

  /**
   * Build system prompt for intent parsing
   */
  private buildSystemPrompt(context: AppContext, tools: any[]): string {
    return `You are the Intent Processor for browser-os, a unified command system.

Your job is to:
1. Parse the user's natural language input
2. Determine which tools to use
3. Generate a clear execution plan

Current Context:
${JSON.stringify(context, null, 2)}

Available Tools:
${tools.map(t => `- ${t.name}: ${t.description}`).join('\n')}

Rules:
- Always prefer using tools when available
- Infer context from the current state
- For ambiguous requests, ask for clarification in your response
- Be concise and direct
- Return your response in a structured format

Response Format:
{
  "intent": "brief description of what user wants",
  "tools": ["tool1", "tool2"],
  "steps": [
    {"tool": "tool1", "args": {...}, "description": "what this step does"}
  ],
  "risk": "low|medium|high",
  "requiresAuth": true|false,
  "predictedResult": "what will happen after execution"
}`;
  }

  /**
   * Create execution plan from LLM tool calls
   */
  private createPlanFromToolCalls(
    input: string,
    context: AppContext,
    toolCalls: any[],
    availableTools: LLMTool[]
  ): ExecutionPlan {
    const steps: ExecutionStep[] = toolCalls.map(call => {
      const toolName = call.function.name;
      const args = JSON.parse(call.function.arguments);
      
      const tool = availableTools.find(t => t.function.name === toolName);
      
      return {
        tool: toolName,
        args,
        description: tool?.function.description || `Execute ${toolName}`,
      };
    });

    // Calculate risk
    const risk = this.calculateRisk(steps);

    // Generate preview
    const preview = this.generatePreview(steps, context);

    // Check if auth is needed
    const requiresAuth = this.checkAuthRequirement(steps);

    return {
      id: `plan-${Date.now()}`,
      intent: {
        raw: input,
        verb: steps[0]?.tool.split('_')[0] || 'execute',
        target: steps[0]?.args.windowId || steps[0]?.args.path || input,
        modifiers: [],
        context,
        confidence: 0.9,
      },
      steps,
      risk,
      preview,
      requiresAuth,
      predictedResult: this.generatePredictedResult(steps, context),
    };
  }

  /**
   * Create execution plan from text response
   */
  private createPlanFromText(
    input: string,
    context: AppContext,
    text: string,
    availableTools: LLMTool[]
  ): ExecutionPlan | null {
    try {
      // Try to parse as JSON
      const parsed = JSON.parse(text);
      
      if (parsed.steps) {
        return {
          id: `plan-${Date.now()}`,
          intent: {
            raw: input,
            verb: parsed.intent?.split(' ')[0] || 'execute',
            target: input,
            modifiers: [],
            context,
            confidence: 0.7,
          },
          steps: parsed.steps,
          risk: parsed.risk || 'low',
          preview: this.generatePreview(parsed.steps, context),
          requiresAuth: parsed.requiresAuth || false,
          predictedResult: parsed.predictedResult || 'Execution complete',
        };
      }
    } catch {
      // Not JSON - treat as simple command
    }

    // Check for simple commands that map to tools
    const simpleCommands: Record<string, { tool: string; args: any }> = {
      'open terminal': { tool: 'open_window', args: { windowId: 'terminal' } },
      'show files': { tool: 'open_window', args: { windowId: 'files' } },
      'open github': { tool: 'open_window', args: { windowId: 'github' } },
      'open gmail': { tool: 'open_window', args: { windowId: 'gmail' } },
      'show calendar': { tool: 'open_window', args: { windowId: 'calendar' } },
    };

    const normalizedInput = input.toLowerCase().trim();
    
    for (const [pattern, command] of Object.entries(simpleCommands)) {
      if (normalizedInput.includes(pattern)) {
        return {
          id: `plan-${Date.now()}`,
          intent: {
            raw: input,
            verb: 'open',
            target: command.args.windowId,
            modifiers: [],
            context,
            confidence: 0.8,
          },
          steps: [{
            tool: command.tool,
            args: command.args,
            description: `Open ${command.args.windowId}`,
          }],
          risk: 'low',
          preview: [{
            label: 'Action',
            value: `Open ${command.args.windowId} window`,
            type: 'info',
          }],
          requiresAuth: false,
          predictedResult: `${command.args.windowId} window will open`,
        };
      }
    }

    // Fallback: return null to indicate couldn't parse
    return null;
  }

  /**
   * Calculate risk level for a plan
   */
  private calculateRisk(steps: ExecutionStep[]): 'low' | 'medium' | 'high' | 'critical' {
    const dangerousVerbs = ['delete', 'remove', 'drop', 'destroy', 'kill', 'stop'];
    const prodPatterns = ['prod', 'production', 'main', 'master'];

    for (const step of steps) {
      // Check for dangerous operations
      if (dangerousVerbs.some(v => step.tool.includes(v))) {
        return 'high';
      }

      // Check for production targets
      const argsStr = JSON.stringify(step.args).toLowerCase();
      if (prodPatterns.some(p => argsStr.includes(p))) {
        return 'medium';
      }

      // Terminal commands are medium risk
      if (step.tool.includes('terminal') || step.tool.includes('shell')) {
        return 'medium';
      }
    }

    return 'low';
  }

  /**
   * Generate preview items for a plan
   */
  private generatePreview(steps: ExecutionStep[], context: AppContext): PreviewItem[] {
    const preview: PreviewItem[] = [];

    for (const step of steps) {
      preview.push({
        label: 'Action',
        value: step.description,
        type: 'info',
      });

      if (step.args.windowId) {
        preview.push({
          label: 'Window',
          value: step.args.windowId,
          type: 'info',
        });
      }

      if (step.args.path) {
        preview.push({
          label: 'Path',
          value: step.args.path,
          type: 'info',
        });
      }
    }

    // Add context info
    if (context.currentRepo) {
      preview.push({
        label: 'Repository',
        value: context.currentRepo,
        type: 'info',
      });
    }

    if (context.currentHost) {
      preview.push({
        label: 'Host',
        value: context.currentHost,
        type: context.environment === 'prod' ? 'warning' : 'info',
      });
    }

    return preview;
  }

  /**
   * Check if authentication is required
   */
  private checkAuthRequirement(steps: ExecutionStep[]): boolean {
    const authRequiredTools = ['github', 'gmail', 'cloudflare', 'drive'];
    
    return steps.some(step => 
      authRequiredTools.some(tool => step.tool.includes(tool))
    );
  }

  /**
   * Generate predicted result description
   */
  private generatePredictedResult(steps: ExecutionStep[], context: AppContext): string {
    if (steps.length === 0) return 'No actions to perform';

    const step = steps[0];
    
    if (step.tool === 'open_window') {
      return `${step.args.windowId} window will open`;
    }

    if (step.tool === 'send_to_terminal') {
      return `Command "${step.args.text}" will be sent to terminal`;
    }

    if (step.tool.includes('file')) {
      return `File operation on ${step.args.path || 'current directory'}`;
    }

    return `Execute ${steps.length} step(s)`;
  }
}

// Export singleton
export const intentProcessor = new IntentProcessor();
