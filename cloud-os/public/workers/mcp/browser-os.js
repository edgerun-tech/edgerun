var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);

// src/workers/mcp/base.ts
var MCPServerBase = class {
  constructor(name, version) {
    this.name = name;
    this.version = version;
    __publicField(this, "tools", /* @__PURE__ */ new Map());
    __publicField(this, "resources", /* @__PURE__ */ new Map());
    __publicField(this, "prompts", /* @__PURE__ */ new Map());
    __publicField(this, "initialized", false);
    this.setupHandlers();
  }
  /**
   * Register a tool
   */
  registerTool(tool, handler) {
    this.tools.set(tool.name, { tool, handler });
  }
  /**
   * Register a resource
   */
  registerResource(resource, handler) {
    this.resources.set(resource.uri, { resource, handler });
  }
  /**
   * Register a prompt
   */
  registerPrompt(prompt) {
    this.prompts.set(prompt.name, prompt);
  }
  /**
   * Get server capabilities
   */
  getCapabilities() {
    return {
      tools: {
        listChanged: true
      },
      resources: {
        subscribe: false,
        listChanged: false
      },
      prompts: {
        listChanged: false
      },
      logging: {}
    };
  }
  /**
   * Handle incoming messages
   */
  handleMessage(message) {
    if (!("id" in message) || message.id === void 0) {
      this.handleNotification(message);
      return null;
    }
    try {
      const result = this.handleRequest(message);
      return {
        jsonrpc: "2.0",
        id: message.id,
        result
      };
    } catch (error) {
      return {
        jsonrpc: "2.0",
        id: message.id,
        error: {
          code: -32603,
          message: error instanceof Error ? error.message : "Internal error"
        }
      };
    }
  }
  handleRequest(request) {
    switch (request.method) {
      case "initialize":
        return this.handleInitialize(request.params);
      case "tools/list":
        return this.handleToolsList();
      case "tools/call":
        return this.handleToolCall(request.params);
      case "resources/list":
        return this.handleResourcesList();
      case "resources/read":
        return this.handleResourceRead(request.params);
      case "prompts/list":
        return this.handlePromptsList();
      case "ping":
        return {};
      default:
        throw new Error(`Method not found: ${request.method}`);
    }
  }
  handleNotification(notification) {
    switch (notification.method) {
      case "notifications/initialized":
        this.initialized = true;
        console.log(`[${this.name}] Client initialized`);
        break;
      case "notifications/cancelled":
        break;
      default:
        console.warn(`[${this.name}] Unknown notification: ${notification.method}`);
    }
  }
  handleInitialize(params) {
    console.log(`[${this.name}] Initialize request from ${params.clientInfo.name}`);
    return {
      protocolVersion: "2024-11-05",
      capabilities: this.getCapabilities(),
      serverInfo: {
        name: this.name,
        version: this.version
      }
    };
  }
  handleToolsList() {
    return {
      tools: Array.from(this.tools.values()).map((t) => t.tool)
    };
  }
  async handleToolCall(params) {
    const { name, arguments: args } = params;
    const toolInfo = this.tools.get(name);
    if (!toolInfo) {
      throw new Error(`Tool not found: ${name}`);
    }
    return await toolInfo.handler(args || {});
  }
  handleResourcesList() {
    return {
      resources: Array.from(this.resources.values()).map((r) => r.resource)
    };
  }
  async handleResourceRead(params) {
    const { uri } = params;
    const resourceInfo = this.resources.get(uri);
    if (!resourceInfo) {
      throw new Error(`Resource not found: ${uri}`);
    }
    return await resourceInfo.handler(uri);
  }
  handlePromptsList() {
    return {
      prompts: Array.from(this.prompts.values())
    };
  }
};
function setupWorkerServer(ServerClass) {
  const server = new ServerClass();
  self.postMessage({ type: "ready" });
  self.onmessage = async (event) => {
    const message = event.data;
    if (!message || typeof message !== "object") return;
    if (message.jsonrpc === "2.0") {
      const response = server.handleMessage(message);
      if (response) {
        self.postMessage(response);
      }
    }
  };
}

