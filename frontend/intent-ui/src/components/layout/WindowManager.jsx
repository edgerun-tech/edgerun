import { For, lazy, Suspense, onMount, onCleanup } from "solid-js";
import { Motion } from "solid-motionone";
import { windows, initializeDefaultWindows } from "../../stores/windows";
import { shiftWindowLayer } from "../../stores/windows";
import { getActiveWorkspaceId } from "../../stores/workspaces";
import { clearProfileRuntimeSession, profileRuntime } from "../../stores/profile-runtime";
import Window from "./Window";
import { currentFile, updateCurrentFileContent } from "../../stores/fs";
import { EditorSkeleton, FileManagerSkeleton, WindowSkeleton, LoadingDots } from "../ui/Skeleton";
import { getIntegrationById } from "../../lib/config/integrations.config";
import { requirementForWindow } from "../../lib/profile-capability-policy";
import { scopeRequirementSatisfied } from "../../lib/oidc-scopes";
const Editor = lazy(() => import("./Editor"));
const Terminal = lazy(() => import("../panels/Terminal").then((m) => ({ default: m.default })));
const FileManager = lazy(() => import("./FileManager"));
const GitHubBrowser = lazy(() => import("./GitHubBrowser"));
const GmailPanel = lazy(() => import("../panels/GmailPanel"));
const DrivePanel = FileManager;
const CallApp = lazy(() => import("../apps/CallApp"));
const CalendarPanel = lazy(() => import("./CalendarPanel"));
const CloudflarePanel = lazy(() => import("./CloudflarePanel"));
const IntegrationsPanel = lazy(() => import("./IntegrationsPanel"));
const SettingsPanel = lazy(() => import("../panels/SettingsPanel"));
const OnvifPanel = lazy(() => import("../panels/OnvifPanel"));
const CloudPanel = lazy(() => import("../panels/CloudPanel"));
const CredentialsPanel = lazy(() => import("../panels/CredentialsPanel"));
const LauncherGuidePanel = lazy(() => import("../panels/LauncherGuidePanel"));
const BrowserApp = lazy(() => import("../apps/BrowserApp"));
const GooglePhotosPanel = lazy(() => import("../panels/GooglePhotosPanel"));

