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

// src/workers/mcp/vercel.ts
var VERCEL_API_BASE = "https://api.vercel.com";
var VercelServer = class extends MCPServerBase {
  constructor() {
    super("vercel", "1.0.0");
  }
  setupHandlers() {
    const getToken = async () => {
      return new Promise((resolve) => {
        const requestId = Date.now().toString();
        const handler = (event) => {
          if (event.data?.type === "token:response" && event.data?.requestId === requestId) {
            self.removeEventListener("message", handler);
            resolve(event.data.token);
          }
        };
        self.addEventListener("message", handler);
        self.postMessage({
          type: "token:request",
          requestId,
          key: "vercel_token"
        });
        setTimeout(() => {
          self.removeEventListener("message", handler);
          resolve(null);
        }, 1e3);
      });
    };
    const getTeamId = async (token) => {
      try {
        const response = await fetch(`${VERCEL_API_BASE}/v2/teams`, {
          headers: { "Authorization": `Bearer ${token}` }
        });
        if (response.ok) {
          const data = await response.json();
          return data.teams?.[0]?.id || null;
        }
      } catch {
      }
      return null;
    };
    this.registerTool(
      {
        name: "vercel_list_projects",
        description: "List Vercel projects",
        inputSchema: {
          type: "object",
          properties: {
            limit: { type: "number", default: 20 }
          }
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return {
            content: [{ type: "text", text: "Not authenticated. Please add Vercel API token." }],
            isError: true
          };
        }
        try {
          const teamId = await getTeamId(token);
          const url = teamId ? `${VERCEL_API_BASE}/v9/projects?teamId=${teamId}&limit=${args.limit || 20}` : `${VERCEL_API_BASE}/v9/projects?limit=${args.limit || 20}`;
          const response = await fetch(url, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          const data = await response.json();
          const projects = data.projects || [];
          return {
            content: [{
              type: "text",
              text: projects.map(
                (p) => `${p.name} (${p.framework || "Static"}) - ${p.url}`
              ).join("\n")
            }],
            ui: {
              viewType: "file-grid",
              title: "Vercel Projects",
              description: `${projects.length} project(s)`,
              metadata: {
                source: "vercel",
                itemCount: projects.length,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              items: projects.map((p) => ({
                id: p.id,
                name: p.name,
                type: "project",
                url: p.url,
                framework: p.framework
              }))
            }
          };
        } catch (error) {
          return {
            content: [{ type: "text", text: `Error: ${error instanceof Error ? error.message : "Unknown error"}` }],
            isError: true
          };
        }
      }
    );
    this.registerTool(
      {
        name: "vercel_get_deployments",
        description: "Get deployment history for a Vercel project",
        inputSchema: {
          type: "object",
          properties: {
            project: { type: "string", description: "Project name or ID" },
            limit: { type: "number", default: 10 }
          }
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return {
            content: [{ type: "text", text: "Not authenticated. Please add Vercel API token." }],
            isError: true
          };
        }
        try {
          const teamId = await getTeamId(token);
          const params = new URLSearchParams({
            limit: String(args.limit || 10),
            ...args.project ? { projectId: args.project } : {}
          });
          const url = `${VERCEL_API_BASE}/v6/deployments?${params}${teamId ? `&teamId=${teamId}` : ""}`;
          const response = await fetch(url, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          const data = await response.json();
          const deployments = data.deployments || [];
          return {
            content: [{
              type: "text",
              text: deployments.map(
                (d) => `${d.url} - ${d.state} (${d.target}, ${new Date(d.created).toLocaleString()})`
              ).join("\n")
            }],
            ui: {
              viewType: "timeline",
              title: "Vercel Deployments",
              description: args.project ? `Deployments for ${args.project}` : "Recent deployments",
              metadata: {
                source: "vercel",
                project: args.project,
                itemCount: deployments.length,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              events: deployments.map((d) => ({
                id: d.id,
                title: d.project?.name || "Unknown",
                description: `${d.url} - ${d.target}`,
                timestamp: new Date(d.created).toISOString(),
                type: "deployment",
                status: d.state === "READY" ? "success" : d.state === "ERROR" ? "error" : "pending",
                metadata: {
                  url: d.url,
                  state: d.state,
                  target: d.target,
                  commitRef: d.ref
                }
              }))
            }
          };
        } catch (error) {
          return {
            content: [{ type: "text", text: `Error: ${error instanceof Error ? error.message : "Unknown error"}` }],
            isError: true
          };
        }
      }
    );
    this.registerTool(
      {
        name: "vercel_list_domains",
        description: "List domains configured in Vercel",
        inputSchema: {
          type: "object",
          properties: {}
        }
      },
      async () => {
        const token = await getToken();
        if (!token) {
          return {
            content: [{ type: "text", text: "Not authenticated. Please add Vercel API token." }],
            isError: true
          };
        }
        try {
          const teamId = await getTeamId(token);
          const url = `${VERCEL_API_BASE}/v9/domains${teamId ? `?teamId=${teamId}` : ""}`;
          const response = await fetch(url, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          const data = await response.json();
          const domains = data.domains || [];
          return {
            content: [{
              type: "text",
              text: domains.map(
                (d) => `${d.name} (${d.verified ? "\u2713 verified" : "\u25CB pending"})`
              ).join("\n")
            }],
            ui: {
              viewType: "data-table",
              title: "Vercel Domains",
              description: `${domains.length} domain(s)`,
              metadata: {
                source: "vercel",
                itemCount: domains.length,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              columns: ["Domain", "Status", "Redirect"],
              rows: domains.map((d) => [
                d.name,
                d.verified ? "Verified" : "Pending",
                d.redirect || "-"
              ])
            }
          };
        } catch (error) {
          return {
            content: [{ type: "text", text: `Error: ${error instanceof Error ? error.message : "Unknown error"}` }],
            isError: true
          };
        }
      }
    );
    this.registerTool(
      {
        name: "vercel_get_logs",
        description: "Get deployment logs from Vercel",
        inputSchema: {
          type: "object",
          properties: {
            deploymentId: { type: "string", description: "Deployment ID" },
            limit: { type: "number", default: 50 }
          }
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return {
            content: [{ type: "text", text: "Not authenticated. Please add Vercel API token." }],
            isError: true
          };
        }
        if (!args.deploymentId) {
          return {
            content: [{ type: "text", text: "Error: deploymentId is required" }],
            isError: true
          };
        }
        try {
          const teamId = await getTeamId(token);
          const url = `${VERCEL_API_BASE}/v1/deployments/${args.deploymentId}/logs${teamId ? `?teamId=${teamId}` : ""}`;
          const response = await fetch(url, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          const data = await response.json();
          const logs = data.logs || [];
          return {
            content: [{
              type: "text",
              text: logs.map((l) => `[${new Date(l.time).toISOString()}] ${l.level?.toUpperCase()}: ${l.text}`).join("\n")
            }],
            ui: {
              viewType: "log-viewer",
              title: "Deployment Logs",
              description: `Logs for ${args.deploymentId}`,
              metadata: {
                source: "vercel",
                deploymentId: args.deploymentId,
                itemCount: logs.length,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              logs: logs.map((l) => ({
                timestamp: new Date(l.time).toISOString(),
                level: l.level || "info",
                message: l.text
              }))
            }
          };
        } catch (error) {
          return {
            content: [{ type: "text", text: `Error: ${error instanceof Error ? error.message : "Unknown error"}` }],
            isError: true
          };
        }
      }
    );
  }
};
setupWorkerServer(VercelServer);
//# sourceMappingURL=vercel.js.map
