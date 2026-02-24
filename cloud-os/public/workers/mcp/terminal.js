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

// src/workers/mcp/frontend-terminal.ts
var mockFileSystem = {
  "/home/package.json": JSON.stringify({ name: "demo-app", version: "1.0.0" }, null, 2),
  "/home/index.js": 'console.log("Hello from browser!");\n',
  "/home/README.md": "# Demo App\n\nRunning in browser with WebContainers!\n",
  "/home/src/app.js": 'export const app = { name: "demo" };\n'
};
var mockCommands = {
  "help": "Available commands: help, ls, pwd, echo, cat, node, npm, git, clear, whoami, date, uname",
  "whoami": "browser-user",
  "pwd": "/home",
  "uname -a": "WebContainer 1.0.0 browser x86_64",
  "node --version": "v20.10.0",
  "npm --version": "10.2.0",
  "git --version": "git version 2.40.0 (webcontainer)"
};
var FrontendTerminalServer = class extends MCPServerBase {
  constructor() {
    super("frontend-terminal", "1.0.0");
    __publicField(this, "webContainer", null);
    __publicField(this, "connected", false);
  }
  async initializeWebContainer() {
    if (typeof navigator === "undefined") return false;
    const isChrome = /Chrome/.test(navigator.userAgent) && /Google/.test(navigator.vendor);
    if (!isChrome) {
      console.log("[Terminal] WebContainers not supported, using mock mode");
      this.connected = true;
      return false;
    }
    try {
      const { WebContainer } = await import("@webcontainer/api");
      this.webContainer = await WebContainer.boot();
      this.connected = true;
      console.log("[Terminal] WebContainer booted successfully");
      return true;
    } catch (error) {
      console.warn("[Terminal] Failed to boot WebContainer:", error);
      this.connected = true;
      return false;
    }
  }
  setupHandlers() {
    this.initializeWebContainer();
    this.registerTool(
      {
        name: "terminal_execute",
        description: "Execute a shell command in the browser terminal (supports Node.js, npm, git, and basic shell commands)",
        inputSchema: {
          type: "object",
          properties: {
            command: {
              type: "string",
              description: "Command to execute"
            },
            args: {
              type: "array",
              description: "Command arguments",
              items: { type: "string" }
            },
            cwd: {
              type: "string",
              description: "Working directory",
              default: "/home"
            }
          },
          required: ["command"]
        }
      },
      async (args) => {
        const fullCommand = `${args.command} ${args.args?.join(" ") || ""}`.trim();
        if (this.webContainer) {
          return await this.executeWebContainerCommand(fullCommand, args.cwd);
        } else {
          return this.executeMockCommand(fullCommand);
        }
      }
    );
    this.registerTool(
      {
        name: "terminal_status",
        description: "Get terminal connection status and capabilities",
        inputSchema: {
          type: "object",
          properties: {}
        }
      },
      async () => {
        const status = {
          connected: this.connected,
          mode: this.webContainer ? "webcontainer" : "mock",
          browser: typeof navigator !== "undefined" ? navigator.userAgent : "unknown",
          capabilities: this.webContainer ? ["node", "npm", "git", "shell", "filesystem"] : ["mock-commands"]
        };
        return {
          content: [{
            type: "text",
            text: JSON.stringify(status, null, 2)
          }]
        };
      }
    );
    this.registerTool(
      {
        name: "terminal_list_files",
        description: "List files in the terminal working directory",
        inputSchema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description: "Directory path",
              default: "/home"
            }
          }
        }
      },
      async (args) => {
        if (this.webContainer) {
          try {
            const dir = await this.webContainer.fs.readdir(args.path || "/home");
            return {
              content: [{
                type: "text",
                text: dir.join("\n")
              }]
            };
          } catch (e) {
            return {
              content: [{ type: "text", text: `Error: ${e}` }],
              isError: true
            };
          }
        } else {
          const files = Object.keys(mockFileSystem).filter((p) => p.startsWith(args.path || "/home")).map((p) => p.split("/").pop() || p);
          return {
            content: [{
              type: "text",
              text: files.join("\n") || "Directory empty"
            }]
          };
        }
      }
    );
    this.registerTool(
      {
        name: "terminal_read_file",
        description: "Read a file in the terminal filesystem",
        inputSchema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description: "File path"
            }
          },
          required: ["path"]
        }
      },
      async (args) => {
        if (this.webContainer) {
          try {
            const content = await this.webContainer.fs.readFile(args.path, "utf-8");
            return {
              content: [{ type: "text", text: content }]
            };
          } catch (e) {
            return {
              content: [{ type: "text", text: `Error: ${e}` }],
              isError: true
            };
          }
        } else {
          const content = mockFileSystem[args.path];
          if (content) {
            return { content: [{ type: "text", text: content }] };
          }
          return {
            content: [{ type: "text", text: `File not found: ${args.path}` }],
            isError: true
          };
        }
      }
    );
    this.registerTool(
      {
        name: "terminal_write_file",
        description: "Write content to a file in the terminal filesystem",
        inputSchema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description: "File path"
            },
            content: {
              type: "string",
              description: "File content"
            }
          },
          required: ["path", "content"]
        }
      },
      async (args) => {
        if (this.webContainer) {
          try {
            await this.webContainer.fs.writeFile(args.path, args.content);
            return {
              content: [{ type: "text", text: `Written to ${args.path}` }]
            };
          } catch (e) {
            return {
              content: [{ type: "text", text: `Error: ${e}` }],
              isError: true
            };
          }
        } else {
          mockFileSystem[args.path] = args.content;
          return {
            content: [{ type: "text", text: `Written to ${args.path} (mock mode)` }]
          };
        }
      }
    );
    this.registerTool(
      {
        name: "terminal_npm_run",
        description: "Run an npm script in the terminal",
        inputSchema: {
          type: "object",
          properties: {
            script: {
              type: "string",
              description: "Script name (e.g., dev, build, test)"
            }
          },
          required: ["script"]
        }
      },
      async (args) => {
        if (this.webContainer) {
          try {
            const installProcess = await this.webContainer.spawn("npm", ["install"]);
            await installProcess.output.pipeTo(new WritableStream({
              write: (data) => console.log("[npm install]", data)
            }));
            const runProcess = await this.webContainer.spawn("npm", ["run", args.script]);
            let output = "";
            runProcess.output.pipeTo(new WritableStream({
              write: (data) => {
                output += data;
                console.log("[npm run]", data);
              }
            }));
            const exitCode = await runProcess.exit;
            return {
              content: [{
                type: "text",
                text: output || `npm run ${args.script} completed (exit code: ${exitCode})`
              }]
            };
          } catch (e) {
            return {
              content: [{ type: "text", text: `Error: ${e}` }],
              isError: true
            };
          }
        } else {
          return {
            content: [{
              type: "text",
              text: `[mock] npm run ${args.script}
> demo-app@1.0.0 ${args.script}
> echo "Running ${args.script}"
Running ${args.script}
`
            }]
          };
        }
      }
    );
    this.registerTool(
      {
        name: "terminal_git",
        description: "Run git commands in the terminal",
        inputSchema: {
          type: "object",
          properties: {
            args: {
              type: "array",
              description: "Git arguments",
              items: { type: "string" }
            }
          },
          required: ["args"]
        }
      },
      async (args) => {
        const gitCmd = `git ${args.args?.join(" ") || ""}`;
        if (this.webContainer) {
          try {
            const process = await this.webContainer.spawn("git", args.args || []);
            let output = "";
            process.output.pipeTo(new WritableStream({
              write: (data) => {
                output += data;
              }
            }));
            await process.exit;
            return {
              content: [{ type: "text", text: output || "Git command completed" }]
            };
          } catch (e) {
            return {
              content: [{ type: "text", text: `Error: ${e}` }],
              isError: true
            };
          }
        } else {
          const mockGit = {
            "status": "On branch main\nYour branch is up to date.\n\nnothing to commit, working tree clean",
            "log --oneline -5": "abc123 Initial commit\n",
            "branch": "* main\n  dev\n  feature/test",
            "remote -v": "origin	https://github.com/user/repo.git (fetch)\norigin	https://github.com/user/repo.git (push)"
          };
          const key = args.args?.join(" ") || "";
          return {
            content: [{
              type: "text",
              text: mockGit[key] || `[mock] ${gitCmd}
Git command executed`
            }]
          };
        }
      }
    );
  }
  async executeWebContainerCommand(command, cwd = "/home") {
    if (!this.webContainer) {
      return this.executeMockCommand(command);
    }
    try {
      const parts = command.split(" ");
      const cmd = parts[0];
      const args = parts.slice(1);
      if (cwd && cwd !== "/home") {
        try {
          await this.webContainer.fs.mkdir(cwd, { recursive: true });
        } catch {
        }
      }
      const process = await this.webContainer.spawn(cmd, args);
      let output = "";
      process.output.pipeTo(new WritableStream({
        write: (data) => {
          output += data;
        }
      }));
      const exitCode = await process.exit;
      return {
        content: [{
          type: "text",
          text: output || `Command completed (exit code: ${exitCode})`
        }],
        ui: {
          viewType: "log-viewer",
          title: command,
          description: `Exit code: ${exitCode}`,
          metadata: {
            command,
            cwd,
            exitCode,
            timestamp: (/* @__PURE__ */ new Date()).toISOString()
          },
          logs: output.split("\n").map((line, i) => ({
            timestamp: (/* @__PURE__ */ new Date()).toISOString(),
            level: "info",
            message: line
          }))
        }
      };
    } catch (error) {
      return {
        content: [{ type: "text", text: `Error: ${error.message || error}` }],
        isError: true
      };
    }
  }
  executeMockCommand(command) {
    const cmd = command.toLowerCase().trim();
    for (const [key, response] of Object.entries(mockCommands)) {
      if (cmd === key || cmd.startsWith(key + " ")) {
        return {
          content: [{ type: "text", text: response }]
        };
      }
    }
    if (cmd.startsWith("echo ")) {
      return {
        content: [{ type: "text", text: cmd.slice(5).replace(/"/g, "") }]
      };
    }
    if (cmd === "ls" || cmd === "ls -la") {
      const files = Object.keys(mockFileSystem).filter((p) => p.startsWith("/home/")).map((p) => p.replace("/home/", ""));
      return {
        content: [{ type: "text", text: files.join("  ") }]
      };
    }
    if (cmd.startsWith("cat ")) {
      const path = cmd.slice(4);
      const fullPath = path.startsWith("/") ? path : `/home/${path}`;
      const content = mockFileSystem[fullPath];
      if (content) {
        return { content: [{ type: "text", text: content }] };
      }
      return {
        content: [{ type: "text", text: `cat: ${path}: No such file or directory` }],
        isError: true
      };
    }
    if (cmd.startsWith("node ")) {
      return {
        content: [{
          type: "text",
          text: `[mock node] Executing ${cmd.slice(5)}
Output would appear here in WebContainer mode`
        }]
      };
    }
    return {
      content: [{
        type: "text",
        text: `[mock terminal] Command executed: ${command}

Tip: Use WebContainer (Chrome/Edge) for real command execution.`
      }]
    };
  }
};
setupWorkerServer(FrontendTerminalServer);
//# sourceMappingURL=terminal.js.map
