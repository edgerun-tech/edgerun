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

// src/workers/mcp/qwen.ts
var QwenServer = class extends MCPServerBase {
  constructor() {
    super("qwen", "1.0.0");
    __publicField(this, "token", null);
    this.requestToken();
  }
  /**
   * Request OAuth token from main thread
   */
  requestToken() {
    const requestId = Math.random().toString(36).substring(2) + Date.now().toString(36);
    const handler = (event) => {
      if (event.data?.type === "token:response" && event.data.requestId === requestId) {
        if (event.data.token) {
          try {
            this.token = JSON.parse(event.data.token);
            console.log("[Qwen MCP] Token received");
          } catch {
            console.error("[Qwen MCP] Failed to parse token");
          }
        }
        self.removeEventListener("message", handler);
      }
    };
    self.addEventListener("message", handler);
    self.postMessage({
      type: "token:request",
      requestId,
      key: "qwen_token"
    });
  }
  /**
   * Call Qwen API
   */
  async callQwenAPI(model, messages) {
    if (!this.token) {
      throw new Error("Qwen OAuth token not available");
    }
    const response = await fetch("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${this.token.access_token}`
      },
      body: JSON.stringify({
        model,
        messages,
        max_tokens: 2e3,
        stream: false
      })
    });
    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Qwen API error: ${error}`);
    }
    return await response.json();
  }
  setupHandlers() {
    this.registerTool(
      {
        name: "qwen_chat",
        description: "Chat with Qwen AI for code assistance, explanations, and analysis",
        inputSchema: {
          type: "object",
          properties: {
            prompt: {
              type: "string",
              description: "Your question or request for Qwen"
            },
            model: {
              type: "string",
              description: "Qwen model to use",
              enum: ["qwen-plus", "qwen-turbo", "qwen-max", "qwen3.5-coder-plus"],
              default: "qwen-plus"
            }
          },
          required: ["prompt"]
        }
      },
      async (args) => {
        try {
          const result = await this.callQwenAPI(args.model || "qwen-plus", [
            { role: "user", content: args.prompt }
          ]);
          const content = result.choices?.[0]?.message?.content || "No response from Qwen";
          return {
            content: [{ type: "text", text: content }]
          };
        } catch (error) {
          return {
            content: [{
              type: "text",
              text: `Qwen error: ${error instanceof Error ? error.message : "Unknown error"}`
            }],
            isError: true
          };
        }
      }
    );
    this.registerTool(
      {
        name: "qwen_code_review",
        description: "Review code for issues, improvements, and best practices",
        inputSchema: {
          type: "object",
          properties: {
            code: {
              type: "string",
              description: "Code to review"
            },
            language: {
              type: "string",
              description: "Programming language"
            },
            focus: {
              type: "string",
              description: "Specific aspect to focus on (performance, security, style, etc.)"
            }
          },
          required: ["code"]
        }
      },
      async (args) => {
        try {
          const prompt = [
            "Review this code for issues and improvements:",
            "",
            "```" + (args.language || ""),
            args.code,
            "```",
            "",
            args.focus ? `Focus on: ${args.focus}` : "Provide general code review feedback."
          ].join("\n");
          const result = await this.callQwenAPI("qwen-plus", [
            { role: "user", content: prompt }
          ]);
          const content = result.choices?.[0]?.message?.content || "No review provided";
          return {
            content: [{ type: "text", text: content }]
          };
        } catch (error) {
          return {
            content: [{
              type: "text",
              text: `Code review error: ${error instanceof Error ? error.message : "Unknown error"}`
            }],
            isError: true
          };
        }
      }
    );
    this.registerTool(
      {
        name: "qwen_explain_code",
        description: "Explain what a piece of code does in simple terms",
        inputSchema: {
          type: "object",
          properties: {
            code: {
              type: "string",
              description: "Code to explain"
            },
            language: {
              type: "string",
              description: "Programming language"
            },
            level: {
              type: "string",
              description: "Explanation level (beginner, intermediate, advanced)",
              enum: ["beginner", "intermediate", "advanced"],
              default: "intermediate"
            }
          },
          required: ["code"]
        }
      },
      async (args) => {
        try {
          const prompt = [
            `Explain this code at ${args.level || "intermediate"} level:`,
            "",
            "```" + (args.language || ""),
            args.code,
            "```",
            "",
            "Include:",
            "- What the code does",
            "- Key concepts used",
            "- How it works step by step"
          ].join("\n");
          const result = await this.callQwenAPI("qwen-plus", [
            { role: "user", content: prompt }
          ]);
          const content = result.choices?.[0]?.message?.content || "No explanation provided";
          return {
            content: [{ type: "text", text: content }]
          };
        } catch (error) {
          return {
            content: [{
              type: "text",
              text: `Explanation error: ${error instanceof Error ? error.message : "Unknown error"}`
            }],
            isError: true
          };
        }
      }
    );
    console.log("[Qwen MCP] Server initialized with 3 tools");
  }
};
setupWorkerServer(QwenServer);
//# sourceMappingURL=qwen.js.map
