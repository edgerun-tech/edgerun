#!/usr/bin/env node
/**
 * Thoroughly clean TypeScript remnants from converted JSX files.
 */
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
  let content = fs.readFileSync(file, "utf-8");

  // Remove `let x!;` (non-null assertion on declarations)
  content = content.replace(/let\s+(\w+)\s*!;/g, 'let $1;');

  // Remove `const x!;` 
  content = content.replace(/const\s+(\w+)\s*!;/g, 'const $1;');

  // Remove `var x!;`
  content = content.replace(/var\s+(\w+)\s*!;/g, 'var $1;');

  // Remove `: ReturnType<typeof setTimeout>` from let/const/var
  content = content.replace(/(let|const|var)\s+(\w+)\s*:\s*ReturnType<typeof\s+\w+>/g, '$1 $2');

  // Remove `: Type` from let/const/var declarations (simple types)
  content = content.replace(/(let|const|var)\s+(\w+)\s*:\s*[A-Z][\w<>\[\]|,\s]*?(?=\s*[=;\n,])/g, '$1 $2');

  // Remove `: Type` from function parameters
  content = content.replace(/(\w+)\s*:\s*[A-Z][\w<>\[\]|,\s\[\]]*?(?=\s*[)=,}])/g, '$1');

  // Remove `| null` and `| undefined` from function return types
  content = content.replace(/\)\s*:\s*[A-Z][\w<>\[\]|,\s]*?\|.*?\{/g, ') {');

  // Remove remaining `as Type` in JSX expressions and elsewhere
  content = content.replace(/\s+as\s+[a-zA-Z_][\w<>\[\],\s|]*?(?=\s*[)}>\s,;])/g, '');

  // Remove `import type` lines
  content = content.replace(/^import type .*$/gm, '');

  // Fix `!.` property access
  content = content.replace(/!\./g, '.');

  // Remove empty type annotation like `: ;`
  content = content.replace(/:\s*;/g, ';');
  content = content.replace(/:\s*,/g, ',');

  fs.writeFileSync(file, content);
  console.log(`Cleaned: ${file}`);
}

console.log("Done!");

// Benchmark comment