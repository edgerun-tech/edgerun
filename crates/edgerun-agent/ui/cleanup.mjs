#!/usr/bin/env node
/**
 * Clean up TypeScript remnants from converted JSX files.
 */
import fs from "node:fs";
import path from "node:path";

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

  // Fix import paths
  content = content.replace(/\.tsx"/g, '.jsx"');
  content = content.replace(/\.tsx'/g, ".jsx'");

  // Remove non-null assertions (!) before property access or method calls
  content = content.replace(/!\./g, '.');

  // Remove "as Type" casts
  content = content.replace(/\s+as\s+[A-Z][\w<>[\],\s|]*?(?=[\s,;)}>\]])/g, '');

  // Remove type annotations in function parameters
  // Simple patterns: remove : Type before ) or = or ,
  content = content.replace(/:\s*(?:Record|Map|Set|Array|Promise)<[^>]+>/g, '');
  content = content.replace(/:\s*[A-Z][\w<>[\],\s|]*?(?=\s*[,)=}])/g, '');

  // Remove ReturnType<> usage
  content = content.replace(/ReturnType<typeof\s+\w+>/g, '');

  // Fix const root = document.getElementById("app-root") missing semicolon
  content = content.replace(/getElementById\("([^"]+)"\)\n/, 'getElementById("$1");\n');

  fs.writeFileSync(file, content);
  console.log(`Cleaned: ${file}`);
}

console.log("Done!");

// Benchmark comment