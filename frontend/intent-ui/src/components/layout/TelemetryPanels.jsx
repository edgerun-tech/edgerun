import { Show } from "solid-js";
import FloatingFeedPanel from "./FloatingFeedPanel";
import { formatEventAge, formatEventPayload, severityBadgeClass } from "../../lib/telemetry-panels";

function TelemetryPanels(props) {
  return (
    <>
      <FloatingFeedPanel
        panelId="event-bus"
        title={props.eventBusPanelTitle}
        maxItems={20}
        minWidth={280}
        minHeight={150}
        defaultLayout={props.layouts.eventBus}
        entries={props.latestEventBusItems}
        emptyLabel={props.eventBusEmptyLabel}
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
      <FloatingFeedPanel
        panelId="docker-logs"
        title="DOCKER LOGS"
        maxItems={20}
        minWidth={280}
        minHeight={150}
        defaultLayout={props.layouts.dockerLogs}
        entries={props.latestDockerLogItems}
        emptyLabel="No docker log events yet."
        renderEntry={(line) => (
          <p class="truncate text-[10px] text-white">
            [{line.containerName}] {line.message} · {formatEventAge(line.timestamp)}
          </p>
        )}
      />
      <FloatingFeedPanel
        panelId="system-state"
        title="SYSTEM STATE"
        maxItems={20}
        minWidth={280}
        minHeight={150}
        defaultLayout={props.layouts.systemState}
        entries={props.systemStateItems}
        emptyLabel="No system state yet."
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
    </>
  );
}

export default TelemetryPanels;
