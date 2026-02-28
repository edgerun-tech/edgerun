import { createMemo, createSignal } from "solid-js";
import { buildSystemStateItems, createDefaultFloatingLayouts } from "../telemetry-panels";
import { eventBusRuntime, eventTimeline } from "../../stores/eventbus";
import { knownDevices } from "../../stores/devices";

export function useIntentTelemetry(params) {
  const [eventBusTopicFilter, setEventBusTopicFilter] = createSignal({ type: "all", value: "" });
  const runtimeSnapshot = createMemo(() => eventBusRuntime());
  const timelineSnapshot = createMemo(() => eventTimeline());
  const normalizedFilter = createMemo(() => {
    const filter = eventBusTopicFilter();
    return {
      type: String(filter?.type || "all"),
      value: String(filter?.value || "").trim()
    };
  });

  const latestEventBusItems = createMemo(() => {
    const filter = normalizedFilter();
    const filterType = filter.type;
    const filterValue = filter.value;
    const timeline = timelineSnapshot();
    const filtered = filterType === "exact" && filterValue
      ? timeline.filter((event) => String(event?.topic || "") === filterValue)
      : filterType === "prefix" && filterValue
        ? timeline.filter((event) => String(event?.topic || "").startsWith(filterValue))
        : timeline;
    return filtered.slice(-20).reverse();
  });

  const eventBusFilterLabel = createMemo(() => {
    const filter = normalizedFilter();
    if (filter.type === "all" || !filter.value) return "";
    return filter.value;
  });

  const eventBusPanelTitle = createMemo(() => (
    eventBusFilterLabel() ? "EVENT BUS · FILTERED" : "EVENT BUS"
  ));

  const eventBusEmptyLabel = createMemo(() => (
    eventBusFilterLabel()
      ? `No events for ${eventBusFilterLabel()}`
      : "No bridge events yet."
  ));

  const latestDockerLogItems = createMemo(() => {
    const timeline = timelineSnapshot();
    const dockerEvents = [];
    for (let index = timeline.length - 1; index >= 0; index -= 1) {
      const event = timeline[index];
      const topic = String(event?.topic || "").toLowerCase();
      const payload = event?.payload && typeof event.payload === "object" ? event.payload : {};
      const hasDockerTopic = topic.includes("docker.log")
        || topic.includes("docker.logs")
        || topic.startsWith("local.docker.");
      const message = String(
        payload?.message
        || payload?.log
        || payload?.line
        || payload?.text
        || ""
      ).trim();
      const hasDockerPayload = message.length > 0 && (
        payload?.container_name
        || payload?.containerName
        || payload?.container
        || payload?.container_id
        || payload?.containerId
      );
      if (!hasDockerTopic && !hasDockerPayload) {
        continue;
      }
      if (!message) {
        continue;
      }
      dockerEvents.push({
        id: String(event?.id || `docker-event-${index}`),
        containerName: String(
          payload?.container_name
          || payload?.containerName
          || payload?.container
          || payload?.container_id
          || payload?.containerId
          || "container"
        ),
        message,
        timestamp: String(event?.createdAt || ""),
        stream: String(payload?.stream || payload?.level || "")
      });
      if (dockerEvents.length >= 20) {
        break;
      }
    }
    return dockerEvents;
  });

  const systemStateItems = createMemo(() => {
    const runtime = runtimeSnapshot();
    return buildSystemStateItems({
      events: timelineSnapshot(),
      eventBusFilterLabel: eventBusFilterLabel(),
      localBridgeConnected: runtime.localBridgeConnected,
      localBridgeStatus: runtime.localBridgeStatus,
      localBridgeError: runtime.localBridgeError,
      knownDevices: knownDevices()
    });
  });

  const localBridgeError = createMemo(() => {
    const runtime = runtimeSnapshot();
    if (!params.isClient()) return "";
    if (runtime.localBridgeConnected) return "";
    if (runtime.localBridgeStatus !== "error") return "";
    return String(runtime.localBridgeError || "Can't connect to local bridge, is it running?");
  });

  const defaultFloatingLayouts = createMemo(() => createDefaultFloatingLayouts());

  const handleSystemStateItemSelect = (entry) => {
    if (entry.topicFilter || entry.filterType) {
      setEventBusTopicFilter({
        type: String(entry.filterType || "exact"),
        value: String(entry.filterValue || entry.topicFilter || "")
      });
      return;
    }
    if (entry.clearFilter) {
      setEventBusTopicFilter({ type: "all", value: "" });
    }
  };

  return {
    latestEventBusItems,
    eventBusPanelTitle,
    eventBusEmptyLabel,
    latestDockerLogItems,
    systemStateItems,
    localBridgeError,
    defaultFloatingLayouts,
    handleSystemStateItemSelect
  };
}
