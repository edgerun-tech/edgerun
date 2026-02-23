/**
 * Log Viewer
 * Terminal-style log output with filtering and auto-scroll
 */

import { createSignal, createEffect, onMount, onCleanup, Show, For } from 'solid-js';
import { Motion } from 'solid-motionone';
import type { ToolResponse } from '../../lib/mcp/types';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { TbOutlineFilter, TbOutlineX, TbOutlineDownload, TbOutlineTrash } from 'solid-icons/tb';

function cn(...classes: ClassValue[]) {
  return twMerge(clsx(classes));
}

interface LogEntry {
  timestamp?: string;
  level?: 'info' | 'warn' | 'error' | 'debug';
  message: string;
  source?: string;
}

interface LogViewerProps {
  response: ToolResponse;
  class?: string;
}

function getLevelColor(level: string) {
  switch (level) {
    case 'error': return 'text-red-400 bg-red-900/20';
    case 'warn': return 'text-yellow-400 bg-yellow-900/20';
    case 'debug': return 'text-purple-400 bg-purple-900/20';
    default: return 'text-green-400 bg-green-900/20';
  }
}

function parseLogData(data: any): LogEntry[] {
  if (Array.isArray(data)) {
    return data.map(entry => {
      if (typeof entry === 'string') {
        return { message: entry };
      }
      return entry as LogEntry;
    });
  }
  
  // If it's a string, split by newlines
  if (typeof data === 'string') {
    return data.split('\n').filter(line => line.trim()).map(line => ({
      message: line,
    }));
  }
  
  return [{ message: JSON.stringify(data, null, 2) }];
}

export function LogViewer(props: LogViewerProps) {
  const ui = () => props.response.ui;
  const [filter, setFilter] = createSignal('');
  const [levelFilter, setLevelFilter] = createSignal<string | null>(null);
  const [autoScroll, setAutoScroll] = createSignal(true);
  const [logs, setLogs] = createSignal<LogEntry[]>([]);
  let logContainerRef: HTMLDivElement | undefined;

  // Parse logs on mount
  onMount(() => {
    const parsed = parseLogData(props.response.data);
    setLogs(parsed);
  });

  // Auto-scroll to bottom when logs change
  createEffect(() => {
    if (autoScroll() && logContainerRef) {
      logContainerRef.scrollTop = logContainerRef.scrollHeight;
    }
  });

  // Filtered logs
  const filteredLogs = () => {
    let result = logs();
    
    if (filter()) {
      const query = filter().toLowerCase();
      result = result.filter(log => 
        log.message.toLowerCase().includes(query) ||
        log.source?.toLowerCase().includes(query)
      );
    }
    
    if (levelFilter()) {
      result = result.filter(log => log.level === levelFilter());
    }
    
    return result;
  };

  const clearLogs = () => {
    setLogs([]);
  };

  const downloadLogs = () => {
    const content = logs().map(l => 
      `[${l.timestamp || 'NOW'}] ${l.level?.toUpperCase() || 'INFO'}: ${l.message}`
    ).join('\n');
    
    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `logs-${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <Motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -8 }}
      transition={{ duration: 0.2 }}
      class={cn(
        "bg-neutral-800/50 rounded-xl border border-neutral-700 overflow-hidden",
        props.class
      )}
    >
      {/* Header */}
      <div class="px-4 py-3 border-b border-neutral-700 bg-neutral-800/50">
        <div class="flex items-center justify-between gap-3 flex-wrap">
          <div class="flex items-center gap-3">
            <Show when={ui()?.title}>
              <h3 class="text-sm font-medium text-white">{ui()!.title}</h3>
            </Show>
            <Show when={ui()?.metadata?.itemCount}>
              <span class="text-xs text-neutral-500">
                {filteredLogs().length} / {ui()!.metadata!.itemCount} lines
              </span>
            </Show>
          </div>
          
          <div class="flex items-center gap-2" role="group" aria-label="Log controls">
            {/* Level filters */}
            <div class="flex items-center gap-1" role="group" aria-label="Filter by log level">
              {['info', 'warn', 'error', 'debug'].map(level => (
                <button
                  type="button"
                  onClick={() => setLevelFilter(levelFilter() === level ? null : level)}
                  class={cn(
                    "px-2 py-1 rounded text-xs font-medium transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800",
                    levelFilter() === level
                      ? getLevelColor(level)
                      : "text-neutral-500 hover:text-neutral-300 hover:bg-neutral-700"
                  )}
                  aria-pressed={levelFilter() === level}
                >
                  {level.toUpperCase()}
                </button>
              ))}
            </div>

            <div class="h-4 w-px bg-neutral-700" />

            {/* Actions */}
            <button
              type="button"
              onClick={downloadLogs}
              class="p-1.5 text-neutral-400 hover:text-white hover:bg-neutral-700 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800"
              title="Download logs"
              aria-label="Download logs"
            >
              <TbOutlineDownload size={16} />
            </button>
            <button
              type="button"
              onClick={clearLogs}
              class="p-1.5 text-neutral-400 hover:text-red-400 hover:bg-neutral-700 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 focus:ring-offset-neutral-800"
              title="Clear logs"
              aria-label="Clear logs"
            >
              <TbOutlineTrash size={16} />
            </button>
          </div>
        </div>
        
        {/* Search filter */}
        <div class="mt-3 flex items-center gap-2">
          <div class="relative flex-1">
            <TbOutlineFilter size={14} class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500" />
            <input
              type="text"
              value={filter()}
              onInput={e => setFilter(e.currentTarget.value)}
              placeholder="Filter logs..."
              class="w-full pl-9 pr-8 py-1.5 bg-neutral-900 border border-neutral-700 rounded-lg text-sm text-neutral-300 placeholder-neutral-500 focus:outline-none focus:border-neutral-600"
            />
            <Show when={filter()}>
              <button
                type="button"
                onClick={() => setFilter('')}
                class="absolute right-2 top-1/2 -translate-y-1/2 text-neutral-500 hover:text-white"
              >
                <TbOutlineX size={14} />
              </button>
            </Show>
          </div>
          
          <label class="flex items-center gap-2 text-xs text-neutral-400 cursor-pointer">
            <input
              type="checkbox"
              checked={autoScroll()}
              onChange={e => setAutoScroll(e.currentTarget.checked)}
              class="rounded border-neutral-700 bg-neutral-800 text-blue-600 focus:ring-blue-600"
            />
            Auto-scroll
          </label>
        </div>
      </div>

      {/* Log content */}
      <div
        ref={logContainerRef}
        class="p-4 font-mono text-sm bg-neutral-900/50 overflow-auto max-h-[500px]"
      >
        <For each={filteredLogs()}>
          {(log) => (
            <div class="flex items-start gap-3 py-0.5 hover:bg-neutral-800/50 rounded px-2">
              <Show when={log.timestamp}>
                <span class="text-neutral-500 whitespace-nowrap">[{log.timestamp}]</span>
              </Show>
              <Show when={log.level}>
                <span class={cn("px-1.5 py-0.5 rounded text-xs font-medium", getLevelColor(log.level!))}>
                  {log.level!.toUpperCase()}
                </span>
              </Show>
              <Show when={log.source}>
                <span class="text-blue-400 whitespace-nowrap">{log.source}:</span>
              </Show>
              <span class="text-neutral-300 flex-1 break-all">{log.message}</span>
            </div>
          )}
        </For>
        
        <Show when={filteredLogs().length === 0}>
          <div class="text-center py-8 text-neutral-500">
            {logs().length === 0 ? 'No logs available' : 'No matching logs'}
          </div>
        </Show>
      </div>
    </Motion.div>
  );
}
