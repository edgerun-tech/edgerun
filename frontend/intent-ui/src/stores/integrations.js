import { createSignal } from "solid-js";
import { UI_EVENT_TOPICS, UI_INTENT_TOPICS, uiIntentMeta } from "../lib/ui-intents";
import { publishEvent, subscribeEvent } from "./eventbus";
import { profileRuntime } from "./profile-runtime";
import { knownDevices } from "./devices";
import { getProfileSecret, removeProfileSecret, setProfileSecret } from "./profile-secrets";

const STORAGE_KEY = "intent-ui-integrations-v1";
let cachedVaultStatus = null;
let vaultStatusCheckedAt = 0;
const VAULT_STATUS_TTL_MS = 30 * 1000;
let subscriptionsInitialized = false;

const catalog = {
  github: {
    id: "github",
    name: "GitHub",
    authMethod: "token",
    supportsPlatformConnector: false,
    defaultConnectorMode: "user_owned",
    tokenKey: "github_token",
    defaultCapabilities: ["repos.read", "repos.write", "prs.read", "prs.write"]
  },
  cloudflare: {
    id: "cloudflare",
    name: "Cloudflare",
    authMethod: "token",
    supportsPlatformConnector: true,
    tokenKey: "cloudflare_token",
    defaultCapabilities: ["zones.read", "workers.read", "workers.write"]
  },
  vercel: {
    id: "vercel",
    name: "Vercel",
    authMethod: "token",
    supportsPlatformConnector: true,
    tokenKey: "vercel_token",
    defaultCapabilities: ["projects.read", "deployments.read", "deployments.write"]
  },
  google: {
    id: "google",
    name: "Google",
    authMethod: "oauth",
    supportsPlatformConnector: true,
    tokenKey: "google_token",
    defaultCapabilities: ["drive.read", "gmail.read", "calendar.read", "contacts.read", "messages.read"]
  },
  google_photos: {
    id: "google_photos",
    name: "Google Photos",
    authMethod: "oauth",
    supportsPlatformConnector: true,
    tokenKey: "google_token",
    defaultCapabilities: ["photos.read"]
  },
  email: {
    id: "email",
    name: "Email",
    authMethod: "oauth",
    supportsPlatformConnector: true,
    tokenKey: "google_token",
    defaultCapabilities: ["messages.read", "messages.send"]
  },
  whatsapp: {
    id: "whatsapp",
    name: "WhatsApp",
    authMethod: "oauth",
    supportsPlatformConnector: true,
    tokenKey: "whatsapp_token",
    defaultCapabilities: ["messages.read", "messages.send"]
  },
  messenger: {
    id: "messenger",
    name: "Messenger",
    authMethod: "oauth",
    supportsPlatformConnector: true,
    tokenKey: "messenger_token",
    defaultCapabilities: ["messages.read", "messages.send"]
  },
  telegram: {
    id: "telegram",
    name: "Telegram",
    authMethod: "oauth",
    supportsPlatformConnector: true,
    tokenKey: "telegram_token",
    defaultCapabilities: ["messages.read", "messages.send"]
  },
  qwen: {
    id: "qwen",
    name: "Qwen",
    authMethod: "oauth",
    supportsPlatformConnector: true,
    tokenKey: "qwen_token",
    defaultCapabilities: ["chat.completions"]
  },
  codex_cli: {
    id: "codex_cli",
    name: "Codex CLI",
    authMethod: "local_cli",
    supportsPlatformConnector: false,
    requiresToken: false,
    tokenKey: "",
    defaultCapabilities: ["assistant.local_cli.execute"]
  },
  tailscale: {
    id: "tailscale",
    name: "Tailscale",
    authMethod: "token",
    supportsPlatformConnector: true,
    defaultConnectorMode: "user_owned",
    tokenKey: "tailscale_api_key",
    defaultCapabilities: ["network.overlay.join", "network.overlay.funnel", "network.overlay.ssh"]
  },
  hetzner: {
    id: "hetzner",
    name: "Hetzner",
    authMethod: "token",
    supportsPlatformConnector: true,
    tokenKey: "hetzner_token",
    defaultCapabilities: ["servers.read", "servers.write", "firewalls.read"]
  },
  web3: {
    id: "web3",
    name: "Web3",
    authMethod: "wallet",
    supportsPlatformConnector: false,
    tokenKey: "web3_wallet",
    defaultCapabilities: ["wallet.connect", "profile.encrypt", "backup.local"]
  }
};

