import { createSignal } from "solid-js";
import { UI_EVENT_TOPICS, UI_INTENT_TOPICS, uiIntentMeta } from "../lib/ui-intents";
import { createIntegrationCatalog } from "../lib/integrations/catalog";
import { publishEvent, subscribeEvent } from "./eventbus";
import { profileRuntime } from "./profile-runtime";
import { knownDevices } from "./devices";
import { callIntegrationWorker, initializeIntegrationWorker } from "./integrations-worker";
import { localBridgeHttpUrl } from "../lib/local-bridge-origin";
import { probeFlipper, verifyFlipperBluetooth } from "../lib/integrations/flipper-ble";
import { probeDalyBms, verifyDalyBmsBluetooth } from "../lib/integrations/daly-bms-ble";
import { OFFICIAL_BRIDGES, canonicalBridgeId, isOfficialBridgeId } from "../lib/integrations/official-bridges";

const STORAGE_KEY = "intent-ui-integrations-v1";
let cachedVaultStatus = null;
let vaultStatusCheckedAt = 0;
const VAULT_STATUS_TTL_MS = 30 * 1000;
let subscriptionsInitialized = false;
const MCP_ENABLED_INTEGRATIONS = new Set(["github", ...OFFICIAL_BRIDGES.map((bridge) => bridge.id)]);

const catalog = createIntegrationCatalog();

