import { For, Show, createMemo, createSignal, onMount } from "solid-js";
import FloatingFeedPanel from "./FloatingFeedPanel";
import { formatEventAge, formatEventPayload, severityBadgeClass } from "../../lib/telemetry-panels";

function TelemetryPanels(props) {
  const MINIMIZED_KEY = "intent-ui-floating-panels-minimized-v1";
  const [minimizedPanels, setMinimizedPanels] = createSignal({
    "event-bus": false,
    "docker-logs": false,
    "system-state": false
  });

  const panelRegistry = createMemo(() => [
    { id: "event-bus", label: "Event Bus" },
    { id: "docker-logs", label: "Docker Logs" },
    { id: "system-state", label: "System State" }
  ]);

  const persistMinimizedPanels = (next) => {
    if (typeof window === "undefined") return;
    try {
      localStorage.setItem(MINIMIZED_KEY, JSON.stringify(next));
    } catch {
      // ignore persistence failures
    }
  };

  const loadMinimizedPanels = () => {
    if (typeof window === "undefined") return;
    try {
      const raw = localStorage.getItem(MINIMIZED_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw);
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return;
      setMinimizedPanels((prev) => {
        const next = {
          ...prev,
          "event-bus": Boolean(parsed["event-bus"]),
          "docker-logs": Boolean(parsed["docker-logs"]),
          "system-state": Boolean(parsed["system-state"])
        };
        return next;
      });
    } catch {
      // ignore parse failures
    }
  };

  const setPanelMinimized = (panelId, minimized) => {
    const id = String(panelId || "").trim();
    if (!id) return;
    setMinimizedPanels((prev) => {
      const next = {
        ...prev,
        [id]: Boolean(minimized)
      };
      persistMinimizedPanels(next);
      return next;
    });
  };

  const minimizedDockItems = createMemo(() => panelRegistry().filter((panel) => minimizedPanels()[panel.id]));

  onMount(() => {
    loadMinimizedPanels();
  });

  return (
    <>
      <Show when={!minimizedPanels()["event-bus"]}>
        <FloatingFeedPanel
          panelId="event-bus"
          title={props.eventBusPanelTitle}
          maxItems={20}
          minWidth={280}
          minHeight={150}
          defaultLayout={props.layouts.eventBus}
          entries={props.latestEventBusItems}
          emptyLabel={props.eventBusEmptyLabel}
          onMinimize={() => setPanelMinimized("event-bus", true)}
          renderEntry={(event) => (
            <>
              <p class="truncate text-[10px] text-white">
                {event.topic || "event.unknown"} · {formatEventAge(event.createdAt)}
              </p>
              <Show when={formatEventPayload(event.payload)}>
                {(preview) => <p class="truncate font-mono text-[9px] text-white">{preview()}</p>}
              </Show>
            </>
          )}
        />
      </Show>
      <Show when={!minimizedPanels()["docker-logs"]}>
        <FloatingFeedPanel
          panelId="docker-logs"
          title="DOCKER LOGS"
          maxItems={20}
          minWidth={280}
          minHeight={150}
          defaultLayout={props.layouts.dockerLogs}
          entries={props.latestDockerLogItems}
          emptyLabel="No docker log events yet."
          onMinimize={() => setPanelMinimized("docker-logs", true)}
          renderEntry={(line) => (
            <p class="truncate text-[10px] text-white">
              [{line.containerName}] {line.message} · {formatEventAge(line.timestamp)}
            </p>
          )}
        />
      </Show>
      <Show when={!minimizedPanels()["system-state"]}>
        <FloatingFeedPanel
          panelId="system-state"
          title="SYSTEM STATE"
          maxItems={20}
          minWidth={280}
          minHeight={150}
          defaultLayout={props.layouts.systemState}
          entries={props.systemStateItems}
          emptyLabel="No system state yet."
          onMinimize={() => setPanelMinimized("system-state", true)}
          renderEntry={(entry) => (
            <>
              <button
                type="button"
                class={`flex w-full items-center gap-1.5 text-left text-[10px] text-white ${
                  entry.topicFilter || entry.clearFilter || entry.filterType ? "cursor-pointer hover:text-[hsl(var(--primary))]" : "cursor-default"
                }`}
                onClick={() => props.onSelectSystemStateItem(entry)}
              >
                <span class="truncate uppercase tracking-wide text-neutral-300">{entry.label}</span>
                <span class={`shrink-0 rounded border px-1 py-[1px] text-[9px] ${severityBadgeClass(entry.severity)}`}>
                  {entry.value}
                </span>
                <Show when={entry.updatedAt}>
                  <span class="truncate text-[9px] text-neutral-500">{formatEventAge(entry.updatedAt)}</span>
                </Show>
              </button>
              <Show when={entry.detail}>
                {(detail) => <p class="truncate text-[9px] text-neutral-300">{detail()}</p>}
              </Show>
            </>
          )}
        />
      </Show>
      <Show when={minimizedDockItems().length > 0}>
        <div
          class="fixed bottom-[88px] right-3 z-[10003] flex flex-wrap items-center justify-end gap-1.5"
          data-testid="telemetry-panel-dock"
        >
          <For each={minimizedDockItems()}>
            {(panel) => (
              <button
                type="button"
                class="rounded-md border border-neutral-700 bg-neutral-900/90 px-2 py-1 text-[10px] uppercase tracking-wide text-neutral-100 transition-colors hover:border-[hsl(var(--primary)/0.45)] hover:text-[hsl(var(--primary))]"
                onClick={() => setPanelMinimized(panel.id, false)}
                data-testid={`telemetry-panel-dock-open-${panel.id}`}
              >
                {panel.label}
              </button>
            )}
          </For>
        </div>
      </Show>
    </>
  );
}

export default TelemetryPanels;
