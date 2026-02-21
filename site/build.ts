import { readFileSync, readdirSync, writeFileSync, mkdirSync, existsSync, copyFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { renderToString } from 'solid-js/web'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const repoRoot = path.resolve(__dirname, '..')
const siteRoot = __dirname
const distRoot = path.join(siteRoot, 'dist')
const wikiRoot = path.join(siteRoot, 'wiki')

const versionFromTag = (process.env.GITHUB_REF_NAME ?? '').replace(/^v/, '')
const version = process.env.EDGERUN_VERSION || versionFromTag || 'main'
const buildNumber =
  process.env.EDGERUN_BUILD_NUMBER ||
  `${version}-${(process.env.GITHUB_SHA || 'local').slice(0, 8)}-${process.env.GITHUB_RUN_NUMBER || '0'}`
const siteUrl = process.env.EDGERUN_SITE_URL || 'https://www.edgerun.tech'
const siteDomain = process.env.EDGERUN_SITE_DOMAIN || ''
const versions = (process.env.EDGERUN_VERSIONS || version)
  .split(',')
  .map((v) => v.trim())
  .filter(Boolean)

const docsSourceFiles = [
  'Whitepaper.md',
  'Phase-2-whitepaper.md',
  ...readdirSync(path.join(repoRoot, 'docs'))
    .filter((f) => f.endsWith('.md'))
    .map((f) => path.join('docs', f))
]

function escapeHtml(input: string) {
  return input
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
}

function pageTemplate(title: string, bodyInnerHtml: string) {
  const solidMarker = renderToString(() => `solid-ssr:${buildNumber}`)
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${escapeHtml(title)} | Edgerun</title>
    <meta name="description" content="Edgerun static site build ${escapeHtml(buildNumber)}" />
    <link rel="stylesheet" href="/assets/styles.css" />
  </head>
  <body>
    <div class="max-w-6xl mx-auto px-4 py-8 md:py-12">
      <header class="mb-8 md:mb-10">
        <p class="kicker">Edgerun Static</p>
        <div class="flex flex-wrap items-center justify-between gap-3 mt-2">
          <h1 class="text-3xl md:text-4xl font-bold">${escapeHtml(title)}</h1>
          <p class="text-xs text-slate-400 font-mono" data-build-meta>build ${escapeHtml(buildNumber)}</p>
        </div>
        <nav class="mt-5 flex flex-wrap gap-2 text-sm">
          <a class="card px-3 py-1.5 hover:text-sky-300" href="/">Home</a>
          <a class="card px-3 py-1.5 hover:text-sky-300" href="/docs/">Docs</a>
          <a class="card px-3 py-1.5 hover:text-sky-300" href="/releases/">Releases</a>
        </nav>
      </header>
      <main>${bodyInnerHtml}</main>
      <footer class="mt-10 text-sm text-slate-400">
        <p>Version: <span class="font-mono">${escapeHtml(version)}</span></p>
        <p>Build: <span class="font-mono">${escapeHtml(buildNumber)}</span></p>
        <p class="text-xs font-mono text-slate-500 mt-2">${escapeHtml(solidMarker)}</p>
      </footer>
    </div>
    <script type="module" src="/assets/client.js"></script>
  </body>
</html>
`
}

function writeHtml(relativePath: string, title: string, content: string) {
  const outPath = path.join(distRoot, relativePath)
  mkdirSync(path.dirname(outPath), { recursive: true })
  writeFileSync(outPath, pageTemplate(title, content), 'utf8')
}

function generateSchedulerApiMarkdown() {
  const schedulerPath = path.join(repoRoot, 'crates/edgerun-scheduler/src/main.rs')
  const src = readFileSync(schedulerPath, 'utf8')
  const routeRegex = /\.route\("([^"]+)",\s*(get|post)\(([^)]+)\)\)/g
  const rows: string[] = []
  for (const m of src.matchAll(routeRegex)) {
    rows.push(`- \`${m[2].toUpperCase()} ${m[1]}\` -> \`${m[3].trim()}\``)
  }
  return ['# Scheduler API (Generated)', '', `Source: \`crates/edgerun-scheduler/src/main.rs\``, '', ...rows.sort()].join(
    '\n'
  )
}

mkdirSync(distRoot, { recursive: true })
mkdirSync(wikiRoot, { recursive: true })

writeHtml(
  'index.html',
  'Edgerun',
  `
  <section class="grid lg:grid-cols-2 gap-4">
    <article class="card p-5">
      <p class="kicker">Architecture</p>
      <h2 class="text-xl font-semibold mt-2">Solid + esbuild + native ESM</h2>
      <p class="mt-2 text-slate-300">Static build uses Solid server rendering, Tailwind CLI, and esbuild.</p>
    </article>
    <article class="card p-5">
      <p class="kicker">Versioning</p>
      <h2 class="text-xl font-semibold mt-2">Build metadata everywhere</h2>
      <p class="mt-2 text-slate-300">Frontend, docs, and release artifacts embed version <code>${escapeHtml(
        version
      )}</code> and build <code>${escapeHtml(buildNumber)}</code>.</p>
    </article>
  </section>`
)

