import { createSignal } from "solid-js";
import { profileRuntime } from "./profile-runtime";
import { encodeProfileBlob, encodeProfilePayload, saveBlobByBackend } from "../lib/profile-blob";

const [profileSecretState, setProfileSecretState] = createSignal({
  profileId: "",
  backend: "",
  rpId: "",
  credentialId: null,
  salt: null,
  key: null,
  integrations: {}
});

function clearProfileSecretsContext() {
  setProfileSecretState({
    profileId: "",
    backend: "",
    rpId: "",
    credentialId: null,
    salt: null,
    key: null,
    integrations: {}
  });
}

function setProfileSecretsContext(input) {
  setProfileSecretState({
    profileId: String(input?.profileId || "").trim(),
    backend: String(input?.backend || "").trim() || "browser_local",
    rpId: String(input?.rpId || "").trim(),
    credentialId: input?.credentialId || null,
    salt: input?.salt || null,
    key: input?.key || null,
    integrations: { ...(input?.integrations || {}) }
  });
}

function getProfileSecret(key) {
  const state = profileSecretState();
  return String(state.integrations?.[key] || "");
}

function hasActiveProfileSecretsContext() {
  const state = profileSecretState();
  return Boolean(
    state.profileId &&
    state.backend &&
    state.rpId &&
    state.credentialId &&
    state.salt &&
    state.key
  );
}

async function persistProfileSecrets(nextIntegrations) {
  const runtime = profileRuntime();
  if (runtime.mode !== "profile" || !runtime.profileLoaded) return false;
  const state = profileSecretState();
  if (!state.key || !state.credentialId || !state.salt || !state.rpId || !state.profileId) return false;
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const payload = {
    version: 2,
    profileId: state.profileId,
    createdUnixMs: Date.now(),
    credentialIdB64url: "",
    integrations: nextIntegrations
  };
  const plain = encodeProfilePayload(payload);
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, state.key, plain));
  const blobBytes = encodeProfileBlob({
    profileId: state.profileId,
    rpId: state.rpId,
    credentialIdBytes: state.credentialId,
    salt: state.salt,
    iv,
    ciphertext,
    backend: state.backend
  });
  saveBlobByBackend(state.backend, blobBytes);
  setProfileSecretState((prev) => ({
    ...prev,
    integrations: { ...nextIntegrations }
  }));
  return true;
}

async function setProfileSecret(key, value) {
  const secretKey = String(key || "").trim();
  if (!secretKey) return false;
  const state = profileSecretState();
  const next = { ...(state.integrations || {}) };
  const trimmed = String(value || "").trim();
  if (!trimmed) {
    delete next[secretKey];
  } else {
    next[secretKey] = trimmed;
  }
  return persistProfileSecrets(next);
}

async function removeProfileSecret(key) {
  return setProfileSecret(key, "");
}

export {
  profileSecretState,
  setProfileSecretsContext,
  clearProfileSecretsContext,
  hasActiveProfileSecretsContext,
  getProfileSecret,
  setProfileSecret,
  removeProfileSecret
};
