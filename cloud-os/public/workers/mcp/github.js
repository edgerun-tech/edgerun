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

// src/workers/mcp/github.ts
var GITHUB_API_BASE = "https://api.github.com";
var GitHubServer = class extends MCPServerBase {
  constructor() {
    super("github", "1.0.0");
  }
  setupHandlers() {
    this.registerTool(
      {
        name: "github_list_repos",
        description: "List repositories for the authenticated user",
        inputSchema: {
          type: "object",
          properties: {
            sort: {
              type: "string",
              enum: ["created", "updated", "pushed", "full_name"],
              default: "updated"
            },
            limit: {
              type: "number",
              default: 30
            }
          }
        }
      },
      async (args) => {
        const token = await this.getToken();
        if (!token) {
          return {
            content: [{ type: "text", text: "Not authenticated. Please connect GitHub first." }],
            isError: true
          };
        }
        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/user/repos?sort=${args.sort || "updated"}&per_page=${args.limit || 30}`,
            {
              headers: {
                "Authorization": `Bearer ${token}`,
                "Accept": "application/vnd.github.v3+json"
              }
            }
          );
          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }
          const repos = await response.json();
          return {
            content: [{
              type: "text",
              text: JSON.stringify(repos.map((r) => ({
                name: r.name,
                full_name: r.full_name,
                description: r.description,
                url: r.html_url,
                stars: r.stargazers_count,
                language: r.language,
                updated: r.updated_at
              })), null, 2)
            }]
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
        name: "github_get_repo",
        description: "Get details of a specific repository",
        inputSchema: {
          type: "object",
          properties: {
            owner: {
              type: "string",
              description: "Repository owner"
            },
            repo: {
              type: "string",
              description: "Repository name"
            }
          },
          required: ["owner", "repo"]
        }
      },
      async (args) => {
        const token = await this.getToken();
        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}`,
            {
              headers: token ? {
                "Authorization": `Bearer ${token}`,
                "Accept": "application/vnd.github.v3+json"
              } : {
                "Accept": "application/vnd.github.v3+json"
              }
            }
          );
          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }
          const repo = await response.json();
          return {
            content: [{
              type: "text",
              text: JSON.stringify({
                name: repo.name,
                full_name: repo.full_name,
                description: repo.description,
                url: repo.html_url,
                stars: repo.stargazers_count,
                forks: repo.forks_count,
                language: repo.language,
                default_branch: repo.default_branch,
                updated: repo.updated_at
              }, null, 2)
            }]
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
        name: "github_list_issues",
        description: "List issues in a repository",
        inputSchema: {
          type: "object",
          properties: {
            owner: {
              type: "string",
              description: "Repository owner"
            },
            repo: {
              type: "string",
              description: "Repository name"
            },
            state: {
              type: "string",
              enum: ["open", "closed", "all"],
              default: "open"
            },
            limit: {
              type: "number",
              default: 30
            }
          },
          required: ["owner", "repo"]
        }
      },
      async (args) => {
        const token = await this.getToken();
        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}/issues?state=${args.state || "open"}&per_page=${args.limit || 30}`,
            {
              headers: token ? {
                "Authorization": `Bearer ${token}`,
                "Accept": "application/vnd.github.v3+json"
              } : {
                "Accept": "application/vnd.github.v3+json"
              }
            }
          );
          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }
          const issues = await response.json();
          return {
            content: [{
              type: "text",
              text: JSON.stringify(issues.map((i) => ({
                number: i.number,
                title: i.title,
                state: i.state,
                url: i.html_url,
                user: i.user.login,
                created: i.created_at,
                labels: i.labels.map((l) => l.name)
              })), null, 2)
            }]
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
        name: "github_create_issue",
        description: "Create a new issue in a repository",
        inputSchema: {
          type: "object",
          properties: {
            owner: {
              type: "string",
              description: "Repository owner"
            },
            repo: {
              type: "string",
              description: "Repository name"
            },
            title: {
              type: "string",
              description: "Issue title"
            },
            body: {
              type: "string",
              description: "Issue body"
            }
          },
          required: ["owner", "repo", "title"]
        }
      },
      async (args) => {
        const token = await this.getToken();
        if (!token) {
          return {
            content: [{ type: "text", text: "Not authenticated. Please connect GitHub first." }],
            isError: true
          };
        }
        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}/issues`,
            {
              method: "POST",
              headers: {
                "Authorization": `Bearer ${token}`,
                "Accept": "application/vnd.github.v3+json",
                "Content-Type": "application/json"
              },
              body: JSON.stringify({
                title: args.title,
                body: args.body
              })
            }
          );
          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }
          const issue = await response.json();
          return {
            content: [{
              type: "text",
              text: `Created issue #${issue.number}: ${issue.title}
${issue.html_url}`
            }]
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
        name: "github_search_code",
        description: "Search code across GitHub",
        inputSchema: {
          type: "object",
          properties: {
            query: {
              type: "string",
              description: "Search query"
            },
            language: {
              type: "string",
              description: "Filter by language"
            },
            limit: {
              type: "number",
              default: 30
            }
          },
          required: ["query"]
        }
      },
      async (args) => {
        const token = await this.getToken();
        let query = args.query;
        if (args.language) {
          query += ` language:${args.language}`;
        }
        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/search/code?q=${encodeURIComponent(query)}&per_page=${args.limit || 30}`,
            {
              headers: token ? {
                "Authorization": `Bearer ${token}`,
                "Accept": "application/vnd.github.v3+json"
              } : {
                "Accept": "application/vnd.github.v3+json"
              }
            }
          );
          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }
          const result = await response.json();
          return {
            content: [{
              type: "text",
              text: JSON.stringify({
                total: result.total_count,
                items: result.items.map((item) => ({
                  name: item.name,
                  path: item.path,
                  repository: item.repository.full_name,
                  url: item.html_url
                }))
              }, null, 2)
            }]
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
        name: "github_get_commits",
        description: "Get commit history for a repository",
        inputSchema: {
          type: "object",
          properties: {
            owner: {
              type: "string",
              description: "Repository owner"
            },
            repo: {
              type: "string",
              description: "Repository name"
            },
            branch: {
              type: "string",
              description: "Branch name (defaults to default branch)"
            },
            limit: {
              type: "number",
              description: "Maximum number of commits",
              default: 10
            },
            sha: {
              type: "string",
              description: "SHA or branch to start from"
            }
          },
          required: ["owner", "repo"]
        }
      },
      async (args) => {
        const token = await this.getToken();
        try {
          let url = `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}/commits?per_page=${args.limit || 10}`;
          if (args.branch) url += `&sha=${args.branch}`;
          if (args.sha) url += `&sha=${args.sha}`;
          const response = await fetch(url, {
            headers: token ? {
              "Authorization": `Bearer ${token}`,
              "Accept": "application/vnd.github.v3+json"
            } : {
              "Accept": "application/vnd.github.v3+json"
            }
          });
          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }
          const commits = await response.json();
          return {
            content: [{
              type: "text",
              text: commits.map(
                (c) => `${c.sha.slice(0, 7)} - ${c.commit.message.split("\n")[0]} (${c.commit.author.name}, ${c.commit.author.date.split("T")[0]})`
              ).join("\n")
            }],
            ui: {
              viewType: "timeline",
              title: "Commit History",
              description: `Last ${commits.length} commits from ${args.owner}/${args.repo}`,
              metadata: {
                source: "github",
                owner: args.owner,
                repo: args.repo,
                branch: args.branch || "default",
                itemCount: commits.length,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              events: commits.map((c) => ({
                id: c.sha,
                title: c.commit.message.split("\n")[0],
                description: c.commit.message,
                timestamp: c.commit.author.date,
                author: c.commit.author.name,
                avatar: c.author?.avatar_url,
                type: "commit"
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
        name: "github_get_diff",
        description: "Get code diff between branches or for a pull request",
        inputSchema: {
          type: "object",
          properties: {
            owner: {
              type: "string",
              description: "Repository owner"
            },
            repo: {
              type: "string",
              description: "Repository name"
            },
            base: {
              type: "string",
              description: "Base branch (e.g., main)"
            },
            head: {
              type: "string",
              description: "Head branch (e.g., feature-branch)"
            },
            pr: {
              type: "number",
              description: "Pull request number (alternative to base/head)"
            }
          },
          required: ["owner", "repo"]
        }
      },
      async (args) => {
        const token = await this.getToken();
        try {
          let url;
          if (args.pr) {
            url = `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}/pulls/${args.pr}`;
          } else if (args.base && args.head) {
            url = `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}/compare/${args.base}...${args.head}`;
          } else {
            return {
              content: [{ type: "text", text: "Error: Must provide either PR number or both base and head branches" }],
              isError: true
            };
          }
          const response = await fetch(url, {
            headers: token ? {
              "Authorization": `Bearer ${token}`,
              "Accept": "application/vnd.github.v3.diff"
            } : {
              "Accept": "application/vnd.github.v3.diff"
            }
          });
          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }
          const diff = await response.text();
          const lines = diff.split("\n");
          const addedLines = lines.filter((l) => l.startsWith("+") && !l.startsWith("+++")).length;
          const removedLines = lines.filter((l) => l.startsWith("-") && !l.startsWith("---")).length;
          const filesChanged = lines.filter((l) => l.startsWith("diff --git")).length;
          return {
            content: [{
              type: "text",
              text: diff.substring(0, 1e4) + (diff.length > 1e4 ? "\n... (truncated)" : "")
            }],
            ui: {
              viewType: "code-diff",
              title: args.pr ? `PR #${args.pr}` : `${args.base} \u2192 ${args.head}`,
              description: `${filesChanged} file(s) changed: +${addedLines} -${removedLines}`,
              metadata: {
                source: "github",
                owner: args.owner,
                repo: args.repo,
                filesChanged,
                additions: addedLines,
                deletions: removedLines,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              diff
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
        name: "github_get_repo_info",
        description: "Get detailed repository information and metadata",
        inputSchema: {
          type: "object",
          properties: {
            owner: {
              type: "string",
              description: "Repository owner"
            },
            repo: {
              type: "string",
              description: "Repository name"
            }
          },
          required: ["owner", "repo"]
        }
      },
      async (args) => {
        const token = await this.getToken();
        try {
          const response = await fetch(
            `${GITHUB_API_BASE}/repos/${args.owner}/${args.repo}`,
            {
              headers: token ? {
                "Authorization": `Bearer ${token}`,
                "Accept": "application/vnd.github.v3+json"
              } : {
                "Accept": "application/vnd.github.v3+json"
              }
            }
          );
          if (!response.ok) {
            throw new Error(`GitHub API error: ${response.statusText}`);
          }
          const repo = await response.json();
          return {
            content: [{
              type: "text",
              text: JSON.stringify({
                name: repo.name,
                full_name: repo.full_name,
                description: repo.description,
                url: repo.html_url,
                stars: repo.stargazers_count,
                forks: repo.forks_count,
                watchers: repo.watchers_count,
                language: repo.language,
                default_branch: repo.default_branch,
                open_issues: repo.open_issues_count,
                license: repo.license?.name,
                topics: repo.topics,
                created: repo.created_at,
                updated: repo.updated_at,
                pushed: repo.pushed_at,
                owner: repo.owner.login,
                is_fork: repo.fork,
                is_private: repo.private,
                has_wiki: repo.has_wiki,
                has_pages: repo.has_pages
              }, null, 2)
            }],
            ui: {
              viewType: "json-tree",
              title: `${args.owner}/${args.repo}`,
              description: repo.description || "GitHub Repository",
              metadata: {
                source: "github",
                owner: args.owner,
                repo: args.repo,
                stars: repo.stargazers_count,
                forks: repo.forks_count,
                language: repo.language,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              data: repo
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
  async getToken() {
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
        key: "github_token"
      });
      setTimeout(() => {
        self.removeEventListener("message", handler);
        resolve(null);
      }, 1e3);
    });
  }
};
setupWorkerServer(GitHubServer);
//# sourceMappingURL=github.js.map
