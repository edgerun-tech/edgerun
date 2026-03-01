function toArray(payload, key) {
  if (Array.isArray(payload)) return payload;
  if (Array.isArray(payload?.[key])) return payload[key];
  return [];
}

function tokenFrom(provider) {
  const value = String(typeof provider === "function" ? provider() : provider || "").trim();
  return value;
}

export function createLocalBridgeTransport({ fetchFn, localBridgeUrl }) {
  return {
    async getJson(path, options = {}) {
      const response = await fetchFn(localBridgeUrl(path), options);
      const payload = await response.json().catch(() => ({}));
      return { ok: response.ok, status: response.status, payload };
    },
    async postJson(path, body, options = {}) {
      const response = await fetchFn(localBridgeUrl(path), {
        method: "POST",
        headers: { "content-type": "application/json; charset=utf-8" },
        body: JSON.stringify(body || {}),
        ...options
      });
      const payload = await response.json().catch(() => ({}));
      return { ok: response.ok, status: response.status, payload };
    }
  };
}

export function createDockerClient({ transport }) {
  return {
    async getSummary() {
      const result = await transport.getJson("/v1/local/docker/summary", { cache: "no-store" });
      if (!result.ok || !result.payload?.ok) {
        return {
          ok: false,
          services: [],
          containers: [],
          swarmActive: false,
          error: String(result.payload?.error || "docker summary unavailable")
        };
      }
      return {
        ok: true,
        services: toArray(result.payload, "services"),
        containers: toArray(result.payload, "containers"),
        swarmActive: Boolean(result.payload?.swarm_active),
        swarmNodeId: String(result.payload?.swarm_node_id || "")
      };
    }
  };
}

export function createCloudflareClient({ transport, tokenProvider }) {
  const withToken = (path, overrideToken) => {
    const token = tokenFrom(overrideToken || tokenProvider);
    if (!token) return null;
    return `${path}${path.includes("?") ? "&" : "?"}token=${encodeURIComponent(token)}`;
  };
  return {
    async listZones(token) {
      const path = withToken("/v1/local/cloudflare/zones", token);
      if (!path) return [];
      const result = await transport.getJson(path);
      return result.ok ? toArray(result.payload, "zones") : [];
    },
    async listWorkers(token) {
      const path = withToken("/v1/local/cloudflare/workers", token);
      if (!path) return [];
      const result = await transport.getJson(path);
      return result.ok ? toArray(result.payload, "workers") : [];
    },
    async listPages(token) {
      const path = withToken("/v1/local/cloudflare/pages", token);
      if (!path) return [];
      const result = await transport.getJson(path);
      return result.ok ? toArray(result.payload, "pages") : [];
    }
  };
}

export function createGithubWorkflowClient({ transport, tokenProvider }) {
  const remoteRunsPath = (perPage = 24, overrideToken) => {
    const token = tokenFrom(overrideToken || tokenProvider);
    if (!token) return null;
    return `/v1/local/github/workflow/runs?token=${encodeURIComponent(token)}&per_page=${perPage}`;
  };
  return {
    async listRemoteRuns({ perPage = 24, token } = {}) {
      const path = remoteRunsPath(perPage, token);
      if (!path) return [];
      const result = await transport.getJson(path, { cache: "no-store" });
      return result.ok ? toArray(result.payload, "runs") : [];
    },
    async listLocalRuns() {
      const result = await transport.getJson("/v1/local/github/workflow/runner/runs", {
        cache: "no-store"
      });
      return result.ok ? toArray(result.payload, "runs") : [];
    },
    async runLocalWorkflow(workflowId) {
      const id = String(workflowId || "").trim();
      if (!id) return { ok: false, error: "workflow_id is required" };
      const result = await transport.postJson("/v1/local/github/workflow/runner/run", {
        workflow_id: id
      });
      if (!result.ok || result.payload?.ok === false) {
        return {
          ok: false,
          error: String(result.payload?.error || `local workflow runner failed (${result.status})`)
        };
      }
      return {
        ok: true,
        run: result.payload?.run || null
      };
    }
  };
}

export function createCloudPanelClients({ fetchFn, localBridgeUrl, getCloudflareToken, getGithubToken }) {
  const transport = createLocalBridgeTransport({ fetchFn, localBridgeUrl });
  return {
    docker: createDockerClient({ transport }),
    cloudflare: createCloudflareClient({ transport, tokenProvider: getCloudflareToken }),
    githubWorkflow: createGithubWorkflowClient({ transport, tokenProvider: getGithubToken })
  };
}
