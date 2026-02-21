import path from 'node:path'
import { mkdirSync } from 'node:fs'
import { build } from 'esbuild'
import { solidPlugin } from 'esbuild-plugin-solid'

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..')
const repoRoot = path.resolve(root, '..')
const outRoot = process.env.EDGERUN_FRONTEND_OUT_ROOT || path.join(repoRoot, 'out', 'frontend')
const distRoot = process.env.EDGERUN_FRONTEND_DIST_ROOT || path.join(outRoot, 'site')
mkdirSync(path.join(distRoot, 'assets'), { recursive: true })

await build({
  entryPoints: [path.join(root, 'src/client.tsx')],
  outfile: path.join(distRoot, 'assets', 'client.js'),
  bundle: true,
  platform: 'browser',
  format: 'esm',
  target: 'es2022',
  jsx: 'preserve',
  plugins: [solidPlugin({ solid: { generate: 'dom', hydratable: true } })]
})
