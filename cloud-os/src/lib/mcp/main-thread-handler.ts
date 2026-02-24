/**
 * MCP Main Thread Handler
 * Handles tool requests from MCP workers and responds with results
 */

import { context, updateContext, addRecentFile } from '../../stores/context';
import { indexedDBFS } from '../../lib/fs/indexeddb';

interface MessageHandler {
  (params: any, requestId: string): Promise<void>;
}

export class MCPMainThreadHandler {
  private handlers: Map<string, MessageHandler> = new Map();
  private messagePorts: Map<string, MessagePort | Worker> = new Map();

  constructor() {
    this.setupHandlers();
    this.startListening();
  }

  private setupHandlers(): void {
    // Context tools
    this.handlers.set('tool:get_context', this.handleGetContext.bind(this));
    this.handlers.set('tool:set_context', this.handleSetContext.bind(this));

    // File system tools
    this.handlers.set('tool:list_files', this.handleListFiles.bind(this));
    this.handlers.set('tool:read_file', this.handleReadFile.bind(this));
    this.handlers.set('tool:search_files', this.handleSearchFiles.bind(this));
    this.handlers.set('tool:write_file', this.handleWriteFile.bind(this));
    this.handlers.set('tool:create_folder', this.handleCreateFolder.bind(this));

    // Logs tool
    this.handlers.set('tool:get_logs', this.handleGetLogs.bind(this));

    // Terminal tools (frontend-only)
    this.handlers.set('tool:terminal_execute', this.handleTerminalExecute.bind(this));
    this.handlers.set('tool:terminal_status', this.handleTerminalStatus.bind(this));
    this.handlers.set('tool:terminal_list_files', this.handleTerminalListFiles.bind(this));
    this.handlers.set('tool:terminal_read_file', this.handleTerminalReadFile.bind(this));

    // Window management (sends events to UI)
    this.handlers.set('tool:open_window', this.handleOpenWindow.bind(this));
    this.handlers.set('tool:close_window', this.handleCloseWindow.bind(this));
    this.handlers.set('tool:send_to_terminal', this.handleSendToTerminal.bind(this));
  }

  private startListening(): void {
    if (typeof window === 'undefined') return;

    window.addEventListener('message', (event) => {
      const data = event.data;
      if (!data || typeof data !== 'object') return;

      // Handle tool requests from workers
      if (data.type?.startsWith('tool:')) {
        const handler = this.handlers.get(data.type);
        if (handler) {
          handler(data.params || {}, data.requestId).catch(err => {
            console.error(`[MCPMainThread] Handler error for ${data.type}:`, err);
            this.sendResponse(data.type.replace('tool:', '') + ':error', {
              requestId: data.requestId,
              error: err instanceof Error ? err.message : 'Unknown error',
            });
          });
        } else {
          console.warn(`[MCPMainThread] No handler for ${data.type}`);
        }
      }

      // Handle token requests
      if (data.type === 'token:request') {
        this.handleTokenRequest(data);
      }
    });
  }

  private async handleGetContext(params: any, requestId: string): Promise<void> {
    const appContext = {
      currentRepo: context.currentRepo,
      currentBranch: context.currentBranch,
      currentHost: context.currentHost,
      currentProject: context.currentProject,
      recentFiles: context.recentFiles,
      recentCommands: context.recentCommands,
      activeIntegrations: context.activeIntegrations,
      environment: context.environment,
      openWindows: context.openWindows,
    };

    this.sendResponse('context:response', {
      requestId,
      context: appContext,
    });
  }

  private async handleSetContext(params: { key: string; value: string }, requestId: string): Promise<void> {
    const { key, value } = params;

    if (key && value) {
      updateContext({ [key]: value });
    }

    this.sendResponse('context:response', {
      requestId,
      success: true,
      message: `Set ${key} = ${value}`,
    });
  }