function normalizeIntegrationId(value) {
  const raw = String(value || "").trim();
  if (!raw) return "";
  const bridgeCanonical = canonicalBridgeId(raw);
  if (bridgeCanonical && catalog[bridgeCanonical]) return bridgeCanonical;
  return raw;
}

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
    const response = await fetch(localBridgeHttpUrl("/v1/local/credentials/status"), { cache: "no-store" });
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
    await fetch(localBridgeHttpUrl("/v1/local/credentials/store"), {
      method: "POST",
      headers: { "content-type": "application/json; charset=utf-8" },
      body: JSON.stringify({
        credentialType: "token",
        entryId: entryName,
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

async function removeIntegrationTokenFromVault(integrationId) {
  const id = String(integrationId || "").trim();
  if (!id) return;
  try {
    await fetch(localBridgeHttpUrl("/v1/local/credentials/delete"), {
      method: "POST",
      headers: { "content-type": "application/json; charset=utf-8" },
      body: JSON.stringify({ entryId: `integration/${id}/token` })
    });
  } catch {
    // best effort cleanup only
  }
}

async function hydrateIntegrationTokenFromVault(integration) {
  if (typeof window === "undefined" || !integration?.id || !integration?.tokenKey) return;
  const existing = String(localStorage.getItem(integration.tokenKey) || "").trim();
  if (existing) return;
  try {
    const query = new URLSearchParams({ integration_id: integration.id });
    const response = await fetch(
      localBridgeHttpUrl(`/v1/local/credentials/integration-token?${query.toString()}`),
      { cache: "no-store" }
    );
    const payload = await response.json().catch(() => ({}));
    const token = String(payload?.token || "").trim();
    if (response.ok && payload?.ok !== false && token) {
      localStorage.setItem(integration.tokenKey, token);
    }
  } catch {
    // ignore vault hydrate errors and keep current runtime state
  }
}

function getRuntimeToken(integration) {
  if (!integration?.tokenKey) return "";
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
const [integrationLifecycle, setIntegrationLifecycle] = createSignal({});

function setLifecycleState(id, status, message = "", extras = {}) {
  const integrationId = String(id || "").trim();
  if (!integrationId) return;
  const next = {
    integrationId,
    status: String(status || "idle").trim() || "idle",
    message: String(message || "").trim(),
    updatedAt: new Date().toISOString(),
    ...extras
  };
  setIntegrationLifecycle((prev) => ({
    ...prev,
    [integrationId]: next
  }));
  publishEvent(
    UI_EVENT_TOPICS.integration.lifecycleChanged,
    next,
    uiIntentMeta("integrations.reducer")
  );
}

function syncLifecycleFromConnections(state = connections()) {
  const nextLifecycle = {};
  for (const [id, connection] of Object.entries(state || {})) {
    const connected = Boolean(connection?.connected);
    nextLifecycle[id] = {
      integrationId: id,
      status: connected ? "connected" : "disconnected",
      message: connected ? "Integration connected." : "Integration not connected.",
      updatedAt: new Date().toISOString()
    };
  }
  setIntegrationLifecycle((prev) => ({ ...prev, ...nextLifecycle }));
}

function emitIntegrationStateChanged(id, reason) {
  publishEvent(
    UI_EVENT_TOPICS.integration.stateChanged,
    { integrationId: id, reason },
    uiIntentMeta("integrations.reducer")
  );
}

async function applyCheckAll() {
  try {
    for (const integration of Object.values(catalog)) {
      if (!integration?.tokenKey) continue;
      await hydrateIntegrationTokenFromVault(integration);
    }
    const hydrated = await hydrateStateInWorker();
    const next = await syncRuntimeBackedConnectionTruth(hydrated);
    setConnections(next);
    persistState(next);
    syncLifecycleFromConnections(next);
    publishEvent(UI_EVENT_TOPICS.integration.stateChanged, { integrationId: "*", reason: "check_all" }, uiIntentMeta("integrations.reducer"));
  } catch {
    // keep last known state if worker is unavailable
  }
}

async function applyConnectIntent(payload = {}) {
  const id = normalizeIntegrationId(payload?.id);
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
  setLifecycleState(id, nextConnection?.connected ? "connected" : "error", nextConnection?.connected ? "Integration connected." : "Failed to connect integration.");
  emitIntegrationStateChanged(id, "connect");
}

async function applyDisconnectIntent(payload = {}) {
  const id = normalizeIntegrationId(payload?.id);
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
  if (typeof window !== "undefined") {
    if (id === "github") localStorage.removeItem("github_auth_mode");
    if (integration?.tokenKey) {
      localStorage.removeItem(integration.tokenKey);
      void removeIntegrationTokenFromVault(id);
    }
  }
  setLifecycleState(id, "disconnected", "Integration disconnected.");
  emitIntegrationStateChanged(id, "disconnect");
}

async function applySetConnectorModeIntent(payload = {}) {
  const id = normalizeIntegrationId(payload?.id);
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
  setLifecycleState(id, nextConnection?.connected ? "connected" : "disconnected", "Connector mode updated.");
  emitIntegrationStateChanged(id, "connector_mode");
}

function applyVerificationEvent(payload = {}, ok) {
  const id = normalizeIntegrationId(payload?.id || payload?.integrationId);
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
  setLifecycleState(
    id,
    ok ? "verified" : "error",
    String(payload?.message || "").trim() || (ok ? "Verification succeeded." : "Verification failed."),
    {
      capabilities: Array.isArray(payload?.capabilities) ? payload.capabilities : []
    }
  );
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

  subscribeEvent(UI_EVENT_TOPICS.integration.verified, (event) => {
    applyVerificationEvent(event?.payload || {}, true);
  });

  subscribeEvent(UI_EVENT_TOPICS.integration.verifyFailed, (event) => {
    applyVerificationEvent(event?.payload || {}, false);
  });
}

ensureSubscriptions();

function resolveLocalNodeManagerId() {
  const host = knownDevices().find((device) => device?.type === "host" && device?.online);
  return String(host?.id || "local-node-manager").trim();
}

async function startIntegrationMcp(integrationId, token) {
  if (!MCP_ENABLED_INTEGRATIONS.has(integrationId)) return { ok: true, skipped: true };
  const trimmed = String(token || "").trim();
  if (trimmed.length < 8) return { ok: false, message: "missing integration token for MCP runtime" };
  const response = await fetch(localBridgeHttpUrl("/v1/local/mcp/integration/start"), {
    method: "POST",
    headers: { "content-type": "application/json; charset=utf-8" },
    body: JSON.stringify({
      integration_id: integrationId,
      token: trimmed,
      node_id: resolveLocalNodeManagerId()
    })
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok || payload?.ok === false) {
    return { ok: false, message: String(payload?.error || `mcp start failed (${response.status})`) };
  }
  return { ok: true, data: payload };
}

async function stopIntegrationMcp(integrationId) {
  if (!MCP_ENABLED_INTEGRATIONS.has(integrationId)) return { ok: true, skipped: true };
  const response = await fetch(localBridgeHttpUrl("/v1/local/mcp/integration/stop"), {
    method: "POST",
    headers: { "content-type": "application/json; charset=utf-8" },
    body: JSON.stringify({
      integration_id: integrationId,
      node_id: resolveLocalNodeManagerId()
    })
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok || payload?.ok === false) {
    return { ok: false, message: String(payload?.error || `mcp stop failed (${response.status})`) };
  }
  return { ok: true, data: payload };
}

async function getIntegrationMcpStatus(integrationId) {
  if (!MCP_ENABLED_INTEGRATIONS.has(integrationId)) return { ok: true, skipped: true, running: false, status: "not_applicable" };
  const params = new URLSearchParams({
    integration_id: integrationId,
    node_id: resolveLocalNodeManagerId()
  });
  const response = await fetch(localBridgeHttpUrl(`/v1/local/mcp/integration/status?${params.toString()}`), {
    cache: "no-store"
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok || payload?.ok === false) {
    return {
      ok: false,
      running: false,
      status: "error",
      message: String(payload?.error || `mcp status failed (${response.status})`)
    };
  }
  return {
    ok: true,
    running: Boolean(payload?.data?.running),
    status: String(payload?.data?.status || "").trim() || (payload?.data?.running ? "running" : "stopped"),
    data: payload?.data || {}
  };
}

async function syncRuntimeBackedConnectionTruth(state) {
  const next = { ...(state || {}) };
  for (const integration of Object.values(catalog)) {
    if (!MCP_ENABLED_INTEGRATIONS.has(integration.id)) continue;
    const current = next[integration.id] || {};
    const mode = String(current?.connectorMode || integration.getDefaultConnectorMode() || "user_owned").trim();
    const token = getRuntimeToken(integration);
    if (mode !== "user_owned" || token.length < 8) {
      void stopIntegrationMcp(integration.id);
      next[integration.id] = {
        ...current,
        connectorMode: "user_owned",
        connected: false,
        linked: false,
        capabilities: []
      };
      continue;
    }
    try {
      const status = await getIntegrationMcpStatus(integration.id);
      if (!status.ok || !status.running) {
        next[integration.id] = {
          ...current,
          connectorMode: "user_owned",
          connected: false,
          linked: false,
          capabilities: []
        };
        continue;
      }
      if (!Boolean(current?.connected)) {
        void stopIntegrationMcp(integration.id);
        next[integration.id] = {
          ...current,
          connectorMode: "user_owned",
          connected: false,
          linked: false,
          capabilities: []
        };
        continue;
      }
      next[integration.id] = {
        ...current,
        connectorMode: "user_owned",
        connected: true,
        linked: true,
        capabilities: Array.isArray(current?.capabilities) && current.capabilities.length > 0
          ? current.capabilities.slice()
          : integration.defaultCapabilities.slice()
      };
    } catch {
      next[integration.id] = {
        ...current,
        connectorMode: "user_owned",
        connected: false,
        linked: false,
        capabilities: []
      };
    }
  }
  return next;
}

async function verifyFlipperWebBluetooth(integration, details = {}) {
  const resolveErrorMessage = (error, fallback) => {
    if (error instanceof Error && String(error.message || "").trim()) return error.message;
    if (error && typeof error === "object" && "message" in error) {
      const text = String(error.message || "").trim();
      if (text) return text;
    }
    return fallback;
  };
  try {
    const verified = await verifyFlipperBluetooth(details);
    return {
      ok: true,
      message: verified.warning
        ? `Verified Web Bluetooth access to ${verified.deviceName} (warning: ${verified.warning}).`
        : `Verified Web Bluetooth access to ${verified.deviceName}.`,
      capabilities: integration.defaultCapabilities.slice(),
      deviceId: verified.deviceId,
      deviceName: verified.deviceName
    };
  } catch (error) {
    return { ok: false, message: resolveErrorMessage(error, "Failed to verify Flipper over Web Bluetooth.") };
  }
}

async function verifyDalyBmsWebBluetooth(integration, details = {}) {
  const resolveErrorMessage = (error, fallback) => {
    if (error instanceof Error && String(error.message || "").trim()) return error.message;
    if (error && typeof error === "object" && "message" in error) {
      const text = String(error.message || "").trim();
      if (text) return text;
    }
    return fallback;
  };
  try {
    const verified = await verifyDalyBmsBluetooth(details);
    return {
      ok: true,
      message: verified.profileLabel
        ? `Verified Web Bluetooth access to ${verified.deviceName} (${verified.profileLabel} profile).`
        : `Verified Web Bluetooth access to ${verified.deviceName}.`,
      capabilities: integration.defaultCapabilities.slice(),
      deviceId: verified.deviceId,
      deviceName: verified.deviceName
    };
  } catch (error) {
    return { ok: false, message: resolveErrorMessage(error, "Failed to verify Daly BMS over Web Bluetooth.") };
  }
}

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
      const lifecycleState = integrationLifecycle()[integration.id] || {
        integrationId: integration.id,
        status: connection?.connected ? "connected" : "idle",
        message: "",
        updatedAt: null
      };
      const lifecycle = integration.listConnectionView({
        connection,
        profileReady,
        deviceReady
      });
      return {
        ...integration,
        ...lifecycle,
        lifecycleStatus: lifecycleState.status,
        lifecycleMessage: lifecycleState.message,
        lifecycleUpdatedAt: lifecycleState.updatedAt
      };
    });
  },
  get(id) {
    const normalizedId = normalizeIntegrationId(id);
    return this.list().find((integration) => integration.id === normalizedId);
  },
  isConnected(id) {
    const normalizedId = normalizeIntegrationId(id);
    return Boolean(connections()[normalizedId]?.connected);
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
    const normalizedId = normalizeIntegrationId(id);
    const integration = catalog[normalizedId];
    if (!integration) return "";
    return getRuntimeToken(integration);
  },
  async connect(id, details = {}) {
    const normalizedId = normalizeIntegrationId(id);
    const integration = catalog[normalizedId];
    if (!integration) return false;
    const connectorMode = String(
      details.connectorMode
      || connections()[normalizedId]?.connectorMode
      || integration.getDefaultConnectorMode()
      || "user_owned"
    );

    const token = String(details?.token || "").trim();
    if (typeof window !== "undefined" && integration.tokenKey && token) {
      localStorage.setItem(integration.tokenKey, token);
      await syncIntegrationTokenToVault(integration, details);
    }

    setLifecycleState(normalizedId, "linking", "Linking integration...");
    if (token && MCP_ENABLED_INTEGRATIONS.has(normalizedId)) {
      try {
        const result = await startIntegrationMcp(normalizedId, token);
        if (!result.ok) {
          setLifecycleState(normalizedId, "error", `MCP runtime failed to start: ${result.message || "unknown error"}`);
          publishEvent(
            UI_EVENT_TOPICS.integration.verifyFailed,
            { integrationId: normalizedId, message: `MCP runtime failed to start: ${result.message || "unknown error"}` },
            uiIntentMeta("integrations.store")
          );
          return false;
        }
        const status = await getIntegrationMcpStatus(normalizedId);
        if (!status.ok || !status.running) {
          setLifecycleState(normalizedId, "error", `MCP runtime not healthy: ${status.message || status.status || "not running"}`);
          publishEvent(
            UI_EVENT_TOPICS.integration.verifyFailed,
            { integrationId: normalizedId, message: `MCP runtime not healthy: ${status.message || status.status || "not running"}` },
            uiIntentMeta("integrations.store")
          );
          return false;
        } else {
          setLifecycleState(normalizedId, "connected", "Integration linked and MCP runtime started.");
        }
      } catch (error) {
        setLifecycleState(normalizedId, "error", `MCP runtime start failed: ${error instanceof Error ? error.message : "unknown error"}`);
        publishEvent(
          UI_EVENT_TOPICS.integration.verifyFailed,
          { integrationId: normalizedId, message: `MCP runtime start failed: ${error instanceof Error ? error.message : "unknown error"}` },
          uiIntentMeta("integrations.store")
        );
        return false;
      }
    }
    publishEvent(
      UI_INTENT_TOPICS.integration.connect,
      {
        id: normalizedId,
        connectorMode,
        accountLabel: String(details.accountLabel || "").trim(),
        capabilities: Array.isArray(details.capabilities) ? details.capabilities : undefined,
        hasToken: Boolean(token)
      },
      uiIntentMeta("integrations.store")
    );
    if (token && MCP_ENABLED_INTEGRATIONS.has(normalizedId)) {
      publishEvent(
        UI_EVENT_TOPICS.integration.stateChanged,
        { integrationId: normalizedId, reason: "mcp_started" },
        uiIntentMeta("integrations.store")
      );
    }
    return true;
  },
  disconnect(id) {
    const normalizedId = normalizeIntegrationId(id);
    setLifecycleState(normalizedId, "disconnecting", "Disconnecting integration...");
    publishEvent(UI_INTENT_TOPICS.integration.disconnect, { id: normalizedId }, uiIntentMeta("integrations.store"));
    void stopIntegrationMcp(normalizedId);
  },
  setConnectorMode(id, mode) {
    const normalizedId = normalizeIntegrationId(id);
    const integration = catalog[normalizedId];
    if (!integration) return false;
    publishEvent(UI_INTENT_TOPICS.integration.setConnectorMode, { id: normalizedId, mode }, uiIntentMeta("integrations.store"));
    return true;
  },
  verification() {
    return integrationVerification();
  },
  async runtimeStatus(id) {
    const normalizedId = normalizeIntegrationId(id);
    if (!normalizedId) return { ok: false, state: "unknown", message: "integration id is required" };
    if (!MCP_ENABLED_INTEGRATIONS.has(normalizedId)) {
      return { ok: true, state: "not_applicable", running: false, message: "No runtime container for this integration." };
    }
    try {
      const status = await getIntegrationMcpStatus(normalizedId);
      if (!status.ok) {
        return {
          ok: false,
          state: "error",
          running: false,
          message: String(status.message || "failed to read runtime status")
        };
      }
      return {
        ok: true,
        state: status.running ? "running" : "stopped",
        running: Boolean(status.running),
        status: String(status.status || ""),
        message: status.running ? "Runtime container is running." : "Runtime container is not started."
      };
    } catch (error) {
      return {
        ok: false,
        state: "error",
        running: false,
        message: error instanceof Error ? error.message : "Failed to query runtime status."
      };
    }
  },
  async verify(id, details = {}) {
    const normalizedId = normalizeIntegrationId(id);
    const integration = catalog[normalizedId];
    if (!integration) {
      return { ok: false, message: `Unknown integration: ${id}` };
    }
    setLifecycleState(normalizedId, "verifying", "Running integration verification...");
    publishEvent(UI_INTENT_TOPICS.integration.verifyStarted, { id: normalizedId }, uiIntentMeta("integrations.store"));
    publishEvent(UI_EVENT_TOPICS.integration.verifyStarted, { integrationId: normalizedId }, uiIntentMeta("integrations.store"));

    const connectorMode = String(
      details.connectorMode
      || connections()[normalizedId]?.connectorMode
      || integration.getDefaultConnectorMode()
    );

    try {
      const runtime = profileRuntime();
      const profileReady = runtime.mode === "profile" && runtime.profileLoaded;
      const deviceReady = knownDevices().some((device) => Boolean(device?.online));
      const token = String(details.token || "").trim() || getRuntimeToken(integration);
      let result = null;
      if (normalizedId === "flipper") {
        result = await verifyFlipperWebBluetooth(integration, details);
      } else if (normalizedId === "daly_bms") {
        result = await verifyDalyBmsWebBluetooth(integration, details);
      } else if (isOfficialBridgeId(normalizedId)) {
        if (token.length < 8) {
          result = { ok: false, message: `${integration.name} bridge token missing or invalid.` };
        } else {
          const statusBefore = await getIntegrationMcpStatus(normalizedId);
          if (statusBefore.ok && statusBefore.running) {
            result = {
              ok: true,
              message: `${integration.name} Matrix bridge runtime is running (${statusBefore.status}).`,
              capabilities: integration.defaultCapabilities.slice()
            };
          } else {
            result = {
              ok: true,
              message: `${integration.name} credentials accepted. Runtime starts on Link Integration.`,
              capabilities: integration.defaultCapabilities.slice()
            };
          }
        }
      } else {
        try {
          result = await callIntegrationWorker("verify_integration", {
            integrationId: normalizedId,
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
      }
      if (!result?.ok) {
        const message = String(result?.message || `Failed to verify ${integration.name}.`);
        publishEvent(UI_INTENT_TOPICS.integration.verifyFailed, { id: normalizedId, message }, uiIntentMeta("integrations.store"));
        publishEvent(UI_EVENT_TOPICS.integration.verifyFailed, { integrationId: normalizedId, message }, uiIntentMeta("integrations.store"));
        return { ok: false, message };
      }
      const message = String(result?.message || `${integration.name} credentials accepted.`);
      publishEvent(
        UI_INTENT_TOPICS.integration.verifySucceeded,
        { id: normalizedId, message, capabilities: Array.isArray(result?.capabilities) ? result.capabilities : integration.defaultCapabilities },
        uiIntentMeta("integrations.store")
      );
      publishEvent(
        UI_EVENT_TOPICS.integration.verified,
        {
          integrationId: normalizedId,
          message,
          capabilities: Array.isArray(result?.capabilities) ? result.capabilities : integration.defaultCapabilities
        },
        uiIntentMeta("integrations.store")
      );
      return { ok: true, message, devices: Array.isArray(result?.devices) ? result.devices : [] };
    } catch (error) {
      const message = error instanceof Error ? error.message : `Failed to verify ${integration.name}.`;
      publishEvent(UI_INTENT_TOPICS.integration.verifyFailed, { id: normalizedId, message }, uiIntentMeta("integrations.store"));
      publishEvent(UI_EVENT_TOPICS.integration.verifyFailed, { integrationId: normalizedId, message }, uiIntentMeta("integrations.store"));
      return { ok: false, message };
    }
  }
  ,
  async probeFlipper(details = {}) {
    try {
      const result = await probeFlipper(details);
      publishEvent(
        UI_EVENT_TOPICS.integration.flipperProbed,
        result,
        uiIntentMeta("integrations.store")
      );
      return { ok: true, ...result };
    } catch (error) {
      return { ok: false, message: error instanceof Error ? error.message : "Failed to probe Flipper." };
    }
  },
  async probeDalyBms(details = {}) {
    try {
      const result = await probeDalyBms(details);
      publishEvent(
        UI_EVENT_TOPICS.integration.dalyBmsProbed,
        result,
        uiIntentMeta("integrations.store")
      );
      return { ok: true, ...result };
    } catch (error) {
      return { ok: false, message: error instanceof Error ? error.message : "Failed to probe Daly BMS." };
    }
  }
};

export {
  integrationStore,
  integrationVerification,
  integrationLifecycle
};
