/**
 * Timeline
 * Event sequences, deployment history, chronological data
 */

import { createSignal, Show, For, onMount } from 'solid-js';
import { Motion } from 'solid-motionone';
import type { ToolResponse } from '../../lib/mcp/types';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { 
  TbOutlineClock, TbOutlineCalendar, TbOutlineFilter,
  TbOutlineSortAscending, TbOutlineSortDescending 
} from 'solid-icons/tb';

function cn(...classes: ClassValue[]) {
  return twMerge(clsx(classes));
}

interface TimelineEvent {
  id?: string;
  timestamp: string;
  title: string;
  description?: string;
  type?: 'info' | 'success' | 'warning' | 'error' | 'deployment' | 'commit' | 'build';
  icon?: any;
  metadata?: Record<string, any>;
  actions?: Array<{ label: string; intent: string }>;
}

interface TimelineProps {
  response: ToolResponse;
  class?: string;
  onAction?: (intent: string) => void;
}

function getTypeColor(type: string) {
  switch (type) {
    case 'success': return 'bg-green-500 border-green-600';
    case 'error': return 'bg-red-500 border-red-600';
    case 'warning': return 'bg-yellow-500 border-yellow-600';
    case 'deployment': return 'bg-blue-500 border-blue-600';
    case 'commit': return 'bg-purple-500 border-purple-600';
    case 'build': return 'bg-orange-500 border-orange-600';
    default: return 'bg-neutral-500 border-neutral-600';
  }
}

function getTypeIcon(type: string) {
  switch (type) {
    case 'success': return '✓';
    case 'error': return '✗';
    case 'warning': return '⚠';
    case 'deployment': return '🚀';
    case 'commit': return '📝';
    case 'build': return '🔨';
    default: return '•';
  }
}

function parseTimelineData(data: any): TimelineEvent[] {
  if (Array.isArray(data)) {
    return data.map(item => {
      if (typeof item === 'string') {
        return {
          timestamp: new Date().toISOString(),
          title: item,
          type: 'info',
        };
      }
      return item as TimelineEvent;
    });
  }
  
  // If it's an object with events property
  if (data?.events && Array.isArray(data.events)) {
    return data.events;
  }
  
  return [];
}

