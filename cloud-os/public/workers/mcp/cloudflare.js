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

// src/workers/mcp/cloudflare.ts
var CLOUDFLARE_API_BASE = "https://api.cloudflare.com/client/v4";
var CloudflareServer = class extends MCPServerBase {
  constructor() {
    super("cloudflare", "1.0.0");
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
          key: "cloudflare_token"
        });
        setTimeout(() => {
          self.removeEventListener("message", handler);
          resolve(null);
        }, 1e3);
      });
    };
    this.registerTool(
      {
        name: "cloudflare_get_account",
        description: "Get Cloudflare account details",
        inputSchema: {
          type: "object",
          properties: {}
        }
      },
      async () => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: "text", text: "Not authenticated. Please add Cloudflare API token." }], isError: true };
        }
        try {
          const response = await fetch(`${CLOUDFLARE_API_BASE}/user`, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          const data = await response.json();
          return { content: [{ type: "text", text: JSON.stringify(data.result, null, 2) }] };
        } catch (error) {
          return { content: [{ type: "text", text: `Error: ${error}` }], isError: true };
        }
      }
    );
    this.registerTool(
      {
        name: "cloudflare_list_zones",
        description: "List DNS zones in Cloudflare account",
        inputSchema: {
          type: "object",
          properties: {
            limit: { type: "number", default: 50 }
          }
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: "text", text: "Not authenticated. Please add Cloudflare API token." }], isError: true };
        }
        try {
          const response = await fetch(`${CLOUDFLARE_API_BASE}/zones?per_page=${args.limit || 50}`, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          const data = await response.json();
          const zones = data.result.map((z) => ({
            id: z.id,
            name: z.name,
            status: z.status,
            plan: z.plan.name,
            created: z.created_on
          }));
          return { content: [{ type: "text", text: JSON.stringify(zones, null, 2) }] };
        } catch (error) {
          return { content: [{ type: "text", text: `Error: ${error}` }], isError: true };
        }
      }
    );
    this.registerTool(
      {
        name: "cloudflare_list_dns",
        description: "List DNS records in a zone",
        inputSchema: {
          type: "object",
          properties: {
            zone_id: { type: "string", description: "Zone ID" },
            type: { type: "string", description: "Filter by record type" }
          },
          required: ["zone_id"]
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: "text", text: "Not authenticated." }], isError: true };
        }
        try {
          let url = `${CLOUDFLARE_API_BASE}/zones/${args.zone_id}/dns_records?per_page=100`;
          if (args.type) url += `&type=${args.type}`;
          const response = await fetch(url, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          const data = await response.json();
          const records = data.result.map((r) => ({
            id: r.id,
            type: r.type,
            name: r.name,
            content: r.content,
            proxied: r.proxiable ? r.proxied : void 0
          }));
          return { content: [{ type: "text", text: JSON.stringify(records, null, 2) }] };
        } catch (error) {
          return { content: [{ type: "text", text: `Error: ${error}` }], isError: true };
        }
      }
    );
    this.registerTool(
      {
        name: "cloudflare_create_dns",
        description: "Create a DNS record",
        inputSchema: {
          type: "object",
          properties: {
            zone_id: { type: "string" },
            type: { type: "string", enum: ["A", "AAAA", "CNAME", "MX", "TXT", "SPF", "SRV"] },
            name: { type: "string" },
            content: { type: "string" },
            proxied: { type: "boolean", default: true }
          },
          required: ["zone_id", "type", "name", "content"]
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: "text", text: "Not authenticated." }], isError: true };
        }
        try {
          const response = await fetch(`${CLOUDFLARE_API_BASE}/zones/${args.zone_id}/dns_records`, {
            method: "POST",
            headers: {
              "Authorization": `Bearer ${token}`,
              "Content-Type": "application/json"
            },
            body: JSON.stringify({
              type: args.type,
              name: args.name,
              content: args.content,
              proxied: args.proxied
            })
          });
          if (!response.ok) {
            const err = await response.json();
            throw new Error(err.errors?.[0]?.message || response.statusText);
          }
          const data = await response.json();
          return { content: [{ type: "text", text: `Created ${args.type} record: ${args.name} -> ${args.content}` }] };
        } catch (error) {
          return { content: [{ type: "text", text: `Error: ${error}` }], isError: true };
        }
      }
    );
    this.registerTool(
      {
        name: "cloudflare_delete_dns",
        description: "Delete a DNS record",
        inputSchema: {
          type: "object",
          properties: {
            zone_id: { type: "string" },
            record_id: { type: "string" }
          },
          required: ["zone_id", "record_id"]
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: "text", text: "Not authenticated." }], isError: true };
        }
        try {
          const response = await fetch(
            `${CLOUDFLARE_API_BASE}/zones/${args.zone_id}/dns_records/${args.record_id}`,
            { method: "DELETE", headers: { "Authorization": `Bearer ${token}` } }
          );
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          return { content: [{ type: "text", text: "DNS record deleted successfully" }] };
        } catch (error) {
          return { content: [{ type: "text", text: `Error: ${error}` }], isError: true };
        }
      }
    );
    this.registerTool(
      {
        name: "cloudflare_list_workers",
        description: "List Cloudflare Workers",
        inputSchema: { type: "object", properties: {} }
      },
      async () => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: "text", text: "Not authenticated." }], isError: true };
        }
        try {
          const response = await fetch(`${CLOUDFLARE_API_BASE}/accounts/workers/scripts`, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          const data = await response.json();
          return { content: [{ type: "text", text: JSON.stringify(data.result, null, 2) }] };
        } catch (error) {
          return { content: [{ type: "text", text: `Error: ${error}` }], isError: true };
        }
      }
    );
    this.registerTool(
      {
        name: "cloudflare_list_tunnels",
        description: "List Cloudflare Tunnels",
        inputSchema: { type: "object", properties: {} }
      },
      async () => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: "text", text: "Not authenticated." }], isError: true };
        }
        try {
          const response = await fetch(`${CLOUDFLARE_API_BASE}/accounts`, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          const account = (await response.json()).result[0];
          if (!account) throw new Error("No account found");
          const tunnelsRes = await fetch(`${CLOUDFLARE_API_BASE}/accounts/${account.id}/tunnels`, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          const tunnels = (await tunnelsRes.json()).result || [];
          return { content: [{ type: "text", text: JSON.stringify(tunnels, null, 2) }] };
        } catch (error) {
          return { content: [{ type: "text", text: `Error: ${error}` }], isError: true };
        }
      }
    );
    this.registerTool(
      {
        name: "cloudflare_get_deployments",
        description: "Get Cloudflare Workers deployment history",
        inputSchema: {
          type: "object",
          properties: {
            script: {
              type: "string",
              description: "Worker script name"
            },
            limit: {
              type: "number",
              description: "Maximum number of deployments to return",
              default: 10
            }
          }
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: "text", text: "Not authenticated. Please add Cloudflare API token." }], isError: true };
        }
        try {
          const accountResponse = await fetch(`${CLOUDFLARE_API_BASE}/user`, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!accountResponse.ok) throw new Error(`API error: ${accountResponse.statusText}`);
          const accountData = await accountResponse.json();
          const accountId = accountData.result.account?.id;
          if (!accountId) {
            return { content: [{ type: "text", text: "No account ID found" }], isError: true };
          }
          let url = `${CLOUDFLARE_API_BASE}/accounts/${accountId}/deployments?per_page=${args.limit || 10}`;
          if (args.script) {
            url += `&script_name=${encodeURIComponent(args.script)}`;
          }
          const response = await fetch(url, {
            headers: { "Authorization": `Bearer ${token}` }
          });
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          const data = await response.json();
          const deployments = data.result || [];
          return {
            content: [{
              type: "text",
              text: deployments.map(
                (d) => `${d.deployment_id.slice(0, 8)} - ${d.script?.tag || "N/A"} (${d.environment}, ${d.stage}, ${new Date(d.created_on).toLocaleString()})`
              ).join("\n")
            }],
            ui: {
              viewType: "timeline",
              title: "Cloudflare Deployments",
              description: args.script ? `Deployments for ${args.script}` : "Recent deployments",
              metadata: {
                source: "cloudflare",
                accountId,
                script: args.script,
                itemCount: deployments.length,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              events: deployments.map((d) => ({
                id: d.deployment_id,
                title: d.script?.tag || d.script?.name || "Unknown",
                description: `Environment: ${d.environment}, Stage: ${d.stage}`,
                timestamp: d.created_on,
                type: "deployment",
                status: d.stage === "production" ? "success" : "pending",
                metadata: {
                  deploymentId: d.deployment_id,
                  environment: d.environment,
                  stage: d.stage,
                  url: d.url
                }
              }))
            }
          };
        } catch (error) {
          return { content: [{ type: "text", text: `Error: ${error instanceof Error ? error.message : "Unknown error"}` }], isError: true };
        }
      }
    );
  }
};
setupWorkerServer(CloudflareServer);
//# sourceMappingURL=cloudflare.js.map
