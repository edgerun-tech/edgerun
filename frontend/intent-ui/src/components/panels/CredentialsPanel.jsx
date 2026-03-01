import { For, Show, createEffect, createMemo, createSignal, onMount } from "solid-js";
import {
  TbOutlineFingerprint,
  TbOutlineKey,
  TbOutlineLock,
  TbOutlinePlus,
  TbOutlineRefresh,
  TbOutlineTrash,
  TbOutlineShieldCheck,
  TbOutlineAlertCircle
} from "solid-icons/tb";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import { localBridgeHttpUrl } from "../../lib/local-bridge-origin";

function cn(...classes) {
  return twMerge(clsx(classes));
}

const PASSKEY_ID_STORAGE_KEY = "intent-ui-credentials-passkey-id-v1";
const SESSION_UNLOCK_KEY = "intent-ui-credentials-unlocked-v1";

const typeOptions = [
  { id: "password", label: "Password" },
  { id: "api_key", label: "API Key" },
  { id: "oidc_key", label: "OIDC Key" },
  { id: "token", label: "Token" },
  { id: "secret", label: "Secret" },
  { id: "ssh_key", label: "SSH Key" },
  { id: "mtls_cert", label: "mTLS Cert" },
  { id: "gpg_key", label: "GPG Key" }
];

const toBase64Url = (buffer) => {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  for (let i = 0; i < bytes.length; i += 1) binary += String.fromCharCode(bytes[i]);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
};

const fromBase64Url = (value) => {
  const base64 = String(value || "").replace(/-/g, "+").replace(/_/g, "/");
  const padded = `${base64}${"=".repeat((4 - (base64.length % 4 || 4)) % 4)}`;
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes.buffer;
};

