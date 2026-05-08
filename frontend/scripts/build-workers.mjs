import { build } from "esbuild"
import { copyFile, mkdir } from "node:fs/promises"

await mkdir("public/workers", { recursive: true })

await build({
  entryPoints: ["worker-src/assemblyscript-compiler-worker.ts"],
  outfile: "public/workers/assemblyscript-compiler-worker.js",
  bundle: true,
  format: "esm",
  platform: "browser",
  target: ["es2022"],
  sourcemap: false,
  legalComments: "none",
  mainFields: ["browser", "module", "main"],
  conditions: ["browser", "worker", "import", "default"],
  external: ["fs", "module", "path", "url", "node:module"],
  define: {
    // binaryen ships an Emscripten-style loader that contains both browser and
    // Node paths. Without this, esbuild sees the Node branch and tries to bundle
    // `node:module` into the browser worker.
    process: "undefined",
    global: "globalThis",
  },
})

await mkdir("public/apps/as-compiler-app", { recursive: true })
await copyFile(
  "public/workers/assemblyscript-compiler-worker.js",
  "public/apps/as-compiler-app/assemblyscript-compiler-worker.js",
)

console.log("built public/workers/assemblyscript-compiler-worker.js")
