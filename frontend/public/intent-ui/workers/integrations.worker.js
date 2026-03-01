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
    if (this.forceUserOwned) return "user_owned";
    return this.defaultConnectorMode || (this.supportsPlatformConnector ? "platform" : "user_owned");
  }

  resolveConnectorMode(mode) {
    if (this.forceUserOwned) return "user_owned";
    const requested = String(mode || "").trim();
    if (requested === "platform" && this.supportsPlatformConnector) return "platform";
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

  connectConnection({ currentConnection = {}, payload = {}, profileReady = false, token = "", nowIso = "" } = {}) {
    const timestamp = nowIso || new Date().toISOString();
    const connectorMode = this.resolveConnectorMode(
      payload?.connectorMode || currentConnection?.connectorMode || this.getDefaultConnectorMode()
    );
    const hasToken = !this.requiresToken ? true : Boolean(payload?.hasToken) || this.hasUsableToken(token);
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

  setConnectorModeConnection({ currentConnection = {}, mode = "", profileReady = false, token = "", nowIso = "" } = {}) {
    const timestamp = nowIso || new Date().toISOString();
    const connectorMode = this.resolveConnectorMode(mode);
    const hasToken = !this.requiresToken ? Boolean(currentConnection?.connected) : this.hasUsableToken(token);
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

  async verifyConnection({ details = {}, connectorMode = "", profileReady = false, deviceReady = false, token = "", fetchImpl = fetch } = {}) {
    const mode = this.resolveConnectorMode(connectorMode || this.getDefaultConnectorMode());
    if (this.isMatrixBridgeIntegration()) {
      return this.verifyMatrixBridgeRuntime({ details, token, fetchImpl });
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
        const response = await fetchImpl("/api/tailscale/devices", {
          method: "POST",
          headers: { "content-type": "application/json; charset=utf-8" },
          body: JSON.stringify({ apiKey, tailnet })
        });
        const body = await response.json().catch(() => ({}));
        if (!response.ok || !body?.ok) {
          return { ok: false, message: String(body?.error || `tailscale devices request failed (${response.status})`) };
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
    if (this.id === "flipper" || this.id === "daly_bms") {
      return { ok: false, message: `${this.name} verification must run in browser main thread.` };
    }
    const resolvedToken = String(details?.token || "").trim() || String(token || "").trim();
    if (this.requiresToken && resolvedToken.length < 8) {
      return { ok: false, message: `${this.name} token missing or invalid.` };
    }
    return { ok: true, message: `${this.name} credentials accepted.`, capabilities: this.defaultCapabilities.slice() };
  }
}

const integrationDefinitions = [
  { id: "github", name: "GitHub", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", tokenKey: "github_token", defaultCapabilities: ["repos.read", "repos.write", "prs.read", "prs.write"], tags: ["code", "storage", "workflows"] },
  { id: "cloudflare", name: "Cloudflare", authMethod: "token", supportsPlatformConnector: true, tokenKey: "cloudflare_token", defaultCapabilities: ["zones.read", "workers.read", "workers.write"], tags: ["network", "deploy", "workflows", "code"] },
  { id: "vercel", name: "Vercel", authMethod: "token", supportsPlatformConnector: true, tokenKey: "vercel_token", defaultCapabilities: ["projects.read", "deployments.read", "deployments.write"], tags: ["deploy", "workflows", "code"] },
  { id: "google", name: "Google", authMethod: "oauth", supportsPlatformConnector: true, tokenKey: "google_token", defaultCapabilities: ["drive.read", "gmail.read", "calendar.read", "contacts.read", "messages.read"], tags: ["messages", "storage", "workflows", "productivity"] },
  { id: "google_photos", name: "Google Photos", authMethod: "oauth", supportsPlatformConnector: true, tokenKey: "google_token", defaultCapabilities: ["photos.read"], tags: ["media", "storage"] },
  { id: "email", name: "Email", authMethod: "oauth", supportsPlatformConnector: true, tokenKey: "google_token", defaultCapabilities: ["messages.read", "messages.send"], tags: ["messages", "workflows"] },
  { id: "whatsapp", name: "WhatsApp", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "whatsapp_token", defaultCapabilities: ["messages.read", "messages.send"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["whatsapp"] },
  { id: "messenger", name: "Messenger", authMethod: "oauth", supportsPlatformConnector: true, tokenKey: "messenger_token", defaultCapabilities: ["messages.read", "messages.send"], tags: ["messages", "workflows"] },
  { id: "telegram", name: "Telegram", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "telegram_token", defaultCapabilities: ["messages.read", "messages.send"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["telegram"] },
  { id: "google_messages", name: "Google Messages", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "google_messages_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge", "sms", "rcs"], aliases: ["gmessages", "googlemessages", "rcs", "sms"] },
  { id: "meta", name: "Meta", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "meta_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge", "facebook", "instagram"], aliases: ["meta", "instagram", "facebook"] },
  { id: "signal", name: "Signal", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "signal_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["signal"] },
  { id: "discord", name: "Discord", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "discord_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["discord"] },
  { id: "slack", name: "Slack", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "slack_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["slack"] },
  { id: "gvoice", name: "Google Voice", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "gvoice_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["gvoice", "googlevoice"] },
  { id: "googlechat", name: "Google Chat", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "googlechat_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["googlechat", "gchat"] },
  { id: "twitter", name: "X / Twitter", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "twitter_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["twitter"] },
  { id: "bluesky", name: "Bluesky", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "bluesky_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["bluesky", "bsky"] },
  { id: "imessage", name: "iMessage", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "imessage_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["imessage"] },
  { id: "imessagego", name: "iMessage (Go)", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "imessagego_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["imessagego"] },
  { id: "linkedin", name: "LinkedIn", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "linkedin_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge"], aliases: ["linkedin"] },
  { id: "heisenbridge", name: "IRC (Heisenbridge)", authMethod: "token", supportsPlatformConnector: false, defaultConnectorMode: "user_owned", forceUserOwned: true, tokenKey: "heisenbridge_token", defaultCapabilities: ["messages.read", "messages.send", "messages.sync"], tags: ["messages", "workflows", "matrix-bridge", "irc"], aliases: ["heisenbridge", "irc"] },
  { id: "opencode_cli", name: "OpenCode CLI", authMethod: "local_cli", supportsPlatformConnector: false, requiresToken: false, tokenKey: "", defaultCapabilities: ["assistant.local_cli.execute"], tags: ["ai", "code", "workflows"] },
  { id: "tailscale", name: "Tailscale", authMethod: "token", supportsPlatformConnector: true, defaultConnectorMode: "user_owned", tokenKey: "tailscale_api_key", defaultCapabilities: ["network.overlay.join", "network.overlay.funnel", "network.overlay.ssh"], tags: ["network", "devices", "workflows"] },
  { id: "hetzner", name: "Hetzner", authMethod: "token", supportsPlatformConnector: true, tokenKey: "hetzner_token", defaultCapabilities: ["servers.read", "servers.write", "firewalls.read"], tags: ["compute", "network", "storage", "workflows"] },
  { id: "web3", name: "Web3", authMethod: "wallet", supportsPlatformConnector: false, tokenKey: "web3_wallet", defaultCapabilities: ["wallet.connect", "profile.encrypt", "backup.local"], tags: ["identity", "security", "workflows"] },
  { id: "flipper", name: "Flipper", authMethod: "web_bluetooth", supportsPlatformConnector: false, requiresToken: false, tokenKey: "flipper_device_id", defaultCapabilities: ["bluetooth.connect", "bluetooth.gatt", "hardware.flipper.interact"], tags: ["devices", "security", "workflows"] },
  { id: "daly_bms", name: "Daly BMS", authMethod: "web_bluetooth", supportsPlatformConnector: false, requiresToken: false, tokenKey: "daly_bms_device_id", defaultCapabilities: ["bluetooth.connect", "bluetooth.gatt", "hardware.bms.read"], tags: ["devices", "energy", "workflows"] }
];

const catalog = Object.fromEntries(integrationDefinitions.map((definition) => [definition.id, new IntegrationLifecycle(definition)]));

function resolveIntegration(integrationId) {
  const id = String(integrationId || "").trim();
  return catalog[id] || null;
}

function reply(ok, id, result, error = "") {
  postMessage({
    ok,
    id,
    result: ok ? result : undefined,
    error: ok ? "" : String(error || "unknown worker error")
  });
}

self.onmessage = async (event) => {
  const data = event?.data || {};
  const requestId = String(data.id || "").trim();
  const type = String(data.type || "").trim();
  const payload = data.payload || {};
  const integration = resolveIntegration(payload.integrationId);

  if (!requestId) return;
  if (!integration) {
    reply(false, requestId, null, `unknown integration: ${payload.integrationId || ""}`);
    return;
  }

  try {
    if (type === "hydrate_connection") {
      const result = integration.hydrateConnection(payload);
      reply(true, requestId, result);
      return;
    }
    if (type === "connect_connection") {
      const result = integration.connectConnection(payload);
      reply(true, requestId, result);
      return;
    }
    if (type === "disconnect_connection") {
      const result = integration.disconnectConnection();
      reply(true, requestId, result);
      return;
    }
    if (type === "set_mode_connection") {
      const result = integration.setConnectorModeConnection(payload);
      reply(true, requestId, result);
      return;
    }
    if (type === "verify_integration") {
      const result = await integration.verifyConnection({ ...payload, fetchImpl: fetch });
      reply(true, requestId, result);
      return;
    }
    if (type === "list_connection_view") {
      const result = integration.listConnectionView(payload);
      reply(true, requestId, result);
      return;
    }
    reply(false, requestId, null, `unknown request type: ${type}`);
  } catch (error) {
    reply(false, requestId, null, error instanceof Error ? error.message : "integration worker execution failed");
  }
};
