import { For, Show, createMemo, createSignal } from "solid-js";
import {
  TbOutlineBook2,
  TbOutlineClock,
  TbOutlineRefresh,
  TbOutlineSettings
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
import { openWindow } from "../../stores/windows";

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
    <div class={`h-full overflow-auto text-neutral-200 ${compact() ? "" : "bg-[#1a1a1a] p-4"}`}>
      <div class="border-b border-neutral-800 px-3 py-2">
        <h2 class="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-neutral-300">
          <TbOutlineSettings size={18} />
          Settings
        </h2>
        <p class="mt-1 text-xs text-neutral-500">Basic runtime preferences only.</p>
      </div>

      <div class="space-y-2 p-3">
        <section class="rounded-md border border-neutral-800 bg-neutral-900/60 p-3">
          <div class="mb-2 flex items-center gap-2">
            <TbOutlineClock size={15} class="text-neutral-300" />
            <p class="text-sm font-medium text-neutral-100">Time & Locale</p>
          </div>
          <label class="mb-2 block text-xs text-neutral-400">Timezone</label>
          <select
            class="mb-3 w-full rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
            value={preferences().timezone}
            onChange={(event) => setTimezone(event.currentTarget.value)}
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
            />
          </label>
          <p class="mt-2 text-[11px] text-neutral-500">Preview: {nowPreview()}</p>
        </section>

        <section class="rounded-md border border-neutral-800 bg-neutral-900/60 p-3 text-xs text-neutral-300">
          <p class="mb-2">Wallpaper widgets are managed in Widgets panel.</p>
          <button
            type="button"
            class="inline-flex h-7 items-center rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
            onClick={() => openWindow("widgets")}
          >
            Open Widgets
          </button>
        </section>

        <section class="rounded-md border border-neutral-800 bg-neutral-900/60 p-3">
          <div class="mb-2 flex items-center gap-2">
            <TbOutlineBook2 size={15} class="text-neutral-300" />
            <p class="text-sm font-medium text-neutral-100">Bookmarks</p>
          </div>
          <div class="mb-2 grid grid-cols-1 gap-2">
            <input
              type="text"
              value={bookmarkLabel()}
              onInput={(event) => setBookmarkLabel(event.currentTarget.value)}
              placeholder="Label"
              class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
            />
            <input
              type="text"
              value={bookmarkUrl()}
              onInput={(event) => setBookmarkUrl(event.currentTarget.value)}
              placeholder="https://example.com"
              class="rounded-md border border-neutral-700 bg-neutral-950 px-2 py-1.5 text-xs text-neutral-100"
            />
            <button
              type="button"
              class="inline-flex h-7 items-center rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
              onClick={saveBookmark}
            >
              Add bookmark
            </button>
          </div>
          <div class="max-h-40 space-y-1 overflow-auto pr-1">
            <For each={preferences().bookmarks}>
              {(item) => (
                <div class="flex items-center justify-between rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1 text-xs">
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
            </For>
          </div>
        </section>

        <section class="rounded-md border border-neutral-800 bg-neutral-900/60 p-3 text-xs text-neutral-300">
          <p class="mb-2">Integrations and AI providers are managed in Integrations.</p>
          <button
            type="button"
            class="inline-flex h-7 items-center rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
            onClick={() => openWorkflowIntegrations("qwen")}
          >
            Open Integrations
          </button>
        </section>
      </div>

      <div class="mt-3 flex items-center justify-between">
        <Show when={status()}>
          <p class="text-xs text-cyan-300">{status()}</p>
        </Show>
        <button
          type="button"
          class="inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-300 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
          onClick={() => {
            resetPreferences();
            setStatus("Settings reset.");
          }}
        >
          <TbOutlineRefresh size={12} />
          Reset
        </button>
      </div>
    </div>
  );
}

export default SettingsPanel;
