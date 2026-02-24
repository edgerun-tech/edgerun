import { openGitHubFile } from '../stores/github';
import { openWindow } from '../stores/windows';

const sampleFiles = [
  { path: 'README.md', content: '# Sample Repo\n\nThis is a sample file.' },
  { path: 'src/main.ts', content: 'export const hello = () => "hello";\n' },
];

export default function GitHubBrowser() {
  return (
    <div class="h-full w-full bg-[#151515] text-neutral-100 p-3">
      <h2 class="text-sm font-semibold mb-3">GitHub Browser</h2>
      <p class="text-xs text-neutral-400 mb-3">Select a file to open it in Editor.</p>
      <div class="space-y-2">
        {sampleFiles.map((file) => (
          <button
            type="button"
            class="w-full text-left rounded border border-neutral-800 px-2 py-1 hover:bg-neutral-800"
            onClick={() => {
              openGitHubFile(file.path, file.content);
              openWindow('editor');
            }}
          >
            {file.path}
          </button>
        ))}
      </div>
    </div>
  );
}
