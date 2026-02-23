/**
 * Intent Execution Engine
 * Executes plans by calling MCP tools and handling results
 */

import type { ExecutionPlan, ExecutionStep, ExecutionResult } from './processor';
import type { ToolResponse, ViewType } from '../mcp/types';
import { mcpManager } from '../mcp/client';
import { addActivity } from '../../components/ActivityFeed';
import { TbOutlineTerminal, TbOutlineGitCommit, TbOutlineCloudUpload } from 'solid-icons/tb';

// View type mapping for common tools
const toolViewTypes: Record<string, ViewType> = {
  'open_window': 'preview',
  'close_window': 'preview',
  'get_context': 'json-tree',
  'send_to_terminal': 'log-viewer',
  'read_file': 'doc-viewer',
  'list_files': 'file-grid',
  'search_files': 'file-grid',
  'get_logs': 'log-viewer',
  'get_emails': 'email-reader',
  'get_calendar_events': 'timeline',
  'get_deployments': 'timeline',
  'get_commits': 'timeline',
  'get_diff': 'code-diff',
  'get_repo_info': 'json-tree',
};

export class IntentExecutor {
  /**
   * Execute a plan
   */
  async execute(plan: ExecutionPlan): Promise<ExecutionResult> {
    const results: ExecutionResult['steps'] = [];
    const responses: ToolResponse[] = [];

    try {
      // Log execution start
      addActivity({
        type: 'command',
        title: 'Executing Command',
        description: plan.intent.raw,
        icon: TbOutlineTerminal,
        color: 'text-green-400',
      });

      for (const step of plan.steps) {
        console.log(`[IntentExecutor] Executing: ${step.tool}`, step.args);

        const result = await this.executeStep(step, plan);

        results.push({
          tool: step.tool,
          success: result.success,
          result: result.data,
          response: result.response,
        });

        if (result.response) {
          responses.push(result.response);
        }

        if (!result.success) {
          addActivity({
            type: 'system',
            title: 'Command Failed',
            description: `Failed at ${step.tool}: ${result.error}`,
            icon: TbOutlineCloudUpload,
            color: 'text-red-400',
          });
          return {
            success: false,
            message: result.error || `Failed to execute ${step.tool}`,
            steps: results,
            responses,
          };
        }
      }

      // Log success
      addActivity({
        type: 'command',
        title: 'Command Completed',
        description: plan.predictedResult,
        icon: TbOutlineGitCommit,
        color: 'text-blue-400',
      });

      return {
        success: true,
        message: plan.predictedResult,
        steps: results,
        responses,
      };
    } catch (error) {
      console.error('Execution error:', error);
      addActivity({
        type: 'system',
        title: 'Execution Error',
        description: error instanceof Error ? error.message : 'Unknown error',
        icon: TbOutlineCloudUpload,
        color: 'text-red-400',
      });
      return {
        success: false,
        message: error instanceof Error ? error.message : 'Execution failed',
        steps: results,
        responses,
      };
    }
  }

