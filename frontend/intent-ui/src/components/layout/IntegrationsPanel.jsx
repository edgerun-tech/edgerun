import { For, Show, createEffect, createMemo, createSignal, onMount } from "solid-js";
import { Portal } from "solid-js/web";
import {
  FiLink2,
  FiCheckCircle,
  FiCloud,
  FiDatabase,
  FiCpu,
  FiSearch,
  FiXCircle,
  FiShield,
  FiArrowRight,
  FiArrowLeft,
  FiZap,
  FiLock
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
  SiWeb3dotjs
} from "solid-icons/si";
import { integrationStore, integrationVerification } from "../../stores/integrations";
import { openWorkflowFlipper, setAssistantProvider, workflowUi } from "../../stores/workflow-ui";
import { profileRuntime } from "../../stores/profile-runtime";
import { getProfileSecret, setProfileSecret } from "../../stores/profile-secrets";

const providerMeta = {
  github: { id: "github", name: "GitHub", icon: SiGithub, tone: "text-neutral-100", tokenHint: "GitHub Personal Access Token", useToken: true },
  cloudflare: { id: "cloudflare", name: "Cloudflare", icon: SiCloudflare, tone: "text-orange-300", tokenHint: "Cloudflare token", useToken: true },
  vercel: { id: "vercel", name: "Vercel", icon: SiVercel, tone: "text-neutral-100", tokenHint: "Vercel token", useToken: true },
  google: { id: "google", name: "Google", icon: SiGoogle, tone: "text-blue-300", useToken: false, oauthRedirect: true },
  google_photos: { id: "google_photos", name: "Google Photos", icon: SiGoogle, tone: "text-sky-300", useToken: false, oauthRedirect: true },
  email: { id: "email", name: "Email", icon: SiGoogle, tone: "text-indigo-300", tokenHint: "Email provider token", useToken: true },
  whatsapp: { id: "whatsapp", name: "WhatsApp", icon: SiWhatsapp, tone: "text-emerald-300", tokenHint: "WhatsApp token", useToken: true },
  messenger: { id: "messenger", name: "Messenger", icon: SiMessenger, tone: "text-blue-300", tokenHint: "Messenger token", useToken: true },
  telegram: { id: "telegram", name: "Telegram", icon: SiTelegram, tone: "text-cyan-300", tokenHint: "Telegram token", useToken: true },
  qwen: { id: "qwen", name: "Qwen", icon: FiCpu, tone: "text-cyan-300", tokenHint: "Qwen token", useToken: true },
  codex_cli: { id: "codex_cli", name: "Codex CLI", icon: FiCpu, tone: "text-emerald-300", useToken: false },
  tailscale: { id: "tailscale", name: "Tailscale", icon: SiTailscale, tone: "text-blue-300", tokenHint: "Tailscale API key", useToken: true },
  hetzner: { id: "hetzner", name: "Hetzner", icon: FiDatabase, tone: "text-emerald-300", tokenHint: "Hetzner token", useToken: true },
  web3: { id: "web3", name: "Web3", icon: SiWeb3dotjs, tone: "text-fuchsia-300", useToken: false },
  flipper: { id: "flipper", name: "Flipper", icon: FiZap, tone: "text-amber-300", useToken: false }
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

  const [connectorMode, setConnectorMode] = createSignal("platform");
  const [accountLabel, setAccountLabel] = createSignal("");
  const [tokenInput, setTokenInput] = createSignal("");
  const [tailscaleTailnet, setTailscaleTailnet] = createSignal("");
  const [tailscaleAuthKey, setTailscaleAuthKey] = createSignal("");
  const [web3Wallet, setWeb3Wallet] = createSignal("");
  const [flipperDeviceName, setFlipperDeviceName] = createSignal("");
  const [flipperProbeSummary, setFlipperProbeSummary] = createSignal("");
  const [flipperKnownDevices, setFlipperKnownDevices] = createSignal([]);
  const [flipperKnownLoading, setFlipperKnownLoading] = createSignal(false);

  const assistantProvider = createMemo(() => workflowUi().provider || "codex");

  onMount(() => {
    integrationStore.checkAll();
    if (props.preselectProviderId) openProviderDialog(props.preselectProviderId);
  });

  createEffect(() => {
    if (typeof window === "undefined") return;
    const runtime = profileRuntime();
    const value = tailscaleAuthKey().trim();
    if (runtime.mode === "profile" && runtime.profileLoaded) {
      void setProfileSecret("tailscale_auth_key", value);
      localStorage.removeItem("tailscale_auth_key");
      return;
    }
    localStorage.setItem("tailscale_auth_key", value);
  });

  const providers = createMemo(() => integrationStore.list().map((integration) => ({
    ...integration,
    ...providerMeta[integration.id]
  })));

  const filteredProviders = createMemo(() => {
    const q = search().trim().toLowerCase();
    if (!q) return providers();
    return providers().filter((provider) => provider.name.toLowerCase().includes(q) || provider.id.toLowerCase().includes(q));
  });

  const activeProvider = createMemo(() => providers().find((provider) => provider.id === dialogProviderId()) || null);
  const verification = createMemo(() => integrationVerification()[dialogProviderId()] || null);

  function openProviderDialog(providerOrId) {
    const provider = typeof providerOrId === "string"
      ? providers().find((entry) => entry.id === providerOrId)
      : providerOrId;
    if (!provider) return;

    setDialogProviderId(provider.id);
    setStep(2);
    setStatus("");
    setBusy(false);
    setVerifiedForDialog(false);
    setFlipperProbeSummary("");
    setConnectorMode(provider.connectorMode || (provider.supportsPlatformConnector ? "platform" : "user_owned"));
    setAccountLabel(provider.accountLabel || `${provider.name} Session`);

    if (provider.id === "tailscale" && typeof window !== "undefined") {
      const runtime = profileRuntime();
      const apiKey = runtime.mode === "profile" && runtime.profileLoaded
        ? String(getProfileSecret("tailscale_api_key") || "").trim()
        : String(localStorage.getItem("tailscale_api_key") || "").trim();
      const authKey = runtime.mode === "profile" && runtime.profileLoaded
        ? String(getProfileSecret("tailscale_auth_key") || "").trim()
        : String(localStorage.getItem("tailscale_auth_key") || "").trim();
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

    if (provider.tokenKey && typeof window !== "undefined") {
      const runtime = profileRuntime();
      const value = runtime.mode === "profile" && runtime.profileLoaded
        ? String(getProfileSecret(provider.tokenKey) || "").trim()
        : String(localStorage.getItem(provider.tokenKey) || "").trim();
      setTokenInput(value);
    } else {
      setTokenInput("");
    }
  }

  function closeDialog() {
    setDialogProviderId("");
    setStep(1);
    setBusy(false);
    setVerifiedForDialog(false);
    setFlipperProbeSummary("");
  }

  function requiredInputsReady(provider) {
    if (!provider) return false;
    if (connectorMode() === "platform") return true;
    if (provider.id === "tailscale") return tokenInput().trim().length > 8 && tailscaleTailnet().trim().length > 0;
    if (provider.id === "web3") return web3Wallet().trim().startsWith("0x");
    if (provider.id === "flipper") return tokenInput().trim().length > 0;
    if (provider.oauthRedirect) return true;
    if (!provider.useToken) return true;
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
    setBusy(true);

    if (provider.oauthRedirect && connectorMode() === "user_owned") {
      if (typeof window !== "undefined") {
        const returnTo = `${window.location.pathname}${window.location.search || ""}`;
        window.location.assign(`/api/google/oauth/start?returnTo=${encodeURIComponent(returnTo)}`);
      }
      setBusy(false);
      return;
    }

    const result = await integrationStore.verify(provider.id, {
      connectorMode: connectorMode(),
      token: tokenInput().trim(),
      tailnet: tailscaleTailnet().trim(),
      apiKey: tokenInput().trim(),
      authKey: tailscaleAuthKey().trim(),
      wallet: web3Wallet().trim(),
      flipperDeviceId: tokenInput().trim(),
      flipperDeviceName: flipperDeviceName().trim()
    });

    setBusy(false);
    if (!result.ok) {
      setVerifiedForDialog(false);
      setStatus(result.message || "Verification failed.");
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
    setVerifiedForDialog(true);
    setStatus(result.message || "Verification succeeded.");
    setStep(4);
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
        optionalServices: ["device_information", "battery_service"]
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

  async function saveProvider(provider) {
    if (!provider) return;

    const payload = {
      connectorMode: connectorMode(),
      accountLabel: accountLabel().trim() || `${provider.name} Session`
    };

    if (connectorMode() === "user_owned" && provider.useToken) {
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

    const linked = await integrationStore.connect(provider.id, payload);
    if (!linked) {
      setStatus(`Unable to link ${provider.name}. Load profile and retry.`);
      return;
    }
    setStatus(`${provider.name} integration linked.`);
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
      setFlipperProbeSummary(result.message || "Flipper probe failed.");
      return;
    }
    setVerifiedForDialog(true);
    const battery = Number.isFinite(result.batteryLevel) ? `${result.batteryLevel}%` : "n/a";
    const model = String(result.deviceInfo?.model || result.deviceInfo?.hardware || result.deviceName || "").trim() || "unknown";
    const serviceCount = Array.isArray(result.services) ? result.services.length : 0;
    const warnings = Array.isArray(result.diagnostics) ? result.diagnostics : [];
    const warningText = warnings.length > 0 ? ` · ${warnings.join(", ")}` : "";
    setFlipperProbeSummary(`Probe ok · battery ${battery} · model ${model} · services ${serviceCount}${warningText}`);
  }

  function disconnectProvider(provider) {
    if (!provider) return;
    integrationStore.disconnect(provider.id);
    setStatus(`${provider.name} disconnected.`);
    closeDialog();
  }

  function stepClass(target) {
    if (step() === target) return "border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.08)] text-[hsl(var(--primary))]";
    if (step() > target) return "border-emerald-500/40 bg-emerald-500/10 text-emerald-300";
    return "border-neutral-800 bg-neutral-900/60 text-neutral-500";
  }

  function stepFlow(provider) {
    if (!provider) return [1, 2, 3, 4];
    return [2, 3, 4];
  }

  function stepTitle(value) {
    if (value === 2) return "Values";
    if (value === 3) return "Verify";
    return "Success";
  }

  return (
    <div class={`h-full overflow-auto text-neutral-200 ${compact() ? "" : "bg-[#0f1013] p-4"}`}>
      <div class="border-b border-neutral-800 px-3 py-2">
        <h3 class="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-neutral-300">
          <FiLink2 size={18} />
          <span>Integrations</span>
        </h3>
        <p class="mt-1 text-xs text-neutral-400">Icons are providers. Hover for details, click Connect to launch stepper.</p>
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

      <Show when={status() && !activeProvider()}>
        <div class="mx-3 mb-3 rounded-md border border-neutral-800 bg-neutral-900/55 px-2.5 py-2 text-xs text-neutral-300">{status()}</div>
      </Show>

      <div class="grid grid-cols-4 gap-2 px-3 pb-3 sm:grid-cols-5" data-testid="integrations-icon-grid">
        <For each={filteredProviders()}>
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

      <Show when={activeProvider()}>
        {(providerAccessor) => {
          const provider = providerAccessor();
          const lifecycleStatus = () => String(integrationStore.get(provider.id)?.lifecycleStatus || "idle").trim() || "idle";
          const verified = () => integrationStore.isConnected(provider.id) || lifecycleStatus() === "verified" || Boolean(verification()?.ok) || verifiedForDialog();
          const verificationMessage = () => String(
            integrationStore.get(provider.id)?.lifecycleMessage
            || verification()?.message
            || ""
          );
          const ProviderIcon = provider.icon || FiCloud;
          const flow = () => stepFlow(provider);
          const stepIndex = () => Math.max(0, flow().indexOf(step()));
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

                <div class={`mb-3 grid gap-1 ${provider.id === "github" ? "grid-cols-3" : "grid-cols-4"}`}>
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

                <Show when={step() === 2}>
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

                    <Show when={connectorMode() === "user_owned" && provider.useToken}>
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
                    <Show when={provider.id === "github"}>
                      <p class="text-[11px] text-neutral-500">PAT is encrypted and stored in your profile secrets.</p>
                    </Show>

                    <Show when={provider.id === "tailscale" && connectorMode() === "user_owned"}>
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

                    <Show when={provider.oauthRedirect && connectorMode() === "user_owned"}>
                      <p class="text-[11px] text-neutral-500">OAuth integrations redirect on Verify step to complete login.</p>
                    </Show>
                  </section>
                </Show>

                <Show when={step() === 3}>
                  <section class="space-y-2 rounded-md border border-neutral-800 bg-neutral-900/50 p-3" data-testid={provider.id === "tailscale" ? "provider-tailscale-quickstart" : "integration-stepper-verify"}>
                    <p class="text-xs text-neutral-400">Run integration checks before linking.</p>
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
                    <Show when={!requiredInputsReady(provider)}>
                      <p class="text-[11px] text-amber-300">Fill required values in Step 2 first.</p>
                    </Show>
                    <Show when={verificationMessage()}>
                      <p class={`text-[11px] ${verified() ? "text-emerald-300" : lifecycleStatus() === "verifying" ? "text-amber-300" : "text-red-300"}`}>{verificationMessage()}</p>
                    </Show>
                  </section>
                </Show>

                <Show when={step() === 4}>
                  <section class="space-y-2 rounded-md border border-emerald-500/35 bg-emerald-500/10 p-3" data-testid="integration-stepper-success">
                    <p class="flex items-center gap-1 text-xs font-medium text-emerald-200">
                      <FiCheckCircle size={12} />
                      Verification passed
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
                      </div>
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
                    disabled={stepIndex() === flow().length - 1 || (step() === 2 && !requiredInputsReady(provider))}
                  >
                    Next
                    <FiArrowRight size={12} />
                  </button>
                  <button
                    type="button"
                    class={primaryClass}
                    disabled={!verified() || busy()}
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
