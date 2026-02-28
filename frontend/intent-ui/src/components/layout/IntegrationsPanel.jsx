import { For, Show, createMemo, createSignal, onMount } from "solid-js";
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
  FiShield
} from "solid-icons/fi";
import { integrationStore } from "../../stores/integrations";
import { openWindow } from "../../stores/windows";
import { setAssistantProvider, workflowUi } from "../../stores/workflow-ui";

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

    const token = tokenInput().trim();
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
