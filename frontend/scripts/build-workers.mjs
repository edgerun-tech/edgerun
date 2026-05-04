import { build } from "esbuild"
import { mkdir } from "node:fs/promises"

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
  define: {
    "process.env.NODE_ENV": "\"production\"",
  },
  external: [
    "fs",
    "module",
    "path",
    "url",
    "process",
  ],
})

console.log("built public/workers/assemblyscript-compiler-worker.js")
