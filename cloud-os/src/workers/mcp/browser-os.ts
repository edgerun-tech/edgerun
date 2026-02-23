/**
 * BrowserOS MCP Server
 * Exposes browser-os capabilities as MCP tools
 */

import { MCPServerBase, setupWorkerServer } from './base';

class BrowserOSServer extends MCPServerBase {
  constructor() {
    super('browser-os', '1.0.0');
  }

  setupHandlers(): void {
    // Register window management tools
    this.registerTool(
      {
        name: 'open_window',
        description: 'Open a window in browser-os',
        inputSchema: {
          type: 'object',
          properties: {
            windowId: {
              type: 'string',
              description: 'The ID of the window to open',
              enum: ['editor', 'terminal', 'files', 'github', 'gmail', 'drive', 'calendar', 'cloudflare', 'integrations', 'prompt', 'call'],
            },
            title: {
              type: 'string',
              description: 'Optional custom title for the window',
            },
            props: {
              type: 'object',
              description: 'Optional properties to pass to the window',
            },
          },
          required: ['windowId'],
        },
      },
      async (args) => {
        // Send message to main thread to open window
        self.postMessage({
          type: 'tool:open_window',
          params: args,
        });

        return {
          content: [{
            type: 'text',
            text: `Opened window: ${args.windowId}`,
          }],
          // UI hints for morphable result
          ui: {
            viewType: 'preview',
            title: 'Window Opened',
            description: `The ${args.windowId} window is now open`,
            metadata: {
              windowId: args.windowId,
              source: 'browser-os',
            },
            actions: [
              { label: 'Close Window', intent: `close the ${args.windowId} window`, variant: 'secondary' },
            ],
          },
        };
      }
    );

    this.registerTool(
      {
        name: 'close_window',
        description: 'Close a window in browser-os',
        inputSchema: {
          type: 'object',
          properties: {
            windowId: {
              type: 'string',
              description: 'The ID of the window to close',
            },
          },
          required: ['windowId'],
        },
      },
      async (args) => {
        self.postMessage({
          type: 'tool:close_window',
          params: args,
        });

        return {
          content: [{
            type: 'text',
            text: `Closed window: ${args.windowId}`,
          }],
        };
      }
    );

    // Register context tools
    this.registerTool(
      {
        name: 'get_context',
        description: 'Get the current browser-os context including active windows, recent files, and environment',
        inputSchema: {
          type: 'object',
          properties: {},
        },
      },
      async () => {
        // Request context from main thread
        return new Promise((resolve) => {
          const requestId = Date.now().toString();
          
          const handler = (event: MessageEvent) => {
            if (event.data?.type === 'context:response' && event.data?.requestId === requestId) {
              self.removeEventListener('message', handler);
              resolve({
                content: [{
                  type: 'text',
                  text: JSON.stringify(event.data.context, null, 2),
                }],
              });
            }
          };

          self.addEventListener('message', handler);
          
          self.postMessage({
            type: 'tool:get_context',
            requestId,
          });

          // Timeout after 5 seconds
          setTimeout(() => {
            self.removeEventListener('message', handler);
            resolve({
              content: [{
                type: 'text',
                text: 'Failed to get context (timeout)',
              }],
              isError: true,
            });
          }, 5000);
        });
      }
    );

    this.registerTool(
      {
        name: 'set_context',
        description: 'Update the browser-os context',
        inputSchema: {
          type: 'object',
          properties: {
            key: {
              type: 'string',
              description: 'The context key to update',
              enum: ['currentRepo', 'currentBranch', 'currentHost', 'currentProject', 'environment'],
            },
            value: {
              type: 'string',
              description: 'The value to set',
            },
          },
          required: ['key', 'value'],
        },
      },
      async (args) => {
        self.postMessage({
          type: 'tool:set_context',
          params: args,
        });

        return {
          content: [{
            type: 'text',
            text: `Set ${args.key} = ${args.value}`,
          }],
        };
      }
    );

    // Register file system tools
    this.registerTool(
      {
        name: 'list_files',
        description: 'List files in a directory',
        inputSchema: {
          type: 'object',
          properties: {
            path: {
              type: 'string',
              description: 'Directory path (defaults to home)',
              default: '/home',
            },
          },
        },
      },
      async (args) => {
        return new Promise((resolve) => {
          const requestId = Date.now().toString();
          
          const handler = (event: MessageEvent) => {
            if (event.data?.type === 'files:response' && event.data?.requestId === requestId) {
              self.removeEventListener('message', handler);
              resolve({
                content: [{
                  type: 'text',
                  text: JSON.stringify(event.data.files, null, 2),
                }],
              });
            }
          };

          self.addEventListener('message', handler);
          
          self.postMessage({
            type: 'tool:list_files',
            requestId,
            params: args,
          });

          setTimeout(() => {
            self.removeEventListener('message', handler);
            resolve({
              content: [{
                type: 'text',
                text: 'Failed to list files (timeout)',
              }],
              isError: true,
            });
          }, 5000);
        });
      }
    );

    this.registerTool(
      {
        name: 'read_file',
        description: 'Read contents of a file',
        inputSchema: {
          type: 'object',
          properties: {
            path: {
              type: 'string',
              description: 'Path to the file',
            },
          },
          required: ['path'],
        },
      },
      async (args) => {
        return new Promise((resolve) => {
          const requestId = Date.now().toString();
          
          const handler = (event: MessageEvent) => {
            if (event.data?.type === 'file:response' && event.data?.requestId === requestId) {
              self.removeEventListener('message', handler);
              resolve({
                content: [{
                  type: 'text',
                  text: event.data.content || 'File not found',
                }],
              });
            }
          };

          self.addEventListener('message', handler);
          
          self.postMessage({
            type: 'tool:read_file',
            requestId,
            params: args,
          });

          setTimeout(() => {
            self.removeEventListener('message', handler);
            resolve({
              content: [{
                type: 'text',
                text: 'Failed to read file (timeout)',
              }],
              isError: true,
            });
          }, 5000);
        });
      }
    );

    // Register terminal tools
    this.registerTool(
      {
        name: 'send_to_terminal',
        description: 'Send text/command to the terminal',
        inputSchema: {
          type: 'object',
          properties: {
            text: {
              type: 'string',
              description: 'Text or command to send',
            },
            execute: {
              type: 'boolean',
              description: 'Whether to execute the command (press Enter)',
              default: false,
            },
          },
          required: ['text'],
        },
      },
      async (args) => {
        self.postMessage({
          type: 'tool:send_to_terminal',
          params: args,
        });

        return {
          content: [{
            type: 'text',
            text: `Sent to terminal: ${args.text}`,
          }],
        };
      }
    );

    // Register search_files tool
    this.registerTool(
      {
        name: 'search_files',
        description: 'Search for files by name or content in the current project',
        inputSchema: {
          type: 'object',
          properties: {
            query: {
              type: 'string',
              description: 'Search query (filename pattern or content)',
            },
            path: {
              type: 'string',
              description: 'Directory to search in (defaults to project root)',
              default: '/',
            },
            type: {
              type: 'string',
              description: 'Search type: name (filename) or content (file contents)',
              enum: ['name', 'content'],
              default: 'name',
            },
            limit: {
              type: 'number',
              description: 'Maximum number of results',
              default: 20,
            },
          },
          required: ['query'],
        },
      },
      async (args) => {
        return new Promise((resolve) => {
          const requestId = Date.now().toString();

          const handler = (event: MessageEvent) => {
            if (event.data?.type === 'search:response' && event.data?.requestId === requestId) {
              self.removeEventListener('message', handler);
              const results = event.data.results || [];
              resolve({
                content: [{
                  type: 'text',
                  text: `Found ${results.length} file(s):\n` + results.map((r: any) => `- ${r.path}`).join('\n'),
                }],
                ui: {
                  viewType: 'file-grid',
                  title: `Search: ${args.query}`,
                  description: `Found ${results.length} file(s) matching "${args.query}"`,
                  metadata: {
                    source: 'file-system',
                    query: args.query,
                    searchType: args.type,
                    itemCount: results.length,
                    timestamp: new Date().toISOString(),
                  },
                  items: results,
                },
              });
            }
          };

          self.addEventListener('message', handler);

          self.postMessage({
            type: 'tool:search_files',
            requestId,
            params: args,
          });

          setTimeout(() => {
            self.removeEventListener('message', handler);
            resolve({
              content: [{
                type: 'text',
                text: 'Search timed out',
              }],
              isError: true,
            });
          }, 10000);
        });
      }
    );

    // Register get_logs tool
    this.registerTool(
      {
        name: 'get_logs',
        description: 'Get application or deployment logs from various sources',
        inputSchema: {
          type: 'object',
          properties: {
            source: {
              type: 'string',
              description: 'Log source: cloudflare, vercel, or local',
              enum: ['cloudflare', 'vercel', 'local'],
              default: 'local',
            },
            project: {
              type: 'string',
              description: 'Project name or ID',
            },
            limit: {
              type: 'number',
              description: 'Maximum number of log entries',
              default: 50,
            },
            level: {
              type: 'string',
              description: 'Filter by log level',
              enum: ['info', 'warn', 'error', 'debug'],
            },
            startTime: {
              type: 'string',
              description: 'Start time (ISO 8601)',
            },
          },
        },
      },
      async (args) => {
        return new Promise((resolve) => {
          const requestId = Date.now().toString();

          const handler = (event: MessageEvent) => {
            if (event.data?.type === 'logs:response' && event.data?.requestId === requestId) {
              self.removeEventListener('message', handler);
              const logs = event.data.logs || [];
              resolve({
                content: [{
                  type: 'text',
                  text: logs.map((l: any) => `[${l.timestamp || 'N/A'}] ${l.level?.toUpperCase() || 'INFO'}: ${l.message}`).join('\n'),
                }],
                ui: {
                  viewType: 'log-viewer',
                  title: `${args.source || 'Local'} Logs`,
                  description: `Showing ${logs.length} log entries`,
                  metadata: {
                    source: args.source || 'local',
                    project: args.project,
                    itemCount: logs.length,
                    timestamp: new Date().toISOString(),
                  },
                  logs: logs,
                },
              });
            }
          };

          self.addEventListener('message', handler);

          self.postMessage({
            type: 'tool:get_logs',
            requestId,
            params: args,
          });

          setTimeout(() => {
            self.removeEventListener('message', handler);
            resolve({
              content: [{
                type: 'text',
                text: 'Log retrieval timed out',
              }],
              isError: true,
            });
          }, 10000);
        });
      }
    );
  }
}

// Setup the worker
setupWorkerServer(BrowserOSServer);
