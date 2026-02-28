import { TbOutlineApps, TbOutlineMap2, TbOutlineClock, TbOutlineCloud, TbOutlineBook2, TbOutlineRotateClockwise2 } from "solid-icons/tb";
import { preferences, setWallpaperWidgetEnabled } from "../../stores/preferences";
import { openWindow } from "../../stores/windows";
import { resetWidgetPositions } from "../../stores/ui-actions";

const WIDGET_POSITIONS_KEY = "intent-ui-widget-positions-v1";

function WidgetsPanel() {
  return (
    <div class="h-full overflow-auto bg-[#1a1a1a] p-4 text-neutral-200">
      <div class="mb-4">
        <h2 class="flex items-center gap-2 text-lg font-semibold text-white">
          <TbOutlineApps size={18} />
          Widgets
        </h2>
        <p class="mt-1 text-xs text-neutral-500">Enable widgets and drag them on wallpaper to place.</p>
      </div>

      <section class="rounded-lg border border-neutral-800 bg-neutral-900/60 p-3">
        <div class="space-y-2 text-xs">
          <label class="flex items-center justify-between gap-3">
            <span class="inline-flex items-center gap-2">
              <TbOutlineMap2 size={14} class="text-[hsl(var(--primary))]" />
              Map wallpaper
            </span>
            <input
              type="checkbox"
              checked={preferences().wallpaperWidgets.map}
              onInput={(event) => setWallpaperWidgetEnabled("map", event.currentTarget.checked)}
              style={{ "accent-color": "hsl(var(--primary))" }}
            />
          </label>
          <label class="flex items-center justify-between gap-3">
            <span class="inline-flex items-center gap-2">
              <TbOutlineClock size={14} class="text-[hsl(var(--primary))]" />
              Clock
            </span>
            <input
              type="checkbox"
              checked={preferences().wallpaperWidgets.clock}
              onInput={(event) => setWallpaperWidgetEnabled("clock", event.currentTarget.checked)}
              style={{ "accent-color": "hsl(var(--primary))" }}
            />
          </label>
          <label class="flex items-center justify-between gap-3">
            <span class="inline-flex items-center gap-2">
              <TbOutlineCloud size={14} class="text-[hsl(var(--primary))]" />
              Weather
            </span>
            <input
              type="checkbox"
              checked={preferences().wallpaperWidgets.weather}
              onInput={(event) => setWallpaperWidgetEnabled("weather", event.currentTarget.checked)}
              style={{ "accent-color": "hsl(var(--primary))" }}
            />
          </label>
          <label class="flex items-center justify-between gap-3">
            <span class="inline-flex items-center gap-2">
              <TbOutlineBook2 size={14} class="text-[hsl(var(--primary))]" />
              Bookmarks
            </span>
            <input
              type="checkbox"
              checked={preferences().wallpaperWidgets.bookmarks}
              onInput={(event) => setWallpaperWidgetEnabled("bookmarks", event.currentTarget.checked)}
              style={{ "accent-color": "hsl(var(--primary))" }}
            />
          </label>
        </div>
      </section>

      <div class="mt-3">
        <button
          type="button"
          class="mr-2 inline-flex items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 py-1 text-xs text-neutral-300 hover:bg-neutral-800"
          onClick={() => openWindow("onvif")}
        >
          Open ONVIF Cameras
        </button>
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 py-1 text-xs text-neutral-300 hover:bg-neutral-800"
          onClick={() => {
            try {
              localStorage.removeItem(WIDGET_POSITIONS_KEY);
              resetWidgetPositions();
            } catch {
              // ignore local storage failures
            }
          }}
        >
          <TbOutlineRotateClockwise2 size={12} />
          Reset widget positions
        </button>
      </div>
    </div>
  );
}

export default WidgetsPanel;
