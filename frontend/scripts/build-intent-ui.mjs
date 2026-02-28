// SPDX-License-Identifier: Apache-2.0
import { copyFileSync, existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { build } from 'esbuild'
import { solidPlugin } from 'esbuild-plugin-solid'

const frontendRoot = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..')
const intentRoot = path.join(frontendRoot, 'intent-ui')
const intentEntry = path.join(intentRoot, 'src', 'main.jsx')
const intentCssEntry = path.join(intentRoot, 'src', 'app.css')
const targetRoot = path.join(frontendRoot, 'public', 'intent-ui')
const targetClientRoot = path.join(targetRoot, 'client')
const targetWorkerRoot = path.join(targetRoot, 'workers')
const targetCss = path.join(targetRoot, 'app.css')
const eventBusWasmTarget = path.join(targetRoot, 'eventbus.wasm')
const eventBusWorkerSource = path.join(intentRoot, 'src', 'workers', 'eventbus.worker.js')
const eventBusWorkerTarget = path.join(targetWorkerRoot, 'eventbus.worker.js')
const tailwindBin = path.join(frontendRoot, 'node_modules', '.bin', 'tailwindcss')
const repoRoot = path.resolve(frontendRoot, '..')

function run(cmd, cwd) {
  const proc = spawnSync(cmd[0], cmd.slice(1), {
    cwd,
    stdio: 'inherit'
  })
  if (proc.status !== 0) {
    throw new Error(`Command failed (${proc.status ?? 'unknown'}): ${cmd.join(' ')}`)
  }
}

if (!existsSync(intentRoot)) {
  throw new Error(`Missing Intent UI source at ${intentRoot}`)
}
if (!existsSync(intentEntry)) {
  throw new Error(`Missing Intent UI entrypoint at ${intentEntry}`)
}
if (!existsSync(intentCssEntry)) {
  throw new Error(`Missing Intent UI CSS entrypoint at ${intentCssEntry}`)
}
if (!existsSync(tailwindBin)) {
  throw new Error(`Missing tailwind binary at ${tailwindBin}; run 'cd frontend && bun install'`)
}

rmSync(targetRoot, { recursive: true, force: true })
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
  define: {
    'process.env.NODE_ENV': '"production"'
  },
  jsx: 'preserve',
  plugins: [solidPlugin({ solid: { generate: 'dom', hydratable: true } })]
})

run(
  [
    tailwindBin,
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
const eventBusWasmArtifact = path.join(
  cargoTargetDir,
  'wasm32-unknown-unknown',
  'release',
  'edgerun_event_bus_wasm.wasm'
)
run(
  [
    'cargo',
    'build',
    '--release',
    '--target',
    'wasm32-unknown-unknown',
    '--package',
    'edgerun-event-bus-wasm'
  ],
  repoRoot
)
if (!existsSync(eventBusWasmArtifact)) {
  throw new Error(`Missing event bus wasm artifact at ${eventBusWasmArtifact}`)
}
copyFileSync(eventBusWasmArtifact, eventBusWasmTarget)
if (!existsSync(eventBusWorkerSource)) {
  throw new Error(`Missing event bus worker source at ${eventBusWorkerSource}`)
}
copyFileSync(eventBusWorkerSource, eventBusWorkerTarget)

writeFileSync(
  path.join(targetRoot, 'index.html'),
  `<!doctype html>
<html lang="en" data-kb-theme="dark">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Intent UI</title>
    <link rel="stylesheet" href="/intent-ui/app.css" />
  </head>
  <body>
    <main id="root"></main>
    <script type="module" src="/intent-ui/client/main.js"></script>
  </body>
</html>
`,
  'utf8'
)
