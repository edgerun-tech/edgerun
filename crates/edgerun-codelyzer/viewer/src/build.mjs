/**
 * Simple bundler using Bun's native build API.
 * Expects pre-transformed JSX files in viewer/.build/ via prebuild.mjs.
 */
import * as BunModule from "bun";
import fs from "node:fs";
import path from "node:path";

const isWatch = process.argv.includes("--watch");
const ROOT = path.resolve(import.meta.dir, "../..");
const BUILD_DIR = path.join(ROOT, "viewer/.build");

async function build() {
  // Pre-transform JSX with Babel Solid preset
  await Bun.spawn(["bun", "run", "viewer/src/prebuild.mjs"]).exited;

  const result = await BunModule.build({
    entrypoints: [path.join(BUILD_DIR, "main.jsx")],
    outdir: path.join(ROOT, "viewer/dist"),
    target: "browser",
    format: "esm",
    minify: !isWatch,
    sourcemap: isWatch ? "inline" : "external",
    external: [],
    splitting: false,
    define: {
      "process.env.NODE_ENV": isWatch ? '"development"' : '"production"',
    },
    loader: {
      ".css": "text",
    },
  });

  if (!result.success) {
    console.error("Build failed:");
    for (const msg of result.logs) {
      console.error(msg);
    }
    process.exit(1);
  }

  console.log(`✓ Built ${result.outputs.length} file(s)`);
}

if (isWatch) {
  console.log("Watching for changes...");
  let debounceTimer;
  const srcDir = path.join(ROOT, "viewer/src");
  fs.watch(srcDir, { recursive: true }, () => {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      console.log("\n🔄 Rebuilding...");
      await build();
    }, 150);
  });
  await build();
  await new Promise(() => {});
} else {
  await build();
}
