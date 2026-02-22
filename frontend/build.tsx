import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { renderToString } from 'solid-js/web'
import { createHighlighter } from 'shiki'

import { writeStaticSiteMetadata } from './lib/build-static-meta'
import { createDocsRenderer } from './lib/build-docs-render'
import { getDocsNav } from './lib/docs-nav'
import { generatedApiSpecs, getDocsSources, type DocsSource } from './lib/docs-catalog'
import { getAllSiteRoutes } from './lib/routes'
import { siteLinks } from './lib/site-links'
import { DocsSidebar } from './components/docs/docs-sidebar'
import { Nav } from './components/nav'
import { Footer } from './components/footer'

const projectRoot = process.env.EDGERUN_FRONTEND_ROOT || process.cwd()
const repoRoot = path.resolve(projectRoot, '..')
const defaultFrontendOutRoot = path.join(repoRoot, 'out', 'frontend')
const distRoot = process.env.EDGERUN_FRONTEND_DIST_ROOT || path.join(defaultFrontendOutRoot, 'site')
const publicRoot = path.join(projectRoot, 'public')
const wikiRoot = process.env.EDGERUN_FRONTEND_WIKI_ROOT || path.join(defaultFrontendOutRoot, 'wiki')

const versionFromTag = (process.env.GITHUB_REF_NAME ?? '').replace(/^v/, '')
const currentVersion = process.env.EDGERUN_VERSION || versionFromTag || 'main'
const buildNumber = process.env.EDGERUN_BUILD_NUMBER || `${currentVersion}-${(process.env.GITHUB_SHA || 'local').slice(0, 8)}-${process.env.GITHUB_RUN_NUMBER || '0'}`
const siteUrl = process.env.EDGERUN_SITE_URL || 'https://www.edgerun.tech'
const siteDomain = process.env.EDGERUN_SITE_DOMAIN || 'www.edgerun.tech'
const apiUrl = process.env.EDGERUN_API_URL || 'https://api.edgerun.tech'
const solanaCluster = process.env.SOLANA_CLUSTER || 'devnet'
const rpcDefaultByCluster: Record<string, string> = {
  localnet: 'http://127.0.0.1:8899',
  devnet: 'https://api.devnet.solana.com',
  testnet: 'https://api.testnet.solana.com',
  'mainnet-beta': 'https://api.mainnet-beta.solana.com'
}
const solanaRpcUrl = process.env.SOLANA_RPC_URL || rpcDefaultByCluster[solanaCluster] || rpcDefaultByCluster.devnet
const treasuryAccount = process.env.EDGERUN_TREASURY_ACCOUNT || ''
const solanaDeployments = JSON.parse(readFileSync(path.join(projectRoot, 'config', 'solana-deployments.json'), 'utf8')) as {
  programs: Record<string, { label: string; programIdByCluster: Record<string, string> }>
}

const requestedVersions = Array.from(new Set((process.env.EDGERUN_VERSIONS || currentVersion).split(',').map((v) => v.trim()).filter(Boolean)))
const versions = requestedVersions.includes('main') ? requestedVersions : ['main', ...requestedVersions]
const docsSources: DocsSource[] = getDocsSources(repoRoot)

const shiki = await createHighlighter({ themes: ['github-dark'], langs: ['plaintext', 'markdown', 'rust', 'toml', 'json', 'yaml', 'bash', 'typescript', 'javascript'] })
const docsRenderer = createDocsRenderer(shiki)

function escapeHtml(value: string): string {
  return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&#39;')
}

function resolveRef(version: string): string {
  if (version === 'main') return 'HEAD'
  const tag = `v${version}`
  try {
    execSync(`git rev-parse --verify --quiet ${tag}`, { cwd: repoRoot, stdio: 'ignore' })
    return tag
  } catch {
    return 'HEAD'
  }
}

function readVersionedFile(ref: string, relativePath: string): string | null {
  if (ref === 'HEAD') {
    const filePath = path.join(repoRoot, relativePath)
    return existsSync(filePath) ? readFileSync(filePath, 'utf8') : null
  }
  try {
    return execSync(`git show ${ref}:${relativePath}`, { cwd: repoRoot, stdio: ['ignore', 'pipe', 'ignore'] }).toString('utf8')
  } catch {
    return null
  }
}

