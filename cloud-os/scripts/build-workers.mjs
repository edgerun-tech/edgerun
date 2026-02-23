import * as esbuild from 'esbuild';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, '..');

const workers = [
  'src/workers/mcp/base.ts',
  'src/workers/mcp/browser-os.ts',
  'src/workers/mcp/github.ts',
  'src/workers/mcp/cloudflare.ts',
  'src/workers/mcp/google.ts',
  'src/workers/mcp/vercel.ts',
  'src/workers/mcp/terminal.ts',
  'src/workers/mcp/frontend-terminal.ts',
  'src/workers/mcp/qwen.ts',
];

const outdir = resolve(rootDir, 'public/workers/mcp');

async function build() {
  console.log('Root dir:', rootDir);
  console.log('Out dir:', outdir);
  
  for (const worker of workers) {
    const name = worker.split('/').pop()?.replace('.ts', '.js') ?? '';
    const inputPath = resolve(rootDir, worker);
    const outputPath = resolve(outdir, name);
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
    console.log(`Built ${name}`);
  }
}

build();
