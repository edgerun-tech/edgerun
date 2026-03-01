import { For, Show, createMemo, createSignal } from "solid-js";
import {
  TbOutlineBook2,
  TbOutlineClock,
  TbOutlineRefresh,
  TbOutlineSettings,
  TbOutlineSparkles,
} from "solid-icons/tb";
import {
  addBookmark,
  preferences,
  removeBookmark,
  resetPreferences,
  setTimezone,
  setUse24HourClock,
} from "../../stores/preferences";
import { openWorkflowIntegrations } from "../../stores/workflow-ui";
import WidgetSettingsSection from "./WidgetSettingsSection";
import VirtualAnimatedList from "../common/VirtualAnimatedList";

const timezoneOptions = [
  { value: "local", label: "System default" },
  { value: "UTC", label: "UTC" },
  { value: "America/New_York", label: "New York (ET)" },
  { value: "America/Chicago", label: "Chicago (CT)" },
  { value: "America/Denver", label: "Denver (MT)" },
  { value: "America/Los_Angeles", label: "Los Angeles (PT)" },
  { value: "Europe/London", label: "London" },
  { value: "Europe/Berlin", label: "Berlin" },
  { value: "Asia/Tokyo", label: "Tokyo" }
];

function SettingsPanel(props) {
  const compact = () => Boolean(props?.compact);
  let bookmarkListRef;
  const [bookmarkLabel, setBookmarkLabel] = createSignal("");
  const [bookmarkUrl, setBookmarkUrl] = createSignal("");
  const [status, setStatus] = createSignal("");

  const nowPreview = createMemo(() => {
    try {
      return new Intl.DateTimeFormat("en-US", {
        hour: "2-digit",
        minute: "2-digit",
        hour12: !preferences().use24HourClock,
        timeZone: preferences().timezone === "local" ? undefined : preferences().timezone
      }).format(new Date());
    } catch {
      return new Date().toLocaleTimeString();
    }
  });

  const saveBookmark = () => {
    const label = bookmarkLabel().trim();
    const urlRaw = bookmarkUrl().trim();
    if (!label || !urlRaw) {
      setStatus("Bookmark label and URL are required.");
      return;
    }
    const url = /^https?:\/\//i.test(urlRaw) ? urlRaw : `https://${urlRaw}`;
    addBookmark({ label, url });
    setBookmarkLabel("");
    setBookmarkUrl("");
    setStatus("Bookmark added.");
  };

  return (
    <div
      class={`h-full overflow-auto text-neutral-200 ${compact() ? "" : "bg-[#161616] p-4"}`}
      data-testid="settings-panel"
    >
      <header class="rounded-xl border border-neutral-800 bg-neutral-900/60 px-4 py-3">
        <h2 class="flex items-center gap-2 text-sm font-semibold uppercase tracking-wide text-neutral-100">
          <TbOutlineSettings size={18} />
          Settings
        </h2>
        <p class="mt-1 text-xs text-neutral-500">
          Keep runtime preferences, widgets, and bookmarks in one place.
        </p>
      </header>

      <div class="mt-3 space-y-3">
        <section class="rounded-lg border border-neutral-800 bg-neutral-900/60 p-3">
          <div class="mb-2 flex items-center gap-2">
            <TbOutlineClock size={15} class="text-neutral-300" />
            <p class="text-sm font-semibold text-neutral-100">Time & locale</p>
          </div>
          <label class="mb-2 block text-xs text-neutral-400">Timezone</label>
          <select
            class="mb-3 w-full rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
            value={preferences().timezone}
            onChange={(event) => setTimezone(event.currentTarget.value)}
            data-testid="settings-timezone-select"
          >
            <For each={timezoneOptions}>
              {(option) => <option value={option.value}>{option.label}</option>}
            </For>
          </select>
          <label class="flex items-center justify-between text-xs text-neutral-300">
            <span>Use 24-hour clock</span>
            <input
              type="checkbox"
              checked={preferences().use24HourClock}
              onInput={(event) => setUse24HourClock(event.currentTarget.checked)}
              style={{ "accent-color": "hsl(var(--primary))" }}
              data-testid="settings-clock-format-toggle"
            />
          </label>
          <p class="mt-2 text-[11px] text-neutral-500">Preview: {nowPreview()}</p>
        </section>

        <WidgetSettingsSection />

        <section class="rounded-lg border border-neutral-800 bg-neutral-900/60 p-3">
          <div class="mb-2 flex items-center gap-2">
            <TbOutlineBook2 size={15} class="text-neutral-300" />
            <p class="text-sm font-semibold text-neutral-100">Bookmarks</p>
          </div>
          <div class="mb-2 grid grid-cols-1 gap-2">
            <input
              type="text"
              value={bookmarkLabel()}
              onInput={(event) => setBookmarkLabel(event.currentTarget.value)}
              placeholder="Label"
              class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
              data-testid="settings-bookmark-label"
            />
            <input
              type="text"
              value={bookmarkUrl()}
              onInput={(event) => setBookmarkUrl(event.currentTarget.value)}
              placeholder="https://example.com"
              class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
              data-testid="settings-bookmark-url"
            />
            <button
              type="button"
              class="inline-flex h-7 items-center rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
              onClick={saveBookmark}
              data-testid="settings-bookmark-add"
            >
              Add bookmark
            </button>
          </div>
          <div class="max-h-40 overflow-auto pr-1" data-testid="settings-bookmark-list" ref={bookmarkListRef}>
            <VirtualAnimatedList
              items={() => preferences().bookmarks}
              estimateSize={34}
              overscan={4}
              containerRef={() => bookmarkListRef}
              animateRows
              renderItem={(item) => (
                <div class="mt-1 flex items-center justify-between rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1 text-xs">
                  <div class="min-w-0">
                    <p class="truncate text-neutral-200">{item.label}</p>
                    <p class="truncate text-neutral-500">{item.url}</p>
                  </div>
                  <button
                    type="button"
                    class="ml-2 rounded border border-neutral-700 px-1.5 py-0.5 text-[10px] text-neutral-300 hover:bg-neutral-800"
                    onClick={() => removeBookmark(item.id)}
                  >
                    Remove
                  </button>
                </div>
              )}
            />
          </div>
        </section>

        <section class="rounded-lg border border-neutral-800 bg-neutral-900/60 p-3 text-xs text-neutral-300">
          <div class="mb-2 flex items-center gap-2">
            <TbOutlineSparkles size={15} class="text-neutral-300" />
            <p class="text-sm font-semibold text-neutral-100">Integrations</p>
          </div>
          <p class="mb-2 text-neutral-400">Manage integrations and AI providers in the Integrations flow.</p>
          <button
            type="button"
            class="inline-flex h-7 items-center rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
            onClick={() => openWorkflowIntegrations("opencode_cli")}
            data-testid="settings-open-integrations"
          >
            Open Integrations
          </button>
        </section>
      </div>

      <div class="mt-3 flex items-center justify-between gap-2">
        <Show when={status()}>
          <p class="text-xs text-cyan-300" data-testid="settings-status">{status()}</p>
        </Show>
        <button
          type="button"
          class="inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-300 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
          onClick={() => {
            resetPreferences();
            setStatus("Settings reset.");
          }}
          data-testid="settings-reset"
        >
          <TbOutlineRefresh size={12} />
          Reset all
        </button>
      </div>
    </div>
  );
}

export default SettingsPanel;
