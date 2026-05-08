const summaryEl = document.getElementById("summary");
const statusEl = document.getElementById("status");
const payloadEl = document.getElementById("payload");
const policyEl = document.getElementById("policy");
const nodesEl = document.getElementById("nodes");
const userNodeEl = document.getElementById("user-node");
const proofEl = document.getElementById("proof");
const eventLogEl = document.getElementById("event-log");
const nodeLogEl = document.getElementById("node-log");
const runEl = document.getElementById("run");
const verifyEl = document.getElementById("verify");
const replayEl = document.getElementById("replay");
const resetEl = document.getElementById("reset");

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();
const LOG_KEY = "edgerun:segmented-verifiable-demo:events";
const pending = new Map();
let hostApp = null;
let nodes = [];
let userNode = null;

const SEGMENTS = [
  { segmentId: "ingest-public-edge-v1", label: "edge-a", role: "public-edge" },
  { segmentId: "policy-private-node-v1", label: "policy-b", role: "private-policy-node" },
  { segmentId: "commit-audit-node-v1", label: "audit-c", role: "audit-node" },
  { segmentId: "replica-check-node-v1", label: "replica-d", role: "replica-check-node" },
  { segmentId: "final-witness-node-v1", label: "witness-e", role: "final-witness-node" },
];

