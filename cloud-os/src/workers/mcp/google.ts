/**
 * Google MCP Server
 * Exposes Google API capabilities (Gmail, Calendar) as MCP tools
 */

import { MCPServerBase, setupWorkerServer } from './base';

const GMAIL_API_BASE = 'https://www.googleapis.com/gmail/v1';
const CALENDAR_API_BASE = 'https://www.googleapis.com/calendar/v3';

class GoogleServer extends MCPServerBase {
  constructor() {
    super('google', '1.0.0');
  }

  setupHandlers(): void {
    // Get OAuth token from main thread
    const getToken = async (): Promise<string | null> => {
      return new Promise((resolve) => {
        const requestId = Date.now().toString();

        const handler = (event: MessageEvent) => {
          if (event.data?.type === 'token:response' && event.data?.requestId === requestId) {
            self.removeEventListener('message', handler);
            resolve(event.data.token);
          }
        };

        self.addEventListener('message', handler);

        self.postMessage({
          type: 'token:request',
          requestId,
          key: 'google_token',
        });

        setTimeout(() => {
          self.removeEventListener('message', handler);
          resolve(null);
        }, 1000);
      });
    };

    // Register get_emails tool
    this.registerTool(
      {
        name: 'google_get_emails',
        description: 'Get emails from Gmail with search support',
        inputSchema: {
          type: 'object',
          properties: {
            query: {
              type: 'string',
              description: 'Gmail search query (e.g., "from:boss@example.com", "label:inbox", "has:attachment")',
              default: 'label:inbox',
            },
            maxResults: {
              type: 'number',
              description: 'Maximum number of emails to return',
              default: 10,
            },
            includeBody: {
              type: 'boolean',
              description: 'Whether to include email body in response',
              default: false,
            },
          },
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { 
            content: [{ type: 'text', text: 'Not authenticated. Please connect Google first.' }], 
            isError: true 
          };
        }

        try {
          // First, list messages
          const listUrl = `${GMAIL_API_BASE}/messages?q=${encodeURIComponent(args.query || 'label:inbox')}&maxResults=${args.maxResults || 10}`;
          
          const listResponse = await fetch(listUrl, {
            headers: { 
              'Authorization': `Bearer ${token}`,
              'Accept': 'application/json',
            },
          });

          if (!listResponse.ok) {
            throw new Error(`Gmail API error: ${listResponse.statusText}`);
          }

          const listData = await listResponse.json();
          const messages = listData.messages || [];

          if (messages.length === 0) {
            return {
              content: [{ type: 'text', text: 'No emails found matching your query.' }],
              ui: {
                viewType: 'email-reader',
                title: 'Gmail',
                description: 'No results',
                metadata: {
                  source: 'gmail',
                  query: args.query,
                  itemCount: 0,
                  timestamp: new Date().toISOString(),
                },
                emails: [],
              },
            };
          }

          // Fetch full message details
          const emailPromises = messages.slice(0, args.maxResults || 10).map(async (msg: any) => {
            const detailUrl = `${GMAIL_API_BASE}/messages/${msg.id}?format=metadata&metadataHeaders=From&metadataHeaders=To&metadataHeaders=Subject&metadataHeaders=Date`;
            const detailResponse = await fetch(detailUrl, {
              headers: { 
                'Authorization': `Bearer ${token}`,
                'Accept': 'application/json',
              },
            });
            
            if (!detailResponse.ok) return null;
            
            const detail = await detailResponse.json();
            const headers = detail.payload?.headers || [];
            
            const getHeader = (name: string) => headers.find((h: any) => h.name === name)?.value || '';
            
            return {
              id: msg.id,
              threadId: msg.threadId,
              from: getHeader('From'),
              to: getHeader('To'),
              subject: getHeader('Subject'),
              date: getHeader('Date'),
              snippet: detail.snippet,
            };
          });

          const emails = (await Promise.all(emailPromises)).filter(Boolean);

          return {
            content: [{
              type: 'text',
              text: emails.map((e: any) => 
                `From: ${e.from}\nSubject: ${e.subject}\nDate: ${e.date}\n${e.snippet}\n---`
              ).join('\n'),
            }],
            ui: {
              viewType: 'email-reader',
              title: 'Gmail',
              description: `Found ${emails.length} email(s) for "${args.query}"`,
              metadata: {
                source: 'gmail',
                query: args.query,
                itemCount: emails.length,
                timestamp: new Date().toISOString(),
              },
              emails: emails.map((e: any) => ({
                id: e.id,
                from: e.from,
                to: e.to,
                subject: e.subject,
                date: e.date,
                snippet: e.snippet,
                unread: false, // Would need labels info to determine
              })),
            },
          };
        } catch (error) {
          return { 
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }], 
            isError: true 
          };
        }
      }
    );

    // Register get_email_detail tool
    this.registerTool(
      {
        name: 'google_get_email_detail',
        description: 'Get full content of a specific email',
        inputSchema: {
          type: 'object',
          properties: {
            messageId: {
              type: 'string',
              description: 'Gmail message ID',
            },
          },
          required: ['messageId'],
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { 
            content: [{ type: 'text', text: 'Not authenticated. Please connect Google first.' }], 
            isError: true 
          };
        }

        try {
          const response = await fetch(`${GMAIL_API_BASE}/messages/${args.messageId}?format=full`, {
            headers: { 
              'Authorization': `Bearer ${token}`,
              'Accept': 'application/json',
            },
          });

          if (!response.ok) {
            throw new Error(`Gmail API error: ${response.statusText}`);
          }

          const message = await response.json();
          const headers = message.payload?.headers || [];
          const getHeader = (name: string) => headers.find((h: any) => h.name === name)?.value || '';

          // Decode body
          let body = '';
          if (message.payload?.body?.data) {
            body = atob(message.payload.body.data);
          } else if (message.payload?.parts?.[0]?.body?.data) {
            body = atob(message.payload.parts[0].body.data);
          }

          return {
            content: [{
              type: 'text',
              text: `From: ${getHeader('From')}\nTo: ${getHeader('To')}\nSubject: ${getHeader('Subject')}\nDate: ${getHeader('Date')}\n\n${body}`,
            }],
            ui: {
              viewType: 'email-reader',
              title: getHeader('Subject') || 'Email',
              description: `From: ${getHeader('From')}`,
              metadata: {
                source: 'gmail',
                messageId: args.messageId,
                from: getHeader('From'),
                to: getHeader('To'),
                subject: getHeader('Subject'),
                date: getHeader('Date'),
                timestamp: new Date().toISOString(),
              },
              email: {
                id: message.id,
                from: getHeader('From'),
                to: getHeader('To'),
                subject: getHeader('Subject'),
                date: getHeader('Date'),
                body: body,
                attachments: message.payload?.parts?.filter((p: any) => p.filename).map((p: any) => ({
                  filename: p.filename,
                  mimeType: p.mimeType,
                  size: p.body.size,
                })) || [],
              },
            },
          };
        } catch (error) {
          return { 
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }], 
            isError: true 
          };
        }
      }
    );

    // Register get_calendar_events tool
    this.registerTool(
      {
        name: 'google_get_calendar_events',
        description: 'Get events from Google Calendar',
        inputSchema: {
          type: 'object',
          properties: {
            calendarId: {
              type: 'string',
              description: 'Calendar ID (defaults to primary)',
              default: 'primary',
            },
            timeMin: {
              type: 'string',
              description: 'Start time (ISO 8601, defaults to now)',
            },
            timeMax: {
              type: 'string',
              description: 'End time (ISO 8601, defaults to 7 days from now)',
            },
            maxResults: {
              type: 'number',
              description: 'Maximum number of events',
              default: 10,
            },
            singleEvents: {
              type: 'boolean',
              description: 'Whether to expand recurring events',
              default: true,
            },
          },
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { 
            content: [{ type: 'text', text: 'Not authenticated. Please connect Google first.' }], 
            isError: true 
          };
        }

        try {
          const now = new Date();
          const timeMin = args.timeMin || now.toISOString();
          const timeMax = args.timeMax || new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000).toISOString();

          let url = `${CALENDAR_API_BASE}/calendars/${encodeURIComponent(args.calendarId || 'primary')}/events?`;
          url += `timeMin=${encodeURIComponent(timeMin)}&`;
          url += `timeMax=${encodeURIComponent(timeMax)}&`;
          url += `maxResults=${args.maxResults || 10}&`;
          url += `singleEvents=${args.singleEvents !== false}&`;
          url += `orderBy=startTime`;

          const response = await fetch(url, {
            headers: { 
              'Authorization': `Bearer ${token}`,
              'Accept': 'application/json',
            },
          });

          if (!response.ok) {
            throw new Error(`Calendar API error: ${response.statusText}`);
          }

          const data = await response.json();
          const events = data.items || [];

          if (events.length === 0) {
            return {
              content: [{ type: 'text', text: 'No calendar events found for the specified time range.' }],
              ui: {
                viewType: 'timeline',
                title: 'Calendar',
                description: 'No events',
                metadata: {
                  source: 'google-calendar',
                  calendarId: args.calendarId || 'primary',
                  itemCount: 0,
                  timeRange: `${timeMin} - ${timeMax}`,
                  timestamp: new Date().toISOString(),
                },
                events: [],
              },
            };
          }

          return {
            content: [{
              type: 'text',
              text: events.map((e: any) => {
                const start = e.start?.dateTime || e.start?.date;
                const end = e.end?.dateTime || e.end?.date;
                return `${e.summary || 'No title'}\nWhen: ${start} - ${end}\nWhere: ${e.location || 'N/A'}\nStatus: ${e.status || 'confirmed'}\n---`;
              }).join('\n'),
            }],
            ui: {
              viewType: 'timeline',
              title: 'Calendar Events',
              description: `${events.length} event(s) from ${new Date(timeMin).toLocaleDateString()} to ${new Date(timeMax).toLocaleDateString()}`,
              metadata: {
                source: 'google-calendar',
                calendarId: args.calendarId || 'primary',
                itemCount: events.length,
                timeRange: `${timeMin} - ${timeMax}`,
                timestamp: new Date().toISOString(),
              },
              events: events.map((e: any) => ({
                id: e.id,
                title: e.summary || 'No title',
                description: e.description,
                timestamp: e.start?.dateTime || e.start?.date,
                endTimestamp: e.end?.dateTime || e.end?.date,
                location: e.location,
                status: e.status,
                attendees: e.attendees?.map((a: any) => a.email) || [],
                type: 'calendar-event',
                color: e.status === 'cancelled' ? 'red' : e.start?.dateTime ? 'blue' : 'green',
              })),
            },
          };
        } catch (error) {
          return { 
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }], 
            isError: true 
          };
        }
      }
    );

    // Register send_email tool
    this.registerTool(
      {
        name: 'google_send_email',
        description: 'Send an email via Gmail',
        inputSchema: {
          type: 'object',
          properties: {
            to: {
              type: 'string',
              description: 'Recipient email address',
            },
            subject: {
              type: 'string',
              description: 'Email subject',
            },
            body: {
              type: 'string',
              description: 'Email body (plain text)',
            },
            cc: {
              type: 'string',
              description: 'CC recipients (comma-separated)',
            },
          },
          required: ['to', 'subject', 'body'],
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { 
            content: [{ type: 'text', text: 'Not authenticated. Please connect Google first.' }], 
            isError: true 
          };
        }

        try {
          // Create RFC 2822 compliant email
          const headers = [
            `To: ${args.to}`,
            `Subject: ${args.subject}`,
            'MIME-Version: 1.0',
            'Content-Type: text/plain; charset="UTF-8"',
            'Content-Transfer-Encoding: 7bit',
          ];

          if (args.cc) {
            headers.unshift(`Cc: ${args.cc}`);
          }

          const rawEmail = `${headers.join('\r\n')}\r\n\r\n${args.body}`;
          const base64Encoded = btoa(rawEmail)
            .replace(/\+/g, '-')
            .replace(/\//g, '_')
            .replace(/=+$/, '');

          const response = await fetch(`${GMAIL_API_BASE}/messages/send`, {
            method: 'POST',
            headers: {
              'Authorization': `Bearer ${token}`,
              'Accept': 'application/json',
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({
              raw: base64Encoded,
            }),
          });

          if (!response.ok) {
            const errorData = await response.json();
            throw new Error(`Gmail API error: ${errorData.error?.message || response.statusText}`);
          }

          const result = await response.json();

          return {
            content: [{
              type: 'text',
              text: `Email sent successfully to ${args.to}\nSubject: ${args.subject}\nMessage ID: ${result.id}`,
            }],
            ui: {
              viewType: 'preview',
              title: 'Email Sent',
              description: `Successfully sent to ${args.to}`,
              metadata: {
                source: 'gmail',
                to: args.to,
                subject: args.subject,
                messageId: result.id,
                timestamp: new Date().toISOString(),
              },
              actions: [
                { label: 'View Sent', intent: 'show sent emails', variant: 'secondary' },
              ],
            },
          };
        } catch (error) {
          return { 
            content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }], 
            isError: true 
          };
        }
      }
    );
  }
}

setupWorkerServer(GoogleServer);