function CredentialsPanel(props) {
  const compact = () => Boolean(props?.compact);
  const [unlocked, setUnlocked] = createSignal(false);
  const [platformAuthenticatorAvailable, setPlatformAuthenticatorAvailable] = createSignal(false);
  const [secureContext, setSecureContext] = createSignal(false);
  const [loading, setLoading] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [error, setError] = createSignal("");
  const [entries, setEntries] = createSignal([]);
  const [search, setSearch] = createSignal("");
  const [typeFilter, setTypeFilter] = createSignal("all");
  const [status, setStatus] = createSignal({ installed: true, locked: true, count: 0 });
  const [form, setForm] = createSignal({
    credentialType: "password",
    name: "",
    username: "",
    secret: "",
    url: "",
    note: "",
    tags: "",
    folder: ""
  });

  const webAuthnSupported = createMemo(() =>
    typeof window !== "undefined" &&
    "credentials" in navigator &&
    typeof window.PublicKeyCredential !== "undefined"
  );
  const authProbeLabel = createMemo(() => {
    if (!webAuthnSupported()) return "webAuthn-unsupported";
    if (!secureContext()) return "insecure-context";
    return platformAuthenticatorAvailable() ? "platform-available" : "platform-not-detected";
  });

  const hasPasskey = createMemo(() => {
    if (typeof window === "undefined") return false;
    return Boolean(localStorage.getItem(PASSKEY_ID_STORAGE_KEY));
  });

  const filteredEntries = createMemo(() => {
    const query = search().trim().toLowerCase();
    return entries().filter((item) => {
      if (typeFilter() !== "all" && item.credentialType !== typeFilter()) return false;
      if (!query) return true;
      return [item.name, item.username, item.url, item.tags, item.folder, item.credentialType]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(query));
    });
  });

  const refreshStatus = async () => {
    try {
      const response = await fetch(localBridgeHttpUrl("/v1/local/credentials/status"), { cache: "no-store" });
      const payload = await response.json().catch(() => ({}));
      if (!response.ok || payload?.ok === false) {
        throw new Error(payload?.error || "Failed to read credentials status.");
      }
      setStatus({
        installed: Boolean(payload.installed),
        locked: Boolean(payload.locked),
        count: Number(payload.count || 0)
      });
      return {
        installed: Boolean(payload.installed),
        locked: Boolean(payload.locked),
        count: Number(payload.count || 0)
      };
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to read credentials status.");
      return null;
    }
  };

  const fetchEntries = async () => {
    setLoading(true);
    setError("");
    try {
      const response = await fetch(localBridgeHttpUrl("/v1/local/credentials/list"), { cache: "no-store" });
      const payload = await response.json().catch(() => ({}));
      if (!response.ok || payload?.ok === false) {
        if (payload?.locked) {
          setStatus((prev) => ({ ...prev, locked: true }));
          throw new Error("Vault is locked. Unlock to manage credentials.");
        }
        throw new Error(payload?.error || "Failed to load credentials.");
      }
      setEntries(Array.isArray(payload.entries) ? payload.entries.map(normalizeEntry) : []);
      setStatus((prev) => ({ ...prev, locked: false, count: Number(payload.count || 0) }));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load credentials.");
    } finally {
      setLoading(false);
    }
  };

  const enrollPasskey = async () => {
    if (!webAuthnSupported()) {
      setError("Fingerprint auth is not available in this browser.");
      return;
    }
    setError("");
    setMessage("");
    try {
      const challenge = crypto.getRandomValues(new Uint8Array(32));
      const userId = crypto.getRandomValues(new Uint8Array(16));
      const credential = await navigator.credentials.create({
        publicKey: {
          challenge,
          rp: { name: "IntentUI Credentials", id: window.location.hostname },
          user: {
            id: userId,
            name: "intentui-user",
            displayName: "IntentUI User"
          },
          pubKeyCredParams: [{ type: "public-key", alg: -7 }, { type: "public-key", alg: -257 }],
          authenticatorSelection: {
            residentKey: "preferred",
            userVerification: "preferred"
          },
          timeout: 60000,
          attestation: "none"
        }
      });
      if (!credential || !(credential instanceof PublicKeyCredential)) {
        throw new Error("Fingerprint registration was cancelled.");
      }
      const id = toBase64Url(credential.rawId);
      localStorage.setItem(PASSKEY_ID_STORAGE_KEY, id);
      setMessage("Fingerprint registration completed.");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Fingerprint registration failed.");
    }
  };

  const unlockWithFingerprint = async () => {
    if (!webAuthnSupported()) {
      setError("Fingerprint auth is not available in this browser.");
      return;
    }
    const credentialId = localStorage.getItem(PASSKEY_ID_STORAGE_KEY);
    if (!credentialId) {
      await enrollPasskey();
      return;
    }
    setError("");
    setMessage("");
    try {
      const assertion = await navigator.credentials.get({
        publicKey: {
          challenge: crypto.getRandomValues(new Uint8Array(32)),
          timeout: 60000,
          userVerification: "preferred",
          allowCredentials: [
            {
              id: fromBase64Url(credentialId),
              type: "public-key"
            }
          ]
        }
      });
      if (!assertion) {
        throw new Error("Fingerprint verification cancelled.");
      }
      const unlockRes = await fetch(localBridgeHttpUrl("/v1/local/credentials/unlock"), {
        method: "POST",
        headers: { "content-type": "application/json; charset=utf-8" },
        body: JSON.stringify({ reason: "biometric" })
      });
      const unlockPayload = await unlockRes.json().catch(() => ({}));
      if (!unlockRes.ok || unlockPayload?.ok === false) {
        throw new Error(unlockPayload?.error || "Vault unlock failed.");
      }
      setUnlocked(true);
      sessionStorage.setItem(SESSION_UNLOCK_KEY, "1");
      setMessage("Vault unlocked. Credentials management enabled.");
      await fetchEntries();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Fingerprint verification failed.");
    }
  };

  const lockVault = async () => {
    try {
      await fetch(localBridgeHttpUrl("/v1/local/credentials/lock"), { method: "POST" });
    } finally {
      sessionStorage.removeItem(SESSION_UNLOCK_KEY);
      setUnlocked(false);
      setEntries([]);
      setStatus((prev) => ({ ...prev, locked: true }));
      setMessage("Vault locked.");
    }
  };

  const resetForm = () => {
    setForm({
      credentialType: "password",
      name: "",
      username: "",
      secret: "",
      url: "",
      note: "",
      tags: "",
      folder: ""
    });
  };

  const saveCredential = async () => {
    const value = form();
    if (!value.name.trim()) {
      setError("Credential label is required.");
      return;
    }
    setSaving(true);
    setError("");
    setMessage("");
    try {
      const response = await fetch(localBridgeHttpUrl("/v1/local/credentials/store"), {
        method: "POST",
        headers: { "content-type": "application/json; charset=utf-8" },
        body: JSON.stringify({
          credentialType: value.credentialType,
          name: value.name,
          username: value.username,
          secret: value.secret,
          url: value.url,
          note: value.note,
          tags: value.tags,
          folder: value.folder
        })
      });
      const payload = await response.json().catch(() => ({}));
      if (!response.ok || payload?.ok === false) {
        throw new Error(payload?.error || "Failed to save credential.");
      }
      setMessage("Credential saved.");
      resetForm();
      await fetchEntries();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save credential.");
    } finally {
      setSaving(false);
    }
  };

  const deleteCredential = async (entryId) => {
    if (!entryId) return;
    setError("");
    setMessage("");
    try {
      const response = await fetch(localBridgeHttpUrl("/v1/local/credentials/delete"), {
        method: "POST",
        headers: { "content-type": "application/json; charset=utf-8" },
        body: JSON.stringify({ entryId })
      });
      const payload = await response.json().catch(() => ({}));
      if (!response.ok || payload?.ok === false) {
        throw new Error(payload?.error || "Failed to delete credential.");
      }
      setMessage("Credential removed.");
      await fetchEntries();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete credential.");
    }
  };

  onMount(async () => {
    setSecureContext(Boolean(window.isSecureContext));
    if (webAuthnSupported() && typeof window.PublicKeyCredential?.isUserVerifyingPlatformAuthenticatorAvailable === "function") {
      try {
        const available = await window.PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
        setPlatformAuthenticatorAvailable(Boolean(available));
      } catch {
        setPlatformAuthenticatorAvailable(false);
      }
    }
    const statusSnapshot = await refreshStatus();
    if (typeof window !== "undefined" && sessionStorage.getItem(SESSION_UNLOCK_KEY) === "1") {
      setUnlocked(true);
      await fetchEntries();
      return;
    }
    if (statusSnapshot && !statusSnapshot.locked) {
      setUnlocked(true);
      await fetchEntries();
    }
  });

  createEffect(() => {
    if (!unlocked()) return;
    refreshStatus();
  });

  return (
    <div class={cn("flex h-full min-h-0 flex-col gap-3", compact() ? "text-xs" : "text-sm")}> 
      <div class="flex items-center justify-between rounded-lg border border-neutral-800 bg-neutral-900/60 px-3 py-2">
        <div>
          <p class="text-xs font-medium uppercase tracking-wide text-neutral-400">Credentials Vault</p>
          <p class="text-[11px] text-neutral-500">Passwords, API keys, OIDC keys and typed secrets via hwvault.</p>
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="rounded-md border border-neutral-700 bg-neutral-900 p-1.5 text-neutral-300 hover:bg-neutral-800"
            onClick={refreshStatus}
            title="Refresh status"
          >
            <TbOutlineRefresh size={15} />
          </button>
          <Show when={unlocked()}>
            <button
              type="button"
              class="rounded-md border border-red-500/40 bg-red-600/10 p-1.5 text-red-300 hover:bg-red-600/20"
              onClick={lockVault}
              title="Lock vault"
            >
              <TbOutlineLock size={15} />
            </button>
          </Show>
        </div>
      </div>

      <Show when={message()}>
        <p class="rounded-md border border-emerald-500/30 bg-emerald-600/10 px-3 py-2 text-xs text-emerald-200">{message()}</p>
      </Show>
      <Show when={error()}>
        <p class="rounded-md border border-red-500/30 bg-red-600/10 px-3 py-2 text-xs text-red-200">{error()}</p>
      </Show>

      <Show when={!status().installed}>
        <div class="rounded-lg border border-red-500/30 bg-red-600/10 px-3 py-3 text-xs text-red-200">
          `hwvault` binary not found. Set `HWVAULT_BIN` on server.
        </div>
      </Show>

      <Show when={!unlocked()} fallback={
        <>
          <div class="grid grid-cols-1 gap-2 sm:grid-cols-[minmax(0,1fr)_auto_auto]">
            <input
              value={search()}
              onInput={(event) => setSearch(event.currentTarget.value)}
              placeholder="Search credentials..."
              class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none"
            />
            <select
              value={typeFilter()}
              onChange={(event) => setTypeFilter(event.currentTarget.value)}
              class="rounded-md border border-neutral-700 bg-neutral-900 px-2 py-2 text-sm text-neutral-200 outline-none"
            >
              <option value="all">All types</option>
              <For each={typeOptions}>{(option) => <option value={option.id}>{option.label}</option>}</For>
            </select>
            <button
              type="button"
              onClick={fetchEntries}
              class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 hover:bg-neutral-800"
            >
              Reload
            </button>
          </div>

          <div class="rounded-lg border border-neutral-800 bg-neutral-900/50 p-3">
            <p class="mb-2 text-xs font-medium uppercase tracking-wide text-neutral-400">Add Credential</p>
            <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
              <select
                value={form().credentialType}
                onChange={(event) => setForm((prev) => ({ ...prev, credentialType: event.currentTarget.value }))}
                class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none"
              >
                <For each={typeOptions}>{(option) => <option value={option.id}>{option.label}</option>}</For>
              </select>
              <input
                value={form().name}
                onInput={(event) => setForm((prev) => ({ ...prev, name: event.currentTarget.value }))}
                placeholder="Label"
                class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none"
              />
              <input
                value={form().username}
                onInput={(event) => setForm((prev) => ({ ...prev, username: event.currentTarget.value }))}
                placeholder="Account / username"
                class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none"
              />
              <input
                type="password"
                value={form().secret}
                onInput={(event) => setForm((prev) => ({ ...prev, secret: event.currentTarget.value }))}
                placeholder="Secret"
                class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none"
              />
              <input
                value={form().url}
                onInput={(event) => setForm((prev) => ({ ...prev, url: event.currentTarget.value }))}
                placeholder="URL"
                class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none"
              />
              <input
                value={form().folder}
                onInput={(event) => setForm((prev) => ({ ...prev, folder: event.currentTarget.value }))}
                placeholder="Folder"
                class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none"
              />
              <input
                value={form().tags}
                onInput={(event) => setForm((prev) => ({ ...prev, tags: event.currentTarget.value }))}
                placeholder="Tags (comma separated)"
                class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none sm:col-span-2"
              />
              <textarea
                value={form().note}
                onInput={(event) => setForm((prev) => ({ ...prev, note: event.currentTarget.value }))}
                placeholder="Note"
                rows={2}
                class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none sm:col-span-2"
              />
            </div>
            <div class="mt-2 flex justify-end">
              <button
                type="button"
                onClick={saveCredential}
                disabled={saving()}
                class="inline-flex items-center gap-1 rounded-md border border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.18)] px-2.5 py-1.5 text-xs text-[hsl(var(--primary))] hover:bg-[hsl(var(--primary)/0.26)] disabled:cursor-not-allowed disabled:opacity-60"
              >
                <TbOutlinePlus size={14} />
                {saving() ? "Saving..." : "Add"}
              </button>
            </div>
          </div>

          <div class="min-h-0 flex-1 overflow-auto rounded-lg border border-neutral-800 bg-neutral-900/40 p-2">
            <Show when={!loading()} fallback={<p class="px-2 py-1 text-xs text-neutral-500">Loading credentials...</p>}>
              <Show when={filteredEntries().length > 0} fallback={<p class="px-2 py-1 text-xs text-neutral-500">No credentials loaded.</p>}>
                <div class="space-y-1.5">
                  <For each={filteredEntries()}>
                    {(entry) => (
                      <div class="rounded-md border border-neutral-800 bg-neutral-900/70 px-2 py-2">
                        <div class="flex items-center justify-between gap-2">
                          <div class="min-w-0">
                            <p class="truncate text-xs font-medium text-neutral-200">{entry.name || entry.entryId}</p>
                            <p class="truncate text-[11px] text-neutral-500">{entry.credentialType} · {entry.username || "no-account"}</p>
                          </div>
                          <button
                            type="button"
                            onClick={() => deleteCredential(entry.entryId)}
                            class="rounded-md border border-red-500/35 bg-red-600/10 p-1 text-red-300 hover:bg-red-600/20"
                            title="Delete credential"
                          >
                            <TbOutlineTrash size={13} />
                          </button>
                        </div>
                        <div class="mt-1.5 flex flex-wrap gap-1 text-[10px] text-neutral-500">
                          <Show when={entry.url}><span class="rounded border border-neutral-700 px-1 py-0.5">{entry.url}</span></Show>
                          <Show when={entry.folder}><span class="rounded border border-neutral-700 px-1 py-0.5">{entry.folder}</span></Show>
                          <Show when={entry.tags}><span class="rounded border border-neutral-700 px-1 py-0.5">{entry.tags}</span></Show>
                        </div>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
            </Show>
          </div>
        </>
      }>
        <div class="rounded-lg border border-neutral-800 bg-neutral-900/65 p-4">
          <div class="mb-3 flex items-center gap-2">
            <TbOutlineShieldCheck size={18} class="text-emerald-300" />
            <p class="text-sm font-medium text-neutral-100">Biometric Unlock Required</p>
          </div>
          <p class="text-xs text-neutral-400">
            Use fingerprint auth before managing passwords, API keys, OIDC keys, and other auth artifacts.
          </p>
          <div class="mt-3 space-y-2 text-xs text-neutral-500">
            <p class="flex items-center gap-2"><TbOutlineKey size={14} class="text-neutral-400" /> Vault entries: {status().count}</p>
            <p class="flex items-center gap-2"><TbOutlineAlertCircle size={14} class="text-neutral-400" /> Host: {typeof window !== "undefined" ? window.location.host : "unknown"}</p>
            <p class="flex items-center gap-2"><TbOutlineAlertCircle size={14} class="text-neutral-400" /> Secure context: {secureContext() ? "yes" : "no"}</p>
            <p class="flex items-center gap-2"><TbOutlineAlertCircle size={14} class="text-neutral-400" /> Auth probe: {authProbeLabel()}</p>
          </div>
          <div class="mt-4 flex flex-wrap items-center gap-2">
            <button
              type="button"
              onClick={unlockWithFingerprint}
              class="inline-flex items-center gap-1 rounded-md border border-emerald-500/45 bg-emerald-600/15 px-3 py-1.5 text-xs text-emerald-100 hover:bg-emerald-600/25"
            >
              <TbOutlineFingerprint size={15} />
              Unlock with passkey
            </button>
            <Show when={webAuthnSupported() && !hasPasskey()}>
              <button
                type="button"
                onClick={enrollPasskey}
                class="inline-flex items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-200 hover:bg-neutral-800"
              >
                <TbOutlinePlus size={14} />
                Enroll fingerprint
              </button>
            </Show>
          </div>
        </div>
      </Show>
    </div>
  );
}

export default CredentialsPanel;
  const normalizeEntry = (entry) => ({
    ...entry,
    entryId: String(entry?.entryId || entry?.entry_id || "").trim(),
    credentialType: String(entry?.credentialType || entry?.credential_type || "secret").trim()
  });
