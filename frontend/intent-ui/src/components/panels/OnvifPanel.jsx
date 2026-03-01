import { For, Show, createMemo, createSignal, onMount } from "solid-js";
import { openWindow } from "../../stores/windows";
import { navigateBrowser } from "../../stores/ui-actions";
import { localBridgeHttpUrl } from "../../lib/local-bridge-origin";

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
  const url = String(input?.url || input?.xaddr || input?.xAddr || input?.endpoint || "").trim();
  const ip = String(input?.ip || input?.host || "").trim();
  const name = String(input?.name || input?.label || ip || "").trim();
  const candidateUrl = url || (ip ? `http://${ip}/onvif/device_service` : "");
  if (!candidateUrl) return null;
  const normalized = normalizeCameraUrl(candidateUrl).url;
  if (!normalized) return null;
  return {
    id: String(input?.id || `scan-${Date.now()}-${index}`),
    name,
    ip,
    url: normalized
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
      setScanResults(items);
      setStatus(items.length > 0 ? `Found ${items.length} ONVIF candidates.` : "No ONVIF candidates found.");
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
          <div class="space-y-1.5">
            <For each={scanResults()}>
              {(item) => (
                <div class="flex items-center justify-between gap-2 rounded border border-neutral-800 bg-neutral-900/60 px-2 py-1.5 text-xs">
                  <div class="min-w-0">
                    <p class="truncate text-neutral-200">{item.name || item.ip || item.url}</p>
                    <p class="truncate text-neutral-500">{item.url}</p>
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
            </For>
          </div>
        </section>
      </Show>

      <Show when={cameraCards().length > 0} fallback={<p class="text-xs text-neutral-500">No cameras added yet.</p>}>
        <div class="grid grid-cols-1 gap-3">
          <For each={cameraCards()}>
            {(camera) => (
              <article
                class="resize overflow-auto rounded-lg border border-neutral-800 bg-neutral-900/70 p-2 flex flex-col"
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
          </For>
        </div>
      </Show>
    </div>
  );
}

export default OnvifPanel;
