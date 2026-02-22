import { render } from 'solid-js/web'

import { applyPersonalizationSettings, readPersonalizationSettings } from '../lib/personalization'
import {
  getClientRouteChromeTitle,
  hasClientRoute,
  loadClientRouteComponent,
  normalizeClientRoutePath
} from '../lib/client-routes'
import { ensureTerminalDrawerStore, getTerminalDrawerState, subscribeTerminalDrawer, type TerminalDrawerState } from '../lib/terminal-drawer-store'
import { WALLET_SESSION_EVENT, readWalletSession, type WalletSessionState } from '../lib/wallet-session'

declare global {
  interface Window {
    __EDGERUN_HYDRATED?: boolean
  }
}

function routeTitle(pathname: string): string {
  return getClientRouteChromeTitle(pathname)
}

type SiteChromeStatus = {
  emoji: string
  label: string
  color: string
  pulse: boolean
}

function computeSiteChromeStatus(): SiteChromeStatus {
  const totalDevices = terminalDrawerState.devices.length
  const onlineDevices = terminalDrawerState.devices.filter((device) => device.status === 'online').length
  const terminalOpen = terminalDrawerState.open
  const walletConnected = walletSessionState.connected

  if (!walletConnected) {
    return {
      emoji: '⚪',
      label: 'Wallet disconnected',
      color: '#64748b',
      pulse: false
    }
  }
  if (onlineDevices > 0) {
    const openLabel = terminalOpen ? 'Terminal open' : 'Terminal ready'
    return {
      emoji: '🟢',
      label: `${openLabel} · ${onlineDevices}/${totalDevices} device${onlineDevices === 1 ? '' : 's'} online`,
      color: '#22c55e',
      pulse: terminalOpen
    }
  }
  if (totalDevices > 0) {
    return {
      emoji: '🟡',
      label: `Wallet connected · ${totalDevices} configured device${totalDevices === 1 ? '' : 's'} (offline)`,
      color: '#f59e0b',
      pulse: false
    }
  }
  return {
    emoji: '🟠',
    label: 'Wallet connected · no devices configured',
    color: '#fb923c',
    pulse: false
  }
}

