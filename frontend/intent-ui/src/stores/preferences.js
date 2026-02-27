import { createSignal } from "solid-js";

const PREFS_KEY = "intent-ui-preferences-v1";

const DEFAULT_PREFERENCES = {
  timezone: "local",
  use24HourClock: true,
  wallpaperWidgets: {
    map: false,
    clock: false,
    weather: false,
    bookmarks: false
  },
  bookmarks: [
    { id: "bm-1", label: "GitHub", url: "https://github.com" },
    { id: "bm-2", label: "Gmail", url: "https://mail.google.com" },
    { id: "bm-3", label: "Drive", url: "https://drive.google.com" }
  ]
};

function clampBookmarks(list) {
  if (!Array.isArray(list)) return DEFAULT_PREFERENCES.bookmarks;
  return list
    .map((item, index) => ({
      id: String(item?.id || `bm-${Date.now()}-${index}`),
      label: String(item?.label || "").trim().slice(0, 64),
      url: String(item?.url || "").trim().slice(0, 512)
    }))
    .filter((item) => item.label && item.url)
    .slice(0, 24);
}

function loadPreferences() {
  if (typeof localStorage === "undefined") return DEFAULT_PREFERENCES;
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (!raw) return DEFAULT_PREFERENCES;
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return DEFAULT_PREFERENCES;
    return {
      timezone: typeof parsed.timezone === "string" && parsed.timezone.trim() ? parsed.timezone : DEFAULT_PREFERENCES.timezone,
      use24HourClock: Boolean(parsed.use24HourClock),
      wallpaperWidgets: {
        map: parsed?.wallpaperWidgets?.map === true,
        clock: parsed?.wallpaperWidgets?.clock === true,
        weather: parsed?.wallpaperWidgets?.weather === true,
        bookmarks: parsed?.wallpaperWidgets?.bookmarks === true
      },
      bookmarks: clampBookmarks(parsed.bookmarks)
    };
  } catch {
    return DEFAULT_PREFERENCES;
  }
}

function persistPreferences(next) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(next));
  } catch {
    // ignore storage failures
  }
}

const [preferences, setPreferences] = createSignal(loadPreferences());

function updatePreferences(patch) {
  setPreferences((prev) => {
    const next = typeof patch === "function" ? patch(prev) : { ...prev, ...patch };
    persistPreferences(next);
    return next;
  });
}

function setTimezone(timezone) {
  updatePreferences((prev) => ({ ...prev, timezone: String(timezone || "local") }));
}

function setUse24HourClock(enabled) {
  updatePreferences((prev) => ({ ...prev, use24HourClock: Boolean(enabled) }));
}

function setWallpaperWidgetEnabled(widget, enabled) {
  if (!["map", "clock", "weather", "bookmarks"].includes(widget)) return;
  updatePreferences((prev) => ({
    ...prev,
    wallpaperWidgets: {
      ...prev.wallpaperWidgets,
      [widget]: Boolean(enabled)
    }
  }));
}

function addBookmark(bookmark) {
  updatePreferences((prev) => ({
    ...prev,
    bookmarks: clampBookmarks([
      ...prev.bookmarks,
      {
        id: `bm-${Date.now()}`,
        label: bookmark?.label,
        url: bookmark?.url
      }
    ])
  }));
}

function removeBookmark(id) {
  updatePreferences((prev) => ({
    ...prev,
    bookmarks: prev.bookmarks.filter((item) => item.id !== id)
  }));
}

function resetPreferences() {
  updatePreferences({ ...DEFAULT_PREFERENCES });
}

export {
  preferences,
  setTimezone,
  setUse24HourClock,
  setWallpaperWidgetEnabled,
  addBookmark,
  removeBookmark,
  resetPreferences
};
