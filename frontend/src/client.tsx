import type { Component } from 'solid-js'
import { render } from 'solid-js/web'

import HomePage from '../app/page'
import DocsPage from '../app/docs/page'
import QuickStartPage from '../app/docs/getting-started/quick-start/page'
import TokenPage from '../app/token/page'
import RunPage from '../app/run/page'
import WorkersPage from '../app/workers/page'
import DashboardPage from '../app/dashboard/page'
import BlogPage from '../app/blog/page'
import BlogPostPage from '../app/blog/[slug]/page'
import JobDetailsPage from '../app/job/[id]/page'
import PrivacyPage from '../app/legal/privacy/page'
import TermsPage from '../app/legal/terms/page'
import SlaPage from '../app/legal/sla/page'
import StyleGuidePage from '../app/style-guide/page'
import { blogPosts, jobs } from '../lib/content'
import { readRuntimeRpcConfig, RPC_CONFIG_EVENT } from '../lib/solana-config'
import { getConfiguredProgramCount, getConfiguredProgramIds } from '../lib/solana-deployments'
import { TerminalDrawer } from '../components/terminal/terminal-drawer'
import { ensureTerminalDrawerStore } from '../lib/terminal-drawer-store'

function normalizePath(pathname: string): string {
  const cleaned = pathname.replace(/index\.html$/, '')
  if (!cleaned) return '/'
  return cleaned.endsWith('/') ? cleaned : `${cleaned}/`
}

const routeComponents: Record<string, Component> = {
  '/': HomePage,
  '/docs/': DocsPage,
  '/docs/getting-started/quick-start/': QuickStartPage,
  '/token/': TokenPage,
  '/run/': RunPage,
  '/workers/': WorkersPage,
  '/dashboard/': DashboardPage,
  '/blog/': BlogPage,
  '/legal/privacy/': PrivacyPage,
  '/legal/terms/': TermsPage,
  '/legal/sla/': SlaPage,
  '/style-guide/': StyleGuidePage
}
for (const post of blogPosts) routeComponents[`/blog/${post.slug}/`] = () => <BlogPostPage slug={post.slug} />
for (const job of jobs) routeComponents[`/job/${job.id}/`] = () => <JobDetailsPage id={job.id} />

let disposePage: null | (() => void) = null
let bootstrapped = false
let terminalDrawerMounted = false
let transitionInFlight = false

function mountGlobalTerminalDrawer(): void {
  if (terminalDrawerMounted) return
  if (typeof document === 'undefined') return

  ensureTerminalDrawerStore()
  const host = document.createElement('div')
  host.id = 'edgerun-terminal-drawer-root'
  document.body.appendChild(host)
  render(() => <TerminalDrawer />, host)
  terminalDrawerMounted = true
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}

async function runRouteTransition(update: () => void): Promise<void> {
  const withTransition = document as Document & {
    startViewTransition?: (callback: () => void) => { finished: Promise<void> }
  }

  if (typeof withTransition.startViewTransition === 'function') {
    const transition = withTransition.startViewTransition(() => {
      update()
    })
    try {
      await transition.finished
    } catch {
      // ignore cancelled transitions
    }
    return
  }

  const root = document.getElementById('edgerun-root')
  if (!root) {
    update()
    return
  }

  root.classList.add('route-fade-out')
  await sleep(80)
  update()
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
  if (!navLinks.length) return
  const path = normalizePath(window.location.pathname)
  for (const link of navLinks) {
    const href = normalizePath(link.getAttribute('href') || '')
    if (!href) continue
    const active = href === '/' ? path === '/' : path === href || path.startsWith(href)
    link.classList.toggle('is-active', active)
    if (active) link.setAttribute('aria-current', 'page')
    else link.removeAttribute('aria-current')
  }

  void initDocsSearch()
  initDocsCodeCopyButtons()
}

function mountCurrentRoute(): boolean {
  const root = document.getElementById('edgerun-root')
  if (!root) return false
  const route = normalizePath(window.location.pathname)
  const Page = routeComponents[route]
  if (!Page) return false

  if (disposePage) {
    disposePage()
    disposePage = null
  }

  root.innerHTML = ''
  disposePage = render(() => <Page />, root)
  bootstrapped = true
  applyPageEnhancements()
  void loadChainData()
  void loadDeploymentStatus()
  return true
}

