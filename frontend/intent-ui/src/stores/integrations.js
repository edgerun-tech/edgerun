import { createSignal } from "solid-js";
import { UI_EVENT_TOPICS, UI_INTENT_TOPICS, uiIntentMeta } from "../lib/ui-intents";
import { createIntegrationCatalog } from "../lib/integrations/catalog";
import { publishEvent, subscribeEvent } from "./eventbus";
import { profileRuntime } from "./profile-runtime";
import { knownDevices } from "./devices";
import { getProfileSecret, removeProfileSecret, setProfileSecret } from "./profile-secrets";
import { callIntegrationWorker, initializeIntegrationWorker } from "./integrations-worker";

const STORAGE_KEY = "intent-ui-integrations-v1";
let cachedVaultStatus = null;
let vaultStatusCheckedAt = 0;
const VAULT_STATUS_TTL_MS = 30 * 1000;
let subscriptionsInitialized = false;

const catalog = createIntegrationCatalog();

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
    const token = getRuntimeToken(integration);
    next[integration.id] = integration.hydrateConnection({
      storedConnection: next[integration.id] || {},
      profileReady,
      token
    });
  }
  return next;
}

async function hydrateStateInWorker() {
  const stored = readStoredState();
  const next = { ...stored };
  const runtime = profileRuntime();
  const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
  for (const integration of Object.values(catalog)) {
    const token = getRuntimeToken(integration);
    try {
      next[integration.id] = await callIntegrationWorker("hydrate_connection", {
        integrationId: integration.id,
        storedConnection: next[integration.id] || {},
        profileReady,
        token
      });
    } catch {
      next[integration.id] = integration.hydrateConnection({
        storedConnection: next[integration.id] || {},
        profileReady,
        token
      });
    }
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

async function applyCheckAll() {
  try {
    const next = await hydrateStateInWorker();
    setConnections(next);
    persistState(next);
    publishEvent(UI_EVENT_TOPICS.integration.stateChanged, { integrationId: "*", reason: "check_all" }, uiIntentMeta("integrations.reducer"));
  } catch {
    // keep last known state if worker is unavailable
  }
}

async function applyConnectIntent(payload = {}) {
  const id = String(payload?.id || "").trim();
  const integration = catalog[id];
  if (!integration) return;
  const runtime = profileRuntime();
  const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
  const token = getRuntimeToken(integration);
  let nextConnection = null;
  try {
    nextConnection = await callIntegrationWorker("connect_connection", {
      integrationId: id,
      currentConnection: connections()[id] || {},
      payload,
      profileReady,
      token
    });
  } catch {
    nextConnection = integration.connectConnection({
      currentConnection: connections()[id] || {},
      payload,
      profileReady,
      token
    });
  }
  const next = {
    ...connections(),
    [id]: nextConnection
  };
  setConnections(next);
  persistState(next);
  emitIntegrationStateChanged(id, "connect");
}

async function applyDisconnectIntent(payload = {}) {
  const id = String(payload?.id || "").trim();
  const integration = catalog[id];
  if (!integration) return;
  let nextConnection = null;
  try {
    nextConnection = await callIntegrationWorker("disconnect_connection", {
      integrationId: id
    });
  } catch {
    nextConnection = integration.disconnectConnection();
  }
  const next = {
    ...connections(),
    [id]: nextConnection
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

async function applySetConnectorModeIntent(payload = {}) {
  const id = String(payload?.id || "").trim();
  const integration = catalog[id];
  if (!integration) return;
  const runtime = profileRuntime();
  const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
  const current = connections()[id] || {};
  const token = getRuntimeToken(integration);
  let nextConnection = null;
  try {
    nextConnection = await callIntegrationWorker("set_mode_connection", {
      integrationId: id,
      currentConnection: current,
      mode: payload.mode,
      profileReady,
      token
    });
  } catch {
    nextConnection = integration.setConnectorModeConnection({
      currentConnection: current,
      mode: payload.mode,
      profileReady,
      token
    });
  }
  const next = {
    ...connections(),
    [id]: nextConnection
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
  initializeIntegrationWorker();

  subscribeEvent(UI_INTENT_TOPICS.integration.checkAll, () => {
    void applyCheckAll();
  });

  subscribeEvent(UI_INTENT_TOPICS.integration.connect, (event) => {
    void applyConnectIntent(event?.payload || {});
  });

  subscribeEvent(UI_INTENT_TOPICS.integration.disconnect, (event) => {
    void applyDisconnectIntent(event?.payload || {});
  });

  subscribeEvent(UI_INTENT_TOPICS.integration.setConnectorMode, (event) => {
    void applySetConnectorModeIntent(event?.payload || {});
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
      const lifecycle = integration.listConnectionView({
        connection,
        profileReady,
        deviceReady
      });
      return {
        ...integration,
        ...lifecycle
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
  getToken(id) {
    const integration = catalog[id];
    if (!integration) return "";
    return getRuntimeToken(integration);
  },
  async connect(id, details = {}) {
    const integration = catalog[id];
    if (!integration) return false;
    const connectorMode = String(
      details.connectorMode
      || connections()[id]?.connectorMode
      || integration.getDefaultConnectorMode()
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
        await setProfileSecret(integration.tokenKey, token);
        localStorage.removeItem(integration.tokenKey);
      } else if (profileReady) {
        await setProfileSecret(integration.tokenKey, token);
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
      || integration.getDefaultConnectorMode()
    );

    try {
      const runtime = profileRuntime();
      const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
      const deviceReady = knownDevices().some((device) => Boolean(device?.online));
      const token = String(details.token || "").trim() || getRuntimeToken(integration);
      let result = null;
      try {
        result = await callIntegrationWorker("verify_integration", {
          integrationId: id,
          details,
          connectorMode,
          profileReady,
          deviceReady,
          token
        });
      } catch {
        result = await integration.verifyConnection({
          details,
          connectorMode,
          profileReady,
          deviceReady,
          token,
          fetchImpl: fetch
        });
      }
      if (!result?.ok) {
        const message = String(result?.message || `Failed to verify ${integration.name}.`);
        publishEvent(UI_INTENT_TOPICS.integration.verifyFailed, { id, message }, uiIntentMeta("integrations.store"));
        publishEvent(UI_EVENT_TOPICS.integration.verifyFailed, { integrationId: id, message }, uiIntentMeta("integrations.store"));
        return { ok: false, message };
      }
      const message = String(result?.message || `${integration.name} credentials accepted.`);
      publishEvent(
        UI_INTENT_TOPICS.integration.verifySucceeded,
        { id, message, capabilities: Array.isArray(result?.capabilities) ? result.capabilities : integration.defaultCapabilities },
        uiIntentMeta("integrations.store")
      );
      publishEvent(UI_EVENT_TOPICS.integration.verified, { integrationId: id, message }, uiIntentMeta("integrations.store"));
      return { ok: true, message, devices: Array.isArray(result?.devices) ? result.devices : [] };
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
