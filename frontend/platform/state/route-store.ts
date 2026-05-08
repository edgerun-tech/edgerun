/**
 * Single source of truth for route state and navigation.
 */

import { atom, computed } from "nanostores"

export interface RouteState {
  currentPath: string
  params: Record<string, string>
  query: Record<string, string>
  history: string[]
}

const initialState: RouteState = {
  currentPath: "/dashboard",
  params: {},
  query: {},
  history: ["/dashboard"],
}

export const routeStore = atom<RouteState>(initialState)

export const currentPath = computed(routeStore, (s) => s.currentPath)

export function navigate(path: string, params?: Record<string, string>): void {
  const state = routeStore.get()
  const newHistory = [...state.history, path]
  routeStore.set({
    currentPath: path,
    params: params || {},
    query: {},
    history: newHistory.slice(-50),
  })
}

export function updateParams(params: Record<string, string>): void {
  const state = routeStore.get()
  routeStore.set({
    ...state,
    params: { ...state.params, ...params },
  })
}

export function goBack(): void {
  const state = routeStore.get()
  if (state.history.length > 1) {
    const newHistory = state.history.slice(0, -1)
    const previousPath = newHistory[newHistory.length - 1]
    routeStore.set({
      currentPath: previousPath,
      params: {},
      query: {},
      history: newHistory,
    })
  }
}
