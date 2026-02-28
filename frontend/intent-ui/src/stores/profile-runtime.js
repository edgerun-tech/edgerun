import { createSignal } from "solid-js";
import { DEFAULT_LOCAL_PROFILE_SCOPES, normalizeScopes } from "../lib/oidc-scopes";

const PROFILE_MODE_SESSION_KEY = "intent-ui-profile-mode-v1";
const PROFILE_ID_SESSION_KEY = "intent-ui-profile-id-v1";
const PROFILE_BACKEND_SESSION_KEY = "intent-ui-profile-backend-v1";
const PROFILE_SCOPES_SESSION_KEY = "intent-ui-profile-scopes-v1";

const MODE_PROFILE = "profile";

const [profileRuntime, setProfileRuntime] = createSignal({
  ready: false,
  mode: MODE_PROFILE,
  profileLoaded: false,
  profileId: "",
  backend: "",
  grantedScopes: []
});

const browser = () => typeof window !== "undefined";

function hydrateProfileRuntime() {
  if (!browser()) return;
  const mode = sessionStorage.getItem(PROFILE_MODE_SESSION_KEY);
  const profileId = sessionStorage.getItem(PROFILE_ID_SESSION_KEY) || "";
  const backend = sessionStorage.getItem(PROFILE_BACKEND_SESSION_KEY) || "";
  const scopesRaw = sessionStorage.getItem(PROFILE_SCOPES_SESSION_KEY) || "[]";
  let grantedScopes = [];
  try {
    grantedScopes = normalizeScopes(JSON.parse(scopesRaw));
  } catch {
    grantedScopes = [];
  }

  if (mode === MODE_PROFILE && profileId.trim()) {
    setProfileRuntime({
      ready: true,
      mode: MODE_PROFILE,
      profileLoaded: true,
      profileId: profileId.trim(),
      backend: backend || "browser_local",
      grantedScopes
    });
    return;
  }

  setProfileRuntime((prev) => ({ ...prev, ready: false }));
}

function activatePersistentProfileSession(input) {
  if (!browser()) return;
  const profileId = String(input?.profileId || "").trim();
  const backend = String(input?.backend || "").trim() || "browser_local";
  const grantedScopes = normalizeScopes(input?.grantedScopes || DEFAULT_LOCAL_PROFILE_SCOPES);
  if (!profileId) return;
  sessionStorage.setItem(PROFILE_MODE_SESSION_KEY, MODE_PROFILE);
  sessionStorage.setItem(PROFILE_ID_SESSION_KEY, profileId);
  sessionStorage.setItem(PROFILE_BACKEND_SESSION_KEY, backend);
  sessionStorage.setItem(PROFILE_SCOPES_SESSION_KEY, JSON.stringify(grantedScopes));
  setProfileRuntime({
    ready: true,
    mode: MODE_PROFILE,
    profileLoaded: true,
    profileId,
    backend,
    grantedScopes
  });
}

function clearProfileRuntimeSession() {
  if (!browser()) return;
  sessionStorage.removeItem(PROFILE_MODE_SESSION_KEY);
  sessionStorage.removeItem(PROFILE_ID_SESSION_KEY);
  sessionStorage.removeItem(PROFILE_BACKEND_SESSION_KEY);
  sessionStorage.removeItem(PROFILE_SCOPES_SESSION_KEY);
  setProfileRuntime({
    ready: false,
    mode: MODE_PROFILE,
    profileLoaded: false,
    profileId: "",
    backend: "",
    grantedScopes: []
  });
}

export {
  profileRuntime,
  hydrateProfileRuntime,
  activatePersistentProfileSession,
  clearProfileRuntimeSession
};
