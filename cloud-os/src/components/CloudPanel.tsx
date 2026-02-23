import { FiCloud, FiServer, FiGlobe, FiShield, FiActivity, FiDatabase, FiRefreshCw, FiSearch, FiExternalLink, FiBox, FiFolder, FiGitPullRequest, FiMail, FiCalendar, FiHardDrive, FiSettings, FiArrowRight } from 'solid-icons/fi';
import { createSignal, For, Show, onMount, createMemo } from 'solid-js';
import { getToken } from '../lib/auth';
import { openWindow } from '../stores/windows';

function logError(message: string, error: unknown) {
  // Silently log errors for UI - these are expected for missing integrations
  // Only log in development
  if (import.meta.env?.DEV) {
    console.warn(message, error);
  }
}

type ResourceType = 'server' | 'domain' | 'deployment' | 'firewall' | 'function' | 'tunnel' | 'pages' | 'repository' | 'pullrequest' | 'email' | 'calendar' | 'storage';
type Provider = 'cloudflare' | 'vercel' | 'hetzner' | 'github' | 'google';

interface CloudResource {
  id: string;
  name: string;
  type: ResourceType;
  provider: Provider;
  status: 'running' | 'stopped' | 'active' | 'inactive' | 'error' | 'pending' | 'degraded' | 'open' | 'closed';
  region?: string;
  ip?: string;
  url?: string;
  metadata?: Record<string, any>;
  description?: string;
  actions?: Array<{
    label: string;
    action: string;
    icon: any;
    danger?: boolean;
  }>;
}

const providers: Record<string, { name: string; color: string; bg: string }> = {
  cloudflare: { name: 'Cloudflare', color: 'text-orange-400', bg: 'bg-orange-900/20' },
  vercel: { name: 'Vercel', color: 'text-white', bg: 'bg-white/10' },
  hetzner: { name: 'Hetzner', color: 'text-red-400', bg: 'bg-red-900/20' },
  github: { name: 'GitHub', color: 'text-white', bg: 'bg-neutral-700' },
  google: { name: 'Google', color: 'text-blue-400', bg: 'bg-blue-900/20' },
};

const resourceTypeIcons: Record<ResourceType, any> = {
  server: FiServer,
  domain: FiGlobe,
  deployment: FiActivity,
  firewall: FiShield,
  function: FiBox,
  tunnel: FiExternalLink,
  pages: FiGlobe,
  repository: FiFolder,
  pullrequest: FiGitPullRequest,
  email: FiMail,
  calendar: FiCalendar,
  storage: FiHardDrive,
};

const resourceTypeNames: Record<ResourceType, string> = {
  server: 'Servers',
  domain: 'Domains',
  deployment: 'Deployments',
  firewall: 'Firewalls',
  function: 'Functions',
  tunnel: 'Tunnels',
  pages: 'Pages',
  repository: 'Repositories',
  pullrequest: 'Pull Requests',
  email: 'Emails',
  calendar: 'Events',
  storage: 'Files',
};

