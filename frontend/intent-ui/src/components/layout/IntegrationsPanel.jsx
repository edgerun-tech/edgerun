import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { Portal } from "solid-js/web";
import {
  FiLink2,
  FiCheckCircle,
  FiCloud,
  FiDatabase,
  FiCpu,
  FiSearch,
  FiXCircle,
  FiArrowRight,
  FiArrowLeft,
  FiZap,
  FiLock,
  FiHash
} from "solid-icons/fi";
import {
  SiGithub,
  SiGoogle,
  SiCloudflare,
  SiVercel,
  SiTelegram,
  SiWhatsapp,
  SiMessenger,
  SiTailscale,
  SiWeb3dotjs,
  SiDiscord,
  SiSignal,
  SiGooglechat,
  SiBluesky,
  SiX,
  SiImessage,
  SiApple
} from "solid-icons/si";
import { integrationStore, integrationVerification } from "../../stores/integrations";
import { openWorkflowFlipper, setAssistantProvider, workflowUi } from "../../stores/workflow-ui";
import { canonicalBridgeId } from "../../lib/integrations/official-bridges";

const FLIPPER_SERIAL_SERVICE_UUID = "8fe5b3d5-2e7f-4a98-2a48-7acc60fe0000";
const DALY_NUS_SERVICE_UUID = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
const DALY_OPTIONAL_SERVICE_UUIDS = [
  DALY_NUS_SERVICE_UUID,
  "0000fff0-0000-1000-8000-00805f9b34fb",
  "0000ffe0-0000-1000-8000-00805f9b34fb",
  "battery_service",
  "device_information"
];

const providerMeta = {
  github: { icon: SiGithub, tone: "text-neutral-100", tokenHint: "GitHub Personal Access Token", category: "development" },
  cloudflare: { icon: SiCloudflare, tone: "text-orange-300", tokenHint: "Cloudflare account API token", category: "cloud" },
  vercel: { icon: SiVercel, tone: "text-neutral-100", tokenHint: "Vercel token", category: "cloud" },
  google: { icon: SiGoogle, tone: "text-blue-300", category: "productivity" },
  google_photos: { icon: SiGoogle, tone: "text-sky-300", category: "productivity" },
  email: { icon: SiGoogle, tone: "text-indigo-300", tokenHint: "Email provider token", category: "messaging" },
  beeper: { icon: SiMessenger, tone: "text-blue-300", tokenHint: "Beeper Desktop API access token", category: "messaging" },
  whatsapp: { icon: SiWhatsapp, tone: "text-emerald-300", tokenHint: "WhatsApp token", category: "messaging" },
  messenger: { icon: SiMessenger, tone: "text-blue-300", tokenHint: "Messenger token", category: "messaging" },
  telegram: { icon: SiTelegram, tone: "text-cyan-300", tokenHint: "Telegram token", category: "messaging" },
  google_messages: { icon: SiGoogle, tone: "text-blue-300", tokenHint: "Mautrix Google Messages bridge token", category: "messaging" },
  meta: { icon: SiMessenger, tone: "text-blue-300", tokenHint: "Mautrix Meta bridge token", category: "messaging" },
  signal: { icon: SiSignal, tone: "text-sky-300", tokenHint: "Mautrix Signal bridge token", category: "messaging" },
  discord: { icon: SiDiscord, tone: "text-indigo-300", tokenHint: "Mautrix Discord bridge token", category: "messaging" },
  slack: { icon: FiHash, tone: "text-emerald-300", tokenHint: "Mautrix Slack bridge token", category: "messaging" },
  gvoice: { icon: SiGoogle, tone: "text-blue-300", tokenHint: "Mautrix Google Voice bridge token", category: "messaging" },
  googlechat: { icon: SiGooglechat, tone: "text-blue-300", tokenHint: "Mautrix Google Chat bridge token", category: "messaging" },
  twitter: { icon: SiX, tone: "text-neutral-200", tokenHint: "Mautrix Twitter bridge token", category: "messaging" },
  bluesky: { icon: SiBluesky, tone: "text-sky-300", tokenHint: "Mautrix Bluesky bridge token", category: "messaging" },
  imessage: { icon: SiImessage, tone: "text-blue-300", tokenHint: "Mautrix iMessage bridge token", category: "messaging" },
  imessagego: { icon: SiApple, tone: "text-blue-300", tokenHint: "Beeper iMessage Go bridge token", category: "messaging" },
  linkedin: { icon: FiLink2, tone: "text-blue-300", tokenHint: "Mautrix LinkedIn bridge token", category: "messaging" },
  heisenbridge: { icon: FiDatabase, tone: "text-neutral-300", tokenHint: "Heisenbridge token", category: "messaging" },
  opencode_cli: { icon: FiCpu, tone: "text-emerald-300", category: "ai" },
  tailscale: { icon: SiTailscale, tone: "text-blue-300", tokenHint: "Tailscale API key", category: "cloud" },
  hetzner: { icon: FiDatabase, tone: "text-emerald-300", tokenHint: "Hetzner token", category: "cloud" },
  web3: { icon: SiWeb3dotjs, tone: "text-fuchsia-300", category: "identity" },
  flipper: { icon: FiZap, tone: "text-amber-300", category: "devices" },
  daly_bms: { icon: FiZap, tone: "text-emerald-300", category: "devices" }
};

const CATEGORY_ORDER = ["ai", "messaging", "cloud", "development", "productivity", "identity", "devices", "other"];
const CATEGORY_LABEL = {
  ai: "AI",
  messaging: "Messaging Bridges",
  cloud: "Cloud & Network",
  development: "Development",
  productivity: "Productivity",
  identity: "Identity",
  devices: "Devices",
  other: "Other"
};

const MATRIX_BRIDGE_TOKEN_SOURCE = {
  default: "your bridge config (provisioning shared secret / API token)",
  whatsapp: "your mautrix-whatsapp bridge config",
  telegram: "your mautrix-telegram bridge config",
  signal: "your mautrix-signal bridge config",
  discord: "your mautrix-discord bridge config",
  slack: "your mautrix-slack bridge config",
  google_messages: "your mautrix-gmessages bridge config",
  meta: "your mautrix-meta bridge config",
  gvoice: "your mautrix-gvoice bridge config",
  googlechat: "your mautrix-googlechat bridge config"
};

