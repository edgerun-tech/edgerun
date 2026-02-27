import { For, Show, createMemo, createSignal } from "solid-js";
import { TbOutlineMapSearch, TbOutlineMinus, TbOutlinePlus, TbOutlineTarget } from "solid-icons/tb";
import { preferences } from "../../stores/preferences";

const MAP_VIEW_KEY = "intent-ui-map-view-v1";
const DEFAULT_VIEW = { lat: 40.7128, lon: -74.006, zoom: 4 };

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function readSavedView() {
  if (typeof localStorage === "undefined") return DEFAULT_VIEW;
  try {
    const parsed = JSON.parse(localStorage.getItem(MAP_VIEW_KEY) || "null");
    if (!parsed || typeof parsed !== "object") return DEFAULT_VIEW;
    const lat = Number(parsed.lat);
    const lon = Number(parsed.lon);
    const zoom = Number(parsed.zoom);
    if (!Number.isFinite(lat) || !Number.isFinite(lon) || !Number.isFinite(zoom)) return DEFAULT_VIEW;
    return {
      lat: clamp(lat, -85, 85),
      lon: clamp(lon, -180, 180),
      zoom: clamp(Math.round(zoom), 2, 14)
    };
  } catch {
    return DEFAULT_VIEW;
  }
}

function persistView(view) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(MAP_VIEW_KEY, JSON.stringify(view));
  } catch {
    // ignore storage issues
  }
}

function getBBox({ lat, lon, zoom }) {
  const lonSpan = clamp(180 / Math.pow(2, zoom - 2), 0.4, 180);
  const latSpan = lonSpan * 0.55;
  const west = clamp(lon - lonSpan, -180, 180);
  const east = clamp(lon + lonSpan, -180, 180);
  const south = clamp(lat - latSpan, -85, 85);
  const north = clamp(lat + latSpan, -85, 85);
  return [west, south, east, north];
}

function toEmbedUrl(view) {
  const [west, south, east, north] = getBBox(view);
  const params = new URLSearchParams({
    bbox: `${west},${south},${east},${north}`,
    layer: "mapnik",
    marker: `${view.lat},${view.lon}`
  });
  return `https://www.openstreetmap.org/export/embed.html?${params.toString()}`;
}

