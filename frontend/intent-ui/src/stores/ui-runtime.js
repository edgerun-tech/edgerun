import { createSignal } from "solid-js";

const ACCENT_INDEX_KEY = "intent-ui-accent-index";
const WEATHER_COORDS_KEY = "intent-ui-weather-coords";

const DEFAULT_STATE = {
  accentIndex: 0,
  weatherCoords: {
    lat: 40.7128,
    lon: -74.006,
    location: "New York, US"
  }
};
const DEFAULT_WEATHER_COORDS = DEFAULT_STATE.weatherCoords;

function loadAccentIndex() {
  if (typeof localStorage === "undefined") return DEFAULT_STATE.accentIndex;
  try {
    const raw = localStorage.getItem(ACCENT_INDEX_KEY);
    const parsed = Number.parseInt(raw || "0", 10);
    return Number.isFinite(parsed) && parsed >= 0 ? parsed : DEFAULT_STATE.accentIndex;
  } catch {
    return DEFAULT_STATE.accentIndex;
  }
}

function loadWeatherCoords() {
  if (typeof localStorage === "undefined") return DEFAULT_STATE.weatherCoords;
  try {
    const raw = localStorage.getItem(WEATHER_COORDS_KEY);
    if (!raw) return DEFAULT_STATE.weatherCoords;
    const parsed = JSON.parse(raw);
    if (!Number.isFinite(parsed?.lat) || !Number.isFinite(parsed?.lon)) return DEFAULT_STATE.weatherCoords;
    return {
      lat: parsed.lat,
      lon: parsed.lon,
      location: typeof parsed.location === "string" && parsed.location ? parsed.location : "Current location"
    };
  } catch {
    return DEFAULT_STATE.weatherCoords;
  }
}

const [uiRuntime, setUiRuntime] = createSignal({
  accentIndex: loadAccentIndex(),
  weatherCoords: loadWeatherCoords()
});

function setRuntimeAccentIndex(index, totalOptions = 1) {
  const max = Math.max(1, totalOptions);
  const next = Number.isFinite(index) ? Math.max(0, Math.min(max - 1, Math.trunc(index))) : 0;
  setUiRuntime((prev) => ({ ...prev, accentIndex: next }));
  if (typeof localStorage === "undefined") return next;
  try {
    localStorage.setItem(ACCENT_INDEX_KEY, String(next));
  } catch {
    // ignore storage failures
  }
  return next;
}

function setRuntimeWeatherCoords(coords) {
  if (!Number.isFinite(coords?.lat) || !Number.isFinite(coords?.lon)) return uiRuntime().weatherCoords;
  const next = {
    lat: coords.lat,
    lon: coords.lon,
    location: typeof coords.location === "string" && coords.location ? coords.location : "Current location"
  };
  setUiRuntime((prev) => ({ ...prev, weatherCoords: next }));
  if (typeof localStorage !== "undefined") {
    try {
      localStorage.setItem(WEATHER_COORDS_KEY, JSON.stringify(next));
    } catch {
      // ignore storage failures
    }
  }
  return next;
}

function getWorkflowLiveActivity(state) {
  const latest = Array.isArray(state?.statusEvents) ? state.statusEvents[state.statusEvents.length - 1] : null;
  if (state?.streaming) {
    return {
      working: true,
      text: latest?.detail || "Streaming response..."
    };
  }
  if (latest?.detail) {
    return {
      working: false,
      text: latest.detail
    };
  }
  if (Array.isArray(state?.messages) && state.messages.length > 0) {
    return {
      working: false,
      text: "Response ready."
    };
  }
  return {
    working: false,
    text: "Ready"
  };
}

export {
  DEFAULT_WEATHER_COORDS,
  DEFAULT_STATE,
  getWorkflowLiveActivity,
  setRuntimeAccentIndex,
  setRuntimeWeatherCoords,
  uiRuntime
};
