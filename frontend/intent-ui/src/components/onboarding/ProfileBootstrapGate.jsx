import { For, Show, createMemo, createSignal } from "solid-js";
import {
  TbOutlineCloud,
  TbOutlineDownload,
  TbOutlineDeviceFloppy,
  TbOutlineFile,
  TbOutlineFingerprint,
  TbOutlineFolderOpen,
  TbOutlineKey,
  TbOutlinePlugConnected,
  TbOutlineShieldLock,
  TbOutlineX
} from "solid-icons/tb";
import { activatePersistentProfileSession } from "../../stores/profile-runtime";
import { setProfileSecretsContext } from "../../stores/profile-secrets";
import { DEFAULT_LOCAL_PROFILE_SCOPES } from "../../lib/oidc-scopes";
import {
  toBase64Url,
  encodeProfileBlob,
  parseProfileBlob,
  derivePassphraseKey,
  encodeProfilePayload,
  decodeProfilePayload,
  saveBlobByBackend,
  loadBlobByBackend
} from "../../lib/profile-blob";

const backendOptions = [
  { id: "browser_local", label: "Browser Local", icon: TbOutlineDeviceFloppy, detail: "Stores encrypted profile blob in browser local storage." },
  { id: "local_file", label: "Profile File", icon: TbOutlineFile, detail: "Load encrypted profile blob from a local file." },
  { id: "google_drive", label: "Google Drive", icon: TbOutlineCloud, detail: "Keeps a local cache now; sync adapter can upload later." },
  { id: "git_repo", label: "Git Repository", icon: TbOutlinePlugConnected, detail: "Keeps a local cache now; git sync adapter can push later." }
];


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
  const [createdProfileId, setCreatedProfileId] = createSignal("");
  const [createdBlobBytes, setCreatedBlobBytes] = createSignal(null);
  const [downloadedCopy, setDownloadedCopy] = createSignal(false);
  const [savedToLocalFolder, setSavedToLocalFolder] = createSignal(false);
  const [localFolderLabel, setLocalFolderLabel] = createSignal("");

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

  const fileSystemAccessReady = createMemo(() => (
    typeof window !== "undefined" &&
    typeof window.showDirectoryPicker === "function"
  ));

  const downloadCreatedProfile = () => {
    const bytes = createdBlobBytes();
    const profileId = createdProfileId();
    if (!bytes || !profileId) {
      setError("No created profile is available to download yet.");
      return;
    }
    const link = document.createElement("a");
    const file = new Blob([bytes], { type: "application/octet-stream" });
    link.href = URL.createObjectURL(file);
    link.download = `${profileId}.edgerun-profile.bin`;
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(link.href);
    setDownloadedCopy(true);
    setStatus("Encrypted profile downloaded. Store it in a secure location.");
  };

  const connectLocalFolderAndSave = async () => {
    const bytes = createdBlobBytes();
    const profileId = createdProfileId();
    if (!bytes || !profileId) {
      setError("Create a profile first.");
      return;
    }
    if (!fileSystemAccessReady()) {
      setError("Browser file system access API is unavailable. Use download for safekeeping.");
      return;
    }
    setBusy(true);
    setError("");
    try {
      const dirHandle = await window.showDirectoryPicker({ mode: "readwrite" });
      const fileHandle = await dirHandle.getFileHandle(`${profileId}.edgerun-profile.bin`, { create: true });
      const writable = await fileHandle.createWritable();
      await writable.write(bytes);
      await writable.close();
      setSavedToLocalFolder(true);
      setLocalFolderLabel(String(dirHandle?.name || "selected folder"));
      setStatus(`Encrypted profile saved to ${String(dirHandle?.name || "selected folder")}.`);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save profile to local folder.");
    } finally {
      setBusy(false);
    }
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
      const payload = {
        version: 2,
        profileId,
        createdUnixMs: Date.now(),
        credentialIdB64url: toBase64Url(credentialId),
        integrations: {}
      };
      const plain = encodeProfilePayload(payload);
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

      activatePersistentProfileSession({ profileId, backend: backend(), grantedScopes: DEFAULT_LOCAL_PROFILE_SCOPES });
      setProfileSecretsContext({
        profileId,
        backend: backend(),
        rpId,
        credentialId,
        salt,
        key,
        integrations: {}
      });
      setCreatedProfileId(profileId);
      setCreatedBlobBytes(blobBytes);
      setDownloadedCopy(false);
      setSavedToLocalFolder(false);
      setLocalFolderLabel("");
      setStatus(`Profile created and unlocked. Download the encrypted profile for safekeeping, then optionally connect a local folder.`);
      setPassphrase("");
      setConfirmPassphrase("");
      setUploadedFile(null);

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
      const plain = await crypto.subtle.decrypt({ name: "AES-GCM", iv: parsed.iv }, key, parsed.ciphertext);
      const payload = decodeProfilePayload(new Uint8Array(plain));

      activatePersistentProfileSession({
        profileId: parsed.profileId,
        backend: backend(),
        grantedScopes: DEFAULT_LOCAL_PROFILE_SCOPES
      });
      setProfileSecretsContext({
        profileId: parsed.profileId,
        backend: backend(),
        rpId: parsed.rpId,
        credentialId: parsed.credentialId,
        salt: parsed.salt,
        key,
        integrations: payload?.integrations && typeof payload.integrations === "object"
          ? payload.integrations
          : {}
      });
      setStatus("Encrypted profile loaded and unlocked.");
      setCreatedProfileId("");
      setCreatedBlobBytes(null);
      setDownloadedCopy(false);
      setSavedToLocalFolder(false);
      setLocalFolderLabel("");
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
              Profile mode unlocks registered devices and integrations. Create a password-protected encrypted profile, then keep a backup.
            </p>
          </div>
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
              setCreatedProfileId("");
              setCreatedBlobBytes(null);
              setDownloadedCopy(false);
              setSavedToLocalFolder(false);
              setLocalFolderLabel("");
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
              setCreatedProfileId("");
              setCreatedBlobBytes(null);
              setDownloadedCopy(false);
              setSavedToLocalFolder(false);
              setLocalFolderLabel("");
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

        <Show when={mode() === "create" && createdProfileId()}>
          <div class="mb-4 rounded-lg border border-amber-500/30 bg-amber-600/10 p-3" data-testid="profile-create-next-steps">
            <p class="text-xs font-medium text-amber-100">Profile created: <span class="font-mono">{createdProfileId()}</span></p>
            <p class="mt-1 text-[11px] text-amber-200/90">
              Recommended: download the encrypted profile and connect a local folder so you always retain access.
            </p>
            <div class="mt-2 flex flex-wrap items-center gap-2">
              <button
                type="button"
                class="inline-flex items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-200 hover:bg-neutral-800"
                onClick={downloadCreatedProfile}
                data-testid="profile-download-created"
              >
                <TbOutlineDownload size={14} />
                {downloadedCopy() ? "Downloaded" : "Download profile"}
              </button>
              <button
                type="button"
                class="inline-flex items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-xs text-neutral-200 hover:bg-neutral-800 disabled:cursor-not-allowed disabled:opacity-60"
                disabled={busy() || !fileSystemAccessReady()}
                onClick={() => {
                  void connectLocalFolderAndSave();
                }}
                data-testid="profile-connect-local-folder"
              >
                <TbOutlineFolderOpen size={14} />
                {savedToLocalFolder() ? `Saved to ${localFolderLabel() || "folder"}` : "Connect local folder"}
              </button>
            </div>
            <Show when={!fileSystemAccessReady()}>
              <p class="mt-2 text-[11px] text-neutral-300">
                Browser file system API unavailable here. Download is still available.
              </p>
            </Show>
          </div>
        </Show>

        <div class="flex flex-wrap items-center justify-end gap-2">
          <button
            type="button"
            class="inline-flex items-center gap-1 rounded-md border border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.16)] px-3 py-1.5 text-xs text-[hsl(var(--primary))] hover:bg-[hsl(var(--primary)/0.25)] disabled:cursor-not-allowed disabled:opacity-60"
            disabled={busy() || (mode() === "create" && !createdProfileId())}
            onClick={() => {
              if (mode() === "create") {
                if (!createdProfileId()) {
                  void createProfile();
                  return;
                }
                setCompleted();
                return;
              }
              void loadProfile();
            }}
            data-testid="profile-bootstrap-submit"
          >
            <TbOutlineKey size={14} />
            {busy()
              ? "Working..."
              : mode() === "create"
                ? createdProfileId() ? "Continue to workspace" : "Create encrypted profile"
                : "Load encrypted profile"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default ProfileBootstrapGate;
