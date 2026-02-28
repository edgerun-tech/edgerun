import { For, Show, createMemo, createSignal } from "solid-js";
import {
  TbOutlineCloud,
  TbOutlineDeviceFloppy,
  TbOutlineFile,
  TbOutlineFingerprint,
  TbOutlineFolderOpen,
  TbOutlineKey,
  TbOutlinePlugConnected,
  TbOutlineShieldLock,
  TbOutlineUserCheck,
  TbOutlineX
} from "solid-icons/tb";
import { activateEphemeralSession, activatePersistentProfileSession } from "../../stores/profile-runtime";
import { DEFAULT_LOCAL_PROFILE_SCOPES } from "../../lib/oidc-scopes";

const PROFILE_BLOB_BROWSER_KEY = "intent-ui-profile-blob-browser-v1";
const PROFILE_BLOB_GOOGLE_KEY = "intent-ui-profile-blob-google-v1";
const PROFILE_BLOB_GIT_KEY = "intent-ui-profile-blob-git-v1";

const MAGIC = [0x45, 0x44, 0x50, 0x52, 0x31];

const backendOptions = [
  { id: "browser_local", label: "Browser Local", icon: TbOutlineDeviceFloppy, detail: "Stores encrypted profile blob in local browser storage." },
  { id: "local_file", label: "Local File", icon: TbOutlineFile, detail: "Exports encrypted profile blob as a local file." },
  { id: "google_drive", label: "Google Drive", icon: TbOutlineCloud, detail: "Keeps a local cache now; sync adapter can upload later." },
  { id: "git_repo", label: "Git Repository", icon: TbOutlinePlugConnected, detail: "Keeps a local cache now; git sync adapter can push later." }
];

const backendStorageKey = (backendId) => {
  if (backendId === "google_drive") return PROFILE_BLOB_GOOGLE_KEY;
  if (backendId === "git_repo") return PROFILE_BLOB_GIT_KEY;
  return PROFILE_BLOB_BROWSER_KEY;
};

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
  return bytes;
};

const writeField = (target, tag, bytes) => {
  const chunk = new Uint8Array(1 + 4 + bytes.length);
  chunk[0] = tag & 0xff;
  const view = new DataView(chunk.buffer);
  view.setUint32(1, bytes.length, false);
  chunk.set(bytes, 5);
  target.push(chunk);
};

const concatBytes = (chunks) => {
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.length;
  }
  return out;
};

const encodeProfileBlob = (input) => {
  const encoder = new TextEncoder();
  const chunks = [new Uint8Array(MAGIC)];
  writeField(chunks, 1, encoder.encode(String(input.profileId || "")));
  writeField(chunks, 2, encoder.encode(String(input.rpId || "")));
  writeField(chunks, 3, input.credentialIdBytes);
  writeField(chunks, 4, input.salt);
  writeField(chunks, 5, input.iv);
  writeField(chunks, 6, input.ciphertext);
  writeField(chunks, 7, encoder.encode(String(input.backend || "")));
  return concatBytes(chunks);
};

const parseProfileBlob = (bytes) => {
  const encoder = new TextDecoder();
  if (!bytes || bytes.length < 5) throw new Error("Profile blob is empty or truncated.");
  for (let i = 0; i < MAGIC.length; i += 1) {
    if (bytes[i] !== MAGIC[i]) throw new Error("Invalid profile blob header.");
  }
  const fields = new Map();
  let cursor = 5;
  while (cursor + 5 <= bytes.length) {
    const tag = bytes[cursor];
    const size = new DataView(bytes.buffer, bytes.byteOffset + cursor + 1, 4).getUint32(0, false);
    cursor += 5;
    if (cursor + size > bytes.length) throw new Error("Profile blob field overrun.");
    fields.set(tag, bytes.slice(cursor, cursor + size));
    cursor += size;
  }

  const profileId = encoder.decode(fields.get(1) || new Uint8Array());
  const rpId = encoder.decode(fields.get(2) || new Uint8Array());
  const credentialId = fields.get(3);
  const salt = fields.get(4);
  const iv = fields.get(5);
  const ciphertext = fields.get(6);
  const backend = encoder.decode(fields.get(7) || new Uint8Array());

  if (!profileId || !rpId || !credentialId || !salt || !iv || !ciphertext) {
    throw new Error("Profile blob missing mandatory encrypted fields.");
  }

  return {
    profileId,
    rpId,
    credentialId,
    salt,
    iv,
    ciphertext,
    backend: backend || "browser_local"
  };
};

