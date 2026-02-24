/**
 * IntentBar - Central Command Center
 * Unified interface for commands, clock, weather, file upload, and resource filtering
 */

import { createSignal, createEffect, Show, For, onMount, onCleanup } from 'solid-js';
import { Motion } from 'solid-motionone';
import {
  TbOutlineCommand,
  TbOutlineMicrophone,
  TbOutlineTerminal,
  TbOutlineX,
  TbOutlineCloud,
  TbOutlineMail,
  TbOutlineCalendar,
  TbOutlineUpload,
  TbOutlineFile,
  TbOutlineFolder,
  TbOutlineSearch,
  TbOutlineSun,
  TbOutlineCloud as CloudIcon,
  TbOutlineCloudRain,
  TbOutlineWind,
  TbOutlineDroplet,
  TbOutlineClock,
  TbOutlineFilter,
  TbOutlineLogs,
  TbOutlinePin,
  TbOutlineTrash,
  TbOutlineHistory,
} from 'solid-icons/tb';
import { FiSettings, FiGithub, FiGlobe } from 'solid-icons/fi';
import { mcpManager } from '../lib/mcp/client';
import { mcpMainThreadHandler } from '../lib/mcp/main-thread-handler';
import { llmRouter, defaultProviders } from '../lib/llm/router';
import { intentProcessor, type ExecutionPlan, type AppContext } from '../lib/intent/processor';
import { intentExecutor } from '../lib/intent/executor';
import { context, addRecentCommand, addOpenWindow, removeOpenWindow } from '../stores/context';
import { openWindow, closeWindow } from '../stores/windows';
import { integrationStore } from '../stores/integrations';
import { 
  getAllResults, 
  getPinnedResults, 
  addResult, 
  removeResult, 
  pinResult,
  clearResults,
} from '../lib/stores/results';
import { ResultRenderer } from './results/ResultRenderer';
import type { ToolResponse } from '../lib/mcp/types';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

function cn(...classes: ClassValue[]) {
  return twMerge(clsx(classes));
}

// Global state
const [query, setQuery] = createSignal('');
const [plan, setPlan] = createSignal<ExecutionPlan | null>(null);
const [loading, setLoading] = createSignal(false);
const [executing, setExecuting] = createSignal(false);
const [error, setError] = createSignal<string | null>(null);
const [mode, setMode] = createSignal<'intent' | 'shell' | 'files' | 'email' | 'logs' | 'cloud'>('intent');
const [listening, setListening] = createSignal(false);
const [uploadedFiles, setUploadedFiles] = createSignal<File[]>([]);
const [filterResults, setFilterResults] = createSignal<any[]>([]);
const [isFiltering, setIsFiltering] = createSignal(false);

// Result history state
const [showHistory, setShowHistory] = createSignal(false);
const [results, setResults] = createSignal(getAllResults());
const [pinnedResults, setPinnedResults] = createSignal(getPinnedResults());

// Time and Weather state
const [currentTime, setCurrentTime] = createSignal(new Date());
const [weather, setWeather] = createSignal({
  temp: 31,
  condition: 'sunny' as 'sunny' | 'cloudy' | 'rainy' | 'stormy',
  humidity: 78,
  windSpeed: 12,
  location: 'Pattaya, Thailand',
  feelsLike: 38,
  forecast: [
    { day: 'Mon', temp: 32, condition: 'sunny' as const },
    { day: 'Tue', temp: 33, condition: 'cloudy' as const },
    { day: 'Wed', temp: 30, condition: 'rainy' as const },
    { day: 'Thu', temp: 31, condition: 'rainy' as const },
    { day: 'Fri', temp: 32, condition: 'cloudy' as const },
    { day: 'Sat', temp: 34, condition: 'sunny' as const },
    { day: 'Sun', temp: 33, condition: 'sunny' as const },
  ]
});

// Speech recognition
let recognition: any | null = null;
let fileInputRef: HTMLInputElement | undefined;
let debounceTimer: ReturnType<typeof setTimeout>;

// Filter presets
const filterPresets = [
  { id: 'files', label: 'Files', icon: TbOutlineFile, color: 'text-blue-400' },
  { id: 'email', label: 'Email', icon: TbOutlineMail, color: 'text-purple-400' },
  { id: 'logs', label: 'Logs', icon: TbOutlineLogs, color: 'text-green-400' },
  { id: 'cloud', label: 'Cloud', icon: TbOutlineCloud, color: 'text-orange-400' },
];