function WallpaperMap() {
  const [view, setView] = createSignal(readSavedView());
  const [query, setQuery] = createSignal("");
  const [results, setResults] = createSignal([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal("");
  const [lastJump, setLastJump] = createSignal("");

  const mapSrc = createMemo(() => toEmbedUrl(view()));

  const updateView = (patch) => {
    setView((prev) => {
      const nextRaw = typeof patch === "function" ? patch(prev) : { ...prev, ...patch };
      const next = {
        lat: clamp(Number(nextRaw.lat), -85, 85),
        lon: clamp(Number(nextRaw.lon), -180, 180),
        zoom: clamp(Math.round(Number(nextRaw.zoom)), 2, 14)
      };
      persistView(next);
      return next;
    });
  };

  const runSearch = async () => {
    const needle = query().trim();
    if (!needle) {
      setResults([]);
      return;
    }
    setLoading(true);
    setError("");
    try {
      const response = await fetch(`/api/geocode?q=${encodeURIComponent(needle)}&limit=8`);
      const payload = await response.json().catch(() => ({}));
      if (!response.ok || !payload?.ok) {
        throw new Error(String(payload?.error || "Search failed"));
      }
      const next = Array.isArray(payload.results)
        ? payload.results.map((item) => ({
          name: String(item?.name || ""),
          lat: Number(item?.lat),
          lon: Number(item?.lon)
        })).filter((item) => item.name && Number.isFinite(item.lat) && Number.isFinite(item.lon))
        : [];
      setResults(next);
    } catch (searchError) {
      setError(searchError instanceof Error ? searchError.message : "Search failed");
      setResults([]);
    } finally {
      setLoading(false);
    }
  };

  const jumpTo = (item) => {
    updateView({ lat: item.lat, lon: item.lon });
    setLastJump(item.name);
  };

  return (
    <Show when={preferences().wallpaperWidgets.map}>
      <div class="pointer-events-none absolute inset-0 z-0 overflow-hidden">
        <iframe
          src={mapSrc()}
          title="Wallpaper map"
          loading="lazy"
          class="pointer-events-auto absolute inset-0 h-full w-full border-0"
          style={{
            filter: "grayscale(1) invert(0.92) hue-rotate(165deg) saturate(0.45) brightness(0.48) contrast(1.15)"
          }}
        />
        <div
          class="pointer-events-none absolute inset-0"
          style={{
            background:
              "linear-gradient(160deg, rgba(18,40,52,0.42), rgba(8,11,18,0.62) 45%, rgba(47,18,40,0.38)), radial-gradient(900px 600px at 15% 10%, rgba(58,116,146,0.25), transparent), radial-gradient(1000px 700px at 85% 85%, rgba(143,63,106,0.2), transparent)"
          }}
        />

        <section class="pointer-events-auto absolute left-3 top-3 w-[min(420px,calc(100vw-24px))] rounded-lg border border-neutral-800/80 bg-[#0b0f15]/82 p-2 shadow-2xl backdrop-blur-md">
          <div class="flex items-center gap-2">
            <TbOutlineMapSearch size={16} class="text-[hsl(var(--primary))]" />
            <input
              type="text"
              value={query()}
              onInput={(event) => setQuery(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void runSearch();
              }}
              placeholder="Search place and jump..."
              class="flex-1 rounded-md border border-neutral-700 bg-neutral-950/70 px-2 py-1.5 text-xs text-neutral-100 outline-none placeholder:text-neutral-500 focus:border-[hsl(var(--primary))]"
            />
            <button
              type="button"
              onClick={() => void runSearch()}
              class="rounded-md border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-xs text-neutral-200 transition-colors hover:border-[hsl(var(--primary))] hover:text-[hsl(var(--primary))]"
            >
              Find
            </button>
          </div>

          <div class="mt-2 flex items-center justify-between text-[11px] text-neutral-400">
            <span class="truncate">{lastJump() ? `Jumped: ${lastJump()}` : "Drag map to explore"}</span>
            <div class="flex items-center gap-1">
              <button
                type="button"
                title="Zoom out"
                onClick={() => updateView((prev) => ({ ...prev, zoom: prev.zoom - 1 }))}
                class="rounded border border-neutral-700 bg-neutral-900 p-1 text-neutral-300 transition-colors hover:border-[hsl(var(--primary))] hover:text-[hsl(var(--primary))]"
              >
                <TbOutlineMinus size={13} />
              </button>
              <span class="min-w-7 text-center font-mono text-neutral-500">{view().zoom}</span>
              <button
                type="button"
                title="Zoom in"
                onClick={() => updateView((prev) => ({ ...prev, zoom: prev.zoom + 1 }))}
                class="rounded border border-neutral-700 bg-neutral-900 p-1 text-neutral-300 transition-colors hover:border-[hsl(var(--primary))] hover:text-[hsl(var(--primary))]"
              >
                <TbOutlinePlus size={13} />
              </button>
              <button
                type="button"
                title="Reset center"
                onClick={() => {
                  updateView(DEFAULT_VIEW);
                  setLastJump("Default view");
                }}
                class="rounded border border-neutral-700 bg-neutral-900 p-1 text-neutral-300 transition-colors hover:border-[hsl(var(--primary))] hover:text-[hsl(var(--primary))]"
              >
                <TbOutlineTarget size={13} />
              </button>
            </div>
          </div>

          <Show when={loading()}>
            <p class="mt-2 text-xs text-neutral-500">Searching...</p>
          </Show>
          <Show when={error()}>
            <p class="mt-2 text-xs text-red-300">{error()}</p>
          </Show>
          <Show when={results().length > 0}>
            <div class="mt-2 max-h-48 space-y-1 overflow-auto pr-1">
              <For each={results()}>
                {(item) => (
                  <button
                    type="button"
                    class="w-full rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1 text-left text-xs text-neutral-200 transition-colors hover:border-[hsl(var(--primary))] hover:text-[hsl(var(--primary))]"
                    onClick={() => jumpTo(item)}
                    title={item.name}
                  >
                    <div class="truncate">{item.name}</div>
                    <div class="font-mono text-[10px] text-neutral-500">
                      {item.lat.toFixed(4)}, {item.lon.toFixed(4)}
                    </div>
                  </button>
                )}
              </For>
            </div>
          </Show>
        </section>
      </div>
    </Show>
  );
}

export default WallpaperMap;
