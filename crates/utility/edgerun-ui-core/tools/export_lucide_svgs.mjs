#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const icons = [
  "activity",
  "app-window",
  "bell",
  "message-circle",
  "check",
  "chevron-right",
  "code",
  "cpu",
  "database",
  "eye",
  "file",
  "key",
  "lock",
  "menu",
  "message-circle-plus",
  "network",
  "route",
  "search",
  "arrow-up",
  "server",
  "settings",
  "shield-check",
  "sparkles",
  "square-terminal",
  "trash-2",
  "user",
  "wallet",
  "triangle-alert",
  "x",
];

function usage() {
  console.error(
    [
      "Usage:",
      "  node tools/export_lucide_svgs.mjs <@lucide/icons package dir> <output svg dir>",
      "",
      "Example:",
      "  npm --prefix /tmp/edgerun-lucide add @lucide/icons",
      "  node crates/utility/edgerun-ui-core/tools/export_lucide_svgs.mjs \\",
      "    /tmp/edgerun-lucide/node_modules/@lucide/icons \\",
      "    /tmp/edgerun-lucide-svg",
    ].join("\n"),
  );
}

function renderSvg(children) {
  const body = children
    .map(([tag, attrs]) => {
      const attr = Object.entries(attrs)
        .filter(([key]) => key !== "key")
        .map(([key, value]) => `${key}="${String(value)}"`)
        .join(" ");
      return `  <${tag} ${attr} />`;
    })
    .join("\n");
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">',
    body,
    "</svg>",
    "",
  ].join("\n");
}

const [packageDir, outDir] = process.argv.slice(2);
if (!packageDir || !outDir) {
  usage();
  process.exit(2);
}

const modulePath = path.join(packageDir, "dist", "esm", "lucide-icons.mjs");
if (!fs.existsSync(modulePath)) {
  console.error(`missing Lucide module: ${modulePath}`);
  process.exit(1);
}

const packageJsonPath = path.join(packageDir, "package.json");
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
const mod = await import(pathToFileURL(modulePath).href);
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(
  path.join(outDir, "lucide-package-version.txt"),
  `${packageJson.name}@${packageJson.version}\n`,
);

const missing = [];
for (const name of icons) {
  const exportName = name.replace(/(^|-)([a-z0-9])/g, (_match, _sep, ch) =>
    ch.toUpperCase(),
  );
  const icon = mod[exportName];
  if (!icon || !Array.isArray(icon.node)) {
    missing.push(name);
    continue;
  }
  fs.writeFileSync(path.join(outDir, `${name}.svg`), renderSvg(icon.node));
}

if (missing.length > 0) {
  console.error(`missing icons: ${missing.join(", ")}`);
  process.exit(1);
}

console.log(`wrote ${icons.length} Lucide SVGs from ${packageJson.name}@${packageJson.version} to ${outDir}`);
