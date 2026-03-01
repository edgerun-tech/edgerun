import { For, Show, createMemo, createSignal, onMount } from "solid-js";
import { FiChevronLeft, FiChevronRight, FiRefreshCw } from "solid-icons/fi";
import { integrationStore } from "../../stores/integrations";
import VirtualAnimatedList from "../common/VirtualAnimatedList";

function startOfMonth(date) {
  return new Date(date.getFullYear(), date.getMonth(), 1);
}

function normalizeDateKey(date) {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function parseEventStart(value) {
  const raw = value?.dateTime || value?.date || "";
  const date = new Date(raw);
  if (Number.isNaN(date.getTime())) return null;
  return date;
}

function eventTimeLabel(event) {
  const start = parseEventStart(event?.start);
  if (!start) return "Unknown time";
  const isAllDay = Boolean(event?.start?.date) && !event?.start?.dateTime;
  if (isAllDay) return "All day";
  return start.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function CalendarPanel() {
  const [monthDate, setMonthDate] = createSignal(startOfMonth(new Date()));
  const [selectedDate, setSelectedDate] = createSignal(new Date());
  const [events, setEvents] = createSignal([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal("");
  const [showSelectedOnly, setShowSelectedOnly] = createSignal(true);
  let eventsListRef;

  const monthLabel = createMemo(() =>
    monthDate().toLocaleDateString("en-US", { month: "long", year: "numeric" })
  );

  const monthCells = createMemo(() => {
    const first = startOfMonth(monthDate());
    const firstWeekday = first.getDay();
    const daysInMonth = new Date(first.getFullYear(), first.getMonth() + 1, 0).getDate();
    const cells = [];
    for (let i = 0; i < firstWeekday; i += 1) cells.push(null);
    for (let day = 1; day <= daysInMonth; day += 1) {
      cells.push(new Date(first.getFullYear(), first.getMonth(), day));
    }
    return cells;
  });

  const eventsByDay = createMemo(() => {
    const map = new Map();
    for (const event of events()) {
      const start = parseEventStart(event?.start);
      if (!start) continue;
      const key = normalizeDateKey(start);
      const existing = map.get(key) || [];
      existing.push(event);
      map.set(key, existing);
    }
    return map;
  });

  const visibleEvents = createMemo(() => {
    const all = events().slice().sort((a, b) => {
      const left = parseEventStart(a?.start)?.getTime() || 0;
      const right = parseEventStart(b?.start)?.getTime() || 0;
      return left - right;
    });
    if (!showSelectedOnly()) return all;
    const key = normalizeDateKey(selectedDate());
    return all.filter((event) => normalizeDateKey(parseEventStart(event?.start) || new Date(0)) === key);
  });

  const loadEvents = async () => {
    const token = String(integrationStore.getToken("google") || localStorage.getItem("google_token") || "").trim();
    if (!token) {
      setEvents([]);
      setError("Google token missing. Connect Google integration to view calendar events.");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const response = await fetch(`/api/google/events?limit=50&token=${encodeURIComponent(token)}`, {
        cache: "no-store"
      });
      const payload = await response.json().catch(() => ({}));
      if (!response.ok || payload?.ok === false) {
        throw new Error(String(payload?.error || `calendar request failed (${response.status})`));
      }
      const next = Array.isArray(payload?.items) ? payload.items : [];
      setEvents(next);
      if (next.length === 0) setError("No calendar events found.");
    } catch (loadError) {
      setEvents([]);
      setError(loadError instanceof Error ? loadError.message : "Failed to load calendar events.");
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    void loadEvents();
  });

  return (
    <div class="flex h-full min-h-0 flex-col bg-[#111317] text-neutral-200" data-testid="calendar-panel">
      <div class="flex items-center justify-between border-b border-neutral-800 px-3 py-2">
        <div>
          <p class="text-xs uppercase tracking-wide text-neutral-400">Calendar</p>
          <p class="text-[11px] text-neutral-500">Google events overview</p>
        </div>
        <button
          type="button"
          class="inline-flex h-7 items-center gap-1 rounded-md border border-neutral-700 bg-neutral-900 px-2 text-[10px] text-neutral-200 hover:border-[hsl(var(--primary)/0.45)]"
          onClick={() => void loadEvents()}
          data-testid="calendar-refresh"
        >
          <FiRefreshCw size={12} />
          Refresh
        </button>
      </div>

      <Show when={error()}>
        <div class="border-b border-neutral-800 bg-red-900/20 px-3 py-2 text-[11px] text-red-200" data-testid="calendar-error">
          {error()}
        </div>
      </Show>

      <div class="grid min-h-0 flex-1 grid-cols-1 gap-3 p-3 lg:grid-cols-[280px_1fr]">
        <section class="rounded border border-neutral-800 bg-neutral-900/45 p-2">
          <div class="mb-2 flex items-center justify-between">
            <button
              type="button"
              class="rounded border border-neutral-700 bg-neutral-900 px-1.5 py-1 text-neutral-300 hover:border-neutral-500"
              onClick={() => {
                const current = monthDate();
                setMonthDate(new Date(current.getFullYear(), current.getMonth() - 1, 1));
              }}
              aria-label="Previous month"
            >
              <FiChevronLeft size={14} />
            </button>
            <p class="text-xs font-medium text-neutral-200">{monthLabel()}</p>
            <button
              type="button"
              class="rounded border border-neutral-700 bg-neutral-900 px-1.5 py-1 text-neutral-300 hover:border-neutral-500"
              onClick={() => {
                const current = monthDate();
                setMonthDate(new Date(current.getFullYear(), current.getMonth() + 1, 1));
              }}
              aria-label="Next month"
            >
              <FiChevronRight size={14} />
            </button>
          </div>
          <div class="grid grid-cols-7 gap-1 text-center text-[10px] text-neutral-500">
            <For each={["S", "M", "T", "W", "T", "F", "S"]}>{(day) => <span>{day}</span>}</For>
          </div>
          <div class="mt-1 grid grid-cols-7 gap-1">
            <For each={monthCells()}>
              {(cell) => {
                if (!cell) return <span class="h-8" />;
                const dateKey = normalizeDateKey(cell);
                const selected = dateKey === normalizeDateKey(selectedDate());
                const hasEvents = (eventsByDay().get(dateKey) || []).length > 0;
                return (
                  <button
                    type="button"
                    class={`relative h-8 rounded text-[11px] ${selected ? "border border-[hsl(var(--primary)/0.7)] bg-[hsl(var(--primary)/0.2)] text-white" : "border border-transparent bg-neutral-900/80 text-neutral-300 hover:border-neutral-700"}`}
                    onClick={() => setSelectedDate(cell)}
                    data-testid={selected ? "calendar-day-selected" : void 0}
                  >
                    {cell.getDate()}
                    <Show when={hasEvents}>
                      <span class="absolute bottom-1 left-1/2 h-1 w-1 -translate-x-1/2 rounded-full bg-cyan-300" />
                    </Show>
                  </button>
                );
              }}
            </For>
          </div>
          <button
            type="button"
            class="mt-2 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-[10px] text-neutral-300 hover:border-[hsl(var(--primary)/0.45)]"
            onClick={() => setShowSelectedOnly((prev) => !prev)}
            data-testid="calendar-toggle-filter"
          >
            {showSelectedOnly() ? "Show all upcoming" : "Show selected day only"}
          </button>
        </section>

        <section class="min-h-0 rounded border border-neutral-800 bg-neutral-900/45 p-2">
          <div class="mb-2 flex items-center justify-between">
            <p class="text-xs uppercase tracking-wide text-neutral-500">Events</p>
            <p class="text-[10px] text-neutral-500">{visibleEvents().length} items</p>
          </div>
          <div class="max-h-full overflow-auto" ref={eventsListRef} data-testid="calendar-events-list">
            <Show when={!loading()} fallback={<p class="text-xs text-neutral-400">Loading events...</p>}>
              <Show when={visibleEvents().length > 0} fallback={<p class="text-xs text-neutral-500">No events for current filter.</p>}>
                <VirtualAnimatedList
                  items={visibleEvents}
                  estimateSize={54}
                  overscan={5}
                  containerRef={() => eventsListRef}
                  animateRows
                  renderItem={(event) => (
                    <article class="mt-1.5 rounded border border-neutral-800 bg-neutral-900/70 px-2 py-1.5" data-testid="calendar-event-item">
                      <p class="truncate text-xs font-medium text-neutral-100">{String(event?.summary || "Untitled event")}</p>
                      <p class="truncate text-[10px] text-neutral-400">{eventTimeLabel(event)} • {String(event?.organizer?.email || "calendar")}</p>
                      <Show when={event?.location}>
                        <p class="truncate text-[10px] text-neutral-500">{String(event.location)}</p>
                      </Show>
                    </article>
                  )}
                />
              </Show>
            </Show>
          </div>
        </section>
      </div>
    </div>
  );
}

export default CalendarPanel;
