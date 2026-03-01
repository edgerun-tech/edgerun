import { FiCloud, FiServer, FiGlobe, FiShield, FiActivity, FiRefreshCw, FiSearch, FiExternalLink, FiBox, FiFolder, FiGitPullRequest, FiMail, FiCalendar, FiHardDrive, FiSettings, FiArrowRight } from "solid-icons/fi";
import { createSignal, For, Show, onMount, onCleanup, createMemo } from "solid-js";
import { getToken } from "../../lib/auth";
import { localBridgeHttpUrl } from "../../lib/local-bridge-origin";
import { openWindow } from "../../stores/windows";
import { openWorkflowIntegrations } from "../../stores/workflow-ui";
import { integrationStore } from "../../stores/integrations";
import IntentContextMenu from "../layout/IntentContextMenu";
function logError(message, error) {
  if (import.meta.env?.DEV) {
    console.warn(message, error);
  }
}
const providers = {
  cloudflare: { name: "Cloudflare", color: "text-orange-400", bg: "bg-orange-900/20" },
  vercel: { name: "Vercel", color: "text-white", bg: "bg-white/10" },
  hetzner: { name: "Hetzner", color: "text-red-400", bg: "bg-red-900/20" },
  github: { name: "GitHub", color: "text-white", bg: "bg-neutral-700" },
  google: { name: "Google", color: "text-blue-400", bg: "bg-blue-900/20" },
  docker: { name: "Docker", color: "text-cyan-300", bg: "bg-cyan-900/20" }
};
const resourceTypeIcons = {
  server: FiServer,
  domain: FiGlobe,
  deployment: FiActivity,
  firewall: FiShield,
  function: FiBox,
  tunnel: FiExternalLink,
  pages: FiGlobe,
  repository: FiFolder,
  pullrequest: FiGitPullRequest,
  workflow: FiActivity,
  email: FiMail,
  calendar: FiCalendar,
  storage: FiHardDrive,
  service: FiBox,
  container: FiBox
};
const resourceTypeNames = {
  server: "Servers",
  domain: "Domains",
  deployment: "Deployments",
  firewall: "Firewalls",
  function: "Functions",
  tunnel: "Tunnels",
  pages: "Pages",
  repository: "Repositories",
  pullrequest: "Pull Requests",
  workflow: "Workflow Runs",
  email: "Emails",
  calendar: "Events",
  storage: "Files",
  service: "Services",
  container: "Containers"
};
function CloudPanel(props) {
  const compact = () => Boolean(props?.compact);
  const panelButtonClass = "inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]";
  const [resources, setResources] = createSignal([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal(null);
  const [searchQuery, setSearchQuery] = createSignal("");
  const [selectedProvider, setSelectedProvider] = createSignal(null);
  const [selectedType, setSelectedType] = createSignal(null);
  const [actionLoading, setActionLoading] = createSignal(null);
  const [stats, setStats] = createSignal({});
  const [localDocker, setLocalDocker] = createSignal({ available: false, swarmActive: false });
  const [notice, setNotice] = createSignal("");
  const [contextMenuState, setContextMenuState] = createSignal({
    open: false,
    x: 0,
    y: 0,
    resource: null
  });
  const fetchAllResources = async () => {
    setLoading(true);
    setError(null);
    const allResources = [];
    const newStats = {};
    const cloudflareToken = getToken("cloudflare");
    const vercelToken = getToken("vercel");
    const hetznerToken = getToken("hetzner");
    const githubToken = integrationStore.getToken("github");
    const googleToken = integrationStore.getToken("google");
    try {
      try {
        const dockerRes = await fetch(localBridgeHttpUrl("/v1/local/docker/summary"), { cache: "no-store" });
        if (dockerRes.ok) {
          const docker = await dockerRes.json();
          if (docker?.ok) {
            const services = Array.isArray(docker.services) ? docker.services : [];
            const containers = Array.isArray(docker.containers) ? docker.containers : [];
            setLocalDocker({
              available: true,
              swarmActive: Boolean(docker.swarm_active)
            });
            newStats.docker = services.length + containers.length;
            newStats.dockerServices = services.length;
            newStats.dockerContainers = containers.length;
            for (const svc of services) {
              allResources.push({
                id: `docker-svc-${svc.id}`,
                name: svc.name || svc.id,
                type: "service",
                provider: "docker",
                status: (svc.replicas || "").startsWith("0/") ? "inactive" : "active",
                metadata: {
                  mode: svc.mode || "",
                  replicas: svc.replicas || "",
                  image: svc.image || "",
                  ports: svc.ports || ""
                }
              });
            }
            for (const ctr of containers) {
              allResources.push({
                id: `docker-ctr-${ctr.id}`,
                name: ctr.name || ctr.id,
                type: "container",
                provider: "docker",
                status: ctr.state || ctr.status || "unknown",
                metadata: {
                  containerId: ctr.id || "",
                  containerName: ctr.name || ctr.id || "",
                  image: ctr.image || "",
                  status: ctr.status || "",
                  ports: ctr.ports || ""
                }
              });
            }
          } else {
            setLocalDocker({ available: false, swarmActive: false });
          }
        } else {
          setLocalDocker({ available: false, swarmActive: false });
        }
      } catch {
        setLocalDocker({ available: false, swarmActive: false });
      }
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
                type: "domain",
                provider: "cloudflare",
                status: zone.paused ? "inactive" : "active"
              });
            }
          }
        } catch (e) {
          logError("CF zones:", e);
        }
        try {
          const workersRes = await fetch(`/api/cloudflare/workers?token=${encodeURIComponent(cloudflareToken)}`);
          if (workersRes.ok) {
            const workers = await workersRes.json();
            newStats.functions = (newStats.functions || 0) + workers.length;
            for (const worker of workers) {
              allResources.push({
                id: `cf-worker-${worker.id}`,
                name: worker.name,
                type: "function",
                provider: "cloudflare",
                status: "active"
              });
            }
          }
        } catch (e) {
          logError("CF workers:", e);
        }
        try {
          const pagesRes = await fetch(`/api/cloudflare/pages?token=${encodeURIComponent(cloudflareToken)}`);
          if (pagesRes.ok) {
            const pages = await pagesRes.json();
            newStats.pages = (newStats.pages || 0) + pages.length;
            for (const page of pages) {
              allResources.push({
                id: `cf-page-${page.id}`,
                name: page.name,
                type: "pages",
                provider: "cloudflare",
                status: "active",
                url: page.subdomain ? `${page.name}.${page.subdomain}.pages.dev` : void 0
              });
            }
          }
        } catch (e) {
          logError("CF pages:", e);
        }
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
                type: "deployment",
                provider: "vercel",
                status: "active",
                metadata: { framework: project.framework },
                url: project.prodSettings?.url
              });
              try {
                const deployRes = await fetch(`/api/vercel/deployments/${project.id}?token=${encodeURIComponent(vercelToken)}`);
                if (deployRes.ok) {
                  const deploys = await deployRes.json();
                  for (const deploy of deploys.slice(0, 2)) {
                    allResources.push({
                      id: `vercel-deploy-${deploy.uid}`,
                      name: `${project.name}`,
                      type: "deployment",
                      provider: "vercel",
                      status: deploy.state === "READY" ? "active" : deploy.state === "ERROR" ? "error" : "pending",
                      metadata: { commit: deploy.meta?.githubCommitMessage?.slice(0, 40) },
                      url: deploy.url
                    });
                  }
                }
              } catch (e) {
              }
            }
          }
        } catch (e) {
          logError("Vercel projects:", e);
        }
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
                type: "server",
                provider: "hetzner",
                status: server.status,
                region: server.datacenter?.location?.city,
                ip: server.public_net?.ipv4?.ip,
                actions: [
                  { label: "Start", action: "start", icon: FiServer },
                  { label: "Stop", action: "stop", icon: FiServer },
                  { label: "Reboot", action: "reboot", icon: FiRefreshCw }
                ]
              });
            }
          }
        } catch (e) {
          logError("Hetzner servers:", e);
        }
        try {
          const firewallsRes = await fetch(`/api/hetzner/firewalls?token=${encodeURIComponent(hetznerToken)}`);
          if (firewallsRes.ok) {
            const firewalls = await firewallsRes.json();
            newStats.firewalls = (newStats.firewalls || 0) + firewalls.length;
            for (const fw of firewalls) {
              allResources.push({
                id: `hetzner-fw-${fw.id}`,
                name: fw.name,
                type: "firewall",
                provider: "hetzner",
                status: "active"
              });
            }
          }
        } catch (e) {
          logError("Hetzner firewalls:", e);
        }
      }
      if (githubToken) {
        try {
          const reposRes = await fetch(`/api/github/repos?token=${encodeURIComponent(githubToken)}`);
          if (reposRes.ok) {
            const repos = await reposRes.json();
            newStats.repositories = (newStats.repositories || 0) + repos.length;
            for (const repo of repos.slice(0, 10)) {
              allResources.push({
                id: `gh-repo-${repo.id}`,
                name: repo.full_name,
                type: "repository",
                provider: "github",
                status: "active",
                url: repo.html_url,
                description: repo.description,
                metadata: { stars: repo.stargazers_count, private: repo.private }
              });
            }
            const actionRepoSet = repos.slice(0, 5);
            for (const repo of actionRepoSet) {
              const owner = repo?.owner?.login;
              const name = repo?.name;
              if (!owner || !name) continue;
              try {
                const runsRes = await fetch(
                  `/api/github/actions/runs?owner=${encodeURIComponent(owner)}&repo=${encodeURIComponent(name)}&per_page=3&token=${encodeURIComponent(githubToken)}`
                );
                if (!runsRes.ok) continue;
                const runsPayload = await runsRes.json().catch(() => ({}));
                const runs = Array.isArray(runsPayload?.workflow_runs) ? runsPayload.workflow_runs : [];
                newStats.workflowRuns = (newStats.workflowRuns || 0) + runs.length;
                for (const run of runs.slice(0, 2)) {
                  allResources.push({
                    id: `gh-run-${run.id}`,
                    name: `${repo.full_name} · ${run.name || run.display_title || "Workflow"}`,
                    type: "workflow",
                    provider: "github",
                    status: run.conclusion || run.status || "unknown",
                    url: run.html_url,
                    metadata: {
                      branch: run.head_branch,
                      event: run.event,
                      actor: run.actor?.login
                    }
                  });
                }
              } catch (e) {
                logError("GitHub actions:", e);
              }
            }
          }
        } catch (e) {
          logError("GitHub repos:", e);
        }
        try {
          const userRes = await fetch(`/api/github/user?token=${encodeURIComponent(githubToken)}`);
          if (userRes.ok) {
            const user = await userRes.json();
            if (user.login) {
              newStats.github = user.login;
            }
          }
        } catch (e) {
        }
      }
      if (googleToken) {
        try {
          const messagesRes = await fetch(`/api/google/messages?limit=10&token=${encodeURIComponent(googleToken)}`);
          if (messagesRes.ok) {
            const data = await messagesRes.json();
            const messages = data.messages || [];
            newStats.emails = (newStats.emails || 0) + messages.length;
            for (const msg of messages.slice(0, 5)) {
              allResources.push({
                id: `google-email-${msg.id}`,
                name: msg.subject?.slice(0, 40) || "Untitled",
                type: "email",
                provider: "google",
                status: "active",
                metadata: { from: msg.from?.name, date: msg.date }
              });
            }
          }
        } catch (e) {
          logError("Gmail:", e);
        }
        try {
          const eventsRes = await fetch(`/api/google/events?limit=10&token=${encodeURIComponent(googleToken)}`);
          if (eventsRes.ok) {
            const data = await eventsRes.json();
            const events = data.items || [];
            newStats.events = (newStats.events || 0) + events.length;
            for (const event of events.slice(0, 5)) {
              allResources.push({
                id: `google-event-${event.id}`,
                name: event.summary?.slice(0, 40) || "Untitled",
                type: "calendar",
                provider: "google",
                status: event.status === "confirmed" ? "active" : "pending",
                metadata: { start: event.start?.dateTime || event.start?.date }
              });
            }
          }
        } catch (e) {
          logError("Calendar:", e);
        }
      }
      setStats(newStats);
      setResources(allResources);
    } catch (e) {
      setError("Failed to load cloud resources");
    }
    setLoading(false);
  };
  const handleAction = async (resource, action) => {
    if (!resource || !action) return;
    setActionLoading(`${resource.id}-${action}`);
    setNotice("");
    try {
      if (resource.provider === "hetzner" && resource.type === "server") {
        const token = getToken("hetzner");
        if (!token) throw new Error("Hetzner token missing");
        const serverId = resource.id.replace("hetzner-", "");
        const res = await fetch(`/api/hetzner/servers?token=${encodeURIComponent(token)}`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ action, serverId })
        });
        if (!res.ok) {
          throw new Error(`Hetzner action failed (${res.status})`);
        }
        setNotice(`Hetzner action '${action}' sent for ${resource.name}.`);
        setTimeout(fetchAllResources, 1500);
        return;
      }
      if (resource.provider === "docker" && resource.type === "container") {
        const selector = String(resource?.metadata?.containerName || resource?.metadata?.containerId || resource.name || "").trim();
        if (!selector) throw new Error("Container selector missing");
        const response = await fetch(localBridgeHttpUrl("/v1/local/docker/container/state"), {
          method: "POST",
          headers: { "content-type": "application/json; charset=utf-8" },
          body: JSON.stringify({ container: selector, action })
        });
        const payload = await response.json().catch(() => ({}));
        if (!response.ok || payload?.ok === false) {
          throw new Error(String(payload?.error || `Docker container action failed (${response.status})`));
        }
        setNotice(`Container ${selector}: ${action} complete${payload?.state ? ` (${payload.state})` : ""}.`);
        setTimeout(fetchAllResources, 500);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Action failed");
    } finally {
      setActionLoading(null);
    }
  };
  const openContainerContextMenu = (resource, event) => {
    if (!resource || resource.provider !== "docker" || resource.type !== "container") return;
    event.preventDefault();
    event.stopPropagation();
    setContextMenuState({
      open: true,
      x: event.clientX,
      y: event.clientY,
      resource
    });
  };
  const closeContextMenu = () => {
    setContextMenuState((prev) => ({
      ...prev,
      open: false,
      resource: null
    }));
  };
  const containerContextActions = createMemo(() => {
    const resource = contextMenuState().resource;
    if (!resource || resource.provider !== "docker" || resource.type !== "container") return [];
    return [
      { label: "Start Container", icon: FiActivity, run: () => handleAction(resource, "start") },
      { label: "Stop Container", icon: FiShield, run: () => handleAction(resource, "stop") },
      { label: "Restart Container", icon: FiRefreshCw, run: () => handleAction(resource, "restart") },
      { label: "__sep__" },
      { label: "Refresh Resources", icon: FiRefreshCw, run: fetchAllResources }
    ];
  });
  const openProviderPanel = (provider) => {
    const panelMap = {
      cloudflare: "cloudflare",
      vercel: "vercel",
      hetzner: "hetzner",
      github: "github",
      google: "email",
      docker: "terminal"
    };
    const panel = panelMap[provider];
    if (panel) openWindow(panel);
  };
  onMount(() => {
    fetchAllResources();
    const dismiss = () => closeContextMenu();
    window.addEventListener("pointerdown", dismiss);
    window.addEventListener("scroll", dismiss, true);
    onCleanup(() => {
      window.removeEventListener("pointerdown", dismiss);
      window.removeEventListener("scroll", dismiss, true);
    });
  });
  const filteredResources = createMemo(() => {
    let result = resources();
    if (searchQuery()) {
      const query = searchQuery().toLowerCase();
      result = result.filter(
        (r) => r.name.toLowerCase().includes(query) || r.provider.toLowerCase().includes(query) || r.type.toLowerCase().includes(query) || r.description?.toLowerCase().includes(query)
      );
    }
    if (selectedProvider()) {
      result = result.filter((r) => r.provider === selectedProvider());
    }
    if (selectedType()) {
      result = result.filter((r) => r.type === selectedType());
    }
    return result;
  });
  const resourcesByProvider = createMemo(() => {
    const grouped = {};
    for (const r of filteredResources()) {
      if (!grouped[r.provider]) grouped[r.provider] = [];
      grouped[r.provider].push(r);
    }
    return grouped;
  });
  const connectedProviders = createMemo(() => {
    const connected = [];
    if (getToken("cloudflare")) connected.push("cloudflare");
    if (getToken("vercel")) connected.push("vercel");
    if (getToken("hetzner")) connected.push("hetzner");
    if (integrationStore.getToken("github")) connected.push("github");
    if (integrationStore.getToken("google")) connected.push("google");
    if (localDocker().available) connected.push("docker");
    return connected;
  });
  const totalResources = createMemo(() => resources().length);
  const getStatusColor = (status) => {
    switch (status) {
      case "running":
      case "active":
      case "open":
        return "bg-green-900/50 text-green-400";
      case "stopped":
      case "inactive":
      case "closed":
        return "bg-neutral-700 text-neutral-400";
      case "error":
      case "degraded":
        return "bg-red-900/50 text-red-400";
      case "pending":
        return "bg-yellow-900/50 text-yellow-400";
      default:
        return "bg-neutral-700 text-neutral-400";
    }
  };
  return <div class={`h-full flex flex-col text-neutral-200 ${compact() ? "" : "bg-[#1a1a1a] p-4"}`}>
      <div class="border-b border-neutral-800 px-3 py-2">
        <div class="mb-3 flex items-center justify-between">
          <div class="flex items-center gap-2">
            <FiCloud class="text-[hsl(var(--primary))]" size={16} />
            <h2 class="text-xs font-medium uppercase tracking-wide text-neutral-300">Cloud</h2>
            <span class="text-[10px] text-neutral-500">({totalResources()})</span>
          </div>
          <button
    type="button"
    onClick={fetchAllResources}
    disabled={loading()}
    class={panelButtonClass}
    title="Refresh"
  >
            <FiRefreshCw size={12} class={loading() ? "animate-spin" : ""} />
          </button>
        </div>

        <Show when={Object.keys(stats()).length > 0}>
          <div class="mb-3 flex gap-1.5 overflow-x-auto pb-1.5">
            <For each={connectedProviders()}>
              {(provider) => <button
    type="button"
    onClick={() => openProviderPanel(provider)}
    class={panelButtonClass}
  >
                  <span class={`text-[10px] font-medium ${providers[provider]?.color || "text-white"}`}>
                    {providers[provider]?.name || provider}
                  </span>
                  <Show when={stats()[provider] && typeof stats()[provider] === "number"}>
                    <span class="text-[10px] text-neutral-500">{stats()[provider]}</span>
                  </Show>
                  <Show when={provider === "github" && typeof stats().github === "string"}>
                    <span class="text-[10px] text-neutral-500">@{stats().github}</span>
                  </Show>
                  <FiArrowRight size={12} class="text-neutral-500" />
                </button>}
            </For>
            <button
    type="button"
    onClick={() => openWorkflowIntegrations("github")}
    class={panelButtonClass}
  >
              <FiSettings size={12} class="text-neutral-400" />
              <span>Manage</span>
            </button>
          </div>
        </Show>

        <div class="space-y-1.5">
          <div class="relative">
            <FiSearch class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500" size={16} />
            <input
    type="text"
    value={searchQuery()}
    onInput={(e) => setSearchQuery(e.currentTarget.value)}
    placeholder="Search resources..."
    class="w-full rounded-md border border-neutral-700 bg-neutral-900 py-2 pl-9 pr-3 text-xs text-neutral-100 placeholder:text-neutral-500"
  />
          </div>
          <div class="grid grid-cols-1 gap-2">
            <select
      value={selectedProvider() || ""}
      onChange={(e) => setSelectedProvider(e.currentTarget.value || null)}
      class="w-full rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-xs text-neutral-100"
    >
              <option value="">All Providers</option>
              <For each={connectedProviders()}>
                {(provider) => <option value={provider}>{providers[provider]?.name || provider}</option>}
              </For>
            </select>
            <select
      value={selectedType() || ""}
      onChange={(e) => setSelectedType(e.currentTarget.value || null)}
      class="w-full rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-xs text-neutral-100"
    >
              <option value="">All Types</option>
              <For each={Object.entries(resourceTypeNames)}>
                {([id, name]) => <option value={id}>{name}</option>}
              </For>
            </select>
          </div>
        </div>
      </div>

      <Show when={error()}>
        <div class="p-4 bg-red-900/50 border-b border-red-800">
          <p class="text-red-200 text-sm">{error()}</p>
        </div>
      </Show>
      <Show when={notice()}>
        <div class="p-3 border-b border-emerald-800 bg-emerald-900/25">
          <p class="text-emerald-200 text-xs">{notice()}</p>
        </div>
      </Show>

      <div class="flex-1 overflow-auto p-3">
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
    onClick={() => openWorkflowIntegrations("github")}
    class={panelButtonClass}
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
          <div class="space-y-3">
            <For each={Object.entries(resourcesByProvider())}>
              {([provider, providerResources]) => <div>
                  <div class="flex items-center justify-between mb-3">
                    <div class="flex items-center gap-2">
                      <span class={`font-medium ${providers[provider]?.color || "text-neutral-400"}`}>
                        {providers[provider]?.name || provider}
                      </span>
                      <span class="text-xs text-neutral-500">({providerResources.length})</span>
                    </div>
                    <button
    type="button"
    onClick={() => openProviderPanel(provider)}
    class="inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-300 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
  >
                      Open <FiArrowRight size={12} />
                    </button>
                  </div>
                  <div class="space-y-1.5">
                    <For each={providerResources}>
                      {(resource) => <div class="flex items-center justify-between rounded-md border border-neutral-800 bg-neutral-900/55 px-2.5 py-2 hover:bg-neutral-800/70 transition-colors">
                          <div class="flex items-center gap-3 min-w-0">
                            <div class="rounded-md bg-neutral-800 p-1.5 flex-shrink-0">
                              {(() => {
    const Icon = resourceTypeIcons[resource.type] || FiServer;
    return <Icon size={14} class="text-neutral-400" />;
  })()}
                            </div>
                            <div class="min-w-0">
                              <p class="truncate text-xs font-medium text-neutral-200">{resource.name}</p>
                              <p class="text-xs text-neutral-500 truncate">
                                {resourceTypeNames[resource.type] || resource.type}
                                <Show when={resource.region}> • {resource.region}</Show>
                                <Show when={resource.ip}> • {resource.ip}</Show>
                                <Show when={resource.metadata?.framework}> • {resource.metadata?.framework}</Show>
                                <Show when={resource.metadata?.replicas}> • {resource.metadata?.replicas}</Show>
                                <Show when={resource.metadata?.mode}> • {resource.metadata?.mode}</Show>
                                <Show when={resource.metadata?.stars}> ★ {resource.metadata?.stars}</Show>
                                <Show when={resource.metadata?.private}> • private</Show>
                                <Show when={resource.metadata?.image}> • {resource.metadata?.image}</Show>
                                <Show when={resource.metadata?.ports}> • {resource.metadata?.ports}</Show>
                              </p>
                            </div>
                          </div>
                          <div class="flex items-center gap-2 flex-shrink-0">
                            <button
    type="button"
    class={`rounded px-2 py-0.5 text-[10px] ${getStatusColor(resource.status)} ${resource.provider === "docker" && resource.type === "container" ? "cursor-context-menu" : "cursor-default"}`}
    title={resource.provider === "docker" && resource.type === "container" ? "Right-click for container actions" : "Status"}
    onClick={(event) => event.preventDefault()}
    onContextMenu={(event) => openContainerContextMenu(resource, event)}
    data-testid={`cloud-resource-status-${resource.id}`}
  >
                              {resource.status}
                            </button>
                            <Show when={resource.url}>
                              <a
    href={resource.url?.startsWith("http") ? resource.url : `https://${resource.url}`}
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
                                  {(act) => <button
    type="button"
    onClick={() => handleAction(resource, act.action)}
    disabled={actionLoading() === `${resource.id}-${act.action}`}
    class={`p-1.5 rounded transition-colors ${act.danger ? "text-red-400 hover:bg-red-900/30" : "text-neutral-400 hover:text-white hover:bg-neutral-700"} disabled:opacity-50`}
    title={act.label}
  >
                                      <act.icon size={14} class={actionLoading() === `${resource.id}-${act.action}` ? "animate-spin" : ""} />
                                    </button>}
                                </For>
                              </div>
                            </Show>
                          </div>
                        </div>}
                    </For>
                  </div>
                </div>}
            </For>
          </div>
        </Show>
      </div>
      <IntentContextMenu
        open={contextMenuState().open}
        position={{ x: contextMenuState().x, y: contextMenuState().y }}
        actions={containerContextActions()}
        onClose={closeContextMenu}
      />
    </div>;
}
export {
  CloudPanel as default
};
