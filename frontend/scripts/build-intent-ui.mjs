// SPDX-License-Identifier: Apache-2.0
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
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
const targetCss = path.join(targetRoot, 'app.css')
const tailwindBin = path.join(frontendRoot, 'node_modules', '.bin', 'tailwindcss')

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
