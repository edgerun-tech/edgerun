#!/usr/bin/env node
/**
 * Pre-transform all JSX files with Babel's Solid preset before Bun builds them.
 * Also copies non-JSX source files to the build directory.
 */
import babel from "@babel/core";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.resolve(import.meta.dir, "../..");
const SRC_DIR = path.join(ROOT, "viewer/src");
const BUILD_DIR = path.join(ROOT, "viewer/.build");

const JSX_FILES = [
  "main.jsx",
  "app.jsx",
  "components/ChatPanel.jsx",
  "components/CodePanel.jsx",
  "components/DiagnosticsPanel.jsx",
  "components/FileExplorer.jsx",
  "components/GraphCanvas.jsx",
  "components/KeyboardShortcuts.jsx",
  "components/RepoPanel.jsx",
  "components/Resizer.jsx",
  "components/Sidebar.jsx",
];

const NON_JSX_FILES = [
  "store.ts",
  "types.ts",
  "ws.ts",
  "lib/gl.ts",
  "index.css",
];

// Clean build dir
fs.rmSync(BUILD_DIR, { recursive: true, force: true });
fs.mkdirSync(BUILD_DIR, { recursive: true });
fs.mkdirSync(path.join(BUILD_DIR, "components"), { recursive: true });
fs.mkdirSync(path.join(BUILD_DIR, "lib"), { recursive: true });

// Copy non-JSX files
for (const file of NON_JSX_FILES) {
  const src = path.join(SRC_DIR, file);
  const dst = path.join(BUILD_DIR, file);
  if (fs.existsSync(src)) {
    fs.cpSync(src, dst);
    console.log(`Copied: ${file}`);
  }
}

// Transform JSX files
for (const file of JSX_FILES) {
  const srcFile = path.join(SRC_DIR, file);
  if (!fs.existsSync(srcFile)) continue;

  const content = fs.readFileSync(srcFile, "utf-8");
  // Strip the jsxImportSource pragma
  const cleanContent = content.replace(/^\/\*\* @jsxImportSource solid-js \*\*\/\n/, "");

  try {
    const result = babel.transformSync(cleanContent, {
      filename: file,
      presets: [
        ["babel-preset-solid", { generate: "dom" }],
      ],
    });

    if (result?.code) {
      const outFile = path.join(BUILD_DIR, file);
      fs.writeFileSync(outFile, result.code);
      console.log(`✓ Transformed: ${file}`);
    }
  } catch (e) {
    console.error(`✗ Failed: ${file}`);
    console.error(`  ${e.message}`);
  }
}

console.log("\nDone!");

// Benchmark comment