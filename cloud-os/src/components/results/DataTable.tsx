/**
 * Data Table
 * Tabular view for structured data
 */

import { Show, For, createSignal, createMemo } from 'solid-js';
import { Motion } from 'solid-motionone';
import type { ToolResponse } from '../../lib/mcp/types';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { TbOutlineSortAscending, TbOutlineSortDescending } from 'solid-icons/tb';

function cn(...classes: ClassValue[]) {
  return twMerge(clsx(classes));
}

interface DataTableProps {
  response: ToolResponse;
  class?: string;
}

interface SortConfig {
  key: string;
  direction: 'asc' | 'desc';
}

export function DataTable(props: DataTableProps) {
  const ui = () => props.response.ui;
  const [sortConfig, setSortConfig] = createSignal<SortConfig | null>(null);
  const [searchQuery, setSearchQuery] = createSignal('');

  // Extract columns and rows from data
  const dataStruct = createMemo(() => {
    const data = props.response.data;
    
    if (!Array.isArray(data) || data.length === 0) {
      return { columns: [], rows: [] };
    }

    // Get all unique keys from first 10 items
    const allKeys = new Set<string>();
    data.slice(0, 10).forEach(item => {
      if (typeof item === 'object' && item !== null) {
        Object.keys(item).forEach(key => allKeys.add(key));
      }
    });

    return {
      columns: Array.from(allKeys),
      rows: data,
    };
  });
  
  const columns = () => dataStruct().columns;
  const rows = () => dataStruct().rows;

  // Sort and filter rows
  const processedRows = createMemo(() => {
    let result = [...rows()];

    // Filter
    const query = searchQuery().toLowerCase();
    if (query) {
      result = result.filter(row => 
        Object.values(row as any).some(val => 
          String(val).toLowerCase().includes(query)
        )
      );
    }

    // Sort
    const sort = sortConfig();
    if (sort) {
      result.sort((a, b) => {
        const aVal = (a as any)[sort.key];
        const bVal = (b as any)[sort.key];
        
        if (aVal < bVal) return sort.direction === 'asc' ? -1 : 1;
        if (aVal > bVal) return sort.direction === 'asc' ? 1 : -1;
        return 0;
      });
    }

    return result;
  });

  const handleSort = (key: string) => {
    setSortConfig(prev => {
      if (prev?.key === key) {
        return prev.direction === 'asc' 
          ? { key, direction: 'desc' }
          : null;
      }
      return { key, direction: 'asc' };
    });
  };

  const formatValue = (value: any) => {
    if (value === null || value === undefined) return '-';
    if (typeof value === 'boolean') return value ? '✓' : '✗';
    if (typeof value === 'object') return JSON.stringify(value);
    return String(value);
  };

  const getStatusColor = (value: any) => {
    const str = String(value).toLowerCase();
    if (['active', 'success', 'ready', 'confirmed'].includes(str)) return 'text-green-400';
    if (['error', 'failed', 'inactive'].includes(str)) return 'text-red-400';
    if (['pending', 'processing', 'warning'].includes(str)) return 'text-yellow-400';
    return 'text-neutral-300';
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
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-3">
            <Show when={ui()?.title}>
              <h3 class="text-sm font-medium text-white">{ui()!.title}</h3>
            </Show>
            <Show when={processedRows().length}>
              <span class="text-xs text-neutral-500">
                {processedRows().length} rows
              </span>
            </Show>
          </div>
          
          {/* Search */}
          <input
            type="text"
            value={searchQuery()}
            onInput={e => setSearchQuery(e.currentTarget.value)}
            placeholder="Search..."
            class="px-3 py-1.5 bg-neutral-900 border border-neutral-700 rounded-lg text-sm text-neutral-300 placeholder-neutral-500 focus:outline-none focus:border-neutral-600"
          />
        </div>
      </div>

      {/* Table */}
      <div class="overflow-auto max-h-[500px]">
        <table class="w-full text-sm">
          <thead class="bg-neutral-900/50 sticky top-0">
            <tr>
              <For each={columns()}>
                {(col) => (
                  <th
                    class="px-4 py-3 text-left text-xs font-medium text-neutral-400 uppercase tracking-wider cursor-pointer hover:text-white hover:bg-neutral-800 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                    onClick={() => handleSort(col)}
                    tabindex="0"
                    role="button"
                    aria-label={`Sort by ${col}`}
                    aria-sort={sortConfig()?.key === col ? (sortConfig()?.direction === 'asc' ? 'ascending' : 'descending') : 'none'}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        handleSort(col);
                      }
                    }}
                  >
                    <div class="flex items-center gap-1">
                      {col}
                      <Show when={sortConfig()?.key === col}>
                        {sortConfig()?.direction === 'asc'
                          ? <TbOutlineSortAscending size={14} />
                          : <TbOutlineSortDescending size={14} />
                        }
                      </Show>
                    </div>
                  </th>
                )}
              </For>
            </tr>
          </thead>
          <tbody class="divide-y divide-neutral-800">
            <For each={processedRows()}>
              {(row, index) => (
                <tr class={cn(
                  "hover:bg-neutral-800/30 transition-colors",
                  index() % 2 === 0 ? "bg-transparent" : "bg-neutral-900/20"
                )}>
                  <For each={columns()}>
                    {(col) => (
                      <td class="px-4 py-3 text-neutral-300">
                        <span class={getStatusColor((row as any)[col])}>
                          {formatValue((row as any)[col])}
                        </span>
                      </td>
                    )}
                  </For>
                </tr>
              )}
            </For>
          </tbody>
        </table>

        {/* Empty state */}
        <Show when={processedRows().length === 0}>
          <div class="p-8 text-center text-neutral-500">
            {rows().length === 0 ? 'No data available' : 'No matching results'}
          </div>
        </Show>
      </div>

      {/* Footer */}
      <Show when={ui()?.metadata}>
        <div class="px-4 py-3 bg-neutral-800/30 border-t border-neutral-700 text-xs text-neutral-500">
          <Show when={ui()!.metadata!.source}>
            Source: {ui()!.metadata!.source}
          </Show>
          <Show when={ui()!.metadata!.timestamp}>
            {' • '}Last updated: {ui()!.metadata!.timestamp}
          </Show>
        </div>
      </Show>
    </Motion.div>
  );
}
