import {
  TbOutlineBook2,
  TbOutlineClock,
  TbOutlineCloud,
  TbOutlineDeviceDesktop,
  TbOutlineMap2,
  TbOutlineRotateClockwise2,
} from "solid-icons/tb";
import { preferences, setWallpaperWidgetEnabled } from "../../stores/preferences";
import { resetWidgetPositions } from "../../stores/ui-actions";
import { openWindow } from "../../stores/windows";

const WIDGET_POSITIONS_KEY = "intent-ui-widget-positions-v1";

function WidgetToggleRow(props) {
  return (
    <label class="flex items-center justify-between gap-3 rounded-md border border-neutral-800 bg-neutral-950/70 px-3 py-2 text-xs text-neutral-200">
      <span class="inline-flex items-center gap-2">
        <props.icon size={14} class="text-[hsl(var(--primary))]" />
        {props.label}
      </span>
      <input
        type="checkbox"
        checked={props.checked}
        onInput={(event) => props.onChange(event.currentTarget.checked)}
        style={{ "accent-color": "hsl(var(--primary))" }}
        data-testid={props.testId}
      />
    </label>
  );
}

function WidgetSettingsSection() {
  return (
    <section class="rounded-lg border border-neutral-800 bg-neutral-900/60 p-3" data-testid="settings-widgets-section">
      <div class="mb-3">
        <h3 class="text-sm font-semibold text-neutral-100">Wallpaper widgets</h3>
        <p class="mt-1 text-xs text-neutral-500">Enable widgets and drag them on wallpaper to place.</p>
      </div>

      <div class="space-y-2">
        <WidgetToggleRow
          icon={TbOutlineMap2}
          label="Map wallpaper"
          checked={preferences().wallpaperWidgets.map}
          onChange={(enabled) => setWallpaperWidgetEnabled("map", enabled)}
          testId="settings-widget-toggle-map"
        />
        <WidgetToggleRow
          icon={TbOutlineClock}
          label="Clock"
          checked={preferences().wallpaperWidgets.clock}
          onChange={(enabled) => setWallpaperWidgetEnabled("clock", enabled)}
          testId="settings-widget-toggle-clock"
        />
        <WidgetToggleRow
          icon={TbOutlineCloud}
          label="Weather"
          checked={preferences().wallpaperWidgets.weather}
          onChange={(enabled) => setWallpaperWidgetEnabled("weather", enabled)}
          testId="settings-widget-toggle-weather"
        />
        <WidgetToggleRow
          icon={TbOutlineBook2}
          label="Bookmarks"
          checked={preferences().wallpaperWidgets.bookmarks}
          onChange={(enabled) => setWallpaperWidgetEnabled("bookmarks", enabled)}
          testId="settings-widget-toggle-bookmarks"
        />
      </div>

      <div class="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          class="inline-flex h-7 items-center rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
          onClick={() => openWindow("onvif")}
          data-testid="settings-open-onvif"
        >
          <TbOutlineDeviceDesktop size={12} class="mr-1" />
          Open ONVIF Cameras
        </button>
        <button
          type="button"
          class="inline-flex h-7 items-center rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
          onClick={() => {
            try {
              localStorage.removeItem(WIDGET_POSITIONS_KEY);
              resetWidgetPositions();
            } catch {
              // ignore storage failures
            }
          }}
          data-testid="settings-reset-widget-positions"
        >
          <TbOutlineRotateClockwise2 size={12} class="mr-1" />
          Reset widget positions
        </button>
      </div>
    </section>
  );
}

export default WidgetSettingsSection;
