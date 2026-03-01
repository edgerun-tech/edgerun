class IntegrationLifecycle {
  constructor(definition = {}) {
    this.id = String(definition.id || "").trim();
    this.name = String(definition.name || this.id || "Integration").trim();
    this.authMethod = String(definition.authMethod || "token").trim();
    this.supportsPlatformConnector = Boolean(definition.supportsPlatformConnector);
    this.defaultConnectorMode = String(definition.defaultConnectorMode || "").trim();
    this.tokenKey = String(definition.tokenKey || "").trim();
    this.requiresToken = definition.requiresToken === false ? false : true;
    this.defaultCapabilities = Array.isArray(definition.defaultCapabilities) ? definition.defaultCapabilities.slice() : [];
    this.tags = Array.isArray(definition.tags) ? definition.tags.slice() : [];
    this.aliases = Array.isArray(definition.aliases) ? definition.aliases.slice() : [];
    this.forceUserOwned = Boolean(definition.forceUserOwned) || this.tags.includes("matrix-bridge");
  }

  getDefaultConnectorMode() {
    return "user_owned";
  }

  resolveConnectorMode(mode) {
    return "user_owned";
  }

  isMatrixBridgeIntegration() {
    return this.tags.includes("matrix-bridge");
  }

  async verifyMatrixBridgeRuntime({ details = {}, token = "", fetchImpl = fetch } = {}) {
    const bridgeToken = String(details?.token || "").trim() || String(token || "").trim();
    if (bridgeToken.length < 8) {
      return { ok: false, message: `${this.name} bridge token missing or invalid.` };
    }

    const nodeId = String(details?.nodeId || details?.node_id || "").trim();
    const query = new URLSearchParams({ integration_id: this.id });
    if (nodeId) query.set("node_id", nodeId);

    const readRuntimeStatus = async () => {
      try {
        const response = await fetchImpl(`/v1/local/mcp/integration/status?${query.toString()}`, { cache: "no-store" });
        const payload = await response.json().catch(() => ({}));
        if (!response.ok || payload?.ok === false) {
          return {
            ok: false,
            running: false,
            message: String(payload?.error || `bridge status request failed (${response.status})`)
          };
        }
        const running = Boolean(payload?.data?.running);
        const status = String(payload?.data?.status || (running ? "running" : "stopped")).trim();
        return { ok: true, running, status };
      } catch (error) {
        return {
          ok: false,
          running: false,
          message: error instanceof Error ? error.message : "Failed to query bridge runtime status."
        };
      }
    };

    const status = await readRuntimeStatus();
    if (status.ok && status.running) {
      return {
        ok: true,
        message: `${this.name} Matrix bridge runtime is running (${status.status || "running"}).`,
        capabilities: this.defaultCapabilities.slice()
      };
    }

    return {
      ok: true,
      message: `${this.name} credentials accepted. Runtime starts on Link Integration.`,
      capabilities: this.defaultCapabilities.slice()
    };
  }

  hasUsableToken(token, fallbackConnected = false) {
    if (!this.requiresToken) return Boolean(fallbackConnected);
    return Boolean(String(token || "").trim());
  }

  hydrateConnection({ storedConnection = {}, profileReady = false, token = "", nowIso = "" } = {}) {
    const timestamp = nowIso || new Date().toISOString();
    const storedMode = String(storedConnection?.connectorMode || "").trim();
    const connectorMode = this.resolveConnectorMode(storedMode || this.getDefaultConnectorMode());
    const linked = Boolean(storedConnection?.linked);
    const hasUsableToken = this.hasUsableToken(token, Boolean(storedConnection?.connected));

    if (connectorMode === "platform") {
      const connected = Boolean(linked && profileReady);
      return {
        connected,
        linked,
        connectorMode: "platform",
        authMethod: this.authMethod,
        capabilities: connected ? this.defaultCapabilities.slice() : [],
        connectedAt: connected ? (storedConnection?.connectedAt || timestamp) : null,
        accountLabel: connected ? (storedConnection?.accountLabel || "Platform Connector") : "Platform Connector"
      };
    }

    return {
      connected: hasUsableToken,
      linked: hasUsableToken,
      connectorMode: "user_owned",
      authMethod: this.authMethod,
      capabilities: hasUsableToken ? this.defaultCapabilities.slice() : [],
      connectedAt: hasUsableToken ? (storedConnection?.connectedAt || timestamp) : null,
      accountLabel: hasUsableToken ? (storedConnection?.accountLabel || `${this.name} Account`) : ""
    };
  }

  connectConnection({
    currentConnection = {},
    payload = {},
    profileReady = false,
    token = "",
    nowIso = ""
  } = {}) {
    const timestamp = nowIso || new Date().toISOString();
    const connectorMode = this.resolveConnectorMode(
      payload?.connectorMode
      || currentConnection?.connectorMode
      || this.getDefaultConnectorMode()
    );
    const hasToken = !this.requiresToken
      ? true
      : Boolean(payload?.hasToken) || this.hasUsableToken(token);
    const connected = connectorMode === "platform" ? Boolean(profileReady) : hasToken;
    const capabilities = connected
      ? (Array.isArray(payload?.capabilities) && payload.capabilities.length > 0
        ? payload.capabilities.slice()
        : this.defaultCapabilities.slice())
      : [];
    const accountLabel = String(payload?.accountLabel || "").trim()
      || (connectorMode === "platform" ? "Platform Connector" : (connected ? `${this.name} Account` : ""));

    return {
      connected,
      linked: connectorMode === "platform" ? true : connected,
      connectorMode,
      authMethod: this.authMethod,
      capabilities,
      connectedAt: connected ? (currentConnection?.connectedAt || timestamp) : null,
      accountLabel
    };
  }

  disconnectConnection() {
    return {
      connected: false,
      linked: false,
      connectorMode: this.getDefaultConnectorMode(),
      capabilities: [],
      accountLabel: ""
    };
  }

  setConnectorModeConnection({
    currentConnection = {},
    mode = "",
    profileReady = false,
    token = "",
    nowIso = ""
  } = {}) {
    const timestamp = nowIso || new Date().toISOString();
    const connectorMode = this.resolveConnectorMode(mode);
    const hasToken = !this.requiresToken
      ? Boolean(currentConnection?.connected)
      : this.hasUsableToken(token);
    const connected = connectorMode === "platform"
      ? Boolean(currentConnection?.linked && profileReady)
      : hasToken;
    return {
      ...currentConnection,
      connected,
      linked: connectorMode === "platform" ? Boolean(currentConnection?.linked) : connected,
      connectorMode,
      authMethod: this.authMethod,
      capabilities: connected
        ? (Array.isArray(currentConnection?.capabilities) && currentConnection.capabilities.length > 0
          ? currentConnection.capabilities.slice()
          : this.defaultCapabilities.slice())
        : [],
      connectedAt: connected ? (currentConnection?.connectedAt || timestamp) : null,
      accountLabel: connectorMode === "platform"
        ? "Platform Connector"
        : (connected ? (currentConnection?.accountLabel || `${this.name} Account`) : "")
    };
  }

  listConnectionView({ connection = {}, profileReady = false, deviceReady = true } = {}) {
    const connected = Boolean(connection?.connected);
    const requiresProfileSession = this.id !== "opencode_cli";
    const available = connected
      && (requiresProfileSession ? profileReady : true)
      && (this.id === "opencode_cli" ? deviceReady : true);
    const availabilityReason = this.id === "tailscale" && !connected
      ? "Provide Tailscale API key and link integration"
      : !connected
        ? "Not connected"
        : this.id === "opencode_cli" && !deviceReady
          ? "Connected device required"
          : requiresProfileSession && !profileReady
            ? "Profile session required"
            : "Ready";
    const connectorMode = this.resolveConnectorMode(connection?.connectorMode || this.getDefaultConnectorMode());
    return {
      connected,
      available,
      availabilityReason,
      connectorMode,
      tags: this.tags.slice(),
      supportsPlatformConnector: this.supportsPlatformConnector,
      linked: Boolean(connection?.linked),
      connectedAt: connection?.connectedAt || null,
      accountLabel: connection?.accountLabel || "",
      capabilities: available
        ? (Array.isArray(connection?.capabilities) ? connection.capabilities.slice() : this.defaultCapabilities.slice())
        : [],
      aliases: this.aliases.slice()
    };
  }

  async verifyConnection({
    details = {},
    connectorMode = "",
    profileReady = false,
    deviceReady = false,
    token = "",
    fetchImpl = fetch
  } = {}) {
    const mode = this.resolveConnectorMode(connectorMode || this.getDefaultConnectorMode());
    if (this.isMatrixBridgeIntegration()) {
      return this.verifyMatrixBridgeRuntime({ details, token, fetchImpl });
    }
    if (this.id === "beeper") {
      const accessToken = String(details?.token || "").trim() || String(token || "").trim();
      if (accessToken.length < 8) {
        return { ok: false, message: "Beeper access token missing or invalid." };
      }
      try {
        let body = null;
        let statusCode = 0;
        for (const path of ["/api/beeper/verify", "/v1/local/beeper/verify"]) {
          const response = await fetchImpl(path, {
            method: "POST",
            headers: { "content-type": "application/json; charset=utf-8" },
            body: JSON.stringify({ token: accessToken })
          });
          statusCode = response.status;
          body = await response.json().catch(() => ({}));
          if (response.ok && body?.ok) break;
          if (response.status !== 404) break;
        }
        if (!body?.ok) {
          return { ok: false, message: String(body?.error || `beeper token verify failed (${statusCode})`) };
        }
        const accountCount = Number(body?.account_count || 0);
        return {
          ok: true,
          message: `Verified Beeper Desktop API access (${accountCount} accounts visible).`,
          capabilities: this.defaultCapabilities.slice(),
          accountLabel: "Beeper Desktop"
        };
      } catch (error) {
        return { ok: false, message: error instanceof Error ? error.message : "Failed to verify Beeper Desktop API access." };
      }
    }
    if (this.id === "github") {
      const pat = String(details?.token || "").trim() || String(token || "").trim();
      if (pat.length < 8) {
        return { ok: false, message: "GitHub Personal Access Token missing or invalid." };
      }
      try {
        const response = await fetchImpl("https://api.github.com/user", {
          cache: "no-store",
          headers: {
            accept: "application/vnd.github+json",
            authorization: `Bearer ${pat}`,
            "x-github-api-version": "2022-11-28"
          }
        });
        const body = await response.json().catch(() => ({}));
        if (!response.ok) {
          return { ok: false, message: String(body?.error || `GitHub user request failed (${response.status})`) };
        }
        const login = String(body?.login || "").trim();
        return {
          ok: true,
          message: login ? `Verified GitHub API access as @${login}.` : "Verified GitHub API access.",
          capabilities: this.defaultCapabilities.slice()
        };
      } catch (error) {
        return { ok: false, message: error instanceof Error ? error.message : "Failed to verify GitHub API access." };
      }
    }
    if (this.id === "cloudflare" && mode === "user_owned") {
      const apiToken = String(details?.token || "").trim() || String(token || "").trim();
      if (apiToken.length < 20) {
        return { ok: false, message: "Cloudflare account API token missing or invalid." };
      }
      try {
        let body = null;
        let statusCode = 0;
        for (const path of ["/api/cloudflare/verify", "/v1/local/cloudflare/verify"]) {
          const response = await fetchImpl(path, {
            method: "POST",
            headers: { "content-type": "application/json; charset=utf-8" },
            body: JSON.stringify({ token: apiToken })
          });
          statusCode = response.status;
          body = await response.json().catch(() => ({}));
          if (response.ok && body?.ok) break;
          if (response.status !== 404) break;
        }
        if (!body?.ok) {
          return { ok: false, message: String(body?.error || `cloudflare token verify failed (${statusCode})`) };
        }
        const status = String(body?.status || "").trim();
        const userEmail = String(body?.user_email || "").trim();
        const accountLabel = userEmail || "Cloudflare Account";
        return {
          ok: true,
          message: userEmail
            ? `Verified Cloudflare account token (${status || "active"}) as ${userEmail}.`
            : (status ? `Verified Cloudflare account token (${status}).` : "Verified Cloudflare account token."),
          capabilities: this.defaultCapabilities.slice(),
          accountLabel
        };
      } catch (error) {
        return { ok: false, message: error instanceof Error ? error.message : "Failed to verify Cloudflare account token." };
      }
    }
    if (mode === "platform") {
      if (!profileReady) return { ok: false, message: "Profile session required for platform connector." };
      return { ok: true, message: "Platform connector session is active.", capabilities: this.defaultCapabilities.slice() };
    }
    if (this.id === "opencode_cli") {
      if (!deviceReady) return { ok: false, message: "No connected node manager device is online." };
      return { ok: true, message: "Connected device is online for local CLI execution.", capabilities: this.defaultCapabilities.slice() };
    }
    if (this.id === "tailscale") {
      const apiKey = String(details?.apiKey || details?.token || "").trim() || String(token || "").trim();
      const tailnet = String(details?.tailnet || "").trim();
      if (!apiKey || !tailnet) return { ok: false, message: "Tailscale API key and tailnet are required." };
      try {
        let body = null;
        let statusCode = 0;
        for (const path of ["/api/tailscale/devices", "/v1/local/tailscale/devices"]) {
          const response = await fetchImpl(path, {
            method: "POST",
            headers: { "content-type": "application/json; charset=utf-8" },
            body: JSON.stringify({ apiKey, tailnet })
          });
          statusCode = response.status;
          body = await response.json().catch(() => ({}));
          if (response.ok && body?.ok) break;
          if (response.status !== 404) break;
        }
        if (!body?.ok) {
          return { ok: false, message: String(body?.error || `tailscale devices request failed (${statusCode})`) };
        }
        const devices = Array.isArray(body?.devices) ? body.devices : [];
        return {
          ok: true,
          message: `Verified Tailscale API access (${devices.length} devices visible).`,
          capabilities: this.defaultCapabilities.slice(),
          devices
        };
      } catch (error) {
        return { ok: false, message: error instanceof Error ? error.message : "Failed to verify Tailscale API access." };
      }
    }
    if (this.id === "web3") {
      const wallet = String(details?.wallet || "").trim();
      if (!wallet.startsWith("0x")) return { ok: false, message: "Connect EVM wallet first." };
      return { ok: true, message: "Wallet is connected and ready.", capabilities: this.defaultCapabilities.slice() };
    }
    if (this.id === "flipper") {
      return { ok: false, message: "Select a Flipper device from Step 2 before verification." };
    }
    if (this.id === "daly_bms") {
      return { ok: false, message: "Select a Daly BMS device from Step 2 before verification." };
    }
    const resolvedToken = String(details?.token || "").trim() || String(token || "").trim();
    if (this.requiresToken && resolvedToken.length < 8) {
      return { ok: false, message: `${this.name} token missing or invalid.` };
    }
    return { ok: true, message: `${this.name} credentials accepted.`, capabilities: this.defaultCapabilities.slice() };
  }
}

export {
  IntegrationLifecycle
};