function pageDocument(title: string, description: string, bodyHtml: string): string {
  const marker = renderToString(() => `solid-ssr:${buildNumber}`)
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${escapeHtml(title)} | Edgerun</title>
    <meta name="description" content="${escapeHtml(description)}" />
    <meta name="theme-color" content="#000000" />
    <link rel="stylesheet" href="/fonts/fonts.css" />
    <link rel="stylesheet" href="/assets/styles.css" />
    <link rel="manifest" href="/manifest.webmanifest" />
    <link rel="icon" href="/favicon.ico" sizes="any" />
    <link rel="icon" href="/icon.svg" type="image/svg+xml" />
    <link rel="apple-touch-icon" href="/apple-icon.png" />
  </head>
  <body>
    <div id="edgerun-root">${bodyHtml}</div>
    <script>
      window.__EDGERUN_RPC_CONFIG = ${JSON.stringify({ cluster: solanaCluster, rpcUrl: solanaRpcUrl, treasuryAccount, deployments: solanaDeployments })}
      window.__EDGERUN_API_BASE = ${JSON.stringify(apiUrl)}
    </script>
    <script type="module" src="/assets/client.js"></script>
    <script>window.__EDGERUN_BUILD = ${JSON.stringify({ version: currentVersion, buildNumber, marker })};</script>
  </body>
</html>`
}

function writePage(relativePath: string, title: string, description: string, bodyHtml: string): void {
  const out = path.join(distRoot, relativePath)
  mkdirSync(path.dirname(out), { recursive: true })
  writeFileSync(out, pageDocument(title, description, bodyHtml), 'utf8')
}

function writeComponentPage(relativePath: string, title: string, description: string, component: any): void {
  writePage(relativePath, title, description, renderToString(component))
}

function docsLayout(version: string, heading: string, contentHtml: string, sourceLabel?: string): string {
  const safeHeading = escapeHtml(heading)
  const safeVersion = escapeHtml(version)
  const safeSource = sourceLabel ? `<p class="mt-2 text-xs font-mono text-muted-foreground">${escapeHtml(sourceLabel)}</p>` : ''
  const sidebarHtml = renderToString(() => <DocsSidebar version={version} showSearch />)
  const navHtml = renderToString(() => <Nav />)
  const footerHtml = renderToString(() => <Footer />)
  return `<div class="flex min-h-screen flex-col bg-background text-foreground">
    ${navHtml}
    <main class="flex-1 bg-background">
      <div class="mx-auto grid w-full max-w-7xl gap-6 px-4 py-6 sm:px-6 lg:grid-cols-[240px_minmax(0,1fr)] lg:px-8">
        ${sidebarHtml}
        <section class="rounded-xl border border-border bg-card/40 overflow-hidden">
          <div class="border-b border-border px-5 py-4 sm:px-6">
            <p class="text-xs font-semibold uppercase tracking-wide text-primary">Version ${safeVersion}</p>
            <h1 class="mt-1 text-2xl font-bold sm:text-3xl">${safeHeading}</h1>
            ${safeSource}
          </div>
          <div class="docs-content p-5 sm:p-6">${contentHtml}</div>
        </section>
      </div>
    </main>
    ${footerHtml}
  </div>`
}

function buildChangelogMarkdown(ref: string, version: string): string {
  let lines: string[] = []
  try {
    const raw = execSync(`git log ${ref} -n 40 --date=short --pretty=format:%H%x09%h%x09%ad%x09%s`, {
      cwd: repoRoot,
      stdio: ['ignore', 'pipe', 'ignore']
    }).toString('utf8').trim()
    lines = raw ? raw.split('\n').filter(Boolean) : []
  } catch {
    lines = []
  }

  const docsNav = getDocsNav(version)
  const navPreview = docsNav.slice(0, 4).map((entry) => `- [${entry.label}](${entry.href})`).join('\n')
  const repo = siteLinks.repository || ''
  const header = [
    '# Changelog',
    '',
    `Version: \`${version}\``,
    `Source ref: \`${ref}\``,
    '',
    '## Docs Navigation',
    '',
    navPreview,
    '',
    '## Commits',
    ''
  ]

  if (!lines.length) {
    return [...header, 'No commit history available for this reference.'].join('\n')
  }

  const entries: string[] = []
  for (const line of lines) {
    const [hash = '', shortHash = '', date = '', subject = ''] = line.split('\t')
    if (!hash || !shortHash) continue
    let parent = ''
    try {
      parent = execSync(`git rev-parse ${hash}^`, { cwd: repoRoot, stdio: ['ignore', 'pipe', 'ignore'] }).toString('utf8').trim()
    } catch {
      parent = ''
    }
    const commitUrl = repo ? `${repo}/commit/${hash}` : ''
    const compareUrl = repo && parent ? `${repo}/compare/${parent}...${hash}` : ''
    const links = [commitUrl ? `[commit](${commitUrl})` : '', compareUrl ? `[diff](${compareUrl})` : ''].filter(Boolean).join(' · ')
    const normalizedSubject = docsRenderer.normalizeDocsTerminology(
      subject.replaceAll('vanity', 'address-prefix').replaceAll('Vanity', 'Address-prefix'),
      'changelog'
    )
    entries.push(`- **${date}** \`${shortHash}\` ${normalizedSubject}${links ? ` (${links})` : ''}`)
  }

  return [...header, ...entries].join('\n')
}

type SchedulerHandler = {
  name: string
  args: Array<{ name: string; type: string }>
  returnType: string
  line: number
  docs: string
}

type SchedulerType = {
  name: string
  kind: 'struct' | 'enum' | 'type'
  line: number
  docs: string
}

function lineFromOffset(text: string, offset: number): number {
  return text.slice(0, offset).split('\n').length
}

function unwrapOuterGeneric(typeExpr: string, outer: string): string {
  const trimmed = typeExpr.trim()
  const prefix = `${outer}<`
  if (!trimmed.startsWith(prefix) || !trimmed.endsWith('>')) return ''
  return trimmed.slice(prefix.length, -1).trim()
}

function extractRouteTypesFromHandler(handler: SchedulerHandler): string[] {
  const out: string[] = []
  for (const arg of handler.args) {
    const t = arg.type
    const stateT = unwrapOuterGeneric(t, 'State')
    if (stateT) continue
    const jsonT = unwrapOuterGeneric(t, 'Json')
    if (jsonT) {
      out.push(jsonT)
      continue
    }
    const queryT = unwrapOuterGeneric(t, 'Query')
    if (queryT) {
      out.push(queryT)
      continue
    }
    const pathT = unwrapOuterGeneric(t, 'Path')
    if (pathT) {
      out.push(pathT)
      continue
    }
  }
  const resultJson = handler.returnType.match(/Json<([^>]+)>/)?.[1]?.trim()
  if (resultJson) out.push(resultJson)
  return Array.from(new Set(out))
}

