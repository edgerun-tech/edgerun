// SPDX-License-Identifier: Apache-2.0
import { mkdirSync } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { build } from 'esbuild'
import { solidPlugin } from 'esbuild-plugin-solid'

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..')
const repoRoot = path.resolve(root, '..')
const outRoot = process.env.EDGERUN_FRONTEND_OUT_ROOT || path.join(repoRoot, 'out', 'frontend')
const distRoot = process.env.EDGERUN_FRONTEND_DIST_ROOT || path.join(outRoot, 'site')
const wikiRoot = process.env.EDGERUN_FRONTEND_WIKI_ROOT || path.join(outRoot, 'wiki')
const tmpRoot = process.env.EDGERUN_FRONTEND_TMP_ROOT || path.join(outRoot, 'tmp')
const outfile = path.join(tmpRoot, 'build.mjs')
mkdirSync(path.dirname(outfile), { recursive: true })
process.env.EDGERUN_FRONTEND_ROOT = root
process.env.EDGERUN_FRONTEND_DIST_ROOT = distRoot
process.env.EDGERUN_FRONTEND_WIKI_ROOT = wikiRoot

await build({
  entryPoints: [path.join(root, 'build.tsx')],
  outfile,
  bundle: true,
  platform: 'node',
  format: 'esm',
  target: 'node20',
  jsx: 'preserve',
  plugins: [solidPlugin({ solid: { generate: 'ssr', hydratable: true } })]
})

await import(pathToFileURL(outfile).href)
