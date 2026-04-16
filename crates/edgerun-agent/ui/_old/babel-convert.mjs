#!/usr/bin/env node
/**
 * Use Babel with babel-preset-solid to properly convert TSX to JSX.
 * Removes TypeScript syntax and applies Solid's JSX transform.
 */
import babel from "@babel/core";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.resolve(import.meta.dir, "../..");

const FILES = [
  "viewer/src/main.jsx",
  "viewer/src/app.jsx",
  "viewer/src/components/ChatPanel.jsx",
  "viewer/src/components/CodePanel.jsx",
  "viewer/src/components/DiagnosticsPanel.jsx",
  "viewer/src/components/FileExplorer.jsx",
  "viewer/src/components/GraphCanvas.jsx",
  "viewer/src/components/KeyboardShortcuts.jsx",
  "viewer/src/components/RepoPanel.jsx",
  "viewer/src/components/Resizer.jsx",
  "viewer/src/components/Sidebar.jsx",
];

for (const file of FILES) {
  const fullPath = path.join(ROOT, file);
  if (!fs.existsSync(fullPath)) {
    console.log(`Skipping missing: ${file}`);
    continue;
  }

  const content = fs.readFileSync(fullPath, "utf-8");

  try {
    const result = babel.transformSync(content, {
      filename: file,
      presets: [
        ["@babel/preset-typescript", { isTSX: true, allExtensions: true }],
      ],
    });

    if (result?.code) {
      // Add jsxImportSource pragma at the top
      const code = `/** @jsxImportSource solid-js **/\n${result.code}`;
      fs.writeFileSync(fullPath, code);
      console.log(`✓ Transpiled: ${file}`);
    }
  } catch (e) {
    console.error(`✗ Failed: ${file}`);
    console.error(`  ${e.message}`);
  }
}

console.log("\nDone!");

// Benchmark comment