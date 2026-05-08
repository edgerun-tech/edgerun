import { atom, computed } from "nanostores"

export type TrustPage =
  | "landing"
  | "dashboard"
  | "capsules"
  | "root"
  | "map"
  | "caps"
  | "policy"
  | "routes"
  | "delegations"
  | "inspector"
  | "compat"
  | "storage"

export type InspectorResult = "safe" | "risky" | null

export interface TrustManagerState {
  page: TrustPage
  sidebarOpen: boolean
  selectedCapsule: number | null
  selectedRoute: number
  selectedCapability: number
  selectedTrustMapNode: string
  rootStep: number
  simulating: boolean
  inspectorResult: InspectorResult
  importing: boolean
}

const initialState: TrustManagerState = {
  page: "landing",
  sidebarOpen: true,
  selectedCapsule: null,
  selectedRoute: 0,
  selectedCapability: 0,
  selectedTrustMapNode: "server",
  rootStep: 0,
  simulating: false,
  inspectorResult: null,
  importing: false,
}

export const trustManagerStore = atom<TrustManagerState>(initialState)

export const currentPageStore = computed(trustManagerStore, (s) => s.page)

export function setPage(page: TrustPage) {
  trustManagerStore.set({ ...trustManagerStore.get(), page })
}

export function toggleSidebar() {
  const state = trustManagerStore.get()
  trustManagerStore.set({ ...state, sidebarOpen: !state.sidebarOpen })
}

export function setSidebarOpen(open: boolean) {
  trustManagerStore.set({ ...trustManagerStore.get(), sidebarOpen: open })
}

export function setSelectedCapsule(index: number | null) {
  trustManagerStore.set({ ...trustManagerStore.get(), selectedCapsule: index })
}

export function setSelectedRoute(index: number) {
  trustManagerStore.set({ ...trustManagerStore.get(), selectedRoute: index })
}

export function toggleSelectedRoute(index: number) {
  const state = trustManagerStore.get()
  trustManagerStore.set({ ...state, selectedRoute: state.selectedRoute === index ? -1 : index })
}

export function setSelectedCapability(index: number) {
  trustManagerStore.set({ ...trustManagerStore.get(), selectedCapability: index })
}

export function toggleSelectedCapability(index: number) {
  const state = trustManagerStore.get()
  trustManagerStore.set({ ...state, selectedCapability: state.selectedCapability === index ? -1 : index })
}

export function setSelectedTrustMapNode(nodeId: string) {
  trustManagerStore.set({ ...trustManagerStore.get(), selectedTrustMapNode: nodeId })
}

export function setRootStep(step: number) {
  trustManagerStore.set({ ...trustManagerStore.get(), rootStep: step })
}

export function toggleSimulating() {
  const state = trustManagerStore.get()
  trustManagerStore.set({ ...state, simulating: !state.simulating })
}

export function setInspectorResult(result: InspectorResult) {
  trustManagerStore.set({ ...trustManagerStore.get(), inspectorResult: result })
}

export function setImporting(importing: boolean) {
  trustManagerStore.set({ ...trustManagerStore.get(), importing })
}

export function enterDashboard() {
  trustManagerStore.set({ ...trustManagerStore.get(), page: "dashboard" })
}