function shouldClientRoute(anchor: HTMLAnchorElement): boolean {
  const rawHref = anchor.getAttribute('href')
  if (!rawHref || rawHref.startsWith('#')) return false
  if (anchor.target && anchor.target !== '_self') return false
  if (anchor.hasAttribute('download')) return false

  const url = new URL(rawHref, window.location.origin)
  if (url.origin !== window.location.origin) return false
  const route = normalizePath(url.pathname)
  return Boolean(routeComponents[route])
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
  const nextPath = normalizePath(nextUrl.pathname)
  const currentPath = normalizePath(window.location.pathname)
  if (nextPath === currentPath) return

  event.preventDefault()
  transitionInFlight = true
  await runRouteTransition(() => {
    window.history.pushState({}, '', nextPath)
    const mounted = mountCurrentRoute()
    if (!mounted) window.location.assign(nextPath)
  })
  transitionInFlight = false
})

window.addEventListener('popstate', async () => {
  if (!bootstrapped) return
  if (transitionInFlight) return
  transitionInFlight = true
  await runRouteTransition(() => {
    const mounted = mountCurrentRoute()
    if (!mounted) window.location.assign(window.location.pathname)
  })
  transitionInFlight = false
})

if (!mountCurrentRoute()) {
  applyPageEnhancements()
  void loadChainData()
  void loadDeploymentStatus()
}

mountGlobalTerminalDrawer()

type RpcEnvelope<T> = {
  jsonrpc: string
  id: number
  result?: T
  error?: { code: number; message: string }
}

function setField(name: string, value: string): void {
  const els = document.querySelectorAll<HTMLElement>(`[data-chain-field="${name}"]`)
  for (const el of els) el.textContent = value
}

function setText(selector: string, value: string): void {
  const el = document.querySelector<HTMLElement>(selector)
  if (el) el.textContent = value
}

function formatInt(value: number): string {
  return new Intl.NumberFormat('en-US').format(value)
}

function formatSol(lamports: number): string {
  return `${(lamports / 1_000_000_000).toLocaleString('en-US', { maximumFractionDigits: 4 })} SOL`
}

async function rpcCall<T>(rpcUrl: string, method: string, params: unknown[] = []): Promise<T> {
  const res = await fetch(rpcUrl, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      id: Date.now(),
      method,
      params
    })
  })

  if (!res.ok) throw new Error(`rpc_http_${res.status}`)
  const payload = (await res.json()) as RpcEnvelope<T>
  if (payload.error) throw new Error(payload.error.message)
  if (payload.result === undefined) throw new Error('rpc_no_result')
  return payload.result
}

async function loadChainData(): Promise<void> {
  if (!document.querySelector('[data-chain-field]')) return
  const cfg = readRuntimeRpcConfig()
  if (!cfg?.rpcUrl) return

  setField('cluster', cfg.cluster || 'unknown')
  setField('rpcUrl', cfg.rpcUrl || 'unknown')

  try {
    const [slot, blockHeight, epochInfo, perf, supply] = await Promise.all([
      rpcCall<number>(cfg.rpcUrl, 'getSlot', []),
      rpcCall<number>(cfg.rpcUrl, 'getBlockHeight', []),
      rpcCall<{ epoch: number }>(cfg.rpcUrl, 'getEpochInfo', []),
      rpcCall<Array<{ numTransactions: number; samplePeriodSecs: number }>>(
        cfg.rpcUrl,
        'getRecentPerformanceSamples',
        [1]
      ),
      rpcCall<{ value: { total: number } }>(cfg.rpcUrl, 'getSupply', [])
    ])

    setField('slot', formatInt(slot))
    setField('blockHeight', formatInt(blockHeight))
    setField('epoch', formatInt(epochInfo.epoch))
    setField('supplySol', formatSol(supply.value.total))

    const sample = perf[0]
    if (sample && sample.samplePeriodSecs > 0) {
      setField('tps', (sample.numTransactions / sample.samplePeriodSecs).toFixed(2))
    } else {
      setField('tps', 'n/a')
    }

    if (cfg.treasuryAccount) {
      const balance = await rpcCall<{ value: number }>(cfg.rpcUrl, 'getBalance', [cfg.treasuryAccount, { commitment: 'confirmed' }])
      setField('treasurySol', formatSol(balance.value))
    } else {
      setField('treasurySol', 'not configured')
    }
  } catch {
    const fallback = 'rpc unavailable'
    setField('slot', fallback)
    setField('blockHeight', fallback)
    setField('epoch', fallback)
    setField('tps', fallback)
    setField('supplySol', fallback)
    setField('treasurySol', fallback)
  }
}

async function isExecutableProgram(rpcUrl: string, programId: string): Promise<boolean> {
  const info = await rpcCall<{ value: { executable?: boolean } | null }>(rpcUrl, 'getAccountInfo', [
    programId,
    { commitment: 'confirmed', encoding: 'base64' }
  ])
  return Boolean(info.value?.executable)
}

