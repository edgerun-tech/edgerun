import { For, Show, createMemo, createSignal, onMount } from "solid-js";
import { openWindow } from "../../stores/windows";
import { navigateBrowser } from "../../stores/ui-actions";

const ONVIF_CAMERAS_KEY = "intent-ui-onvif-cameras-v1";

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

function isVideoLike(url) {
  return /^rtsp:\/\//i.test(url) || /\.m3u8($|\?)/i.test(url) || /\.(mp4|webm)($|\?)/i.test(url);
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
  const [urlInput, setUrlInput] = createSignal("");
  const [labelInput, setLabelInput] = createSignal("");
  const [scanBusy, setScanBusy] = createSignal(false);
  const [scanResults, setScanResults] = createSignal([]);
  const [status, setStatus] = createSignal("");

  onMount(() => {
    setCameras(readStoredCameras());
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
    const key = getCameraKey(normalized);
    if (cameras().some((item) => getCameraKey(item.url) === key)) {
      setStatus("Camera already added.");
      return;
    }
    const id = `cam-${Date.now()}-${Math.random().toString(16).slice(2, 7)}`;
    const next = [{ id, url: normalized, label: String(label || "").trim() }, ...cameras()];
    saveCameras(next);
    setUrlInput("");
    setLabelInput("");
    setStatus(normalizedResult.hadCredentials ? "Camera added. Embedded credentials were stripped." : "Camera added.");
  };

  const removeCamera = (id) => {
    saveCameras(cameras().filter((item) => item.id !== id));
  };

  const runScan = async () => {
    setScanBusy(true);
    setStatus("");
    try {
      const response = await fetch("/api/onvif/discover", { cache: "no-store" });
      const payload = await response.json().catch(() => ({}));
      if (!response.ok || payload?.ok === false) {
        throw new Error(String(payload?.error || "Failed to scan ONVIF cameras."));
      }
      const items = Array.isArray(payload?.items) ? payload.items : [];
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
    previewSrc: getPreviewSrc(camera.url)
  })));

  return (
    <div class="h-full overflow-auto bg-[#1a1a1a] p-4 text-neutral-200">
      <div class="mb-3">
        <h2 class="text-lg font-semibold text-white">ONVIF Cameras</h2>
        <p class="mt-1 text-xs text-neutral-500">Add camera URL or scan LAN for ONVIF endpoints. You can keep multiple camera widgets.</p>
      </div>

      <div class="mb-3 grid grid-cols-1 gap-2">
        <input
          type="text"
          value={labelInput()}
          onInput={(event) => setLabelInput(event.currentTarget.value)}
          placeholder="Label (optional)"
          class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
        />
        <input
          type="text"
          value={urlInput()}
          onInput={(event) => setUrlInput(event.currentTarget.value)}
          placeholder="Camera URL or host (example: 192.168.1.22/onvif/device_service)"
          class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
        />
        <div class="flex gap-2">
          <button
            type="button"
            class="rounded-md border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-xs text-neutral-200 hover:bg-neutral-800"
            onClick={() => addCamera(urlInput(), labelInput())}
          >
            Add Camera
          </button>
          <button
            type="button"
            class="rounded-md border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-xs text-neutral-200 hover:bg-neutral-800 disabled:opacity-50"
            onClick={runScan}
            disabled={scanBusy()}
          >
            {scanBusy() ? "Scanning..." : "Scan LAN"}
          </button>
        </div>
      </div>

      <Show when={status()}>
        <p class="mb-3 text-xs text-cyan-300">{status()}</p>
      </Show>

      <Show when={scanResults().length > 0}>
        <section class="mb-4 rounded-lg border border-neutral-800 bg-neutral-900/60 p-2.5">
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
              >
                <div class="mb-2 flex items-center justify-between gap-2">
                  <div class="min-w-0">
                    <p class="truncate text-sm font-medium text-neutral-100">{camera.title}</p>
                    <p class="truncate text-[11px] text-neutral-500">{camera.url}</p>
                  </div>
                  <div class="flex items-center gap-1">
                    <button
                      type="button"
                      class="rounded border border-neutral-700 px-2 py-1 text-[11px] text-neutral-300 hover:bg-neutral-800"
                      onClick={() => {
                        openWindow("browser");
                        navigateBrowser(camera.url);
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
                      when={isVideoLike(camera.url)}
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