const derivePassphraseKey = async (passphrase, saltBytes) => {
  const material = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(String(passphrase || "")),
    { name: "PBKDF2" },
    false,
    ["deriveKey"]
  );
  return crypto.subtle.deriveKey(
    {
      name: "PBKDF2",
      salt: saltBytes,
      iterations: 310000,
      hash: "SHA-256"
    },
    material,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"]
  );
};

const profilePlaintext = (profileId, credentialId) => new TextEncoder().encode(
  `edgerun.profile.v1\nprofile_id=${profileId}\ncreated_unix_ms=${Date.now()}\ncredential_id=${toBase64Url(credentialId)}\n`
);

const saveBlobByBackend = (backend, blobBytes) => {
  const encoded = toBase64Url(blobBytes);
  const key = backendStorageKey(backend);
  localStorage.setItem(key, encoded);
  return encoded;
};

const loadBlobByBackend = async (backend, uploadedFile) => {
  if (backend === "local_file") {
    if (!uploadedFile) throw new Error("Choose a profile file first.");
    const bytes = new Uint8Array(await uploadedFile.arrayBuffer());
    return bytes;
  }
  const value = localStorage.getItem(backendStorageKey(backend));
  if (!value) throw new Error("No encrypted profile blob found for selected backend.");
  return fromBase64Url(value);
};

const enrollYubikeyCredential = async () => {
  const challenge = crypto.getRandomValues(new Uint8Array(32));
  const userId = crypto.getRandomValues(new Uint8Array(16));
  const rpId = window.location.hostname;
  const cred = await navigator.credentials.create({
    publicKey: {
      challenge,
      rp: { id: rpId, name: "EdgeRun Intent UI" },
      user: {
        id: userId,
        name: "edgerun-user",
        displayName: "EdgeRun User"
      },
      pubKeyCredParams: [{ type: "public-key", alg: -7 }, { type: "public-key", alg: -257 }],
      authenticatorSelection: {
        authenticatorAttachment: "cross-platform",
        residentKey: "preferred",
        userVerification: "required"
      },
      timeout: 90000,
      attestation: "none"
    }
  });
  if (!(cred instanceof PublicKeyCredential)) {
    throw new Error("YubiKey credential enrollment cancelled.");
  }
  return {
    credentialId: new Uint8Array(cred.rawId),
    rpId
  };
};

const verifyYubikeyCredential = async (rpId, credentialIdBytes) => {
  const assertion = await navigator.credentials.get({
    publicKey: {
      challenge: crypto.getRandomValues(new Uint8Array(32)),
      timeout: 90000,
      userVerification: "required",
      rpId,
      allowCredentials: [{ id: credentialIdBytes, type: "public-key", transports: ["usb", "nfc", "ble"] }]
    }
  });
  if (!assertion) {
    throw new Error("YubiKey verification was cancelled.");
  }
};

