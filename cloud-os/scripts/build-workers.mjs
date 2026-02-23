import * as esbuild from 'esbuild';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, '..');

const workers = [
  { entry: 'src/workers/mcp/base.ts', out: 'base.js' },
  { entry: 'src/workers/mcp/browser-os.ts', out: 'browser-os.js' },
  { entry: 'src/workers/mcp/github.ts', out: 'github.js' },
  { entry: 'src/workers/mcp/cloudflare.ts', out: 'cloudflare.js' },
  { entry: 'src/workers/mcp/google.ts', out: 'google.js' },
  { entry: 'src/workers/mcp/vercel.ts', out: 'vercel.js' },
  { entry: 'src/workers/mcp/frontend-terminal.ts', out: 'terminal.js' },
  { entry: 'src/workers/mcp/qwen.ts', out: 'qwen.js' },
];

const outdir = resolve(rootDir, 'public/workers/mcp');

async function build() {
  console.log('Root dir:', rootDir);
  console.log('Out dir:', outdir);
  
  for (const worker of workers) {
    const inputPath = resolve(rootDir, worker.entry);
    const outputPath = resolve(outdir, worker.out);
    console.log(`Building ${inputPath} -> ${outputPath}`);
    await esbuild.build({
      entryPoints: [inputPath],
      bundle: true,
      outfile: outputPath,
      format: 'esm',
      target: 'es2020',
      minify: false,
      sourcemap: true,
      external: [],
    });
    console.log(`Built ${worker.out}`);
  }
}

build();
