import { readFileSync, readdirSync, writeFileSync, mkdirSync, existsSync, copyFileSync } from 'node:fs'
import { execSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { renderToString } from 'solid-js/web'
import MarkdownIt from 'markdown-it'
import { createHighlighter } from 'shiki'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const repoRoot = path.resolve(__dirname, '..')
const distRoot = path.join(__dirname, 'dist')
const wikiRoot = path.join(__dirname, 'wiki')

const versionFromTag = (process.env.GITHUB_REF_NAME ?? '').replace(/^v/, '')
const currentVersion = process.env.EDGERUN_VERSION || versionFromTag || 'main'
const buildNumber =
  process.env.EDGERUN_BUILD_NUMBER ||
  `${currentVersion}-${(process.env.GITHUB_SHA || 'local').slice(0, 8)}-${process.env.GITHUB_RUN_NUMBER || '0'}`
const siteUrl = process.env.EDGERUN_SITE_URL || 'https://www.edgerun.tech'
const siteDomain = process.env.EDGERUN_SITE_DOMAIN || ''

const versions = Array.from(
  new Set(
    (process.env.EDGERUN_VERSIONS || currentVersion)
      .split(',')
      .map((v) => v.trim())
      .filter(Boolean)
  )
)

const docsSourceFiles = [
  'Whitepaper.md',
  'Phase-2-whitepaper.md',
  ...readdirSync(path.join(repoRoot, 'docs'))
    .filter((f) => f.endsWith('.md'))
    .map((f) => path.join('docs', f))
]

const blogPosts = [
  {
    slug: 'introducing-edgerun',
    title: 'Introducing Edgerun: Verifiable Compute for the Decentralized Web',
    excerpt: 'Deterministic WASM compute with cryptographic proof settlement on Solana.'
  },
  {
    slug: 'deterministic-wasm-execution',
    title: 'Understanding Deterministic WASM Execution',
    excerpt: 'How deterministic execution and quorum make distributed compute auditable.'
  },
  {
    slug: 'solana-settlement-layer',
    title: 'Why Solana for Settlement',
    excerpt: 'Settlement guarantees, performance, and operating assumptions on Solana.'
  }
]

const highlighter = await createHighlighter({
  themes: ['github-dark'],
  langs: ['plaintext', 'markdown', 'bash', 'rust', 'toml', 'json', 'yaml', 'typescript', 'javascript']
})

const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  typographer: true,
  highlight: (code, lang) => {
    const normalizedLang = (lang || 'plaintext').toLowerCase()
    try {
      return highlighter.codeToHtml(code, {
        lang: normalizedLang,
        theme: 'github-dark'
      })
    } catch {
      return `<pre class="shiki"><code>${escapeHtml(code)}</code></pre>`
    }
  }
})

