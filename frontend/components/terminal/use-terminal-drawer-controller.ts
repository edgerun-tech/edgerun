// SPDX-License-Identifier: Apache-2.0
import { createEffect, createMemo, createSignal, onCleanup, onMount, untrack, type Accessor, type Setter } from 'solid-js'
import {
  getTerminalDrawerState,
  terminalDrawerActions,
  type PaneTransport,
  type TerminalDevice,
  type TerminalDrawerState,
  type TerminalSplitMode,
  type TerminalTab
} from '../../lib/terminal-drawer-store'
import { refreshTerminalDevices } from '../../lib/terminal-device-service'
import { mountTerminalDrawerRuntime } from '../../lib/terminal-drawer-runtime'
import { getWebRtcPeerSupervisor } from '../../lib/webrtc-peer-supervisor'
import { getRouteControlBase, parseRouteDeviceId, resolveDeviceRoute, resolveOwnerRoutes } from '../../lib/webrtc-route-client'
import { readWalletSession, type WalletSessionState } from '../../lib/wallet-session'

export type TerminalTabsController = {
  state: Accessor<TerminalDrawerState>
  activeTransport: Accessor<PaneTransport>
  tabMenuTabId: Accessor<string | null>
  setTabMenuTabId: Setter<string | null>
  hasTabsLeft: (tabId: string) => boolean
  hasTabsRight: (tabId: string) => boolean
  hasOtherTabs: () => boolean
  splitChange: (mode: TerminalSplitMode) => void
}

export type TerminalDevicesController = {
  state: Accessor<TerminalDrawerState>
  refreshDeviceStatus: () => Promise<void>
  walletAddress: Accessor<string>
  deviceNameInput: Accessor<string>
  setDeviceNameInput: Setter<string>
  deviceUrlInput: Accessor<string>
  setDeviceUrlInput: Setter<string>
  addDevice: () => void
  connectDevice: (device: Pick<TerminalDevice, 'id' | 'baseUrl'>) => Promise<void>
  ownerPubkeyInput: Accessor<string>
  setOwnerPubkeyInput: Setter<string>
  ownerImporting: Accessor<boolean>
  ownerImportNote: Accessor<string>
  syncMyDevices: () => Promise<void>
  importOwnerDevices: () => Promise<void>
}

export type TerminalPanesController = {
  activeTab: Accessor<TerminalTab | undefined>
}

export type TerminalDrawerController = {
  state: Accessor<TerminalDrawerState>
  walletConnected: Accessor<boolean>
  startResize: (ev: PointerEvent) => void
  tabs: TerminalTabsController
  devices: TerminalDevicesController
  panes: TerminalPanesController
}

