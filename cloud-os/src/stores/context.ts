import { createStore } from 'solid-js/store';

export type EnvironmentType = 'dev' | 'staging' | 'prod' | 'unknown';

export type AppContextState = {
  currentRepo?: string;
  currentBranch?: string;
  currentHost?: string;
  currentProject?: string;
  recentFiles: string[];
  recentCommands: string[];
  activeIntegrations: string[];
  environment: EnvironmentType;
  openWindows: string[];
};

const [state, setState] = createStore<AppContextState>({
  currentRepo: '/home/ken/edgerun',
  currentBranch: 'main',
  currentHost: 'local',
  currentProject: 'cloud-os',
  recentFiles: [],
  recentCommands: [],
  activeIntegrations: [],
  environment: 'dev',
  openWindows: [],
});

export const context = state;

export function updateContext(next: Partial<AppContextState>): void {
  setState(next as any);
}

function pushUnique<T>(items: T[], value: T, limit: number): T[] {
  return [value, ...items.filter((item) => item !== value)].slice(0, limit);
}

export function addRecentCommand(command: string): void {
  const value = command.trim();
  if (!value) return;
  setState('recentCommands', (prev) => pushUnique(prev, value, 40));
}

export function addRecentFile(filePath: string): void {
  const value = filePath.trim();
  if (!value) return;
  setState('recentFiles', (prev) => pushUnique(prev, value, 40));
}

export function addOpenWindow(windowId: string): void {
  const value = windowId.trim();
  if (!value) return;
  setState('openWindows', (prev) => pushUnique(prev, value, 20));
}

export function removeOpenWindow(windowId: string): void {
  setState('openWindows', (prev) => prev.filter((id) => id !== windowId));
}
