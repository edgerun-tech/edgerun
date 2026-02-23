import { createSignal, createRoot, lazy } from 'solid-js';
import type { Component } from 'solid-js';
import { getEnabledIntegrations } from '../lib/config/integrations.config';

export type WindowId = string;

export type WindowState = {
  isOpen: boolean;
  isMinimized: boolean;
  isMaximized: boolean;
  title: string;
  component?: Component | (() => Promise<{ default: Component }>);
  props?: Record<string, any>;
  position?: { x: number; y: number };
  size?: { width: number; height: number };
  workspaceId?: string;
};

export type WindowsStore = Record<WindowId, WindowState>;

const STORAGE_KEY = 'browser-os-windows';

// Load initial state from localStorage
function loadFromStorage(): WindowsStore {
  if (typeof window === 'undefined') return {};
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      // Reset open state but keep position/size
      Object.keys(parsed).forEach(key => {
        parsed[key].isOpen = false;
        parsed[key].isMinimized = false;
      });
      return parsed;
    }
  } catch (e) {
    console.error('Failed to load windows from storage:', e);
  }
  return {};
}

// Save to localStorage
function saveToStorage(store: WindowsStore) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
  } catch (e) {
    console.error('Failed to save windows to storage:', e);
  }
}

// Create a singleton store outside of components
const [windows, setWindows] = createSignal<WindowsStore>(loadFromStorage());
const [activeWindowId, setActiveWindowId] = createSignal<WindowId | null>(null);

export function getActiveWindowId() {
  return activeWindowId;
}

export function bringWindowToFront(id: WindowId) {
  setActiveWindowId(id);
}

// Window registry to store window configurations
const windowRegistry = new Map<WindowId, Omit<WindowState, 'isOpen' | 'isMinimized' | 'isMaximized'>>();

// Register a window type without opening it
export function registerWindow(
  id: WindowId,
  config: Omit<WindowState, 'isOpen' | 'isMinimized' | 'isMaximized'>
) {
  windowRegistry.set(id, config);
  // Initialize the window in the store if not exists
  const current = windows()[id];
  if (!current) {
    setWindows(prev => ({
      ...prev,
      [id]: {
        isOpen: false,
        isMinimized: false,
        isMaximized: false,
        title: config.title,
        component: config.component,
        props: config.props,
      }
    }));
  }
}

// Get window config from registry
export function getWindowConfig(id: WindowId) {
  return windowRegistry.get(id);
}

// Get all registered window IDs
export function getRegisteredWindows(): WindowId[] {
  return Array.from(windowRegistry.keys());
}

export function openWindow(id: WindowId) {
  const current = windows()[id];
  const config = windowRegistry.get(id);
  
  if (!current && config) {
    setWindows(prev => {
      const updated = {
        ...prev,
        [id]: {
          isOpen: true,
          isMinimized: false,
          isMaximized: false,
          title: config.title,
          component: config.component,
          props: config.props,
        }
      };
      saveToStorage(updated);
      return updated;
    });
  } else if (current) {
    setWindows(prev => {
      const updated = {
        ...prev,
        [id]: {
          ...current,
          isOpen: true,
          isMinimized: false,
        }
      };
      saveToStorage(updated);
      return updated;
    });
  }
  bringWindowToFront(id);
}

export function closeWindow(id: WindowId) {
  const current = windows()[id];
  if (current) {
    setWindows(prev => {
      const updated = {
        ...prev,
        [id]: {
          ...current,
          isOpen: false,
          isMinimized: false,
          isMaximized: false,
        }
      };
      saveToStorage(updated);
      return updated;
    });
  }
}

export function toggleWindow(id: WindowId) {
  const window = windows()[id];
  if (window?.isOpen) {
    closeWindow(id);
  } else {
    openWindow(id);
  }
}

export function minimizeWindow(id: WindowId) {
  const current = windows()[id];
  if (current) {
    setWindows(prev => {
      const updated = {
        ...prev,
        [id]: {
          ...current,
          isMinimized: true,
        }
      };
      saveToStorage(updated);
      return updated;
    });
  }
}

export function maximizeWindow(id: WindowId) {
  const current = windows()[id];
  if (current) {
    setWindows(prev => {
      const updated = {
        ...prev,
        [id]: {
          ...current,
          isMaximized: true,
          isMinimized: false,
        }
      };
      saveToStorage(updated);
      return updated;
    });
  }
}

export function restoreWindow(id: WindowId) {
  const current = windows()[id];
  if (current) {
    setWindows(prev => {
      const updated = {
        ...prev,
        [id]: {
          ...current,
          isMaximized: false,
          isMinimized: false,
        }
      };
      saveToStorage(updated);
      return updated;
    });
  }
}

export function updateWindowPosition(id: WindowId, position: { x: number; y: number }) {
  const current = windows()[id];
  if (current) {
    setWindows(prev => {
      const updated = {
        ...prev,
        [id]: {
          ...current,
          position,
        }
      };
      saveToStorage(updated);
      return updated;
    });
  }
}

export function updateWindowSize(id: WindowId, size: { width: number; height: number }) {
  const current = windows()[id];
  if (current) {
    setWindows(prev => {
      const updated = {
        ...prev,
        [id]: {
          ...current,
          size,
        }
      };
      saveToStorage(updated);
      return updated;
    });
  }
}

// Export the signal for components to use
export { windows };

// Bulk register multiple windows
export function registerWindows(
  configs: Record<WindowId, Omit<WindowState, 'isOpen' | 'isMinimized' | 'isMaximized'>>
) {
  Object.entries(configs).forEach(([id, config]) => {
    registerWindow(id, config);
  });
}

// Component loader registry
const componentRegistry: Record<string, () => Promise<{ default: Component }>> = {
  editor: () => import('../components/Editor'),
  terminal: () => import('../components/Terminal'),
  files: () => import('../components/FileManager'),
  email: () => import('../components/GmailPanel'),
  github: () => import('../components/GitHubBrowser'),
  cloudflare: () => import('../components/CloudflarePanel'),
  drive: () => import('../components/FileManager'),  // Use FileManager with Drive source
  calendar: () => import('../components/CalendarPanel'),
  call: () => import('../components/CallApp'),
  settings: () => import('../components/SettingsPanel'),
  cloud: () => import('../components/CloudPanel'),
  activity: () => import('../components/ActivityFeed'),
  integrations: () => import('../components/IntegrationsPanel'),
  // Placeholders for truly missing components - using lazy component factories
  prompt: () => import('../components/Placeholder').then(m => ({ default: () => m.Placeholder('Prompt Library') })),
  products: () => import('../components/Placeholder').then(m => ({ default: () => m.Placeholder('Products') })),
  components: () => import('../components/Placeholder').then(m => ({ default: () => m.Placeholder('Component Library') })),
  changelog: () => import('../components/Placeholder').then(m => ({ default: () => m.Placeholder('Changelog') })),
  theme: () => import('../components/Placeholder').then(m => ({ default: () => m.Placeholder('Theme Settings') })),
};

// Auto-register windows from config
export function initializeDefaultWindows() {
  const integrations = getEnabledIntegrations();
  
  for (const integration of integrations) {
    const componentLoader = componentRegistry[integration.id];

    if (componentLoader !== undefined) {
      registerWindow(integration.id, {
        title: integration.window.title,
      });
    }
  }
}

// Get component loader for a window
export function getComponentLoader(id: WindowId) {
  return componentRegistry[id];
}