export function useTerminalDrawerController(): TerminalDrawerController {
  const [state, setState] = createSignal<TerminalDrawerState>(getTerminalDrawerState())
  const [dragging, setDragging] = createSignal(false)
  const [wallet, setWallet] = createSignal<WalletSessionState>(readWalletSession())
  const [deviceNameInput, setDeviceNameInput] = createSignal('')
  const [deviceUrlInput, setDeviceUrlInput] = createSignal('')
  const [ownerPubkeyInput, setOwnerPubkeyInput] = createSignal('')
  const [ownerImporting, setOwnerImporting] = createSignal(false)
  const [ownerImportNote, setOwnerImportNote] = createSignal('')
  const [tabMenuTabId, setTabMenuTabId] = createSignal<string | null>(null)
  let refreshInFlight = false
  let autoWalletRouteInFlight = false
  let autoWalletRouteResolvedFor = ''

  const walletConnected = createMemo(() => wallet().connected)
  const walletAddress = createMemo(() => wallet().address.trim())
  const activeTab = createMemo(() => {
    const current = state()
    return current.tabs.find((tab) => tab.id === current.activeTabId) ?? current.tabs[0]
  })
  const activeTransport = createMemo<PaneTransport>(() => {
    const tab = activeTab()
    if (!tab || tab.panes.length === 0) return 'unknown'
    const map = state().paneTransport
    const values = tab.panes.map((pane) => map[pane.id] ?? 'unknown')
    if (values.includes('raw')) return 'raw'
    if (values.includes('mux')) return 'mux'
    return 'unknown'
  })
  const drawerHeight = createMemo(() => {
    if (!walletConnected()) return 0
    const current = state()
    if (!current.open) return 48
    return Math.round(window.innerHeight * current.heightRatio)
  })

  const splitChange = (mode: TerminalSplitMode) => terminalDrawerActions.setSplit(mode)
  const tabIndex = (tabId: string): number => state().tabs.findIndex((entry) => entry.id === tabId)
  const hasTabsLeft = (tabId: string): boolean => tabIndex(tabId) > 0
  const hasTabsRight = (tabId: string): boolean => {
    const idx = tabIndex(tabId)
    return idx >= 0 && idx < state().tabs.length - 1
  }
  const hasOtherTabs = (): boolean => state().tabs.length > 1

  const refreshDeviceStatus = async () => {
    if (refreshInFlight) return
    const connected = untrack(() => walletConnected())
    if (!connected) return
    const devices = untrack(() => state().devices)
    if (devices.length === 0) return
    refreshInFlight = true
    await refreshTerminalDevices(
      devices,
      (id, status) => terminalDrawerActions.markDeviceStatus(id, status)
    ).finally(() => {
      refreshInFlight = false
    })
  }

  const restoreLastDevice = () => terminalDrawerActions.restoreLastDeviceOnActiveTab()

  const addDevice = () => {
    terminalDrawerActions.addDevice(deviceNameInput(), deviceUrlInput())
    setDeviceNameInput('')
    setDeviceUrlInput('')
    void refreshDeviceStatus()
  }

  const connectDevice = async (device: Pick<TerminalDevice, 'id' | 'baseUrl'>) => {
    const routeDeviceId = parseRouteDeviceId(device.baseUrl)
    if (!routeDeviceId) {
      terminalDrawerActions.connectActiveTabToDevice(device.id)
      terminalDrawerActions.markDeviceStatus(device.id, 'offline')
      return
    }

    const supervisor = getWebRtcPeerSupervisor()
    terminalDrawerActions.connectActiveTabToDevice(device.id)
    await supervisor.connectToDevice(routeDeviceId).catch(() => {
      // keep probing through existing route table
    })
    const routedOnline = await supervisor.waitForRoutedPong(routeDeviceId, 1400).catch(() => false)
    terminalDrawerActions.markDeviceStatus(device.id, routedOnline ? 'online' : 'offline')
  }

  const importOwnerDevicesForOwner = async (owner: string): Promise<number> => {
    try {
      const controlBase = getRouteControlBase()
      const routes = await resolveOwnerRoutes(controlBase, owner)
      const existing = new Set(untrack(() => state().devices.map((device) => device.baseUrl)))
      let imported = 0
      for (const route of routes) {
        const deviceId = (route.device_id || '').trim()
        if (!deviceId) continue
        const routeUrl = `route://${deviceId}`
        if (existing.has(routeUrl)) continue
        const ownerLabel = owner.length > 12 ? `${owner.slice(0, 6)}...${owner.slice(-4)}` : owner
        const label = `Owner ${ownerLabel} · ${deviceId}`
        terminalDrawerActions.addDevice(label, routeUrl)
        existing.add(routeUrl)
        imported += 1
      }
      return imported
    } catch {
      return 0
    }
  }

  const importOwnerDevices = async () => {
    const owner = ownerPubkeyInput().trim() || walletAddress()
    if (!owner || ownerImporting()) return
    setOwnerImporting(true)
    setOwnerImportNote('')
    try {
      const imported = await importOwnerDevicesForOwner(owner)
      if (imported > 0) {
        setOwnerImportNote(`Imported ${imported} routed device${imported === 1 ? '' : 's'} for owner.`)
      } else {
        setOwnerImportNote('No new routed devices found for owner.')
      }
      await refreshDeviceStatus()
    } catch {
      setOwnerImportNote('Owner route lookup failed.')
    } finally {
      setOwnerImporting(false)
    }
  }

  const syncMyDevices = async () => {
    const owner = walletAddress()
    if (!owner) {
      setOwnerImportNote('Connect wallet first to sync devices.')
      return
    }
    if (ownerImporting()) return
    setOwnerImporting(true)
    setOwnerImportNote('')
    setOwnerPubkeyInput(owner)
    try {
      const imported = await importOwnerDevicesForOwner(owner)
      if (imported > 0) {
        setOwnerImportNote(`Imported ${imported} routed device${imported === 1 ? '' : 's'} for owner.`)
        await refreshDeviceStatus()
        return
      }

      // Fallback: if owner itself is a device id, use route.resolve to add it directly.
      const controlBase = getRouteControlBase()
      const resolved = await resolveDeviceRoute(controlBase, owner)
      if (resolved) {
        const routeUrl = `route://${owner}`
        const existing = new Set(untrack(() => state().devices.map((device) => device.baseUrl)))
        if (!existing.has(routeUrl)) {
          const ownerLabel = owner.length > 12 ? `${owner.slice(0, 6)}...${owner.slice(-4)}` : owner
          terminalDrawerActions.addDevice(`My Device · ${ownerLabel}`, routeUrl)
        }
        setOwnerImportNote('Synced routed device from wallet.')
        await refreshDeviceStatus()
        return
      }

      setOwnerImportNote('No routed devices found for connected wallet.')
    } catch {
      setOwnerImportNote('Wallet device sync failed.')
    } finally {
      setOwnerImporting(false)
    }
  }

  const startResize = (ev: PointerEvent) => {
    ev.preventDefault()
    setDragging(true)
    terminalDrawerActions.setOpen(true)
  }

  createEffect(() => {
    if (typeof document === 'undefined') return
    if (!walletConnected()) {
      document.documentElement.style.removeProperty('--terminal-drawer-height')
      return
    }
    document.documentElement.style.setProperty('--terminal-drawer-height', `${drawerHeight()}px`)
  })

  createEffect(() => {
    const walletOwner = walletAddress()
    if (!walletOwner) return
    if (ownerPubkeyInput().trim()) return
    setOwnerPubkeyInput(walletOwner)
  })

  createEffect(() => {
    const owner = walletAddress()
    if (!owner) return
    if (autoWalletRouteInFlight) return
    if (autoWalletRouteResolvedFor === owner) return
    autoWalletRouteInFlight = true
    void (async () => {
      try {
        const controlBase = getRouteControlBase()
        const resolved = await resolveDeviceRoute(controlBase, owner)
        if (!resolved) return
        const target = `route://${owner}`
        const exists = untrack(() => state().devices.some((device) => device.baseUrl === target))
        if (!exists) {
          const ownerLabel = owner.length > 12 ? `${owner.slice(0, 6)}...${owner.slice(-4)}` : owner
          terminalDrawerActions.addDevice(`My Device · ${ownerLabel}`, target)
          await refreshDeviceStatus()
        }
      } finally {
        autoWalletRouteResolvedFor = owner
        autoWalletRouteInFlight = false
      }
    })()
  })

  onMount(() => {
    const cleanupRuntime = mountTerminalDrawerRuntime({
      setState,
      setWallet,
      walletConnected,
      dragging,
      setDragging,
      tabMenuTabId,
      closeTabMenu: () => setTabMenuTabId(null),
      refreshDeviceStatus,
      restoreLastDevice
    })

    onCleanup(() => {
      cleanupRuntime()
      document.documentElement.style.removeProperty('--terminal-drawer-height')
    })
  })

  return {
    state,
    walletConnected,
    startResize,
    tabs: {
      state,
      activeTransport,
      tabMenuTabId,
      setTabMenuTabId,
      hasTabsLeft,
      hasTabsRight,
      hasOtherTabs,
      splitChange
    },
    devices: {
      state,
      refreshDeviceStatus,
      walletAddress,
      deviceNameInput,
      setDeviceNameInput,
      deviceUrlInput,
      setDeviceUrlInput,
      addDevice,
      connectDevice,
      ownerPubkeyInput,
      setOwnerPubkeyInput,
      ownerImporting,
      ownerImportNote,
      syncMyDevices,
      importOwnerDevices
    },
    panes: {
      activeTab
    }
  }
}
