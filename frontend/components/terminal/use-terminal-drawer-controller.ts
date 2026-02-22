import { createEffect, createMemo, createSignal, onCleanup, onMount, untrack, type Accessor, type Setter } from 'solid-js'
import QRCode from 'qrcode'
import {
  getTerminalDrawerState,
  terminalDrawerActions,
  type TerminalDevice,
  type TerminalDrawerState,
  type TerminalSplitMode,
  type TerminalTab
} from '../../lib/terminal-drawer-store'
import { canUseCurrentOriginAsDevice, importTailscaleBridgeDevices, refreshTerminalDevices } from '../../lib/terminal-device-service'
import { mountTerminalDrawerRuntime } from '../../lib/terminal-drawer-runtime'
import { resolveTerminalBaseUrl } from '../../lib/webrtc-route-client'
import { readWalletSession, type WalletSessionState } from '../../lib/wallet-session'

export type TerminalTabsController = {
  state: Accessor<TerminalDrawerState>
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
  importTailscaleDevices: (opts?: { silent?: boolean }) => Promise<void>
  tailscaleImporting: Accessor<boolean>
  tailscaleImportNote: Accessor<string>
  deviceNameInput: Accessor<string>
  setDeviceNameInput: Setter<string>
  deviceUrlInput: Accessor<string>
  setDeviceUrlInput: Setter<string>
  addDevice: () => void
  connectDevice: (device: Pick<TerminalDevice, 'id' | 'baseUrl'>) => Promise<void>
  qrDeviceUrl: Accessor<string>
  setQrDeviceUrl: Setter<string>
  qrImageDataUrl: Accessor<string>
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
  const [qrDeviceUrl, setQrDeviceUrl] = createSignal('')
  const [qrImageDataUrl, setQrImageDataUrl] = createSignal('')
  const [tailscaleImporting, setTailscaleImporting] = createSignal(false)
  const [tailscaleImportNote, setTailscaleImportNote] = createSignal('')
  const [tabMenuTabId, setTabMenuTabId] = createSignal<string | null>(null)
  let lastAutoImportAt = 0

  const walletConnected = createMemo(() => wallet().connected)
  const activeTab = createMemo(() => {
    const current = state()
    return current.tabs.find((tab) => tab.id === current.activeTabId) ?? current.tabs[0]
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
    const connected = untrack(() => walletConnected())
    if (!connected) return
    const devices = untrack(() => state().devices)
    await refreshTerminalDevices(
      devices,
      (id, status) => terminalDrawerActions.markDeviceStatus(id, status)
    )
  }

  const maybeRegisterCurrentOriginDevice = async () => {
    if (typeof window === 'undefined') return
    const origin = window.location.origin
    const existing = untrack(() => state().devices.some((device) => device.baseUrl === origin))
    if (existing) return

    try {
      const available = await canUseCurrentOriginAsDevice(origin)
      if (!available) return
      terminalDrawerActions.addDevice('This Laptop', origin)
      await refreshDeviceStatus()
    } catch {
      // ignore: origin is not running term-server
    }
  }

  const importTailscaleDevices = async (opts?: { silent?: boolean }) => {
    const silent = Boolean(opts?.silent)
    if (tailscaleImporting()) return
    setTailscaleImporting(true)
    if (!silent) setTailscaleImportNote('')
    const endpoints = [
      'http://127.0.0.1:49201/v1/tailscale/devices',
      'http://localhost:49201/v1/tailscale/devices'
    ] as const

    const existing = new Set(untrack(() => state().devices.map((device) => device.baseUrl)))
    const result = await importTailscaleBridgeDevices(endpoints, existing)
    if (result.error) {
      if (!silent) setTailscaleImportNote(result.error)
      setTailscaleImporting(false)
      return
    }

    for (const device of result.added) {
      terminalDrawerActions.addDevice(device.name, device.baseUrl)
    }
    await refreshDeviceStatus()
    const imported = result.added.length
    if (!silent) {
      setTailscaleImportNote(imported > 0 ? `Imported ${imported} device${imported === 1 ? '' : 's'} from Tailscale.` : 'No new Tailscale devices to import.')
    }
    setTailscaleImporting(false)
  }

  const autoImportTailscaleDevices = () => {
    if (!untrack(() => state().autoImportTailscale)) return
    const now = Date.now()
    if (now - lastAutoImportAt < 15000) return
    lastAutoImportAt = now
    void importTailscaleDevices({ silent: true })
  }

  const restoreLastDevice = () => terminalDrawerActions.restoreLastDeviceOnActiveTab()

  const addDevice = () => {
    terminalDrawerActions.addDevice(deviceNameInput(), deviceUrlInput())
    setDeviceNameInput('')
    setDeviceUrlInput('')
    void refreshDeviceStatus()
  }

  const connectDevice = async (device: Pick<TerminalDevice, 'id' | 'baseUrl'>) => {
    const resolved = await resolveTerminalBaseUrl(device.baseUrl)
    if (resolved) {
      terminalDrawerActions.connectActiveTabToBaseUrl(resolved)
      if (resolved !== device.baseUrl.trim()) {
        terminalDrawerActions.markDeviceStatus(device.id, 'online')
      }
      return
    }
    terminalDrawerActions.connectActiveTabToDevice(device.id)
  }

  const startResize = (ev: PointerEvent) => {
    ev.preventDefault()
    setDragging(true)
    terminalDrawerActions.setOpen(true)
  }

  createEffect(() => {
    const target = qrDeviceUrl().trim()
    if (!target) {
      setQrImageDataUrl('')
      return
    }
    void QRCode.toDataURL(target, {
      width: 220,
      margin: 1,
      errorCorrectionLevel: 'M'
    })
      .then((dataUrl: string) => setQrImageDataUrl(dataUrl))
      .catch(() => setQrImageDataUrl(''))
  })

  createEffect(() => {
    if (typeof document === 'undefined') return
    if (!walletConnected()) {
      document.documentElement.style.removeProperty('--terminal-drawer-height')
      return
    }
    document.documentElement.style.setProperty('--terminal-drawer-height', `${drawerHeight()}px`)
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
      maybeRegisterCurrentOriginDevice,
      autoImportTailscaleDevices,
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
      importTailscaleDevices,
      tailscaleImporting,
      tailscaleImportNote,
      deviceNameInput,
      setDeviceNameInput,
      deviceUrlInput,
      setDeviceUrlInput,
      addDevice,
      connectDevice,
      qrDeviceUrl,
      setQrDeviceUrl,
      qrImageDataUrl
    },
    panes: {
      activeTab
    }
  }
}