writeHtml(
  'docs/index.html',
  'Documentation',
  `
  <section class="card p-5">
    <p class="kicker">Documentation</p>
    <h2 class="text-2xl font-semibold mt-2">Browse by version</h2>
    <div class="mt-4 flex flex-wrap items-center gap-2">
      <label for="version-select" class="text-sm text-slate-300">Version:</label>
      <select id="version-select" class="card px-2 py-1 bg-transparent">
        ${versions
          .map((v) => `<option value="${escapeHtml(v)}" ${v === version ? 'selected' : ''}>${escapeHtml(v)}</option>`)
          .join('\n')}
      </select>
      <a class="card px-3 py-1.5 hover:text-sky-300" href="/docs/${escapeHtml(version)}/">Open ${escapeHtml(version)}</a>
    </div>
  </section>`
)

mkdirSync(path.join(distRoot, 'docs', version), { recursive: true })
mkdirSync(path.join(wikiRoot, version), { recursive: true })

const docsLinks: Array<{ title: string; href: string }> = []

for (const sourceRel of docsSourceFiles) {
  const sourcePath = path.join(repoRoot, sourceRel)
  if (!existsSync(sourcePath)) continue
  const base = sourceRel.replaceAll(path.sep, '-').replace(/\.md$/, '')
  const content = readFileSync(sourcePath, 'utf8')
  const htmlName = `${base}.html`
  docsLinks.push({ title: base, href: `/docs/${version}/${htmlName}` })
  writeHtml(
    path.join('docs', version, htmlName),
    `${base} (${version})`,
    `<article class="card p-5"><p class="text-xs font-mono text-slate-400 mb-3">${escapeHtml(
      sourceRel
    )}</p><pre class="whitespace-pre-wrap text-sm leading-6 text-slate-100">${escapeHtml(content)}</pre></article>`
  )
  writeFileSync(path.join(wikiRoot, version, `${base}.md`), content, 'utf8')
}

const schedulerMd = generateSchedulerApiMarkdown()
docsLinks.push({ title: 'scheduler-api', href: `/docs/${version}/scheduler-api.html` })
writeHtml(
  path.join('docs', version, 'scheduler-api.html'),
  `scheduler-api (${version})`,
  `<article class="card p-5"><pre class="whitespace-pre-wrap text-sm leading-6 text-slate-100">${escapeHtml(
    schedulerMd
  )}</pre></article>`
)
writeFileSync(path.join(wikiRoot, version, 'Scheduler-API.md'), schedulerMd, 'utf8')

writeHtml(
  path.join('docs', version, 'index.html'),
  `Documentation ${version}`,
  `<section class="card p-5">
      <p class="kicker">Docs ${escapeHtml(version)}</p>
      <ul class="mt-3 space-y-2">
        ${docsLinks
          .map(
            (d) =>
              `<li><a class="hover:text-sky-300 underline decoration-dotted" href="${escapeHtml(d.href)}">${escapeHtml(
                d.title
              )}</a></li>`
          )
          .join('\n')}
      </ul>
    </section>`
)

writeHtml(
  'releases/index.html',
  'Releases',
  `<section class="card p-5">
    <p class="kicker">Release Artifacts</p>
    <p class="mt-2 text-slate-200">Binaries and versioned docs are published from GitHub Actions. Current version: <code>${escapeHtml(
      version
    )}</code>.</p>
    <p class="mt-2 text-slate-400">Download from GitHub Releases and browse docs under <code>/docs/&lt;version&gt;/</code>.</p>
  </section>`
)

writeFileSync(path.join(distRoot, 'build-meta.json'), JSON.stringify({ version, buildNumber, siteUrl }, null, 2))
writeFileSync(path.join(distRoot, 'versions.json'), JSON.stringify(versions, null, 2))

writeFileSync(
  path.join(wikiRoot, version, 'Home.md'),
  [`# Edgerun Docs ${version}`, '', `Build: \`${buildNumber}\``, '', 'Generated pages:', ...docsLinks.map((d) => `- [${d.title}](${d.href})`)].join(
    '\n'
  ),
  'utf8'
)

if (siteDomain) {
  writeFileSync(path.join(distRoot, 'CNAME'), `${siteDomain}\n`, 'utf8')
}

writeFileSync(path.join(distRoot, 'robots.txt'), `User-agent: *\nAllow: /\nSitemap: ${siteUrl}/sitemap.xml\n`, 'utf8')
writeFileSync(
  path.join(distRoot, 'sitemap.xml'),
  `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>${siteUrl}/</loc></url>
  <url><loc>${siteUrl}/docs/</loc></url>
  <url><loc>${siteUrl}/docs/${version}/</loc></url>
  <url><loc>${siteUrl}/releases/</loc></url>
</urlset>
`,
  'utf8'
)

const llmsText = [
  '# Edgerun',
  '',
  '> Deterministic WASM compute with SOL settlement.',
  '',
  `Version: ${version}`,
  `Build: ${buildNumber}`,
  '',
  `Site: ${siteUrl}`,
  `Docs: ${siteUrl}/docs/${version}/`,
  `Releases: ${siteUrl}/releases/`
].join('\n')
writeFileSync(path.join(distRoot, 'llms.txt'), llmsText, 'utf8')
writeFileSync(path.join(distRoot, 'llms-full.txt'), `${llmsText}\n\n${schedulerMd}\n`, 'utf8')

const brandMark = path.join(repoRoot, 'frontend/public/brand/edgerun-mark.svg')
if (existsSync(brandMark)) {
  mkdirSync(path.join(distRoot, 'assets', 'brand'), { recursive: true })
  copyFileSync(brandMark, path.join(distRoot, 'assets', 'brand', 'edgerun-mark.svg'))
}

console.log(`Generated static site dist at ${distRoot}`)
console.log(`Generated wiki docs at ${wikiRoot}`)

