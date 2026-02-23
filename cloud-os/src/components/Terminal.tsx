import { onMount, onCleanup, createSignal } from 'solid-js';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { ClipboardAddon } from '@xterm/addon-clipboard';
import '@xterm/xterm/css/xterm.css';

// Frontend-only terminal - no server required
// Uses mock responses or WebContainers API when available

interface TerminalProps {
  onData?: (data: string) => void;
}

// Mock command responses for demo
const mockResponses: Record<string, string> = {
  'help': 'Available commands:\n  help, ls, pwd, echo, cat, node, npm, git, clear, whoami, date, uname\n\nType a command to execute.',
  'whoami': 'browser-user',
  'pwd': '/home',
  'uname -a': 'WebContainer 1.0.0 browser x86_64',
  'node --version': 'v20.10.0',
  'npm --version': '10.2.0',
  'git --version': 'git version 2.40.0 (webcontainer)',
  'date': new Date().toString(),
  'clear': '\x1b[2J\x1b[3J\x1b[H', // ANSI clear sequence
};

const mockFileSystem: Record<string, string> = {
  '/home/package.json': JSON.stringify({ name: 'demo-app', version: '1.0.0', scripts: { dev: 'vite', build: 'tsc' } }, null, 2),
  '/home/index.js': 'console.log("Hello from browser!");\n',
  '/home/README.md': '# Demo App\n\nRunning in browser!\n',
  '/home/src/app.js': 'export const app = { name: "demo" };\n',
};