function safeParse(raw) {
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function readStoredState() {
  if (typeof window === "undefined") return {};
  const parsed = safeParse(localStorage.getItem(STORAGE_KEY) || "");
  return parsed && typeof parsed === "object" ? parsed : {};
}

function persistState(state) {
  if (typeof window === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

async function getVaultStatus() {
  if (typeof window === "undefined") return { installed: false, locked: true };
  const now = Date.now();
  if (cachedVaultStatus && now - vaultStatusCheckedAt < VAULT_STATUS_TTL_MS) {
    return cachedVaultStatus;
  }
  try {
    const response = await fetch("/api/credentials/status", { cache: "no-store" });
    const payload = await response.json().catch(() => ({}));
    cachedVaultStatus = {
      installed: Boolean(payload?.installed),
      locked: Boolean(payload?.locked)
    };
  } catch {
    cachedVaultStatus = { installed: false, locked: true };
  }
  vaultStatusCheckedAt = now;
  return cachedVaultStatus;
}

async function syncIntegrationTokenToVault(integration, details) {
  if (typeof window === "undefined") return;
  const token = String(details?.token || "").trim();
  if (!integration || !token) return;
  const status = await getVaultStatus();
  if (!status.installed || status.locked) return;
  const accountLabel = String(details?.accountLabel || `${integration.name} Session`).trim();
  const entryName = `integration/${integration.id}/token`;
  try {
    await fetch("/api/credentials/store", {
      method: "POST",
      headers: { "content-type": "application/json; charset=utf-8" },
      body: JSON.stringify({
        credentialType: "token",
        name: entryName,
        username: accountLabel,
        secret: token,
        note: `${integration.name} token managed by Integrations panel.`,
        tags: `integration,${integration.id}`,
        folder: "integrations",
        payload: {
          provider: integration.id,
          source: "integrations-panel",
          savedAt: new Date().toISOString()
        }
      })
    });
  } catch {
    // best-effort mirror only
  }
}

function getRuntimeToken(integration) {
  if (!integration?.tokenKey) return "";
  const runtime = profileRuntime();
  if (integration.id === "github") {
    if (runtime.mode === "profile" && runtime.profileLoaded) {
      return getProfileSecret(integration.tokenKey).trim();
    }
    if (typeof window !== "undefined") {
      localStorage.removeItem(integration.tokenKey);
      localStorage.removeItem("github_auth_mode");
    }
    return "";
  }
  if (runtime.mode === "profile" && runtime.profileLoaded) {
    return getProfileSecret(integration.tokenKey).trim();
  }
  if (typeof window === "undefined") return "";
  return String(localStorage.getItem(integration.tokenKey) || "").trim();
}

function hydrateState() {
  const stored = readStoredState();
  const next = { ...stored };
  const runtime = profileRuntime();
  const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
  for (const integration of Object.values(catalog)) {
    const storedMode = String(next[integration.id]?.connectorMode || "").trim();
    const defaultMode = integration.defaultConnectorMode
      || (integration.supportsPlatformConnector ? "platform" : "user_owned");
    const connectorMode = storedMode || defaultMode;
    const token = getRuntimeToken(integration);
    const hasUsableToken = integration.requiresToken === false
      ? Boolean(next[integration.id]?.connected)
      : Boolean(token);
    const wasLinked = Boolean(next[integration.id]?.linked);
    if (connectorMode === "platform" && integration.supportsPlatformConnector) {
      const connected = Boolean(wasLinked && profileReady);
      next[integration.id] = {
        connected,
        linked: wasLinked,
        connectorMode: "platform",
        authMethod: integration.authMethod,
        capabilities: connected ? integration.defaultCapabilities : [],
        connectedAt: connected ? (next[integration.id]?.connectedAt || new Date().toISOString()) : null,
        accountLabel: connected ? (next[integration.id]?.accountLabel || "Platform Connector") : "Platform Connector"
      };
      continue;
    }
    next[integration.id] = {
      connected: hasUsableToken,
      linked: hasUsableToken,
      connectorMode: "user_owned",
      authMethod: integration.authMethod,
      capabilities: hasUsableToken ? integration.defaultCapabilities : [],
      connectedAt: hasUsableToken ? (next[integration.id]?.connectedAt || new Date().toISOString()) : null,
      accountLabel: hasUsableToken ? (next[integration.id]?.accountLabel || `${integration.name} Account`) : ""
    };
  }
  return next;
}

const [connections, setConnections] = createSignal(hydrateState());
const [integrationVerification, setIntegrationVerification] = createSignal({});

function emitIntegrationStateChanged(id, reason) {
  publishEvent(
    UI_EVENT_TOPICS.integration.stateChanged,
    { integrationId: id, reason },
    uiIntentMeta("integrations.reducer")
  );
}

function applyCheckAll() {
  const next = hydrateState();
  setConnections(next);
  persistState(next);
  publishEvent(UI_EVENT_TOPICS.integration.stateChanged, { integrationId: "*", reason: "check_all" }, uiIntentMeta("integrations.reducer"));
}

function applyConnectIntent(payload = {}) {
  const id = String(payload?.id || "").trim();
  const integration = catalog[id];
  if (!integration) return;
  const connectorMode = String(
    payload.connectorMode
    || connections()[id]?.connectorMode
    || integration.defaultConnectorMode
    || "user_owned"
  );
  const runtime = profileRuntime();
  const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
  const hasToken = integration.requiresToken === false
    ? true
    : Boolean(payload.hasToken) || Boolean(getRuntimeToken(integration));
  const connected = connectorMode === "platform"
    ? profileReady
    : hasToken;
  const next = {
    ...connections(),
    [id]: {
      connected,
      linked: connectorMode === "platform" ? true : connected,
      connectorMode,
      authMethod: integration.authMethod,
      capabilities: connected ? (Array.isArray(payload.capabilities) && payload.capabilities.length > 0
        ? payload.capabilities
        : integration.defaultCapabilities) : [],
      connectedAt: connected ? (connections()[id]?.connectedAt || new Date().toISOString()) : null,
      accountLabel: String(payload.accountLabel || "").trim()
        || (connectorMode === "platform" ? "Platform Connector" : (connected ? `${integration.name} Account` : ""))
    }
  };
  setConnections(next);
  persistState(next);
  emitIntegrationStateChanged(id, "connect");
}

function applyDisconnectIntent(payload = {}) {
  const id = String(payload?.id || "").trim();
  const integration = catalog[id];
  if (!integration) return;
  const supportsPlatformConnector = Boolean(integration.supportsPlatformConnector);
  const defaultMode = integration.defaultConnectorMode || (supportsPlatformConnector ? "platform" : "user_owned");
  const next = {
    ...connections(),
    [id]: {
      connected: false,
      linked: false,
      connectorMode: defaultMode,
      capabilities: [],
      accountLabel: ""
    }
  };
  setConnections(next);
  persistState(next);
  const runtime = profileRuntime();
  if (typeof window !== "undefined") {
    if (id === "github") localStorage.removeItem("github_auth_mode");
    if (integration?.tokenKey) {
      if (runtime.mode === "profile" && runtime.profileLoaded) {
        void removeProfileSecret(integration.tokenKey);
        localStorage.removeItem(integration.tokenKey);
      } else {
        localStorage.removeItem(integration.tokenKey);
      }
    }
  }
  emitIntegrationStateChanged(id, "disconnect");
}

function applySetConnectorModeIntent(payload = {}) {
  const id = String(payload?.id || "").trim();
  const integration = catalog[id];
  if (!integration) return;
  const nextMode = payload.mode === "platform" && integration.supportsPlatformConnector ? "platform" : "user_owned";
  const current = connections()[id] || {};
  const runtime = profileRuntime();
  const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
  const hasToken = integration.requiresToken === false
    ? Boolean(current.connected)
    : Boolean(getRuntimeToken(integration));
  const connected = nextMode === "platform"
    ? Boolean(current.linked && profileReady)
    : hasToken;
  const next = {
    ...connections(),
    [id]: {
      ...current,
      connected,
      linked: nextMode === "platform" ? Boolean(current.linked) : connected,
      connectorMode: nextMode,
      authMethod: integration.authMethod,
      capabilities: connected ? (current.capabilities?.length ? current.capabilities : integration.defaultCapabilities) : [],
      connectedAt: connected ? (current.connectedAt || new Date().toISOString()) : null,
      accountLabel: nextMode === "platform" ? "Platform Connector" : (connected ? (current.accountLabel || `${integration.name} Account`) : "")
    }
  };
  setConnections(next);
  persistState(next);
  emitIntegrationStateChanged(id, "connector_mode");
}

function applyVerificationEvent(payload = {}, ok) {
  const id = String(payload?.id || payload?.integrationId || "").trim();
  if (!id) return;
  setIntegrationVerification((prev) => ({
    ...prev,
    [id]: {
      ok,
      checkedAt: new Date().toISOString(),
      message: String(payload?.message || "").trim(),
      capabilities: Array.isArray(payload?.capabilities) ? payload.capabilities : []
    }
  }));
}

function ensureSubscriptions() {
  if (subscriptionsInitialized) return;
  subscriptionsInitialized = true;

  subscribeEvent(UI_INTENT_TOPICS.integration.checkAll, () => {
    applyCheckAll();
  });

  subscribeEvent(UI_INTENT_TOPICS.integration.connect, (event) => {
    applyConnectIntent(event?.payload || {});
  });

  subscribeEvent(UI_INTENT_TOPICS.integration.disconnect, (event) => {
    applyDisconnectIntent(event?.payload || {});
  });

  subscribeEvent(UI_INTENT_TOPICS.integration.setConnectorMode, (event) => {
    applySetConnectorModeIntent(event?.payload || {});
  });

  subscribeEvent(UI_INTENT_TOPICS.integration.verifySucceeded, (event) => {
    applyVerificationEvent(event?.payload || {}, true);
  });

  subscribeEvent(UI_INTENT_TOPICS.integration.verifyFailed, (event) => {
    applyVerificationEvent(event?.payload || {}, false);
  });
}

ensureSubscriptions();

const integrationStore = {
  checkAll() {
    publishEvent(UI_INTENT_TOPICS.integration.checkAll, {}, uiIntentMeta("integrations.store"));
    return true;
  },
  list() {
    const runtime = profileRuntime();
    const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
    const deviceReady = knownDevices().some((device) => Boolean(device?.online));
    const state = connections();
    return Object.values(catalog).map((integration) => {
      const connection = state[integration.id];
      const connected = Boolean(connection?.connected);
      const available = connected && profileReady && (integration.id === "codex_cli" ? deviceReady : true);
      const availabilityReason = integration.id === "tailscale" && !connected
        ? "Provide Tailscale API key and link integration"
        : !connected
          ? "Not connected"
          : !profileReady
            ? "Profile session required"
            : integration.id === "codex_cli" && !deviceReady
              ? "Connected device required"
              : "Ready";
      return {
        ...integration,
        connected,
        available,
        availabilityReason,
        connectorMode: String(
          connection?.connectorMode
          || integration.defaultConnectorMode
          || (integration.supportsPlatformConnector ? "platform" : "user_owned")
        ),
        supportsPlatformConnector: Boolean(integration.supportsPlatformConnector),
        linked: Boolean(connection?.linked),
        connectedAt: connection?.connectedAt || null,
        accountLabel: connection?.accountLabel || "",
        capabilities: available
          ? connection?.capabilities || integration.defaultCapabilities
          : []
      };
    });
  },
  get(id) {
    return this.list().find((integration) => integration.id === id);
  },
  isConnected(id) {
    return Boolean(connections()[id]?.connected);
  },
  getCapabilities(id) {
    const integration = this.get(id);
    return integration?.capabilities || [];
  },
  hasCapability(capability) {
    for (const integration of this.list()) {
      if (integration.available && integration.capabilities.includes(capability)) return true;
    }
    return false;
  },
  connect(id, details = {}) {
    const integration = catalog[id];
    if (!integration) return false;
    const connectorMode = String(
      details.connectorMode
      || connections()[id]?.connectorMode
      || integration.defaultConnectorMode
      || "user_owned"
    );

    const runtime = profileRuntime();
    const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
    const token = String(details?.token || "").trim();
    if (id === "github" && !profileReady) {
      return false;
    }
    if (typeof window !== "undefined" && integration.tokenKey && token) {
      if (id === "github") {
        void setProfileSecret(integration.tokenKey, token);
        localStorage.removeItem(integration.tokenKey);
      } else if (profileReady) {
        void setProfileSecret(integration.tokenKey, token);
        localStorage.removeItem(integration.tokenKey);
      } else {
        localStorage.setItem(integration.tokenKey, token);
      }
      void syncIntegrationTokenToVault(integration, details);
    }

    publishEvent(
      UI_INTENT_TOPICS.integration.connect,
      {
        id,
        connectorMode,
        accountLabel: String(details.accountLabel || "").trim(),
        capabilities: Array.isArray(details.capabilities) ? details.capabilities : undefined,
        hasToken: Boolean(token)
      },
      uiIntentMeta("integrations.store")
    );
    return true;
  },
  disconnect(id) {
    publishEvent(UI_INTENT_TOPICS.integration.disconnect, { id }, uiIntentMeta("integrations.store"));
  },
  setConnectorMode(id, mode) {
    const integration = catalog[id];
    if (!integration) return false;
    publishEvent(UI_INTENT_TOPICS.integration.setConnectorMode, { id, mode }, uiIntentMeta("integrations.store"));
    return true;
  },
  verification() {
    return integrationVerification();
  },
  async verify(id, details = {}) {
    const integration = catalog[id];
    if (!integration) {
      return { ok: false, message: `Unknown integration: ${id}` };
    }
    publishEvent(UI_INTENT_TOPICS.integration.verifyStarted, { id }, uiIntentMeta("integrations.store"));

    const connectorMode = String(
      details.connectorMode
      || connections()[id]?.connectorMode
      || integration.defaultConnectorMode
      || "user_owned"
    );

    try {
      const runtime = profileRuntime();
      const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
      if (id === "github" && !profileReady) {
        throw new Error("Profile session required for GitHub PAT.");
      }
      if (connectorMode === "platform") {
        if (!profileReady) throw new Error("Profile session required for platform connector.");
        const message = "Platform connector session is active.";
        publishEvent(UI_INTENT_TOPICS.integration.verifySucceeded, { id, message, capabilities: integration.defaultCapabilities }, uiIntentMeta("integrations.store"));
        publishEvent(UI_EVENT_TOPICS.integration.verified, { integrationId: id, message }, uiIntentMeta("integrations.store"));
        return { ok: true, message };
      }

      if (id === "codex_cli") {
        const deviceReady = knownDevices().some((device) => Boolean(device?.online));
        if (!deviceReady) throw new Error("No connected node manager device is online.");
        const message = "Connected device is online for local CLI execution.";
        publishEvent(UI_INTENT_TOPICS.integration.verifySucceeded, { id, message, capabilities: integration.defaultCapabilities }, uiIntentMeta("integrations.store"));
        publishEvent(UI_EVENT_TOPICS.integration.verified, { integrationId: id, message }, uiIntentMeta("integrations.store"));
        return { ok: true, message };
      }

      if (id === "tailscale") {
        const apiKey = String(details.apiKey || details.token || "").trim() || getRuntimeToken(integration);
        const tailnet = String(details.tailnet || "").trim();
        if (!apiKey || !tailnet) throw new Error("Tailscale API key and tailnet are required.");
        const response = await fetch("/api/tailscale/devices", {
          method: "POST",
          headers: { "content-type": "application/json; charset=utf-8" },
          body: JSON.stringify({ apiKey, tailnet })
        });
        const body = await response.json().catch(() => ({}));
        if (!response.ok || !body?.ok) {
          throw new Error(String(body?.error || `tailscale devices request failed (${response.status})`));
        }
        const count = Array.isArray(body?.devices) ? body.devices.length : 0;
        const message = `Verified Tailscale API access (${count} devices visible).`;
        publishEvent(UI_INTENT_TOPICS.integration.verifySucceeded, { id, message, capabilities: integration.defaultCapabilities }, uiIntentMeta("integrations.store"));
        publishEvent(UI_EVENT_TOPICS.integration.verified, { integrationId: id, message }, uiIntentMeta("integrations.store"));
        return { ok: true, message, devices: Array.isArray(body?.devices) ? body.devices : [] };
      }

      if (id === "web3") {
        const wallet = String(details.wallet || "").trim();
        if (!wallet || !wallet.startsWith("0x")) throw new Error("Connect EVM wallet first.");
        const message = "Wallet is connected and ready.";
        publishEvent(UI_INTENT_TOPICS.integration.verifySucceeded, { id, message, capabilities: integration.defaultCapabilities }, uiIntentMeta("integrations.store"));
        publishEvent(UI_EVENT_TOPICS.integration.verified, { integrationId: id, message }, uiIntentMeta("integrations.store"));
        return { ok: true, message };
      }

      const token = String(details.token || "").trim() || getRuntimeToken(integration);
      if (integration.requiresToken !== false && token.length < 8) {
        throw new Error(`${integration.name} token missing or invalid.`);
      }
      const message = `${integration.name} credentials accepted.`;
      publishEvent(UI_INTENT_TOPICS.integration.verifySucceeded, { id, message, capabilities: integration.defaultCapabilities }, uiIntentMeta("integrations.store"));
      publishEvent(UI_EVENT_TOPICS.integration.verified, { integrationId: id, message }, uiIntentMeta("integrations.store"));
      return { ok: true, message };
    } catch (error) {
      const message = error instanceof Error ? error.message : `Failed to verify ${integration.name}.`;
      publishEvent(UI_INTENT_TOPICS.integration.verifyFailed, { id, message }, uiIntentMeta("integrations.store"));
      publishEvent(UI_EVENT_TOPICS.integration.verifyFailed, { integrationId: id, message }, uiIntentMeta("integrations.store"));
      return { ok: false, message };
    }
  }
};

export {
  integrationStore,
  integrationVerification
};