async function loadDeploymentStatus(): Promise<void> {
  if (!document.querySelector('[data-deployment-badge]') && !document.querySelector('[data-deployment-detail]')) return
  const cfg = readRuntimeRpcConfig()
  const cluster = cfg.cluster || 'unknown'
  const rpcUrl = cfg.rpcUrl || ''
  const configuredCount = getConfiguredProgramCount(cluster)
  const badgePrefix = cluster === 'localnet' ? 'Live on Localnet' : `Cluster: ${cluster}`

  if (!configuredCount) {
    setText('[data-deployment-badge]', `${badgePrefix} (No deployment)`)
    setText('[data-deployment-detail]', `No program IDs configured for ${cluster} yet.`)
    return
  }
  if (!rpcUrl) {
    setText('[data-deployment-badge]', `${badgePrefix} (RPC unavailable)`)
    setText('[data-deployment-detail]', `Configured program IDs: ${configuredCount}. Live verification requires RPC connectivity.`)
    return
  }

  try {
    const ids = getConfiguredProgramIds(cluster)
    const checks = await Promise.all(ids.map((id) => isExecutableProgram(rpcUrl, id)))
    const liveCount = checks.filter(Boolean).length
    const isLive = liveCount > 0
    setText('[data-deployment-badge]', isLive ? `${badgePrefix} (${liveCount}/${configuredCount} live)` : `${badgePrefix} (Not deployed)`)
    setText('[data-deployment-detail]', `Program deployments verified on ${cluster} via RPC: ${liveCount} of ${configuredCount} configured IDs.`)
  } catch {
    setText('[data-deployment-badge]', `${badgePrefix} (Verification unavailable)`)
    setText('[data-deployment-detail]', `Configured program IDs: ${configuredCount}. Could not verify deployments against current RPC endpoint.`)
  }
}

window.addEventListener(RPC_CONFIG_EVENT, () => {
  void loadChainData()
  void loadDeploymentStatus()
})

type DocsSearchEntry = {
  title: string
  href: string
  text: string
}

let docsSearchBoundPath = ''
let docsCopyBoundPath = ''
const docsSearchQueryKeyPrefix = 'edgerun_docs_search_query_'
const docsSearchIndexCache = new Map<string, DocsSearchEntry[]>()
let docsSearchCleanup: null | (() => void) = null
let docsSearchToken = 0

function escapeHtml(value: string): string {
  return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&#39;')
}

function setCopyIcon(button: HTMLButtonElement, mode: 'copy' | 'copied'): void {
  const ns = 'http://www.w3.org/2000/svg'
  const svg = document.createElementNS(ns, 'svg')
  svg.setAttribute('viewBox', '0 0 24 24')
  svg.setAttribute('aria-hidden', 'true')
  const path = document.createElementNS(ns, 'path')
  if (mode === 'copied') {
    path.setAttribute('d', 'M9.2 16.6 4.6 12l1.4-1.4 3.2 3.2 8.8-8.8 1.4 1.4z')
  } else {
    path.setAttribute('d', 'M16 1H8a2 2 0 0 0-2 2v2H5a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-2h1a2 2 0 0 0 2-2V7zM8 3h8v2H8zm7 17H5V7h10zm3-4h-1V7a2 2 0 0 0-2-2H8V4h8l2 3z')
  }
  svg.appendChild(path)
  button.replaceChildren(svg)
}

function renderDocsSearchMessage(container: HTMLElement, message: string): void {
  container.innerHTML = `<p class="rounded border border-border bg-muted/20 px-2 py-1 text-muted-foreground">${message}</p>`
}

function renderDocsSearchResults(container: HTMLElement, results: DocsSearchEntry[]): void {
  if (!results.length) {
    renderDocsSearchMessage(container, 'No matching docs yet.')
    return
  }
  const listItems = results.slice(0, 8).map((entry) => {
    const title = escapeHtml(entry.title)
    const href = escapeHtml(entry.href)
    return `<li><a class="block rounded border border-border bg-background px-2 py-1 hover:border-primary/50 hover:bg-muted/30" href="${href}">${title}</a></li>`
  }).join('')
  container.innerHTML = `<ul class="space-y-1">${listItems}</ul>`
}

