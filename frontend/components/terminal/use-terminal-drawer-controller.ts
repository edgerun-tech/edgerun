import { createEffect, createMemo, createSignal, onCleanup, onMount, untrack } from 'solid-js'
import QRCode from 'qrcode'
import {
  ensureTerminalDrawerStore,
  getTerminalDrawerState,
  subscribeTerminalDrawer,
  terminalDrawerActions,
  type TerminalDrawerState,
  type TerminalSplitMode
} from '../../lib/terminal-drawer-store'
import { WALLET_SESSION_EVENT, readWalletSession, type WalletSessionState } from '../../lib/wallet-session'
import { canUseCurrentOriginAsDevice, importTailscaleBridgeDevices, refreshTerminalDevices } from '../../lib/terminal-device-service'

export function useTerminalDrawerController() {
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
    ensureTerminalDrawerStore()
    setState(getTerminalDrawerState())
    const initialWallet = readWalletSession()
    setWallet(initialWallet)
    void maybeRegisterCurrentOriginDevice()
    if (initialWallet.connected) {
      restoreLastDevice()
      autoImportTailscaleDevices()
    }

    const unsubscribe = subscribeTerminalDrawer((next) => setState(next))

    const onWalletSession = (event: Event) => {
      const custom = event as CustomEvent<WalletSessionState>
      const nextWallet = custom.detail || readWalletSession()
      setWallet(nextWallet)
      if (nextWallet.connected) {
        void maybeRegisterCurrentOriginDevice()
        autoImportTailscaleDevices()
        restoreLastDevice()
        void refreshDeviceStatus()
      }
    }

    const onPointerMove = (ev: PointerEvent) => {
      if (!dragging() || !walletConnected()) return
      const minPx = Math.round(window.innerHeight * 0.2)
      const maxPx = Math.round(window.innerHeight * 0.85)
      const nextHeight = window.innerHeight - ev.clientY
      const clamped = Math.max(minPx, Math.min(maxPx, nextHeight))
      terminalDrawerActions.setHeightRatio(clamped / window.innerHeight)
    }

    const onPointerUp = () => setDragging(false)
    const onResize = () => setState(getTerminalDrawerState())
    const onPointerDown = (event: PointerEvent) => {
      const menuTab = tabMenuTabId()
      if (!menuTab) return
      const target = event.target as HTMLElement | null
      if (target?.closest('[data-tab-menu]')) return
      if (target?.closest('[data-tab-menu-trigger]')) return
      setTabMenuTabId(null)
    }

    window.addEventListener('pointermove', onPointerMove)
    window.addEventListener('pointerup', onPointerUp)
    window.addEventListener('pointercancel', onPointerUp)
    window.addEventListener('resize', onResize)
    window.addEventListener('pointerdown', onPointerDown)
    window.addEventListener(WALLET_SESSION_EVENT, onWalletSession as EventListener)

    void refreshDeviceStatus()
    const timer = window.setInterval(() => {
      void refreshDeviceStatus()
    }, 12000)

    onCleanup(() => {
      unsubscribe()
      document.documentElement.style.removeProperty('--terminal-drawer-height')
      window.removeEventListener('pointermove', onPointerMove)
      window.removeEventListener('pointerup', onPointerUp)
      window.removeEventListener('pointercancel', onPointerUp)
      window.removeEventListener('resize', onResize)
      window.removeEventListener('pointerdown', onPointerDown)
      window.removeEventListener(WALLET_SESSION_EVENT, onWalletSession as EventListener)
      window.clearInterval(timer)
    })
  })

  return {
    state,
    walletConnected,
    activeTab,
    tabMenuTabId,
    setTabMenuTabId,
    hasTabsLeft,
    hasTabsRight,
    hasOtherTabs,
    splitChange,
    refreshDeviceStatus,
    importTailscaleDevices,
    startResize,
    deviceNameInput,
    setDeviceNameInput,
    deviceUrlInput,
    setDeviceUrlInput,
    addDevice,
    tailscaleImporting,
    tailscaleImportNote,
    qrDeviceUrl,
    setQrDeviceUrl,
    qrImageDataUrl
  }
}