export default function IntentBar() {
  let inputRef: HTMLInputElement | undefined;

  onMount(async () => {
    // Initialize time updates
    const timeInterval = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);

    // Check all integration connections
    integrationStore.checkAll();

    try {
      await mcpManager.connectServer({
        id: 'browser-os',
        name: 'BrowserOS',
        type: 'builtin',
        workerScript: '/workers/mcp/browser-os.js',
        enabled: true,
      });

      const githubToken = localStorage.getItem('github_token');
      if (githubToken) {
        await mcpManager.connectServer({
          id: 'github',
          name: 'GitHub',
          type: 'builtin',
          workerScript: '/workers/mcp/github.js',
          enabled: true,
        });
      }

      const cloudflareToken = localStorage.getItem('cloudflare_token');
      if (cloudflareToken) {
        await mcpManager.connectServer({
          id: 'cloudflare',
          name: 'Cloudflare',
          type: 'builtin',
          workerScript: '/workers/mcp/cloudflare.js',
          enabled: true,
        });
      }

      // Check Qwen OAuth connection
      const qwenToken = localStorage.getItem('qwen_token');
      if (qwenToken) {
        try {
          const tokenData = JSON.parse(qwenToken);
          // Connect Qwen MCP server if token is valid
          if (tokenData.access_token && Date.now() < tokenData.expiry_date) {
            await mcpManager.connectServer({
              id: 'qwen',
              name: 'Qwen Code',
              type: 'builtin',
              workerScript: '/workers/mcp/qwen.js',
              enabled: true,
              auth: {
                type: 'oauth',
                oauthProvider: 'qwen',
                tokenKey: 'qwen_token',
              },
            });
          }
        } catch (e) {
          console.warn('[IntentBar] Qwen token parse error:', e);
        }
      }

      await mcpManager.connectServer({
        id: 'terminal',
        name: 'Terminal',
        type: 'builtin',
        workerScript: '/workers/mcp/terminal.js',
        enabled: true,
      });

      defaultProviders.forEach(p => {
        llmRouter.addProvider(p);
      });

      setupSpeechRecognition();
      setupMCPMessageHandling();
    } catch (e) {
      console.error('Failed to initialize IntentBar:', e);
    }

    onCleanup(() => {
      clearInterval(timeInterval);
      if (recognition) {
        recognition.stop();
      }
      clearTimeout(debounceTimer);
    });
  });

  createEffect(() => {
    const q = query();
    clearTimeout(debounceTimer);

    if (!q.trim()) {
      setPlan(null);
      setError(null);
      setFilterResults([]);
      setIsFiltering(false);
      return;
    }

    // Check for filter mode
    const filterMatch = q.match(/^\/(files|email|logs|cloud)\s+(.+)/i);
    if (filterMatch) {
      const [, filterType, searchTerm] = filterMatch;
      setMode(filterType as any);
      performFilter(filterType, searchTerm);
      return;
    }

    if (q.toLowerCase().trim() === 'shell') {
      setMode('shell');
      handleShellMode();
      return;
    }

    // Only auto-process for filter modes, not for general queries
    // General queries are processed on Enter key press
  });

  const handleKeyDown = async (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      const q = query();
      if (q.trim()) {
        await processQuery(q);
      }
    }
  };

  const performFilter = async (type: string, searchTerm: string) => {
    setIsFiltering(true);
    setError(null);

    try {
      const mockResults = [
        { id: 1, type, name: `${searchTerm} - Result 1`, path: `/${type}/${searchTerm}-1`, modified: new Date() },
        { id: 2, type, name: `${searchTerm} - Result 2`, path: `/${type}/${searchTerm}-2`, modified: new Date() },
        { id: 3, type, name: `${searchTerm} - Result 3`, path: `/${type}/${searchTerm}-3`, modified: new Date() },
      ];
      setFilterResults(mockResults);
    } catch (e) {
      setError('Filter operation failed');
    } finally {
      setIsFiltering(false);
    }
  };

  const runCodexCli = async (rawPrompt: string) => {
    const prompt = rawPrompt.trim();
    if (!prompt) {
      setError('Usage: /codex <prompt>');
      return;
    }

    const startedAt = Date.now();
    const response = await fetch('/api/codex/execute', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        prompt,
        cwd: context.currentRepo || undefined,
      }),
    });

    const payload = await response.json();
    if (!response.ok || payload?.error) {
      throw new Error(payload?.error || 'Codex CLI request failed');
    }

    const logs = [
      {
        level: 'info',
        source: 'codex-cli',
        message: `Prompt: ${prompt}`,
        timestamp: new Date().toISOString(),
      },
      {
        level: payload.ok ? 'info' : 'error',
        source: 'codex-cli',
        message: `Exit ${payload.exitCode} in ${payload.durationMs}ms`,
        timestamp: new Date().toISOString(),
      },
      ...(payload.finalText
        ? [{
            level: 'info',
            source: 'assistant',
            message: payload.finalText,
            timestamp: new Date().toISOString(),
          }]
        : []),
      ...(payload.stderr
        ? payload.stderr.split('\n').filter(Boolean).map((line: string) => ({
            level: 'error',
            source: 'stderr',
            message: line,
            timestamp: new Date().toISOString(),
          }))
        : []),
      ...(payload.stdout
        ? payload.stdout.split('\n').filter(Boolean).slice(-80).map((line: string) => ({
            level: 'debug',
            source: 'stdout',
            message: line,
            timestamp: new Date().toISOString(),
          }))
        : []),
    ];

    const toolResponse: ToolResponse = {
      success: Boolean(payload.ok),
      data: logs,
      error: payload.ok ? undefined : payload.stderr || `Exit ${payload.exitCode}`,
      ui: {
        viewType: 'log-viewer',
        title: 'Codex CLI',
        description: 'Command output from /codex execution',
        metadata: {
          source: 'codex-cli',
          itemCount: logs.length,
          duration: `${Date.now() - startedAt}ms`,
          exitCode: payload.exitCode,
          cwd: payload.cwd,
          timestamp: new Date().toISOString(),
        },
      },
    };

    addResult({
      query: `/codex ${prompt}`,
      response: toolResponse,
    });
    setResults(getAllResults());
    setPinnedResults(getPinnedResults());
    setQuery('');
    setPlan(null);
    addRecentCommand(`/codex ${prompt}`);
  };

  const processQuery = async (q: string) => {
    setLoading(true);
    setError(null);

    try {
      if (q.trim().toLowerCase().startsWith('/codex ')) {
        await runCodexCli(q.trim().slice('/codex '.length));
        return;
      }

      const appCtx: AppContext = {
        currentRepo: context.currentRepo,
        currentBranch: context.currentBranch,
        currentHost: context.currentHost,
        currentProject: context.currentProject,
        recentFiles: context.recentFiles,
        recentCommands: context.recentCommands,
        activeIntegrations: context.activeIntegrations,
        environment: context.environment,
        openWindows: context.openWindows,
      };

      const result = await intentProcessor.process(q, appCtx);
      setPlan(result);
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Processing failed';
      if (msg.includes('LLM provider') || msg.includes('No LLM')) {
        // Check if Qwen OAuth is available
        const qwenToken = localStorage.getItem('qwen_token');
        const hasQwen = qwenToken && (() => {
          try {
            const data = JSON.parse(qwenToken);
            return data.access_token && Date.now() < data.expiry_date;
          } catch {
            return false;
          }
        })();

        if (hasQwen) {
          setError('Qwen connected. Processing with Qwen...');
          // Retry with Qwen provider
          try {
            const qwenProvider = {
              id: 'qwen-oauth',
              name: 'Qwen Code',
              type: 'qwen' as const,
              baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
              apiKey: JSON.parse(qwenToken).access_token,
              defaultModel: 'qwen-plus',
              availableModels: ['qwen-plus', 'qwen-turbo', 'qwen-max'],
              enabled: true,
              priority: 1,
            };
            llmRouter.addProvider(qwenProvider);
            const appCtx: AppContext = {
              currentRepo: context.currentRepo,
              currentBranch: context.currentBranch,
              currentHost: context.currentHost,
              currentProject: context.currentProject,
              recentFiles: context.recentFiles,
              recentCommands: context.recentCommands,
              activeIntegrations: context.activeIntegrations,
              environment: context.environment,
              openWindows: context.openWindows,
            };
            const result = await intentProcessor.process(q, appCtx);
            setPlan(result);
            return;
          } catch (retryError) {
            setError('Qwen request failed. Please try again.');
          }
        } else {
          setError('AI commands require an LLM. Use Qwen OAuth or configure in settings.');
        }
        
        setPlan({
          id: 'llm-prompt',
          intent: { raw: q, verb: '', target: '', modifiers: [], context: context, confidence: 0 },
          steps: [],
          risk: 'low',
          preview: [],
          requiresAuth: true,
          predictedResult: 'Configure LLM in settings'
        });
      } else {
        setError(msg);
        setPlan(null);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleExecute = async () => {
    const p = plan();
    if (!p) return;

    setExecuting(true);
    setError(null);

    try {
      const result = await intentExecutor.execute(p);

      if (result.success) {
        addRecentCommand(query());

        // Save all tool responses to history
        if (result.responses && result.responses.length > 0) {
          for (const response of result.responses) {
            addResult({
              query: query(),
              response,
            });
          }
        } else {
          // Fallback: create a summary response
          const toolResponse: ToolResponse = {
            success: true,
            data: result,
            ui: {
              viewType: 'preview',
              title: p.intent.raw,
              description: p.predictedResult,
              metadata: {
                timestamp: new Date().toISOString(),
                source: 'Intent Execution',
              },
            },
          };
          addResult({
            query: query(),
            response: toolResponse,
          });
        }

        // Refresh results
        setResults(getAllResults());
        setPinnedResults(getPinnedResults());

        setQuery('');
        setPlan(null);
      } else {
        setError(result.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Execution failed');
    } finally {
      setExecuting(false);
    }
  };

  const handleShellMode = () => {
    openWindow('terminal');

    if (recognition && !listening()) {
      recognition.start();
      setListening(true);
    }

    setQuery('');
    setMode('shell');
  };

  const setupSpeechRecognition = () => {
    const SpeechRecognition: any = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    if (!SpeechRecognition) {
      console.warn('Speech recognition not supported');
      return;
    }

    const rec = new SpeechRecognition();
    recognition = rec;
    rec.continuous = true;
    rec.interimResults = true;

    rec.onresult = (event: any) => {
      const results = event.results;
      const latest = results[results.length - 1];
      const transcript = latest[0].transcript;

      window.dispatchEvent(new CustomEvent('intent:terminal:input', {
        detail: { text: transcript, final: latest.isFinal }
      }));

      if (mode() === 'intent' && latest.isFinal) {
        setQuery(prev => prev + ' ' + transcript);
      }
    };

    rec.onerror = (event: any) => {
      console.error('Speech recognition error:', event.error);
      setListening(false);
    };

    rec.onend = () => {
      setListening(false);
    };
  };

  const setupMCPMessageHandling = () => {
    // mcpMainThreadHandler already listens to window messages
    // We just need to handle specific tool responses for UI actions

    window.addEventListener('message', (event) => {
      const data = event.data;
      if (!data?.type?.startsWith('tool:')) return;

      // Handle window management tools directly
      switch (data.type) {
        case 'tool:open_window':
          openWindow(data.params.windowId);
          addOpenWindow(data.params.windowId);
          break;

        case 'tool:close_window':
          closeWindow(data.params.windowId);
          removeOpenWindow(data.params.windowId);
          break;

        case 'tool:send_to_terminal':
          window.dispatchEvent(new CustomEvent('intent:terminal:input', {
            detail: {
              text: data.params.text,
              execute: data.params.execute
            }
          }));
          break;

        // Other tool messages are handled by mcpMainThreadHandler
        // and responses are sent back to workers via:
        // - context:response
        // - files:response
        // - file:response
        // - search:response
        // - logs:response
      }
    });
  };

  const toggleListening = () => {
    if (!recognition) return;

    if (listening()) {
      recognition.stop();
    } else {
      recognition.start();
      setListening(true);
    }
  };

  const handleFileUpload = (e: Event) => {
    const target = e.target as HTMLInputElement;
    const files = target.files;
    if (files && files.length > 0) {
      setUploadedFiles(prev => [...prev, ...Array.from(files)]);
    }
    target.value = '';
  };

  const removeUploadedFile = (index: number) => {
    setUploadedFiles(prev => prev.filter((_, i) => i !== index));
  };

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' });
  };

  const formatDate = (date: Date) => {
    return date.toLocaleDateString('en-GB', { weekday: 'short', month: 'short', day: 'numeric' });
  };

  const getWeatherIcon = (condition: string, size = 20) => {
    switch (condition) {
      case 'sunny':
        return <TbOutlineSun size={size} class="text-yellow-400" />;
      case 'cloudy':
        return <CloudIcon size={size} class="text-gray-400" />;
      case 'rainy':
      case 'stormy':
        return <TbOutlineCloudRain size={size} class="text-blue-400" />;
      default:
        return <TbOutlineSun size={size} class="text-yellow-400" />;
    }
  };

  const triggerFileUpload = () => {
    fileInputRef?.click();
  };

  return (
    <Motion.div
      initial={{ y: -100, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: 0.4, easing: [0.33, 1, 0.68, 1] }}
      class="fixed top-0 left-0 right-0 z-[10003] flex flex-col items-center px-4 pt-4"
    >
      {/* Main Bar Container */}
      <div class="w-full max-w-3xl bg-[#1a1a1a]/90 backdrop-blur-xl rounded-2xl border border-neutral-700/50 shadow-2xl overflow-hidden transition-all duration-500">
        {/* Top Bar - Clock, Weather, Status */}
        <div class="flex items-center justify-between px-4 py-2 border-b border-neutral-800/50">
          {/* Left - Time */}
          <Motion.div
            class="flex items-center gap-3"
            initial={{ x: -20, opacity: 0 }}
            animate={{ x: 0, opacity: 1 }}
            transition={{ delay: 0.1 }}
          >
            <div class="flex items-center gap-2 text-white">
              <TbOutlineClock size={16} class="text-neutral-400" />
              <span class="text-sm font-medium tabular-nums">{formatTime(currentTime())}</span>
              <span class="text-xs text-neutral-500">{formatDate(currentTime())}</span>
            </div>
          </Motion.div>

          {/* Center - Mode Indicator */}
          <Show when={mode() !== 'intent'}>
            <Motion.div
              initial={{ scale: 0.8, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.8, opacity: 0 }}
              class="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-neutral-800/50 border border-neutral-700"
            >
              <Show
                when={mode() === 'shell'}
                fallback={<TbOutlineFilter size={14} class="text-blue-400" />}
              >
                <TbOutlineTerminal size={14} class="text-green-400" />
              </Show>
              <span class="text-xs text-neutral-300 capitalize">{mode()}</span>
            </Motion.div>
          </Show>

          {/* Right - Weather */}
          <Motion.div
            class="flex items-center gap-3"
            initial={{ x: 20, opacity: 0 }}
            animate={{ x: 0, opacity: 1 }}
            transition={{ delay: 0.1 }}
          >
            <div class="flex items-center gap-2 text-neutral-300">
              {getWeatherIcon(weather().condition, 18)}
              <span class="text-sm font-medium">{weather().temp}°</span>
              <span class="text-xs text-neutral-500 hidden sm:inline">{weather().location}</span>
            </div>
          </Motion.div>
        </div>

        {/* Input Area */}
        <div class="p-4">
          <div class="flex items-center gap-3">
            <Show
              when={mode() === 'shell'}
              fallback={
                <Motion.div
                  animate={{ rotate: loading() ? 360 : 0 }}
                  transition={{ duration: 2, repeat: loading() ? Infinity : 0, easing: "linear" }}
                >
                  <TbOutlineCommand size={20} class="text-blue-400" />
                </Motion.div>
              }
            >
              <TbOutlineTerminal size={20} class="text-green-400" />
            </Show>

            <form onSubmit={async e => {
              e.preventDefault();
              const q = query();
              if (q.trim()) {
                await processQuery(q);
              }
            }} class="flex-1 flex items-center gap-2">
              <input
                ref={inputRef}
                type="text"
                value={query()}
                onInput={e => {
                  setQuery(e.currentTarget.value);
                  setMode('intent');
                }}
                placeholder={
                  mode() === 'shell' ? 'Voice input active...' :
                  mode() === 'files' ? 'Search files...' :
                  mode() === 'email' ? 'Search emails...' :
                  mode() === 'logs' ? 'Search logs...' :
                  mode() === 'cloud' ? 'Search cloud resources...' :
                  'What do you want to do? (Press Enter to send)'
                }
                class="flex-1 bg-transparent border-none outline-none text-white placeholder-neutral-500 text-base"
              />
            </form>

            {/* File Upload */}
            <input
              ref={fileInputRef}
              type="file"
              multiple
              onChange={handleFileUpload}
              class="hidden"
            />
            <Motion.button
              type="button"
              onClick={triggerFileUpload}
              hover={{ scale: 1.1 }}
              press={{ scale: 0.95 }}
              class="p-2 text-neutral-400 hover:text-white hover:bg-neutral-800 rounded-lg transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900"
              title="Upload files"
              aria-label="Upload files"
            >
              <TbOutlineUpload size={18} />
            </Motion.button>

            {/* Voice Input */}
            <Motion.button
              type="button"
              onClick={toggleListening}
              hover={{ scale: 1.1 }}
              press={{ scale: 0.95 }}
              class={cn(
                "p-2 rounded-lg transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
                listening()
                  ? "text-red-400 bg-red-900/20 animate-pulse"
                  : "text-neutral-400 hover:text-white hover:bg-neutral-800"
              )}
              title="Voice input"
              aria-label={listening() ? 'Stop voice input' : 'Start voice input'}
              aria-pressed={listening()}
            >
              <TbOutlineMicrophone size={18} />
            </Motion.button>

            <Show when={query() || uploadedFiles().length > 0}>
              <Motion.button
                type="button"
                onClick={() => {
                  setQuery('');
                  setUploadedFiles([]);
                  setPlan(null);
                  setFilterResults([]);
                }}
                initial={{ scale: 0 }}
                animate={{ scale: 1 }}
                exit={{ scale: 0 }}
                hover={{ scale: 1.1 }}
                press={{ scale: 0.95 }}
                class="p-2 text-neutral-500 hover:text-white hover:bg-neutral-800 rounded-lg transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900"
                aria-label="Clear query"
              >
                <TbOutlineX size={18} />
              </Motion.button>
            </Show>
          </div>

          {/* Uploaded Files */}
          <Show when={uploadedFiles().length > 0}>
            <Motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              class="flex flex-wrap gap-2 mt-3 pt-3 border-t border-neutral-800"
            >
              <For each={uploadedFiles()}>
                {(file, index) => (
                  <Motion.div
                    initial={{ scale: 0.8, opacity: 0 }}
                    animate={{ scale: 1, opacity: 1 }}
                    exit={{ scale: 0.8, opacity: 0 }}
                    class="flex items-center gap-2 px-3 py-1.5 bg-neutral-800 rounded-lg text-sm text-neutral-300"
                  >
                    <TbOutlineFile size={14} class="text-blue-400" />
                    <span class="truncate max-w-[150px]">{file.name}</span>
                    <button
                      type="button"
                      onClick={() => removeUploadedFile(index())}
                      class="text-neutral-500 hover:text-red-400"
                    >
                      <TbOutlineX size={14} />
                    </button>
                  </Motion.div>
                )}
              </For>
            </Motion.div>
          </Show>
        </div>

        {/* Execution Plan */}
        <Show when={plan()}>
          <Motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3 }}
            class="px-4 pb-3 border-b border-neutral-800"
          >
            <div class="flex items-center gap-2 mb-3">
              <span class="text-xs text-neutral-400">Intent:</span>
              <span class="text-sm text-white">{plan()?.intent.raw}</span>
              <span class={cn(
                "text-xs px-2 py-0.5 rounded-full",
                plan()!.risk === 'low' && 'bg-green-900/30 text-green-300',
                plan()!.risk === 'medium' && 'bg-yellow-900/30 text-yellow-300',
                plan()!.risk === 'high' && 'bg-red-900/30 text-red-300'
              )}>
                {plan()!.risk}
              </span>
            </div>

            <Show when={plan()?.preview?.length}>
              <div class="space-y-1.5 mb-3">
                <For each={plan()?.preview}>
                  {item => (
                    <div class="flex justify-between text-xs">
                      <span class="text-neutral-500">{item.label}</span>
                      <span class={item.type === 'danger' ? 'text-red-400' : 'text-neutral-300'}>
                        {item.value}
                      </span>
                    </div>
                  )}
                </For>
              </div>
            </Show>

            <div class="flex items-center justify-between">
              <span class="text-xs text-neutral-500">{plan()?.predictedResult}</span>
              <Motion.button
                type="button"
                onClick={() => plan()?.requiresAuth ? openWindow('settings') : handleExecute()}
                disabled={executing()}
                hover={{ scale: executing() ? 1 : 1.02 }}
                press={{ scale: executing() ? 1 : 0.98 }}
                class={cn(
                  "px-4 py-1.5 rounded-lg text-sm font-medium transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
                  executing()
                    ? 'bg-neutral-700 text-neutral-400 cursor-not-allowed'
                    : plan()?.requiresAuth
                    ? 'bg-orange-600 hover:bg-orange-500 text-white'
                    : 'bg-blue-600 hover:bg-blue-500 text-white'
                )}
                aria-label={executing() ? 'Executing plan' : (plan()?.requiresAuth ? 'Setup required' : 'Execute plan')}
              >
                {executing() ? 'Executing...' : plan()?.requiresAuth ? 'Setup Required' : 'Run'}
              </Motion.button>
            </div>
          </Motion.div>
        </Show>

        {/* Filter Results */}
        <Show when={filterResults().length > 0}>
          <Motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            class="px-4 pb-3 border-b border-neutral-800"
          >
            <div class="flex items-center gap-2 mb-2">
              <TbOutlineSearch size={14} class="text-blue-400" />
              <span class="text-xs text-neutral-400">Found {filterResults().length} results</span>
            </div>
            <div class="space-y-1 max-h-40 overflow-y-auto">
              <For each={filterResults()}>
                {(result) => (
                  <div class="flex items-center gap-2 text-sm p-2 hover:bg-neutral-800 rounded-lg cursor-pointer transition-colors">
                    <Show
                      when={result.type === 'files'}
                      fallback={<TbOutlineFolder size={16} class="text-neutral-400" />}
                    >
                      <TbOutlineFile size={16} class="text-blue-400" />
                    </Show>
                    <span class="text-neutral-300 flex-1 truncate">{result.name}</span>
                    <span class="text-xs text-neutral-500">{result.path}</span>
                  </div>
                )}
              </For>
            </div>
          </Motion.div>
        </Show>

        {/* Error State */}
        <Show when={error()}>
          <Motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            class="px-4 py-2 bg-red-900/20 border-b border-red-900/30 flex items-center justify-between"
          >
            <p class="text-xs text-red-300">{error()}</p>
            <button
              type="button"
              onClick={() => openWindow('settings')}
              class="text-xs text-red-200 hover:text-white underline cursor-pointer focus:outline-none focus:ring-2 focus:ring-red-500 rounded"
              aria-label="Open settings to fix authentication"
            >
              Setup
            </button>
          </Motion.div>
        </Show>

        {/* Quick Actions */}
        <div class="px-4 py-3 flex items-center gap-2 overflow-x-auto">
          <span class="text-xs text-neutral-500 whitespace-nowrap">Quick:</span>

          <For each={filterPresets}>
            {(preset) => (
              <Motion.button
                type="button"
                onClick={() => {
                  setMode(preset.id as any);
                  setQuery(`/${preset.id} `);
                  inputRef?.focus();
                }}
                hover={{ scale: 1.05 }}
                press={{ scale: 0.95 }}
                class={cn(
                  "flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
                  mode() === preset.id
                    ? 'bg-neutral-700 text-white'
                    : 'text-neutral-400 hover:text-white hover:bg-neutral-800'
                )}
                aria-label={`Switch to ${preset.id} mode`}
              >
                <preset.icon size={14} class={preset.color} />
                {preset.label}
              </Motion.button>
            )}
          </For>

          <div class="flex-1" />

          <Motion.button
            type="button"
            onClick={() => openWindow('github')}
            hover={{ scale: 1.05 }}
            press={{ scale: 0.95 }}
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs text-neutral-400 hover:text-white hover:bg-neutral-800 rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900"
            aria-label="Open GitHub"
          >
            <FiGithub size={14} />
            GitHub
          </Motion.button>

          <Motion.button
            type="button"
            onClick={() => openWindow('cloudflare')}
            hover={{ scale: 1.05 }}
            press={{ scale: 0.95 }}
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs text-neutral-400 hover:text-white hover:bg-neutral-800 rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900"
            aria-label="Open Cloudflare"
          >
            <FiGlobe size={14} />
            Cloudflare
          </Motion.button>

          <Motion.button
            type="button"
            onClick={() => openWindow('settings')}
            hover={{ scale: 1.05 }}
            press={{ scale: 0.95 }}
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs text-neutral-400 hover:text-white hover:bg-neutral-800 rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900"
            aria-label="Open settings"
          >
            <FiSettings size={14} />
            Settings
          </Motion.button>

          {/* History Toggle */}
          <Motion.button
            type="button"
            onClick={() => setShowHistory(!showHistory())}
            hover={{ scale: 1.05 }}
            press={{ scale: 0.95 }}
            class={cn(
              "flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg transition-colors whitespace-nowrap cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
              showHistory()
                ? 'bg-blue-600 text-white'
                : 'text-neutral-400 hover:text-white hover:bg-neutral-800'
            )}
            aria-label={showHistory() ? 'Close history' : 'Open history'}
            aria-pressed={showHistory()}
          >
            <TbOutlineHistory size={14} />
            History
            <Show when={results().length > 0}>
              <span class="ml-1 px-1.5 py-0.5 bg-white/20 rounded-full text-[10px]">
                {results().length}
              </span>
            </Show>
          </Motion.button>
        </div>

        {/* Result History Panel */}
        <Show when={showHistory()}>
          <Motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            class="border-t border-neutral-800"
          >
            <div class="px-4 py-3 flex items-center justify-between">
              <div class="flex items-center gap-3">
                <span class="text-xs font-medium text-neutral-400">Result History</span>
                <Show when={pinnedResults().length > 0}>
                  <span class="text-xs text-neutral-500">•</span>
                  <span class="text-xs text-neutral-500">{pinnedResults().length} pinned</span>
                </Show>
              </div>
              <div class="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => {
                    clearResults();
                    setResults(getAllResults());
                    setPinnedResults(getPinnedResults());
                  }}
                  class="text-xs text-neutral-500 hover:text-red-400 transition-colors flex items-center gap-1"
                >
                  <TbOutlineTrash size={12} />
                  Clear
                </button>
              </div>
            </div>

            <div class="px-4 pb-4 space-y-3 max-h-[400px] overflow-y-auto">
              {/* Pinned Results First */}
              <For each={pinnedResults()}>
                {(result) => (
                  <div class="relative group">
                    <ResultRenderer
                      response={result.response}
                      onAction={(intent) => {
                        setQuery(intent);
                        inputRef?.focus();
                      }}
                    />
                    <div class="absolute top-2 right-2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
                        type="button"
                        onClick={() => {
                          pinResult(result.id, false);
                          setPinnedResults(getPinnedResults());
                        }}
                        class="p-1 text-neutral-400 hover:text-white bg-neutral-800 rounded"
                        title="Unpin"
                      >
                        <TbOutlinePin size={14} />
                      </button>
                      <button
                        type="button"
                        onClick={() => {
                          removeResult(result.id);
                          setResults(getAllResults());
                          setPinnedResults(getPinnedResults());
                        }}
                        class="p-1 text-neutral-400 hover:text-red-400 bg-neutral-800 rounded"
                        title="Remove"
                      >
                        <TbOutlineTrash size={14} />
                      </button>
                    </div>
                  </div>
                )}
              </For>

              {/* Regular Results */}
              <For each={results().filter(r => !r.isPinned)}>
                {(result) => (
                  <div class="relative group">
                    <ResultRenderer
                      response={result.response}
                      onAction={(intent) => {
                        setQuery(intent);
                        inputRef?.focus();
                      }}
                    />
                    <div class="absolute top-2 right-2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
                        type="button"
                        onClick={() => {
                          pinResult(result.id, true);
                          setPinnedResults(getPinnedResults());
                        }}
                        class="p-1 text-neutral-400 hover:text-blue-400 bg-neutral-800 rounded"
                        title="Pin"
                      >
                        <TbOutlinePin size={14} />
                      </button>
                      <button
                        type="button"
                        onClick={() => {
                          removeResult(result.id);
                          setResults(getAllResults());
                          setPinnedResults(getPinnedResults());
                        }}
                        class="p-1 text-neutral-400 hover:text-red-400 bg-neutral-800 rounded"
                        title="Remove"
                      >
                        <TbOutlineTrash size={14} />
                      </button>
                    </div>
                  </div>
                )}
              </For>

              {/* Empty State */}
              <Show when={results().length === 0}>
                <div class="text-center py-8 text-neutral-500">
                  <TbOutlineHistory size={32} class="mx-auto mb-2 opacity-50" />
                  <p class="text-sm">No results yet</p>
                  <p class="text-xs mt-1">Your query results will appear here</p>
                </div>
              </Show>
            </div>
          </Motion.div>
        </Show>
      </div>
    </Motion.div>
  );
}
