import { bootstrapBrowserCdpRelay } from "@/platform/dev/browser-cdp-relay";

export type RelayDestination = "frontend" | "backend" | "chatgpt";
export type RouteMode = "auto" | RelayDestination;
export type ResolvedDockRoute = {
  relay: ReturnType<typeof bootstrapBrowserCdpRelay> | null;
  relayCanHandleSelectedDestination: boolean;
  resolvedRouteLabel: string;
  routeLabel: RelayDestination;
  selectedRelayDestination: RelayDestination | null;
  forceBackendMode: boolean;
  relayPrefix: "!" | "~";
};

export function resolveDockCommandRoute(explicitRoute: RouteMode): ResolvedDockRoute {
  const relay = bootstrapBrowserCdpRelay();
  const relayDestination = explicitRoute === "auto" ? relay?.status().destination : explicitRoute;
  const selectedRelayDestination = normalizeRelayDestination(relayDestination);
  const forceBackendMode = explicitRoute === "backend";
  const routeLabel = forceBackendMode || explicitRoute === "auto"
    ? (selectedRelayDestination || "backend")
    : explicitRoute;
  const relayCanHandleSelectedDestination = Boolean(
    selectedRelayDestination && (
      selectedRelayDestination === "frontend"
      || selectedRelayDestination === "chatgpt"
      || (selectedRelayDestination === "backend" && relay?.status().backendBridgeRegistered)
    ),
  );

  return {
    relay,
    relayCanHandleSelectedDestination,
    resolvedRouteLabel: labelForRoute(routeLabel),
    routeLabel,
    selectedRelayDestination,
    forceBackendMode,
    relayPrefix: routeLabel === "chatgpt" ? "!" : "~",
  };
}

function normalizeRelayDestination(destination: unknown): RelayDestination | null {
  if (destination === "frontend" || destination === "backend" || destination === "chatgpt") {
    return destination;
  }
  return null;
}

function labelForRoute(routeLabel: RelayDestination): string {
  return routeLabel === "backend"
    ? "/api/codex"
    : routeLabel === "chatgpt"
      ? "CDP localhost (chatgpt)"
      : "frontend relay";
}
