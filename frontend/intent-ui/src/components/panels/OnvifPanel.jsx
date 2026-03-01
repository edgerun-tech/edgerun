import { Show, createMemo, createSignal, onMount } from "solid-js";
import { openWindow } from "../../stores/windows";
import { navigateBrowser } from "../../stores/ui-actions";
import { localBridgeHttpUrl } from "../../lib/local-bridge-origin";
import VirtualAnimatedList from "../common/VirtualAnimatedList";

const ONVIF_CAMERAS_KEY = "intent-ui-onvif-cameras-v1";
const ONVIF_DEFAULTS_KEY = "intent-ui-onvif-defaults-v1";
const ONVIF_DEFAULTS_BASE = {
  protocol: "rtsp",
  streamPath: "stream1",
  username: "",
  password: ""
};

function safeParse(raw) {
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function readStoredCameras() {
  if (typeof localStorage === "undefined") return [];
  const parsed = safeParse(localStorage.getItem(ONVIF_CAMERAS_KEY) || "");
  if (!Array.isArray(parsed)) return [];
  return parsed
    .map((item, index) => ({
      id: String(item?.id || `cam-${Date.now()}-${index}`),
      label: String(item?.label || "").trim().slice(0, 120),
      url: String(item?.url || "").trim().slice(0, 1200)
    }))
    .filter((item) => item.url);
}

function persistCameras(cameras) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(ONVIF_CAMERAS_KEY, JSON.stringify(cameras));
  } catch {
    // ignore storage failures
  }
}

function normalizeOnvifDefaults(input) {
  const value = input && typeof input === "object" ? input : {};
  const protocolRaw = String(value?.protocol || ONVIF_DEFAULTS_BASE.protocol).trim().toLowerCase();
  const protocol = ["rtsp", "http", "https"].includes(protocolRaw) ? protocolRaw : ONVIF_DEFAULTS_BASE.protocol;
  return {
    protocol,
    streamPath: String(value?.streamPath || ONVIF_DEFAULTS_BASE.streamPath).trim().slice(0, 120) || ONVIF_DEFAULTS_BASE.streamPath,
    username: String(value?.username || "").trim().slice(0, 120),
    password: String(value?.password || "").slice(0, 120)
  };
}

function readStoredOnvifDefaults() {
  if (typeof localStorage === "undefined") return ONVIF_DEFAULTS_BASE;
  const parsed = safeParse(localStorage.getItem(ONVIF_DEFAULTS_KEY) || "");
  return normalizeOnvifDefaults(parsed);
}

function persistOnvifDefaults(defaults) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(ONVIF_DEFAULTS_KEY, JSON.stringify(normalizeOnvifDefaults(defaults)));
  } catch {
    // ignore storage failures
  }
}

function normalizeInputUrl(raw) {
  const value = String(raw || "").trim();
  if (!value) return "";
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(value)) return value;
  return `http://${value}`;
}

function normalizeCameraUrl(raw) {
  const normalized = normalizeInputUrl(raw);
  if (!normalized) return { url: "", hadCredentials: false };
  try {
    const parsed = new URL(normalized);
    const hadCredentials = Boolean(parsed.username || parsed.password);
    parsed.username = "";
    parsed.password = "";
    return { url: parsed.toString(), hadCredentials };
  } catch {
    return { url: normalized, hadCredentials: false };
  }
}

function getCameraKey(url) {
  try {
    const parsed = new URL(url);
    const protocol = parsed.protocol.toLowerCase();
    const host = parsed.host.toLowerCase();
    const pathname = parsed.pathname.replace(/\/+$/, "").toLowerCase() || "/";
    const search = parsed.search || "";
    return `${protocol}//${host}${pathname}${search}`;
  } catch {
    return String(url || "").trim().toLowerCase();
  }
}

function getCardTitle(camera) {
  if (camera.label) return camera.label;
  try {
    const parsed = new URL(camera.url);
    return parsed.hostname || camera.url;
  } catch {
    return camera.url;
  }
}