  private async handleListFiles(params: { path?: string }, requestId: string): Promise<void> {
    const path = params.path || '/';

    try {
      // Try IndexedDB first
      const files = await indexedDBFS.listFiles(path);
      
      if (files.length > 0) {
        this.sendResponse('files:response', {
          requestId,
          files: files.map(f => ({
            id: f.path,
            name: f.path.split('/').pop() || f.path,
            type: f.type,
            size: f.size,
            modified: new Date(f.modified).toISOString(),
          })),
        });
        return;
      }

      // Fallback: try file system API (Node.js environments)
      try {
        const response = await fetch(`/api/fs/?path=${encodeURIComponent(path)}`);
        
        if (response.ok) {
          const data = await response.json();
          this.sendResponse('files:response', {
            requestId,
            files: data.files || [],
          });
          return;
        }
      } catch {}

      this.sendResponse('files:error', {
        requestId,
        error: 'No filesystem backend available for list_files',
      });
    } catch (error) {
      console.error('[MCPMainThread] list_files error:', error);
      this.sendResponse('files:error', {
        requestId,
        error: error instanceof Error ? error.message : 'Failed to list files',
      });
    }
  }

  private async handleReadFile(params: { path: string }, requestId: string): Promise<void> {
    const path = params.path;

    if (!path) {
      this.sendResponse('file:error', {
        requestId,
        error: 'No path provided',
      });
      return;
    }

    try {
      // Try IndexedDB first
      const content = await indexedDBFS.readFile(path);
      
      if (content !== null) {
        addRecentFile(path);
        this.sendResponse('file:response', {
          requestId,
          content,
          path,
        });
        return;
      }

      // Fallback: try file system API
      try {
        const response = await fetch(`/api/fs/?path=${encodeURIComponent(path)}&action=read`);
        
        if (response.ok) {
          const data = await response.json();
          addRecentFile(path);
          this.sendResponse('file:response', {
            requestId,
            content: data.content || '',
            path,
          });
          return;
        }
      } catch {}

      this.sendResponse('file:error', {
        requestId,
        error: `No filesystem backend available to read ${path}`,
      });
    } catch (error) {
      console.error('[MCPMainThread] read_file error:', error);
      this.sendResponse('file:error', {
        requestId,
        error: error instanceof Error ? error.message : 'Failed to read file',
      });
    }
  }

  private async handleSearchFiles(params: { query: string; path?: string; type?: string; limit?: number }, requestId: string): Promise<void> {
    const { query, path = '/', type = 'name', limit = 20 } = params;

    if (!query) {
      this.sendResponse('search:error', {
        requestId,
        error: 'No query provided',
      });
      return;
    }

    try {
      // IndexedDB search
      const results = await indexedDBFS.searchFiles(query, limit);
      
      if (results.length > 0) {
        this.sendResponse('search:response', {
          requestId,
          results: results.map(f => ({
            name: f.path.split('/').pop() || f.path,
            path: f.path,
            type: 'file',
            size: f.size,
            modified: new Date(f.modified).toISOString(),
          })),
        });
        return;
      }

      // Fallback: try file system API
      try {
        const response = await fetch(`/api/fs/?action=search&query=${encodeURIComponent(query)}&path=${encodeURIComponent(path)}&type=${type}&limit=${limit}`);
        
        if (response.ok) {
          const data = await response.json();
          this.sendResponse('search:response', {
            requestId,
            results: data.results || [],
          });
          return;
        }
      } catch {}

      this.sendResponse('search:error', {
        requestId,
        error: `No filesystem backend available for search_files (path=${path}, type=${type}, limit=${limit})`,
      });
    } catch (error) {
      console.error('[MCPMainThread] search_files error:', error);
      this.sendResponse('search:error', {
        requestId,
        error: error instanceof Error ? error.message : 'Search failed',
      });
    }
  }

  // Handle file write operations
  private async handleWriteFile(params: { path: string; content: string }, requestId: string): Promise<void> {
    const { path, content } = params;

    if (!path) {
      this.sendResponse('file:error', {
        requestId,
        error: 'No path provided',
      });
      return;
    }

    try {
      // Write to IndexedDB
      await indexedDBFS.writeFile(path, content);
      
      this.sendResponse('file:write:response', {
        requestId,
        success: true,
        path,
        size: content.length,
      });
    } catch (error) {
      console.error('[MCPMainThread] write_file error:', error);
      this.sendResponse('file:error', {
        requestId,
        error: error instanceof Error ? error.message : 'Failed to write file',
      });
    }
  }

