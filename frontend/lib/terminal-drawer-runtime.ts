import { ensureTerminalDrawerStore, getTerminalDrawerState, subscribeTerminalDrawer, terminalDrawerActions, type TerminalDrawerState } from './terminal-drawer-store'
import { WALLET_SESSION_EVENT, readWalletSession, type WalletSessionState } from './wallet-session'

type RuntimeOptions = {
  setState: (next: TerminalDrawerState) => void
  setWallet: (next: WalletSessionState) => void
  walletConnected: () => boolean
  dragging: () => boolean
  setDragging: (next: boolean) => void
  tabMenuTabId: () => string | null
  closeTabMenu: () => void
  refreshDeviceStatus: () => Promise<void>
  restoreLastDevice: () => void
}

export function mountTerminalDrawerRuntime(options: RuntimeOptions): () => void {
  ensureTerminalDrawerStore()
  options.setState(getTerminalDrawerState())
  const initialWallet = readWalletSession()
  options.setWallet(initialWallet)
  if (initialWallet.connected) {
    options.restoreLastDevice()
  }

  const unsubscribe = subscribeTerminalDrawer((next) => options.setState(next))

  const onWalletSession = (event: Event) => {
    const custom = event as CustomEvent<WalletSessionState>
    const nextWallet = custom.detail || readWalletSession()
    options.setWallet(nextWallet)
    if (nextWallet.connected) {
      options.restoreLastDevice()
      void options.refreshDeviceStatus()
    }
  }

  const onPointerMove = (ev: PointerEvent) => {
    if (!options.dragging() || !options.walletConnected()) return
    const minPx = Math.round(window.innerHeight * 0.2)
    const maxPx = Math.round(window.innerHeight * 0.85)
    const nextHeight = window.innerHeight - ev.clientY
    const clamped = Math.max(minPx, Math.min(maxPx, nextHeight))
    terminalDrawerActions.setHeightRatio(clamped / window.innerHeight)
  }

  const onPointerUp = () => options.setDragging(false)
  const onResize = () => options.setState(getTerminalDrawerState())
  const onPointerDown = (event: PointerEvent) => {
    const menuTab = options.tabMenuTabId()
    if (!menuTab) return
    const target = event.target as HTMLElement | null
    if (target?.closest('[data-tab-menu]')) return
    if (target?.closest('[data-tab-menu-trigger]')) return
    options.closeTabMenu()
  }
  const onMessage = (event: MessageEvent) => {
    const payload = event.data
    if (!payload || typeof payload !== 'object') return
    const source = (payload as { source?: unknown }).source
    if (source !== 'edgerun-term-web') return
    const type = (payload as { type?: unknown }).type
    if (type !== 'transport') return
    const sid = (payload as { sid?: unknown }).sid
    const transport = (payload as { transport?: unknown }).transport
    if (typeof sid !== 'string' || !sid) return
    if (transport !== 'mux' && transport !== 'raw' && transport !== 'unknown') return
    terminalDrawerActions.setPaneTransport(sid, transport)
  }

  window.addEventListener('pointermove', onPointerMove)
  window.addEventListener('pointerup', onPointerUp)
  window.addEventListener('pointercancel', onPointerUp)
  window.addEventListener('resize', onResize)
  window.addEventListener('pointerdown', onPointerDown)
  window.addEventListener('message', onMessage)
  window.addEventListener(WALLET_SESSION_EVENT, onWalletSession as EventListener)

  void options.refreshDeviceStatus()
  const timer = window.setInterval(() => {
    void options.refreshDeviceStatus()
  }, 12000)

  return () => {
    unsubscribe()
    window.removeEventListener('pointermove', onPointerMove)
    window.removeEventListener('pointerup', onPointerUp)
    window.removeEventListener('pointercancel', onPointerUp)
    window.removeEventListener('resize', onResize)
    window.removeEventListener('pointerdown', onPointerDown)
    window.removeEventListener('message', onMessage)
    window.removeEventListener(WALLET_SESSION_EVENT, onWalletSession as EventListener)
    window.clearInterval(timer)
  }
}
