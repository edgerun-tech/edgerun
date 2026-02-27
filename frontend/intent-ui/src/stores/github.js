import { createSignal } from "solid-js";

/**
 * @typedef {object} GitHubFile
 * @property {string} path
 * @property {string} content
 */

/** @type {[() => GitHubFile, import("solid-js").Setter<GitHubFile>]} */
const [currentFile, setCurrentFile] = createSignal({
  path: "README.md",
  content: "# Demo file\n\nEditable content."
});
/**
 * @param {string} path
 * @param {string} [content]
 */
function openGitHubFile(path, content = "") {
  setCurrentFile({ path, content });
}
/** @param {string} content */
function updateFileContent(content) {
  const file = currentFile();
  if (file) {
    setCurrentFile({ ...file, content });
  }
}
export {
  currentFile,
  openGitHubFile,
  updateFileContent
};
