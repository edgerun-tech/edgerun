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
  entryPoints: {
    client: path.join(root, 'src/client.tsx'),
    'workers/browser-worker-runtime.worker': path.join(root, 'lib/browser-worker-runtime.worker.ts'),
    'workers/thread-benchmark.worker': path.join(root, 'lib/thread-benchmark.worker.ts')
  },
  outdir: path.join(distRoot, 'assets'),
  entryNames: '[name]',
  bundle: true,
  splitting: true,
  chunkNames: 'chunks/[name]-[hash]',
  minify: true,
  treeShaking: true,
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
