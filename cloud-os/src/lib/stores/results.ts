/**
 * Result History Store
 * Manages morphable result panels for AI-centric CloudOS
 */

import { createSignal, createRoot } from 'solid-js';
import type { ToolResponse } from '../mcp/types';

export interface ResultItem {
  id: string;
  query: string;
  timestamp: Date;
  response: ToolResponse;
  viewState?: Record<string, any>;  // Scroll position, expanded nodes, etc.
  isPinned?: boolean;
}

export interface ContextGroup {
  id: string;
  name: string;
  resultIds: string[];
  createdAt: Date;
  isActive?: boolean;
}

export interface ResultStoreState {
  results: ResultItem[];
  contexts: ContextGroup[];
  activeContextId?: string;
}

const STORAGE_KEY = 'browser-os-results';

function loadFromStorage(): Partial<ResultStoreState> {
  if (typeof window === 'undefined') return {};
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      // Convert timestamp strings back to Date objects
      parsed.results?.forEach((r: any) => {
        r.timestamp = new Date(r.timestamp);
      });
      parsed.contexts?.forEach((c: any) => {
        c.createdAt = new Date(c.createdAt);
      });
      return parsed;
    }
  } catch (e) {
    console.warn('Failed to load results from storage:', e);
  }
  return {};
}

function saveToStorage(state: ResultStoreState) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch (e) {
    console.warn('Failed to save results to storage:', e);
  }
}

// Create singleton store (client-side only)
let _resultsState: any = null;

if (typeof window !== 'undefined') {
  _resultsState = createRoot(() => {
    const initial = loadFromStorage();

    return createSignal<ResultStoreState>({
      results: initial.results || [],
      contexts: initial.contexts || [{
        id: 'default',
        name: 'Default',
        resultIds: initial.results?.map((r: ResultItem) => r.id) || [],
        createdAt: new Date(),
        isActive: true,
      }],
      activeContextId: initial.activeContextId || 'default',
    });
  });
}

// Helper to get current state
const getState = (): ResultStoreState => {
  if (!_resultsState) {
    return { results: [], contexts: [{ id: 'default', name: 'Default', resultIds: [], createdAt: new Date(), isActive: true }] };
  }
  return _resultsState[0]();
};

// Add a new result
export function addResult(result: Omit<ResultItem, 'id' | 'timestamp'>) {
  if (!_resultsState) return null;
  
  const newItem: ResultItem = {
    ...result,
    id: `result-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    timestamp: new Date(),
  };

  _resultsState[1]((prev: ResultStoreState) => {
    const updated = {
      ...prev,
      results: [newItem, ...prev.results].slice(0, 100), // Keep last 100 results
    };

    // Add to active context
    const activeContext = prev.contexts.find(c => c.isActive);
    if (activeContext) {
      updated.contexts = prev.contexts.map(c =>
        c.isActive
          ? { ...c, resultIds: [newItem.id, ...c.resultIds].slice(0, 50) }
          : c
      );
    }

    saveToStorage(updated);
    return updated;
  });

  return newItem.id;
}

// Remove a result
export function removeResult(id: string) {
  if (!_resultsState) return;
  
  _resultsState[1]((prev: ResultStoreState) => {
    const updated = {
      ...prev,
      results: prev.results.filter(r => r.id !== id),
      contexts: prev.contexts.map(c => ({
        ...c,
        resultIds: c.resultIds.filter(rid => rid !== id),
      })),
    };
    saveToStorage(updated);
    return updated;
  });
}

// Pin/unpin a result
export function pinResult(id: string, pinned?: boolean) {
  if (!_resultsState) return;
  
  _resultsState[1]((prev: ResultStoreState) => {
    const updated = {
      ...prev,
      results: prev.results.map(r =>
        r.id === id ? { ...r, isPinned: pinned ?? !r.isPinned } : r
      ),
    };
    saveToStorage(updated);
    return updated;
  });
}

// Clear all results
export function clearResults() {
  if (!_resultsState) return;
  
  _resultsState[1]((prev: ResultStoreState) => {
    const updated = {
      ...prev,
      results: [],
      contexts: prev.contexts.map(c => ({ ...c, resultIds: [] })),
    };
    saveToStorage(updated);
    return updated;
  });
}

// Create a new context (workspace)
export function createContext(name: string) {
  if (!_resultsState) return null;
  
  const newContext: ContextGroup = {
    id: `context-${Date.now()}`,
    name,
    resultIds: [],
    createdAt: new Date(),
    isActive: true,
  };

  _resultsState[1]((prev: ResultStoreState) => {
    const updated = {
      ...prev,
      contexts: [...prev.contexts.map(c => ({ ...c, isActive: false })), newContext],
      activeContextId: newContext.id,
    };
    saveToStorage(updated);
    return updated;
  });

  return newContext.id;
}

// Switch to a context
export function switchContext(id: string) {
  if (!_resultsState) return;
  
  _resultsState[1]((prev: ResultStoreState) => {
    const updated = {
      ...prev,
      contexts: prev.contexts.map(c => ({
        ...c,
        isActive: c.id === id,
      })),
      activeContextId: id,
    };
    saveToStorage(updated);
    return updated;
  });
}

// Get current context
export function getCurrentContext() {
  const state = getState();
  return state.contexts.find(c => c.isActive) || state.contexts[0];
}

// Get all contexts
export function getContexts() {
  return getState().contexts;
}

// Get results for current context
export function getContextResults() {
  const state = getState();
  const context = state.contexts.find(c => c.isActive) || state.contexts[0];
  return state.results.filter(r => context.resultIds.includes(r.id));
}

// Get pinned results
export function getPinnedResults() {
  return getState().results.filter(r => r.isPinned);
}

// Get all results
export function getAllResults() {
  return getState().results;
}

// Update result view state (scroll position, etc.)
export function updateResultViewState(id: string, viewState: Record<string, any>) {
  if (!_resultsState) return;
  
  _resultsState[1]((prev: ResultStoreState) => {
    const updated = {
      ...prev,
      results: prev.results.map(r =>
        r.id === id ? { ...r, viewState: { ...r.viewState, ...viewState } } : r
      ),
    };
    saveToStorage(updated);
    return updated;
  });
}

// Export for components
export { _resultsState as resultsState };
