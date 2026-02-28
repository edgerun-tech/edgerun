import { createSignal } from "solid-js";

const [fsNodeTargetId, setFsNodeTargetId] = createSignal("");

function getFsNodeTargetId() {
  return String(fsNodeTargetId() || "").trim();
}

function setFsNodeTarget(nodeId) {
  setFsNodeTargetId(String(nodeId || "").trim());
}

export { fsNodeTargetId, getFsNodeTargetId, setFsNodeTarget };