export default function TerminalComponent(props: TerminalProps) {
  let terminalContainer: HTMLDivElement | undefined;
  let terminal: Terminal | undefined;
  let fitAddon: FitAddon | undefined;
  const [isReady, setIsReady] = createSignal(false);
  const [currentDir, setCurrentDir] = createSignal('/home');

  const prompt = '\x1b[1;32muser@browser-os\x1b[0m:\x1b[1;34m~\x1b[0m$ ';

  const executeCommand = (cmd: string): string => {
    const trimmedCmd = cmd.trim();
    
    // Handle clear
    if (trimmedCmd === 'clear') {
      terminal?.clear();
      return '';
    }

    // Handle cd
    if (trimmedCmd.startsWith('cd ')) {
      const target = trimmedCmd.slice(3);
      if (target === '..') {
        const parts = currentDir().split('/');
        parts.pop();
        setCurrentDir(parts.join('/') || '/');
      } else if (target.startsWith('/')) {
        setCurrentDir(target);
      } else {
        setCurrentDir(`${currentDir()}/${target}`);
      }
      return '';
    }

    // Handle echo
    if (trimmedCmd.startsWith('echo ')) {
      return trimmedCmd.slice(5).replace(/"/g, '').replace(/'/g, '');
    }

    // Handle ls
    if (trimmedCmd === 'ls' || trimmedCmd === 'ls -la') {
      const files = Object.keys(mockFileSystem)
        .filter(p => p.startsWith(currentDir() + '/'))
        .map(p => {
          const relative = p.replace(currentDir() + '/', '');
          const parts = relative.split('/');
          return parts.length === 1 ? parts[0] : null;
        })
        .filter(Boolean)
        .join('  ');
      return files || 'Directory empty';
    }

    // Handle cat
    if (trimmedCmd.startsWith('cat ')) {
      const file = trimmedCmd.slice(4);
      const path = file.startsWith('/') ? file : `${currentDir()}/${file}`;
      return mockFileSystem[path] || `cat: ${file}: No such file or directory`;
    }

    // Handle npm
    if (trimmedCmd.startsWith('npm ')) {
      const args = trimmedCmd.slice(4);
      if (args.startsWith('run ')) {
        const script = args.slice(4);
        return `> demo-app@1.0.0 ${script}\n> echo "Running ${script}"\nRunning ${script}\n\nDone.`;
      }
      if (args === 'install' || args === 'i') {
        return 'added 0 packages in 0.1s';
      }
      return `npm ${args} executed`;
    }

    // Handle git
    if (trimmedCmd.startsWith('git ')) {
      const args = trimmedCmd.slice(4);
      const gitMocks: Record<string, string> = {
        'status': 'On branch main\nYour branch is up to date.\n\nnothing to commit, working tree clean',
        'log --oneline -5': 'abc1234 Initial commit\n',
        'branch': '* main\n  dev\n  feature/new',
        'remote -v': 'origin\thttps://github.com/user/repo.git (fetch)\norigin\thttps://github.com/user/repo.git (push)',
        'diff': '',
      };
      return gitMocks[args] || `git ${args} executed`;
    }

    // Handle node
    if (trimmedCmd.startsWith('node ')) {
      const script = trimmedCmd.slice(5);
      try {
        // Very basic eval for demo
        if (script.includes('console.log')) {
          const match = script.match(/console\.log\(['"](.+)['"]\)/);
          return match ? match[1] : '';
        }
        return 'Node.js execution complete';
      } catch {
        return 'SyntaxError in script';
      }
    }

    // Check mock responses
    for (const [key, response] of Object.entries(mockResponses)) {
      if (trimmedCmd === key) {
        return response;
      }
    }

    // Unknown command
    return `bash: ${trimmedCmd}: command not found`;
  };

  const showPrompt = () => {
    const dir = currentDir().replace('/home', '~');
    terminal?.write(`\r\n\x1b[1;32muser@browser-os\x1b[0m:\x1b[1;34m${dir}\x1b[0m$ `);
  };

  onMount(() => {
    if (!terminalContainer) return;

    terminal = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: '#1a1a1a',
        foreground: '#d4d4d4',
        cursor: '#ffffff',
        selectionBackground: '#264f78',
      },
      allowTransparency: true,
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(terminalContainer);
    fitAddon.fit();

    try {
      const webglAddon = new WebglAddon();
      terminal.loadAddon(webglAddon);
    } catch (e) {
      console.log('WebGL not available');
    }

    const clipboardAddon = new ClipboardAddon();
    terminal.loadAddon(clipboardAddon);

    setIsReady(true);

    // Show welcome message
    terminal.write('\x1b[1;34m🖥️  Browser OS Terminal\x1b[0m\r\n');
    terminal.write('Frontend-only mode - no server required\r\n');
    terminal.write('Type \x1b[1;33mhelp\x1b[0m for available commands\r\n\r\n');
    showPrompt();

    let inputBuffer = '';

    const resizeObserver = new ResizeObserver(() => fitAddon?.fit());
    resizeObserver.observe(terminalContainer);

    terminal.onData((data) => {
      const char = data;

      // Handle special characters
      if (char === '\r') { // Enter
        terminal?.write('\r\n');
        if (inputBuffer.trim()) {
          const output = executeCommand(inputBuffer);
          if (output) {
            terminal?.write(output + '\r\n');
          }
        }
        inputBuffer = '';
        showPrompt();
      } else if (char === '\x7f') { // Backspace
        if (inputBuffer.length > 0) {
          inputBuffer = inputBuffer.slice(0, -1);
          terminal?.write('\b \b');
        }
      } else if (char === '\x03') { // Ctrl+C
        terminal?.write('^C\r\n');
        inputBuffer = '';
        showPrompt();
      } else {
        // Regular character
        inputBuffer += char;
        terminal?.write(char);
      }

      props.onData?.(data);
    });

    onCleanup(() => {
      resizeObserver.disconnect();
      terminal?.dispose();
    });
  });

  return (
    <div class="h-full w-full bg-[#1a1a1a] relative">
      <div ref={terminalContainer} class="h-full w-full p-2" />
      {!isReady() && (
        <div class="absolute inset-0 flex items-center justify-center text-neutral-500">
          Loading terminal...
        </div>
      )}
    </div>
  );
}

export function writeToTerminal(terminal: Terminal | undefined, text: string) {
  terminal?.write(text);
}
