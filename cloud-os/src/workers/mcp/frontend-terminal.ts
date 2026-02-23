/**
 * Frontend-Only Terminal
 * Uses WebContainers API for Node.js in browser, with mock fallback
 */

import { MCPServerBase, setupWorkerServer } from './base';

// Mock file system for demo
const mockFileSystem: Record<string, string> = {
  '/home/package.json': JSON.stringify({ name: 'demo-app', version: '1.0.0' }, null, 2),
  '/home/index.js': 'console.log("Hello from browser!");\n',
  '/home/README.md': '# Demo App\n\nRunning in browser with WebContainers!\n',
  '/home/src/app.js': 'export const app = { name: "demo" };\n',
};

// Mock command responses
const mockCommands: Record<string, string> = {
  'help': 'Available commands: help, ls, pwd, echo, cat, node, npm, git, clear, whoami, date, uname',
  'whoami': 'browser-user',
  'pwd': '/home',
  'uname -a': 'WebContainer 1.0.0 browser x86_64',
  'node --version': 'v20.10.0',
  'npm --version': '10.2.0',
  'git --version': 'git version 2.40.0 (webcontainer)',
};

class FrontendTerminalServer extends MCPServerBase {
  private webContainer: any = null;
  private connected = false;

  constructor() {
    super('frontend-terminal', '1.0.0');
  }

  async initializeWebContainer(): Promise<boolean> {
    // Check if WebContainers are supported (Chrome/Edge only)
    if (typeof navigator === 'undefined') return false;
    
    const isChrome = /Chrome/.test(navigator.userAgent) && /Google/.test(navigator.vendor);
    if (!isChrome) {
      console.log('[Terminal] WebContainers not supported, using mock mode');
      this.connected = true; // Mock mode is always "connected"
      return false;
    }

    try {
      // Dynamically import WebContainers
      const { WebContainer } = await import('@webcontainer/api');
      this.webContainer = await WebContainer.boot();
      this.connected = true;
      console.log('[Terminal] WebContainer booted successfully');
      return true;
    } catch (error) {
      console.warn('[Terminal] Failed to boot WebContainer:', error);
      this.connected = true; // Fall back to mock mode
      return false;
    }
  }

