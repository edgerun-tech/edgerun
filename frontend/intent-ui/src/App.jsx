import { Show, createSignal } from "solid-js";
import { IntentBar } from "./components/panels";
import { WindowManager, WorkflowOverlay } from "./components/layout";
import WallpaperWidgets from "./components/layout/WallpaperWidgets";
import WallpaperMap from "./components/layout/WallpaperMap";
import AccountCircleMenu from "./components/layout/AccountCircleMenu";
import IntentContextMenu from "./components/layout/IntentContextMenu";
import LayerIndicator from "./components/layout/LayerIndicator";
import LocalBridgeRequiredOverlay from "./components/layout/LocalBridgeRequiredOverlay";
import TelemetryPanels from "./components/layout/TelemetryPanels";
import ProfileBootstrapGate from "./components/onboarding/ProfileBootstrapGate";
import { useAccountSession } from "./lib/hooks/useAccountSession";
import { useIntentContextMenuActions } from "./lib/hooks/useIntentContextMenuActions";
import { useIntentTelemetry } from "./lib/hooks/useIntentTelemetry";
import { useIntentUiLifecycle } from "./lib/hooks/useIntentUiLifecycle";
import { localBridgeWsUrl } from "./lib/local-bridge-origin";
import { profileRuntime } from "./stores/profile-runtime";
import { retryLocalBridgeConnection } from "./stores/eventbus";

function App() {
  const localBridgeWsEndpoint = localBridgeWsUrl("/v1/local/eventbus/ws");
  const [inputLayer, setInputLayer] = createSignal(1);
  const [isClient, setIsClient] = createSignal(false);
  const [showLayerIndicator, setShowLayerIndicator] = createSignal(false);
  const {
    menuOpen,
    setMenuOpen,
    menuPos,
    contextMenuActions,
    handleRootContextMenu
  } = useIntentContextMenuActions();
  const {
    showBootstrapGate,
    setShowBootstrapGate,
    accountMenuOpen,
    setAccountMenuOpen,
    registeredDomain,
    setRegisteredDomain,
    sessionModeLabel,
    shortProfileId,
    resetSession,
    completeBootstrap
  } = useAccountSession();
  const {
    latestEventBusItems,
    eventBusPanelTitle,
    eventBusEmptyLabel,
    latestDockerLogItems,
    systemStateItems,
    localBridgeError,
    defaultFloatingLayouts,
    handleSystemStateItemSelect
  } = useIntentTelemetry({ isClient });

  useIntentUiLifecycle({
    setIsClient,
    setInputLayer,
    setShowLayerIndicator,
    setMenuOpen,
    setAccountMenuOpen,
    setShowBootstrapGate,
    setRegisteredDomain
  });

  const AppShell = () => (
    <div
      class="relative min-h-screen overflow-hidden bg-[#090909] text-foreground"
      data-input-layer={inputLayer()}
      onContextMenu={handleRootContextMenu}
    >
      <div class="pointer-events-none absolute inset-0 opacity-70" style={{
        background:
          "radial-gradient(1200px 700px at 20% -10%, rgba(38,78,125,0.24), transparent), radial-gradient(900px 560px at 88% 115%, rgba(64,42,101,0.2), transparent)"
      }} />
      <Show when={isClient()}>
        <WallpaperMap />
      </Show>
      <Show when={isClient()}>
        <WallpaperWidgets />
      </Show>
      <LayerIndicator visible={showLayerIndicator()} layer={inputLayer()} />
      <Show when={profileRuntime().ready}>
        <AccountCircleMenu
          open={accountMenuOpen()}
          sessionModeLabel={sessionModeLabel()}
          shortProfileId={shortProfileId()}
          backend={profileRuntime().backend}
          registeredDomain={registeredDomain()}
          onToggle={() => setAccountMenuOpen((prev) => !prev)}
          onResetSession={resetSession}
        />
      </Show>
      <Show
        when={!localBridgeError()}
        fallback={
          <LocalBridgeRequiredOverlay
            error={localBridgeError()}
            wsEndpoint={localBridgeWsEndpoint}
            onRetry={() => retryLocalBridgeConnection()}
          />
        }
      >
        <WindowManager />
        <TelemetryPanels
          layouts={defaultFloatingLayouts()}
          eventBusPanelTitle={eventBusPanelTitle()}
          latestEventBusItems={latestEventBusItems}
          eventBusEmptyLabel={eventBusEmptyLabel()}
          latestDockerLogItems={latestDockerLogItems}
          systemStateItems={systemStateItems}
          onSelectSystemStateItem={handleSystemStateItemSelect}
        />
        <IntentBar />
        <WorkflowOverlay />
      </Show>
    </div>
  );

  return (
    <>
      <AppShell />
      <Show when={isClient() && (!profileRuntime().ready || showBootstrapGate())}>
        <ProfileBootstrapGate
          allowDismiss={profileRuntime().ready}
          onDismiss={() => setShowBootstrapGate(false)}
          onComplete={completeBootstrap}
        />
      </Show>
      <IntentContextMenu
        open={isClient() && menuOpen()}
        position={menuPos()}
        actions={contextMenuActions()}
        onClose={() => setMenuOpen(false)}
      />
    </>
  );
}

export default App;