function hex(bytes) {
  return [...new Uint8Array(bytes)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function short(value, head = 10, tail = 8) {
  if (!value) return "none";
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

async function sha256(bytes) {
  return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
}

async function sha256Hex(bytes) {
  return hex(await sha256(bytes));
}

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

async function makeNode(label, role) {
  const keyPair = await crypto.subtle.generateKey(
    { name: "ECDSA", namedCurve: "P-256" },
    true,
    ["sign", "verify"],
  );
  const publicKey = new Uint8Array(await crypto.subtle.exportKey("raw", keyPair.publicKey));
  return {
    label,
    role,
    keyPair,
    publicKeyHex: hex(publicKey),
    nodeId: hex(publicKey.slice(1)),
  };
}

async function ensureNodes() {
  if (nodes.length) return nodes;
  nodes = await Promise.all(SEGMENTS.map((segment) => makeNode(segment.label, segment.role)));
  renderNodes();
  return nodes;
}

async function computeSegment(segmentId, inputHash, policyHash, previousOutputHash, logHead, proofEnvelope) {
  const envelopeHash = await sha256Hex(textEncoder.encode(canonical(proofEnvelope)));
  let material;
  if (segmentId === "ingest-public-edge-v1") {
    material = `segment:${segmentId}\ninput:${inputHash}\nuser:${proofEnvelope.originUserNodeId}`;
  } else if (segmentId === "policy-private-node-v1") {
    material = `segment:${segmentId}\nprevious:${previousOutputHash}\npolicy:${policyHash}`;
  } else if (segmentId === "commit-audit-node-v1") {
    material = `segment:${segmentId}\nprevious:${previousOutputHash}\nhead:${logHead}`;
  } else if (segmentId === "replica-check-node-v1") {
    material = `segment:${segmentId}\nprevious:${previousOutputHash}\nuser:${proofEnvelope.originUserNodeId}\nrelease:${proofEnvelope.releaseId}`;
  } else {
    material = `segment:${segmentId}\nprevious:${previousOutputHash}\nenvelope:${envelopeHash}`;
  }
  return sha256Hex(textEncoder.encode(material));
}

function loadEvents() {
  try {
    const parsed = JSON.parse(localStorage.getItem(LOG_KEY) || "[]");
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function saveEvents(events) {
  localStorage.setItem(LOG_KEY, JSON.stringify(events));
}

async function signEvent(node, eventHash) {
  const signature = await crypto.subtle.sign(
    { name: "ECDSA", hash: "SHA-256" },
    node.keyPair.privateKey,
    textEncoder.encode(eventHash),
  );
  return hex(signature);
}

async function verifyEventSignature(event) {
  const publicKey = await crypto.subtle.importKey(
    "raw",
    fromHex(event.publicKey),
    { name: "ECDSA", namedCurve: "P-256" },
    false,
    ["verify"],
  );
  return crypto.subtle.verify(
    { name: "ECDSA", hash: "SHA-256" },
    publicKey,
    fromHex(event.signature),
    textEncoder.encode(event.eventHash),
  );
}

async function buildEvent(seq, node, segmentId, inputHash, policyHash, previousOutputHash, outputHash, previousEventHash, proofEnvelope) {
  const unsigned = {
    kind: "SEGMENT_COMMITTED",
    seq,
    previousEventHash,
    proofEnvelope,
    nodeId: node.nodeId,
    nodeRole: node.role,
    segmentId,
    inputHash,
    policyHash,
    previousOutputHash,
    outputHash,
    publicKey: node.publicKeyHex,
  };
  const eventHash = await sha256Hex(textEncoder.encode(canonical(unsigned)));
  return {
    ...unsigned,
    eventHash,
    signature: await signEvent(node, eventHash),
  };
}

async function runProof() {
  runEl.disabled = true;
  statusEl.textContent = "Running";
  try {
    const activeNodes = await ensureNodes();
    const anchor = await refreshNode();
    const payloadBytes = textEncoder.encode(payloadEl.value);
    const policyBytes = textEncoder.encode(policyEl.value);
    const inputHash = await sha256Hex(payloadBytes);
    const policyHash = await sha256Hex(policyBytes);
    const proofEnvelope = {
      originUserNodeId: anchor?.nodeId || "standalone-browser-user-node",
      runtimeAppId: hostApp?.runtimeAppId || "not-installed",
      releaseId: hostApp?.releaseId || "not-installed",
      manifestSha256: hostApp?.manifestSha256 || "not-installed",
      packageHash: hostApp?.packageHash || "not-installed",
      executionNodeCount: activeNodes.length,
      segmentCount: SEGMENTS.length,
    };
    let previousOutputHash = inputHash;
    const existingEvents = loadEvents();
    let previousEventHash = existingEvents.at(-1)?.eventHash || "genesis";
    const nextEvents = [];
    for (let index = 0; index < activeNodes.length; index++) {
      const segmentId = SEGMENTS[index].segmentId;
      const outputHash = await computeSegment(segmentId, inputHash, policyHash, previousOutputHash, previousEventHash, proofEnvelope);
      const event = await buildEvent(
        existingEvents.length + index,
        activeNodes[index],
        segmentId,
        inputHash,
        policyHash,
        previousOutputHash,
        outputHash,
        previousEventHash,
        proofEnvelope,
      );
      nextEvents.push(event);
      previousOutputHash = outputHash;
      previousEventHash = event.eventHash;
    }
    const events = [...existingEvents, ...nextEvents];
    saveEvents(events);
    await renderProof(events, inputHash, policyHash);
    statusEl.textContent = "Committed";
    summaryEl.textContent = "Five execution nodes ran the user node anchored input chain, signed each segment report, and appended it to a hash-linked log.";
  } catch (error) {
    statusEl.textContent = "Failed";
    eventLogEl.textContent = error instanceof Error ? error.message : String(error);
  } finally {
    runEl.disabled = false;
  }
}

async function verifyLog() {
  const events = loadEvents();
  if (!events.length) throw new Error("no events to verify");
  for (let index = 0; index < events.length; index++) {
    const event = events[index];
    const previousEventHash = index === 0 ? "genesis" : events[index - 1].eventHash;
    if (event.seq !== index) {
      throw new Error(`event ${index} sequence mismatch`);
    }
    if (event.previousEventHash !== previousEventHash) {
      throw new Error(`event ${index} previous hash mismatch`);
    }
    const unsigned = {
      kind: event.kind,
      seq: event.seq,
      previousEventHash: event.previousEventHash,
      proofEnvelope: event.proofEnvelope,
      nodeId: event.nodeId,
      nodeRole: event.nodeRole,
      segmentId: event.segmentId,
      inputHash: event.inputHash,
      policyHash: event.policyHash,
      previousOutputHash: event.previousOutputHash,
      outputHash: event.outputHash,
      publicKey: event.publicKey,
    };
    const eventHash = await sha256Hex(textEncoder.encode(canonical(unsigned)));
    if (eventHash !== event.eventHash) {
      throw new Error(`event ${index} event hash mismatch`);
    }
    if (!await verifyEventSignature(event)) {
      throw new Error(`event ${index} signature rejected`);
    }
  }
  statusEl.textContent = "Verified";
  summaryEl.textContent = `${events.length} signed segment events verified with contiguous hash links.`;
  return events;
}

async function replayLatest() {
  const events = await verifyLog();
  const latest = events.slice(-SEGMENTS.length);
  if (latest.length !== SEGMENTS.length) throw new Error(`latest proof requires ${SEGMENTS.length} segment events`);
  let previousOutputHash = latest[0].inputHash;
  for (const event of latest) {
    const replayed = await computeSegment(
      event.segmentId,
      event.inputHash,
      event.policyHash,
      previousOutputHash,
      event.previousEventHash,
      event.proofEnvelope,
    );
    if (replayed !== event.outputHash) {
      throw new Error(`${event.segmentId} replay output mismatch`);
    }
    previousOutputHash = replayed;
  }
  statusEl.textContent = "Replayed";
  summaryEl.textContent = "Latest five-node proof replayed with identical output hashes from the recorded inputs and user-node anchor.";
  await renderProof(events, latest[0].inputHash, latest[0].policyHash);
}

function metric(label, value) {
  return `<div class="metric"><span>${label}</span><strong title="${value || ""}">${short(value || "none", 14, 12)}</strong></div>`;
}

async function renderProof(events, inputHash, policyHash) {
  const latest = events.slice(-SEGMENTS.length);
  proofEl.innerHTML = [
    metric("User node", latest[0]?.proofEnvelope?.originUserNodeId || userNode?.nodeId),
    metric("Input hash", inputHash || latest[0]?.inputHash),
    metric("Policy hash", policyHash || latest[0]?.policyHash),
    metric("Final output", latest.at(-1)?.outputHash),
    metric("Log head", events.at(-1)?.eventHash),
  ].join("");
  eventLogEl.textContent = JSON.stringify({
    eventCount: events.length,
    latestProof: latest.map((event) => ({
      seq: event.seq,
      node: short(event.nodeId),
      userNode: short(event.proofEnvelope?.originUserNodeId),
      role: event.nodeRole,
      segment: event.segmentId,
      previous: short(event.previousEventHash),
      output: short(event.outputHash),
      signature: short(event.signature),
    })),
    head: events.at(-1)?.eventHash || null,
  }, null, 2);
}

function renderNodes() {
  nodesEl.innerHTML = nodes.map((node) => `
    <div class="node">
      <strong>${node.label}</strong>
      <span>${node.role}</span>
      <span title="${node.nodeId}">${short(node.nodeId, 16, 12)}</span>
    </div>
  `).join("");
}

function renderUserNode() {
  userNodeEl.innerHTML = [
    metric("Node id", userNode?.nodeId),
    metric("Runtime app", hostApp?.runtimeAppId),
    metric("Release", hostApp?.releaseId),
    metric("Manifest", hostApp?.manifestSha256),
  ].join("");
}

function requestNode(method, path) {
  const id = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  window.parent.postMessage({ type: "edgerun:node.request", id, method, path }, window.location.origin);
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    window.setTimeout(() => {
      if (!pending.has(id)) return;
      pending.delete(id);
      reject(new Error(`node request timed out: ${method} ${path}`));
    }, 8000);
  });
}

async function refreshNode() {
  if (!hostApp) {
    renderUserNode();
    return userNode;
  }
  try {
    const [status, apps] = await Promise.all([
      requestNode("GET", "/protocol/node/status"),
      requestNode("GET", "/protocol/apps"),
    ]);
    const parsedStatus = JSON.parse(status.body || "{}");
    const parsedApps = JSON.parse(apps.body || "{}");
    userNode = {
      nodeId: parsedStatus.nodeId || "unknown-user-node",
      runtime: parsedStatus.runtimeVersion || "unknown",
    };
    renderUserNode();
    nodeLogEl.textContent = JSON.stringify({
      nodeId: parsedStatus.nodeId,
      runtime: parsedStatus.runtimeVersion,
      installedRuntimeAppId: hostApp.runtimeAppId,
      releaseId: hostApp.releaseId,
      manifestSha256: hostApp.manifestSha256,
      installedAppCount: Array.isArray(parsedApps.apps) ? parsedApps.apps.length : 0,
    }, null, 2);
    return userNode;
  } catch (error) {
    nodeLogEl.textContent = error instanceof Error ? error.message : String(error);
    renderUserNode();
    return userNode;
  }
}

window.addEventListener("message", (event) => {
  if (event.origin !== window.location.origin) return;
  const data = event.data;
  if (!data || typeof data !== "object") return;
  if (data.type === "edgerun:host.ready") {
    hostApp = data.app || null;
    summaryEl.textContent = "Installed through the EdgeRun app store and bound to the browser node runtime.";
    void refreshNode();
    return;
  }
  if (data.type === "edgerun:node.response") {
    const wait = pending.get(data.id);
    if (!wait) return;
    pending.delete(data.id);
    if (data.ok) wait.resolve(data);
    else wait.reject(new Error(data.error || `node request failed with status ${data.status}`));
  }
});

runEl.addEventListener("click", () => void runProof());
verifyEl.addEventListener("click", () => void verifyLog().then((events) => renderProof(events)).catch((error) => {
  statusEl.textContent = "Rejected";
  summaryEl.textContent = error instanceof Error ? error.message : String(error);
}));
replayEl.addEventListener("click", () => void replayLatest().catch((error) => {
  statusEl.textContent = "Replay failed";
  summaryEl.textContent = error instanceof Error ? error.message : String(error);
}));
resetEl.addEventListener("click", () => {
  localStorage.removeItem(LOG_KEY);
  eventLogEl.textContent = "No proof has run yet.";
  proofEl.innerHTML = "";
  statusEl.textContent = "Ready";
});

await ensureNodes();
renderUserNode();
await renderProof(loadEvents());
