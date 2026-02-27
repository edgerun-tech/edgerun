import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { TbOutlineBook2, TbOutlineCloud, TbOutlineClock, TbOutlineGripVertical } from "solid-icons/tb";
import { openWindow } from "../../stores/windows";
import { preferences } from "../../stores/preferences";

const WEATHER_SNAPSHOT_KEY = "intent-ui-weather-snapshot-v1";
const WIDGET_POSITIONS_KEY = "intent-ui-widget-positions-v1";
const WIDGET_WIDTH = 240;

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function fallbackPositions() {
  const width = typeof window !== "undefined" ? window.innerWidth : 1280;
  return {
    clock: { x: Math.max(8, width - WIDGET_WIDTH - 12), y: 12 },
    weather: { x: Math.max(8, width - WIDGET_WIDTH - 12), y: 86 },
    bookmarks: { x: Math.max(8, width - WIDGET_WIDTH - 12), y: 152 }
  };
}

function normalizePosition(position, viewportWidth, viewportHeight) {
  const x = Number(position?.x);
  const y = Number(position?.y);
  return {
    x: clamp(Number.isFinite(x) ? x : 12, 8, Math.max(8, viewportWidth - WIDGET_WIDTH - 8)),
    y: clamp(Number.isFinite(y) ? y : 12, 8, Math.max(8, viewportHeight - 80))
  };
}

function readWidgetPositions() {
  if (typeof localStorage === "undefined" || typeof window === "undefined") return fallbackPositions();
  const fallback = fallbackPositions();
  try {
    const parsed = JSON.parse(localStorage.getItem(WIDGET_POSITIONS_KEY) || "null");
    if (!parsed || typeof parsed !== "object") return fallback;
    return {
      clock: normalizePosition(parsed.clock, window.innerWidth, window.innerHeight),
      weather: normalizePosition(parsed.weather, window.innerWidth, window.innerHeight),
      bookmarks: normalizePosition(parsed.bookmarks, window.innerWidth, window.innerHeight)
    };
  } catch {
    return fallback;
  }
}

function persistWidgetPositions(positions) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(WIDGET_POSITIONS_KEY, JSON.stringify(positions));
  } catch {
    // ignore storage failures
  }
}

function readWeatherSnapshot() {
  if (typeof localStorage === "undefined") return null;
  try {
    const parsed = JSON.parse(localStorage.getItem(WEATHER_SNAPSHOT_KEY) || "null");
    if (!parsed || typeof parsed !== "object") return null;
    if (!Number.isFinite(parsed.temp)) return null;
    return {
      temp: Number(parsed.temp),
      condition: String(parsed.condition || ""),
      location: String(parsed.location || "")
    };
  } catch {
    return null;
  }
}