function normalizeStreamPath(pathValue) {
  const raw = String(pathValue || "").trim().replace(/^\/+/, "");
  const normalized = raw || ONVIF_DEFAULTS_BASE.streamPath;
  return `/${normalized}`;
}

function deriveStreamUrlFromOnvifService(url, defaults) {
  try {
    const parsed = new URL(url);
    if (!parsed.pathname.toLowerCase().includes("/onvif/device_service")) {
      return url;
    }
    const settings = normalizeOnvifDefaults(defaults);
    const streamPath = normalizeStreamPath(settings.streamPath);
    return `${settings.protocol}://${parsed.host}${streamPath}`;
  } catch {
    return url;
  }
}

function applyRuntimeCredentials(url, defaults) {
  const settings = normalizeOnvifDefaults(defaults);
  if (!settings.username) return url;
  try {
    const parsed = new URL(url);
    if (parsed.username) return url;
    parsed.username = settings.username;
    parsed.password = settings.password || "";
    return parsed.toString();
  } catch {
    return url;
  }
}

function normalizeStoredCameraUrl(url, defaults) {
  return deriveStreamUrlFromOnvifService(url, defaults);
}

function resolveRuntimeCameraUrl(url, defaults) {
  const streamUrl = deriveStreamUrlFromOnvifService(url, defaults);
  return applyRuntimeCredentials(streamUrl, defaults);
}

function isVideoLike(url) {
  return /^rtsp:\/\//i.test(url) || /\.m3u8($|\?)/i.test(url) || /\.(mp4|webm)($|\?)/i.test(url);
}

function toScanItem(input, index = 0) {
  const serviceUrl = String(input?.url || input?.xaddr || input?.xAddr || input?.endpoint || "").trim();
  const streamUrl = String(input?.streamUrl || input?.stream_url || input?.stream || "").trim();
  const ip = String(input?.ip || input?.host || "").trim();
  const name = String(input?.name || input?.label || ip || "").trim();
  const candidateUrl = streamUrl || serviceUrl || (ip ? `http://${ip}/onvif/device_service` : "");
  if (!candidateUrl) return null;
  const normalized = normalizeCameraUrl(candidateUrl).url;
  if (!normalized) return null;
  return {
    id: String(input?.id || `scan-${Date.now()}-${index}`),
    name,
    ip,
    url: normalized,
    serviceUrl: normalizeCameraUrl(serviceUrl).url || ""
  };
}

function normalizeScanItems(items) {
  const source = Array.isArray(items) ? items : [];
  const seen = new Set();
  const results = [];
  source.forEach((item, index) => {
    const next = toScanItem(item, index);
    if (!next) return;
    const key = getCameraKey(next.url);
    if (seen.has(key)) return;
    seen.add(key);
    results.push(next);
  });
  return results;
}

