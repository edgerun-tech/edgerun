import { For, Show, createMemo, createSignal, createEffect, onMount } from "solid-js";
import {
  FiGithub,
  FiLink2,
  FiCheckCircle,
  FiCloud,
  FiDatabase,
  FiCpu,
  FiGlobe,
  FiBox,
  FiSearch,
  FiKey,
  FiXCircle,
  FiSettings,
  FiShield,
  FiCopy
} from "solid-icons/fi";
import { integrationStore } from "../../stores/integrations";
import { openWindow } from "../../stores/windows";
import { setAssistantProvider, workflowUi } from "../../stores/workflow-ui";
import { profileRuntime } from "../../stores/profile-runtime";
import { getProfileSecret, setProfileSecret } from "../../stores/profile-secrets";

const WEB3_PROFILE_CIPHER_KEY = "intent-ui-web3-profile-cipher-v1";
const WEB3_PROFILE_PLAIN_KEY = "intent-ui-profile-v1";
const WEB3_WALLET_SIGN_MESSAGE = "IntentUI Web3 profile encryption key";

const toBase64 = (buffer) => {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  for (let i = 0; i < bytes.length; i += 1) binary += String.fromCharCode(bytes[i]);
  return btoa(binary);
};

const fromBase64 = (value) => {
  const binary = atob(String(value || ""));
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) out[i] = binary.charCodeAt(i);
  return out;
};

const toHex = (value) => Array.from(new Uint8Array(value))
  .map((byte) => byte.toString(16).padStart(2, "0"))
  .join("");

const resolveEthereumProvider = () =>
  typeof window !== "undefined" ? window.ethereum : undefined;

const signWithEthereumWallet = async (address) => {
  const provider = resolveEthereumProvider();
  if (!provider?.request) throw new Error("No EVM wallet provider detected.");
  try {
    return await provider.request({
      method: "personal_sign",
      params: [WEB3_WALLET_SIGN_MESSAGE, address]
    });
  } catch {
    const messageHex = `0x${toHex(new TextEncoder().encode(WEB3_WALLET_SIGN_MESSAGE))}`;
    return provider.request({
      method: "personal_sign",
      params: [messageHex, address]
    });
  }
};

const deriveAesKeyFromWallet = async (address) => {
  const signature = await signWithEthereumWallet(address);
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(`evm:${String(signature || "")}`));
  return crypto.subtle.importKey("raw", digest, { name: "AES-GCM" }, false, ["encrypt", "decrypt"]);
};

const encryptProfilePayload = async (address, payload) => {
  const key = await deriveAesKeyFromWallet(address);
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const data = new TextEncoder().encode(JSON.stringify(payload));
  const cipher = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, data);
  return {
    wallet: address,
    chain: "evm",
    algorithm: "AES-GCM",
    iv: toBase64(iv.buffer),
    cipher: toBase64(cipher),
    createdAt: new Date().toISOString()
  };
};

const decryptProfilePayload = async (address, payload) => {
  const key = await deriveAesKeyFromWallet(address);
  const iv = fromBase64(payload?.iv || "");
  const cipher = fromBase64(payload?.cipher || "");
  const plain = await crypto.subtle.decrypt({ name: "AES-GCM", iv }, key, cipher);
  const text = new TextDecoder().decode(plain);
  return JSON.parse(text);
};

const providerMeta = {
  github: {
    id: "github",
    name: "GitHub",
    description: "Repository access, PR operations, and code browsing.",
    authLabel: "GitHub Token",
    icon: FiGithub,
    tone: "text-neutral-100",
    tokenHint: "GitHub token",
    useToken: true
  },
  cloudflare: {
    id: "cloudflare",
    name: "Cloudflare",
    description: "Manage zones, workers, and pages.",
    authLabel: "API Token",
    icon: FiCloud,
    tone: "text-orange-300",
    tokenHint: "Cloudflare token",
    useToken: true
  },
  vercel: {
    id: "vercel",
    name: "Vercel",
    description: "Projects, deployments, and release visibility.",
    authLabel: "API Token",
    icon: FiBox,
    tone: "text-neutral-100",
    tokenHint: "Vercel token",
    useToken: true
  },
  google: {
    id: "google",
    name: "Google",
    description: "Drive, Gmail, Calendar, Contacts, and Messages.",
    authLabel: "OAuth Redirect",
    icon: FiGlobe,
    tone: "text-blue-300",
    tokenHint: "",
    useToken: false,
    oauthRedirect: true
  },
  google_photos: {
    id: "google_photos",
    name: "Google Photos",
    description: "Photos library connection via Google OAuth.",
    authLabel: "OAuth Redirect",
    icon: FiGlobe,
    tone: "text-sky-300",
    tokenHint: "",
    useToken: false,
    oauthRedirect: true
  },
  email: {
    id: "email",
    name: "Email",
    description: "Unified email provider for conversation threads.",
    authLabel: "OAuth Token",
    icon: FiGlobe,
    tone: "text-indigo-300",
    tokenHint: "Email provider token",
    useToken: true
  },
  whatsapp: {
    id: "whatsapp",
    name: "WhatsApp",
    description: "WhatsApp message provider for conversations.",
    authLabel: "OAuth Token",
    icon: FiGlobe,
    tone: "text-emerald-300",
    tokenHint: "WhatsApp token",
    useToken: true
  },
  messenger: {
    id: "messenger",
    name: "Messenger",
    description: "Messenger provider for conversation sync.",
    authLabel: "OAuth Token",
    icon: FiGlobe,
    tone: "text-blue-300",
    tokenHint: "Messenger token",
    useToken: true
  },
  telegram: {
    id: "telegram",
    name: "Telegram",
    description: "Telegram provider for unified inbox and threads.",
    authLabel: "OAuth Token",
    icon: FiGlobe,
    tone: "text-cyan-300",
    tokenHint: "Telegram token",
    useToken: true
  },
  qwen: {
    id: "qwen",
    name: "Qwen",
    description: "LLM provider for assistant and intent execution.",
    authLabel: "OAuth Token",
    icon: FiCpu,
    tone: "text-cyan-300",
    tokenHint: "Qwen token",
    useToken: true
  },
  codex_cli: {
    id: "codex_cli",
    name: "Codex CLI",
    description: "Local CLI executor for assistant tasks on connected devices.",
    authLabel: "Local Runtime",
    icon: FiCpu,
    tone: "text-emerald-300",
    tokenHint: "",
    useToken: false
  },
  tailscale: {
    id: "tailscale",
    name: "Tailscale",
    description: "Zero-config overlay network with Funnel and SSH access.",
    authLabel: "API Access Token",
    icon: FiShield,
    tone: "text-blue-300",
    tokenHint: "Tailscale API key (tskey-api-...)",
    useToken: true
  },
  hetzner: {
    id: "hetzner",
    name: "Hetzner",
    description: "Servers and firewall inventory.",
    authLabel: "API Token",
    icon: FiDatabase,
    tone: "text-emerald-300",
    tokenHint: "Hetzner token",
    useToken: true
  },
  web3: {
    id: "web3",
    name: "Web3",
    description: "EVM wallet encryption for profile backups and portable ciphertext exports.",
    authLabel: "Wallet Signature",
    icon: FiShield,
    tone: "text-fuchsia-300",
    tokenHint: "",
    useToken: false
  }
};

