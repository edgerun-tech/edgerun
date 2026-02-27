import { createSignal } from "solid-js";

const STORAGE_KEY = "intent-ui-integrations-v1";
let cachedVaultStatus = null;
let vaultStatusCheckedAt = 0;
const VAULT_STATUS_TTL_MS = 30 * 1000;

const catalog = {
  github: {
    id: "github",
    name: "GitHub",
    authMethod: "oidc",
    tokenKey: "github_token",
    defaultCapabilities: ["repos.read", "repos.write", "prs.read", "prs.write"]
  },
  cloudflare: {
    id: "cloudflare",
    name: "Cloudflare",
    authMethod: "token",
    tokenKey: "cloudflare_token",
    defaultCapabilities: ["zones.read", "workers.read", "workers.write"]
  },
  vercel: {
    id: "vercel",
    name: "Vercel",
    authMethod: "token",
    tokenKey: "vercel_token",
    defaultCapabilities: ["projects.read", "deployments.read", "deployments.write"]
  },
  google: {
    id: "google",
    name: "Google",
    authMethod: "oauth",
    tokenKey: "google_token",
    defaultCapabilities: ["drive.read", "gmail.read", "calendar.read", "contacts.read", "messages.read"]
  },
  google_photos: {
    id: "google_photos",
    name: "Google Photos",
    authMethod: "oauth",
    tokenKey: "google_token",
    defaultCapabilities: ["photos.read"]
  },
  qwen: {
    id: "qwen",
    name: "Qwen",
    authMethod: "oauth",
    tokenKey: "qwen_token",
    defaultCapabilities: ["chat.completions"]
  },
  hetzner: {
    id: "hetzner",
    name: "Hetzner",
    authMethod: "token",
    tokenKey: "hetzner_token",
    defaultCapabilities: ["servers.read", "servers.write", "firewalls.read"]
  },
  web3: {
    id: "web3",
    name: "Web3",
    authMethod: "wallet",
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

function hydrateState() {
  const stored = readStoredState();
  const next = { ...stored };
  for (const integration of Object.values(catalog)) {
    const rawToken = typeof window !== "undefined" && integration.tokenKey
      ? localStorage.getItem(integration.tokenKey)
      : null;
    const token = typeof rawToken === "string" ? rawToken.trim() : "";
    const hasUsableToken = integration.id === "github" ? Boolean(token && !token.startsWith("oidc_")) : Boolean(token);
    if (!hasUsableToken && integration.id === "github" && next[integration.id]?.connected) {
      next[integration.id] = {
        connected: false,
        capabilities: [],
        accountLabel: ""
      };
    }
    if (hasUsableToken && !next[integration.id]?.connected) {
      next[integration.id] = {
        connected: true,
        authMethod: integration.authMethod,
        capabilities: integration.defaultCapabilities,
        connectedAt: new Date().toISOString(),
        accountLabel: `${integration.name} Account`
      };
    }
  }
  return next;
}

const [connections, setConnections] = createSignal(hydrateState());

const integrationStore = {
  checkAll() {
    const next = hydrateState();
    setConnections(next);
    persistState(next);
    return true;
  },
  list() {
    const state = connections();
    return Object.values(catalog).map((integration) => {
      const connection = state[integration.id];
      return {
        ...integration,
        connected: Boolean(connection?.connected),
        connectedAt: connection?.connectedAt || null,
        accountLabel: connection?.accountLabel || "",
        capabilities: connection?.connected
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
      if (integration.connected && integration.capabilities.includes(capability)) return true;
    }
    return false;
  },
  connect(id, details = {}) {
    const integration = catalog[id];
    if (!integration) return false;
    const next = {
      ...connections(),
      [id]: {
        connected: true,
        authMethod: integration.authMethod,
        capabilities: details.capabilities || integration.defaultCapabilities,
        connectedAt: new Date().toISOString(),
        accountLabel: details.accountLabel || ""
      }
    };
    setConnections(next);
    persistState(next);
    if (typeof window !== "undefined") {
      if (integration.tokenKey && typeof details.token === "string") {
        localStorage.setItem(integration.tokenKey, details.token);
        void syncIntegrationTokenToVault(integration, details);
      }
    }
    return true;
  },
  disconnect(id) {
    const next = {
      ...connections(),
      [id]: { connected: false, capabilities: [], accountLabel: "" }
    };
    setConnections(next);
    persistState(next);
    if (typeof window !== "undefined") {
      if (id === "github") {
        localStorage.removeItem("github_auth_mode");
      }
      const integration = catalog[id];
      if (integration?.tokenKey) {
        localStorage.removeItem(integration.tokenKey);
      }
    }
  }
};

export { integrationStore };
