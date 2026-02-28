// SPDX-License-Identifier: Apache-2.0
import { copyFileSync, cpSync, existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { build } from 'esbuild'
import { solidPlugin } from 'esbuild-plugin-solid'

const frontendRoot = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..')
const repoRoot = path.resolve(frontendRoot, '..')
const intentRoot = path.join(frontendRoot, 'intent-ui')
const intentEntry = path.join(intentRoot, 'src', 'main.jsx')
const intentCssEntry = path.join(intentRoot, 'src', 'app.css')
const outputRoot = process.env.INTENT_UI_DEV_OUT_DIR
  ? path.resolve(process.env.INTENT_UI_DEV_OUT_DIR)
  : path.join(repoRoot, 'out', 'frontend', 'osdev')
const staticSourceRoot = path.join(frontendRoot, 'public')
const targetRoot = path.join(outputRoot, 'intent-ui')
const targetClientRoot = path.join(targetRoot, 'client')
const targetWorkerRoot = path.join(targetRoot, 'workers')
const targetCss = path.join(targetRoot, 'app.css')
const targetIndex = path.join(targetRoot, 'index.html')
const rootIndex = path.join(outputRoot, 'index.html')
const eventBusWasmTarget = path.join(targetRoot, 'eventbus.wasm')
const eventBusWorkerSource = path.join(intentRoot, 'src', 'workers', 'eventbus.worker.js')
const eventBusWorkerTarget = path.join(targetWorkerRoot, 'eventbus.worker.js')
const integrationsWorkerSource = path.join(intentRoot, 'src', 'workers', 'integrations.worker.js')
const integrationsWorkerTarget = path.join(targetWorkerRoot, 'integrations.worker.js')
const buildVersion = Date.now().toString()

function run(cmd, cwd) {
  const proc = spawnSync(cmd[0], cmd.slice(1), { cwd, stdio: 'inherit' })
  if (proc.status !== 0) {
    throw new Error(`Command failed (${proc.status ?? 'unknown'}): ${cmd.join(' ')}`)
  }
}

if (!existsSync(intentRoot)) throw new Error(`Missing Intent UI source at ${intentRoot}`)
if (!existsSync(intentEntry)) throw new Error(`Missing Intent UI entrypoint at ${intentEntry}`)
if (!existsSync(intentCssEntry)) throw new Error(`Missing Intent UI CSS entrypoint at ${intentCssEntry}`)
rmSync(outputRoot, { recursive: true, force: true })
mkdirSync(outputRoot, { recursive: true })
if (existsSync(staticSourceRoot)) {
  cpSync(staticSourceRoot, outputRoot, { recursive: true })
}
mkdirSync(targetClientRoot, { recursive: true })
mkdirSync(targetWorkerRoot, { recursive: true })

await build({
  entryPoints: { main: intentEntry },
  outdir: targetClientRoot,
  entryNames: '[name]',
  chunkNames: 'chunks/[name]-[hash]',
  bundle: true,
  splitting: true,
  minify: true,
  legalComments: 'none',
  platform: 'browser',
  format: 'esm',
  target: 'es2022',
  define: { 'process.env.NODE_ENV': '"production"' },
  jsx: 'preserve',
  plugins: [solidPlugin({ solid: { generate: 'dom', hydratable: true } })]
})

run(
  [
    'bunx',
    '--yes',
    '@tailwindcss/cli',
    '-i',
    intentCssEntry,
    '-o',
    targetCss,
    '--minify',
    '--config',
    path.join(intentRoot, 'tailwind.config.cjs')
  ],
  frontendRoot
)

const cargoTargetDir = process.env.CARGO_TARGET_DIR || path.join(repoRoot, 'target')
const wasmFromCargo = path.join(
  cargoTargetDir,
  'wasm32-unknown-unknown',
  'release',
  'edgerun_event_bus_wasm.wasm'
)
if (existsSync(wasmFromCargo)) {
  copyFileSync(wasmFromCargo, eventBusWasmTarget)
} else {
  console.warn(`[intent-ui-dev] missing ${eventBusWasmTarget}; eventbus wasm may be unavailable`)
}

copyFileSync(eventBusWorkerSource, eventBusWorkerTarget)
copyFileSync(integrationsWorkerSource, integrationsWorkerTarget)

const intentHtml = `<!doctype html>
<html lang="en" data-kb-theme="dark">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Intent UI</title>
    <link rel="stylesheet" href="/intent-ui/app.css?v=${buildVersion}" />
  </head>
  <body>
    <main id="root"></main>
    <script type="module" src="/intent-ui/client/main.js?v=${buildVersion}"></script>
  </body>
</html>
`

writeFileSync(targetIndex, intentHtml, 'utf8')
writeFileSync(rootIndex, intentHtml, 'utf8')
