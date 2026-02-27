import { createSignal } from "solid-js";
import { fsRouter } from "../lib/fs/router";

const [currentFile, setCurrentFile] = createSignal({
  path: "/ram/README.md",
  content: "",
  provider: "ramfs"
});
const [lastFsError, setLastFsError] = createSignal("");

async function openFsFile(path) {
  try {
    const content = await fsRouter.read(path);
    const resolved = fsRouter.resolve(path);
    setCurrentFile({
      path,
      content: typeof content === "string" ? content : String(content),
      provider: resolved?.provider?.id || "unknown"
    });
    setLastFsError("");
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to open file.";
    setLastFsError(message);
  }
}

async function updateCurrentFileContent(content) {
  const file = currentFile();
  if (!file?.path) return;
  setCurrentFile({ ...file, content });
  try {
    await fsRouter.write(file.path, content);
    setLastFsError("");
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to save file.";
    setLastFsError(message);
  }
}

async function listFsDir(path) {
  try {
    setLastFsError("");
    return await fsRouter.list(path);
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to list directory.";
    setLastFsError(message);
    return [];
  }
}

async function mkdirFsPath(path) {
  try {
    await fsRouter.mkdir(path);
    setLastFsError("");
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to create directory.";
    setLastFsError(message);
    throw error;
  }
}

async function deleteFsPath(path) {
  try {
    await fsRouter.delete(path);
    setLastFsError("");
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to delete path.";
    setLastFsError(message);
    throw error;
  }
}

async function moveFsPath(from, to) {
  try {
    await fsRouter.move(from, to);
    setLastFsError("");
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to move path.";
    setLastFsError(message);
    throw error;
  }
}

async function copyFsPath(from, to) {
  try {
    await fsRouter.copy(from, to);
    setLastFsError("");
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to copy path.";
    setLastFsError(message);
    throw error;
  }
}

function getFsMounts() {
  return fsRouter.getMounts();
}

if (typeof window !== "undefined") {
  queueMicrotask(() => {
    openFsFile("/ram/README.md");
  });
}

export {
  currentFile,
  getFsMounts,
  lastFsError,
  listFsDir,
  mkdirFsPath,
  deleteFsPath,
  moveFsPath,
  copyFsPath,
  openFsFile,
  updateCurrentFileContent
};
