import { createEffect, createMemo, createSignal } from "solid-js";
import { CURRENT_DEVICE_ID, knownDevices } from "../../stores/devices";
import { DOMAIN_RESERVATION_STORAGE_KEYS, LOCAL_BRIDGE_LISTEN } from "./workflow-overlay.constants";
import { groupFleetDevices } from "./workflow-overlay.utils";

export function useWorkflowDeviceConnect({ state }) {
  const [selectedDeviceId, setSelectedDeviceId] = createSignal("");
  const devices = createMemo(() => knownDevices());
  const fleetDevices = createMemo(() => groupFleetDevices(devices(), CURRENT_DEVICE_ID));
  const selectedDevice = createMemo(() => fleetDevices().find((item) => item.id === selectedDeviceId()) || null);

  const [connectPlatform, setConnectPlatform] = createSignal("linux");
  const initialPairingCode = typeof window === "undefined"
    ? ""
    : String(window.localStorage.getItem("intent-ui-device-pairing-code-v1") || "").trim();
  const [pairingCodeInput, setPairingCodeInput] = createSignal(initialPairingCode);
  const [deviceConnectCopied, setDeviceConnectCopied] = createSignal(false);

  const readDomainReservation = () => {
    if (typeof window === "undefined") return { domain: "", registrationToken: "" };
    let domain = "";
    let registrationToken = "";
    for (const key of DOMAIN_RESERVATION_STORAGE_KEYS) {
      const raw = localStorage.getItem(key);
      if (!raw) continue;
      const text = String(raw).trim();
      if (!text) continue;
      if (text.startsWith("{")) {
        try {
          const parsed = JSON.parse(text);
          domain = domain || String(parsed?.domain || parsed?.assignedDomain || parsed?.fqdn || "").trim();
          registrationToken = registrationToken || String(parsed?.registrationToken || parsed?.registration_token || parsed?.token || "").trim();
        } catch {
          // ignore parse failures
        }
      } else if (text.includes(".")) {
        domain = domain || text;
      }
    }
    domain = domain || String(localStorage.getItem("intent-ui-device-connect-domain-v1") || "").trim();
    registrationToken = registrationToken || String(localStorage.getItem("intent-ui-device-connect-registration-token-v1") || "").trim();
    return { domain, registrationToken };
  };

  const initialReservation = readDomainReservation();
  const [profilePublicKeyInput, setProfilePublicKeyInput] = createSignal(
    typeof window === "undefined"
      ? ""
      : String(window.localStorage.getItem("intent-ui-profile-public-key-v1") || "").trim()
  );
  const [requestedLabelInput, setRequestedLabelInput] = createSignal("");
  const [connectDomain, setConnectDomain] = createSignal(initialReservation.domain);
  const [connectRegistrationToken, setConnectRegistrationToken] = createSignal(initialReservation.registrationToken);
  const [reserveBusy, setReserveBusy] = createSignal(false);
  const [reserveError, setReserveError] = createSignal("");
  const [reserveStatus, setReserveStatus] = createSignal("");
  const [pairingBusy, setPairingBusy] = createSignal(false);
  const [pairingError, setPairingError] = createSignal("");
  const [pairingStatus, setPairingStatus] = createSignal("");
  const [pairingExpiresAt, setPairingExpiresAt] = createSignal("");
  const [showDeviceConnectDialog, setShowDeviceConnectDialog] = createSignal(false);

  const linuxConnectScript = createMemo(() => {
    const pairingCode = pairingCodeInput().trim() || "<PAIRING_CODE>";
    return [
      "# 1) Install node manager",
      "curl -fsSL https://downloads.edgerun.tech/install-node-manager.sh | sh -s -- --bridge-listen 127.0.0.1:7777",
      "",
      "# 2) Pair this machine to your EdgeRun domain",
      `edgerun-node-manager tunnel-connect --relay-control-base https://relay.edgerun.tech --pairing-code \"${pairingCode}\"`,
      "",
      "# 3) Start node manager with local bridge for browser eventbus",
      `edgerun-node-manager run --local-bridge-listen ${LOCAL_BRIDGE_LISTEN}`,
      "",
      "# 4) Optional: keep it running on boot (if package installs service unit)",
      "sudo systemctl enable --now edgerun-node-manager.service"
    ].join("\\n");
  });

  const copyConnectScript = async () => {
    try {
      await navigator.clipboard.writeText(linuxConnectScript());
      setDeviceConnectCopied(true);
      window.setTimeout(() => setDeviceConnectCopied(false), 1200);
    } catch {
      setDeviceConnectCopied(false);
    }
  };

  const issuePairingCode = async () => {
    if (pairingBusy()) return;
    const domain = connectDomain().trim();
    const registrationToken = connectRegistrationToken().trim();
    if (!domain || !registrationToken) {
      setPairingError("Domain and registration token are required.");
      setPairingStatus("");
      return;
    }
    setPairingBusy(true);
    setPairingError("");
    setPairingStatus("");
    try {
      const response = await fetch("/api/tunnel/create-pairing-code", {
        method: "POST",
        headers: { "content-type": "application/json; charset=utf-8" },
        body: JSON.stringify({ domain, registrationToken, ttlSeconds: 300 })
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok || !body?.ok) {
        throw new Error(String(body?.error || `pairing request failed (${response.status})`));
      }
      const code = String(body?.pairingCode || "").trim();
      if (!code) throw new Error("Pairing code was empty in relay response.");
      setPairingCodeInput(code);
      const expiresMs = Number(body?.expiresUnixMs || 0);
      setPairingExpiresAt(expiresMs > 0 ? new Date(expiresMs).toISOString() : "");
      setPairingStatus("Pairing code issued.");
    } catch (err) {
      setPairingError(err instanceof Error ? err.message : "Failed to issue pairing code.");
    } finally {
      setPairingBusy(false);
    }
  };

  const reserveDomain = async () => {
    if (reserveBusy()) return;
    const profilePublicKeyB64url = profilePublicKeyInput().trim();
    const requestedLabel = requestedLabelInput().trim();
    if (!profilePublicKeyB64url) {
      setReserveError("Profile public key is required.");
      setReserveStatus("");
      return;
    }
    setReserveBusy(true);
    setReserveError("");
    setReserveStatus("");
    try {
      const response = await fetch("/api/tunnel/reserve-domain", {
        method: "POST",
        headers: { "content-type": "application/json; charset=utf-8" },
        body: JSON.stringify({ profilePublicKeyB64url, requestedLabel })
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok || !body?.ok) {
        throw new Error(String(body?.error || `domain reserve failed (${response.status})`));
      }
      const domain = String(body?.domain || "").trim();
      const token = String(body?.registrationToken || "").trim();
      if (!domain || !token) throw new Error("Relay response missing domain or registration token.");
      setConnectDomain(domain);
      setConnectRegistrationToken(token);
      setReserveStatus(String(body?.status || "reserved"));
      localStorage.setItem(
        "intent-ui-domain-reservation-v1",
        JSON.stringify({
          domain,
          registrationToken: token,
          status: String(body?.status || "reserved"),
          userId: String(body?.userId || "")
        })
      );
    } catch (err) {
      setReserveError(err instanceof Error ? err.message : "Failed to reserve domain.");
    } finally {
      setReserveBusy(false);
    }
  };

  createEffect(() => {
    const list = fleetDevices();
    if (list.length === 0) return;
    if (!selectedDeviceId() || !list.some((item) => item.id === selectedDeviceId())) {
      setSelectedDeviceId(list[0].id);
    }
  });

  createEffect(() => {
    if (typeof window === "undefined") return;
    const value = pairingCodeInput().trim();
    if (!value) {
      window.localStorage.removeItem("intent-ui-device-pairing-code-v1");
      return;
    }
    window.localStorage.setItem("intent-ui-device-pairing-code-v1", value);
  });

  createEffect(() => {
    if (typeof window === "undefined") return;
    localStorage.setItem("intent-ui-device-connect-domain-v1", connectDomain().trim());
  });

  createEffect(() => {
    if (typeof window === "undefined") return;
    localStorage.setItem("intent-ui-profile-public-key-v1", profilePublicKeyInput().trim());
  });

  createEffect(() => {
    if (typeof window === "undefined") return;
    localStorage.setItem("intent-ui-device-connect-registration-token-v1", connectRegistrationToken().trim());
  });

  createEffect(() => {
    if (state().rightOpen && state().rightPanel === "devices") return;
    setShowDeviceConnectDialog(false);
  });

  return {
    selectedDeviceId,
    setSelectedDeviceId,
    devices,
    fleetDevices,
    selectedDevice,
    connectPlatform,
    setConnectPlatform,
    pairingCodeInput,
    setPairingCodeInput,
    deviceConnectCopied,
    showDeviceConnectDialog,
    setShowDeviceConnectDialog,
    profilePublicKeyInput,
    setProfilePublicKeyInput,
    requestedLabelInput,
    setRequestedLabelInput,
    connectDomain,
    setConnectDomain,
    connectRegistrationToken,
    setConnectRegistrationToken,
    reserveBusy,
    reserveError,
    reserveStatus,
    pairingBusy,
    pairingError,
    pairingStatus,
    pairingExpiresAt,
    linuxConnectScript,
    copyConnectScript,
    issuePairingCode,
    reserveDomain
  };
}
