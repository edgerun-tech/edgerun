/**
 * Cloudflare MCP Server
 * Exposes Cloudflare API capabilities as MCP tools
 */

import { MCPServerBase, setupWorkerServer } from './base';

const CLOUDFLARE_API_BASE = 'https://api.cloudflare.com/client/v4';

class CloudflareServer extends MCPServerBase {
  constructor() {
    super('cloudflare', '1.0.0');
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
          key: 'cloudflare_token',
        });

        setTimeout(() => {
          self.removeEventListener('message', handler);
          resolve(null);
        }, 1000);
      });
    };

    this.registerTool(
      {
        name: 'cloudflare_get_account',
        description: 'Get Cloudflare account details',
        inputSchema: {
          type: 'object',
          properties: {},
        },
      },
      async () => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: 'text', text: 'Not authenticated. Please add Cloudflare API token.' }], isError: true };
        }

        try {
          const response = await fetch(`${CLOUDFLARE_API_BASE}/user`, {
            headers: { 'Authorization': `Bearer ${token}` }
          });
          
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          
          const data = await response.json();
          return { content: [{ type: 'text', text: JSON.stringify(data.result, null, 2) }] };
        } catch (error) {
          return { content: [{ type: 'text', text: `Error: ${error}` }], isError: true };
        }
      }
    );

    this.registerTool(
      {
        name: 'cloudflare_list_zones',
        description: 'List DNS zones in Cloudflare account',
        inputSchema: {
          type: 'object',
          properties: {
            limit: { type: 'number', default: 50 },
          },
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: 'text', text: 'Not authenticated. Please add Cloudflare API token.' }], isError: true };
        }

        try {
          const response = await fetch(`${CLOUDFLARE_API_BASE}/zones?per_page=${args.limit || 50}`, {
            headers: { 'Authorization': `Bearer ${token}` }
          });
          
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          
          const data = await response.json();
          const zones = data.result.map((z: any) => ({
            id: z.id,
            name: z.name,
            status: z.status,
            plan: z.plan.name,
            created: z.created_on,
          }));
          
          return { content: [{ type: 'text', text: JSON.stringify(zones, null, 2) }] };
        } catch (error) {
          return { content: [{ type: 'text', text: `Error: ${error}` }], isError: true };
        }
      }
    );

    this.registerTool(
      {
        name: 'cloudflare_list_dns',
        description: 'List DNS records in a zone',
        inputSchema: {
          type: 'object',
          properties: {
            zone_id: { type: 'string', description: 'Zone ID' },
            type: { type: 'string', description: 'Filter by record type' },
          },
          required: ['zone_id'],
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: 'text', text: 'Not authenticated.' }], isError: true };
        }

        try {
          let url = `${CLOUDFLARE_API_BASE}/zones/${args.zone_id}/dns_records?per_page=100`;
          if (args.type) url += `&type=${args.type}`;
          
          const response = await fetch(url, {
            headers: { 'Authorization': `Bearer ${token}` }
          });
          
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          
          const data = await response.json();
          const records = data.result.map((r: any) => ({
            id: r.id,
            type: r.type,
            name: r.name,
            content: r.content,
            proxied: r.proxiable ? r.proxied : undefined,
          }));
          
          return { content: [{ type: 'text', text: JSON.stringify(records, null, 2) }] };
        } catch (error) {
          return { content: [{ type: 'text', text: `Error: ${error}` }], isError: true };
        }
      }
    );

    this.registerTool(
      {
        name: 'cloudflare_create_dns',
        description: 'Create a DNS record',
        inputSchema: {
          type: 'object',
          properties: {
            zone_id: { type: 'string' },
            type: { type: 'string', enum: ['A', 'AAAA', 'CNAME', 'MX', 'TXT', 'SPF', 'SRV'] },
            name: { type: 'string' },
            content: { type: 'string' },
            proxied: { type: 'boolean', default: true },
          },
          required: ['zone_id', 'type', 'name', 'content'],
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: 'text', text: 'Not authenticated.' }], isError: true };
        }

        try {
          const response = await fetch(`${CLOUDFLARE_API_BASE}/zones/${args.zone_id}/dns_records`, {
            method: 'POST',
            headers: { 
              'Authorization': `Bearer ${token}`,
              'Content-Type': 'application/json'
            },
            body: JSON.stringify({
              type: args.type,
              name: args.name,
              content: args.content,
              proxied: args.proxied,
            })
          });
          
          if (!response.ok) {
            const err = await response.json();
            throw new Error(err.errors?.[0]?.message || response.statusText);
          }
          
          const data = await response.json();
          return { content: [{ type: 'text', text: `Created ${args.type} record: ${args.name} -> ${args.content}` }] };
        } catch (error) {
          return { content: [{ type: 'text', text: `Error: ${error}` }], isError: true };
        }
      }
    );

    this.registerTool(
      {
        name: 'cloudflare_delete_dns',
        description: 'Delete a DNS record',
        inputSchema: {
          type: 'object',
          properties: {
            zone_id: { type: 'string' },
            record_id: { type: 'string' },
          },
          required: ['zone_id', 'record_id'],
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: 'text', text: 'Not authenticated.' }], isError: true };
        }

        try {
          const response = await fetch(
            `${CLOUDFLARE_API_BASE}/zones/${args.zone_id}/dns_records/${args.record_id}`,
            { method: 'DELETE', headers: { 'Authorization': `Bearer ${token}` } }
          );
          
          if (!response.ok) throw new Error(`API error: ${response.statusText}`);
          
          return { content: [{ type: 'text', text: 'DNS record deleted successfully' }] };
        } catch (error) {
          return { content: [{ type: 'text', text: `Error: ${error}` }], isError: true };
        }
      }
    );

    this.registerTool(
      {
        name: 'cloudflare_list_workers',
        description: 'List Cloudflare Workers',
        inputSchema: { type: 'object', properties: {} },
      },
      async () => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: 'text', text: 'Not authenticated.' }], isError: true };
        }

        try {
          const response = await fetch(`${CLOUDFLARE_API_BASE}/accounts/workers/scripts`, {
            headers: { 'Authorization': `Bearer ${token}` }
          });

          if (!response.ok) throw new Error(`API error: ${response.statusText}`);

          const data = await response.json();
          return { content: [{ type: 'text', text: JSON.stringify(data.result, null, 2) }] };
        } catch (error) {
          return { content: [{ type: 'text', text: `Error: ${error}` }], isError: true };
        }
      }
    );

    this.registerTool(
      {
        name: 'cloudflare_list_tunnels',
        description: 'List Cloudflare Tunnels',
        inputSchema: { type: 'object', properties: {} },
      },
      async () => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: 'text', text: 'Not authenticated.' }], isError: true };
        }

        try {
          const response = await fetch(`${CLOUDFLARE_API_BASE}/accounts`, {
            headers: { 'Authorization': `Bearer ${token}` }
          });

          if (!response.ok) throw new Error(`API error: ${response.statusText}`);

          const account = (await response.json()).result[0];
          if (!account) throw new Error('No account found');

          const tunnelsRes = await fetch(`${CLOUDFLARE_API_BASE}/accounts/${account.id}/tunnels`, {
            headers: { 'Authorization': `Bearer ${token}` }
          });

          const tunnels = (await tunnelsRes.json()).result || [];
          return { content: [{ type: 'text', text: JSON.stringify(tunnels, null, 2) }] };
        } catch (error) {
          return { content: [{ type: 'text', text: `Error: ${error}` }], isError: true };
        }
      }
    );

    // Register get_deployments tool
    this.registerTool(
      {
        name: 'cloudflare_get_deployments',
        description: 'Get Cloudflare Workers deployment history',
        inputSchema: {
          type: 'object',
          properties: {
            script: {
              type: 'string',
              description: 'Worker script name',
            },
            limit: {
              type: 'number',
              description: 'Maximum number of deployments to return',
              default: 10,
            },
          },
        },
      },
      async (args) => {
        const token = await getToken();
        if (!token) {
          return { content: [{ type: 'text', text: 'Not authenticated. Please add Cloudflare API token.' }], isError: true };
        }

        try {
          // First get account ID
          const accountResponse = await fetch(`${CLOUDFLARE_API_BASE}/user`, {
            headers: { 'Authorization': `Bearer ${token}` }
          });

          if (!accountResponse.ok) throw new Error(`API error: ${accountResponse.statusText}`);

          const accountData = await accountResponse.json();
          const accountId = accountData.result.account?.id;

          if (!accountId) {
            return { content: [{ type: 'text', text: 'No account ID found' }], isError: true };
          }

          // Get deployments
          let url = `${CLOUDFLARE_API_BASE}/accounts/${accountId}/deployments?per_page=${args.limit || 10}`;
          if (args.script) {
            url += `&script_name=${encodeURIComponent(args.script)}`;
          }

          const response = await fetch(url, {
            headers: { 'Authorization': `Bearer ${token}` }
          });

          if (!response.ok) throw new Error(`API error: ${response.statusText}`);

          const data = await response.json();
          const deployments = data.result || [];

          return {
            content: [{
              type: 'text',
              text: deployments.map((d: any) => 
                `${d.deployment_id.slice(0, 8)} - ${d.script?.tag || 'N/A'} (${d.environment}, ${d.stage}, ${new Date(d.created_on).toLocaleString()})`
              ).join('\n'),
            }],
            ui: {
              viewType: 'timeline',
              title: 'Cloudflare Deployments',
              description: args.script ? `Deployments for ${args.script}` : 'Recent deployments',
              metadata: {
                source: 'cloudflare',
                accountId,
                script: args.script,
                itemCount: deployments.length,
                timestamp: new Date().toISOString(),
              },
              events: deployments.map((d: any) => ({
                id: d.deployment_id,
                title: d.script?.tag || d.script?.name || 'Unknown',
                description: `Environment: ${d.environment}, Stage: ${d.stage}`,
                timestamp: d.created_on,
                type: 'deployment',
                status: d.stage === 'production' ? 'success' : 'pending',
                metadata: {
                  deploymentId: d.deployment_id,
                  environment: d.environment,
                  stage: d.stage,
                  url: d.url,
                },
              })),
            },
          };
        } catch (error) {
          return { content: [{ type: 'text', text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }], isError: true };
        }
      }
    );
  }
}

setupWorkerServer(CloudflareServer);
