import { createSignal } from 'solid-js';

export type GitHubFile = {
  path: string;
  content: string;
};

const [file, setFile] = createSignal<GitHubFile | null>(null);

export function currentFile() {
  return file;
}

export function openGitHubFile(path: string, content: string): void {
  setFile({ path, content });
}

export function updateFileContent(content: string): void {
  const current = file();
  if (!current) return;
  setFile({ ...current, content });
}
