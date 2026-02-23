/**
 * Preview Card - Default result view
 * Simple, clean summary of any result
 */

import { Show, For } from 'solid-js';
import { Motion } from 'solid-motionone';
import type { ToolResponse, ToolAction } from '../../lib/mcp/types';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

function cn(...classes: ClassValue[]) {
  return twMerge(clsx(classes));
}

interface PreviewCardProps {
  response: ToolResponse;
  onAction?: (intent: string) => void;
  class?: string;
}

export function PreviewCard(props: PreviewCardProps) {
  const ui = () => props.response.ui;
  
  // Format data for display
  const formattedData = () => {
    if (!props.response.data) return null;
    
    if (typeof props.response.data === 'string') {
      return props.response.data;
    }
    
    if (typeof props.response.data === 'object') {
      // Show first few key-value pairs
      const entries = Object.entries(props.response.data).slice(0, 5);
      return entries.map(([key, value]) => ({
        key,
        value: typeof value === 'object' ? JSON.stringify(value) : String(value),
      }));
    }
    
    return String(props.response.data);
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
      <Show when={ui()?.title || ui()?.metadata}>
        <div class="px-4 py-3 border-b border-neutral-700 bg-neutral-800/50">
          <div class="flex items-center justify-between">
            <Show when={ui()?.title}>
              <h3 class="text-sm font-medium text-white">{ui()!.title}</h3>
            </Show>
            <Show when={ui()?.metadata?.source}>
              <span class="text-xs text-neutral-400">{ui()!.metadata!.source}</span>
            </Show>
          </div>
          <Show when={ui()?.description}>
            <p class="text-xs text-neutral-500 mt-1">{ui()!.description}</p>
          </Show>
        </div>
      </Show>

      {/* Content */}
      <div class="p-4">
        <Show
          when={typeof formattedData() === 'string'}
          fallback={
            <div class="space-y-2">
              <For each={formattedData() as any[]}>
                {(item) => (
                  <div class="flex gap-2 text-sm">
                    <span class="text-neutral-500 min-w-[100px]">{item.key}:</span>
                    <span class="text-neutral-300 truncate">{item.value}</span>
                  </div>
                )}
              </For>
            </div>
          }
        >
          <p class="text-sm text-neutral-300 whitespace-pre-wrap">
            {formattedData() as string}
          </p>
        </Show>

        {/* Metadata badges */}
        <Show when={ui()?.metadata?.itemCount}>
          <div class="flex items-center gap-2 mt-3 pt-3 border-t border-neutral-700">
            <span class="text-xs text-neutral-500">
              {ui()!.metadata!.itemCount} items
            </span>
            <Show when={ui()?.metadata?.duration}>
              <span class="text-xs text-neutral-500">•</span>
              <span class="text-xs text-neutral-500">{ui()!.metadata!.duration}</span>
            </Show>
          </div>
        </Show>
      </div>

      {/* Actions */}
      <Show when={ui()?.actions?.length}>
        <div class="px-4 py-3 bg-neutral-800/30 border-t border-neutral-700 flex flex-wrap gap-2" role="group" aria-label="Actions">
          <For each={ui()?.actions}>
            {(action) => (
              <button
                type="button"
                onClick={() => props.onAction?.(action.intent)}
                class={cn(
                  "px-3 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800",
                  action.variant === 'primary' && "bg-blue-600 text-white hover:bg-blue-500",
                  action.variant === 'danger' && "bg-red-600 text-white hover:bg-red-500",
                  action.variant === 'ghost' && "bg-transparent text-neutral-400 hover:text-white hover:bg-neutral-700",
                  !action.variant || action.variant === 'secondary' && "bg-neutral-700 text-neutral-300 hover:bg-neutral-600"
                )}
              >
                {action.label}
              </button>
            )}
          </For>
        </div>
      </Show>
    </Motion.div>
  );
}