  setupHandlers(): void {
    // Initialize WebContainer on startup
    this.initializeWebContainer();

    // Send command to terminal
    this.registerTool(
      {
        name: 'terminal_execute',
        description: 'Execute a shell command in the browser terminal (supports Node.js, npm, git, and basic shell commands)',
        inputSchema: {
          type: 'object',
          properties: {
            command: {
              type: 'string',
              description: 'Command to execute',
            },
            args: {
              type: 'array',
              description: 'Command arguments',
              items: { type: 'string' },
            },
            cwd: {
              type: 'string',
              description: 'Working directory',
              default: '/home',
            },
          },
          required: ['command'],
        },
      },
      async (args) => {
        const fullCommand = `${args.command} ${args.args?.join(' ') || ''}`.trim();

        if (this.webContainer) {
          return await this.executeWebContainerCommand(fullCommand, args.cwd);
        } else {
          return this.executeMockCommand(fullCommand);
        }
      }
    );

    // Get terminal status
    this.registerTool(
      {
        name: 'terminal_status',
        description: 'Get terminal connection status and capabilities',
        inputSchema: {
          type: 'object',
          properties: {},
        },
      },
      async () => {
        const status = {
          connected: this.connected,
          mode: this.webContainer ? 'webcontainer' : 'mock',
          browser: typeof navigator !== 'undefined' ? navigator.userAgent : 'unknown',
          capabilities: this.webContainer 
            ? ['node', 'npm', 'git', 'shell', 'filesystem']
            : ['mock-commands'],
        };

        return {
          content: [{
            type: 'text',
            text: JSON.stringify(status, null, 2),
          }],
        };
      }
    );

    // List files
    this.registerTool(
      {
        name: 'terminal_list_files',
        description: 'List files in the terminal working directory',
        inputSchema: {
          type: 'object',
          properties: {
            path: {
              type: 'string',
              description: 'Directory path',
              default: '/home',
            },
          },
        },
      },
      async (args) => {
        if (this.webContainer) {
          try {
            const dir = await this.webContainer.fs.readdir(args.path || '/home');
            return {
              content: [{
                type: 'text',
                text: dir.join('\n'),
              }],
            };
          } catch (e) {
            return {
              content: [{ type: 'text', text: `Error: ${e}` }],
              isError: true,
            };
          }
        } else {
          // Mock file listing
          const files = Object.keys(mockFileSystem)
            .filter(p => p.startsWith(args.path || '/home'))
            .map(p => p.split('/').pop() || p);
          
          return {
            content: [{
              type: 'text',
              text: files.join('\n') || 'Directory empty',
            }],
          };
        }
      }
    );

    // Read file
    this.registerTool(
      {
        name: 'terminal_read_file',
        description: 'Read a file in the terminal filesystem',
        inputSchema: {
          type: 'object',
          properties: {
            path: {
              type: 'string',
              description: 'File path',
            },
          },
          required: ['path'],
        },
      },
      async (args) => {
        if (this.webContainer) {
          try {
            const content = await this.webContainer.fs.readFile(args.path, 'utf-8');
            return {
              content: [{ type: 'text', text: content }],
            };
          } catch (e) {
            return {
              content: [{ type: 'text', text: `Error: ${e}` }],
              isError: true,
            };
          }
        } else {
          const content = mockFileSystem[args.path];
          if (content) {
            return { content: [{ type: 'text', text: content }] };
          }
          return {
            content: [{ type: 'text', text: `File not found: ${args.path}` }],
            isError: true,
          };
        }
      }
    );

    // Write file
    this.registerTool(
      {
        name: 'terminal_write_file',
        description: 'Write content to a file in the terminal filesystem',
        inputSchema: {
          type: 'object',
          properties: {
            path: {
              type: 'string',
              description: 'File path',
            },
            content: {
              type: 'string',
              description: 'File content',
            },
          },
          required: ['path', 'content'],
        },
      },
      async (args) => {
        if (this.webContainer) {
          try {
            await this.webContainer.fs.writeFile(args.path, args.content);
            return {
              content: [{ type: 'text', text: `Written to ${args.path}` }],
            };
          } catch (e) {
            return {
              content: [{ type: 'text', text: `Error: ${e}` }],
              isError: true,
            };
          }
        } else {
          mockFileSystem[args.path] = args.content;
          return {
            content: [{ type: 'text', text: `Written to ${args.path} (mock mode)` }],
          };
        }
      }
    );

    // Run npm script
    this.registerTool(
      {
        name: 'terminal_npm_run',
        description: 'Run an npm script in the terminal',
        inputSchema: {
          type: 'object',
          properties: {
            script: {
              type: 'string',
              description: 'Script name (e.g., dev, build, test)',
            },
          },
          required: ['script'],
        },
      },
      async (args) => {
        if (this.webContainer) {
          try {
            const installProcess = await this.webContainer.spawn('npm', ['install']);
            await installProcess.output.pipeTo(new WritableStream({
              write: (data) => console.log('[npm install]', data),
            }));

            const runProcess = await this.webContainer.spawn('npm', ['run', args.script]);
            let output = '';
            
            runProcess.output.pipeTo(new WritableStream({
              write: (data) => {
                output += data;
                console.log('[npm run]', data);
              },
            }));

            const exitCode = await runProcess.exit;
            return {
              content: [{ 
                type: 'text', 
                text: output || `npm run ${args.script} completed (exit code: ${exitCode})`,
              }],
            };
          } catch (e) {
            return {
              content: [{ type: 'text', text: `Error: ${e}` }],
              isError: true,
            };
          }
        } else {
          return {
            content: [{ 
              type: 'text', 
              text: `[mock] npm run ${args.script}\n> demo-app@1.0.0 ${args.script}\n> echo "Running ${args.script}"\nRunning ${args.script}\n`,
            }],
          };
        }
      }
    );

    // Git commands
    this.registerTool(
      {
        name: 'terminal_git',
        description: 'Run git commands in the terminal',
        inputSchema: {
          type: 'object',
          properties: {
            args: {
              type: 'array',
              description: 'Git arguments',
              items: { type: 'string' },
            },
          },
          required: ['args'],
        },
      },
      async (args) => {
        const gitCmd = `git ${args.args?.join(' ') || ''}`;
        
        if (this.webContainer) {
          try {
            const process = await this.webContainer.spawn('git', args.args || []);
            let output = '';
            
            process.output.pipeTo(new WritableStream({
              write: (data) => { output += data; },
            }));

            await process.exit;
            return {
              content: [{ type: 'text', text: output || 'Git command completed' }],
            };
          } catch (e) {
            return {
              content: [{ type: 'text', text: `Error: ${e}` }],
              isError: true,
            };
          }
        } else {
          // Mock git responses
          const mockGit: Record<string, string> = {
            'status': 'On branch main\nYour branch is up to date.\n\nnothing to commit, working tree clean',
            'log --oneline -5': 'abc123 Initial commit\n',
            'branch': '* main\n  dev\n  feature/test',
            'remote -v': 'origin\thttps://github.com/user/repo.git (fetch)\norigin\thttps://github.com/user/repo.git (push)',
          };
          
          const key = args.args?.join(' ') || '';
          return {
            content: [{ 
              type: 'text', 
              text: mockGit[key] || `[mock] ${gitCmd}\nGit command executed`,
            }],
          };
        }
      }
    );
  }

