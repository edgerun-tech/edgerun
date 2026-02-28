import { createMemo, createSignal } from "solid-js";
import { clearProfileRuntimeSession, hydrateProfileRuntime, profileRuntime } from "../../stores/profile-runtime";
import { clearProfileSecretsContext } from "../../stores/profile-secrets";

export function useAccountSession() {
  const [showBootstrapGate, setShowBootstrapGate] = createSignal(false);
  const [accountMenuOpen, setAccountMenuOpen] = createSignal(false);
  const [registeredDomain, setRegisteredDomain] = createSignal("");

  const sessionModeLabel = createMemo(() => (
    profileRuntime().mode === "profile" && profileRuntime().profileLoaded
      ? `profile (${profileRuntime().backend})`
      : "profile required"
  ));

  const shortProfileId = createMemo(() => {
    const id = String(profileRuntime().profileId || "").trim();
    if (!id) return "Not loaded";
    if (id.length <= 18) return id;
    return `${id.slice(0, 8)}...${id.slice(-6)}`;
  });

  const resetSession = () => {
    clearProfileRuntimeSession();
    clearProfileSecretsContext();
    setShowBootstrapGate(true);
    setAccountMenuOpen(false);
  };

  const completeBootstrap = () => {
    hydrateProfileRuntime();
    setShowBootstrapGate(false);
  };

  return {
    showBootstrapGate,
    setShowBootstrapGate,
    accountMenuOpen,
    setAccountMenuOpen,
    registeredDomain,
    setRegisteredDomain,
    sessionModeLabel,
    shortProfileId,
    resetSession,
    completeBootstrap
  };
}
