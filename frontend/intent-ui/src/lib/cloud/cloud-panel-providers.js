export async function loadDockerProvider({ dockerClient }) {
  const result = {
    resources: [],
    stats: {},
    localDocker: { available: false, swarmActive: false }
  };
  try {
    const summary = await dockerClient.getSummary();
    if (!summary?.ok) return result;
    const services = Array.isArray(summary.services) ? summary.services : [];
    const containers = Array.isArray(summary.containers) ? summary.containers : [];
    result.localDocker = {
      available: true,
      swarmActive: Boolean(summary.swarmActive)
    };
    result.stats.docker = services.length + containers.length;
    result.stats.dockerServices = services.length;
    result.stats.dockerContainers = containers.length;
    for (const svc of services) {
      result.resources.push({
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
      result.resources.push({
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
  } catch {
    // ignore transport/runtime errors for optional provider
  }
  return result;
}

export async function loadCloudflareProvider({ cloudflareClient, token }) {
  const result = {
    resources: [],
    stats: {}
  };
  if (!token) return result;
  try {
    const zones = await cloudflareClient.listZones(token);
    result.stats.domains = zones.length;
    for (const zone of zones) {
      result.resources.push({
        id: `cf-zone-${zone.id}`,
        name: zone.name,
        type: "domain",
        provider: "cloudflare",
        status: zone.paused ? "inactive" : zone.status || "active"
      });
    }
  } catch {
    // ignore provider failure
  }
  try {
    const workers = await cloudflareClient.listWorkers(token);
    result.stats.functions = workers.length;
    for (const worker of workers) {
      result.resources.push({
        id: `cf-worker-${worker.id || worker.name}`,
        name: worker.name || worker.id,
        type: "function",
        provider: "cloudflare",
        status: "active"
      });
    }
  } catch {
    // ignore provider failure
  }
  try {
    const pages = await cloudflareClient.listPages(token);
    result.stats.pages = pages.length;
    for (const page of pages) {
      const subdomain = String(page?.subdomain || "").trim();
      result.resources.push({
        id: `cf-page-${page.id || page.name}`,
        name: page.name || page.id,
        type: "pages",
        provider: "cloudflare",
        status: "active",
        url: subdomain ? `https://${subdomain}` : void 0
      });
    }
  } catch {
    // ignore provider failure
  }
  return result;
}

export async function loadGithubWorkflowProvider({ githubWorkflowClient, token }) {
  const result = {
    resources: [],
    stats: {}
  };
  if (!token) return result;
  try {
    const runs = await githubWorkflowClient.listRemoteRuns({ perPage: 24, token });
    result.stats.workflowRuns = runs.length;
    for (const run of runs) {
      const repoName = String(run?.repo_full_name || run?.repository?.full_name || "github/workflow").trim();
      const workflowName = String(run?.name || run?.display_title || "Workflow").trim();
      result.resources.push({
        id: `gh-run-${run.id || `${repoName}-${workflowName}`}`,
        name: `${repoName} · ${workflowName}`,
        type: "workflow",
        provider: "github",
        status: run.conclusion || run.status || "unknown",
        url: run.html_url,
        metadata: {
          source: "github",
          branch: run.head_branch,
          event: run.event,
          actor: run.actor?.login
        }
      });
    }
  } catch {
    // ignore provider failure
  }
  return result;
}

export async function loadLocalWorkflowRunnerProvider({ githubWorkflowClient }) {
  const result = {
    resources: [],
    stats: {}
  };
  try {
    const runs = await githubWorkflowClient.listLocalRuns();
    result.stats.localWorkflowRuns = runs.length;
    for (const run of runs.slice(0, 12)) {
      result.resources.push({
        id: `local-run-${run.id || run.started_unix_ms}`,
        name: `local · ${run.workflow_id || "intent-ui-ci"}`,
        type: "workflow",
        provider: "github",
        status: run.status || "unknown",
        metadata: {
          source: "local",
          duration: typeof run.duration_ms === "number" ? `${Math.round(run.duration_ms / 1000)}s` : "",
          actor: "local-runner",
          event: "workflow_dispatch"
        },
        description: String(run.message || "").trim()
      });
    }
  } catch {
    // ignore provider failure
  }
  return result;
}