function ProfileBootstrapGate(props) {
  const [mode, setMode] = createSignal("create");
  const [backend, setBackend] = createSignal("browser_local");
  const [passphrase, setPassphrase] = createSignal("");
  const [confirmPassphrase, setConfirmPassphrase] = createSignal("");
  const [uploadedFile, setUploadedFile] = createSignal(null);
  const [busy, setBusy] = createSignal(false);
  const [status, setStatus] = createSignal("");
  const [error, setError] = createSignal("");

  const webauthnReady = createMemo(() => (
    typeof window !== "undefined" &&
    window.isSecureContext &&
    typeof window.PublicKeyCredential !== "undefined" &&
    "credentials" in navigator
  ));

  const setCompleted = () => {
    props.onComplete?.();
  };

  const dismissGate = () => {
    props.onDismiss?.();
  };

  const proceedEphemeral = () => {
    activateEphemeralSession();
    setCompleted();
  };

  const createProfile = async () => {
    if (!webauthnReady()) {
      setError("WebAuthn security-key flow is unavailable in this browser/context.");
      return;
    }
    if (!passphrase().trim()) {
      setError("Recovery passphrase is required.");
      return;
    }
    if (passphrase() !== confirmPassphrase()) {
      setError("Recovery passphrase confirmation does not match.");
      return;
    }

    setBusy(true);
    setStatus("");
    setError("");
    try {
      const profileId = `profile_${crypto.randomUUID()}`;
      const { credentialId, rpId } = await enrollYubikeyCredential();

      const salt = crypto.getRandomValues(new Uint8Array(16));
      const iv = crypto.getRandomValues(new Uint8Array(12));
      const key = await derivePassphraseKey(passphrase(), salt);
      const plain = profilePlaintext(profileId, credentialId);
      const ciphertext = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, plain));

      const blobBytes = encodeProfileBlob({
        profileId,
        rpId,
        credentialIdBytes: credentialId,
        salt,
        iv,
        ciphertext,
        backend: backend()
      });

      const encoded = saveBlobByBackend(backend(), blobBytes);
      if (backend() === "local_file") {
        const link = document.createElement("a");
        const file = new Blob([blobBytes], { type: "application/octet-stream" });
        link.href = URL.createObjectURL(file);
        link.download = `${profileId}.edgerun-profile.bin`;
        document.body.appendChild(link);
        link.click();
        link.remove();
        URL.revokeObjectURL(link.href);
      }

      activatePersistentProfileSession({ profileId, backend: backend(), grantedScopes: DEFAULT_LOCAL_PROFILE_SCOPES });
      setStatus(
        backend() === "local_file"
          ? "Profile created and exported as encrypted file."
          : `Profile created and encrypted blob stored (${backend()}).`
      );
      setPassphrase("");
      setConfirmPassphrase("");
      setUploadedFile(null);
      setCompleted();

      if (backend() === "google_drive" || backend() === "git_repo") {
        localStorage.setItem("intent-ui-profile-sync-pending-v1", `${backend()}:${encoded.slice(0, 16)}`);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create encrypted profile.");
    } finally {
      setBusy(false);
    }
  };

  const loadProfile = async () => {
    if (!webauthnReady()) {
      setError("WebAuthn security-key flow is unavailable in this browser/context.");
      return;
    }
    if (!passphrase().trim()) {
      setError("Recovery passphrase is required.");
      return;
    }

    setBusy(true);
    setStatus("");
    setError("");
    try {
      const blob = await loadBlobByBackend(backend(), uploadedFile());
      const parsed = parseProfileBlob(blob);
      await verifyYubikeyCredential(parsed.rpId, parsed.credentialId);

      const key = await derivePassphraseKey(passphrase(), parsed.salt);
      await crypto.subtle.decrypt({ name: "AES-GCM", iv: parsed.iv }, key, parsed.ciphertext);

      activatePersistentProfileSession({
        profileId: parsed.profileId,
        backend: backend(),
        grantedScopes: DEFAULT_LOCAL_PROFILE_SCOPES
      });
      setStatus("Encrypted profile loaded and unlocked.");
      setPassphrase("");
      setConfirmPassphrase("");
      setUploadedFile(null);
      setCompleted();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load encrypted profile.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div class="fixed inset-0 z-[15000] flex items-center justify-center bg-black/75 px-4 py-6 backdrop-blur-sm" data-testid="profile-bootstrap-gate">
      <div class="w-full max-w-2xl rounded-2xl border border-neutral-700 bg-[#0d1016] p-5 shadow-2xl">
        <div class="mb-4 flex items-start justify-between gap-3">
          <div>
            <p class="text-[11px] uppercase tracking-wide text-neutral-400">EdgeRun Intent UI</p>
            <h1 class="mt-1 text-xl font-semibold text-neutral-100">Load or create profile</h1>
            <p class="mt-1 text-sm text-neutral-400">
              Persistent mode unlocks registered devices. Ephemeral mode runs local session state only.
            </p>
          </div>
          <button
            type="button"
            class="inline-flex items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-200 hover:bg-neutral-800"
            onClick={proceedEphemeral}
            data-testid="profile-bootstrap-ephemeral"
          >
            <TbOutlineUserCheck size={15} />
            Proceed without profile
          </button>
        </div>
        <Show when={Boolean(props.allowDismiss)}>
          <div class="mb-4 flex justify-end">
            <button
              type="button"
              class="inline-flex items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-1.5 text-xs text-neutral-300 hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
              onClick={dismissGate}
              data-testid="profile-bootstrap-dismiss"
            >
              <TbOutlineX size={14} />
              Close
            </button>
          </div>
        </Show>

        <div class="mb-4 grid grid-cols-2 gap-2 rounded-lg border border-neutral-800 bg-neutral-900/45 p-1">
          <button
            type="button"
            class={`rounded-md px-3 py-2 text-xs ${mode() === "create" ? "bg-[hsl(var(--primary)/0.2)] text-[hsl(var(--primary))]" : "text-neutral-300 hover:bg-neutral-800"}`}
            onClick={() => {
              setMode("create");
              setError("");
              setStatus("");
            }}
            data-testid="profile-bootstrap-tab-create"
          >
            Create profile
          </button>
          <button
            type="button"
            class={`rounded-md px-3 py-2 text-xs ${mode() === "load" ? "bg-[hsl(var(--primary)/0.2)] text-[hsl(var(--primary))]" : "text-neutral-300 hover:bg-neutral-800"}`}
            onClick={() => {
              setMode("load");
              setError("");
              setStatus("");
            }}
            data-testid="profile-bootstrap-tab-load"
          >
            Load profile
          </button>
        </div>

        <div class="mb-4 rounded-lg border border-neutral-800 bg-neutral-900/40 p-3">
          <p class="mb-2 text-xs font-medium uppercase tracking-wide text-neutral-400">Storage Backend</p>
          <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
            <For each={backendOptions}>
              {(option) => {
                const Icon = option.icon;
                return (
                  <button
                    type="button"
                    onClick={() => setBackend(option.id)}
                    class={`rounded-md border px-2.5 py-2 text-left ${backend() === option.id ? "border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.14)]" : "border-neutral-700 bg-neutral-900 hover:border-neutral-500"}`}
                    data-testid={`profile-backend-${option.id}`}
                  >
                    <p class="flex items-center gap-2 text-xs font-medium text-neutral-100"><Icon size={14} /> {option.label}</p>
                    <p class="mt-1 text-[11px] text-neutral-500">{option.detail}</p>
                  </button>
                );
              }}
            </For>
          </div>
        </div>

        <div class="mb-4 rounded-lg border border-neutral-800 bg-neutral-900/40 p-3">
          <p class="mb-2 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-neutral-400">
            <TbOutlineShieldLock size={14} />
            Auth and Encryption
          </p>
          <p class="mb-3 text-[11px] text-neutral-500">
            YubiKey WebAuthn verifies profile owner. Recovery passphrase decrypts encrypted profile content.
          </p>

          <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
            <div class="sm:col-span-2 rounded-md border border-neutral-700 bg-neutral-900/60 px-2.5 py-2 text-[11px] text-neutral-300">
              <p class="flex items-center gap-2">
                <TbOutlineFingerprint size={14} class="text-neutral-400" />
                WebAuthn security key: {webauthnReady() ? "available" : "unavailable"}
              </p>
            </div>
            <label class="text-xs text-neutral-300" for="profile-passphrase">Recovery passphrase</label>
            <input
              id="profile-passphrase"
              type="password"
              value={passphrase()}
              onInput={(event) => setPassphrase(event.currentTarget.value)}
              placeholder="Enter passphrase"
              class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none"
              data-testid="profile-passphrase"
            />
            <Show when={mode() === "create"}>
              <>
                <label class="text-xs text-neutral-300" for="profile-passphrase-confirm">Confirm passphrase</label>
                <input
                  id="profile-passphrase-confirm"
                  type="password"
                  value={confirmPassphrase()}
                  onInput={(event) => setConfirmPassphrase(event.currentTarget.value)}
                  placeholder="Re-enter passphrase"
                  class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-sm text-neutral-200 outline-none"
                  data-testid="profile-passphrase-confirm"
                />
              </>
            </Show>
            <Show when={mode() === "load" && backend() === "local_file"}>
              <>
                <label class="text-xs text-neutral-300" for="profile-file">Profile file</label>
                <input
                  id="profile-file"
                  type="file"
                  accept=".bin,.edgerun-profile,.edgerun-profile.bin,application/octet-stream"
                  class="rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-2 text-xs text-neutral-300"
                  onInput={(event) => setUploadedFile(event.currentTarget.files?.[0] || null)}
                  data-testid="profile-file-input"
                />
              </>
            </Show>
          </div>
        </div>

        <Show when={status()}>
          <p class="mb-3 rounded-md border border-emerald-500/30 bg-emerald-600/10 px-3 py-2 text-xs text-emerald-200">{status()}</p>
        </Show>
        <Show when={error()}>
          <p class="mb-3 rounded-md border border-red-500/30 bg-red-600/10 px-3 py-2 text-xs text-red-200">{error()}</p>
        </Show>

        <div class="flex flex-wrap items-center justify-end gap-2">
          <button
            type="button"
            class="inline-flex items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-200 hover:bg-neutral-800"
            onClick={proceedEphemeral}
          >
            <TbOutlineFolderOpen size={14} />
            Ephemeral session
          </button>
          <button
            type="button"
            class="inline-flex items-center gap-1 rounded-md border border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.16)] px-3 py-1.5 text-xs text-[hsl(var(--primary))] hover:bg-[hsl(var(--primary)/0.25)] disabled:cursor-not-allowed disabled:opacity-60"
            disabled={busy()}
            onClick={() => {
              if (mode() === "create") {
                void createProfile();
                return;
              }
              void loadProfile();
            }}
            data-testid="profile-bootstrap-submit"
          >
            <TbOutlineKey size={14} />
            {busy() ? "Working..." : mode() === "create" ? "Create encrypted profile" : "Load encrypted profile"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default ProfileBootstrapGate;
