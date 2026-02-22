import { normalizeRoutePath } from '../../lib/routes'

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
  const route = normalizeRoutePath(window.location.pathname)
  if (!route.startsWith('/docs/')) {
    docsSearchBoundPath = ''
    return
  }
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
      const encoded = root.getAttribute('data-docs-search-index') || '[]'
      const parsed = JSON.parse(encoded) as DocsSearchEntry[]
      index = Array.isArray(parsed) ? parsed : []
      if (!index.length) throw new Error('search_index_empty')
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
        return tokens.every((searchToken) => haystack.includes(searchToken))
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
  const route = normalizeRoutePath(window.location.pathname)
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

export async function initDocsEnhancements(): Promise<void> {
  await initDocsSearch()
  initDocsCodeCopyButtons()
}