async function initDocsSearch(): Promise<void> {
  const route = normalizePath(window.location.pathname)
  if (docsSearchBoundPath === route) return
  if (docsSearchCleanup) {
    docsSearchCleanup()
    docsSearchCleanup = null
  }
  const root = document.querySelector<HTMLElement>('[data-docs-search]')
  const input = document.querySelector<HTMLInputElement>('[data-docs-search-input]')
  const results = document.querySelector<HTMLElement>('[data-docs-search-results]')
  const version = root?.getAttribute('data-docs-version')?.trim()
  if (!root || !input || !results || !version) {
    docsSearchBoundPath = ''
    return
  }
  const token = ++docsSearchToken
  docsSearchBoundPath = route

  root.setAttribute('aria-busy', 'true')
  input.disabled = true
  input.placeholder = 'Search by keyword...'
  renderDocsSearchMessage(results, 'Type at least 2 characters.')

  let index = docsSearchIndexCache.get(version) ?? null
  if (!index) {
    try {
      const response = await fetch(`/docs/${encodeURIComponent(version)}/search-index.json`, { headers: { accept: 'application/json' } })
      if (!response.ok) throw new Error(`search_index_${response.status}`)
      index = await response.json() as DocsSearchEntry[]
      docsSearchIndexCache.set(version, index)
    } catch {
      if (token !== docsSearchToken) return
      input.disabled = true
      input.placeholder = 'Search unavailable'
      root.setAttribute('aria-busy', 'false')
      renderDocsSearchMessage(results, 'Search index unavailable on this page.')
      return
    }
  }
  if (token !== docsSearchToken) return
  input.disabled = false
  input.placeholder = 'Search by keyword...'
  root.setAttribute('aria-busy', 'false')

  const queryStorageKey = `${docsSearchQueryKeyPrefix}${version}`
  const restoreQuery = window.sessionStorage.getItem(queryStorageKey)?.trim() || ''
  if (restoreQuery.length >= 2) input.value = restoreQuery

  let timer: number | null = null
  const onInput = (): void => {
    if (timer !== null) window.clearTimeout(timer)
    timer = window.setTimeout(() => {
      if (token !== docsSearchToken) return
      const query = input.value.trim().toLowerCase()
      window.sessionStorage.setItem(queryStorageKey, query)
      if (query.length < 2) {
        renderDocsSearchMessage(results, 'Type at least 2 characters.')
        return
      }
      const tokens = query.split(/\s+/g).filter(Boolean)
      const matched = index.filter((entry) => {
        const haystack = `${entry.title} ${entry.text}`.toLowerCase()
        return tokens.every((token) => haystack.includes(token))
      })
      renderDocsSearchResults(results, matched)
    }, 90)
  }

  input.addEventListener('input', onInput)
  onInput()
  docsSearchCleanup = () => {
    input.removeEventListener('input', onInput)
    if (timer !== null) window.clearTimeout(timer)
  }
}

function copyTextToClipboard(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) return navigator.clipboard.writeText(text)
  return new Promise((resolve, reject) => {
    const area = document.createElement('textarea')
    area.value = text
    area.setAttribute('readonly', 'true')
    area.style.position = 'fixed'
    area.style.opacity = '0'
    document.body.appendChild(area)
    area.select()
    const ok = document.execCommand('copy')
    document.body.removeChild(area)
    if (ok) resolve()
    else reject(new Error('copy_failed'))
  })
}

function initDocsCodeCopyButtons(): void {
  const route = normalizePath(window.location.pathname)
  if (!route.startsWith('/docs/')) return
  if (docsCopyBoundPath === route) return
  docsCopyBoundPath = route

  const blocks = document.querySelectorAll<HTMLElement>('pre')
  for (const pre of blocks) {
    if (pre.dataset.copyReady === 'true') continue
    pre.dataset.copyReady = 'true'
    pre.classList.add('code-copy-host')

    const button = document.createElement('button')
    button.type = 'button'
    button.className = 'code-copy-button'
    setCopyIcon(button, 'copy')
    button.setAttribute('aria-label', 'Copy code block')
    button.setAttribute('title', 'Copy code')

    button.addEventListener('click', async () => {
      const codeEl = pre.querySelector('code')
      const text = (codeEl?.textContent || pre.textContent || '').trim()
      if (!text) return
      try {
        await copyTextToClipboard(text)
        button.classList.add('is-copied')
        setCopyIcon(button, 'copied')
        button.setAttribute('aria-label', 'Copied')
        button.setAttribute('title', 'Copied')
        window.setTimeout(() => {
          button.classList.remove('is-copied')
          button.classList.remove('is-error')
          setCopyIcon(button, 'copy')
          button.setAttribute('aria-label', 'Copy code block')
          button.setAttribute('title', 'Copy code')
        }, 1300)
      } catch {
        button.classList.add('is-error')
        setCopyIcon(button, 'copy')
        button.setAttribute('aria-label', 'Copy failed')
        button.setAttribute('title', 'Copy failed')
        window.setTimeout(() => {
          button.classList.remove('is-error')
          setCopyIcon(button, 'copy')
          button.setAttribute('aria-label', 'Copy code block')
          button.setAttribute('title', 'Copy code')
        }, 1300)
      }
    })

    pre.appendChild(button)
  }
}