export function Timeline(props: TimelineProps) {
  const ui = () => props.response.ui;
  const [filter, setFilter] = createSignal('');
  const [typeFilter, setTypeFilter] = createSignal<string | null>(null);
  const [sortOrder, setSortOrder] = createSignal<'asc' | 'desc'>('desc');
  const [events, setEvents] = createSignal<TimelineEvent[]>([]);

  // Parse events on mount
  onMount(() => {
    const parsed = parseTimelineData(props.response.data);
    setEvents(parsed);
  });

  // Filtered and sorted events
  const filteredEvents = () => {
    let result = [...events()];
    
    // Filter by type
    if (typeFilter()) {
      result = result.filter(e => e.type === typeFilter());
    }
    
    // Filter by search
    const query = filter().toLowerCase();
    if (query) {
      result = result.filter(e => 
        e.title.toLowerCase().includes(query) ||
        e.description?.toLowerCase().includes(query)
      );
    }
    
    // Sort
    result.sort((a, b) => {
      const aTime = new Date(a.timestamp).getTime();
      const bTime = new Date(b.timestamp).getTime();
      return sortOrder() === 'asc' ? aTime - bTime : bTime - aTime;
    });
    
    return result;
  };

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(hours / 24);
    
    if (hours < 1) return 'Just now';
    if (hours < 24) return `${hours}h ago`;
    if (days < 7) return `${days}d ago`;
    return date.toLocaleDateString();
  };

  const eventTypes = ['info', 'success', 'warning', 'error', 'deployment', 'commit', 'build'];

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
            <TbOutlineClock size={18} class="text-blue-400" />
            <Show when={ui()?.title}>
              <h3 class="text-sm font-medium text-white">{ui()!.title}</h3>
            </Show>
            <Show when={filteredEvents().length}>
              <span class="text-xs text-neutral-500">
                {filteredEvents().length} events
              </span>
            </Show>
          </div>
          
          <div class="flex items-center gap-2">
            {/* Sort toggle */}
            <button
              type="button"
              onClick={() => setSortOrder(sortOrder() === 'asc' ? 'desc' : 'asc')}
              class="p-1.5 text-neutral-400 hover:text-white hover:bg-neutral-700 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800"
              title={sortOrder() === 'asc' ? 'Sort ascending' : 'Sort descending'}
              aria-label={sortOrder() === 'asc' ? 'Sort ascending' : 'Sort descending'}
            >
              {sortOrder() === 'asc'
                ? <TbOutlineSortAscending size={16} />
                : <TbOutlineSortDescending size={16} />
              }
            </button>
          </div>
        </div>
        
        {/* Filters */}
        <div class="mt-3 flex items-center gap-2 flex-wrap" role="group" aria-label="Filter events">
          {/* Type filters */}
          <div class="flex items-center gap-1 flex-wrap" role="group" aria-label="Event type filters">
            <button
              type="button"
              onClick={() => setTypeFilter(null)}
              class={cn(
                "px-2 py-1 rounded text-xs font-medium transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800",
                !typeFilter()
                  ? 'bg-neutral-600 text-white'
                  : 'text-neutral-500 hover:text-neutral-300 hover:bg-neutral-700'
              )}
            >
              All
            </button>
            {eventTypes.map(type => (
              <button
                type="button"
                onClick={() => setTypeFilter(typeFilter() === type ? null : type)}
                class={cn(
                  "px-2 py-1 rounded text-xs font-medium transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800",
                  typeFilter() === type
                    ? getTypeColor(type) + ' text-white'
                    : 'text-neutral-500 hover:text-neutral-300 hover:bg-neutral-700'
                )}
                aria-pressed={typeFilter() === type}
              >
                {type}
              </button>
            ))}
          </div>
          
          <div class="h-4 w-px bg-neutral-700" />
          
          {/* Search */}
          <div class="relative flex-1 min-w-[200px]">
            <TbOutlineFilter size={14} class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500" />
            <input
              type="text"
              value={filter()}
              onInput={e => setFilter(e.currentTarget.value)}
              placeholder="Search events..."
              class="w-full pl-9 pr-4 py-1.5 bg-neutral-900 border border-neutral-700 rounded-lg text-sm text-neutral-300 placeholder-neutral-500 focus:outline-none focus:border-neutral-600"
            />
          </div>
        </div>
      </div>

      {/* Timeline content */}
      <div class="p-4">
        <div class="relative">
          {/* Timeline line */}
          <div class="absolute left-4 top-0 bottom-0 w-px bg-neutral-700" />
          
          {/* Events */}
          <div class="space-y-4">
            <For each={filteredEvents()}>
              {(event) => (
                <div class="relative flex gap-4">
                  {/* Icon */}
                  <div class={cn(
                    "flex-shrink-0 w-8 h-8 rounded-full border-2 flex items-center justify-center text-xs font-bold text-white z-10",
                    getTypeColor(event.type || 'info')
                  )}>
                    {getTypeIcon(event.type || 'info')}
                  </div>
                  
                  {/* Content */}
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2 mb-1">
                      <h4 class="text-sm font-medium text-white">{event.title}</h4>
                      <span class="text-xs text-neutral-500">{formatTimestamp(event.timestamp)}</span>
                    </div>
                    
                    <Show when={event.description}>
                      <p class="text-sm text-neutral-400">{event.description}</p>
                    </Show>
                    
                    {/* Metadata */}
                    <Show when={event.metadata}>
                      <div class="mt-2 flex flex-wrap gap-2">
                        <For each={Object.entries(event.metadata || {})}>
                          {([key, value]) => (
                            <span class="text-xs px-2 py-1 bg-neutral-900 rounded text-neutral-400">
                              {key}: {String(value)}
                            </span>
                          )}
                        </For>
                      </div>
                    </Show>
                    
                    {/* Actions */}
                    <Show when={event.actions?.length}>
                      <div class="mt-2 flex flex-wrap gap-2">
                        <For each={event.actions}>
                          {(action) => (
                            <button
                              type="button"
                              onClick={() => props.onAction?.(action.intent)}
                              class="text-xs px-3 py-1 bg-blue-600 hover:bg-blue-500 text-white rounded transition-colors"
                            >
                              {action.label}
                            </button>
                          )}
                        </For>
                      </div>
                    </Show>
                  </div>
                </div>
              )}
            </For>
          </div>
        </div>

        {/* Empty state */}
        <Show when={filteredEvents().length === 0}>
          <div class="text-center py-12 text-neutral-500">
            <TbOutlineCalendar size={48} class="mx-auto mb-3 opacity-50" />
            <p class="text-sm">No events found</p>
            <Show when={filter() || typeFilter()}>
              <p class="text-xs mt-1">Try adjusting your filters</p>
            </Show>
          </div>
        </Show>
      </div>
    </Motion.div>
  );
}
