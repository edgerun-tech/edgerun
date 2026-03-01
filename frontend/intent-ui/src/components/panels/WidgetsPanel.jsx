import { TbOutlineApps, TbOutlineSettings } from "solid-icons/tb";
import { openWindow } from "../../stores/windows";
import WidgetSettingsSection from "./WidgetSettingsSection";

function WidgetsPanel() {
  return (
    <div class="h-full overflow-auto bg-[#161616] p-4 text-neutral-200">
      <div class="mb-3 rounded-lg border border-neutral-800 bg-neutral-900/60 p-3">
        <h2 class="flex items-center gap-2 text-sm font-semibold uppercase tracking-wide text-neutral-100">
          <TbOutlineApps size={18} />
          Widgets
        </h2>
        <p class="mt-1 text-xs text-neutral-500">
          Widget controls now live in Settings for a cleaner single control surface.
        </p>
        <button
          type="button"
          class="mt-2 inline-flex h-7 items-center rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
          onClick={() => openWindow("settings")}
          data-testid="widgets-open-settings"
        >
          <TbOutlineSettings size={12} class="mr-1" />
          Open Settings
        </button>
      </div>

      <WidgetSettingsSection />
    </div>
  );
}

export default WidgetsPanel;