function ProfileCapabilityLocked(props) {
  return <div class="h-full p-4" data-testid="profile-capability-locked">
      <div class="mx-auto flex h-full max-w-lg items-center">
        <div class="w-full rounded-xl border border-neutral-700 bg-neutral-900/80 p-4">
          <p class="text-[11px] uppercase tracking-wide text-neutral-400">Profile-Gated Capability</p>
          <h3 class="mt-1 text-sm font-semibold text-neutral-100">{props.windowTitle} is locked</h3>
          <p class="mt-2 text-xs text-neutral-400">
            {props.reason}
          </p>
          <p class="mt-2 text-[11px] text-neutral-500">
            Required scopes: {props.requiredScopesText}
          </p>
          <button
            type="button"
            class="mt-3 inline-flex items-center gap-1 rounded-md border border-[hsl(var(--primary)/0.45)] bg-[hsl(var(--primary)/0.16)] px-3 py-1.5 text-xs text-[hsl(var(--primary))] hover:bg-[hsl(var(--primary)/0.25)]"
            onClick={() => clearProfileRuntimeSession()}
            data-testid="profile-open-bootstrap-gate"
          >
            Load or create profile
          </button>
        </div>
      </div>
    </div>;
}
function LoadingFallback(props) {
  return <Motion.div
    initial={{ opacity: 0 }}
    animate={{ opacity: 1 }}
    transition={{ duration: 0.2 }}
    class="h-full w-full"
  >
      {props.type === "editor" && <EditorSkeleton />}
      {props.type === "files" && <FileManagerSkeleton />}
      {props.type === "terminal" && <div class="h-full flex items-center justify-center bg-[#0a0a0a]">
          <LoadingDots />
        </div>}
      {!props.type && <WindowSkeleton showTabs />}
    </Motion.div>;
}
function getWindowContent(id) {
  const requirement = requirementForWindow(id);
  if (requirement) {
    const runtime = profileRuntime();
    const scopeOk = scopeRequirementSatisfied(runtime.grantedScopes, requirement);
    const profileLoaded = runtime.mode === "profile" && runtime.profileLoaded;
    if (!profileLoaded || !scopeOk) {
      const required = [...(requirement.requiredAll || []), ...(requirement.requiredAny || [])];
      return <ProfileCapabilityLocked
          windowTitle={id}
          requiredScopesText={required.join(", ")}
          reason={!profileLoaded
            ? "This surface requires a loaded profile session."
            : "This surface is blocked because the active profile session is missing required OIDC scopes."}
        />;
    }
  }
  const integration = getIntegrationById(id);
  const githubFile = currentFile();
  switch (id) {
    case "editor":
      return <Suspense fallback={<LoadingFallback type="editor" />}>
          <Editor
        value={githubFile?.content || ""}
        path={githubFile?.path}
        onChange={(value) => {
          if (githubFile) updateCurrentFileContent(value);
        }}
        onSave={(value) => {
          if (githubFile) updateCurrentFileContent(value);
        }}
      />
        </Suspense>;
    case "files":
      return <Suspense fallback={<LoadingFallback type="files" />}>
          <FileManager />
        </Suspense>;
    case "integrations":
      return <Suspense fallback={<LoadingFallback />}>
          <IntegrationsPanel />
        </Suspense>;
    case "github":
      return <Suspense fallback={<LoadingFallback />}>
          <GitHubBrowser />
        </Suspense>;
    case "email":
      return <Suspense fallback={<LoadingFallback />}>
          <GmailPanel />
        </Suspense>;
    case "drive":
      return <Suspense fallback={<LoadingFallback />}>
          <DrivePanel />
        </Suspense>;
    case "calendar":
      return <Suspense fallback={<LoadingFallback />}>
          <CalendarPanel />
        </Suspense>;
    case "cloudflare":
      return <Suspense fallback={<LoadingFallback />}>
          <CloudflarePanel />
        </Suspense>;
    case "call":
      return <Suspense fallback={<LoadingFallback />}>
          <CallApp />
        </Suspense>;
    case "terminal":
      return <Suspense fallback={<LoadingFallback type="terminal" />}>
          <Terminal />
        </Suspense>;
    case "settings":
      return <Suspense fallback={<LoadingFallback />}>
          <SettingsPanel />
        </Suspense>;
    case "widgets":
      return <Suspense fallback={<LoadingFallback />}>
          <SettingsPanel />
        </Suspense>;
    case "onvif":
      return <Suspense fallback={<LoadingFallback />}>
          <OnvifPanel />
        </Suspense>;
    case "cloud":
      return <Suspense fallback={<LoadingFallback />}>
          <CloudPanel />
        </Suspense>;
    case "browser":
      return <Suspense fallback={<LoadingFallback />}>
          <BrowserApp windowId={id} />
        </Suspense>;
    case "photos":
      return <Suspense fallback={<LoadingFallback />}>
          <GooglePhotosPanel />
        </Suspense>;
    case "credentials":
      return <Suspense fallback={<LoadingFallback />}>
          <CredentialsPanel />
        </Suspense>;
    case "guide":
      return <Suspense fallback={<LoadingFallback />}>
          <LauncherGuidePanel />
        </Suspense>;
    default:
      return <div class="p-4">
          <h2 class="text-lg font-bold mb-4">{integration?.name || id}</h2>
          <p class="text-neutral-400">
            {integration?.description || `Window ${id}`}
          </p>
        </div>;
  }
}
function WindowManager() {
  let handleMouseDown;
  let handleMouseMove;
  let handleMouseUp;
  onMount(() => {
    initializeDefaultWindows();
    let isPanningLayer = false;
    let lastX = 0;
    let lastY = 0;
    handleMouseDown = (event) => {
      if (!event.ctrlKey || event.button !== 0) return;
      isPanningLayer = true;
      lastX = event.clientX;
      lastY = event.clientY;
      event.preventDefault();
    };
    handleMouseMove = (event) => {
      if (!isPanningLayer) return;
      const dx = event.clientX - lastX;
      const dy = event.clientY - lastY;
      if (dx !== 0 || dy !== 0) {
        shiftWindowLayer(dx, dy);
        lastX = event.clientX;
        lastY = event.clientY;
      }
    };
    handleMouseUp = () => {
      isPanningLayer = false;
    };
    window.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    window.addEventListener("blur", handleMouseUp);
  });
  onCleanup(() => {
    if (handleMouseDown) window.removeEventListener("mousedown", handleMouseDown);
    if (handleMouseMove) window.removeEventListener("mousemove", handleMouseMove);
    if (handleMouseUp) {
      window.removeEventListener("mouseup", handleMouseUp);
      window.removeEventListener("blur", handleMouseUp);
    }
  });
  const openWindows = () => {
    const store = windows();
    const activeWorkspace = getActiveWorkspaceId();
    return Object.entries(store).filter(([_, state]) => state.isOpen && (state.workspaceId === activeWorkspace || !state.workspaceId)).map(([id, state], index) => ({ id, state, index }));
  };
  return <>
      <For each={openWindows()}>
        {({ id, state }) => <Window id={id} title={state.title}>
            {getWindowContent(id)}
          </Window>}
      </For>
    </>;
}
export {
  WindowManager as default
};
