import { onMount, onCleanup, createSignal } from 'solid-js';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { ClipboardAddon } from '@xterm/addon-clipboard';
import '@xterm/xterm/css/xterm.css';

// Frontend-only terminal surface.
// Real command execution is only available through the dedicated MCP terminal worker.

interface TerminalProps {
  onData?: (data: string) => void;
}

export default function TerminalComponent(props: TerminalProps) {
  let terminalContainer: HTMLDivElement | undefined;
  let terminal: Terminal | undefined;
  let fitAddon: FitAddon | undefined;
  const [isReady, setIsReady] = createSignal(false);
  const [currentDir] = createSignal('/home');

  const executeCommand = (cmd: string): string => {
    const trimmedCmd = cmd.trim();
    if (trimmedCmd === 'clear') {
      terminal?.clear();
      return '';
    }
    if (trimmedCmd === 'help') {
      return 'Interactive mock terminal commands have been removed. Use MCP terminal tools for real execution.';
    }
    if (!trimmedCmd) return '';
    return `Command execution unavailable in this view: ${trimmedCmd}`;
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
      // WebGL acceleration is optional.
    }

    const clipboardAddon = new ClipboardAddon();
    terminal.loadAddon(clipboardAddon);

    setIsReady(true);

    // Show welcome message
    terminal.write('\x1b[1;34m🖥️  Browser OS Terminal\x1b[0m\r\n');
    terminal.write('Command mocks are disabled in this surface.\r\n');
    terminal.write('Use MCP terminal tools for real command execution.\r\n\r\n');
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
