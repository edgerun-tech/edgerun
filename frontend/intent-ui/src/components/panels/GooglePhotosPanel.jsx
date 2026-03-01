import { createSignal, For, Show, onMount } from "solid-js";
import { FiExternalLink, FiRefreshCw } from "solid-icons/fi";
import { integrationStore } from "../../stores/integrations";

const GOOGLE_PHOTOS_PAGE_SIZE = 40;

function formatTimestamp(value) {
  if (!value) return "";
  const time = new Date(value);
  if (Number.isNaN(time.getTime())) return "";
  return time.toLocaleString();
}

function imageUrlFor(item) {
  const base = String(item?.baseUrl || "").trim();
  if (!base) return "";
  return `${base}=w640-h640-c`;
}

export default function GooglePhotosPanel() {
  const [items, setItems] = createSignal([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal("");

  const loadPhotos = async () => {
    const token = String(integrationStore.getToken("google") || localStorage.getItem("google_token") || "").trim();
    if (!token) {
      setItems([]);
      setError("Google token is missing. Connect Google integration first.");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const response = await fetch(`/api/google/photos?page_size=${GOOGLE_PHOTOS_PAGE_SIZE}&token=${encodeURIComponent(token)}`, { cache: "no-store" });
      const payload = await response.json().catch(() => ({}));
      if (!response.ok || payload?.ok === false) {
        throw new Error(payload?.error || `google photos request failed (${response.status})`);
      }
      const next = Array.isArray(payload?.items) ? payload.items : [];
      setItems(next);
      if (next.length === 0) {
        setError(String(payload?.hint || "No Google Photos items found."));
      }
    } catch (loadError) {
      setItems([]);
      setError(loadError instanceof Error ? loadError.message : "Failed to load Google Photos.");
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    void loadPhotos();
  });

  return (
    <div class="flex h-full min-h-0 flex-col bg-[#0c0d11]" data-testid="google-photos-panel">
      <div class="flex items-center justify-between border-b border-neutral-800 px-3 py-2">
        <div>
          <p class="text-xs uppercase tracking-wide text-neutral-400">Google Photos</p>
          <p class="text-[11px] text-neutral-500">Local bridge media browser</p>
        </div>
        <div class="flex items-center gap-1.5">
          <button
            type="button"
            class="inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 hover:border-[hsl(var(--primary)/0.45)]"
            onClick={() => void loadPhotos()}
            data-testid="google-photos-refresh"
          >
            <FiRefreshCw size={12} />
            Refresh
          </button>
          <a
            class="inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 hover:border-[hsl(var(--primary)/0.45)]"
            href="https://photos.google.com"
            target="_blank"
            rel="noopener noreferrer"
          >
            <FiExternalLink size={12} />
            Open Web
          </a>
        </div>
      </div>

      <Show when={error()}>
        <div class="border-b border-neutral-800 bg-red-900/20 px-3 py-2 text-[11px] text-red-200" data-testid="google-photos-error">
          {error()}
        </div>
      </Show>

      <div class="min-h-0 flex-1 overflow-auto p-3">
        <Show when={!loading()} fallback={<p class="text-[11px] text-neutral-400">Loading photos...</p>}>
          <Show when={items().length > 0} fallback={<p class="text-[11px] text-neutral-500">No photos to display.</p>}>
            <div class="grid grid-cols-2 gap-2 md:grid-cols-3 lg:grid-cols-4" data-testid="google-photos-grid">
              <For each={items()}>
                {(item) => (
                  <article class="overflow-hidden rounded border border-neutral-800 bg-neutral-900/60" data-testid="google-photos-item">
                    <img
                      src={imageUrlFor(item)}
                      alt={String(item?.filename || "Google Photo")}
                      class="h-40 w-full object-cover"
                      loading="lazy"
                      referrerPolicy="no-referrer"
                    />
                    <div class="space-y-1 px-2 py-1.5">
                      <p class="truncate text-[10px] text-neutral-200">{String(item?.filename || item?.id || "Photo")}</p>
                      <p class="truncate text-[9px] text-neutral-500">{formatTimestamp(item?.mediaMetadata?.creationTime)}</p>
                    </div>
                  </article>
                )}
              </For>
            </div>
          </Show>
        </Show>
      </div>
    </div>
  );
}
