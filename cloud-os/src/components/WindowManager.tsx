import { For, lazy, Suspense, onMount, Show } from 'solid-js';
import { Motion } from 'solid-motionone';
import { windows, initializeDefaultWindows, type WindowId } from '../stores/windows';
import { getActiveWorkspaceId } from '../stores/workspaces';
import Window from './Window';
import { openGitHubFile, currentFile } from '../stores/github';
import { EditorSkeleton, FileManagerSkeleton, WindowSkeleton, LoadingDots } from './ui/Skeleton';
import { getIntegrationById } from '../lib/config/integrations.config';

// Component loaders
const Editor = lazy(() => import('./Editor'));
const Terminal = lazy(() => import('./Terminal').then(m => ({ default: m.default })));
const FileManager = lazy(() => import('./FileManager'));
const GitHubBrowser = lazy(() => import('./GitHubBrowser'));
const GmailPanel = lazy(() => import('./GmailPanel'));
const DrivePanel = FileManager;  // Drive uses FileManager with Drive source
const CallApp = lazy(() => import('./CallApp'));
const CalendarPanel = lazy(() => import('./CalendarPanel'));
const CloudflarePanel = lazy(() => import('./CloudflarePanel'));
const IntegrationsPanel = lazy(() => import('./IntegrationsPanel'));
const SettingsPanel = lazy(() => import('./SettingsPanel'));
const CloudPanel = lazy(() => import('./CloudPanel'));

// Skeleton fallback for Suspense
function LoadingFallback(props: { type?: string }) {
  return (
    <Motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.2 }}
      class="h-full w-full"
    >
      {props.type === 'editor' && <EditorSkeleton />}
      {props.type === 'files' && <FileManagerSkeleton />}
      {props.type === 'terminal' && (
        <div class="h-full flex items-center justify-center bg-[#0a0a0a]">
          <LoadingDots />
        </div>
      )}
      {!props.type && <WindowSkeleton showTabs />}
    </Motion.div>
  );
}

// Window content renderer
function getWindowContent(id: WindowId) {
  const integration = getIntegrationById(id);
  const githubFile = currentFile();
  
  switch (id) {
    case 'editor':
      return (
        <Suspense fallback={<LoadingFallback type="editor" />}>
          <Editor 
            value={githubFile?.content || ''} 
            path={githubFile?.path}
            onChange={(value) => {
              // If editing a GitHub file, update the store
              if (githubFile) {
                import('../stores/github').then(({ updateFileContent }) => {
                  updateFileContent(value);
                });
              }
            }}
          />
        </Suspense>
      );
    case 'files':
      return (
        <Suspense fallback={<LoadingFallback type="files" />}>
          <FileManager />
        </Suspense>
      );
    case 'integrations':
      return (
        <Suspense fallback={<LoadingFallback />}>
          <IntegrationsPanel />
        </Suspense>
      );
    case 'github':
      return (
        <Suspense fallback={<LoadingFallback />}>
          <GitHubBrowser />
        </Suspense>
      );
    case 'email':
      return (
        <Suspense fallback={<LoadingFallback />}>
          <GmailPanel />
        </Suspense>
      );
    case 'drive':
      return (
        <Suspense fallback={<LoadingFallback />}>
          <DrivePanel />
        </Suspense>
      );
    case 'calendar':
      return (
        <Suspense fallback={<LoadingFallback />}>
          <CalendarPanel />
        </Suspense>
      );
    case 'cloudflare':
      return (
        <Suspense fallback={<LoadingFallback />}>
          <CloudflarePanel />
        </Suspense>
      );
    case 'call':
      return (
        <Suspense fallback={<LoadingFallback />}>
          <CallApp />
        </Suspense>
      );
    case 'terminal':
      return (
        <Suspense fallback={<LoadingFallback type="terminal" />}>
          <Terminal />
        </Suspense>
      );
    case 'settings':
      return (
        <Suspense fallback={<LoadingFallback />}>
          <SettingsPanel />
        </Suspense>
      );
    case 'cloud':
      return (
        <Suspense fallback={<LoadingFallback />}>
          <CloudPanel />
        </Suspense>
      );
    case 'theme':
      return (
        <div class="p-4">
          <h2 class="text-lg font-bold mb-4">Theme Settings</h2>
          <div class="space-y-4">
            <div>
              <label for="theme-appearance" class="block text-sm mb-2">Appearance</label>
              <select id="theme-appearance" class="w-full p-2 rounded bg-neutral-800 border border-neutral-700">
                <option>Light</option>
                <option>Dark</option>
                <option>System</option>
              </select>
            </div>
            <div>
              <span class="block text-sm mb-2">Accent Color</span>
              <div class="flex gap-2">
                <button type="button" class="w-8 h-8 rounded-full bg-blue-500" />
                <button type="button" class="w-8 h-8 rounded-full bg-green-500" />
                <button type="button" class="w-8 h-8 rounded-full bg-purple-500" />
                <button type="button" class="w-8 h-8 rounded-full bg-orange-500" />
              </div>
            </div>
          </div>
        </div>
      );
    default:
      return (
        <div class="p-4">
          <h2 class="text-lg font-bold mb-4">{integration?.name || id}</h2>
          <p class="text-neutral-400">
            {integration?.description || `Window ${id}`}
          </p>
        </div>
      );
  }
}

export default function WindowManager() {
  onMount(() => {
    initializeDefaultWindows();
  });
  
  const openWindows = () => {
    const store = windows();
    const activeWorkspace = getActiveWorkspaceId();
    return Object.entries(store)
      .filter(([_, state]) => state.isOpen && (state.workspaceId === activeWorkspace || !state.workspaceId))
      .map(([id, state], index) => ({ id, state, index }));
  };

  return (
    <>
      <For each={openWindows()}>
        {({ id, state }) => (
          <Window id={id} title={state.title}>
            {getWindowContent(id)}
          </Window>
        )}
      </For>
    </>
  );
}