function IntegrationsPanel(props) {
  const compact = () => Boolean(props?.compact);
  const panelButtonClass = "inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]";
  const modalPrimaryButtonClass = "inline-flex h-7 items-center gap-1 rounded-md border border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.16)] px-2 text-[10px] font-medium text-[hsl(var(--primary))] transition-colors hover:bg-[hsl(var(--primary)/0.24)] disabled:cursor-not-allowed disabled:opacity-60";
  const modalNeutralButtonClass = "inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] font-medium text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]";
  const stateBlockClass = "rounded-md border border-neutral-800 bg-neutral-900/55 px-2.5 py-2 text-xs text-neutral-500";
  const [busyId, setBusyId] = createSignal("");
  const [status, setStatus] = createSignal("");
  const [search, setSearch] = createSignal("");
  const [dialogProviderId, setDialogProviderId] = createSignal("");
  const [tokenInput, setTokenInput] = createSignal("");
  const [accountLabelInput, setAccountLabelInput] = createSignal("");
  const [web3Wallet, setWeb3Wallet] = createSignal("");
  const [web3ProfileInput, setWeb3ProfileInput] = createSignal("{}");
  const [web3CipherInput, setWeb3CipherInput] = createSignal("");
  const [tailscaleApiKeyInput, setTailscaleApiKeyInput] = createSignal("");
  const [tailscaleAuthKeyInput, setTailscaleAuthKeyInput] = createSignal("");
  const [tailscaleTailnetInput, setTailscaleTailnetInput] = createSignal(
    typeof window === "undefined" ? "" : String(localStorage.getItem("tailscale_tailnet") || "").trim()
  );
  const [tailscaleConnectorTag, setTailscaleConnectorTag] = createSignal("tag:edgerun-app-connector");
  const [tailscaleDomains, setTailscaleDomains] = createSignal("os.edgerun.tech,*.users.edgerun.tech");
  const [tailscaleFunnelTarget, setTailscaleFunnelTarget] = createSignal("http://127.0.0.1:7777");
  const [tailscaleCopied, setTailscaleCopied] = createSignal("");
  const [tailscaleDevices, setTailscaleDevices] = createSignal([]);
  const [tailscaleSelectedDeviceId, setTailscaleSelectedDeviceId] = createSignal("");
  const [tailscaleRoutesInput, setTailscaleRoutesInput] = createSignal("");
  const [tailscaleRoutesBusy, setTailscaleRoutesBusy] = createSignal(false);
  const [tailscaleRoutesStatus, setTailscaleRoutesStatus] = createSignal("");
  const [tailscaleRoutesError, setTailscaleRoutesError] = createSignal("");
  const assistantProvider = createMemo(() => workflowUi().provider || "codex");

  onMount(() => {
    integrationStore.checkAll();
    if (typeof window !== "undefined") {
      const parsed = new URL(window.location.href);
      const oauthState = parsed.searchParams.get("google_oauth");
      if (oauthState) {
        setStatus(oauthState === "access_denied" ? "Google connection cancelled." : `Google OAuth error: ${oauthState}`);
        parsed.searchParams.delete("google_oauth");
        window.history.replaceState({}, "", `${parsed.pathname}${parsed.search}`);
      } else if (localStorage.getItem("google_token")) {
        setStatus("Google integration connected.");
      }
    }
    if (props.preselectProviderId) {
      setDialogProviderId(props.preselectProviderId);
    }
  });

  createEffect(() => {
    if (typeof window === "undefined") return;
    const runtime = profileRuntime();
    const value = tailscaleApiKeyInput().trim();
    if (runtime.mode === "profile" && runtime.profileLoaded) {
      void setProfileSecret("tailscale_api_key", value);
      localStorage.removeItem("tailscale_api_key");
      return;
    }
    localStorage.setItem("tailscale_api_key", value);
  });

  createEffect(() => {
    if (typeof window === "undefined") return;
    const runtime = profileRuntime();
    const value = tailscaleAuthKeyInput().trim();
    if (runtime.mode === "profile" && runtime.profileLoaded) {
      void setProfileSecret("tailscale_auth_key", value);
      localStorage.removeItem("tailscale_auth_key");
      return;
    }
    localStorage.setItem("tailscale_auth_key", value);
  });

  createEffect(() => {
    if (typeof window === "undefined") return;
    localStorage.setItem("tailscale_tailnet", tailscaleTailnetInput().trim());
  });

  const providers = createMemo(() => integrationStore.list().map((integration) => ({
    ...integration,
    ...providerMeta[integration.id]
  })));

  const filteredProviders = createMemo(() => {
    const q = search().trim().toLowerCase();
    if (!q) return providers();
    return providers().filter((provider) =>
      provider.name.toLowerCase().includes(q) || provider.id.toLowerCase().includes(q)
    );
  });

  const activeProvider = createMemo(() => providers().find((provider) => provider.id === dialogProviderId()) || null);

  const openProviderDialog = (provider) => {
    if (!provider) return;
    setDialogProviderId(provider.id);
    setTokenInput("");
    setAccountLabelInput(provider.accountLabel || `${provider.name} Session`);
    if (provider.id === "tailscale" && typeof window !== "undefined") {
      const runtime = profileRuntime();
      const apiKey = runtime.mode === "profile" && runtime.profileLoaded
        ? String(getProfileSecret("tailscale_api_key") || "").trim()
        : String(localStorage.getItem("tailscale_api_key") || "").trim();
      const authKey = runtime.mode === "profile" && runtime.profileLoaded
        ? String(getProfileSecret("tailscale_auth_key") || "").trim()
        : String(localStorage.getItem("tailscale_auth_key") || "").trim();
      const tailnet = String(localStorage.getItem("tailscale_tailnet") || "").trim();
      setTailscaleApiKeyInput(apiKey);
      setTailscaleAuthKeyInput(authKey);
      setTailscaleTailnetInput(tailnet);
      setTokenInput(apiKey);
      setTailscaleDevices([]);
      setTailscaleSelectedDeviceId("");
      setTailscaleRoutesInput("");
      setTailscaleRoutesStatus("");
      setTailscaleRoutesError("");
    }
    if (provider.id === "web3" && typeof window !== "undefined") {
      const storedToken = String(localStorage.getItem("web3_wallet") || "").trim();
      const wallet = String(storedToken.includes(":") ? storedToken.split(":", 2)[1] : storedToken).trim();
      setWeb3Wallet(wallet);
      setWeb3ProfileInput(localStorage.getItem(WEB3_PROFILE_PLAIN_KEY) || "{}");
      setWeb3CipherInput(localStorage.getItem(WEB3_PROFILE_CIPHER_KEY) || "");
    }
  };

  const startProviderAuth = (provider) => {
    if (!provider) return;
    if ((provider.id === "google" || provider.id === "google_photos") && typeof window !== "undefined") {
      const returnTo = `${window.location.pathname}${window.location.search || ""}`;
      window.location.assign(`/api/google/oauth/start?returnTo=${encodeURIComponent(returnTo)}`);
    }
  };

  const closeDialog = () => {
    setDialogProviderId("");
    setTokenInput("");
    setAccountLabelInput("");
    setWeb3Wallet("");
    setWeb3ProfileInput("{}");
    setWeb3CipherInput("");
    setTailscaleConnectorTag("tag:edgerun-app-connector");
    setTailscaleDomains("os.edgerun.tech,*.users.edgerun.tech");
    setTailscaleFunnelTarget("http://127.0.0.1:7777");
    setTailscaleDevices([]);
    setTailscaleSelectedDeviceId("");
    setTailscaleRoutesInput("");
    setTailscaleRoutesStatus("");
    setTailscaleRoutesError("");
    setTailscaleCopied("");
  };

  const tailscaleDomainsList = createMemo(() => tailscaleDomains()
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean));

  const tailscaleJoinCommand = createMemo(() => {
    const key = tailscaleAuthKeyInput().trim() || "<TS_AUTH_KEY>";
    const tag = tailscaleConnectorTag().trim() || "tag:edgerun-app-connector";
    return `sudo tailscale up --auth-key "${key}" --ssh --advertise-tags=${tag} --advertise-connector`;
  });

  const tailscaleFunnelCommand = createMemo(
    () => `sudo tailscale funnel --bg ${tailscaleFunnelTarget().trim() || "http://127.0.0.1:7777"}`
  );

  const tailscalePolicySnippet = createMemo(() => {
    const tag = tailscaleConnectorTag().trim() || "tag:edgerun-app-connector";
    const domains = tailscaleDomainsList().length > 0
      ? tailscaleDomainsList()
      : ["os.edgerun.tech", "*.users.edgerun.tech"];
    const domainLines = domains.map((domain) => `            "${domain}"`).join(",\n");
    return [
      "{",
      "  // Add this to your tailnet policy file (Access controls)",
      "  \"tagOwners\": {",
      `    \"${tag}\": [\"autogroup:admin\"]`,
      "  },",
      "  \"nodeAttrs\": [",
      "    {",
      `      \"target\": [\"${tag}\"],`,
      "      \"app\": {",
      "        \"tailscale.com/app-connectors\": [",
      "          {",
      "            \"name\": \"edgerun-control-plane\",",
      `            \"connectors\": [\"${tag}\"],`,
      "            \"domains\": [",
      domainLines,
      "            ]",
      "          }",
      "        ]",
      "      }",
      "    }",
      "  ]",
      "}"
    ].join("\n");
  });

  const tailscaleSelectedDevice = createMemo(
    () => tailscaleDevices().find((device) => String(device?.id || "") === tailscaleSelectedDeviceId()) || null
  );

  const parseRoutesInput = (value) => String(value || "")
    .split(/[,\n]/)
    .map((entry) => entry.trim())
    .filter(Boolean);

  const loadTailscaleDevices = async () => {
    const apiKey = tailscaleApiKeyInput().trim();
    const tailnet = tailscaleTailnetInput().trim();
    if (!apiKey || !tailnet) {
      setTailscaleRoutesError("API key and tailnet are required.");
      setTailscaleRoutesStatus("");
      return;
    }
    setTailscaleRoutesBusy(true);
    setTailscaleRoutesError("");
    setTailscaleRoutesStatus("");
    try {
      const response = await fetch("/api/tailscale/devices", {
        method: "POST",
        headers: { "content-type": "application/json; charset=utf-8" },
        body: JSON.stringify({ apiKey, tailnet })
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok || !body?.ok) {
        throw new Error(String(body?.error || `tailscale devices request failed (${response.status})`));
      }
      const devices = Array.isArray(body?.devices) ? body.devices : [];
      setTailscaleDevices(devices);
      setTailscaleSelectedDeviceId(devices[0]?.id || "");
      setTailscaleRoutesStatus(`${devices.length} device${devices.length === 1 ? "" : "s"} loaded.`);
      const runtime = profileRuntime();
      if (runtime.mode === "profile" && runtime.profileLoaded) {
        void setProfileSecret("tailscale_api_key", apiKey);
      } else {
        localStorage.setItem("tailscale_api_key", apiKey);
      }
      localStorage.setItem("tailscale_tailnet", tailnet);
      setTokenInput(apiKey);
    } catch (error) {
      setTailscaleRoutesError(error instanceof Error ? error.message : "Failed to load Tailscale devices.");
    } finally {
      setTailscaleRoutesBusy(false);
    }
  };

  const applyTailscaleRoutes = async () => {
    const apiKey = tailscaleApiKeyInput().trim();
    const deviceId = tailscaleSelectedDeviceId().trim();
    const routes = parseRoutesInput(tailscaleRoutesInput());
    if (!apiKey || !deviceId) {
      setTailscaleRoutesError("API key and device selection are required.");
      setTailscaleRoutesStatus("");
      return;
    }
    setTailscaleRoutesBusy(true);
    setTailscaleRoutesError("");
    setTailscaleRoutesStatus("");
    try {
      const response = await fetch("/api/tailscale/device-routes", {
        method: "POST",
        headers: { "content-type": "application/json; charset=utf-8" },
        body: JSON.stringify({ apiKey, deviceId, routes })
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok || !body?.ok) {
        throw new Error(String(body?.error || `tailscale routes request failed (${response.status})`));
      }
      setTailscaleDevices((prev) => prev.map((item) => {
        if (String(item?.id || "") !== deviceId) return item;
        return {
          ...item,
          advertisedRoutes: Array.isArray(body?.advertisedRoutes) ? body.advertisedRoutes : [],
          enabledRoutes: Array.isArray(body?.enabledRoutes) ? body.enabledRoutes : []
        };
      }));
      setTailscaleRoutesStatus("Routes updated.");
      const runtime = profileRuntime();
      if (runtime.mode === "profile" && runtime.profileLoaded) {
        void setProfileSecret("tailscale_api_key", apiKey);
      } else {
        localStorage.setItem("tailscale_api_key", apiKey);
      }
      setTokenInput(apiKey);
    } catch (error) {
      setTailscaleRoutesError(error instanceof Error ? error.message : "Failed to apply routes.");
    } finally {
      setTailscaleRoutesBusy(false);
    }
  };

  const copyText = async (id, text) => {
    try {
      await navigator.clipboard.writeText(text);
      setTailscaleCopied(id);
      window.setTimeout(() => {
        setTailscaleCopied((current) => (current === id ? "" : current));
      }, 1200);
    } catch {
      setStatus("Clipboard write failed.");
    }
  };

  const connectWeb3Wallet = async (provider) => {
    try {
      const walletProvider = resolveEthereumProvider();
      if (!walletProvider?.request) throw new Error("No injected EVM wallet found. Install MetaMask/Rabby.");
      const accounts = await walletProvider.request({ method: "eth_requestAccounts" });
      const wallet = String(Array.isArray(accounts) ? accounts[0] : "").trim();
      if (!wallet) throw new Error("Wallet did not return an address.");
      integrationStore.connect(provider.id, {
        accountLabel: accountLabelInput().trim() || "EVM Wallet",
        token: `evm:${wallet}`
      });
      setWeb3Wallet(wallet);
      setStatus(`Web3 evm connected: ${wallet.slice(0, 6)}...${wallet.slice(-4)}`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to connect Web3 wallet.");
    }
  };

  const encryptWeb3Profile = async () => {
    const wallet = web3Wallet().trim();
    if (!wallet) {
      setStatus("Connect wallet first.");
      return;
    }
    try {
      const payload = JSON.parse(web3ProfileInput() || "{}");
      const encrypted = await encryptProfilePayload(wallet, payload);
      const serialized = JSON.stringify(encrypted);
      localStorage.setItem(WEB3_PROFILE_CIPHER_KEY, serialized);
      localStorage.setItem(WEB3_PROFILE_PLAIN_KEY, JSON.stringify(payload));
      setWeb3CipherInput(serialized);
      setStatus("Profile encrypted with evm wallet and saved locally.");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to encrypt profile.");
    }
  };

  const decryptWeb3Profile = async () => {
    const wallet = web3Wallet().trim();
    const raw = web3CipherInput().trim() || localStorage.getItem(WEB3_PROFILE_CIPHER_KEY) || "";
    if (!wallet || !raw) {
      setStatus("Connect wallet and provide ciphertext first.");
      return;
    }
    try {
      const payload = JSON.parse(raw);
      const decrypted = await decryptProfilePayload(wallet, payload);
      const pretty = JSON.stringify(decrypted, null, 2);
      setWeb3ProfileInput(pretty);
      localStorage.setItem(WEB3_PROFILE_PLAIN_KEY, pretty);
      setStatus("Profile decrypted.");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to decrypt profile.");
    }
  };

  const saveProvider = async (provider) => {
    if (!provider) return;
    if (provider.oauthRedirect) {
      integrationStore.setConnectorMode(provider.id, "user_owned");
      startProviderAuth(provider);
      return;
    }
    setBusyId(provider.id);

    const token = provider.id === "tailscale"
      ? tailscaleApiKeyInput().trim() || tokenInput().trim()
      : tokenInput().trim();
    if (!token) {
      setStatus(`Enter a ${provider.name} token first.`);
      setBusyId("");
      return;
    }

    integrationStore.connect(provider.id, {
      connectorMode: "user_owned",
      accountLabel: accountLabelInput().trim() || `${provider.name} Session`,
      token
    });
    if (provider.id === "tailscale" && typeof window !== "undefined") {
      const runtime = profileRuntime();
      if (runtime.mode === "profile" && runtime.profileLoaded) {
        void setProfileSecret("tailscale_api_key", token);
        void setProfileSecret("tailscale_auth_key", tailscaleAuthKeyInput().trim());
        localStorage.removeItem("tailscale_api_key");
        localStorage.removeItem("tailscale_auth_key");
      } else {
        localStorage.setItem("tailscale_api_key", token);
        localStorage.setItem("tailscale_auth_key", tailscaleAuthKeyInput().trim());
      }
      localStorage.setItem("tailscale_tailnet", tailscaleTailnetInput().trim());
    }
    setStatus(`${provider.name} integration saved.`);
    setBusyId("");
    closeDialog();
  };

  const disconnectProvider = (provider) => {
    if (!provider) return;
    integrationStore.disconnect(provider.id);
    setStatus(`${provider.name} disconnected.`);
    closeDialog();
  };

  const setProviderOwnership = (provider, mode) => {
    if (!provider) return;
    integrationStore.setConnectorMode(provider.id, mode);
    setStatus(
      mode === "platform"
        ? `${provider.name} now uses platform connector.`
        : `${provider.name} switched to user-owned connector.`
    );
  };

  return (
    <div class={`h-full overflow-auto text-neutral-200 ${compact() ? "" : "bg-[#0f1013] p-4"}`}>
      <div class="border-b border-neutral-800 px-3 py-2">
        <h3 class="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-neutral-300">
          <FiLink2 size={18} />
          <span>Integrations</span>
        </h3>
        <p class="mt-1 text-xs text-neutral-400">Use Link to add or update details for each provider.</p>
        <div class="mt-2 grid grid-cols-2 gap-1 rounded-md border border-neutral-800 bg-neutral-900/60 p-1">
          <button
            type="button"
            class={`rounded px-2 py-1 text-[11px] transition-colors ${assistantProvider() === "codex" ? "font-semibold text-[hsl(var(--primary))]" : "text-neutral-400 hover:bg-neutral-800"}`}
            onClick={() => setAssistantProvider("codex")}
          >
            Codex active
          </button>
          <button
            type="button"
            class={`rounded px-2 py-1 text-[11px] transition-colors ${assistantProvider() === "qwen" ? "font-semibold text-[hsl(var(--primary))]" : "text-neutral-400 hover:bg-neutral-800"}`}
            onClick={() => setAssistantProvider("qwen")}
          >
            Qwen active
          </button>
        </div>
      </div>

      <div class="relative mb-3 px-3 pt-3">
        <FiSearch size={14} class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-neutral-500" />
        <input
          id="integration-search"
          type="text"
          value={search()}
          onInput={(event) => setSearch(event.currentTarget.value)}
          placeholder="Search providers..."
          class="w-full rounded-md border border-neutral-700 bg-[#0b0c0f] py-2 pl-8 pr-2 text-xs text-neutral-100 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
        />
      </div>

      <Show when={status()}>
        <div class={`mx-3 mb-3 ${stateBlockClass} text-neutral-300`}>
          {status()}
        </div>
      </Show>

      <div class="space-y-1.5 px-3 pb-3">
        <For each={filteredProviders()}>
          {(provider) => {
            const Icon = provider.icon || FiCloud;
            return (
              <article class="rounded-md border border-neutral-800 bg-neutral-900/45 px-2.5 py-2.5">
                <div class="flex items-start justify-between gap-3">
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                      <Icon size={16} class={provider.connected ? "text-[hsl(var(--primary))]" : "text-neutral-400"} />
                      <p class="truncate text-sm font-medium text-neutral-100">{provider.name}</p>
                      <span
                        class={`inline-flex items-center gap-1 text-[11px] ${provider.connected ? "text-[hsl(var(--primary))]" : "text-neutral-500"}`}
                        data-testid={`provider-connected-${provider.id}`}
                      >
                        <Show when={provider.connected} fallback={<FiLink2 size={10} />}>
                          <FiCheckCircle size={10} />
                        </Show>
                        {provider.connected ? "Connected" : "Not connected"}
                      </span>
                      <span
                        class={`inline-flex items-center gap-1 text-[11px] ${provider.available ? "text-emerald-300" : "text-amber-300"}`}
                        data-testid={`provider-available-${provider.id}`}
                      >
                        <Show when={provider.available} fallback={<FiXCircle size={10} />}>
                          <FiCheckCircle size={10} />
                        </Show>
                        {provider.available ? "Available" : "Unavailable"}
                      </span>
                      <span
                        class={`inline-flex items-center rounded px-1.5 py-0.5 text-[10px] ${
                          provider.connectorMode === "platform"
                            ? "border border-cyan-500/40 bg-cyan-500/10 text-cyan-200"
                            : "border border-amber-500/40 bg-amber-500/10 text-amber-200"
                        }`}
                        data-testid={`provider-mode-${provider.id}`}
                      >
                        {provider.connectorMode === "platform" ? "Platform" : "User-owned"}
                      </span>
                    </div>
                    <p class="mt-1 truncate text-xs text-neutral-500">{provider.description}</p>
                    <p class="mt-0.5 truncate text-[11px] text-neutral-500">{provider.availabilityReason}</p>
                  </div>

                  <button
                    type="button"
                    onClick={() => openProviderDialog(provider)}
                    class="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-neutral-700 bg-neutral-900 text-neutral-300 transition hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
                    title={provider.connected ? "Settings" : "Link"}
                    data-testid={`provider-open-${provider.id}`}
                  >
                    <Show when={provider.connected} fallback={<FiLink2 size={14} />}>
                      <FiSettings size={14} />
                    </Show>
                  </button>
                </div>
              </article>
            );
          }}
        </For>
      </div>

      <Show when={filteredProviders().length === 0}>
        <p class={`mt-3 ${stateBlockClass}`}>No providers match your search.</p>
      </Show>

      <Show when={activeProvider()}>
        {(providerAccessor) => {
          const provider = providerAccessor();
          const isConnected = () => Boolean(provider.connected);
          const ProviderIcon = provider.icon || FiCloud;
          return (
            <div class="fixed inset-0 z-[10040] flex items-center justify-center bg-black/50 px-4">
              <div class="w-full max-w-md rounded-xl border border-neutral-700 bg-[#101216] p-4 shadow-2xl" data-testid={`provider-dialog-${provider.id}`}>
                <div class="mb-3 flex items-start justify-between gap-2">
                  <div>
                    <h4 class="flex items-center gap-2 text-sm font-semibold text-white">
                      <ProviderIcon size={16} class={isConnected() ? "text-[hsl(var(--primary))]" : "text-neutral-300"} />
                      <span>{provider.name} Integration</span>
                    </h4>
                    <p class="mt-0.5 inline-flex items-center gap-1 text-xs text-neutral-500">
                      <FiKey size={11} />
                      {provider.authLabel}
                    </p>
                  </div>
                  <button
                    type="button"
                    class={panelButtonClass}
                    onClick={closeDialog}
                  >
                    <FiXCircle size={12} />
                    Close
                  </button>
                </div>

                <Show when={provider.supportsPlatformConnector}>
                  <div class="mb-3 rounded-md border border-neutral-800 bg-neutral-900/50 p-2">
                    <p class="mb-1 text-[11px] uppercase tracking-wide text-neutral-400">Connector ownership</p>
                    <div class="grid grid-cols-2 gap-1 rounded-md border border-neutral-800 bg-neutral-900/70 p-1">
                      <button
                        type="button"
                        class={`rounded px-2 py-1 text-[11px] ${
                          provider.connectorMode === "platform"
                            ? "bg-[hsl(var(--primary)/0.2)] text-[hsl(var(--primary))]"
                            : "text-neutral-300 hover:bg-neutral-800"
                        }`}
                        onClick={() => setProviderOwnership(provider, "platform")}
                        data-testid={`provider-ownership-platform-${provider.id}`}
                      >
                        Platform connector
                      </button>
                      <button
                        type="button"
                        class={`rounded px-2 py-1 text-[11px] ${
                          provider.connectorMode === "user_owned"
                            ? "bg-[hsl(var(--primary)/0.2)] text-[hsl(var(--primary))]"
                            : "text-neutral-300 hover:bg-neutral-800"
                        }`}
                        onClick={() => setProviderOwnership(provider, "user_owned")}
                        data-testid={`provider-ownership-user-${provider.id}`}
                      >
                        Use my own
                      </button>
                    </div>
                  </div>
                </Show>

                <Show when={provider.useToken && provider.connectorMode === "user_owned"}>
                  <input
                    id="provider-label"
                    type="text"
                    value={accountLabelInput()}
                    onInput={(event) => setAccountLabelInput(event.currentTarget.value)}
                    placeholder="Account label"
                    class="mb-3 w-full rounded-md border border-neutral-700 bg-[#0b0c0f] px-2 py-2 text-sm text-neutral-100 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                  />
                </Show>

                <Show when={provider.useToken && provider.connectorMode === "user_owned"}>
                  <div class="relative mb-3">
                    <FiKey size={14} class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-neutral-500" />
                    <input
                      id="provider-token"
                      type="password"
                      value={tokenInput()}
                      onInput={(event) => setTokenInput(event.currentTarget.value)}
                      placeholder={provider.tokenHint || "Access token"}
                      class="w-full rounded-md border border-neutral-700 bg-[#0b0c0f] py-2 pl-8 pr-2 text-sm text-neutral-100 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                    />
                  </div>
                </Show>
                <Show when={provider.id === "web3"}>
                  <div class="mb-3 space-y-2 rounded-md border border-neutral-800 bg-neutral-900/50 p-2">
                    <div class="flex items-center justify-between gap-2">
                      <p class="text-xs text-neutral-300">
                        evm wallet: <span class="font-mono text-neutral-100">{web3Wallet() || "Not connected"}</span>
                      </p>
                      <button
                        type="button"
                        class={panelButtonClass}
                        onClick={() => connectWeb3Wallet(provider)}
                      >
                        Connect Wallet
                      </button>
                    </div>
                    <textarea
                      value={web3ProfileInput()}
                      onInput={(event) => setWeb3ProfileInput(event.currentTarget.value)}
                      placeholder='{"profile":"data"}'
                      class="h-24 w-full resize-none rounded-md border border-neutral-700 bg-[#0b0c0f] px-2 py-1.5 text-xs text-neutral-100 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                    />
                    <div class="flex flex-wrap gap-2">
                      <button
                        type="button"
                        class={panelButtonClass}
                        onClick={encryptWeb3Profile}
                      >
                        Encrypt + Save Local
                      </button>
                      <button
                        type="button"
                        class={panelButtonClass}
                        onClick={decryptWeb3Profile}
                      >
                        Decrypt
                      </button>
                      <button
                        type="button"
                        class={panelButtonClass}
                        onClick={() => {
                          navigator.clipboard?.writeText(web3CipherInput() || "");
                          setStatus("Ciphertext copied. Store it in Drive or anywhere safe.");
                        }}
                      >
                        Copy Ciphertext
                      </button>
                      <button
                        type="button"
                        class={panelButtonClass}
                        onClick={() => openWindow("credentials")}
                      >
                        Store Keys in TPM (hwvault)
                      </button>
                    </div>
                    <textarea
                      value={web3CipherInput()}
                      onInput={(event) => setWeb3CipherInput(event.currentTarget.value)}
                      placeholder="Encrypted ciphertext JSON"
                      class="h-20 w-full resize-none rounded-md border border-neutral-700 bg-[#0b0c0f] px-2 py-1.5 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                    />
                  </div>
                </Show>
                <Show when={provider.id === "tailscale"}>
                  <div class="mb-3 space-y-2 rounded-md border border-neutral-800 bg-neutral-900/50 p-2" data-testid="provider-tailscale-quickstart">
                    <p class="text-[11px] uppercase tracking-wide text-neutral-400">Quick start</p>
                    <p class="text-[11px] text-neutral-400">
                      Run on node-manager host to join your tailnet, advertise as an App Connector, and expose browser entry with Funnel.
                    </p>
                    <label class="block text-[10px] text-neutral-500">
                      Tailscale API key
                      <input
                        type="password"
                        value={tailscaleApiKeyInput()}
                        onInput={(event) => {
                          const value = event.currentTarget.value;
                          setTailscaleApiKeyInput(value);
                          setTokenInput(value);
                        }}
                        placeholder="tskey-api-..."
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-[#0b0c0f] px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                        data-testid="tailscale-api-key-input"
                      />
                    </label>
                    <label class="block text-[10px] text-neutral-500">
                      Tailnet
                      <input
                        type="text"
                        value={tailscaleTailnetInput()}
                        onInput={(event) => setTailscaleTailnetInput(event.currentTarget.value)}
                        placeholder="example.github"
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-[#0b0c0f] px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                        data-testid="tailscale-tailnet-input"
                      />
                    </label>
                    <label class="block text-[10px] text-neutral-500">
                      Node auth key (for tailscale up)
                      <input
                        type="password"
                        value={tailscaleAuthKeyInput()}
                        onInput={(event) => setTailscaleAuthKeyInput(event.currentTarget.value)}
                        placeholder="tskey-auth-..."
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-[#0b0c0f] px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                        data-testid="tailscale-auth-key-input"
                      />
                    </label>
                    <label class="block text-[10px] text-neutral-500">
                      Connector tag
                      <input
                        type="text"
                        value={tailscaleConnectorTag()}
                        onInput={(event) => setTailscaleConnectorTag(event.currentTarget.value)}
                        placeholder="tag:edgerun-app-connector"
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-[#0b0c0f] px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                        data-testid="tailscale-connector-tag-input"
                      />
                    </label>
                    <label class="block text-[10px] text-neutral-500">
                      App domains (comma separated)
                      <input
                        type="text"
                        value={tailscaleDomains()}
                        onInput={(event) => setTailscaleDomains(event.currentTarget.value)}
                        placeholder="os.edgerun.tech,*.users.edgerun.tech"
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-[#0b0c0f] px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                        data-testid="tailscale-app-domains-input"
                      />
                    </label>
                    <label class="block text-[10px] text-neutral-500">
                      Funnel target
                      <input
                        type="text"
                        value={tailscaleFunnelTarget()}
                        onInput={(event) => setTailscaleFunnelTarget(event.currentTarget.value)}
                        placeholder="http://127.0.0.1:7777"
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-[#0b0c0f] px-2 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                        data-testid="tailscale-funnel-target-input"
                      />
                    </label>
                    <div class="space-y-1">
                      <p class="text-[10px] text-neutral-500">1) Join tailnet and advertise app connector</p>
                      <pre class="overflow-x-auto rounded border border-neutral-800 bg-[#0b0c0f] p-2 text-[10px] text-neutral-200" data-testid="tailscale-up-command">
{tailscaleJoinCommand()}
                      </pre>
                      <button
                        type="button"
                        class={panelButtonClass}
                        onClick={() => copyText("tailscale-up", tailscaleJoinCommand())}
                        data-testid="tailscale-copy-up-command"
                      >
                        <FiCopy size={12} />
                        {tailscaleCopied() === "tailscale-up" ? "Copied" : "Copy join command"}
                      </button>
                    </div>
                    <div class="space-y-1">
                      <p class="text-[10px] text-neutral-500">2) Tailnet policy for app connector</p>
                      <pre class="max-h-40 overflow-auto rounded border border-neutral-800 bg-[#0b0c0f] p-2 text-[10px] text-neutral-200" data-testid="tailscale-policy-snippet">
{tailscalePolicySnippet()}
                      </pre>
                      <button
                        type="button"
                        class={panelButtonClass}
                        onClick={() => copyText("tailscale-policy", tailscalePolicySnippet())}
                        data-testid="tailscale-copy-policy-snippet"
                      >
                        <FiCopy size={12} />
                        {tailscaleCopied() === "tailscale-policy" ? "Copied" : "Copy policy snippet"}
                      </button>
                    </div>
                    <div class="space-y-1">
                      <p class="text-[10px] text-neutral-500">3) Enable Funnel</p>
                      <pre class="overflow-x-auto rounded border border-neutral-800 bg-[#0b0c0f] p-2 text-[10px] text-neutral-200" data-testid="tailscale-funnel-command">
{tailscaleFunnelCommand()}
                      </pre>
                      <button
                        type="button"
                        class={panelButtonClass}
                        onClick={() => copyText("tailscale-funnel", tailscaleFunnelCommand())}
                        data-testid="tailscale-copy-funnel-command"
                      >
                        <FiCopy size={12} />
                        {tailscaleCopied() === "tailscale-funnel" ? "Copied" : "Copy funnel command"}
                      </button>
                    </div>
                    <div class="mt-2 rounded-md border border-neutral-800 bg-neutral-900/55 p-2">
                      <p class="text-[10px] uppercase tracking-wide text-neutral-400">API routing controls</p>
                      <p class="mt-1 text-[10px] text-neutral-500">Load devices from tailnet and apply enabled routes to the selected device.</p>
                      <div class="mt-2 flex flex-wrap items-center gap-2">
                        <button
                          type="button"
                          class={panelButtonClass}
                          disabled={tailscaleRoutesBusy()}
                          onClick={loadTailscaleDevices}
                          data-testid="tailscale-load-devices"
                        >
                          {tailscaleRoutesBusy() ? "Loading..." : "Load devices"}
                        </button>
                      </div>
                      <Show when={tailscaleRoutesError()}>
                        <p class="mt-2 text-[10px] text-red-300" data-testid="tailscale-routes-error">{tailscaleRoutesError()}</p>
                      </Show>
                      <Show when={tailscaleRoutesStatus()}>
                        <p class="mt-2 text-[10px] text-[hsl(var(--primary))]" data-testid="tailscale-routes-status">{tailscaleRoutesStatus()}</p>
                      </Show>
                      <Show when={tailscaleDevices().length > 0}>
                        <label class="mt-2 block text-[10px] text-neutral-500">
                          Device
                          <select
                            value={tailscaleSelectedDeviceId()}
                            onInput={(event) => setTailscaleSelectedDeviceId(event.currentTarget.value)}
                            class="mt-1 h-8 w-full rounded border border-neutral-700 bg-[#0b0c0f] px-2 text-[11px] text-neutral-200 focus:border-neutral-500 focus:outline-none"
                            data-testid="tailscale-device-select"
                          >
                            <For each={tailscaleDevices()}>
                              {(device) => (
                                <option value={String(device?.id || "")}>
                                  {String(device?.hostname || device?.name || device?.id || "unknown")}
                                </option>
                              )}
                            </For>
                          </select>
                        </label>
                        <Show when={tailscaleSelectedDevice()}>
                          {(deviceAccessor) => (
                            <div class="mt-2 rounded border border-neutral-800 bg-[#0b0c0f] p-2 text-[10px] text-neutral-300" data-testid="tailscale-selected-device-routes">
                              <p><span class="text-neutral-500">Advertised:</span> {(deviceAccessor().advertisedRoutes || []).join(", ") || "none"}</p>
                              <p><span class="text-neutral-500">Enabled:</span> {(deviceAccessor().enabledRoutes || []).join(", ") || "none"}</p>
                            </div>
                          )}
                        </Show>
                        <label class="mt-2 block text-[10px] text-neutral-500">
                          Enabled routes (comma or newline separated CIDRs)
                          <textarea
                            value={tailscaleRoutesInput()}
                            onInput={(event) => setTailscaleRoutesInput(event.currentTarget.value)}
                            placeholder="10.0.0.0/16, 192.168.1.0/24"
                            class="mt-1 h-16 w-full resize-y rounded border border-neutral-700 bg-[#0b0c0f] px-2 py-1.5 text-[11px] text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
                            data-testid="tailscale-routes-input"
                          />
                        </label>
                        <div class="mt-2 flex flex-wrap items-center gap-2">
                          <button
                            type="button"
                            class={panelButtonClass}
                            disabled={tailscaleRoutesBusy()}
                            onClick={applyTailscaleRoutes}
                            data-testid="tailscale-apply-routes"
                          >
                            {tailscaleRoutesBusy() ? "Applying..." : "Apply routes"}
                          </button>
                        </div>
                      </Show>
                    </div>
                  </div>
                </Show>

                <div class="mt-2 flex flex-wrap gap-2">
                  <Show
                    when={provider.oauthRedirect && provider.connectorMode === "user_owned"}
                    fallback={
                      <button
                        type="button"
                        disabled={busyId() === provider.id}
                        class={modalPrimaryButtonClass}
                        onClick={() => {
                          if (provider.id === "web3") {
                            connectWeb3Wallet(provider);
                            return;
                          }
                          if (provider.connectorMode === "platform" && provider.supportsPlatformConnector) {
                            integrationStore.connect(provider.id, {
                              connectorMode: "platform",
                              accountLabel: "Platform Connector"
                            });
                            setStatus(`${provider.name} platform connector linked.`);
                            closeDialog();
                            return;
                          }
                          saveProvider(provider);
                        }}
                        data-testid={`provider-save-${provider.id}`}
                      >
                        <Show when={isConnected()} fallback={<FiLink2 size={12} />}>
                          <FiCheckCircle size={12} />
                        </Show>
                        {busyId() === provider.id ? "Saving..." : isConnected() ? "Update Integration" : "Link Integration"}
                      </button>
                    }
                  >
                    <button
                      type="button"
                      class={modalPrimaryButtonClass}
                      onClick={() => startProviderAuth(provider)}
                      data-testid={`provider-oauth-connect-${provider.id}`}
                    >
                      <FiLink2 size={12} />
                      {isConnected() ? "Reconnect" : "Connect"}
                    </button>
                  </Show>

                  <Show when={isConnected()}>
                    <button
                      type="button"
                      class={modalNeutralButtonClass}
                      onClick={() => disconnectProvider(provider)}
                    >
                      <FiXCircle size={12} />
                      Disconnect
                    </button>
                  </Show>
                </div>
              </div>
            </div>
          );
        }}
      </Show>
    </div>
  );
}

export { IntegrationsPanel as default };
