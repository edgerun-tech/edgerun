/**
 * Vercel MCP Server
 * Exposes Vercel API capabilities as MCP tools
 */

import { MCPServerBase, setupWorkerServer } from './base';

const VERCEL_API_BASE = 'https://api.vercel.com';

class VercelServer extends MCPServerBase {
  constructor() {
    super('vercel', '1.0.0');
  }

  setupHandlers(): void {
    // Get API token from main thread
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
          key: 'vercel_token',
        });

        setTimeout(() => {
          self.removeEventListener('message', handler);
          resolve(null);
        }, 1000);
      });
    };

    // Get team ID
    const getTeamId = async (token: string): Promise<string | null> => {
      try {
        const response = await fetch(`${VERCEL_API_BASE}/v2/teams`, {
          headers: { 'Authorization': `Bearer ${token}` },
        });
        if (response.ok) {
          const data = await response.json();
          return data.teams?.[0]?.id || null;
        }
      } catch {}
      return null;
    };

    // Register vercel_list_projects tool
    this.registerTool(
      {
        name: 'vercel_list_projects',
        description: 'List Vercel projects',
        inputSchema: {
          type: 'object',
          properties: {
            limit: { type: 'number', default: 20 },
          },
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { 
            content: [{ type: 'text', text: 'Not authenticated. Please add Vercel API token.' }], 
            isError: true 
          };
        }

        try {
          const teamId = await getTeamId(token);
          const url = teamId 
            ? `${VERCEL_API_BASE}/v9/projects?teamId=${teamId}&limit=${args.limit || 20}`
            : `${VERCEL_API_BASE}/v9/projects?limit=${args.limit || 20}`;

          const response = await fetch(url, {
            headers: { 'Authorization': `Bearer ${token}` },
          });

          if (!response.ok) throw new Error(`API error: ${response.statusText}`);

          const data = await response.json();
          const projects = data.projects || [];

          return {
            content: [{
              type: 'text',
              text: projects.map((p: any) => 
                `${p.name} (${p.framework || 'Static'}) - ${p.url}`
              ).join('\n'),
            }],
            ui: {
              viewType: 'file-grid',
              title: 'Vercel Projects',
              description: `${projects.length} project(s)`,
              metadata: {
                source: 'vercel',
                itemCount: projects.length,
                timestamp: new Date().toISOString(),
              },
              items: projects.map((p: any) => ({
                id: p.id,
                name: p.name,
                type: 'project',
                url: p.url,
                framework: p.framework,
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

    // Register vercel_get_deployments tool
    this.registerTool(
      {
        name: 'vercel_get_deployments',
        description: 'Get deployment history for a Vercel project',
        inputSchema: {
          type: 'object',
          properties: {
            project: { type: 'string', description: 'Project name or ID' },
            limit: { type: 'number', default: 10 },
          },
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { 
            content: [{ type: 'text', text: 'Not authenticated. Please add Vercel API token.' }], 
            isError: true 
          };
        }

        try {
          const teamId = await getTeamId(token);
          const params = new URLSearchParams({
            limit: String(args.limit || 10),
            ...(args.project ? { projectId: args.project } : {}),
          });

          const url = `${VERCEL_API_BASE}/v6/deployments?${params}${teamId ? `&teamId=${teamId}` : ''}`;

          const response = await fetch(url, {
            headers: { 'Authorization': `Bearer ${token}` },
          });

          if (!response.ok) throw new Error(`API error: ${response.statusText}`);

          const data = await response.json();
          const deployments = data.deployments || [];

          return {
            content: [{
              type: 'text',
              text: deployments.map((d: any) => 
                `${d.url} - ${d.state} (${d.target}, ${new Date(d.created).toLocaleString()})`
              ).join('\n'),
            }],
            ui: {
              viewType: 'timeline',
              title: 'Vercel Deployments',
              description: args.project ? `Deployments for ${args.project}` : 'Recent deployments',
              metadata: {
                source: 'vercel',
                project: args.project,
                itemCount: deployments.length,
                timestamp: new Date().toISOString(),
              },
              events: deployments.map((d: any) => ({
                id: d.id,
                title: d.project?.name || 'Unknown',
                description: `${d.url} - ${d.target}`,
                timestamp: new Date(d.created).toISOString(),
                type: 'deployment',
                status: d.state === 'READY' ? 'success' : d.state === 'ERROR' ? 'error' : 'pending',
                metadata: {
                  url: d.url,
                  state: d.state,
                  target: d.target,
                  commitRef: d.ref,
                },
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

    // Register vercel_list_domains tool
    this.registerTool(
      {
        name: 'vercel_list_domains',
        description: 'List domains configured in Vercel',
        inputSchema: {
          type: 'object',
          properties: {},
        },
      },
      async () => {
        const token = await getToken();
        if (!token) {
          return { 
            content: [{ type: 'text', text: 'Not authenticated. Please add Vercel API token.' }], 
            isError: true 
          };
        }

        try {
          const teamId = await getTeamId(token);
          const url = `${VERCEL_API_BASE}/v9/domains${teamId ? `?teamId=${teamId}` : ''}`;

          const response = await fetch(url, {
            headers: { 'Authorization': `Bearer ${token}` },
          });

          if (!response.ok) throw new Error(`API error: ${response.statusText}`);

          const data = await response.json();
          const domains = data.domains || [];

          return {
            content: [{
              type: 'text',
              text: domains.map((d: any) => 
                `${d.name} (${d.verified ? '✓ verified' : '○ pending'})`
              ).join('\n'),
            }],
            ui: {
              viewType: 'data-table',
              title: 'Vercel Domains',
              description: `${domains.length} domain(s)`,
              metadata: {
                source: 'vercel',
                itemCount: domains.length,
                timestamp: new Date().toISOString(),
              },
              columns: ['Domain', 'Status', 'Redirect'],
              rows: domains.map((d: any) => [
                d.name,
                d.verified ? 'Verified' : 'Pending',
                d.redirect || '-',
              ]),
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

    // Register vercel_get_logs tool
    this.registerTool(
      {
        name: 'vercel_get_logs',
        description: 'Get deployment logs from Vercel',
        inputSchema: {
          type: 'object',
          properties: {
            deploymentId: { type: 'string', description: 'Deployment ID' },
            limit: { type: 'number', default: 50 },
          },
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { 
            content: [{ type: 'text', text: 'Not authenticated. Please add Vercel API token.' }], 
            isError: true 
          };
        }

        if (!args.deploymentId) {
          return {
            content: [{ type: 'text', text: 'Error: deploymentId is required' }],
            isError: true,
          };
        }

        try {
          const teamId = await getTeamId(token);
          const url = `${VERCEL_API_BASE}/v1/deployments/${args.deploymentId}/logs${teamId ? `?teamId=${teamId}` : ''}`;

          const response = await fetch(url, {
            headers: { 'Authorization': `Bearer ${token}` },
          });

          if (!response.ok) throw new Error(`API error: ${response.statusText}`);

          const data = await response.json();
          const logs = data.logs || [];

          return {
            content: [{
              type: 'text',
              text: logs.map((l: any) => `[${new Date(l.time).toISOString()}] ${l.level?.toUpperCase()}: ${l.text}`).join('\n'),
            }],
            ui: {
              viewType: 'log-viewer',
              title: 'Deployment Logs',
              description: `Logs for ${args.deploymentId}`,
              metadata: {
                source: 'vercel',
                deploymentId: args.deploymentId,
                itemCount: logs.length,
                timestamp: new Date().toISOString(),
              },
              logs: logs.map((l: any) => ({
                timestamp: new Date(l.time).toISOString(),
                level: l.level || 'info',
                message: l.text,
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
  }
}

setupWorkerServer(VercelServer);
