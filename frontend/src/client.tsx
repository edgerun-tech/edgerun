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
import { JOB_TAB_STATUS_EVENT, type JobTabStatus } from '../lib/tab-job-status'

declare global {
  interface Window {
    __EDGERUN_HYDRATED?: boolean
  }
}

const TERMINAL_DRAWER_HOST_ID = 'edgerun-terminal-drawer-root'
const TERMINAL_DRAWER_SECTION_ID = 'edgerun-terminal-drawer'

function routeTitle(pathname: string): string {
  return getClientRouteChromeTitle(pathname)
}

type SiteChromeStatus = {
  title: string
  kind: 'neutral' | 'running' | 'success' | 'warning' | 'error'
  color: string
  pulse: boolean
  progressPercent?: number
}

function computeJobChromeStatus(status: JobTabStatus): SiteChromeStatus {
  const clampedProgress = Math.max(0, Math.min(100, Math.round(status.progressPercent ?? 0)))
  if (status.phase === 'running') {
    const workers = Number.isFinite(status.workersActive) ? Math.max(0, Math.round(status.workersActive || 0)) : 0
    const workerPrefix = workers > 0 ? `[${workers} workers] ` : ''
    return {
      title: `[${clampedProgress}%] ${workerPrefix}Running — Edgerun`,
      kind: 'running',
      color: '#3b82f6',
      pulse: true,
      progressPercent: clampedProgress
    }
  }
  if (status.phase === 'quorum') {
    return { title: '✔ Quorum — Edgerun', kind: 'success', color: '#14b8a6', pulse: false }
  }
  if (status.phase === 'finalized') {
    return { title: '✓ Finalized — Edgerun', kind: 'success', color: '#22c55e', pulse: false }
  }
  if (status.phase === 'settled') {
    return { title: '✓ Settled — Edgerun', kind: 'success', color: '#16a34a', pulse: false }
  }
  if (status.phase === 'slashed') {
    return { title: '⚠ Slashed — Edgerun', kind: 'error', color: '#dc2626', pulse: false }
  }
  return { title: '⚠ Error — Edgerun', kind: 'error', color: '#ef4444', pulse: false }
}

function computeSiteChromeStatus(): SiteChromeStatus {
  if (jobTabStatus) {
    return computeJobChromeStatus(jobTabStatus)
  }
  const totalDevices = terminalDrawerState.devices.length
  const onlineDevices = terminalDrawerState.devices.filter((device) => device.status === 'online').length
  const terminalOpen = terminalDrawerState.open
  const walletConnected = walletSessionState.connected

  if (!walletConnected) {
    return {
      title: `${currentRouteTitle} · Wallet disconnected | Edgerun`,
      kind: 'neutral',
      color: '#64748b',
      pulse: false
    }
  }
  if (onlineDevices > 0) {
    const openLabel = terminalOpen ? 'Terminal open' : 'Terminal ready'
    return {
      title: `${currentRouteTitle} · ${openLabel} · ${onlineDevices}/${totalDevices} device${onlineDevices === 1 ? '' : 's'} online | Edgerun`,
      kind: 'success',
      color: '#22c55e',
      pulse: terminalOpen
    }
  }
  if (totalDevices > 0) {
    return {
      title: `${currentRouteTitle} · Wallet connected · ${totalDevices} configured device${totalDevices === 1 ? '' : 's'} (offline) | Edgerun`,
      kind: 'warning',
      color: '#f59e0b',
      pulse: false
    }
  }
  return {
    title: `${currentRouteTitle} · Wallet connected · no devices configured | Edgerun`,
    kind: 'warning',
    color: '#fb923c',
    pulse: false
  }
}

function initBrandFaviconAsset(): void {
  if (brandFaviconImage || typeof Image === 'undefined') return
  brandFaviconImage = new Image()
  brandFaviconImage.src = '/icon-192.png'
}

function drawRoundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number): void {
  ctx.beginPath()
  ctx.moveTo(x + r, y)
  ctx.arcTo(x + w, y, x + w, y + h, r)
  ctx.arcTo(x + w, y + h, x, y + h, r)
  ctx.arcTo(x, y + h, x, y, r)
  ctx.arcTo(x, y, x + w, y, r)
  ctx.closePath()
}

function faviconBadgeColor(kind: SiteChromeStatus['kind']): string {
  if (kind === 'running') return '#3b82f6'
  if (kind === 'success') return '#22c55e'
  if (kind === 'error') return '#dc2626'
  if (kind === 'warning') return '#f59e0b'
  return '#64748b'
}