  // Handle folder creation
  private async handleCreateFolder(params: { path: string }, requestId: string): Promise<void> {
    const { path } = params;

    if (!path) {
      this.sendResponse('folder:error', {
        requestId,
        error: 'No path provided',
      });
      return;
    }

    try {
      await indexedDBFS.createFolder(path);
      
      this.sendResponse('folder:create:response', {
        requestId,
        success: true,
        path,
      });
    } catch (error) {
      console.error('[MCPMainThread] create_folder error:', error);
      this.sendResponse('folder:error', {
        requestId,
        error: error instanceof Error ? error.message : 'Failed to create folder',
      });
    }
  }

  private async handleGetLogs(params: { source?: string; project?: string; limit?: number; level?: string }, requestId: string): Promise<void> {
    const { source = 'local', project, limit = 50, level } = params;

    try {
      this.sendResponse('logs:error', {
        requestId,
        error: `No log provider configured (source=${source}, project=${project || 'n/a'}, limit=${limit}, level=${level || 'n/a'})`,
      });
    } catch (error) {
      console.error('[MCPMainThread] get_logs error:', error);
      this.sendResponse('logs:error', {
        requestId,
        error: error instanceof Error ? error.message : 'Failed to get logs',
      });
    }
  }

  private async handleSendToTerminal(params: { text: string; execute?: boolean }, requestId: string): Promise<void> {
    const { text, execute = false } = params;

    // Dispatch to terminal via custom event
    window.dispatchEvent(new CustomEvent('intent:terminal:input', {
      detail: { text, execute },
    }));

    this.sendResponse('terminal:response', {
      requestId,
      success: true,
      message: execute ? `Executing: ${text}` : `Sent: ${text}`,
    });
  }

  private async handleTerminalExecute(params: { command: string; args?: string[]; cwd?: string }, requestId: string): Promise<void> {
    // Forward to frontend-terminal worker via MCP
    // The worker handles execution and returns result
    // This is a pass-through - actual execution happens in worker
    this.sendResponse('terminal:execute:response', {
      requestId,
      forwarding: true,
      message: 'Command forwarded to frontend-terminal worker',
    });
  }

  private async handleTerminalStatus(params: any, requestId: string): Promise<void> {
    // Return browser capabilities
    const status = {
      connected: false,
      mode: /Chrome/.test(navigator.userAgent || '') && /Google/.test(navigator.vendor || '') 
        ? 'webcontainer-available' 
        : 'unavailable',
      browser: typeof navigator !== 'undefined' ? navigator.userAgent : 'unknown',
      capabilities: ['execute', 'list_files', 'read_file', 'write_file', 'npm', 'git', 'search_files'],
    };

    this.sendResponse('terminal:status:response', {
      requestId,
      status,
    });
  }

  private async handleTerminalListFiles(params: { path?: string }, requestId: string): Promise<void> {
    // Forward to frontend-terminal worker
    this.sendResponse('terminal:list_files:response', {
      requestId,
      forwarding: true,
    });
  }

  private async handleTerminalReadFile(params: { path: string }, requestId: string): Promise<void> {
    // Forward to frontend-terminal worker
    this.sendResponse('terminal:read_file:response', {
      requestId,
      forwarding: true,
    });
  }

  private async handleOpenWindow(params: { windowId: string }, requestId: string): Promise<void> {
    // Handled by IntentBar message listener
    this.sendResponse('open_window:response', {
      requestId,
      success: true,
    });
  }

  private async handleCloseWindow(params: { windowId: string }, requestId: string): Promise<void> {
    // Handled by IntentBar message listener
    this.sendResponse('close_window:response', {
      requestId,
      success: true,
    });
  }

  private handleTokenRequest(data: { requestId: string; key: string }): void {
    const { requestId, key } = data;

    let token: string | null = null;

    // Handle Qwen OAuth token specially
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

    this.sendResponse('token:response', {
      requestId,
      token,
    });
  }

  private sendResponse(type: string, data: any): void {
    if (typeof window === 'undefined') return;

    window.postMessage({ type, ...data }, '*');
  }

  // Register a custom handler
  public registerHandler(type: string, handler: MessageHandler): void {
    this.handlers.set(type, handler);
  }

  // Send message to specific worker
  public sendToWorker(workerId: string, message: any): void {
    const port = this.messagePorts.get(workerId);
    if (port) {
      port.postMessage(message);
    }
  }

  // Register a worker port
  public registerWorker(workerId: string, port: MessagePort | Worker): void {
    this.messagePorts.set(workerId, port);
  }
}

// Export singleton instance
export const mcpMainThreadHandler = new MCPMainThreadHandler();