export default function CloudPanel() {
  const [resources, setResources] = createSignal<CloudResource[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [searchQuery, setSearchQuery] = createSignal('');
  const [selectedProvider, setSelectedProvider] = createSignal<string | null>(null);
  const [selectedType, setSelectedType] = createSignal<ResourceType | null>(null);
  const [actionLoading, setActionLoading] = createSignal<string | null>(null);
  const [stats, setStats] = createSignal<Record<string, number>>({});

  const fetchAllResources = async () => {
    setLoading(true);
    setError(null);
    
    const allResources: CloudResource[] = [];
    const newStats: Record<string, number> = {};
    
    const cloudflareToken = getToken('cloudflare');
    const vercelToken = getToken('vercel');
    const hetznerToken = getToken('hetzner');
    const githubToken = localStorage.getItem('github_token');
    const googleToken = localStorage.getItem('google_token');
    
    try {
      if (cloudflareToken) {
        try {
          const zonesRes = await fetch(`/api/cloudflare/zones?token=${encodeURIComponent(cloudflareToken)}`);
          if (zonesRes.ok) {
            const zones = await zonesRes.json();
            newStats.domains = (newStats.domains || 0) + zones.length;
            for (const zone of zones) {
              allResources.push({
                id: `cf-zone-${zone.id}`,
                name: zone.name,
                type: 'domain',
                provider: 'cloudflare',
                status: zone.paused ? 'inactive' : 'active',
              });
            }
          }
        } catch (e) { logError('CF zones:', e); }

        try {
          const workersRes = await fetch(`/api/cloudflare/workers?token=${encodeURIComponent(cloudflareToken)}`);
          if (workersRes.ok) {
            const workers = await workersRes.json();
            newStats.functions = (newStats.functions || 0) + workers.length;
            for (const worker of workers) {
              allResources.push({
                id: `cf-worker-${worker.id}`,
                name: worker.name,
                type: 'function',
                provider: 'cloudflare',
                status: 'active',
              });
            }
          }
        } catch (e) { logError('CF workers:', e); }

        try {
          const pagesRes = await fetch(`/api/cloudflare/pages?token=${encodeURIComponent(cloudflareToken)}`);
          if (pagesRes.ok) {
            const pages = await pagesRes.json();
            newStats.pages = (newStats.pages || 0) + pages.length;
            for (const page of pages) {
              allResources.push({
                id: `cf-page-${page.id}`,
                name: page.name,
                type: 'pages',
                provider: 'cloudflare',
                status: 'active',
                url: page.subdomain ? `${page.name}.${page.subdomain}.pages.dev` : undefined,
              });
            }
          }
        } catch (e) { logError('CF pages:', e); }
      }
      
      if (vercelToken) {
        try {
          const projectsRes = await fetch(`/api/vercel/projects?token=${encodeURIComponent(vercelToken)}`);
          if (projectsRes.ok) {
            const projects = await projectsRes.json();
            newStats.deployments = (newStats.deployments || 0) + projects.length;
            for (const project of projects) {
              allResources.push({
                id: `vercel-project-${project.id}`,
                name: project.name,
                type: 'deployment',
                provider: 'vercel',
                status: 'active',
                metadata: { framework: project.framework },
                url: project.prodSettings?.url,
              });
              
              try {
                const deployRes = await fetch(`/api/vercel/deployments/${project.id}?token=${encodeURIComponent(vercelToken)}`);
                if (deployRes.ok) {
                  const deploys = await deployRes.json();
                  for (const deploy of deploys.slice(0, 2)) {
                    allResources.push({
                      id: `vercel-deploy-${deploy.uid}`,
                      name: `${project.name}`,
                      type: 'deployment',
                      provider: 'vercel',
                      status: deploy.state === 'READY' ? 'active' : deploy.state === 'ERROR' ? 'error' : 'pending',
                      metadata: { commit: deploy.meta?.githubCommitMessage?.slice(0, 40) },
                      url: deploy.url,
                    });
                  }
                }
              } catch (e) {
                // Silently ignore deployment fetch errors
              }
            }
          }
        } catch (e) { logError('Vercel projects:', e); }
      }

      if (hetznerToken) {
        try {
          const serversRes = await fetch(`/api/hetzner/servers?token=${encodeURIComponent(hetznerToken)}`);
          if (serversRes.ok) {
            const servers = await serversRes.json();
            newStats.servers = (newStats.servers || 0) + servers.length;
            for (const server of servers) {
              allResources.push({
                id: `hetzner-${server.id}`,
                name: server.name,
                type: 'server',
                provider: 'hetzner',
                status: server.status,
                region: server.datacenter?.location?.city,
                ip: server.public_net?.ipv4?.ip,
                actions: [
                  { label: 'Start', action: 'start', icon: FiServer },
                  { label: 'Stop', action: 'stop', icon: FiServer },
                  { label: 'Reboot', action: 'reboot', icon: FiRefreshCw },
                ],
              });
            }
          }
        } catch (e) { logError('Hetzner servers:', e); }

        try {
          const firewallsRes = await fetch(`/api/hetzner/firewalls?token=${encodeURIComponent(hetznerToken)}`);
          if (firewallsRes.ok) {
            const firewalls = await firewallsRes.json();
            newStats.firewalls = (newStats.firewalls || 0) + firewalls.length;
            for (const fw of firewalls) {
              allResources.push({
                id: `hetzner-fw-${fw.id}`,
                name: fw.name,
                type: 'firewall',
                provider: 'hetzner',
                status: 'active',
              });
            }
          }
        } catch (e) { logError('Hetzner firewalls:', e); }
      }

      if (githubToken) {
        try {
          const reposRes = await fetch('/api/github/repos');
          if ( reposRes.ok) {
            const repos = await reposRes.json();
            newStats.repositories = (newStats.repositories || 0) + repos.length;
            for (const repo of repos.slice(0, 10)) {
              allResources.push({
                id: `gh-repo-${repo.id}`,
                name: repo.full_name,
                type: 'repository',
                provider: 'github',
                status: 'active',
                url: repo.html_url,
                description: repo.description,
                metadata: { stars: repo.stargazers_count, private: repo.private },
              });
            }
          }
        } catch (e) { logError('GitHub repos:', e); }

        try {
          const userRes = await fetch('/api/github/user');
          if (userRes.ok) {
            const user = await userRes.json();
            if (user.login) {
              newStats.github = user.login;
            }
          }
        } catch (e) {
          // Silently ignore user fetch errors
        }
      }

      if (googleToken) {
        try {
          const messagesRes = await fetch('/api/google/messages?limit=10');
          if (messagesRes.ok) {
            const data = await messagesRes.json();
            const messages = data.messages || [];
            newStats.emails = (newStats.emails || 0) + messages.length;
            for (const msg of messages.slice(0, 5)) {
              allResources.push({
                id: `google-email-${msg.id}`,
                name: msg.subject?.slice(0, 40) || 'Untitled',
                type: 'email',
                provider: 'google',
                status: 'active',
                metadata: { from: msg.from?.name, date: msg.date },
              });
            }
          }
        } catch (e) { logError('Gmail:', e); }

        try {
          const eventsRes = await fetch('/api/google/events?limit=10');
          if (eventsRes.ok) {
            const data = await eventsRes.json();
            const events = data.items || [];
            newStats.events = (newStats.events || 0) + events.length;
            for (const event of events.slice(0, 5)) {
              allResources.push({
                id: `google-event-${event.id}`,
                name: event.summary?.slice(0, 40) || 'Untitled',
                type: 'calendar',
                provider: 'google',
                status: event.status === 'confirmed' ? 'active' : 'pending',
                metadata: { start: event.start?.dateTime || event.start?.date },
              });
            }
          }
        } catch (e) { logError('Calendar:', e); }
      }
      
      setStats(newStats);
      setResources(allResources);
    } catch (e) {
      setError('Failed to load cloud resources');
    }
    
    setLoading(false);
  };

  const handleAction = async (resource: CloudResource, action: string) => {
    if (resource.provider !== 'hetzner' || resource.type !== 'server') return;
    
    const token = getToken('hetzner');
    if (!token) return;
    
    const serverId = resource.id.replace('hetzner-', '');
    setActionLoading(`${resource.id}-${action}`);
    
    try {
      const res = await fetch(`/api/hetzner/servers?token=${encodeURIComponent(token)}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action, serverId }),
      });
      
      if (res.ok) {
        setTimeout(fetchAllResources, 1500);
      }
    } catch (e) {
      console.error('Action failed:', e);
    } finally {
      setActionLoading(null);
    }
  };

  const openProviderPanel = (provider: string) => {
    const panelMap: Record<string, string> = {
      cloudflare: 'cloudflare',
      vercel: 'vercel',
      hetzner: 'hetzner',
      github: 'github',
      google: 'email',
    };
    const panel = panelMap[provider];
    if (panel) openWindow(panel);
  };

  onMount(() => {
    fetchAllResources();
  });

  const filteredResources = createMemo(() => {
    let result = resources();
    
    if (searchQuery()) {
      const query = searchQuery().toLowerCase();
      result = result.filter(r => 
        r.name.toLowerCase().includes(query) ||
        r.provider.toLowerCase().includes(query) ||
        r.type.toLowerCase().includes(query) ||
        r.description?.toLowerCase().includes(query)
      );
    }
    
    if (selectedProvider()) {
      result = result.filter(r => r.provider === selectedProvider());
    }
    
    if (selectedType()) {
      result = result.filter(r => r.type === selectedType());
    }
    
    return result;
  });

  const resourcesByProvider = createMemo(() => {
    const grouped: Record<string, CloudResource[]> = {};
    for (const r of filteredResources()) {
      if (!grouped[r.provider]) grouped[r.provider] = [];
      grouped[r.provider].push(r);
    }
    return grouped;
  });

  const connectedProviders = createMemo(() => {
    const connected: string[] = [];
    if (getToken('cloudflare')) connected.push('cloudflare');
    if (getToken('vercel')) connected.push('vercel');
    if (getToken('hetzner')) connected.push('hetzner');
    if (localStorage.getItem('github_token')) connected.push('github');
    if (localStorage.getItem('google_token')) connected.push('google');
    return connected;
  });

  const totalResources = createMemo(() => resources().length);

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'running': case 'active': case 'open': return 'bg-green-900/50 text-green-400';
      case 'stopped': case 'inactive': case 'closed': return 'bg-neutral-700 text-neutral-400';
      case 'error': case 'degraded': return 'bg-red-900/50 text-red-400';
      case 'pending': return 'bg-yellow-900/50 text-yellow-400';
      default: return 'bg-neutral-700 text-neutral-400';
    }
  };

  return (
    <div class="h-full flex flex-col bg-[#1a1a1a] text-neutral-200 p-4">
      <div class="p-4 border-b border-neutral-800">
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-3">
            <FiCloud class="text-blue-400" size={24} />
            <h2 class="text-lg font-semibold">Cloud Resources</h2>
            <span class="text-xs text-neutral-500">({totalResources()})</span>
          </div>
          <button
            type="button"
            onClick={fetchAllResources}
            disabled={loading()}
            class="p-2 text-neutral-400 hover:text-white transition-colors disabled:opacity-50"
            title="Refresh"
          >
            <FiRefreshCw size={18} class={loading() ? 'animate-spin' : ''} />
          </button>
        </div>

        <Show when={Object.keys(stats()).length > 0}>
          <div class="flex gap-2 mb-4 overflow-x-auto pb-2">
            <For each={connectedProviders()}>
              {(provider) => (
                <button
                  type="button"
                  onClick={() => openProviderPanel(provider)}
                  class={`flex items-center gap-2 px-3 py-1.5 rounded-lg ${providers[provider]?.bg || 'bg-neutral-800'} hover:opacity-80 transition-opacity`}
                >
                  <span class={`text-sm font-medium ${providers[provider]?.color || 'text-white'}`}>
                    {providers[provider]?.name || provider}
                  </span>
                  <Show when={stats()[provider] && typeof stats()[provider] === 'number'}>
                    <span class="text-xs text-neutral-400">{stats()[provider as keyof typeof stats]}</span>
                  </Show>
                  <Show when={provider === 'github' && typeof stats().github === 'string'}>
                    <span class="text-xs text-neutral-400">@{stats().github}</span>
                  </Show>
                  <FiArrowRight size={12} class="text-neutral-500" />
                </button>
              )}
            </For>
            <button
              type="button"
              onClick={() => openWindow('integrations')}
              class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 transition-colors"
            >
              <FiSettings size={14} class="text-neutral-400" />
              <span class="text-sm text-neutral-400">Manage</span>
            </button>
          </div>
        </Show>

        <div class="flex gap-3">
          <div class="flex-1 relative">
            <FiSearch class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500" size={16} />
            <input
              type="text"
              value={searchQuery()}
              onInput={(e) => setSearchQuery(e.currentTarget.value)}
              placeholder="Search resources..."
              class="w-full pl-10 pr-4 py-2 bg-neutral-800 border border-neutral-700 rounded-lg text-sm"
            />
          </div>
          <select
            value={selectedProvider() || ''}
            onChange={(e) => setSelectedProvider(e.currentTarget.value || null)}
            class="px-3 py-2 bg-neutral-800 border border-neutral-700 rounded-lg text-sm"
          >
            <option value="">All Providers</option>
            <For each={connectedProviders()}>
              {(provider) => (
                <option value={provider}>{providers[provider]?.name || provider}</option>
              )}
            </For>
          </select>
          <select
            value={selectedType() || ''}
            onChange={(e) => setSelectedType(e.currentTarget.value as ResourceType || null)}
            class="px-3 py-2 bg-neutral-800 border border-neutral-700 rounded-lg text-sm"
          >
            <option value="">All Types</option>
            <For each={Object.entries(resourceTypeNames)}>
              {([id, name]) => (
                <option value={id}>{name}</option>
              )}
            </For>
          </select>
        </div>
      </div>

      <Show when={error()}>
        <div class="p-4 bg-red-900/50 border-b border-red-800">
          <p class="text-red-200 text-sm">{error()}</p>
        </div>
      </Show>

      <div class="flex-1 overflow-auto p-4">
        <Show when={loading()}>
          <div class="flex items-center justify-center h-full">
            <FiRefreshCw class="animate-spin text-blue-400" size={32} />
          </div>
        </Show>

        <Show when={!loading() && connectedProviders().length === 0}>
          <div class="flex flex-col items-center justify-center h-full text-center">
            <FiCloud size={48} class="text-neutral-600 mb-4" />
            <h3 class="text-lg font-medium mb-2">No Cloud Providers Connected</h3>
            <p class="text-neutral-400 text-sm mb-4">
              Connect a cloud provider to see your resources here
            </p>
            <button
              type="button"
              onClick={() => openWindow('integrations')}
              class="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded-lg text-sm"
            >
              Connect Provider
            </button>
          </div>
        </Show>

        <Show when={!loading() && connectedProviders().length > 0 && filteredResources().length === 0}>
          <div class="text-center py-8">
            <FiSearch size={32} class="text-neutral-600 mx-auto mb-2" />
            <p class="text-neutral-400">No resources match your filters</p>
          </div>
        </Show>

        <Show when={!loading() && filteredResources().length > 0}>
          <div class="space-y-6">
            <For each={Object.entries(resourcesByProvider())}>
              {([provider, providerResources]) => (
                <div>
                  <div class="flex items-center justify-between mb-3">
                    <div class="flex items-center gap-2">
                      <span class={`font-medium ${providers[provider]?.color || 'text-neutral-400'}`}>
                        {providers[provider]?.name || provider}
                      </span>
                      <span class="text-xs text-neutral-500">({providerResources.length})</span>
                    </div>
                    <button
                      type="button"
                      onClick={() => openProviderPanel(provider)}
                      class="text-xs text-blue-400 hover:text-blue-300 flex items-center gap-1"
                    >
                      Open <FiArrowRight size={12} />
                    </button>
                  </div>
                  <div class="space-y-2">
                    <For each={providerResources}>
                      {(resource) => (
                        <div class="flex items-center justify-between p-3 bg-neutral-800/50 rounded-lg hover:bg-neutral-800 transition-colors">
                          <div class="flex items-center gap-3 min-w-0">
                            <div class="p-2 bg-neutral-700 rounded-lg flex-shrink-0">
                              {(() => {
                                const Icon = resourceTypeIcons[resource.type] || FiServer;
                                return <Icon size={18} class="text-neutral-400" />;
                              })()}
                            </div>
                            <div class="min-w-0">
                              <p class="font-medium text-sm truncate">{resource.name}</p>
                              <p class="text-xs text-neutral-500 truncate">
                                {resourceTypeNames[resource.type] || resource.type}
                                <Show when={resource.region}> • {resource.region}</Show>
                                <Show when={resource.ip}> • {resource.ip}</Show>
                                <Show when={resource.metadata?.framework}> • {resource.metadata?.framework}</Show>
                                <Show when={resource.metadata?.stars}> ★ {resource.metadata?.stars}</Show>
                                <Show when={resource.metadata?.private}> • private</Show>
                              </p>
                            </div>
                          </div>
                          <div class="flex items-center gap-2 flex-shrink-0">
                            <span class={`text-xs px-2 py-1 rounded ${getStatusColor(resource.status)}`}>
                              {resource.status}
                            </span>
                            <Show when={resource.url}>
                              <a
                                href={resource.url?.startsWith('http') ? resource.url : `https://${resource.url}`}
                                target="_blank"
                                rel="noopener noreferrer"
                                class="p-1.5 text-neutral-400 hover:text-white transition-colors"
                                title="Open"
                              >
                                <FiExternalLink size={14} />
                              </a>
                            </Show>
                            <Show when={resource.actions}>
                              <div class="flex gap-1">
                                <For each={resource.actions}>
                                  {(act) => (
                                    <button
                                      type="button"
                                      onClick={() => handleAction(resource, act.action)}
                                      disabled={actionLoading() === `${resource.id}-${act.action}`}
                                      class={`p-1.5 rounded transition-colors ${
                                        act.danger 
                                          ? 'text-red-400 hover:bg-red-900/30' 
                                          : 'text-neutral-400 hover:text-white hover:bg-neutral-700'
                                      } disabled:opacity-50`}
                                      title={act.label}
                                    >
                                      <act.icon size={14} class={actionLoading() === `${resource.id}-${act.action}` ? 'animate-spin' : ''} />
                                    </button>
                                  )}
                                </For>
                              </div>
                            </Show>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>
    </div>
  );
}