function xmlEscape(value) {
  return String(value || "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function wsseHeader(defaults) {
  const username = String(defaults?.username || "").trim();
  if (!username) return "";
  const password = String(defaults?.password || "");
  const nonce = Math.random().toString(16).slice(2);
  const created = new Date().toISOString();
  return `<wsse:Security s:mustUnderstand="1"><wsse:UsernameToken><wsse:Username>${xmlEscape(username)}</wsse:Username><wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText">${xmlEscape(password)}</wsse:Password><wsse:Nonce>${xmlEscape(nonce)}</wsse:Nonce><wsu:Created>${xmlEscape(created)}</wsu:Created></wsse:UsernameToken></wsse:Security>`;
}

function extractProfileTokens(xml) {
  if (!xml) return [];
  const tokenMatches = Array.from(xml.matchAll(/Profiles[^>]*token\s*=\s*"([^"]+)"/gi));
  return tokenMatches
    .map((match) => String(match?.[1] || "").trim())
    .filter(Boolean)
    .slice(0, 6);
}

function extractRtspUrls(text) {
  if (!text) return [];
  return Array.from(new Set(Array.from(text.matchAll(/rtsps?:\/\/[^\s<"']+/gi)).map((match) => String(match?.[0] || "").trim()).filter(Boolean)));
}

function serviceUrlForDirectQuery(item) {
  const serviceUrl = String(item?.serviceUrl || "").trim();
  if (serviceUrl) return serviceUrl;
  const url = String(item?.url || "").trim();
  return url.toLowerCase().includes("/onvif/device_service") ? url : "";
}

function canAttemptDirectOnvifQuery() {
  if (typeof window === "undefined") return false;
  if (window.Cypress) return false;
  return true;
}

async function postOnvifSoap(serviceUrl, action, body, timeoutMs = 2200) {
  const controller = typeof AbortController === "function" ? new AbortController() : null;
  const timer = controller ? setTimeout(() => controller.abort(), timeoutMs) : null;
  try {
    const response = await fetch(serviceUrl, {
      method: "POST",
      mode: "cors",
      cache: "no-store",
      signal: controller?.signal,
      headers: {
        "Content-Type": `application/soap+xml; charset=utf-8; action=\"${action}\"`,
        Accept: "application/soap+xml, text/xml, */*"
      },
      body
    });
    const text = await response.text().catch(() => "");
    if (!response.ok) {
      throw new Error(text || `ONVIF request failed (${response.status}).`);
    }
    return text;
  } finally {
    if (timer) clearTimeout(timer);
  }
}

function onvifEnvelope(innerBody, defaults) {
  const security = wsseHeader(defaults);
  return `<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema" xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd" xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd"><s:Header>${security}</s:Header><s:Body>${innerBody}</s:Body></s:Envelope>`;
}

async function resolveOnvifStreamUrlDirect(item, defaults) {
  const serviceUrl = serviceUrlForDirectQuery(item);
  if (!serviceUrl) return null;

  const profilesXml = await postOnvifSoap(
    serviceUrl,
    "http://www.onvif.org/ver10/media/wsdl/GetProfiles",
    onvifEnvelope("<trt:GetProfiles/>", defaults)
  );

  const directFromProfiles = extractRtspUrls(profilesXml)[0] || null;
  if (directFromProfiles) return directFromProfiles;

  const tokens = extractProfileTokens(profilesXml);
  for (const token of tokens.slice(0, 4)) {
    const streamXml = await postOnvifSoap(
      serviceUrl,
      "http://www.onvif.org/ver10/media/wsdl/GetStreamUri",
      onvifEnvelope(`<trt:GetStreamUri><trt:StreamSetup><tt:Stream>RTP-Unicast</tt:Stream><tt:Transport><tt:Protocol>RTSP</tt:Protocol></tt:Transport></trt:StreamSetup><trt:ProfileToken>${xmlEscape(token)}</trt:ProfileToken></trt:GetStreamUri>`, defaults)
    );
    const rtsp = extractRtspUrls(streamXml)[0] || null;
    if (rtsp) return rtsp;
  }

  return null;
}

async function resolveScanStreamUrls(items, defaults) {
  if (!canAttemptDirectOnvifQuery()) {
    return { items, resolved: 0, attempted: 0 };
  }
  const source = Array.isArray(items) ? items : [];
  const output = [];
  let resolved = 0;
  let attempted = 0;

  for (const item of source) {
    const serviceUrl = serviceUrlForDirectQuery(item);
    if (!serviceUrl) {
      output.push(item);
      continue;
    }
    attempted += 1;
    try {
      const directStream = await resolveOnvifStreamUrlDirect(item, defaults);
      if (directStream) {
        const normalized = normalizeCameraUrl(directStream).url;
        output.push({ ...item, url: normalized, streamUrl: normalized, serviceUrl });
        resolved += 1;
      } else {
        output.push(item);
      }
    } catch {
      output.push(item);
    }
  }

  return { items: output, resolved, attempted };
}

async function requestOnvifDiscover() {
  const endpoints = [
    localBridgeHttpUrl("/v1/local/onvif/discover"),
    "/api/onvif/discover"
  ];
  let sawMissingEndpoint = false;
  let lastError = null;

  for (const endpoint of endpoints) {
    try {
      const response = await fetch(endpoint, { cache: "no-store" });
      const payload = await response.json().catch(() => ({}));
      if (!response.ok || payload?.ok === false) {
        if (response.status === 404 || response.status === 405 || response.status === 501) {
          sawMissingEndpoint = true;
          continue;
        }
        const detail = String(payload?.error || "").trim();
        throw new Error(detail || `ONVIF scan failed (${response.status}).`);
      }
      return normalizeScanItems(payload?.items || payload?.devices || payload?.results || []);
    } catch (error) {
      lastError = error;
    }
  }

  if (sawMissingEndpoint) {
    throw new Error("ONVIF scan service unavailable on this node. Add camera URL manually.");
  }
  throw (lastError instanceof Error ? lastError : new Error("Failed to scan ONVIF cameras."));
}

function getPreviewSrc(url) {
  if (!url) return "";
  if (isVideoLike(url)) return url;
  try {
    const parsed = new URL(url);
    if (parsed.pathname.toLowerCase().includes("/onvif/device_service")) {
      return `/api/browser/proxy?url=${encodeURIComponent(`${parsed.protocol}//${parsed.host}/`)}`;
    }
    return `/api/browser/proxy?url=${encodeURIComponent(url)}`;
  } catch {
    return "";
  }
}

function OnvifPanel() {
  let scanResultsListRef;
  let camerasListRef;
  const [cameras, setCameras] = createSignal(readStoredCameras());
  const [defaults, setDefaults] = createSignal(readStoredOnvifDefaults());
  const [urlInput, setUrlInput] = createSignal("");
  const [labelInput, setLabelInput] = createSignal("");
  const [scanBusy, setScanBusy] = createSignal(false);
  const [scanResults, setScanResults] = createSignal([]);
  const [status, setStatus] = createSignal("");

  onMount(() => {
    setCameras(readStoredCameras());
    setDefaults(readStoredOnvifDefaults());
  });

  const saveCameras = (next) => {
    setCameras(next);
    persistCameras(next);
  };

  const addCamera = (url, label = "") => {
    const normalizedResult = normalizeCameraUrl(url);
    const normalized = normalizedResult.url;
    if (!normalized) {
      setStatus("Camera URL is required.");
      return;
    }
    const storedUrl = normalizeStoredCameraUrl(normalized, defaults());
    const key = getCameraKey(storedUrl);
    if (cameras().some((item) => getCameraKey(item.url) === key)) {
      setStatus("Camera already added.");
      return;
    }
    const id = `cam-${Date.now()}-${Math.random().toString(16).slice(2, 7)}`;
    const next = [{ id, url: storedUrl, label: String(label || "").trim() }, ...cameras()];
    saveCameras(next);
    setUrlInput("");
    setLabelInput("");

    const mappedToStream = storedUrl !== normalized;
    const hasDefaultAuth = Boolean(defaults()?.username);
    if (normalizedResult.hadCredentials) {
      setStatus("Camera added. Embedded credentials were stripped.");
    } else if (mappedToStream && hasDefaultAuth) {
      setStatus("Camera added. ONVIF service endpoint mapped to stream path; default auth applies at runtime.");
    } else if (mappedToStream) {
      setStatus("Camera added. ONVIF service endpoint mapped to stream path.");
    } else if (hasDefaultAuth) {
      setStatus("Camera added. Default auth applies at runtime.");
    } else {
      setStatus("Camera added.");
    }
  };

  const updateDefaults = (patch) => {
    const next = normalizeOnvifDefaults({ ...defaults(), ...(patch || {}) });
    setDefaults(next);
    persistOnvifDefaults(next);
  };

  const removeCamera = (id) => {
    saveCameras(cameras().filter((item) => item.id !== id));
  };

  const runScan = async () => {
    setScanBusy(true);
    setStatus("");
    try {
      const items = await requestOnvifDiscover();
      const direct = await resolveScanStreamUrls(items, defaults());
      setScanResults(direct.items);
      if (direct.items.length === 0) {
        setStatus("No ONVIF candidates found.");
      } else if (direct.attempted > 0 && direct.resolved > 0) {
        setStatus(`Found ${direct.items.length} ONVIF candidates · resolved ${direct.resolved}/${direct.attempted} direct stream URLs.`);
      } else {
        setStatus(`Found ${direct.items.length} ONVIF candidates.`);
      }
    } catch (error) {
      setScanResults([]);
      setStatus(error instanceof Error ? error.message : "Failed to scan ONVIF cameras.");
    } finally {
      setScanBusy(false);
    }
  };

  const cameraCards = createMemo(() => cameras().map((camera) => ({
    ...camera,
    title: getCardTitle(camera),
    runtimeUrl: resolveRuntimeCameraUrl(camera.url, defaults()),
    previewSrc: getPreviewSrc(resolveRuntimeCameraUrl(camera.url, defaults()))
  })));

  return (
    <div class="h-full overflow-auto bg-[#1a1a1a] p-4 text-neutral-200" data-testid="onvif-panel">
      <div class="mb-3">
        <h2 class="text-lg font-semibold text-white">ONVIF Cameras</h2>
        <p class="mt-1 text-xs text-neutral-500">Add camera URL or scan LAN for ONVIF endpoints. You can keep multiple camera widgets.</p>
      </div>

      <div class="mb-3 grid grid-cols-1 gap-2">
        <div class="rounded-md border border-neutral-800 bg-neutral-900/50 p-2">
          <p class="mb-1 text-[11px] uppercase tracking-wide text-neutral-500">Stream Defaults</p>
          <div class="grid grid-cols-1 gap-2 md:grid-cols-2">
            <input
              type="text"
              value={defaults().username}
              onInput={(event) => updateDefaults({ username: event.currentTarget.value })}
              placeholder="Default username"
              class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
              data-testid="onvif-default-username"
            />
            <input
              type="password"
              value={defaults().password}
              onInput={(event) => updateDefaults({ password: event.currentTarget.value })}
              placeholder="Default password"
              class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
              data-testid="onvif-default-password"
            />
            <select
              value={defaults().protocol}
              onChange={(event) => updateDefaults({ protocol: event.currentTarget.value })}
              class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
              data-testid="onvif-default-protocol"
            >
              <option value="rtsp">rtsp</option>
              <option value="http">http</option>
              <option value="https">https</option>
            </select>
            <input
              type="text"
              value={defaults().streamPath}
              onInput={(event) => updateDefaults({ streamPath: event.currentTarget.value })}
              placeholder="Default stream path (stream1)"
              class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
              data-testid="onvif-default-stream-path"
            />
          </div>
          <p class="mt-1 text-[10px] text-neutral-500">Default credentials apply only at runtime for preview/open actions.</p>
        </div>

        <input
          type="text"
          value={labelInput()}
          onInput={(event) => setLabelInput(event.currentTarget.value)}
          placeholder="Label (optional)"
          class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
          data-testid="onvif-label-input"
        />
        <input
          type="text"
          value={urlInput()}
          onInput={(event) => setUrlInput(event.currentTarget.value)}
          placeholder="Camera URL or host (example: 192.168.1.22/onvif/device_service)"
          class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
          data-testid="onvif-url-input"
        />
        <div class="flex gap-2">
          <button
            type="button"
            class="rounded-md border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-xs text-neutral-200 hover:bg-neutral-800"
            onClick={() => addCamera(urlInput(), labelInput())}
            data-testid="onvif-add-camera"
          >
            Add Camera
          </button>
          <button
            type="button"
            class="rounded-md border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-xs text-neutral-200 hover:bg-neutral-800 disabled:opacity-50"
            onClick={runScan}
            disabled={scanBusy()}
            data-testid="onvif-scan-lan"
          >
            {scanBusy() ? "Scanning..." : "Scan LAN"}
          </button>
        </div>
      </div>

      <Show when={status()}>
        <p class="mb-3 text-xs text-cyan-300" data-testid="onvif-status">{status()}</p>
      </Show>

      <Show when={scanResults().length > 0}>
        <section class="mb-4 rounded-lg border border-neutral-800 bg-neutral-900/60 p-2.5" data-testid="onvif-scan-results">
          <p class="mb-2 text-xs uppercase tracking-wide text-neutral-500">Scan Results</p>
          <div class="max-h-56 overflow-auto" ref={scanResultsListRef}>
            <VirtualAnimatedList
              items={scanResults}
              estimateSize={56}
              overscan={4}
              containerRef={() => scanResultsListRef}
              animateRows
              renderItem={(item) => (
                <div class="mt-1.5 flex items-center justify-between gap-2 rounded border border-neutral-800 bg-neutral-900/60 px-2 py-1.5 text-xs">
                  <div class="min-w-0">
                    <p class="truncate text-neutral-200">{item.name || item.ip || item.url}</p>
                    <p class="truncate text-neutral-500">{item.url}</p>
                    <Show when={item.serviceUrl && item.serviceUrl !== item.url}>
                      <p class="truncate text-neutral-600">service: {item.serviceUrl}</p>
                    </Show>
                  </div>
                  <button
                    type="button"
                    class="shrink-0 rounded border border-neutral-700 px-2 py-0.5 text-[11px] text-neutral-200 hover:bg-neutral-800"
                    onClick={() => addCamera(item.url, item.name)}
                    data-testid="onvif-scan-add"
                  >
                    Add
                  </button>
                </div>
              )}
            />
          </div>
        </section>
      </Show>

      <Show when={cameraCards().length > 0} fallback={<p class="text-xs text-neutral-500">No cameras added yet.</p>}>
        <div class="max-h-[72vh] overflow-auto" ref={camerasListRef}>
          <VirtualAnimatedList
            items={cameraCards}
            estimateSize={250}
            overscan={2}
            containerRef={() => camerasListRef}
            animateRows
            renderItem={(camera) => (
              <article
                class="mt-3 resize overflow-auto rounded-lg border border-neutral-800 bg-neutral-900/70 p-2 flex flex-col"
                style={{ "min-height": "220px", "min-width": "280px" }}
                data-testid="onvif-camera-card"
              >
                <div class="mb-2 flex items-center justify-between gap-2">
                  <div class="min-w-0">
                    <p class="truncate text-sm font-medium text-neutral-100">{camera.title}</p>
                    <p class="truncate text-[11px] text-neutral-500">{camera.runtimeUrl}</p>
                    <Show when={camera.runtimeUrl !== camera.url}>
                      <p class="truncate text-[10px] text-neutral-600">source: {camera.url}</p>
                    </Show>
                  </div>
                  <div class="flex items-center gap-1">
                    <button
                      type="button"
                      class="rounded border border-neutral-700 px-2 py-1 text-[11px] text-neutral-300 hover:bg-neutral-800"
                      onClick={() => {
                        openWindow("browser");
                        navigateBrowser(camera.runtimeUrl);
                      }}
                    >
                      Open
                    </button>
                    <button
                      type="button"
                      class="rounded border border-neutral-700 px-2 py-1 text-[11px] text-neutral-300 hover:bg-neutral-800"
                      onClick={() => removeCamera(camera.id)}
                    >
                      Remove
                    </button>
                  </div>
                </div>
                <div class="min-h-[150px] flex-1 overflow-hidden rounded border border-neutral-800 bg-black/40">
                  <Show
                    when={camera.previewSrc}
                    fallback={<div class="p-3 text-xs text-neutral-500">Preview unavailable for this URL.</div>}
                  >
                    <Show
                      when={isVideoLike(camera.runtimeUrl)}
                      fallback={
                        <iframe
                          src={camera.previewSrc}
                          title={camera.title}
                          class="h-full w-full border-0"
                        />
                      }
                    >
                      <video
                        src={camera.previewSrc}
                        controls
                        muted
                        playsinline
                        class="h-full w-full bg-black object-contain"
                      />
                    </Show>
                  </Show>
                </div>
              </article>
            )}
          />
        </div>
      </Show>
    </div>
  );
}

export default OnvifPanel;
