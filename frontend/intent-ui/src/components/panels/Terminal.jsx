import { For, createSignal, onCleanup, onMount } from "solid-js";

const mockResponses = {
  help: "Available commands:\n  help, ls, pwd, echo, cat, node, npm, git, clear, whoami, date, uname",
  whoami: "browser-user",
  pwd: "/home",
  "uname -a": "WebContainer 1.0.0 browser x86_64",
  "node --version": "v20.10.0",
  "npm --version": "10.2.0",
  "git --version": "git version 2.40.0 (webcontainer)",
  date: new Date().toString(),
};

const mockFileSystem = {
  "/home/package.json": JSON.stringify({ name: "demo-app", version: "1.0.0", scripts: { dev: "bun run dev", build: "bun run build" } }, null, 2),
  "/home/index.js": 'console.log("Hello from browser!");\n',
  "/home/README.md": "# Demo App\n\nRunning in browser!\n",
  "/home/src/app.js": "export const app = { name: 'demo' };\n",
};

function TerminalComponent(props) {
  const [currentDir, setCurrentDir] = createSignal("/home");
  const [history, setHistory] = createSignal([
    "Browser OS Terminal",
    "Frontend-only mode - no server required",
    "Type `help` for available commands",
  ]);
  const [input, setInput] = createSignal("");

  const executeCommand = (cmd) => {
    const trimmedCmd = cmd.trim();

    if (!trimmedCmd) return "";
    if (trimmedCmd === "clear") {
      setHistory([]);
      return "";
    }

    if (trimmedCmd.startsWith("cd ")) {
      const target = trimmedCmd.slice(3);
      if (target === "..") {
        const parts = currentDir().split("/");
        parts.pop();
        setCurrentDir(parts.join("/") || "/");
      } else if (target.startsWith("/")) {
        setCurrentDir(target);
      } else {
        setCurrentDir(`${currentDir()}/${target}`);
      }
      return "";
    }

    if (trimmedCmd.startsWith("echo ")) {
      return trimmedCmd.slice(5).replace(/"/g, "").replace(/'/g, "");
    }

    if (trimmedCmd === "ls" || trimmedCmd === "ls -la") {
      const files = Object.keys(mockFileSystem)
        .filter((p) => p.startsWith(`${currentDir()}/`))
        .map((p) => {
          const relative = p.replace(`${currentDir()}/`, "");
          const parts = relative.split("/");
          return parts.length === 1 ? parts[0] : null;
        })
        .filter(Boolean)
        .join("  ");
      return files || "Directory empty";
    }

    if (trimmedCmd.startsWith("cat ")) {
      const file = trimmedCmd.slice(4);
      const path = file.startsWith("/") ? file : `${currentDir()}/${file}`;
      return mockFileSystem[path] || `cat: ${file}: No such file or directory`;
    }

    if (trimmedCmd.startsWith("npm ")) {
      const args = trimmedCmd.slice(4);
      if (args === "install" || args === "i") return "added 0 packages in 0.1s";
      if (args.startsWith("run ")) return `Running ${args.slice(4)}... Done.`;
      return `npm ${args} executed`;
    }

    if (trimmedCmd.startsWith("git ")) {
      const args = trimmedCmd.slice(4);
      const gitMocks = {
        status: "On branch main\nYour branch is up to date.\n\nnothing to commit, working tree clean",
        "log --oneline -5": "abc1234 Initial commit",
        branch: "* main\n  dev\n  feature/new",
      };
      return gitMocks[args] || `git ${args} executed`;
    }

    if (trimmedCmd.startsWith("node ")) {
      return "Node.js execution complete";
    }

    for (const [key, response] of Object.entries(mockResponses)) {
      if (trimmedCmd === key) return response;
    }

    return `bash: ${trimmedCmd}: command not found`;
  };

  const runCommand = (value = input()) => {
    const cmd = value;
    const dir = currentDir().replace("/home", "~");
    const output = executeCommand(cmd);
    const next = [`user@browser-os:${dir}$ ${cmd}`];
    if (output) next.push(output);
    setHistory((prev) => [...prev, ...next]);
    props.onData?.(cmd);
    setInput("");
  };

  const handleForwardedInput = (event) => {
    const detail = event.detail || {};
    const text = typeof detail.text === "string" ? detail.text : "";
    if (!text.trim()) return;

    if (detail.execute) {
      runCommand(text);
      return;
    }

    setInput(text);
  };

  onMount(() => {
    window.addEventListener("intent:terminal:input", handleForwardedInput);
  });

  onCleanup(() => {
    window.removeEventListener("intent:terminal:input", handleForwardedInput);
  });

  return <div class="h-full w-full bg-[#1a1a1a] text-neutral-200 p-4 flex flex-col gap-3 font-mono text-sm">
      <div class="flex-1 overflow-auto space-y-1">
        <For each={history()}>{(line) => <pre class="whitespace-pre-wrap leading-6">{line}</pre>}</For>
      </div>
      <div class="flex items-center gap-2 border-t border-neutral-700 pt-3">
        <span class="text-emerald-400">user@browser-os:{currentDir().replace("/home", "~")}$</span>
        <input
          class="flex-1 bg-transparent outline-none text-neutral-100"
          value={input()}
          onInput={(e) => setInput(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") runCommand();
          }}
          placeholder="Type a command..."
        />
      </div>
    </div>;
}

function writeToTerminal(text, execute = true) {
  if (!text || typeof window === "undefined") return;
  window.dispatchEvent(
    new CustomEvent("intent:terminal:input", {
      detail: { text, execute }
    })
  );
}

export {
  TerminalComponent as default,
  writeToTerminal,
};