  /**
   * Execute a single step and return structured response
   */
  private async executeStep(
    step: ExecutionStep, 
    plan: ExecutionPlan
  ): Promise<{ success: boolean; data?: any; error?: string; response?: ToolResponse }> {
    try {
      // Execute via MCP
      const result = await mcpManager.executeTool(step.tool, step.args);

      if (result.isError) {
        const errorResponse: ToolResponse = {
          success: false,
          data: null,
          error: result.content[0]?.text || 'Tool execution failed',
          ui: {
            viewType: 'preview',
            title: 'Error',
            description: result.content[0]?.text,
            metadata: {
              tool: step.tool,
              timestamp: new Date().toISOString(),
            },
          },
        };
        return {
          success: false,
          error: result.content[0]?.text,
          response: errorResponse,
        };
      }

      // Build structured response with UI hints
      const response = this.buildToolResponse(step.tool, result.content, plan);

      return {
        success: true,
        data: result.content,
        response,
      };
    } catch (error) {
      const errorResponse: ToolResponse = {
        success: false,
        data: null,
        error: error instanceof Error ? error.message : 'Unknown error',
        ui: {
          viewType: 'preview',
          title: 'Execution Error',
          description: error instanceof Error ? error.message : 'Unknown error',
          metadata: {
            tool: step.tool,
            timestamp: new Date().toISOString(),
          },
        },
      };
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
        response: errorResponse,
      };
    }
  }

  /**
   * Build structured ToolResponse with UI hints
   */
  private buildToolResponse(tool: string, data: any, plan: ExecutionPlan): ToolResponse {
    // Determine view type based on tool name
    const viewType = toolViewTypes[tool] || 'preview';

    // Build response based on tool type
    switch (tool) {
      case 'open_window':
        return {
          success: true,
          data,
          ui: {
            viewType: 'preview',
            title: 'Window Opened',
            description: `Opened ${data?.windowId || 'window'}`,
            metadata: {
              windowId: data?.windowId,
              source: 'browser-os',
              timestamp: new Date().toISOString(),
            },
            actions: [
              { label: 'Close', intent: `close ${data?.windowId}`, variant: 'secondary' as const },
            ],
          },
        };

      case 'send_to_terminal':
        return {
          success: true,
          data,
          ui: {
            viewType: 'log-viewer',
            title: 'Terminal Output',
            description: plan.predictedResult,
            metadata: {
              source: 'terminal',
              timestamp: new Date().toISOString(),
            },
          },
        };

      case 'get_context':
        return {
          success: true,
          data,
          ui: {
            viewType: 'json-tree',
            title: 'Current Context',
            description: 'BrowserOS state and configuration',
            metadata: {
              source: 'browser-os',
              timestamp: new Date().toISOString(),
            },
          },
        };

      case 'read_file':
        return {
          success: true,
          data,
          ui: {
            viewType: 'doc-viewer',
            title: data?.path || 'File Content',
            description: `Content of ${data?.path || 'file'}`,
            metadata: {
              source: 'file-system',
              path: data?.path,
              timestamp: new Date().toISOString(),
            },
            actions: [
              { label: 'Edit', intent: `edit ${data?.path}`, variant: 'primary' as const },
            ],
          },
        };

      case 'list_files':
      case 'search_files':
        return {
          success: true,
          data,
          ui: {
            viewType: 'file-grid',
            title: tool === 'search_files' ? 'Search Results' : 'Files',
            description: plan.predictedResult,
            metadata: {
              source: 'file-system',
              itemCount: Array.isArray(data) ? data.length : 0,
              timestamp: new Date().toISOString(),
            },
          },
        };

      case 'get_logs':
        return {
          success: true,
          data,
          ui: {
            viewType: 'log-viewer',
            title: 'Logs',
            description: plan.predictedResult,
            metadata: {
              source: 'logs',
              itemCount: Array.isArray(data) ? data.length : 0,
              timestamp: new Date().toISOString(),
            },
          },
        };

      case 'get_emails':
        return {
          success: true,
          data,
          ui: {
            viewType: 'email-reader',
            title: 'Emails',
            description: plan.predictedResult,
            metadata: {
              source: 'gmail',
              itemCount: Array.isArray(data) ? data.length : 0,
              timestamp: new Date().toISOString(),
            },
          },
        };

      case 'get_calendar_events':
      case 'get_deployments':
      case 'get_commits':
        return {
          success: true,
          data,
          ui: {
            viewType: 'timeline',
            title: this.getEventTitle(tool),
            description: plan.predictedResult,
            metadata: {
              source: tool.replace('get_', ''),
              itemCount: Array.isArray(data) ? data.length : 0,
              timestamp: new Date().toISOString(),
            },
          },
        };

      case 'get_diff':
        return {
          success: true,
          data,
          ui: {
            viewType: 'code-diff',
            title: 'Code Changes',
            description: plan.predictedResult,
            metadata: {
              source: 'github',
              timestamp: new Date().toISOString(),
            },
          },
        };

      default:
        // Generic response
        return {
          success: true,
          data,
          ui: {
            viewType: 'preview',
            title: plan.intent.raw,
            description: plan.predictedResult,
            metadata: {
              tool,
              timestamp: new Date().toISOString(),
            },
          },
        };
    }
  }

  private getEventTitle(tool: string): string {
    const titles: Record<string, string> = {
      'get_calendar_events': 'Calendar Events',
      'get_deployments': 'Deployment History',
      'get_commits': 'Commit History',
    };
    return titles[tool] || 'Events';
  }

  /**
   * Execute a plan with progress callbacks
   */
  async executeWithProgress(
    plan: ExecutionPlan,
    onProgress: (step: ExecutionStep, index: number, total: number) => void
  ): Promise<ExecutionResult> {
    const results: ExecutionResult['steps'] = [];
    const responses: ToolResponse[] = [];

    for (let i = 0; i < plan.steps.length; i++) {
      const step = plan.steps[i];

      onProgress(step, i, plan.steps.length);

      const result = await this.executeStep(step, plan);

      results.push({
        tool: step.tool,
        success: result.success,
        result: result.data,
        response: result.response,
      });

      if (result.response) {
        responses.push(result.response);
      }

      if (!result.success) {
        return {
          success: false,
          message: result.error || `Failed at step ${i + 1}`,
          steps: results,
          responses,
        };
      }
    }

    return {
      success: true,
      message: plan.predictedResult,
      steps: results,
      responses,
    };
  }

  /**
   * Dry run - validate plan without executing
   */
  async dryRun(plan: ExecutionPlan): Promise<ExecutionResult> {
    // Check if all tools exist
    const availableTools = mcpManager.getAllTools();
    const toolNames = new Set(availableTools.map(t => t.name));

    for (const step of plan.steps) {
      if (!toolNames.has(step.tool)) {
        return {
          success: false,
          message: `Tool not found: ${step.tool}`,
          steps: [],
          responses: [],
        };
      }
    }

    return {
      success: true,
      message: `Plan validated: ${plan.steps.length} step(s) ready to execute`,
      steps: plan.steps.map(s => ({
        tool: s.tool,
        success: true,
        result: null,
      })),
      responses: [],
    };
  }
}

// Export singleton
export const intentExecutor = new IntentExecutor();
