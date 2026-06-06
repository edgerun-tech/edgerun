#!/usr/bin/env node

const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const packageRoot = path.join(root, "standards/ui-framework-wat");

execFileSync(path.join(packageRoot, "validate.sh"), {
  cwd: packageRoot,
  stdio: "inherit",
});

console.log("OK ui-framework-wat-smoke");
