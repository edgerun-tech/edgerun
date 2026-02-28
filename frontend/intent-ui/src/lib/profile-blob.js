const PROFILE_BLOB_BROWSER_KEY = "intent-ui-profile-blob-browser-v1";
const PROFILE_BLOB_GOOGLE_KEY = "intent-ui-profile-blob-google-v1";
const PROFILE_BLOB_GIT_KEY = "intent-ui-profile-blob-git-v1";

const MAGIC = [0x45, 0x44, 0x50, 0x52, 0x31];

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
  const decoder = new TextDecoder();
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
  const profileId = decoder.decode(fields.get(1) || new Uint8Array());
  const rpId = decoder.decode(fields.get(2) || new Uint8Array());
  const credentialId = fields.get(3);
  const salt = fields.get(4);
  const iv = fields.get(5);
  const ciphertext = fields.get(6);
  const backend = decoder.decode(fields.get(7) || new Uint8Array());
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

const encodeProfilePayload = (payload) => new TextEncoder().encode(JSON.stringify(payload));

const decodeProfilePayload = (plainBytes) => {
  const text = new TextDecoder().decode(plainBytes);
  try {
    const payload = JSON.parse(text);
    if (payload && typeof payload === "object") return payload;
  } catch {
    // legacy/plaintext format fallback
  }
  const lines = text.split("\n").map((line) => line.trim()).filter(Boolean);
  const kv = new Map();
  for (const line of lines) {
    const idx = line.indexOf("=");
    if (idx <= 0) continue;
    kv.set(line.slice(0, idx), line.slice(idx + 1));
  }
  return {
    version: 1,
    profileId: kv.get("profile_id") || "",
    credentialIdB64url: kv.get("credential_id") || "",
    integrations: {}
  };
};

const saveBlobByBackend = (backend, blobBytes) => {
  const encoded = toBase64Url(blobBytes);
  const key = backendStorageKey(backend);
  localStorage.setItem(key, encoded);
  return encoded;
};

const loadBlobByBackend = async (backend, uploadedFile) => {
  if (backend === "local_file") {
    if (!uploadedFile) throw new Error("Choose a profile file first.");
    return new Uint8Array(await uploadedFile.arrayBuffer());
  }
  const value = localStorage.getItem(backendStorageKey(backend));
  if (!value) throw new Error("No encrypted profile blob found for selected backend.");
  return fromBase64Url(value);
};

export {
  toBase64Url,
  fromBase64Url,
  backendStorageKey,
  encodeProfileBlob,
  parseProfileBlob,
  derivePassphraseKey,
  encodeProfilePayload,
  decodeProfilePayload,
  saveBlobByBackend,
  loadBlobByBackend
};
