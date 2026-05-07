#!/usr/bin/env node

const crypto = require("crypto");
const { spawnSync } = require("child_process");
const fs = require("fs");

const graphPath = process.argv[2] || "standards/build/wasm/udp-tftp-hashed-fixed-abi/graph.json";
const corpusDir = process.argv[3] || "standards/corpus/udp-tftp";

function sha256(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

const graph = JSON.parse(fs.readFileSync(graphPath, "utf8"));
const result = {
  graph: graph.id,
  graph_path: graphPath,
  ok: true,
  checks: [],
};

if (typeof graph.canonical_preimage === "string") {
  const computedGraphHash = sha256(Buffer.from(graph.canonical_preimage, "utf8"));
  const ok = computedGraphHash === graph.sha256;
  result.ok &&= ok;
  result.checks.push({
    kind: "graph-preimage-sha256",
    ok,
    expected: graph.sha256,
    actual: computedGraphHash,
  });
} else {
  result.ok = false;
  result.checks.push({
    kind: "graph-preimage-sha256",
    ok: false,
    error: "missing canonical_preimage",
  });
}

for (const node of graph.nodes || []) {
  let actual = null;
  let ok = false;
  try {
    actual = sha256(fs.readFileSync(node.wasm));
    ok = actual === node.sha256;
  } catch (error) {
    result.checks.push({
      kind: "node-wasm-sha256",
      node: node.id,
      ok: false,
      expected: node.sha256,
      error: String(error.message || error),
    });
    result.ok = false;
    continue;
  }
  result.ok &&= ok;
  result.checks.push({
    kind: "node-wasm-sha256",
    node: node.id,
    ok,
    expected: node.sha256,
    actual,
  });
}

const runner = "standards/runners/fixed-abi-graph.js";
const args = [runner, graphPath, corpusDir];
const run = spawnSync(process.execPath, args, { encoding: "utf8" });
let execution = null;
try {
  execution = JSON.parse(run.stdout);
} catch {
  execution = {
    ok: false,
    stdout: run.stdout,
    stderr: run.stderr,
  };
}
const executionOk = run.status === 0 && execution.ok === true;
result.ok &&= executionOk;
result.checks.push({
  kind: "graph-execution",
  ok: executionOk,
  status: run.status,
  execution,
});

console.log(JSON.stringify(result, null, 2));
process.exit(result.ok ? 0 : 1);