function WallpaperWidgets() {
  const [now, setNow] = createSignal(new Date());
  const [weather, setWeather] = createSignal(readWeatherSnapshot());
  const [positions, setPositions] = createSignal(readWidgetPositions());
  const [dragging, setDragging] = createSignal(null);

  const formattedTime = createMemo(() => {
    const p = preferences();
    try {
      return new Intl.DateTimeFormat("en-US", {
        hour: "2-digit",
        minute: "2-digit",
        hour12: !p.use24HourClock,
        timeZone: p.timezone === "local" ? undefined : p.timezone
      }).format(now());
    } catch {
      return now().toLocaleTimeString();
    }
  });

  const formattedDate = createMemo(() => {
    const p = preferences();
    try {
      return new Intl.DateTimeFormat("en-US", {
        weekday: "short",
        month: "short",
        day: "numeric",
        timeZone: p.timezone === "local" ? undefined : p.timezone
      }).format(now());
    } catch {
      return now().toLocaleDateString();
    }
  });

  const setWidgetPosition = (id, nextPos) => {
    setPositions((prev) => {
      const next = {
        ...prev,
        [id]: normalizePosition(nextPos, window.innerWidth, window.innerHeight)
      };
      persistWidgetPositions(next);
      return next;
    });
  };

  const beginDrag = (id, event) => {
    if (!(event.currentTarget instanceof HTMLElement)) return;
    const pointerEvent = event;
    pointerEvent.preventDefault();
    const origin = positions()[id] || { x: 12, y: 12 };
    setDragging({
      id,
      pointerStartX: pointerEvent.clientX,
      pointerStartY: pointerEvent.clientY,
      originX: origin.x,
      originY: origin.y
    });
  };

  onMount(() => {
    const timer = window.setInterval(() => setNow(new Date()), 1000);
    const syncWeather = () => setWeather(readWeatherSnapshot());
    const storageHandler = (event) => {
      if (!event.key || event.key === WEATHER_SNAPSHOT_KEY) syncWeather();
    };
    const moveHandler = (event) => {
      const active = dragging();
      if (!active) return;
      const deltaX = event.clientX - active.pointerStartX;
      const deltaY = event.clientY - active.pointerStartY;
      setWidgetPosition(active.id, {
        x: active.originX + deltaX,
        y: active.originY + deltaY
      });
    };
    const endHandler = () => setDragging(null);
    const resizeHandler = () => {
      setPositions((prev) => {
        const next = {
          clock: normalizePosition(prev.clock, window.innerWidth, window.innerHeight),
          weather: normalizePosition(prev.weather, window.innerWidth, window.innerHeight),
          bookmarks: normalizePosition(prev.bookmarks, window.innerWidth, window.innerHeight)
        };
        persistWidgetPositions(next);
        return next;
      });
    };
    const resetPositionsHandler = () => {
      const next = readWidgetPositions();
      setPositions(next);
      persistWidgetPositions(next);
    };

    window.addEventListener("storage", storageHandler);
    window.addEventListener("pointermove", moveHandler);
    window.addEventListener("pointerup", endHandler);
    window.addEventListener("resize", resizeHandler);
    window.addEventListener("intent:widgets:reset-positions", resetPositionsHandler);
    const weatherTimer = window.setInterval(syncWeather, 60000);
    onCleanup(() => {
      window.clearInterval(timer);
      window.clearInterval(weatherTimer);
      window.removeEventListener("storage", storageHandler);
      window.removeEventListener("pointermove", moveHandler);
      window.removeEventListener("pointerup", endHandler);
      window.removeEventListener("resize", resizeHandler);
      window.removeEventListener("intent:widgets:reset-positions", resetPositionsHandler);
    });
  });

  const handleClass = "mb-1 flex cursor-grab select-none items-center gap-1.5 rounded px-1 py-0.5 text-[10px] uppercase tracking-wide text-neutral-500 active:cursor-grabbing";
  const cardClass = "pointer-events-auto absolute w-60 rounded-md border border-neutral-800/70 bg-neutral-950/52 px-2.5 py-2 shadow-xl backdrop-blur-sm";

  return (
    <aside class="pointer-events-none fixed inset-0 z-[11900]">
      <Show when={preferences().wallpaperWidgets.clock}>
        <section class={cardClass} style={{ left: `${positions().clock.x}px`, top: `${positions().clock.y}px` }}>
          <div class={handleClass} onPointerDown={(event) => beginDrag("clock", event)}>
            <TbOutlineGripVertical size={12} />
            Clock
          </div>
          <div class="flex items-center gap-1.5 text-neutral-200">
            <TbOutlineClock size={13} class="text-[hsl(var(--primary))]" />
            <span class="text-base font-semibold tabular-nums">{formattedTime()}</span>
          </div>
          <p class="text-[11px] text-neutral-500">{formattedDate()}</p>
        </section>
      </Show>

      <Show when={preferences().wallpaperWidgets.weather && weather()}>
        <section class={cardClass} style={{ left: `${positions().weather.x}px`, top: `${positions().weather.y}px` }}>
          <div class={handleClass} onPointerDown={(event) => beginDrag("weather", event)}>
            <TbOutlineGripVertical size={12} />
            Weather
          </div>
          <div class="flex items-center gap-1.5 text-neutral-200">
            <TbOutlineCloud size={13} class="text-[hsl(var(--primary))]" />
            <span class="text-sm font-semibold">{weather().temp}°</span>
            <span class="truncate text-[11px] text-neutral-500">{weather().location}</span>
          </div>
        </section>
      </Show>

      <Show when={preferences().wallpaperWidgets.bookmarks}>
        <section class={cardClass} style={{ left: `${positions().bookmarks.x}px`, top: `${positions().bookmarks.y}px` }}>
          <div class={handleClass} onPointerDown={(event) => beginDrag("bookmarks", event)}>
            <TbOutlineGripVertical size={12} />
            Bookmarks
          </div>
          <div class="mb-1.5 flex items-center gap-1.5 text-neutral-200">
            <TbOutlineBook2 size={13} class="text-[hsl(var(--primary))]" />
            <span class="text-[11px] uppercase tracking-wide text-neutral-500">Quick Links</span>
          </div>
          <div class="space-y-1">
            <For each={preferences().bookmarks.slice(0, 6)}>
              {(item) => (
                <button
                  type="button"
                  class="w-full truncate rounded px-2 py-1 text-left text-xs text-neutral-300 transition-colors hover:bg-neutral-800/60 hover:text-[hsl(var(--primary))]"
                  onClick={() => {
                    openWindow("browser");
                    window.dispatchEvent(new CustomEvent("intent:browser:navigate", { detail: { url: item.url } }));
                  }}
                  title={item.url}
                >
                  {item.label}
                </button>
              )}
            </For>
          </div>
        </section>
      </Show>
    </aside>
  );
}

export default WallpaperWidgets;