  private async executeWebContainerCommand(command: string, cwd: string = '/home') {
    if (!this.webContainer) {
      return this.executeMockCommand(command);
    }

    try {
      // Parse command into parts
      const parts = command.split(' ');
      const cmd = parts[0];
      const args = parts.slice(1);

      // Change to working directory
      if (cwd && cwd !== '/home') {
        try {
          await this.webContainer.fs.mkdir(cwd, { recursive: true });
        } catch {}
      }

      // Execute command
      const process = await this.webContainer.spawn(cmd, args);
      let output = '';

      process.output.pipeTo(new WritableStream({
        write: (data) => { output += data; },
      }));

      const exitCode = await process.exit;

      return {
        content: [{ 
          type: 'text', 
          text: output || `Command completed (exit code: ${exitCode})`,
        }],
        ui: {
          viewType: 'log-viewer',
          title: command,
          description: `Exit code: ${exitCode}`,
          metadata: {
            command,
            cwd,
            exitCode,
            timestamp: new Date().toISOString(),
          },
          logs: output.split('\n').map((line, i) => ({
            timestamp: new Date().toISOString(),
            level: 'info',
            message: line,
          })),
        },
      };
    } catch (error: any) {
      return {
        content: [{ type: 'text', text: `Error: ${error.message || error}` }],
        isError: true,
      };
    }
  }

  private executeMockCommand(command: string) {
    const cmd = command.toLowerCase().trim();

    // Check mock commands
    for (const [key, response] of Object.entries(mockCommands)) {
      if (cmd === key || cmd.startsWith(key + ' ')) {
        return {
          content: [{ type: 'text', text: response }],
        };
      }
    }

    // Handle echo
    if (cmd.startsWith('echo ')) {
      return {
        content: [{ type: 'text', text: cmd.slice(5).replace(/"/g, '') }],
      };
    }

    // Handle ls
    if (cmd === 'ls' || cmd === 'ls -la') {
      const files = Object.keys(mockFileSystem)
        .filter(p => p.startsWith('/home/'))
        .map(p => p.replace('/home/', ''));
      return {
        content: [{ type: 'text', text: files.join('  ') }],
      };
    }

    // Handle cat
    if (cmd.startsWith('cat ')) {
      const path = cmd.slice(4);
      const fullPath = path.startsWith('/') ? path : `/home/${path}`;
      const content = mockFileSystem[fullPath];
      if (content) {
        return { content: [{ type: 'text', text: content }] };
      }
      return {
        content: [{ type: 'text', text: `cat: ${path}: No such file or directory` }],
        isError: true,
      };
    }

    // Handle node
    if (cmd.startsWith('node ')) {
      return {
        content: [{ 
          type: 'text', 
          text: `[mock node] Executing ${cmd.slice(5)}\nOutput would appear here in WebContainer mode`,
        }],
      };
    }

    // Default response
    return {
      content: [{ 
        type: 'text', 
        text: `[mock terminal] Command executed: ${command}\n\nTip: Use WebContainer (Chrome/Edge) for real command execution.`,
      }],
    };
  }
}

setupWorkerServer(FrontendTerminalServer);
