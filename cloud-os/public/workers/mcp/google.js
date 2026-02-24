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

// src/workers/mcp/google.ts
var GMAIL_API_BASE = "https://www.googleapis.com/gmail/v1";
var CALENDAR_API_BASE = "https://www.googleapis.com/calendar/v3";
var GoogleServer = class extends MCPServerBase {
  constructor() {
    super("google", "1.0.0");
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
          key: "google_token"
        });
        setTimeout(() => {
          self.removeEventListener("message", handler);
          resolve(null);
        }, 1e3);
      });
    };
    this.registerTool(
      {
        name: "google_get_emails",
        description: "Get emails from Gmail with search support",
        inputSchema: {
          type: "object",
          properties: {
            query: {
              type: "string",
              description: 'Gmail search query (e.g., "from:boss@example.com", "label:inbox", "has:attachment")',
              default: "label:inbox"
            },
            maxResults: {
              type: "number",
              description: "Maximum number of emails to return",
              default: 10
            },
            includeBody: {
              type: "boolean",
              description: "Whether to include email body in response",
              default: false
            }
          }
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return {
            content: [{ type: "text", text: "Not authenticated. Please connect Google first." }],
            isError: true
          };
        }
        try {
          const listUrl = `${GMAIL_API_BASE}/messages?q=${encodeURIComponent(args.query || "label:inbox")}&maxResults=${args.maxResults || 10}`;
          const listResponse = await fetch(listUrl, {
            headers: {
              "Authorization": `Bearer ${token}`,
              "Accept": "application/json"
            }
          });
          if (!listResponse.ok) {
            throw new Error(`Gmail API error: ${listResponse.statusText}`);
          }
          const listData = await listResponse.json();
          const messages = listData.messages || [];
          if (messages.length === 0) {
            return {
              content: [{ type: "text", text: "No emails found matching your query." }],
              ui: {
                viewType: "email-reader",
                title: "Gmail",
                description: "No results",
                metadata: {
                  source: "gmail",
                  query: args.query,
                  itemCount: 0,
                  timestamp: (/* @__PURE__ */ new Date()).toISOString()
                },
                emails: []
              }
            };
          }
          const emailPromises = messages.slice(0, args.maxResults || 10).map(async (msg) => {
            const detailUrl = `${GMAIL_API_BASE}/messages/${msg.id}?format=metadata&metadataHeaders=From&metadataHeaders=To&metadataHeaders=Subject&metadataHeaders=Date`;
            const detailResponse = await fetch(detailUrl, {
              headers: {
                "Authorization": `Bearer ${token}`,
                "Accept": "application/json"
              }
            });
            if (!detailResponse.ok) return null;
            const detail = await detailResponse.json();
            const headers = detail.payload?.headers || [];
            const getHeader = (name) => headers.find((h) => h.name === name)?.value || "";
            return {
              id: msg.id,
              threadId: msg.threadId,
              from: getHeader("From"),
              to: getHeader("To"),
              subject: getHeader("Subject"),
              date: getHeader("Date"),
              snippet: detail.snippet
            };
          });
          const emails = (await Promise.all(emailPromises)).filter(Boolean);
          return {
            content: [{
              type: "text",
              text: emails.map(
                (e) => `From: ${e.from}
Subject: ${e.subject}
Date: ${e.date}
${e.snippet}
---`
              ).join("\n")
            }],
            ui: {
              viewType: "email-reader",
              title: "Gmail",
              description: `Found ${emails.length} email(s) for "${args.query}"`,
              metadata: {
                source: "gmail",
                query: args.query,
                itemCount: emails.length,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              emails: emails.map((e) => ({
                id: e.id,
                from: e.from,
                to: e.to,
                subject: e.subject,
                date: e.date,
                snippet: e.snippet,
                unread: false
                // Would need labels info to determine
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
        name: "google_get_email_detail",
        description: "Get full content of a specific email",
        inputSchema: {
          type: "object",
          properties: {
            messageId: {
              type: "string",
              description: "Gmail message ID"
            }
          },
          required: ["messageId"]
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return {
            content: [{ type: "text", text: "Not authenticated. Please connect Google first." }],
            isError: true
          };
        }
        try {
          const response = await fetch(`${GMAIL_API_BASE}/messages/${args.messageId}?format=full`, {
            headers: {
              "Authorization": `Bearer ${token}`,
              "Accept": "application/json"
            }
          });
          if (!response.ok) {
            throw new Error(`Gmail API error: ${response.statusText}`);
          }
          const message = await response.json();
          const headers = message.payload?.headers || [];
          const getHeader = (name) => headers.find((h) => h.name === name)?.value || "";
          let body = "";
          if (message.payload?.body?.data) {
            body = atob(message.payload.body.data);
          } else if (message.payload?.parts?.[0]?.body?.data) {
            body = atob(message.payload.parts[0].body.data);
          }
          return {
            content: [{
              type: "text",
              text: `From: ${getHeader("From")}
To: ${getHeader("To")}
Subject: ${getHeader("Subject")}
Date: ${getHeader("Date")}

${body}`
            }],
            ui: {
              viewType: "email-reader",
              title: getHeader("Subject") || "Email",
              description: `From: ${getHeader("From")}`,
              metadata: {
                source: "gmail",
                messageId: args.messageId,
                from: getHeader("From"),
                to: getHeader("To"),
                subject: getHeader("Subject"),
                date: getHeader("Date"),
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              email: {
                id: message.id,
                from: getHeader("From"),
                to: getHeader("To"),
                subject: getHeader("Subject"),
                date: getHeader("Date"),
                body,
                attachments: message.payload?.parts?.filter((p) => p.filename).map((p) => ({
                  filename: p.filename,
                  mimeType: p.mimeType,
                  size: p.body.size
                })) || []
              }
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
        name: "google_get_calendar_events",
        description: "Get events from Google Calendar",
        inputSchema: {
          type: "object",
          properties: {
            calendarId: {
              type: "string",
              description: "Calendar ID (defaults to primary)",
              default: "primary"
            },
            timeMin: {
              type: "string",
              description: "Start time (ISO 8601, defaults to now)"
            },
            timeMax: {
              type: "string",
              description: "End time (ISO 8601, defaults to 7 days from now)"
            },
            maxResults: {
              type: "number",
              description: "Maximum number of events",
              default: 10
            },
            singleEvents: {
              type: "boolean",
              description: "Whether to expand recurring events",
              default: true
            }
          }
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return {
            content: [{ type: "text", text: "Not authenticated. Please connect Google first." }],
            isError: true
          };
        }
        try {
          const now = /* @__PURE__ */ new Date();
          const timeMin = args.timeMin || now.toISOString();
          const timeMax = args.timeMax || new Date(now.getTime() + 7 * 24 * 60 * 60 * 1e3).toISOString();
          let url = `${CALENDAR_API_BASE}/calendars/${encodeURIComponent(args.calendarId || "primary")}/events?`;
          url += `timeMin=${encodeURIComponent(timeMin)}&`;
          url += `timeMax=${encodeURIComponent(timeMax)}&`;
          url += `maxResults=${args.maxResults || 10}&`;
          url += `singleEvents=${args.singleEvents !== false}&`;
          url += `orderBy=startTime`;
          const response = await fetch(url, {
            headers: {
              "Authorization": `Bearer ${token}`,
              "Accept": "application/json"
            }
          });
          if (!response.ok) {
            throw new Error(`Calendar API error: ${response.statusText}`);
          }
          const data = await response.json();
          const events = data.items || [];
          if (events.length === 0) {
            return {
              content: [{ type: "text", text: "No calendar events found for the specified time range." }],
              ui: {
                viewType: "timeline",
                title: "Calendar",
                description: "No events",
                metadata: {
                  source: "google-calendar",
                  calendarId: args.calendarId || "primary",
                  itemCount: 0,
                  timeRange: `${timeMin} - ${timeMax}`,
                  timestamp: (/* @__PURE__ */ new Date()).toISOString()
                },
                events: []
              }
            };
          }
          return {
            content: [{
              type: "text",
              text: events.map((e) => {
                const start = e.start?.dateTime || e.start?.date;
                const end = e.end?.dateTime || e.end?.date;
                return `${e.summary || "No title"}
When: ${start} - ${end}
Where: ${e.location || "N/A"}
Status: ${e.status || "confirmed"}
---`;
              }).join("\n")
            }],
            ui: {
              viewType: "timeline",
              title: "Calendar Events",
              description: `${events.length} event(s) from ${new Date(timeMin).toLocaleDateString()} to ${new Date(timeMax).toLocaleDateString()}`,
              metadata: {
                source: "google-calendar",
                calendarId: args.calendarId || "primary",
                itemCount: events.length,
                timeRange: `${timeMin} - ${timeMax}`,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              events: events.map((e) => ({
                id: e.id,
                title: e.summary || "No title",
                description: e.description,
                timestamp: e.start?.dateTime || e.start?.date,
                endTimestamp: e.end?.dateTime || e.end?.date,
                location: e.location,
                status: e.status,
                attendees: e.attendees?.map((a) => a.email) || [],
                type: "calendar-event",
                color: e.status === "cancelled" ? "red" : e.start?.dateTime ? "blue" : "green"
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
        name: "google_send_email",
        description: "Send an email via Gmail",
        inputSchema: {
          type: "object",
          properties: {
            to: {
              type: "string",
              description: "Recipient email address"
            },
            subject: {
              type: "string",
              description: "Email subject"
            },
            body: {
              type: "string",
              description: "Email body (plain text)"
            },
            cc: {
              type: "string",
              description: "CC recipients (comma-separated)"
            }
          },
          required: ["to", "subject", "body"]
        }
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return {
            content: [{ type: "text", text: "Not authenticated. Please connect Google first." }],
            isError: true
          };
        }
        try {
          const headers = [
            `To: ${args.to}`,
            `Subject: ${args.subject}`,
            "MIME-Version: 1.0",
            'Content-Type: text/plain; charset="UTF-8"',
            "Content-Transfer-Encoding: 7bit"
          ];
          if (args.cc) {
            headers.unshift(`Cc: ${args.cc}`);
          }
          const rawEmail = `${headers.join("\r\n")}\r
\r
${args.body}`;
          const base64Encoded = btoa(rawEmail).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
          const response = await fetch(`${GMAIL_API_BASE}/messages/send`, {
            method: "POST",
            headers: {
              "Authorization": `Bearer ${token}`,
              "Accept": "application/json",
              "Content-Type": "application/json"
            },
            body: JSON.stringify({
              raw: base64Encoded
            })
          });
          if (!response.ok) {
            const errorData = await response.json();
            throw new Error(`Gmail API error: ${errorData.error?.message || response.statusText}`);
          }
          const result = await response.json();
          return {
            content: [{
              type: "text",
              text: `Email sent successfully to ${args.to}
Subject: ${args.subject}
Message ID: ${result.id}`
            }],
            ui: {
              viewType: "preview",
              title: "Email Sent",
              description: `Successfully sent to ${args.to}`,
              metadata: {
                source: "gmail",
                to: args.to,
                subject: args.subject,
                messageId: result.id,
                timestamp: (/* @__PURE__ */ new Date()).toISOString()
              },
              actions: [
                { label: "View Sent", intent: "show sent emails", variant: "secondary" }
              ]
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
setupWorkerServer(GoogleServer);
//# sourceMappingURL=google.js.map