// src/workers/mcp/browser-os.ts
var BrowserOSServer = class extends MCPServerBase {
  constructor() {
    super("browser-os", "1.0.0");
  }
  setupHandlers() {
    this.registerTool(
      {
        name: "open_window",
        description: "Open a window in browser-os",
        inputSchema: {
          type: "object",
          properties: {
            windowId: {
              type: "string",
              description: "The ID of the window to open",
              enum: ["editor", "terminal", "files", "github", "gmail", "drive", "calendar", "cloudflare", "integrations", "prompt", "call"]
            },
            title: {
              type: "string",
              description: "Optional custom title for the window"
            },
            props: {
              type: "object",
              description: "Optional properties to pass to the window"
            }
          },
          required: ["windowId"]
        }
      },
      async (args) => {
        self.postMessage({
          type: "tool:open_window",
          params: args
        });
        return {
          content: [{
            type: "text",
            text: `Opened window: ${args.windowId}`
          }],
          // UI hints for morphable result
          ui: {
            viewType: "preview",
            title: "Window Opened",
            description: `The ${args.windowId} window is now open`,
            metadata: {
              windowId: args.windowId,
              source: "browser-os"
            },
            actions: [
              { label: "Close Window", intent: `close the ${args.windowId} window`, variant: "secondary" }
            ]
          }
        };
      }
    );
    this.registerTool(
      {
        name: "close_window",
        description: "Close a window in browser-os",
        inputSchema: {
          type: "object",
          properties: {
            windowId: {
              type: "string",
              description: "The ID of the window to close"
            }
          },
          required: ["windowId"]
        }
      },
      async (args) => {
        self.postMessage({
          type: "tool:close_window",
          params: args
        });
        return {
          content: [{
            type: "text",
            text: `Closed window: ${args.windowId}`
          }]
        };
      }
    );
    this.registerTool(
      {
        name: "get_context",
        description: "Get the current browser-os context including active windows, recent files, and environment",
        inputSchema: {
          type: "object",
          properties: {}
        }
      },
      async () => {
        return new Promise((resolve) => {
          const requestId = Date.now().toString();
          const handler = (event) => {
            if (event.data?.type === "context:response" && event.data?.requestId === requestId) {
              self.removeEventListener("message", handler);
              resolve({
                content: [{
                  type: "text",
                  text: JSON.stringify(event.data.context, null, 2)
                }]
              });
            }
          };
          self.addEventListener("message", handler);
          self.postMessage({
            type: "tool:get_context",
            requestId
          });
          setTimeout(() => {
            self.removeEventListener("message", handler);
            resolve({
              content: [{
                type: "text",
                text: "Failed to get context (timeout)"
              }],
              isError: true
            });
          }, 5e3);
        });
      }
    );
    this.registerTool(
      {
        name: "set_context",
        description: "Update the browser-os context",
        inputSchema: {
          type: "object",
          properties: {
            key: {
              type: "string",
              description: "The context key to update",
              enum: ["currentRepo", "currentBranch", "currentHost", "currentProject", "environment"]
            },
            value: {
              type: "string",
              description: "The value to set"
            }
          },
          required: ["key", "value"]
        }
      },
      async (args) => {
        self.postMessage({
          type: "tool:set_context",
          params: args
        });
        return {
          content: [{
            type: "text",
            text: `Set ${args.key} = ${args.value}`
          }]
        };
      }
    );
    this.registerTool(
      {
        name: "list_files",
        description: "List files in a directory",
        inputSchema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description: "Directory path (defaults to home)",
              default: "/home"
            }
          }
        }
      },
      async (args) => {
        return new Promise((resolve) => {
          const requestId = Date.now().toString();
          const handler = (event) => {
            if (event.data?.type === "files:response" && event.data?.requestId === requestId) {
              self.removeEventListener("message", handler);
              resolve({
                content: [{
                  type: "text",
                  text: JSON.stringify(event.data.files, null, 2)
                }]
              });
            } else if (event.data?.type === "files:error" && event.data?.requestId === requestId) {
              self.removeEventListener("message", handler);
              resolve({
                content: [{
                  type: "text",
                  text: event.data.error || "Failed to list files"
                }],
                isError: true
              });
            }
          };
          self.addEventListener("message", handler);
          self.postMessage({
            type: "tool:list_files",
            requestId,
            params: args
          });
          setTimeout(() => {
            self.removeEventListener("message", handler);
            resolve({
              content: [{
                type: "text",
                text: "Failed to list files (timeout)"
              }],
              isError: true
            });
          }, 5e3);
        });
      }
    );
    this.registerTool(
      {
        name: "read_file",
        description: "Read contents of a file",
        inputSchema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description: "Path to the file"
            }
          },
          required: ["path"]
        }
      },
      async (args) => {
        return new Promise((resolve) => {
          const requestId = Date.now().toString();
          const handler = (event) => {
            if (event.data?.type === "file:response" && event.data?.requestId === requestId) {
              self.removeEventListener("message", handler);
              resolve({
                content: [{
                  type: "text",
                  text: event.data.content || "File not found"
                }]
              });
            } else if (event.data?.type === "file:error" && event.data?.requestId === requestId) {
              self.removeEventListener("message", handler);
              resolve({
                content: [{
                  type: "text",
                  text: event.data.error || "Failed to read file"
                }],
                isError: true
              });
            }
          };
          self.addEventListener("message", handler);
          self.postMessage({
            type: "tool:read_file",
            requestId,
            params: args
          });
          setTimeout(() => {
            self.removeEventListener("message", handler);
            resolve({
              content: [{
                type: "text",
                text: "Failed to read file (timeout)"
              }],
              isError: true
            });
          }, 5e3);
        });
      }
    );
    this.registerTool(
      {
        name: "send_to_terminal",
        description: "Send text/command to the terminal",
        inputSchema: {
          type: "object",
          properties: {
            text: {
              type: "string",
              description: "Text or command to send"
            },
            execute: {
              type: "boolean",
              description: "Whether to execute the command (press Enter)",
              default: false
            }
          },
          required: ["text"]
        }
      },
      async (args) => {
        self.postMessage({
          type: "tool:send_to_terminal",
          params: args
        });
        return {
          content: [{
            type: "text",
            text: `Sent to terminal: ${args.text}`
          }]
        };
      }
    );
    this.registerTool(
      {
        name: "search_files",
        description: "Search for files by name or content in the current project",
        inputSchema: {
          type: "object",
          properties: {
            query: {
              type: "string",
              description: "Search query (filename pattern or content)"
            },
            path: {
              type: "string",
              description: "Directory to search in (defaults to project root)",
              default: "/"
            },
            type: {
              type: "string",
              description: "Search type: name (filename) or content (file contents)",
              enum: ["name", "content"],
              default: "name"
            },
            limit: {
              type: "number",
              description: "Maximum number of results",
              default: 20
            }
          },
          required: ["query"]
        }
      },
      async (args) => {
        return new Promise((resolve) => {
          const requestId = Date.now().toString();
          const handler = (event) => {
            if (event.data?.type === "search:response" && event.data?.requestId === requestId) {
              self.removeEventListener("message", handler);
              const results = event.data.results || [];
              resolve({
                content: [{
                  type: "text",
                  text: `Found ${results.length} file(s):
` + results.map((r) => `- ${r.path}`).join("\n")
                }],
                ui: {
                  viewType: "file-grid",
                  title: `Search: ${args.query}`,
                  description: `Found ${results.length} file(s) matching "${args.query}"`,
                  metadata: {
                    source: "file-system",
                    query: args.query,
                    searchType: args.type,
                    itemCount: results.length,
                    timestamp: (/* @__PURE__ */ new Date()).toISOString()
                  },
                  items: results
                }
              });
            } else if (event.data?.type === "search:error" && event.data?.requestId === requestId) {
              self.removeEventListener("message", handler);
              resolve({
                content: [{
                  type: "text",
                  text: event.data.error || "Search failed"
                }],
                isError: true
              });
            }
          };
          self.addEventListener("message", handler);
          self.postMessage({
            type: "tool:search_files",
            requestId,
            params: args
          });
          setTimeout(() => {
            self.removeEventListener("message", handler);
            resolve({
              content: [{
                type: "text",
                text: "Search timed out"
              }],
              isError: true
            });
          }, 1e4);
        });
      }
    );
    this.registerTool(
      {
        name: "get_logs",
        description: "Get application or deployment logs from various sources",
        inputSchema: {
          type: "object",
          properties: {
            source: {
              type: "string",
              description: "Log source: cloudflare, vercel, or local",
              enum: ["cloudflare", "vercel", "local"],
              default: "local"
            },
            project: {
              type: "string",
              description: "Project name or ID"
            },
            limit: {
              type: "number",
              description: "Maximum number of log entries",
              default: 50
            },
            level: {
              type: "string",
              description: "Filter by log level",
              enum: ["info", "warn", "error", "debug"]
            },
            startTime: {
              type: "string",
              description: "Start time (ISO 8601)"
            }
          }
        }
      },
      async (args) => {
        return new Promise((resolve) => {
          const requestId = Date.now().toString();
          const handler = (event) => {
            if (event.data?.type === "logs:response" && event.data?.requestId === requestId) {
              self.removeEventListener("message", handler);
              const logs = event.data.logs || [];
              resolve({
                content: [{
                  type: "text",
                  text: logs.map((l) => `[${l.timestamp || "N/A"}] ${l.level?.toUpperCase() || "INFO"}: ${l.message}`).join("\n")
                }],
                ui: {
                  viewType: "log-viewer",
                  title: `${args.source || "Local"} Logs`,
                  description: `Showing ${logs.length} log entries`,
                  metadata: {
                    source: args.source || "local",
                    project: args.project,
                    itemCount: logs.length,
                    timestamp: (/* @__PURE__ */ new Date()).toISOString()
                  },
                  logs
                }
              });
            } else if (event.data?.type === "logs:error" && event.data?.requestId === requestId) {
              self.removeEventListener("message", handler);
              resolve({
                content: [{
                  type: "text",
                  text: event.data.error || "Failed to retrieve logs"
                }],
                isError: true
              });
            }
          };
          self.addEventListener("message", handler);
          self.postMessage({
            type: "tool:get_logs",
            requestId,
            params: args
          });
          setTimeout(() => {
            self.removeEventListener("message", handler);
            resolve({
              content: [{
                type: "text",
                text: "Log retrieval timed out"
              }],
              isError: true
            });
          }, 1e4);
        });
      }
    );
  }
};
setupWorkerServer(BrowserOSServer);
//# sourceMappingURL=browser-os.js.map