function faviconSvg(status: SiteChromeStatus, frame: number): string {
  const glow = status.pulse ? (frame % 2 === 0 ? 15 : 19) : 16
  const core = status.pulse ? (frame % 2 === 0 ? 9 : 11) : 10
  const ringOpacity = status.pulse ? (frame % 2 === 0 ? 0.3 : 0.55) : 0.35
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="#05070d"/><circle cx="32" cy="32" r="${glow}" fill="${status.color}" opacity="${ringOpacity}"/><circle cx="32" cy="32" r="${core}" fill="${status.color}"/><path d="M18 48h28" stroke="#d4d4d8" stroke-width="3" stroke-linecap="round" opacity="0.9"/></svg>`
}

function updateFavicon(status: SiteChromeStatus): void {
  if (typeof document === 'undefined') return
  let link = document.querySelector<HTMLLinkElement>('link[data-edgerun-dynamic-favicon]')
  if (!link) {
    link = document.createElement('link')
    link.setAttribute('rel', 'icon')
    link.setAttribute('type', 'image/svg+xml')
    link.setAttribute('data-edgerun-dynamic-favicon', '1')
    document.head.appendChild(link)
  }
  const svg = faviconSvg(status, faviconFrame)
  link.setAttribute('href', `data:image/svg+xml,${encodeURIComponent(svg)}`)
}

function renderSiteChrome(): void {
  const status = computeSiteChromeStatus()
  document.title = `${status.emoji} ${currentRouteTitle} · ${status.label} | Edgerun`
  updateFavicon(status)
  if (status.pulse) {
    if (faviconAnimationTimer === null) {
      faviconAnimationTimer = window.setInterval(() => {
        faviconFrame = (faviconFrame + 1) % 4
        updateFavicon(computeSiteChromeStatus())
      }, 700)
    }
  } else if (faviconAnimationTimer !== null) {
    window.clearInterval(faviconAnimationTimer)
    faviconAnimationTimer = null
    faviconFrame = 0
    updateFavicon(status)
  }
}

function initSiteChromeStatus(): void {
  if (chromeStatusInitialized) return
  chromeStatusInitialized = true
  ensureTerminalDrawerStore()
  walletSessionState = readWalletSession()
  terminalDrawerState = getTerminalDrawerState()
  currentRouteTitle = routeTitle(window.location.pathname)
  renderSiteChrome()
  subscribeTerminalDrawer((next) => {
    terminalDrawerState = next
    renderSiteChrome()
  })
  window.addEventListener(WALLET_SESSION_EVENT, (event) => {
    const custom = event as CustomEvent<WalletSessionState>
    walletSessionState = custom.detail || readWalletSession()
    renderSiteChrome()
  })
}

let disposePage: null | (() => void) = null
let bootstrapped = false
let terminalDrawerMounted = false
let terminalDrawerMounting = false
let transitionInFlight = false
let chromeStatusInitialized = false
let walletSessionState: WalletSessionState = readWalletSession()
let terminalDrawerState: TerminalDrawerState = getTerminalDrawerState()
let faviconAnimationTimer: number | null = null
let faviconFrame = 0
let currentRouteTitle = 'Home'
let docsEnhancementsModulePromise: Promise<typeof import('./runtime/docs-enhancements')> | null = null
let chainStatusModulePromise: Promise<typeof import('./runtime/chain-status')> | null = null

if (typeof window !== 'undefined') {
  window.__EDGERUN_HYDRATED = false
  applyPersonalizationSettings(readPersonalizationSettings())
}

async function mountGlobalTerminalDrawer(): Promise<void> {
  if (terminalDrawerMounted) return
  if (terminalDrawerMounting) return
  if (typeof document === 'undefined') return

  terminalDrawerMounting = true
  ensureTerminalDrawerStore()
  try {
    const { TerminalDrawer } = await import('../components/terminal/terminal-drawer')
    const host = document.createElement('div')
    host.id = 'edgerun-terminal-drawer-root'
    document.body.appendChild(host)
    render(() => <TerminalDrawer />, host)
    terminalDrawerMounted = true
  } finally {
    terminalDrawerMounting = false
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}

async function runRouteTransition(update: () => void | Promise<void>): Promise<void> {
  const withTransition = document as Document & {
    startViewTransition?: (callback: () => void | Promise<void>) => { finished: Promise<void> }
  }

  if (typeof withTransition.startViewTransition === 'function') {
    const transition = withTransition.startViewTransition(() => update())
    try {
      await transition.finished
    } catch {
      // ignore cancelled transitions
    }
    return
  }

  const root = document.getElementById('edgerun-root')
  if (!root) {
    await update()
    return
  }

  root.classList.add('route-fade-out')
  await sleep(80)
  await update()
  root.classList.remove('route-fade-out')
  root.classList.add('route-fade-in')
  window.requestAnimationFrame(() => {
    root.classList.remove('route-fade-in')
  })
}

function applyPageEnhancements(): void {
  const yearEl = document.querySelector<HTMLElement>('[data-current-year]')
  if (yearEl) yearEl.textContent = String(new Date().getFullYear())

  const navLinks = document.querySelectorAll<HTMLAnchorElement>('[data-nav-link]')
  const path = normalizeClientRoutePath(window.location.pathname)
  if (navLinks.length) {
    for (const link of navLinks) {
      const href = normalizeClientRoutePath(link.getAttribute('href') || '')
      if (!href) continue
      const active = href === '/' ? path === '/' : path === href || path.startsWith(href)
      link.classList.toggle('is-active', active)
      if (active) link.setAttribute('aria-current', 'page')
      else link.removeAttribute('aria-current')
    }
  }

  void loadPageFeatureEnhancements(path)
}

function pageHasChainWidgets(): boolean {
  return Boolean(document.querySelector('[data-chain-field], [data-deployment-badge], [data-deployment-detail]'))
}

async function loadPageFeatureEnhancements(path: string): Promise<void> {
  if (path.startsWith('/docs/')) {
    docsEnhancementsModulePromise ??= import('./runtime/docs-enhancements')
    const docsEnhancements = await docsEnhancementsModulePromise.catch(() => null)
    await docsEnhancements?.initDocsEnhancements()
  }

  if (pageHasChainWidgets()) {
    chainStatusModulePromise ??= import('./runtime/chain-status')
    const chainStatus = await chainStatusModulePromise.catch(() => null)
    await chainStatus?.initChainStatusWidgets()
  }
}

async function mountCurrentRoute(): Promise<boolean> {
  const root = document.getElementById('edgerun-root')
  if (!root) return false
  const route = normalizeClientRoutePath(window.location.pathname)
  const Page = await loadClientRouteComponent(route)
  if (!Page) return false

  if (disposePage) {
    disposePage()
    disposePage = null
  }

  root.innerHTML = ''
  disposePage = render(() => <Page />, root)
  currentRouteTitle = routeTitle(route)
  bootstrapped = true
  window.__EDGERUN_HYDRATED = true
  applyPageEnhancements()
  renderSiteChrome()
  return true
}

function shouldClientRoute(anchor: HTMLAnchorElement): boolean {
  const rawHref = anchor.getAttribute('href')
  if (!rawHref || rawHref.startsWith('#')) return false
  if (anchor.target && anchor.target !== '_self') return false
  if (anchor.hasAttribute('download')) return false

  const url = new URL(rawHref, window.location.origin)
  if (url.origin !== window.location.origin) return false
  return hasClientRoute(url.pathname)
}

document.addEventListener('change', (event) => {
  const target = event.target as HTMLElement | null
  if (!target || !(target instanceof HTMLSelectElement) || target.id !== 'version-select') return
  const v = target.value.trim()
  if (!v) return
  window.location.href = `/docs/${v}/`
})

document.addEventListener('click', async (event) => {
  if (event.defaultPrevented) return
  if (event.button !== 0) return
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return
  if (transitionInFlight) return

  const node = event.target as HTMLElement | null
  const anchor = node?.closest('a') as HTMLAnchorElement | null
  if (!anchor || !shouldClientRoute(anchor)) return

  const nextUrl = new URL(anchor.href, window.location.origin)
  const nextPath = normalizeClientRoutePath(nextUrl.pathname)
  const currentPath = normalizeClientRoutePath(window.location.pathname)
  if (nextPath === currentPath) return

  event.preventDefault()
  transitionInFlight = true
  await runRouteTransition(async () => {
    window.history.pushState({}, '', nextPath)
    const mounted = await mountCurrentRoute()
    if (!mounted) window.location.assign(nextPath)
  })
  transitionInFlight = false
})

window.addEventListener('popstate', async () => {
  if (!bootstrapped) return
  if (transitionInFlight) return
  transitionInFlight = true
  await runRouteTransition(async () => {
    const mounted = await mountCurrentRoute()
    if (!mounted) window.location.assign(window.location.pathname)
  })
  transitionInFlight = false
})

void mountCurrentRoute().then((mounted) => {
  if (!mounted) {
    applyPageEnhancements()
    currentRouteTitle = routeTitle(window.location.pathname)
    renderSiteChrome()
  }
  window.__EDGERUN_HYDRATED = true
})

void mountGlobalTerminalDrawer()
initSiteChromeStatus()
queueMicrotask(() => {
  import('../lib/webrtc-peer-supervisor')
    .then((mod) => mod.initWebRtcPeerSupervisor())
    .catch(() => {
      // WebRTC supervisor is an enhancement path; fail-soft on bootstrap.
    })
})