function escapeHtml(input: string) {
  return input
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
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

function readVersionedFile(ref: string, relPath: string): string | null {
  if (ref === 'HEAD') {
    const p = path.join(repoRoot, relPath)
    return existsSync(p) ? readFileSync(p, 'utf8') : null
  }
  try {
    return execSync(`git show ${ref}:${relPath}`, {
      cwd: repoRoot,
      stdio: ['ignore', 'pipe', 'ignore']
    }).toString('utf8')
  } catch {
    return null
  }
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
          <a class="card px-3 py-1.5 hover:text-sky-300" href="/run/">Run</a>
          <a class="card px-3 py-1.5 hover:text-sky-300" href="/workers/">Workers</a>
          <a class="card px-3 py-1.5 hover:text-sky-300" href="/token/">Economics</a>
          <a class="card px-3 py-1.5 hover:text-sky-300" href="/docs/">Docs</a>
          <a class="card px-3 py-1.5 hover:text-sky-300" href="/blog/">Blog</a>
          <a class="card px-3 py-1.5 hover:text-sky-300" href="/releases/">Releases</a>
        </nav>
      </header>
      <main>${bodyInnerHtml}</main>
      <footer class="mt-10 text-sm text-slate-400">
        <p>Version: <span class="font-mono">${escapeHtml(currentVersion)}</span></p>
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

function generateSchedulerApiMarkdown(ref: string) {
  const schedulerSource = readVersionedFile(ref, 'crates/edgerun-scheduler/src/main.rs')
  if (!schedulerSource) {
    return '# Scheduler API (Generated)\n\nScheduler source unavailable for this version.\n'
  }
  const routeRegex = /\.route\("([^"]+)",\s*(get|post)\(([^)]+)\)\)/g
  const rows: string[] = []
  for (const m of schedulerSource.matchAll(routeRegex)) {
    rows.push(`- \`${m[2].toUpperCase()} ${m[1]}\` -> \`${m[3].trim()}\``)
  }
  return ['# Scheduler API (Generated)', '', `Source ref: \`${ref}\``, '', ...rows.sort()].join('\n')
}

function generateVersionDocs(version: string) {
  const ref = resolveRef(version)
  const docsDir = path.join(distRoot, 'docs', version)
  const wikiDir = path.join(wikiRoot, version)
  mkdirSync(docsDir, { recursive: true })
  mkdirSync(wikiDir, { recursive: true })

  const docsLinks: Array<{ title: string; href: string }> = []
  const generatedPaths: string[] = [`/docs/${version}/`]

  for (const sourceRel of docsSourceFiles) {
    const content = readVersionedFile(ref, sourceRel)
    if (!content) continue
    const base = sourceRel.replaceAll(path.sep, '-').replace(/\.md$/, '')
    const htmlName = `${base}.html`
    docsLinks.push({ title: base, href: `/docs/${version}/${htmlName}` })
    generatedPaths.push(`/docs/${version}/${htmlName}`)

    const rendered = markdown.render(content)

    writeHtml(
      path.join('docs', version, htmlName),
      `${base} (${version})`,
      `<article class="card p-5"><p class="text-xs font-mono text-slate-400 mb-3">${escapeHtml(
        `${sourceRel} @ ${ref}`
      )}</p><div class="docs-content mt-4">${rendered}</div></article>`
    )

    writeFileSync(path.join(wikiDir, `${base}.md`), content, 'utf8')
  }

  const schedulerMd = generateSchedulerApiMarkdown(ref)
  docsLinks.push({ title: 'scheduler-api', href: `/docs/${version}/scheduler-api.html` })
  generatedPaths.push(`/docs/${version}/scheduler-api.html`)

  writeHtml(
    path.join('docs', version, 'scheduler-api.html'),
    `scheduler-api (${version})`,
    `<article class="card p-5"><div class="docs-content">${markdown.render(schedulerMd)}</div></article>`
  )
  writeFileSync(path.join(wikiDir, 'Scheduler-API.md'), schedulerMd, 'utf8')

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

  writeFileSync(
    path.join(wikiDir, 'Home.md'),
    [
      `# Edgerun Docs ${version}`,
      '',
      `Build: \`${buildNumber}\``,
      '',
      `Source ref: \`${ref}\``,
      '',
      'Generated pages:',
      ...docsLinks.map((d) => `- [${d.title}](${d.href})`)
    ].join('\n'),
    'utf8'
  )

  return generatedPaths
}

mkdirSync(distRoot, { recursive: true })
mkdirSync(wikiRoot, { recursive: true })

writeHtml(
  'index.html',
  'Edgerun',
  `<section class="grid lg:grid-cols-2 gap-4">
    <article class="card p-5">
      <p class="kicker">Architecture</p>
      <h2 class="text-xl font-semibold mt-2">Solid + esbuild + native ESM</h2>
      <p class="mt-2 text-slate-300">Static build uses Solid server rendering, Tailwind CLI, and esbuild.</p>
    </article>
    <article class="card p-5">
      <p class="kicker">Versioning</p>
      <h2 class="text-xl font-semibold mt-2">Build metadata everywhere</h2>
      <p class="mt-2 text-slate-300">Frontend, docs, and release artifacts embed version <code>${escapeHtml(
        currentVersion
      )}</code> and build <code>${escapeHtml(buildNumber)}</code>.</p>
    </article>
  </section>`
)

writeHtml(
  'run/index.html',
  'Run Job',
  `<section class="card p-5">
    <p class="kicker">Compute</p>
    <h2 class="text-2xl font-semibold mt-2">Submit a deterministic WASM job</h2>
    <p class="mt-3 text-slate-300">Use the CLI for production submissions. This static page mirrors the product flow with SOL-denominated limits and escrow.</p>
    <pre class="mt-4 whitespace-pre-wrap text-sm leading-6 text-slate-100">edgerun version\nedgerun run replay-corpus\n# scheduler endpoint configured in runtime env</pre>
  </section>`
)

writeHtml(
  'workers/index.html',
  'Workers',
  `<section class="card p-5">
    <p class="kicker">Network</p>
    <h2 class="text-2xl font-semibold mt-2">Worker operations</h2>
    <ul class="mt-3 space-y-2 text-slate-300 list-disc pl-5">
      <li>Stake SOL for participation and slash-resistance.</li>
      <li>Publish deterministic output attestations.</li>
      <li>Track uptime/SLA and failure recovery.</li>
    </ul>
  </section>`
)

writeHtml(
  'token/index.html',
  'SOL Economics',
  `<section class="card p-5">
    <p class="kicker">Economics</p>
    <h2 class="text-2xl font-semibold mt-2">SOL-only settlement</h2>
    <p class="mt-3 text-slate-300">Escrow, worker stake, rewards, and slashing are all SOL-denominated in the current protocol.</p>
    <blockquote class="mt-4 p-4 card text-slate-200">"I think compute will be the currency of the future..."</blockquote>
    <p class="text-sm mt-2 text-slate-400">Source: Sam Altman on Lex Fridman podcast.</p>
  </section>`
)

writeHtml(
  'dashboard/index.html',
  'Dashboard',
  `<section class="card p-5">
    <p class="kicker">Operations</p>
    <h2 class="text-2xl font-semibold mt-2">Status overview</h2>
    <p class="mt-3 text-slate-300">Track job throughput, worker health, and settlement latency by release build.</p>
    <p class="mt-3 text-slate-400">For dynamic status, use scheduler APIs and CLI in runtime environments.</p>
  </section>`
)

writeHtml(
  'blog/index.html',
  'Blog',
  `<section class="card p-5">
    <p class="kicker">Updates</p>
    <div class="mt-3 space-y-4">
      ${blogPosts
        .map(
          (p) => `<article class="card p-4"><h3 class="text-lg font-semibold"><a class="hover:text-sky-300" href="/blog/${p.slug}/">${escapeHtml(
            p.title
          )}</a></h3><p class="text-slate-300 mt-1">${escapeHtml(p.excerpt)}</p></article>`
        )
        .join('\n')}
    </div>
  </section>`
)

for (const post of blogPosts) {
  writeHtml(
    path.join('blog', post.slug, 'index.html'),
    post.title,
    `<article class="card p-5"><p class="text-slate-300">${escapeHtml(post.excerpt)}</p><p class="mt-4 text-slate-400">This static article is generated per release for long-term version browseability.</p></article>`
  )
}

writeHtml('legal/privacy/index.html', 'Privacy Policy', '<section class="card p-5"><p>Privacy terms for Edgerun services.</p></section>')
writeHtml('legal/terms/index.html', 'Terms of Service', '<section class="card p-5"><p>Terms governing Edgerun usage and settlement on Solana.</p></section>')
writeHtml('legal/sla/index.html', 'Service Level Agreement', '<section class="card p-5"><p>Operational targets and exclusions for scheduler and worker services.</p></section>')

writeHtml(
  'docs/index.html',
  'Documentation',
  `<section class="card p-5">
    <p class="kicker">Documentation</p>
    <h2 class="text-2xl font-semibold mt-2">Browse by version</h2>
    <div class="mt-4 flex flex-wrap items-center gap-2">
      <label for="version-select" class="text-sm text-slate-300">Version:</label>
      <select id="version-select" class="card px-2 py-1 bg-transparent">
        ${versions
          .map((v) => `<option value="${escapeHtml(v)}" ${v === currentVersion ? 'selected' : ''}>${escapeHtml(v)}</option>`)
          .join('\n')}
      </select>
      <a class="card px-3 py-1.5 hover:text-sky-300" href="/docs/${escapeHtml(currentVersion)}/">Open ${escapeHtml(currentVersion)}</a>
    </div>
  </section>`
)

const versionedDocPaths = versions.flatMap((v) => generateVersionDocs(v))

writeHtml(
  'releases/index.html',
  'Releases',
  `<section class="card p-5">
    <p class="kicker">Release Artifacts</p>
    <p class="mt-2 text-slate-200">Binaries and versioned docs are published from GitHub Actions.</p>
    <ul class="mt-3 space-y-1 text-slate-300 list-disc pl-5">${versions
      .map((v) => `<li><a class="hover:text-sky-300" href="/docs/${escapeHtml(v)}/">Docs ${escapeHtml(v)}</a></li>`)
      .join('')}</ul>
  </section>`
)

writeHtml(
  '404.html',
  'Page Not Available Yet',
  '<section class="card p-5"><p>This route is generating or moved. Use top navigation to continue.</p></section>'
)

writeFileSync(path.join(distRoot, 'build-meta.json'), JSON.stringify({ version: currentVersion, buildNumber, siteUrl }, null, 2))
writeFileSync(path.join(distRoot, 'versions.json'), JSON.stringify(versions, null, 2))

if (siteDomain) {
  writeFileSync(path.join(distRoot, 'CNAME'), `${siteDomain}\n`, 'utf8')
}

const sitePaths = [
  '/',
  '/run/',
  '/workers/',
  '/token/',
  '/dashboard/',
  '/blog/',
  ...blogPosts.map((p) => `/blog/${p.slug}/`),
  '/docs/',
  ...versionedDocPaths,
  '/releases/',
  '/legal/privacy/',
  '/legal/terms/',
  '/legal/sla/'
]

writeFileSync(path.join(distRoot, 'robots.txt'), `User-agent: *\nAllow: /\nSitemap: ${siteUrl}/sitemap.xml\n`, 'utf8')
writeFileSync(
  path.join(distRoot, 'sitemap.xml'),
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${sitePaths
    .map((p) => `  <url><loc>${siteUrl}${p}</loc></url>`)
    .join('\n')}\n</urlset>\n`,
  'utf8'
)

const schedulerMdCurrent = generateSchedulerApiMarkdown(resolveRef(currentVersion))
const llmsText = [
  '# Edgerun',
  '',
  '> Deterministic WASM compute with SOL settlement.',
  '',
  `Version: ${currentVersion}`,
  `Build: ${buildNumber}`,
  '',
  `Site: ${siteUrl}`,
  `Docs: ${siteUrl}/docs/${currentVersion}/`,
  `Releases: ${siteUrl}/releases/`
].join('\n')
writeFileSync(path.join(distRoot, 'llms.txt'), llmsText, 'utf8')
writeFileSync(path.join(distRoot, 'llms-full.txt'), `${llmsText}\n\n${schedulerMdCurrent}\n`, 'utf8')

const brandMark = path.join(repoRoot, 'frontend/public/brand/edgerun-mark.svg')
if (existsSync(brandMark)) {
  mkdirSync(path.join(distRoot, 'assets', 'brand'), { recursive: true })
  copyFileSync(brandMark, path.join(distRoot, 'assets', 'brand', 'edgerun-mark.svg'))
}

console.log(`Generated static site dist at ${distRoot}`)
console.log(`Generated wiki docs at ${wikiRoot}`)