function faviconPng(status: SiteChromeStatus, frame: number): string {
  const canvas = document.createElement('canvas')
  canvas.width = 64
  canvas.height = 64
  const ctx = canvas.getContext('2d')
  if (!ctx) return ''

  drawRoundRect(ctx, 0, 0, 64, 64, 14)
  ctx.fillStyle = '#05070d'
  ctx.fill()

  drawRoundRect(ctx, 5, 5, 54, 54, 11)
  ctx.save()
  ctx.clip()
  if (brandFaviconImage?.complete && brandFaviconImage.naturalWidth > 0) {
    ctx.imageSmoothingEnabled = true
    ctx.drawImage(brandFaviconImage, 5, 5, 54, 54)
  } else {
    const grad = ctx.createLinearGradient(8, 8, 56, 56)
    grad.addColorStop(0, '#0f172a')
    grad.addColorStop(1, '#111827')
    ctx.fillStyle = grad
    ctx.fillRect(5, 5, 54, 54)
    ctx.strokeStyle = '#d4d4d8'
    ctx.lineWidth = 4
    ctx.lineCap = 'round'
    ctx.beginPath()
    ctx.moveTo(20, 18)
    ctx.lineTo(44, 18)
    ctx.moveTo(20, 32)
    ctx.lineTo(40, 32)
    ctx.moveTo(20, 46)
    ctx.lineTo(44, 46)
    ctx.stroke()
  }
  ctx.restore()

  if (status.kind === 'running') {
    const progress = Math.max(0, Math.min(100, status.progressPercent ?? 0))
    const extra = status.pulse ? (frame % 2 === 0 ? 0 : 0.18) : 0
    const end = (-Math.PI / 2) + ((Math.PI * 2) * (progress / 100 + extra))
    ctx.strokeStyle = '#60a5fa'
    ctx.lineWidth = 4
    ctx.lineCap = 'round'
    ctx.beginPath()
    ctx.arc(32, 32, 28, -Math.PI / 2, end)
    ctx.stroke()
  }

  const badge = faviconBadgeColor(status.kind)
  const badgeRadius = status.pulse ? (frame % 2 === 0 ? 7.5 : 8.5) : 8
  ctx.beginPath()
  ctx.arc(50, 50, badgeRadius + 2, 0, Math.PI * 2)
  ctx.fillStyle = '#05070d'
  ctx.fill()
  ctx.beginPath()
  ctx.arc(50, 50, badgeRadius, 0, Math.PI * 2)
  ctx.fillStyle = badge
  ctx.fill()

  return canvas.toDataURL('image/png')
}

function updateFavicon(status: SiteChromeStatus): void {
  if (typeof document === 'undefined') return
  initBrandFaviconAsset()
  let link = document.querySelector<HTMLLinkElement>('link[data-edgerun-dynamic-favicon]')
  if (!link) {
    link = document.createElement('link')
    link.setAttribute('rel', 'icon')
    link.setAttribute('type', 'image/png')
    link.setAttribute('data-edgerun-dynamic-favicon', '1')
    document.head.appendChild(link)
  }
  const png = faviconPng(status, faviconFrame)
  if (png) link.setAttribute('href', png)
}

function renderSiteChrome(): void {
  const status = computeSiteChromeStatus()
  document.title = titleFlashOverride || status.title
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

function setTitleFlashOverride(nextTitle: string, durationMs: number): void {
  titleFlashOverride = nextTitle
  if (titleFlashTimer !== null) {
    window.clearTimeout(titleFlashTimer)
    titleFlashTimer = null
  }
  renderSiteChrome()
  titleFlashTimer = window.setTimeout(() => {
    titleFlashOverride = ''
    titleFlashTimer = null
    renderSiteChrome()
  }, durationMs)
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
  window.addEventListener(JOB_TAB_STATUS_EVENT, (event) => {
    const custom = event as CustomEvent<JobTabStatus | null>
    const next = custom.detail ?? null
    const shouldFlash = Boolean(next?.flashIfHidden && document.hidden)
    jobTabStatus = next
    if (shouldFlash) {
      const settled = next?.phase === 'settled' || next?.phase === 'finalized'
      const slashed = next?.phase === 'slashed'
      const label = settled ? '✔ Finalized — Edgerun' : slashed ? '⚠ Slashed — Edgerun' : '⚠ Job Update — Edgerun'
      setTitleFlashOverride(label, 5000)
      return
    }
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
let brandFaviconImage: HTMLImageElement | null = null
let currentRouteTitle = 'Home'
let jobTabStatus: JobTabStatus | null = null
let titleFlashOverride = ''
let titleFlashTimer: number | null = null
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
    let host = document.getElementById(TERMINAL_DRAWER_HOST_ID)
    if (!host) {
      host = document.createElement('div')
      host.id = TERMINAL_DRAWER_HOST_ID
      document.body.appendChild(host)
    }
    host.innerHTML = ''
    render(() => <TerminalDrawer />, host)
    terminalDrawerMounted = true
  } catch {
    terminalDrawerMounted = false
  } finally {
    terminalDrawerMounting = false
  }
}

function forceRemountTerminalDrawer(): void {
  if (typeof document === 'undefined') return
  const host = document.getElementById(TERMINAL_DRAWER_HOST_ID)
  if (host) host.remove()
  terminalDrawerMounted = false
  void mountGlobalTerminalDrawer()
}

function ensureTerminalDrawerMounted(): void {
  if (typeof document === 'undefined') return
  const host = document.getElementById(TERMINAL_DRAWER_HOST_ID)
  const section = document.getElementById(TERMINAL_DRAWER_SECTION_ID)
  if (!host || !section) {
    forceRemountTerminalDrawer()
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
  ensureTerminalDrawerMounted()
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
  ensureTerminalDrawerMounted()
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
document.addEventListener('visibilitychange', () => {
  if (!document.hidden) ensureTerminalDrawerMounted()
})
window.addEventListener('edgerun:terminal-rerender', () => {
  forceRemountTerminalDrawer()
})
queueMicrotask(() => {
  import('../lib/webrtc-peer-supervisor')
    .then((mod) => mod.initWebRtcPeerSupervisor())
    .catch(() => {
      // WebRTC supervisor is an enhancement path; fail-soft on bootstrap.
    })
  import('../lib/routed-terminal-shell')
    .then((mod) => mod.initRoutedTerminalShell())
    .catch(() => {
      // Routed terminal shell is an enhancement path; fail-soft on bootstrap.
    })
})