function parseSchedulerSource(source: string): { handlers: Map<string, SchedulerHandler>; types: Map<string, SchedulerType> } {
  const handlers = new Map<string, SchedulerHandler>()
  const types = new Map<string, SchedulerType>()

  for (const match of source.matchAll(/(?:^|\n)((?:\s*\/\/\/[^\n]*\n)*)\s*async fn\s+([A-Za-z0-9_]+)\s*\(([\s\S]*?)\)\s*(?:->\s*([^{]+?))?\s*\{/g)) {
    const docsBlock = match[1] || ''
    const docs = rustDocToText(
      docsBlock
        .split('\n')
        .map((line) => line.trim())
        .filter((line) => line.startsWith('///'))
        .map((line) => line.replace(/^\/\/\/\s?/, ''))
    )
    const name = match[2] || ''
    const args = parseRustArgs(match[3] || '')
    const returnType = (match[4] || '').trim()
    const offset = match.index || 0
    const line = lineFromOffset(source, offset)
    if (!name) continue
    handlers.set(name, { name, args, returnType, line, docs })
  }

  const lines = source.split('\n')
  let pendingDocs: string[] = []
  for (let i = 0; i < lines.length; i += 1) {
    const line = (lines[i] || '').trim()
    const docLine = line.match(/^\/\/\/\s?(.*)$/)?.[1]
    if (docLine !== undefined) {
      pendingDocs.push(docLine)
      continue
    }
    const kind = line.match(/^(?:pub\s+)?(struct|enum|type)\s+([A-Za-z0-9_]+)/)
    if (kind?.[1] && kind[2]) {
      const kindValue = kind[1] as 'struct' | 'enum' | 'type'
      const name = kind[2]
      types.set(name, {
        name,
        kind: kindValue,
        line: i + 1,
        docs: rustDocToText(pendingDocs)
      })
      pendingDocs = []
      continue
    }
    if (!line || line.startsWith('#[')) continue
    pendingDocs = []
  }
  return { handlers, types }
}

function buildSchedulerApiHtml(ref: string): { html: string; searchText: string; wikiMdx: string } {
  const schedulerSource = readVersionedFile(ref, 'crates/edgerun-scheduler/src/main.rs')
  if (!schedulerSource) {
    return {
      html: '<p>Scheduler source unavailable for this version.</p>',
      searchText: 'scheduler api unavailable',
      wikiMdx: '# Scheduler API\n\nSource unavailable for this version.\n'
    }
  }
  const parsed = parseSchedulerSource(schedulerSource)
  const routes: Array<{ method: string; routePath: string; handler: string }> = []
  for (const match of schedulerSource.matchAll(/\.route\("([^"]+)",\s*(get|post)\(([^)]+)\)\)/g)) {
    const routePath = match[1]
    const method = match[2]
    const handler = match[3]
    if (!routePath || !method || !handler) continue
    routes.push({ method: method.toUpperCase(), routePath, handler: handler.trim() })
  }
  routes.sort((a, b) => `${a.method} ${a.routePath}`.localeCompare(`${b.method} ${b.routePath}`))

  const routeRows = routes.map((route) => {
    const handler = parsed.handlers.get(route.handler)
    const sourceUrl = handler ? buildSourceUrl(ref, 'crates/edgerun-scheduler/src/main.rs', handler.line) : ''
    const routeTypes = handler ? extractRouteTypesFromHandler(handler) : []
    const typeChips = routeTypes.length
      ? routeTypes.map((t) => {
        const token = t.replaceAll(' ', '')
        const anchorToken = token.replace(/[<>,:&()[\]]/g, '')
        const knownToken = extractRustTypeTokens(token).find((candidate) => parsed.types.has(candidate))
        const known = knownToken ? parsed.types.get(knownToken) : undefined
        if (known) {
          return `<a class="rounded border border-border bg-muted/20 px-2 py-0.5 hover:bg-muted/40" href="#type-${known.name.toLowerCase()}" title="${escapeHtml(known.docs || `${known.kind} ${known.name}`)}">${escapeHtml(known.name)}</a>`
        }
        return `<span class="rounded border border-border bg-muted/10 px-2 py-0.5" title="${escapeHtml(token)}">${escapeHtml(anchorToken || token)}</span>`
      }).join('')
      : '<span class="text-muted-foreground">none</span>'
    const handlerLabel = handler
      ? `<a class="underline decoration-dotted hover:text-primary" href="#fn-${handler.name.toLowerCase()}" title="${escapeHtml(handler.docs || `Handler ${handler.name}`)}">${escapeHtml(handler.name)}</a>`
      : `<code>${escapeHtml(route.handler)}</code>`
    return `<tr class="border-b border-border/70">
      <td class="px-3 py-2 align-top"><code>${route.method}</code></td>
      <td class="px-3 py-2 align-top"><code>${escapeHtml(route.routePath)}</code></td>
      <td class="px-3 py-2 align-top">
        <div class="flex flex-wrap items-center gap-2">
          ${handlerLabel}
          ${sourceUrl ? `<a class="text-xs underline decoration-dotted hover:text-primary" href="${sourceUrl}" target="_blank" rel="noreferrer">source</a>` : ''}
        </div>
      </td>
      <td class="px-3 py-2 align-top"><div class="flex flex-wrap gap-1 text-xs">${typeChips}</div></td>
    </tr>`
  }).join('\n')

  const handlerCards = Array.from(parsed.handlers.values())
    .sort((a, b) => a.name.localeCompare(b.name))
    .map((handler) => {
      const sourceUrl = buildSourceUrl(ref, 'crates/edgerun-scheduler/src/main.rs', handler.line)
      const args = handler.args.length
        ? handler.args.map((arg) => `<li><code>${escapeHtml(arg.name)}</code>: <code>${escapeHtml(arg.type)}</code></li>`).join('')
        : '<li><span class="text-muted-foreground">No arguments</span></li>'
      return `<article id="fn-${handler.name.toLowerCase()}" class="rounded-lg border border-border bg-card/50 p-4">
        <div class="flex items-center justify-between gap-2">
          <h3 class="text-base font-semibold">${escapeHtml(handler.name)}</h3>
          ${sourceUrl ? `<a class="text-xs underline decoration-dotted hover:text-primary" href="${sourceUrl}" target="_blank" rel="noreferrer">source:L${handler.line}</a>` : ''}
        </div>
        <div class="mt-2 grid gap-3 md:grid-cols-2 text-sm">
          <div><p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Arguments</p><ul class="mt-1 space-y-1">${args}</ul></div>
          <div><p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Returns</p><p class="mt-1"><code>${escapeHtml(handler.returnType || '()')}</code></p></div>
        </div>
        <p class="mt-2 text-sm text-muted-foreground">${escapeHtml(handler.docs || 'No inline doc comment found.')}</p>
      </article>`
    }).join('\n')

  const typeCards = Array.from(parsed.types.values())
    .sort((a, b) => a.name.localeCompare(b.name))
    .map((item) => {
      const sourceUrl = buildSourceUrl(ref, 'crates/edgerun-scheduler/src/main.rs', item.line)
      return `<article id="type-${item.name.toLowerCase()}" class="rounded-lg border border-border bg-card/50 p-4">
        <div class="flex items-center justify-between gap-2">
          <h3 class="text-base font-semibold">${escapeHtml(item.name)}</h3>
          <div class="flex items-center gap-2">
            <span class="rounded border border-border bg-muted/20 px-2 py-0.5 text-xs font-mono">${item.kind}</span>
            ${sourceUrl ? `<a class="text-xs underline decoration-dotted hover:text-primary" href="${sourceUrl}" target="_blank" rel="noreferrer">source:L${item.line}</a>` : ''}
          </div>
        </div>
        <p class="mt-2 text-sm text-muted-foreground">${escapeHtml(item.docs || 'No inline doc comment found.')}</p>
      </article>`
    }).join('\n')

  const searchText = [
    'scheduler api',
    ...routes.map((r) => `${r.method} ${r.routePath} ${r.handler}`),
    ...Array.from(parsed.handlers.values()).map((h) => `${h.name} ${h.returnType} ${h.args.map((arg) => `${arg.name} ${arg.type}`).join(' ')} ${h.docs}`),
    ...Array.from(parsed.types.values()).map((t) => `${t.kind} ${t.name} ${t.docs}`)
  ].join(' ')

  const wikiMdx = [
    '# Scheduler API',
    '',
    `Source ref: \`${ref}\``,
    '',
    'Routes:',
    ...routes.map((r) => `- \`${r.method} ${r.routePath}\` -> \`${r.handler}\``),
    '',
    'Related local types:',
    ...Array.from(parsed.types.values()).sort((a, b) => a.name.localeCompare(b.name)).map((t) => `- \`${t.kind}\` \`${t.name}\``)
  ].join('\n')

  return {
    html: `<section class="space-y-4">
      <div class="rounded-lg border border-border bg-card/50 p-4">
        <p class="text-sm text-muted-foreground">Source ref: <code>${escapeHtml(ref)}</code></p>
      </div>
      <section class="space-y-2">
        <h2 class="text-lg font-semibold">Routes</h2>
        <div class="overflow-x-auto rounded-lg border border-border">
          <table class="min-w-full text-sm">
            <thead class="bg-muted/20 text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr>
                <th class="px-3 py-2">Method</th>
                <th class="px-3 py-2">Path</th>
                <th class="px-3 py-2">Handler</th>
                <th class="px-3 py-2">Related Types</th>
              </tr>
            </thead>
            <tbody>${routeRows}</tbody>
          </table>
        </div>
      </section>
      <section class="space-y-3">
        <h2 class="text-lg font-semibold">Handlers</h2>
        ${handlerCards}
      </section>
      <section class="space-y-3">
        <h2 class="text-lg font-semibold">Types</h2>
        ${typeCards}
      </section>
    </section>`,
    searchText,
    wikiMdx
  }
}

type RustApiFn = {
  name: string
  signature: string
  returnType: string
  args: Array<{ name: string; type: string }>
  docs: string
  line: number
}

type RustApiSymbol = {
  kind: 'struct' | 'enum' | 'trait' | 'type' | 'const'
  name: string
  signature: string
  docs: string
  line: number
}

type RustApiScan = {
  functions: RustApiFn[]
  symbols: RustApiSymbol[]
  typeNames: Set<string>
}

function rustDocToText(docs: string[]): string {
  return docs
    .map((line) => line.trim())
    .filter(Boolean)
    .join(' ')
    .replace(/\s+/g, ' ')
    .trim()
}

function splitTopLevelComma(value: string): string[] {
  const out: string[] = []
  let current = ''
  let angle = 0
  let paren = 0
  let bracket = 0
  for (const ch of value) {
    if (ch === '<') angle += 1
    else if (ch === '>') angle = Math.max(0, angle - 1)
    else if (ch === '(') paren += 1
    else if (ch === ')') paren = Math.max(0, paren - 1)
    else if (ch === '[') bracket += 1
    else if (ch === ']') bracket = Math.max(0, bracket - 1)

    if (ch === ',' && angle === 0 && paren === 0 && bracket === 0) {
      const part = current.trim()
      if (part) out.push(part)
      current = ''
      continue
    }
    current += ch
  }
  const tail = current.trim()
  if (tail) out.push(tail)
  return out
}

function parseRustArgs(rawArgs: string): Array<{ name: string; type: string }> {
  if (!rawArgs.trim()) return []
  const args: Array<{ name: string; type: string }> = []
  for (const part of splitTopLevelComma(rawArgs)) {
    const trimmed = part.trim()
    if (!trimmed || trimmed === 'self' || trimmed === '&self' || trimmed === '&mut self') continue
    const index = trimmed.indexOf(':')
    if (index < 0) continue
    const name = trimmed.slice(0, index).trim()
    const type = trimmed.slice(index + 1).trim()
    if (!name || !type) continue
    args.push({ name, type })
  }
  return args
}

function extractRustTypeTokens(value: string): string[] {
  return Array.from(new Set(value.match(/[A-Z][A-Za-z0-9_]*/g) || []))
}

function buildSourceUrl(ref: string, sourcePath: string, line: number): string {
  if (!siteLinks.repository) return ''
  return `${siteLinks.repository}/blob/${ref}/${sourcePath}#L${line}`
}

function linkifyRustTypeToHtml(value: string, typeNames: Set<string>, typePageHref: string, docsByType: Map<string, string>): string {
  if (!value) return '<code>()</code>'
  let linked = escapeHtml(value)
  for (const token of extractRustTypeTokens(value)) {
    if (!typeNames.has(token)) continue
    const docTitle = docsByType.get(token) || `Type ${token}`
    const anchor = `${typePageHref}#type-${token.toLowerCase()}`
    const link = `<a class="underline decoration-dotted hover:text-primary" href="${anchor}" title="${escapeHtml(docTitle)}">${escapeHtml(token)}</a>`
    linked = linked.replaceAll(token, link)
  }
  return `<code>${linked}</code>`
}

function scanRustApi(source: string): RustApiScan {
  const lines = source.split('\n')
  const functions: RustApiFn[] = []
  const symbols: RustApiSymbol[] = []
  const typeNames = new Set<string>()
  let pendingDocs: string[] = []

  for (let i = 0; i < lines.length; i += 1) {
    const raw = lines[i] || ''
    const line = raw.trim()
    const docLine = line.match(/^\/\/\/\s?(.*)$/)?.[1]
    if (docLine !== undefined) {
      pendingDocs.push(docLine)
      continue
    }
    if (!line) continue
    if (line.startsWith('#[')) continue
    if (!line.startsWith('pub ')) {
      pendingDocs = []
      continue
    }

    const fnMatch = line.match(/^pub\s+(?:async\s+)?fn\s+([A-Za-z0-9_]+)\s*\(([^)]*)\)\s*(?:->\s*(.+?))?\s*(?:\{|where|;|$)/)
    const fnName = fnMatch?.[1]
    if (fnName) {
      const argsRaw = fnMatch?.[2] || ''
      const returnType = (fnMatch?.[3] || '').trim()
      const docs = rustDocToText(pendingDocs)
      functions.push({
        name: fnName,
        signature: line.replace('{', '').trimEnd(),
        returnType,
        args: parseRustArgs(argsRaw),
        docs,
        line: i + 1
      })
      pendingDocs = []
      continue
    }

    const structName = line.match(/^pub\s+struct\s+([A-Za-z0-9_]+)/)?.[1]
    if (structName) {
      symbols.push({ kind: 'struct', name: structName, signature: line.replace('{', '').trimEnd(), docs: rustDocToText(pendingDocs), line: i + 1 })
      typeNames.add(structName)
      pendingDocs = []
      continue
    }

    const enumName = line.match(/^pub\s+enum\s+([A-Za-z0-9_]+)/)?.[1]
    if (enumName) {
      symbols.push({ kind: 'enum', name: enumName, signature: line.replace('{', '').trimEnd(), docs: rustDocToText(pendingDocs), line: i + 1 })
      typeNames.add(enumName)
      pendingDocs = []
      continue
    }

    const traitName = line.match(/^pub\s+trait\s+([A-Za-z0-9_]+)/)?.[1]
    if (traitName) {
      symbols.push({ kind: 'trait', name: traitName, signature: line.replace('{', '').trimEnd(), docs: rustDocToText(pendingDocs), line: i + 1 })
      typeNames.add(traitName)
      pendingDocs = []
      continue
    }

    const constName = line.match(/^pub\s+const\s+([A-Za-z0-9_]+)/)?.[1]
    if (constName) {
      symbols.push({ kind: 'const', name: constName, signature: line.trimEnd(), docs: rustDocToText(pendingDocs), line: i + 1 })
      pendingDocs = []
      continue
    }

    const typeName = line.match(/^pub\s+type\s+([A-Za-z0-9_]+)/)?.[1]
    if (typeName) {
      symbols.push({ kind: 'type', name: typeName, signature: line.trimEnd(), docs: rustDocToText(pendingDocs), line: i + 1 })
      typeNames.add(typeName)
      pendingDocs = []
    }
  }

  functions.sort((a, b) => a.name.localeCompare(b.name))
  symbols.sort((a, b) => a.name.localeCompare(b.name))
  return { functions, symbols, typeNames }
}

function buildRustTypesHtml(ref: string, sourcePath: string, title: string, version: string, apiSlug: string): { html: string; searchText: string } {
  const source = readVersionedFile(ref, sourcePath)
  if (!source) return { html: `<h1>${escapeHtml(title)}</h1><p>Source unavailable for this version.</p>`, searchText: `${title} source unavailable` }
  const scan = scanRustApi(source)
  const apiHref = `/docs/${version}/${apiSlug}.html`
  const typedSymbols = scan.symbols.filter((item) => item.kind !== 'const')
  const functionsByType = new Map<string, Set<string>>()
  for (const fn of scan.functions) {
    for (const token of [...extractRustTypeTokens(fn.returnType), ...fn.args.flatMap((arg) => extractRustTypeTokens(arg.type))]) {
      if (!scan.typeNames.has(token)) continue
      if (!functionsByType.has(token)) functionsByType.set(token, new Set())
      functionsByType.get(token)?.add(fn.name)
    }
  }

  const cards = typedSymbols.length
    ? typedSymbols.map((item) => {
      const id = `type-${item.name.toLowerCase()}`
      const src = buildSourceUrl(ref, sourcePath, item.line)
      const usedBy = Array.from(functionsByType.get(item.name) || []).sort((a, b) => a.localeCompare(b))
      return `<article id="${id}" class="rounded-lg border border-border bg-card/50 p-4">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <h3 class="text-lg font-semibold">${escapeHtml(item.name)}</h3>
          <div class="flex items-center gap-2 text-xs">
            <span class="rounded border border-border bg-muted/30 px-2 py-0.5 font-mono">${escapeHtml(item.kind)}</span>
            ${src ? `<a class="underline decoration-dotted hover:text-primary" href="${src}" target="_blank" rel="noreferrer">source:L${item.line}</a>` : ''}
          </div>
        </div>
        <pre class="mt-2 overflow-x-auto rounded border border-border bg-black/30 p-3 text-xs"><code>${escapeHtml(item.signature)}</code></pre>
        ${item.docs ? `<p class="mt-2 text-sm text-muted-foreground">${escapeHtml(item.docs)}</p>` : '<p class="mt-2 text-sm text-muted-foreground">No inline doc comment found.</p>'}
        <div class="mt-3 text-xs text-muted-foreground">
          Used by:
          ${usedBy.length ? usedBy.map((fnName) => `<a class="ml-2 underline decoration-dotted hover:text-primary" href="${apiHref}#fn-${fnName.toLowerCase()}">${escapeHtml(fnName)}()</a>`).join('') : '<span class="ml-2">none</span>'}
        </div>
      </article>`
    }).join('\n')
    : '<p>No public types detected.</p>'

  const searchText = [
    title,
    ...typedSymbols.map((item) => `${item.kind} ${item.name} ${item.signature} ${item.docs}`),
    ...scan.functions.map((fn) => `${fn.name} ${fn.returnType} ${fn.args.map((arg) => `${arg.name} ${arg.type}`).join(' ')}`)
  ].join(' ')

  return {
    html: `<section class="space-y-4">
      <div class="rounded-lg border border-border bg-card/50 p-4">
        <p class="text-sm text-muted-foreground">Source ref: <code>${escapeHtml(ref)}</code></p>
        <p class="mt-1 text-sm text-muted-foreground">Function details: <a class="underline decoration-dotted hover:text-primary" href="${apiHref}">${escapeHtml(apiSlug)}</a></p>
      </div>
      ${cards}
    </section>`,
    searchText
  }
}

function buildRustApiHtml(ref: string, sourcePath: string, title: string, version: string, slug: string): { html: string; searchText: string } {
  const source = readVersionedFile(ref, sourcePath)
  if (!source) return { html: `<h1>${escapeHtml(title)}</h1><p>Source unavailable for this version.</p>`, searchText: `${title} source unavailable` }
  const scan = scanRustApi(source)
  const typePageHref = `/docs/${version}/${slug}-types.html`
  const docsByType = new Map(scan.symbols.map((s) => [s.name, s.docs] as const))

  const symbolLinks = scan.symbols.length
    ? scan.symbols
      .map((item) => `<a class="rounded border border-border bg-muted/20 px-2 py-1 text-xs hover:bg-muted/40" href="${typePageHref}#type-${item.name.toLowerCase()}" title="${escapeHtml(item.docs || `Jump to ${item.kind} ${item.name}`)}">${escapeHtml(item.kind)} ${escapeHtml(item.name)}</a>`)
      .join('')
    : '<span class="text-sm text-muted-foreground">No public symbols detected.</span>'

  const functionCards = scan.functions.length
    ? scan.functions.map((fn) => {
      const id = `fn-${fn.name.toLowerCase()}`
      const sourceUrl = buildSourceUrl(ref, sourcePath, fn.line)
      const argsHtml = fn.args.length
        ? fn.args.map((arg) => `<li><code>${escapeHtml(arg.name)}</code>: ${linkifyRustTypeToHtml(arg.type, scan.typeNames, typePageHref, docsByType)}</li>`).join('')
        : '<li><span class="text-muted-foreground">No arguments</span></li>'
      return `<article id="${id}" class="rounded-lg border border-border bg-card/50 p-4">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <h3 class="text-lg font-semibold"><a class="hover:text-primary" href="#${id}" title="${escapeHtml(fn.docs || `Function ${fn.name}`)}">${escapeHtml(fn.name)}</a></h3>
          <div class="flex items-center gap-2 text-xs">
            <a class="underline decoration-dotted hover:text-primary" href="${typePageHref}">types</a>
            ${sourceUrl ? `<a class="underline decoration-dotted hover:text-primary" href="${sourceUrl}" target="_blank" rel="noreferrer">source:L${fn.line}</a>` : ''}
          </div>
        </div>
        <pre class="mt-2 overflow-x-auto rounded border border-border bg-black/30 p-3 text-xs"><code>${escapeHtml(fn.signature)}</code></pre>
        <div class="mt-3 grid gap-3 md:grid-cols-2">
          <div>
            <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Arguments</p>
            <ul class="mt-1 space-y-1 text-sm">${argsHtml}</ul>
          </div>
          <div>
            <p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Returns</p>
            <p class="mt-1 text-sm">${linkifyRustTypeToHtml(fn.returnType || '()', scan.typeNames, typePageHref, docsByType)}</p>
          </div>
        </div>
        <p class="mt-3 text-sm text-muted-foreground">${escapeHtml(fn.docs || 'No inline doc comment found.')}</p>
      </article>`
    }).join('\n')
    : '<p>No public functions detected.</p>'

  const searchText = [
    title,
    ...scan.functions.map((fn) => `${fn.name} ${fn.signature} ${fn.returnType} ${fn.docs} ${fn.args.map((arg) => `${arg.name} ${arg.type}`).join(' ')}`),
    ...scan.symbols.map((item) => `${item.kind} ${item.name} ${item.signature} ${item.docs}`)
  ].join(' ')

  return {
    html: `<section class="space-y-4">
      <div class="rounded-lg border border-border bg-card/50 p-4">
        <p class="text-sm text-muted-foreground">Source ref: <code>${escapeHtml(ref)}</code></p>
        <p class="mt-1 text-sm text-muted-foreground">Types page: <a class="underline decoration-dotted hover:text-primary" href="${typePageHref}">${escapeHtml(title)} Types</a></p>
      </div>
      <section class="space-y-2">
        <h2 class="text-lg font-semibold">Public Symbols</h2>
        <div class="flex flex-wrap gap-2">${symbolLinks}</div>
      </section>
      <section class="space-y-3">
        <h2 class="text-lg font-semibold">Functions</h2>
        ${functionCards}
      </section>
    </section>`,
    searchText
  }
}

function buildCliApiMarkdown(ref: string, sourcePath: string, title: string): string {
  const source = readVersionedFile(ref, sourcePath)
  if (!source) return `# ${title}\n\nSource unavailable for this version.`

  const commandMatches = Array.from(source.matchAll(/^\s{4}([A-Z][A-Za-z0-9_]*)\s*\{/gm))
    .map((m) => m[1])
    .filter((value): value is string => Boolean(value))
  const commands = Array.from(new Set(commandMatches)).sort((a, b) => a.localeCompare(b))

  const options: Array<{ longName: string; field: string }> = []
  for (const match of source.matchAll(/#\[arg\(([\s\S]*?)\)\]\s*\n\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*:/g)) {
    const attrs = match[1] || ''
    const field = match[2] || 'unknown'
    const explicitLong = attrs.match(/long\s*=\s*"([^"]+)"/)?.[1]
    const hasLong = /\blong\b/.test(attrs)
    if (!explicitLong && !hasLong) continue
    const longName = explicitLong || field.replaceAll('_', '-')
    options.push({ longName, field })
  }
  const uniqueOptions = Array.from(new Map(options.map((item) => [item.longName, item])).values()).sort((a, b) => a.longName.localeCompare(b.longName))

  return [
    `# ${title}`,
    '',
    `Source ref: \`${ref}\``,
    '',
    '## Commands',
    '',
    ...(commands.length ? commands.map((name) => `- \`${name}\``) : ['- None detected']),
    '',
    '## Long Options',
    '',
    ...(uniqueOptions.length ? uniqueOptions.map((item) => `- \`--${item.longName}\` (field: \`${item.field}\`)`) : ['- None detected'])
  ].join('\n')
}

function readVersionedDoc(ref: string, sourcePath: string): { content: string; resolvedPath: string } | null {
  const candidates = sourcePath.endsWith('.md')
    ? [sourcePath.replace(/\.md$/, '.mdx'), sourcePath]
    : sourcePath.endsWith('.mdx')
      ? [sourcePath, sourcePath.replace(/\.mdx$/, '.md')]
      : [sourcePath]
  for (const candidate of candidates) {
    const content = readVersionedFile(ref, candidate)
    if (content) return { content, resolvedPath: candidate }
  }
  return null
}

function assertDocsNavPaths(version: string, generatedPaths: Set<string>): void {
  const versionPrefix = `/docs/${version}/`
  for (const item of getDocsNav(version)) {
    if (!item.href.startsWith(versionPrefix)) continue
    if (generatedPaths.has(item.href)) continue
    throw new Error(`Broken docs nav link for ${version}: ${item.href}`)
  }
}

function generateVersionDocs(version: string): string[] {
  const ref = resolveRef(version)
  const generated: string[] = [`/docs/${version}/`]
  const generatedSet = new Set(generated)
  const addGeneratedPath = (href: string) => {
    if (generatedSet.has(href)) return
    generatedSet.add(href)
    generated.push(href)
  }
  const writeDocsLeafPage = (slug: string, title: string, description: string, bodyHtml: string) => {
    writePage(path.join('docs', version, `${slug}.html`), title, description, bodyHtml)
    addGeneratedPath(`/docs/${version}/${slug}.html`)
    writePage(path.join('docs', version, slug, 'index.html'), title, description, bodyHtml)
    addGeneratedPath(`/docs/${version}/${slug}/`)
  }
  const links: Array<{ title: string; href: string }> = []
  const searchIndex: Array<{ title: string; href: string; text: string }> = []
  const wikiVersionDir = path.join(wikiRoot, version)
  mkdirSync(wikiVersionDir, { recursive: true })

  for (const source of docsSources) {
    const found = readVersionedDoc(ref, source.sourcePath)
    if (!found) continue
    const slug = source.slug || source.sourcePath.replaceAll(path.sep, '-').replace(/\.mdx?$/, '')
    const pageTitle = source.title || slug
    const href = `/docs/${version}/${slug}.html`
    links.push({ title: pageTitle, href })
    addGeneratedPath(href)
    const normalizedSourceContent = docsRenderer.normalizeDocsTerminology(found.content, found.resolvedPath)
    searchIndex.push({
      title: pageTitle,
      href,
      text: docsRenderer.stripMarkdownForSearch(normalizedSourceContent)
    })

    writeDocsLeafPage(
      slug,
      `${pageTitle} (${version})`,
      `Docs for ${pageTitle} in ${version}`,
      docsLayout(version, pageTitle, docsRenderer.renderDocsContent(found.content, found.resolvedPath), `${(source.sourceLabel || found.resolvedPath).replaceAll('vanity', 'address').replaceAll('Vanity', 'Address')} @ ${ref}`)
    )
    writeFileSync(path.join(wikiVersionDir, `${slug}.mdx`), `${normalizedSourceContent}\n`, 'utf8')
  }

  const schedulerApi = buildSchedulerApiHtml(ref)
  links.push({ title: 'scheduler-api', href: `/docs/${version}/scheduler-api.html` })
  addGeneratedPath(`/docs/${version}/scheduler-api.html`)
  searchIndex.push({
    title: 'scheduler-api',
    href: `/docs/${version}/scheduler-api.html`,
    text: docsRenderer.stripMarkdownForSearch(schedulerApi.searchText)
  })
  writeDocsLeafPage(
    'scheduler-api',
    `scheduler-api (${version})`,
    `Scheduler API snapshot for ${version}`,
    docsLayout(version, 'scheduler-api', schedulerApi.html, `source: crates/edgerun-scheduler/src/main.rs @ ${ref}`)
  )
  writeFileSync(path.join(wikiVersionDir, 'scheduler-api.mdx'), `${schedulerApi.wikiMdx}\n`, 'utf8')

  const changelogMd = buildChangelogMarkdown(ref, version)
  links.push({ title: 'changelog', href: `/docs/${version}/changelog.html` })
  addGeneratedPath(`/docs/${version}/changelog.html`)
  searchIndex.push({
    title: 'changelog',
    href: `/docs/${version}/changelog.html`,
    text: docsRenderer.stripMarkdownForSearch(changelogMd)
  })
  writeDocsLeafPage(
    'changelog',
    `changelog (${version})`,
    `Changelog for ${version}`,
    docsLayout(version, 'changelog', docsRenderer.renderMarkdown(changelogMd), `source: git history @ ${ref}`)
  )
  writeFileSync(path.join(wikiVersionDir, 'changelog.mdx'), `${changelogMd}\n`, 'utf8')

  const apiPages: Array<{ slug: string; title: string; contentHtml: string; searchText: string; wikiMdx: string }> = []
  for (const spec of generatedApiSpecs) {
    if (spec.mode === 'rust') {
      const apiPage = buildRustApiHtml(ref, spec.sourcePath, spec.title, version, spec.slug)
      const typesTitle = `${spec.title} Types`
      const typesSlug = `${spec.slug}-types`
      const typesPage = buildRustTypesHtml(ref, spec.sourcePath, typesTitle, version, spec.slug)
      apiPages.push({
        slug: spec.slug,
        title: spec.title,
        contentHtml: apiPage.html,
        searchText: apiPage.searchText,
        wikiMdx: `# ${spec.title}\n\nThis page is rendered as interactive HTML in site docs.\n\nSearch summary:\n\n${apiPage.searchText}\n`
      })
      apiPages.push({
        slug: typesSlug,
        title: typesTitle,
        contentHtml: typesPage.html,
        searchText: typesPage.searchText,
        wikiMdx: `# ${typesTitle}\n\nThis page is rendered as interactive HTML in site docs.\n\nSearch summary:\n\n${typesPage.searchText}\n`
      })
    } else {
      const cliMarkdown = buildCliApiMarkdown(ref, spec.sourcePath, spec.title)
      apiPages.push({
        slug: spec.slug,
        title: spec.title,
        contentHtml: docsRenderer.renderMarkdown(cliMarkdown),
        searchText: docsRenderer.stripMarkdownForSearch(cliMarkdown),
        wikiMdx: cliMarkdown
      })
    }
  }
  for (const page of apiPages) {
    const href = `/docs/${version}/${page.slug}.html`
    const normalizedSearchText = docsRenderer.normalizeDocsTerminology(page.searchText, page.slug)
    links.push({ title: page.slug, href })
    addGeneratedPath(href)
    searchIndex.push({
      title: page.slug,
      href,
      text: docsRenderer.stripMarkdownForSearch(normalizedSearchText)
    })
    writeDocsLeafPage(
      page.slug,
      `${page.slug} (${version})`,
      `API reference for ${page.slug} (${version})`,
      docsLayout(version, page.slug, page.contentHtml, `source: API snapshot @ ${ref}`)
    )
    writeFileSync(path.join(wikiVersionDir, `${page.slug}.mdx`), `${page.wikiMdx}\n`, 'utf8')
  }

  const apiIndexMarkdown = [
    '# API Reference',
    '',
    ...apiPages.map((page) => `- [${page.title}](/docs/${version}/${page.slug}.html)`),
    '- [Scheduler API (HTTP)](/docs/' + version + '/scheduler-api.html)'
  ].join('\n')
  links.push({ title: 'api-reference', href: `/docs/${version}/api-reference.html` })
  addGeneratedPath(`/docs/${version}/api-reference.html`)
  searchIndex.push({
    title: 'api-reference',
    href: `/docs/${version}/api-reference.html`,
    text: docsRenderer.stripMarkdownForSearch(apiIndexMarkdown)
  })
  writeDocsLeafPage(
    'api-reference',
    `api-reference (${version})`,
    `API references for ${version}`,
    docsLayout(version, 'api-reference', docsRenderer.renderMarkdown(apiIndexMarkdown), `source: docs index @ ${ref}`)
  )
  writeFileSync(path.join(wikiVersionDir, 'api-reference.mdx'), `${apiIndexMarkdown}\n`, 'utf8')

  writePage(
    path.join('docs', version, 'index.html'),
    `Documentation ${version}`,
    `Versioned docs index for ${version}`,
    docsLayout(version, `Documentation ${version}`, `<ul class="space-y-2">${links.map((entry) => `<li><a class="underline decoration-dotted hover:text-primary" href="${entry.href}">${escapeHtml(entry.title)}</a></li>`).join('\n')}</ul>`, `source: version index @ ${ref}`)
  )
  writeFileSync(path.join(distRoot, 'docs', version, 'search-index.json'), `${JSON.stringify(searchIndex, null, 2)}\n`, 'utf8')
  addGeneratedPath(`/docs/${version}/search-index.json`)

  writeFileSync(path.join(wikiVersionDir, 'Home.md'), ['# Edgerun Docs ' + version, '', `Build: \`${buildNumber}\``, `Source ref: \`${ref}\``, '', 'Pages:', ...links.map((entry) => `- [${entry.title}](${entry.href})`)].join('\n') + '\n', 'utf8')
  assertDocsNavPaths(version, generatedSet)
  return generated
}

rmSync(wikiRoot, { recursive: true, force: true })
mkdirSync(path.join(distRoot, 'assets'), { recursive: true })
if (existsSync(publicRoot)) cpSync(publicRoot, distRoot, { recursive: true })

const docsPaths = versions.flatMap((v) => generateVersionDocs(v))
const siteRoutes = getAllSiteRoutes()
for (const route of siteRoutes) {
  writeComponentPage(route.outputPath, route.pageTitle, route.description, route.component)
}

writePage('404.html', 'Page Not Found', 'The requested page is unavailable.', '<div class="min-h-screen bg-background text-foreground"><main class="mx-auto max-w-3xl p-8"><h1 class="text-3xl font-bold">Page Not Found</h1><p class="mt-3 text-muted-foreground">This route is unavailable or still generating.</p><a class="inline-block mt-4 text-primary underline" href="/">Go home</a></main></div>')

const paths = [
  ...siteRoutes.map((route) => route.path),
  ...docsPaths
]

writeFileSync(path.join(distRoot, 'versions.json'), JSON.stringify(versions, null, 2) + '\n', 'utf8')
const llmsScheduler = buildSchedulerApiHtml(resolveRef(currentVersion)).wikiMdx
writeStaticSiteMetadata(
  {
    projectRoot,
    distRoot,
    siteUrl,
    siteDomain,
    currentVersion,
    buildNumber,
    solanaCluster,
    solanaRpcUrl: solanaRpcUrl || '',
    treasuryAccount
  },
  paths,
  llmsScheduler
)