function IntegrationsPanel(props) {
  const compact = () => Boolean(props?.compact);
  const buttonClass = "inline-flex h-8 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-xs text-neutral-200 transition hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))] disabled:cursor-not-allowed disabled:opacity-60";
  const primaryClass = "inline-flex h-8 items-center gap-1 rounded-md border border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.16)] px-2 text-xs font-medium text-[hsl(var(--primary))] transition hover:bg-[hsl(var(--primary)/0.24)] disabled:cursor-not-allowed disabled:opacity-60";

  const [status, setStatus] = createSignal("");
  const [search, setSearch] = createSignal("");
  const [dialogProviderId, setDialogProviderId] = createSignal("");
  const [step, setStep] = createSignal(1);
  const [busy, setBusy] = createSignal(false);
  const [verifiedForDialog, setVerifiedForDialog] = createSignal(false);

  const [connectorMode, setConnectorMode] = createSignal("user_owned");
  const [accountLabel, setAccountLabel] = createSignal("");
  const [tokenInput, setTokenInput] = createSignal("");
  const [tailscaleTailnet, setTailscaleTailnet] = createSignal("");
  const [tailscaleAuthKey, setTailscaleAuthKey] = createSignal("");
  const [web3Wallet, setWeb3Wallet] = createSignal("");
  const [flipperDeviceName, setFlipperDeviceName] = createSignal("");
  const [flipperProbeSummary, setFlipperProbeSummary] = createSignal("");
  const [flipperProbeDetails, setFlipperProbeDetails] = createSignal(null);
  const [flipperKnownDevices, setFlipperKnownDevices] = createSignal([]);
  const [flipperKnownLoading, setFlipperKnownLoading] = createSignal(false);
  const [dalyDeviceName, setDalyDeviceName] = createSignal("");
  const [dalyProbeSummary, setDalyProbeSummary] = createSignal("");
  const [dalyProbeDetails, setDalyProbeDetails] = createSignal(null);
  const [dalyKnownDevices, setDalyKnownDevices] = createSignal([]);
  const [dalyKnownLoading, setDalyKnownLoading] = createSignal(false);
  const [testRunning, setTestRunning] = createSignal(false);
  const [testCompleted, setTestCompleted] = createSignal(false);
  const [testStageIndex, setTestStageIndex] = createSignal(0);
  const [runtimeState, setRuntimeState] = createSignal({ state: "unknown", message: "" });
  const [runtimePreflight, setRuntimePreflight] = createSignal({ ok: true, imageResolved: true, image: "", tokenEnv: "", message: "" });
  let testLoaderTimer = null;

  const assistantProvider = createMemo(() => workflowUi().provider || "opencode");

  onMount(() => {
    if (typeof window !== "undefined") {
      const url = new URL(window.location.href);
      const params = url.searchParams;
      const oauthStatus = String(params.get("google_oauth") || "").trim().toLowerCase();
      const oauthMessage = String(params.get("google_oauth_message") || "").trim();
      const oauthAccessToken = String(params.get("google_access_token") || "").trim();
      const oauthRefreshToken = String(params.get("google_refresh_token") || "").trim();
      if (oauthStatus || oauthMessage || oauthAccessToken || oauthRefreshToken) {
        if (oauthAccessToken) localStorage.setItem("google_token", oauthAccessToken);
        if (oauthRefreshToken) localStorage.setItem("google_refresh_token", oauthRefreshToken);
        if (oauthStatus === "ok") {
          setStatus(oauthMessage || "Google OAuth connected.");
        } else if (oauthStatus === "error") {
          setStatus(oauthMessage || "Google OAuth failed.");
        }
        params.delete("google_oauth");
        params.delete("google_oauth_message");
        params.delete("google_access_token");
        params.delete("google_refresh_token");
        const query = params.toString();
        window.history.replaceState({}, "", `${url.pathname}${query ? `?${query}` : ""}${url.hash || ""}`);
      }
    }
    integrationStore.checkAll();
    if (props.preselectProviderId) openProviderDialog(canonicalBridgeId(props.preselectProviderId) || props.preselectProviderId);
  });

  createEffect(() => {
    if (typeof window === "undefined") return;
    const value = tailscaleAuthKey().trim();
    localStorage.setItem("tailscale_auth_key", value);
  });

  const providers = createMemo(() => integrationStore.list().map((integration) => ({
    ...integration,
    ...providerMeta[integration.id],
    category: providerMeta[integration.id]?.category || "other"
  })));

  const filteredProviders = createMemo(() => {
    const q = search().trim().toLowerCase();
    if (!q) return providers();
    return providers().filter((provider) => provider.name.toLowerCase().includes(q) || provider.id.toLowerCase().includes(q));
  });

  const groupedProviders = createMemo(() => {
    const groups = new Map(CATEGORY_ORDER.map((category) => [category, []]));
    for (const provider of filteredProviders()) {
      const category = CATEGORY_ORDER.includes(provider.category) ? provider.category : "other";
      groups.get(category).push(provider);
    }
    return CATEGORY_ORDER
      .map((category) => ({
        id: category,
        label: CATEGORY_LABEL[category] || category,
        providers: groups.get(category) || []
      }))
      .filter((group) => group.providers.length > 0);
  });

  const activeProvider = createMemo(() => providers().find((provider) => provider.id === dialogProviderId()) || null);
  const verification = createMemo(() => integrationVerification()[dialogProviderId()] || null);

  function isMatrixBridgeProvider(provider) {
    return Boolean(provider && Array.isArray(provider.tags) && provider.tags.includes("matrix-bridge"));
  }

  function generateBridgeSecret() {
    if (typeof crypto === "undefined" || typeof crypto.getRandomValues !== "function") {
      return `edgerun-bridge-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 18)}`;
    }
    const bytes = new Uint8Array(24);
    crypto.getRandomValues(bytes);
    const token = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
    return `edgerun-bridge-${token}`;
  }

  function ensureMatrixBridgeSecret(provider) {
    if (!isMatrixBridgeProvider(provider)) return;
    const current = tokenInput().trim();
    if (current) return;
    setTokenInput(generateBridgeSecret());
  }

  function usesRuntimeContainer(provider) {
    if (!provider) return false;
    return provider.id === "github" || isMatrixBridgeProvider(provider);
  }

  function providerUsesToken(provider) {
    if (!provider) return false;
    if (provider.id === "web3" || provider.id === "flipper" || provider.id === "daly_bms") return false;
    return provider.requiresToken !== false && provider.authMethod === "token";
  }

  function providerUsesOAuthRedirect(provider) {
    return Boolean(provider && provider.authMethod === "oauth");
  }

  function bypassVerificationGate(provider) {
    return Boolean(provider && provider.id === "github");
  }

  function matrixBridgeTokenGuide(provider) {
    if (!provider || !Array.isArray(provider.tags) || !provider.tags.includes("matrix-bridge")) return null;
    return MATRIX_BRIDGE_TOKEN_SOURCE[provider.id] || MATRIX_BRIDGE_TOKEN_SOURCE.default;
  }

  async function refreshRuntimeState(provider) {
    if (!provider || !usesRuntimeContainer(provider)) {
      setRuntimeState({ state: "not_applicable", message: "No runtime container for this integration." });
      return;
    }
    const result = await integrationStore.runtimeStatus(provider.id);
    setRuntimeState({
      state: String(result?.state || "unknown"),
      message: String(result?.message || "").trim()
    });
  }

  async function refreshRuntimePreflight(provider) {
    if (!provider || !usesRuntimeContainer(provider)) {
      setRuntimePreflight({ ok: true, imageResolved: true, image: "", tokenEnv: "", message: "" });
      return;
    }
    const result = await integrationStore.runtimePreflight(provider.id);
    setRuntimePreflight({
      ok: Boolean(result?.ok),
      imageResolved: Boolean(result?.imageResolved),
      image: String(result?.image || "").trim(),
      tokenEnv: String(result?.tokenEnv || "").trim(),
      message: String(result?.message || "").trim()
    });
  }

  function getLoadingStates(provider) {
    const providerName = provider?.name || "Integration";
    return [
      { text: `Validating ${providerName} credentials` },
      { text: "Checking ownership and profile policy" },
      { text: "Verifying connector capability wiring" },
      { text: "Running readiness tests" },
      { text: "Preparing unlocked capability set" }
    ];
  }

  function stopLoadingStates() {
    if (testLoaderTimer) {
      clearInterval(testLoaderTimer);
      testLoaderTimer = null;
    }
    setTestRunning(false);
  }

  function resetTestFlow() {
    stopLoadingStates();
    setTestCompleted(false);
    setTestStageIndex(0);
  }

  function startLoadingStates(provider) {
    const states = getLoadingStates(provider);
    stopLoadingStates();
    setTestStageIndex(0);
    setTestRunning(true);
    if (states.length <= 1 || typeof window === "undefined") return;
    testLoaderTimer = window.setInterval(() => {
      setTestStageIndex((current) => Math.min(current + 1, states.length - 1));
    }, 2000);
  }

  function openProviderDialog(providerOrId) {
    const normalizedId = typeof providerOrId === "string"
      ? (canonicalBridgeId(providerOrId) || providerOrId)
      : "";
    const provider = typeof providerOrId === "string"
      ? providers().find((entry) => entry.id === normalizedId)
      : providerOrId;
    if (!provider) return;

    setDialogProviderId(provider.id);
    setStep(1);
    setStatus("");
    setBusy(false);
    setVerifiedForDialog(false);
    resetTestFlow();
    setFlipperProbeSummary("");
    setFlipperProbeDetails(null);
    setDalyProbeSummary("");
    setDalyProbeDetails(null);
    setConnectorMode("user_owned");
    setAccountLabel(provider.accountLabel || `${provider.name} Session`);

    if (provider.id === "tailscale" && typeof window !== "undefined") {
      const apiKey = String(integrationStore.getToken("tailscale") || "").trim();
      const authKey = String(localStorage.getItem("tailscale_auth_key") || "").trim();
      setTokenInput(apiKey);
      setTailscaleAuthKey(authKey);
      setTailscaleTailnet(String(localStorage.getItem("tailscale_tailnet") || "").trim());
      return;
    }

    if (provider.id === "web3") {
      setWeb3Wallet(String(localStorage.getItem("web3_wallet") || "").replace(/^evm:/, ""));
      setTokenInput("");
      return;
    }

    if (provider.id === "flipper" && typeof window !== "undefined") {
      setTokenInput(String(localStorage.getItem("flipper_device_id") || "").trim());
      setFlipperDeviceName(String(localStorage.getItem("flipper_device_name") || "").trim());
      void refreshKnownFlipperDevices();
      return;
    }
    if (provider.id === "daly_bms" && typeof window !== "undefined") {
      setTokenInput(String(localStorage.getItem("daly_bms_device_id") || "").trim());
      setDalyDeviceName(String(localStorage.getItem("daly_bms_device_name") || "").trim());
      void refreshKnownDalyDevices();
      return;
    }

    if (provider.tokenKey && typeof window !== "undefined") {
      const value = String(integrationStore.getToken(provider.id) || "").trim();
      setTokenInput(value);
      ensureMatrixBridgeSecret(provider);
    } else {
      setTokenInput("");
      ensureMatrixBridgeSecret(provider);
    }
    void refreshRuntimePreflight(provider);
    void refreshRuntimeState(provider);
  }

  function closeDialog() {
    setDialogProviderId("");
    setStep(1);
    setBusy(false);
    setVerifiedForDialog(false);
    resetTestFlow();
    setFlipperProbeSummary("");
    setFlipperProbeDetails(null);
    setDalyProbeSummary("");
    setDalyProbeDetails(null);
    setRuntimeState({ state: "unknown", message: "" });
    setRuntimePreflight({ ok: true, imageResolved: true, image: "", tokenEnv: "", message: "" });
  }

  function requiredInputsReady(provider) {
    if (!provider) return false;
    if (isMatrixBridgeProvider(provider)) return true;
    if (provider.id === "tailscale") return tokenInput().trim().length > 8 && tailscaleTailnet().trim().length > 0;
    if (provider.id === "web3") return web3Wallet().trim().startsWith("0x");
    if (provider.id === "flipper") return tokenInput().trim().length > 0;
    if (provider.id === "daly_bms") return tokenInput().trim().length > 0;
    if (providerUsesOAuthRedirect(provider)) return true;
    if (!providerUsesToken(provider)) return true;
    return tokenInput().trim().length > 7;
  }

  async function connectWeb3Wallet() {
    try {
      const provider = typeof window !== "undefined" ? window.ethereum : undefined;
      if (!provider?.request) throw new Error("No EVM wallet provider detected.");
      const accounts = await provider.request({ method: "eth_requestAccounts" });
      const wallet = String(Array.isArray(accounts) ? accounts[0] : "").trim();
      if (!wallet) throw new Error("Wallet did not return an address.");
      setWeb3Wallet(wallet);
      localStorage.setItem("web3_wallet", `evm:${wallet}`);
      setStatus(`Wallet connected: ${wallet.slice(0, 6)}...${wallet.slice(-4)}`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to connect wallet.");
    }
  }

  async function runVerification(provider) {
    if (!provider) return;
    setStatus("");
    setTestCompleted(false);
    setFlipperProbeSummary("");
    setFlipperProbeDetails(null);
    setDalyProbeSummary("");
    setDalyProbeDetails(null);
    setBusy(true);

    if (providerUsesOAuthRedirect(provider) && connectorMode() === "user_owned") {
      if (typeof window !== "undefined") {
        const returnTo = `${window.location.pathname}${window.location.search || ""}`;
        const integrationId = encodeURIComponent(provider.id || "google");
        window.location.assign(`/api/google/oauth/start?returnTo=${encodeURIComponent(returnTo)}&integration_id=${integrationId}`);
      }
      setBusy(false);
      return;
    }

    startLoadingStates(provider);
    ensureMatrixBridgeSecret(provider);
    const result = await integrationStore.verify(provider.id, {
      connectorMode: connectorMode(),
      token: tokenInput().trim(),
      tailnet: tailscaleTailnet().trim(),
      apiKey: tokenInput().trim(),
      authKey: tailscaleAuthKey().trim(),
      wallet: web3Wallet().trim(),
      flipperDeviceId: tokenInput().trim(),
      flipperDeviceName: flipperDeviceName().trim(),
      dalyDeviceId: tokenInput().trim(),
      dalyDeviceName: dalyDeviceName().trim()
    });
    stopLoadingStates();
    setBusy(false);

    if (!result.ok) {
      setVerifiedForDialog(false);
      setStatus(result.message || "Verification failed.");
      await refreshRuntimePreflight(provider);
      await refreshRuntimeState(provider);
      return;
    }

    if (provider.id === "flipper") {
      const resolvedId = String(result.deviceId || "").trim();
      const resolvedName = String(result.deviceName || "").trim();
      if (resolvedId) setTokenInput(resolvedId);
      if (resolvedName) {
        setFlipperDeviceName(resolvedName);
        setAccountLabel(resolvedName);
      }
    }
    if (provider.id === "daly_bms") {
      const resolvedId = String(result.deviceId || "").trim();
      const resolvedName = String(result.deviceName || "").trim();
      if (resolvedId) setTokenInput(resolvedId);
      if (resolvedName) {
        setDalyDeviceName(resolvedName);
        setAccountLabel(resolvedName);
      }
    }
    setVerifiedForDialog(true);
    setTestCompleted(true);
    if (result.accountLabel) {
      setAccountLabel(String(result.accountLabel));
    }
    setStatus(result.message || "Verification succeeded.");
    setStep(2);
    await refreshRuntimePreflight(provider);
    await refreshRuntimeState(provider);
  }

  async function selectFlipperDevice() {
    if (typeof window === "undefined" || !window.isSecureContext) {
      setStatus("Web Bluetooth requires HTTPS and a secure browser context.");
      return;
    }
    const bluetooth = navigator?.bluetooth;
    if (!bluetooth?.requestDevice) {
      setStatus("Web Bluetooth API is unavailable in this browser.");
      return;
    }
    try {
      const device = await bluetooth.requestDevice({
        acceptAllDevices: true,
        optionalServices: [FLIPPER_SERIAL_SERVICE_UUID, "battery_service", "device_information"]
      });
      const deviceId = String(device?.id || "").trim();
      const deviceName = String(device?.name || "Flipper").trim();
      if (!deviceId) throw new Error("Selected device did not return an id.");
      setTokenInput(deviceId);
      setFlipperDeviceName(deviceName);
      setAccountLabel(deviceName);
      setStatus(`Selected ${deviceName} via Web Bluetooth.`);
      void refreshKnownFlipperDevices();
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to select Flipper device.");
    }
  }

  async function refreshKnownFlipperDevices() {
    if (typeof window === "undefined" || !window.isSecureContext) {
      setFlipperKnownDevices([]);
      return;
    }
    const bluetooth = navigator?.bluetooth;
    if (!bluetooth?.getDevices) {
      setFlipperKnownDevices([]);
      return;
    }
    setFlipperKnownLoading(true);
    try {
      const devices = await bluetooth.getDevices();
      setFlipperKnownDevices((Array.isArray(devices) ? devices : []).map((device) => ({
        id: String(device?.id || "").trim(),
        name: String(device?.name || "").trim() || "Unknown BLE device"
      })).filter((device) => device.id));
    } catch {
      setFlipperKnownDevices([]);
    } finally {
      setFlipperKnownLoading(false);
    }
  }

  function chooseKnownFlipperDevice(device) {
    const id = String(device?.id || "").trim();
    if (!id) return;
    const name = String(device?.name || "Flipper").trim();
    setTokenInput(id);
    setFlipperDeviceName(name);
    setAccountLabel(name);
    setStatus(`Selected known device: ${name}.`);
  }

  async function selectAndVerifyFlipper(provider) {
    await selectFlipperDevice();
    if (!tokenInput().trim()) return;
    await runVerification(provider);
  }

  async function selectDalyDevice() {
    if (typeof window === "undefined" || !window.isSecureContext) {
      setStatus("Web Bluetooth requires HTTPS and a secure browser context.");
      return;
    }
    const bluetooth = navigator?.bluetooth;
    if (!bluetooth?.requestDevice) {
      setStatus("Web Bluetooth API is unavailable in this browser.");
      return;
    }
    try {
      const device = await bluetooth.requestDevice({
        acceptAllDevices: true,
        optionalServices: DALY_OPTIONAL_SERVICE_UUIDS
      });
      const deviceId = String(device?.id || "").trim();
      const deviceName = String(device?.name || "Daly BMS").trim();
      if (!deviceId) throw new Error("Selected device did not return an id.");
      setTokenInput(deviceId);
      setDalyDeviceName(deviceName);
      setAccountLabel(deviceName);
      setStatus(`Selected ${deviceName} via Web Bluetooth.`);
      void refreshKnownDalyDevices();
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Failed to select Daly BMS device.");
    }
  }

  async function refreshKnownDalyDevices() {
    if (typeof window === "undefined" || !window.isSecureContext) {
      setDalyKnownDevices([]);
      return;
    }
    const bluetooth = navigator?.bluetooth;
    if (!bluetooth?.getDevices) {
      setDalyKnownDevices([]);
      return;
    }
    setDalyKnownLoading(true);
    try {
      const devices = await bluetooth.getDevices();
      const list = (Array.isArray(devices) ? devices : []).map((device) => ({
        id: String(device?.id || "").trim(),
        name: String(device?.name || "").trim() || "Unknown BLE device"
      })).filter((device) => device.id);
      setDalyKnownDevices(list);
    } catch {
      setDalyKnownDevices([]);
    } finally {
      setDalyKnownLoading(false);
    }
  }

  function chooseKnownDalyDevice(device) {
    const id = String(device?.id || "").trim();
    if (!id) return;
    const name = String(device?.name || "Daly BMS").trim();
    setTokenInput(id);
    setDalyDeviceName(name);
    setAccountLabel(name);
    setStatus(`Selected known device: ${name}.`);
  }

  async function selectAndVerifyDaly(provider) {
    await selectDalyDevice();
    if (!tokenInput().trim()) return;
    await runVerification(provider);
  }

  async function saveProvider(provider) {
    if (!provider) return;

    const payload = {
      connectorMode: connectorMode(),
      accountLabel: accountLabel().trim() || `${provider.name} Session`
    };

    if (connectorMode() === "user_owned" && providerUsesToken(provider)) {
      payload.token = tokenInput().trim();
    }

    if (provider.id === "tailscale" && typeof window !== "undefined") {
      localStorage.setItem("tailscale_tailnet", tailscaleTailnet().trim());
      payload.token = tokenInput().trim();
    }

    if (provider.id === "web3") {
      payload.token = `evm:${web3Wallet().trim()}`;
    }
    if (provider.id === "flipper" && typeof window !== "undefined") {
      payload.token = tokenInput().trim();
      payload.accountLabel = accountLabel().trim() || flipperDeviceName().trim() || "Flipper";
      localStorage.setItem("flipper_device_id", tokenInput().trim());
      localStorage.setItem("flipper_device_name", flipperDeviceName().trim());
    }
    if (provider.id === "daly_bms" && typeof window !== "undefined") {
      payload.token = tokenInput().trim();
      payload.accountLabel = accountLabel().trim() || dalyDeviceName().trim() || "Daly BMS";
      localStorage.setItem("daly_bms_device_id", tokenInput().trim());
      localStorage.setItem("daly_bms_device_name", dalyDeviceName().trim());
    }

    const linked = await integrationStore.connect(provider.id, payload);
    if (!linked) {
      const detail = String(integrationStore.get(provider.id)?.lifecycleMessage || "").trim();
      setStatus(detail || `Unable to link ${provider.name}.`);
      await refreshRuntimePreflight(provider);
      await refreshRuntimeState(provider);
      return;
    }
    const linkedDetail = String(integrationStore.get(provider.id)?.lifecycleMessage || "").trim();
    setStatus(linkedDetail || `${provider.name} integration linked.`);
    await refreshRuntimePreflight(provider);
    await refreshRuntimeState(provider);
    closeDialog();
  }

  async function runFlipperProbe() {
    setBusy(true);
    const result = await integrationStore.probeFlipper({
      flipperDeviceId: tokenInput().trim(),
      flipperDeviceName: flipperDeviceName().trim()
    });
    setBusy(false);
    if (!result.ok) {
      setFlipperProbeDetails(null);
      setFlipperProbeSummary(result.message || "Flipper probe failed.");
      return;
    }
    setVerifiedForDialog(true);
    setFlipperProbeDetails(result);
    const battery = Number.isFinite(result.batteryLevel) ? `${result.batteryLevel}%` : "n/a";
    const model = String(result.summary?.model || result.deviceName || "").trim() || "unknown";
    const serviceCount = Array.isArray(result.services) ? result.services.length : 0;
    const warnings = Array.isArray(result.diagnostics) ? result.diagnostics : [];
    const warningText = warnings.length > 0 ? ` · ${warnings.join(", ")}` : "";
    const latency = Number.isFinite(result?.rpc?.ping?.latencyMs) ? ` · ping ${result.rpc.ping.latencyMs}ms` : "";
    setFlipperProbeSummary(`Probe ok · battery ${battery} · model ${model} · services ${serviceCount}${latency}${warningText}`);
  }

  async function runDalyProbe() {
    setBusy(true);
    const result = await integrationStore.probeDalyBms({
      dalyDeviceId: tokenInput().trim(),
      dalyDeviceName: dalyDeviceName().trim()
    });
    setBusy(false);
    if (!result.ok) {
      setDalyProbeDetails(null);
      setDalyProbeSummary(result.message || "Daly probe failed.");
      return;
    }
    setVerifiedForDialog(true);
    setDalyProbeDetails(result);
    const battery = Number.isFinite(result.batteryLevel) ? `${result.batteryLevel}%` : "n/a";
    const packets = Number.isFinite(result?.stats?.packetCount)
      ? result.stats.packetCount
      : (Array.isArray(result.packetSamplesHex) ? result.packetSamplesHex.length : 0);
    const protocol = String(result.protocol || "unknown").trim();
    const elapsed = Number.isFinite(result?.stats?.elapsedMs) ? ` · ${result.stats.elapsedMs}ms` : "";
    const warnings = Array.isArray(result.diagnostics) ? result.diagnostics : [];
    const warningText = warnings.length > 0 ? ` · ${warnings.join(", ")}` : "";
    setDalyProbeSummary(`Probe ok · battery ${battery} · protocol ${protocol} · packets ${packets}${elapsed}${warningText}`);
  }

  function disconnectProvider(provider) {
    if (!provider) return;
    integrationStore.disconnect(provider.id);
    setStatus(`${provider.name} disconnected.`);
    void refreshRuntimeState(provider);
    closeDialog();
  }

  function stepClass(target) {
    if (step() === target) return "border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.08)] text-[hsl(var(--primary))]";
    if (step() > target) return "border-emerald-500/40 bg-emerald-500/10 text-emerald-300";
    return "border-neutral-800 bg-neutral-900/60 text-neutral-500";
  }

  function stepFlow(provider) {
    if (!provider) return [1, 2];
    return [1, 2];
  }

  function stepTitle(value) {
    if (value === 1) return "Required Info";
    return "Run Tests";
  }

  onCleanup(() => {
    stopLoadingStates();
  });

  return (
    <div class={`h-full overflow-auto text-neutral-200 ${compact() ? "" : "bg-[#0f1013] p-4"}`}>
      <div class="border-b border-neutral-800 px-3 py-2">
        <h3 class="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-neutral-300">
          <FiLink2 size={18} />
          <span>Integrations</span>
        </h3>
        <p class="mt-1 text-xs text-neutral-400">Icons are providers. Hover for details, click Connect to launch stepper.</p>
        <div class="mt-2 grid grid-cols-1 gap-1 rounded-md border border-neutral-800 bg-neutral-900/60 p-1">
          <button
            type="button"
            class={`rounded px-2 py-1 text-[11px] transition-colors ${assistantProvider() === "opencode" ? "font-semibold text-[hsl(var(--primary))]" : "text-neutral-400 hover:bg-neutral-800"}`}
            onClick={() => setAssistantProvider("opencode")}
          >
            OpenCode active
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

      <Show when={status() && !activeProvider()}>
        <div class="mx-3 mb-3 rounded-md border border-neutral-800 bg-neutral-900/55 px-2.5 py-2 text-xs text-neutral-300">{status()}</div>
      </Show>

      <div class="space-y-3 px-3 pb-3" data-testid="integrations-icon-grid">
        <For each={groupedProviders()}>
          {(group) => (
            <section>
              <p class="mb-2 text-[10px] font-medium uppercase tracking-wide text-neutral-500">{group.label}</p>
              <div class="grid grid-cols-4 gap-2 sm:grid-cols-5">
                <For each={group.providers}>
                  {(provider) => {
                    const Icon = provider.icon || FiCloud;
                    const connected = () => Boolean(provider.connected);
                    return (
                      <div class="group relative flex flex-col items-center gap-1">
                        <button
                          type="button"
                          class={`relative inline-flex h-12 w-12 items-center justify-center rounded-xl border transition ${connected() ? "border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.12)]" : "border-neutral-700 bg-neutral-900 hover:border-neutral-500"}`}
                          onClick={() => openProviderDialog(provider)}
                          title={`${provider.name} • ${provider.connected ? "Connected" : "Not connected"} • ${provider.available ? "Available" : "Unavailable"}`}
                          data-testid={`provider-open-${provider.id}`}
                        >
                          <Icon size={18} class={connected() ? "text-[hsl(var(--primary))]" : "text-neutral-300"} />
                          <span class={`absolute -right-1 -top-1 h-2.5 w-2.5 rounded-full ${provider.available ? "bg-emerald-400" : provider.connected ? "bg-amber-400" : "bg-neutral-600"}`} />
                        </button>
                        <button
                          type="button"
                          class="max-w-[84px] truncate text-[10px] text-neutral-300 transition hover:text-[hsl(var(--primary))]"
                          onClick={() => openProviderDialog(provider)}
                          data-testid={`provider-connect-${provider.id}`}
                        >
                          {provider.name}
                        </button>
                        <div class="pointer-events-none absolute -bottom-9 z-10 hidden whitespace-nowrap rounded border border-neutral-700 bg-[#0b0c0f] px-2 py-1 text-[10px] text-neutral-200 shadow-xl group-hover:block">
                          {provider.name} • {provider.availabilityReason}
                        </div>

                        <span class="sr-only" data-testid={`provider-connected-${provider.id}`}>{provider.connected ? "Connected" : "Not connected"}</span>
                        <span class="sr-only" data-testid={`provider-available-${provider.id}`}>{provider.available ? "Available" : "Unavailable"}</span>
                        <span class="sr-only" data-testid={`provider-mode-${provider.id}`}>{provider.connectorMode === "platform" ? "Platform" : "User-owned"}</span>
                      </div>
                    );
                  }}
                </For>
              </div>
            </section>
          )}
        </For>
      </div>

      <Show when={activeProvider()}>
        {(providerAccessor) => {
          const provider = providerAccessor();
          const lifecycleStatus = () => String(integrationStore.get(provider.id)?.lifecycleStatus || "idle").trim() || "idle";
          const verified = () => bypassVerificationGate(provider)
            || integrationStore.isConnected(provider.id)
            || lifecycleStatus() === "verified"
            || Boolean(verification()?.ok)
            || verifiedForDialog();
          const verificationMessage = () => String(
            integrationStore.get(provider.id)?.lifecycleMessage
            || verification()?.message
            || ""
          );
          const ProviderIcon = provider.icon || FiCloud;
          const flow = () => stepFlow(provider);
          const stepIndex = () => Math.max(0, flow().indexOf(step()));
          const loadingStates = createMemo(() => getLoadingStates(provider));
          const visibleLoadingStage = createMemo(() => {
            const list = loadingStates();
            if (list.length === 0) return 0;
            return Math.min(testStageIndex(), list.length - 1);
          });
          const activeLoadingText = createMemo(() => loadingStates()[visibleLoadingStage()]?.text || "Running integration tests...");
          return (
            <Portal>
              <div class="fixed inset-0 z-[12000] flex items-center justify-center bg-black/70 px-4 py-6">
                <div class="h-[min(489px,60vh)] w-[min(748px,92vw)] overflow-auto rounded-xl border border-neutral-700 bg-[#101216] p-5 shadow-2xl" data-testid={`provider-dialog-${provider.id}`}>
                <div class="mb-3 flex items-start justify-between gap-2">
                  <div>
                    <h4 class="flex items-center gap-2 text-sm font-semibold text-white">
                      <ProviderIcon size={16} class="text-[hsl(var(--primary))]" />
                      <span>{provider.name} Setup</span>
                      <span class="rounded border border-neutral-700 bg-neutral-900/80 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-neutral-300">
                        {lifecycleStatus()}
                      </span>
                    </h4>
                    <p class="mt-0.5 text-xs text-neutral-500">Intent-driven setup flow with verification gate.</p>
                  </div>
                  <button type="button" class={buttonClass} onClick={closeDialog}>
                    <FiXCircle size={12} />
                    Close
                  </button>
                </div>
                <Show when={status()}>
                  <div class="mb-3 rounded-md border border-neutral-800 bg-neutral-900/55 px-2.5 py-2 text-xs text-neutral-300">{status()}</div>
                </Show>

                <div class="mb-3 grid grid-cols-2 gap-1">
                  <For each={flow()}>
                    {(stepValue, index) => (
                      <button
                        type="button"
                        class={`rounded-md border px-2 py-1 text-[11px] ${stepClass(stepValue)}`}
                        onClick={() => setStep(stepValue)}
                        data-testid={`integration-step-${stepValue}`}
                      >
                        {index() + 1}. {stepTitle(stepValue)}
                      </button>
                    )}
                  </For>
                </div>

                <Show when={step() === 1}>
                  <section class="space-y-2 rounded-md border border-neutral-800 bg-neutral-900/50 p-3" data-testid="integration-stepper-values">
                    <label class="block text-[11px] text-neutral-500">
                      Account label
                      <input
                        type="text"
                        value={accountLabel()}
                        onInput={(event) => setAccountLabel(event.currentTarget.value)}
                        class="mt-1 h-8 w-full rounded border border-neutral-700 bg-neutral-900 px-2 text-xs text-neutral-200 focus:border-neutral-500 focus:outline-none"
                      />
                    </label>

                    <Show when={providerUsesToken(provider) && !isMatrixBridgeProvider(provider)}>
                      <label class="block text-[11px] text-neutral-500">
                        Token
                        <input
                          type="password"
                          value={tokenInput()}
                          onInput={(event) => setTokenInput(event.currentTarget.value)}
                          placeholder={provider.tokenHint || "Access token"}
                          class="mt-1 h-8 w-full rounded border border-neutral-700 bg-neutral-900 px-2 text-xs text-neutral-200 focus:border-neutral-500 focus:outline-none"
                          data-testid={provider.id === "tailscale" ? "tailscale-api-key-input" : `provider-token-${provider.id}`}
                        />
                      </label>
                    </Show>
                    <Show when={isMatrixBridgeProvider(provider)}>
                      <p class="text-[11px] text-neutral-500" data-testid={`matrix-token-auto-${provider.id}`}>
                        EdgeRun generates and manages the bridge secret automatically for this integration.
                      </p>
                    </Show>
                    <p class="text-[11px] text-neutral-500" data-testid="integration-mode-help-label">
                      User-owned setup: provide your own integration credentials in this dialog.
                    </p>
                    <Show when={matrixBridgeTokenGuide(provider)}>
                      {(source) => (
                        <div
                          class="rounded-md border border-neutral-800 bg-[#0b0c0f] px-2.5 py-2 text-[11px] text-neutral-400"
                          data-testid={`matrix-token-guidance-${provider.id}`}
                        >
                          <p>
                            We still use the bridge provisioning/API secret from {source()} under the hood. This is not your Matrix account password or OAuth token.
                          </p>
                        </div>
                      )}
                    </Show>
                    <Show when={provider.id === "github"}>
                      <p class="text-[11px] text-neutral-500">PAT is persisted in TPM-backed local credentials vault.</p>
                    </Show>
                    <Show when={provider.id === "cloudflare"}>
                      <div class="space-y-1.5 rounded-md border border-neutral-800 bg-[#0b0c0f] px-2.5 py-2 text-[11px] text-neutral-400">
                        <p>Use a Cloudflare account API token (not Global API Key). Minimum scope: User Tokens:Read.</p>
                        <button
                          type="button"
                          class={buttonClass}
                          onClick={() => {
                            if (typeof window === "undefined") return;
                            window.open("https://dash.cloudflare.com/profile/api-tokens", "_blank", "noopener,noreferrer");
                          }}
                          data-testid="cloudflare-open-token-dashboard"
                        >
                          Open Cloudflare token page
                        </button>
                      </div>
                    </Show>
                    <Show when={provider.id === "beeper"}>
                      <div class="space-y-1.5 rounded-md border border-neutral-800 bg-[#0b0c0f] px-2.5 py-2 text-[11px] text-neutral-400">
                        <p>Sign in to Beeper Desktop, enable Desktop API in Settings -&gt; Developers, then paste your Beeper access token.</p>
                        <button
                          type="button"
                          class={buttonClass}
                          onClick={() => {
                            if (typeof window === "undefined") return;
                            window.open("https://developers.beeper.com/desktop-api/", "_blank", "noopener,noreferrer");
                          }}
                          data-testid="beeper-open-desktop-api-docs"
                        >
                          Open Beeper Desktop API guide
                        </button>
                      </div>
                    </Show>

                    <Show when={provider.id === "tailscale"}>
                      <label class="block text-[11px] text-neutral-500">
                        Tailnet
                        <input
                          type="text"
                          value={tailscaleTailnet()}
                          onInput={(event) => setTailscaleTailnet(event.currentTarget.value)}
                          placeholder="example.github"
                          class="mt-1 h-8 w-full rounded border border-neutral-700 bg-neutral-900 px-2 text-xs text-neutral-200 focus:border-neutral-500 focus:outline-none"
                          data-testid="tailscale-tailnet-input"
                        />
                      </label>
                      <label class="block text-[11px] text-neutral-500">
                        Node auth key (optional)
                        <input
                          type="password"
                          value={tailscaleAuthKey()}
                          onInput={(event) => setTailscaleAuthKey(event.currentTarget.value)}
                          placeholder="tskey-auth-..."
                          class="mt-1 h-8 w-full rounded border border-neutral-700 bg-neutral-900 px-2 text-xs text-neutral-200 focus:border-neutral-500 focus:outline-none"
                          data-testid="tailscale-auth-key-input"
                        />
                      </label>
                    </Show>

                    <Show when={provider.id === "web3"}>
                      <div class="space-y-2 rounded border border-neutral-800 bg-[#0b0c0f] p-2">
                        <p class="text-[11px] text-neutral-400">Wallet: {web3Wallet() || "Not connected"}</p>
                        <button type="button" class={buttonClass} onClick={connectWeb3Wallet}>Connect wallet</button>
                      </div>
                    </Show>

                    <Show when={provider.id === "flipper"}>
                      <div class="space-y-2 rounded border border-neutral-800 bg-[#0b0c0f] p-2">
                        <p class="text-[11px] text-neutral-400">Device: {flipperDeviceName() || "Not selected"}</p>
                        <p class="text-[11px] text-neutral-500">
                          Pairing PIN/passkey is handled by your browser + OS + device. Edgerun does not store BLE passkeys.
                        </p>
                        <div class="flex flex-wrap gap-2">
                          <button
                            type="button"
                            class={buttonClass}
                            onClick={selectFlipperDevice}
                            data-testid="flipper-select-device"
                          >
                            Select Device
                          </button>
                          <button
                            type="button"
                            class={buttonClass}
                            onClick={() => { void refreshKnownFlipperDevices(); }}
                            disabled={flipperKnownLoading()}
                            data-testid="flipper-refresh-known"
                          >
                            {flipperKnownLoading() ? "Refreshing..." : "Refresh Known"}
                          </button>
                          <button
                            type="button"
                            class={primaryClass}
                            onClick={() => { void selectAndVerifyFlipper(provider); }}
                            disabled={busy()}
                            data-testid="flipper-select-verify"
                          >
                            Select + Verify
                          </button>
                        </div>
                        <Show when={flipperKnownDevices().length > 0}>
                          <div class="max-h-20 overflow-auto rounded border border-neutral-800 bg-neutral-950/50 p-1.5">
                            <For each={flipperKnownDevices()}>
                              {(device) => (
                                <button
                                  type="button"
                                  class="mb-1 flex w-full items-center justify-between rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1 text-[10px] text-neutral-200 hover:bg-neutral-800"
                                  onClick={() => chooseKnownFlipperDevice(device)}
                                  data-testid={`flipper-known-${device.id}`}
                                >
                                  <span class="truncate">{device.name}</span>
                                  <span class="ml-2 truncate text-neutral-500">{device.id.slice(0, 8)}...</span>
                                </button>
                              )}
                            </For>
                          </div>
                        </Show>
                      </div>
                    </Show>
                    <Show when={provider.id === "daly_bms"}>
                      <div class="space-y-2 rounded border border-neutral-800 bg-[#0b0c0f] p-2">
                        <p class="text-[11px] text-neutral-400">Device: {dalyDeviceName() || "Not selected"}</p>
                        <p class="text-[11px] text-neutral-500">
                          Daly integration uses BLE transport with read-first diagnostics. Writes are limited to probe polling frames.
                        </p>
                        <div class="flex flex-wrap gap-2">
                          <button
                            type="button"
                            class={buttonClass}
                            onClick={selectDalyDevice}
                            data-testid="daly-select-device"
                          >
                            Select Device
                          </button>
                          <button
                            type="button"
                            class={buttonClass}
                            onClick={() => { void refreshKnownDalyDevices(); }}
                            disabled={dalyKnownLoading()}
                            data-testid="daly-refresh-known"
                          >
                            {dalyKnownLoading() ? "Refreshing..." : "Refresh Known"}
                          </button>
                          <button
                            type="button"
                            class={primaryClass}
                            onClick={() => { void selectAndVerifyDaly(provider); }}
                            disabled={busy()}
                            data-testid="daly-select-verify"
                          >
                            Select + Verify
                          </button>
                        </div>
                        <Show when={dalyKnownDevices().length > 0}>
                          <div class="max-h-20 overflow-auto rounded border border-neutral-800 bg-neutral-950/50 p-1.5">
                            <For each={dalyKnownDevices()}>
                              {(device) => (
                                <button
                                  type="button"
                                  class="mb-1 flex w-full items-center justify-between rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1 text-[10px] text-neutral-200 hover:bg-neutral-800"
                                  onClick={() => chooseKnownDalyDevice(device)}
                                  data-testid={`daly-known-${device.id}`}
                                >
                                  <span class="truncate">{device.name}</span>
                                  <span class="ml-2 truncate text-neutral-500">{device.id.slice(0, 8)}...</span>
                                </button>
                              )}
                            </For>
                          </div>
                        </Show>
                      </div>
                    </Show>

                    <Show when={providerUsesOAuthRedirect(provider)}>
                      <p class="text-[11px] text-neutral-500">OAuth integrations redirect on Run Tests step to complete login.</p>
                    </Show>
                  </section>
                </Show>

                <Show when={step() === 2}>
                  <section class="space-y-2 rounded-md border border-neutral-800 bg-neutral-900/50 p-3" data-testid={provider.id === "tailscale" ? "provider-tailscale-quickstart" : "integration-stepper-verify"}>
                    <p class="text-xs text-neutral-400">
                      {bypassVerificationGate(provider)
                        ? "Verification is optional for this provider. Link directly with your token."
                        : "Run integration tests before linking."}
                    </p>
                    <Show when={!bypassVerificationGate(provider)}>
                      <button
                        type="button"
                        class={primaryClass}
                        disabled={busy() || !requiredInputsReady(provider)}
                        onClick={() => runVerification(provider)}
                        data-testid={provider.id === "tailscale" ? "tailscale-load-devices" : `provider-verify-${provider.id}`}
                      >
                        <FiZap size={12} />
                        {busy() ? "Verifying..." : "Run verification"}
                      </button>
                    </Show>
                    <Show when={!requiredInputsReady(provider)}>
                      <p class="text-[11px] text-amber-300">Fill required values in Step 1 first.</p>
                    </Show>
                    <Show when={usesRuntimeContainer(provider)}>
                      <div class="rounded-md border border-neutral-800 bg-black/25 px-2 py-1.5 text-[11px]" data-testid={`integration-runtime-preflight-${provider.id}`}>
                        <p class="text-neutral-400">Runtime preflight</p>
                        <p class={`mt-0.5 ${runtimePreflight().ok && runtimePreflight().imageResolved ? "text-emerald-300" : "text-amber-300"}`}>
                          {runtimePreflight().ok && runtimePreflight().imageResolved ? "Image mapping configured" : "Image mapping missing"}
                        </p>
                        <Show when={runtimePreflight().image}>
                          <p class="mt-0.5 break-all font-mono text-[10px] text-neutral-500">{runtimePreflight().image}</p>
                        </Show>
                        <Show when={runtimePreflight().tokenEnv}>
                          <p class="mt-0.5 text-[10px] text-neutral-500">Token env: {runtimePreflight().tokenEnv}</p>
                        </Show>
                        <Show when={runtimePreflight().message && !(runtimePreflight().ok && runtimePreflight().imageResolved)}>
                          <p class="mt-0.5 text-[10px] text-amber-300">{runtimePreflight().message}</p>
                        </Show>
                      </div>
                      <div class="rounded-md border border-neutral-800 bg-black/25 px-2 py-1.5 text-[11px]" data-testid={`integration-runtime-state-${provider.id}`}>
                        <p class="text-neutral-400">Runtime container</p>
                        <p class={`mt-0.5 ${
                          runtimeState().state === "running"
                            ? "text-emerald-300"
                            : runtimeState().state === "error"
                              ? "text-red-300"
                              : "text-amber-300"
                        }`}>
                          {runtimeState().state === "running"
                            ? "Running"
                            : runtimeState().state === "error"
                              ? "Error"
                              : "Not started"}
                        </p>
                        <Show when={runtimeState().message}>
                          <p class="mt-0.5 text-[10px] text-neutral-500">{runtimeState().message}</p>
                        </Show>
                      </div>
                    </Show>
                    <Show when={testRunning()}>
                      <div class="rounded-md border border-neutral-700 bg-black/20 p-2" data-testid="integration-test-loader">
                        <p class="text-[11px] text-neutral-300">{activeLoadingText()}</p>
                        <div class="mt-2 space-y-1">
                          <For each={loadingStates()}>
                            {(state, index) => {
                              const done = () => index() < visibleLoadingStage();
                              const active = () => index() === visibleLoadingStage();
                              return (
                                <div class="flex items-center gap-2 text-[10px]">
                                  <span class={`h-2 w-2 rounded-full ${done() ? "bg-emerald-400" : active() ? "bg-cyan-300 animate-pulse" : "bg-neutral-700"}`} />
                                  <span class={`${done() ? "text-emerald-200" : active() ? "text-cyan-200" : "text-neutral-500"}`}>{state.text}</span>
                                </div>
                              );
                            }}
                          </For>
                        </div>
                      </div>
                    </Show>
                    <Show when={verificationMessage()}>
                      <p class={`text-[11px] ${verified() ? "text-emerald-300" : lifecycleStatus() === "verifying" ? "text-amber-300" : "text-red-300"}`}>{verificationMessage()}</p>
                    </Show>
                    <Show when={verified() || testCompleted() || bypassVerificationGate(provider)}>
                      <section class="space-y-2 rounded-md border border-emerald-500/35 bg-emerald-500/10 p-3" data-testid="integration-stepper-success">
                        <p class="flex items-center gap-1 text-xs font-medium text-emerald-200">
                          <FiCheckCircle size={12} />
                          Tests passed
                        </p>
                        <p class="text-[11px] text-emerald-100/80">Link integration to unlock:</p>
                        <ul class="space-y-1 text-[11px] text-emerald-100/90" data-testid="integration-unlocked-capabilities">
                          <For each={provider.defaultCapabilities || []}>
                            {(capability) => <li>• {capability}</li>}
                          </For>
                        </ul>
                        <Show when={provider.id === "flipper"}>
                          <div class="mt-2 rounded border border-neutral-700 bg-neutral-950/55 p-2">
                            <button
                              type="button"
                              class={buttonClass}
                              onClick={() => { void runFlipperProbe(); }}
                              disabled={busy()}
                              data-testid="flipper-run-probe"
                            >
                              <FiZap size={12} />
                              {busy() ? "Probing..." : "Probe Device"}
                            </button>
                            <Show when={flipperProbeSummary()}>
                              <p class="mt-2 text-[11px] text-neutral-200" data-testid="flipper-probe-summary">{flipperProbeSummary()}</p>
                            </Show>
                            <Show when={flipperProbeDetails()}>
                              {(probeAccessor) => {
                                const probe = probeAccessor();
                                const warnings = Array.isArray(probe?.diagnostics) ? probe.diagnostics : [];
                                const entries = Array.isArray(probe?.deviceInfoEntries) ? probe.deviceInfoEntries.slice(0, 4) : [];
                                return (
                                  <div class="mt-2 space-y-1 rounded border border-neutral-800 bg-black/20 p-2 text-[10px] text-neutral-300" data-testid="flipper-probe-details">
                                    <p>Model: {probe?.summary?.model || "unknown"}</p>
                                    <p>Firmware: {probe?.summary?.firmware || "unknown"}</p>
                                    <p>Hardware: {probe?.summary?.hardware || "unknown"}</p>
                                    <p>Flow budget: {Number.isFinite(probe?.serial?.flowBudget) ? probe.serial.flowBudget : "n/a"}</p>
                                    <Show when={Number.isFinite(probe?.rpc?.ping?.latencyMs)}>
                                      <p>Ping latency: {probe.rpc.ping.latencyMs}ms</p>
                                    </Show>
                                    <Show when={entries.length > 0}>
                                      <div class="space-y-0.5">
                                        <p class="text-neutral-400">Device info sample:</p>
                                        <For each={entries}>
                                          {(entry) => <p>{entry.key}: {entry.value || "n/a"}</p>}
                                        </For>
                                      </div>
                                    </Show>
                                    <Show when={warnings.length > 0}>
                                      <div class="space-y-0.5 text-amber-300">
                                        <p>Warnings:</p>
                                        <For each={warnings}>
                                          {(warning) => <p>• {warning}</p>}
                                        </For>
                                      </div>
                                    </Show>
                                  </div>
                                );
                              }}
                            </Show>
                          </div>
                        </Show>
                        <Show when={provider.id === "daly_bms"}>
                          <div class="mt-2 rounded border border-neutral-700 bg-neutral-950/55 p-2">
                            <button
                              type="button"
                              class={buttonClass}
                              onClick={() => { void runDalyProbe(); }}
                              disabled={busy()}
                              data-testid="daly-run-probe"
                            >
                              <FiZap size={12} />
                              {busy() ? "Probing..." : "Probe Daly"}
                            </button>
                            <Show when={dalyProbeSummary()}>
                              <p class="mt-2 text-[11px] text-neutral-200" data-testid="daly-probe-summary">{dalyProbeSummary()}</p>
                            </Show>
                            <Show when={dalyProbeDetails()}>
                              {(probeAccessor) => {
                                const probe = probeAccessor();
                                const warnings = Array.isArray(probe?.diagnostics) ? probe.diagnostics : [];
                                const packets = Array.isArray(probe?.packetSamplesHex) ? probe.packetSamplesHex.slice(0, 3) : [];
                                const stats = probe?.stats || {};
                                return (
                                  <div class="mt-2 space-y-1 rounded border border-neutral-800 bg-black/20 p-2 text-[10px] text-neutral-300" data-testid="daly-probe-details">
                                    <p>Protocol: {probe?.protocol || "unknown"}</p>
                                    <p>Battery: {Number.isFinite(probe?.batteryLevel) ? `${probe.batteryLevel}%` : "n/a"}</p>
                                    <p>Services: {Array.isArray(probe?.services) ? probe.services.length : 0}</p>
                                    <p>Profile: {probe?.serial?.profileLabel || "unknown"}</p>
                                    <Show when={Number.isFinite(stats?.packetCount) || Number.isFinite(stats?.elapsedMs)}>
                                      <div class="space-y-0.5">
                                        <p class="text-neutral-400">Stats:</p>
                                        <Show when={Number.isFinite(stats?.packetCount)}>
                                          <p>Packets captured: {stats.packetCount}</p>
                                        </Show>
                                        <Show when={Number.isFinite(stats?.d2PacketCount) || Number.isFinite(stats?.a5PacketCount)}>
                                          <p>D2/A5 frames: {Number.isFinite(stats?.d2PacketCount) ? stats.d2PacketCount : 0}/{Number.isFinite(stats?.a5PacketCount) ? stats.a5PacketCount : 0}</p>
                                        </Show>
                                        <Show when={Number.isFinite(stats?.packetBytesAvg) || Number.isFinite(stats?.packetBytesMax)}>
                                          <p>Packet bytes avg/max: {Number.isFinite(stats?.packetBytesAvg) ? stats.packetBytesAvg : 0}/{Number.isFinite(stats?.packetBytesMax) ? stats.packetBytesMax : 0}</p>
                                        </Show>
                                        <Show when={Number.isFinite(stats?.writeAttempts) || Number.isFinite(stats?.writeSuccessRate)}>
                                          <p>Write success: {Number.isFinite(stats?.writeSuccesses) ? stats.writeSuccesses : 0}/{Number.isFinite(stats?.writeAttempts) ? stats.writeAttempts : 0} ({Number.isFinite(stats?.writeSuccessRate) ? stats.writeSuccessRate : 0}%)</p>
                                        </Show>
                                        <Show when={Number.isFinite(stats?.notifyCharacteristicCount) || Number.isFinite(stats?.writeCharacteristicCount)}>
                                          <p>Notify/write chars: {Number.isFinite(stats?.notifyCharacteristicCount) ? stats.notifyCharacteristicCount : 0}/{Number.isFinite(stats?.writeCharacteristicCount) ? stats.writeCharacteristicCount : 0}</p>
                                        </Show>
                                        <Show when={Number.isFinite(stats?.elapsedMs)}>
                                          <p>Probe elapsed: {stats.elapsedMs}ms</p>
                                        </Show>
                                      </div>
                                    </Show>
                                    <Show when={packets.length > 0}>
                                      <div class="space-y-0.5">
                                        <p class="text-neutral-400">Packet samples:</p>
                                        <For each={packets}>
                                          {(sample) => <p>{sample}</p>}
                                        </For>
                                      </div>
                                    </Show>
                                    <Show when={warnings.length > 0}>
                                      <div class="space-y-0.5 text-amber-300">
                                        <p>Warnings:</p>
                                        <For each={warnings}>
                                          {(warning) => <p>• {warning}</p>}
                                        </For>
                                      </div>
                                    </Show>
                                  </div>
                                );
                              }}
                            </Show>
                          </div>
                        </Show>
                      </section>
                    </Show>
                  </section>
                </Show>

                <div class="mt-3 flex flex-wrap gap-2">
                  <button
                    type="button"
                    class={buttonClass}
                    onClick={() => setStep(flow()[Math.max(0, stepIndex() - 1)] || flow()[0])}
                    disabled={stepIndex() === 0}
                  >
                    <FiArrowLeft size={12} />
                    Back
                  </button>
                  <button
                    type="button"
                    class={buttonClass}
                    onClick={() => setStep(flow()[Math.min(flow().length - 1, stepIndex() + 1)] || flow()[flow().length - 1])}
                    disabled={stepIndex() === flow().length - 1 || (step() === 1 && !requiredInputsReady(provider))}
                  >
                    Next
                    <FiArrowRight size={12} />
                  </button>
                  <button
                    type="button"
                    class={primaryClass}
                    disabled={(step() === 1 && !requiredInputsReady(provider)) || (!verified() && !bypassVerificationGate(provider)) || busy()}
                    onClick={() => { void saveProvider(provider); }}
                    data-testid={`provider-save-${provider.id}`}
                  >
                    <FiLock size={12} />
                    {provider.connected ? "Update Integration" : "Link Integration"}
                  </button>
                  <Show when={provider.connected}>
                    <button type="button" class={buttonClass} onClick={() => disconnectProvider(provider)}>
                      <FiXCircle size={12} />
                      Disconnect
                    </button>
                  </Show>
                  <Show when={provider.id === "flipper" && verified()}>
                    <button
                      type="button"
                      class={buttonClass}
                      onClick={() => openWorkflowFlipper(accountLabel().trim() || flipperDeviceName().trim() || "Flipper")}
                      data-testid="flipper-create-workflow"
                    >
                      <FiZap size={12} />
                      Create Flipper Workflow
                    </button>
                  </Show>
                </div>
                </div>
              </div>
            </Portal>
          );
        }}
      </Show>
    </div>
  );
}

export { IntegrationsPanel as default };
