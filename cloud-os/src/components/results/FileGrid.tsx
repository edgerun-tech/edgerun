/**
 * File Grid
 * Grid view for file search results with thumbnails and metadata
 */

import { Show, For, createSignal, onMount } from 'solid-js';
import { Motion } from 'solid-motionone';
import type { ToolResponse } from '../../lib/mcp/types';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import {
  TbOutlineFile, TbOutlineFolder, TbOutlinePhoto, TbOutlineFileText,
  TbOutlineSearch, TbOutlineLayoutGrid, TbOutlineList, TbOutlineDownload
} from 'solid-icons/tb';

function cn(...classes: ClassValue[]) {
  return twMerge(clsx(classes));
}

interface FileItem {
  id?: string;
  name: string;
  path?: string;
  type?: 'file' | 'folder';
  size?: number;
  modified?: string;
  mimeType?: string;
  url?: string;
  thumbnail?: string;
}

interface FileGridProps {
  response: ToolResponse;
  class?: string;
}

function getFileIcon(file: FileItem, size = 24) {
  if (file.type === 'folder') {
    return <TbOutlineFolder size={size} class="text-yellow-400" />;
  }
  
  const mime = file.mimeType || '';
  const ext = file.name.split('.').pop()?.toLowerCase();
  
  if (mime.startsWith('image/') || ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp'].includes(ext || '')) {
    return <TbOutlinePhoto size={size} class="text-purple-400" />;
  }
  if (mime.startsWith('text/') || ['txt', 'md', 'json', 'js', 'ts', 'tsx', 'jsx', 'css', 'html', 'py', 'rs'].includes(ext || '')) {
    return <TbOutlineFileText size={size} class="text-blue-400" />;
  }
  
  return <TbOutlineFile size={size} class="text-neutral-400" />;
}

function formatFileSize(bytes?: number): string {
  if (!bytes) return '--';
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function parseFileData(data: any): FileItem[] {
  if (Array.isArray(data)) {
    return data as FileItem[];
  }
  
  // If it's an object with files property
  if (data?.files && Array.isArray(data.files)) {
    return data.files;
  }
  
  return [];
}

export function FileGrid(props: FileGridProps) {
  const ui = () => props.response.ui;
  const [viewMode, setViewMode] = createSignal<'grid' | 'list'>('grid');
  const [searchQuery, setSearchQuery] = createSignal('');
  const [selectedFiles, setSelectedFiles] = createSignal<Set<string>>(new Set());
  const [files, setFiles] = createSignal<FileItem[]>([]);

  // Parse files on mount
  onMount(() => {
    const parsed = parseFileData(props.response.data);
    setFiles(parsed);
  });

  // Filtered files
  const filteredFiles = () => {
    const query = searchQuery().toLowerCase();
    if (!query) return files();
    
    return files().filter(file => 
      file.name.toLowerCase().includes(query) ||
      file.path?.toLowerCase().includes(query)
    );
  };

  const toggleSelect = (id: string) => {
    const current = selectedFiles();
    const newSet = new Set(current);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    setSelectedFiles(newSet);
  };

  const downloadFile = (file: FileItem) => {
    if (file.url) {
      window.open(file.url, '_blank');
    }
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
            <Show when={filteredFiles().length}>
              <span class="text-xs text-neutral-500">
                {filteredFiles().length} files
              </span>
            </Show>
          </div>
          
          <div class="flex items-center gap-2">
            {/* View mode toggle */}
            <div class="flex items-center gap-1 bg-neutral-900 rounded-lg p-1" role="group" aria-label="View mode">
              <button
                type="button"
                onClick={() => setViewMode('grid')}
                class={cn(
                  "p-1.5 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
                  viewMode() === 'grid'
                    ? 'bg-neutral-700 text-white'
                    : 'text-neutral-400 hover:text-white'
                )}
                aria-label="Grid view"
                aria-pressed={viewMode() === 'grid'}
              >
                <TbOutlineLayoutGrid size={16} />
              </button>
              <button
                type="button"
                onClick={() => setViewMode('list')}
                class={cn(
                  "p-1.5 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900",
                  viewMode() === 'list'
                    ? 'bg-neutral-700 text-white'
                    : 'text-neutral-400 hover:text-white'
                )}
                aria-label="List view"
                aria-pressed={viewMode() === 'list'}
              >
                <TbOutlineList size={16} />
              </button>
            </div>
          </div>
        </div>
        
        {/* Search */}
        <div class="mt-3">
          <div class="relative">
            <TbOutlineSearch size={14} class="absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500" />
            <input
              type="text"
              value={searchQuery()}
              onInput={e => setSearchQuery(e.currentTarget.value)}
              placeholder="Search files..."
              class="w-full pl-9 pr-4 py-1.5 bg-neutral-900 border border-neutral-700 rounded-lg text-sm text-neutral-300 placeholder-neutral-500 focus:outline-none focus:border-neutral-600"
            />
          </div>
        </div>
      </div>

      {/* Content */}
      <div class="p-4">
        <Show
          when={viewMode() === 'grid'}
          fallback={
            /* List view */
            <div class="space-y-1">
              <For each={filteredFiles()}>
                {(file) => (
                  <div
                    class={cn(
                      "flex items-center gap-3 p-3 rounded-lg transition-colors cursor-pointer",
                      selectedFiles().has(file.id || file.name)
                        ? 'bg-blue-900/20 border border-blue-600/30'
                        : 'hover:bg-neutral-800/50 border border-transparent'
                    )}
                    onClick={() => toggleSelect(file.id || file.name)}
                  >
                    <div class="flex-shrink-0">
                      {getFileIcon(file, 24)}
                    </div>
                    <div class="flex-1 min-w-0">
                      <div class="text-sm text-neutral-300 truncate">{file.name}</div>
                      <div class="text-xs text-neutral-500 truncate">{file.path || '--'}</div>
                    </div>
                    <div class="flex-shrink-0 text-xs text-neutral-500">
                      {formatFileSize(file.size)}
                    </div>
                    <div class="flex-shrink-0 text-xs text-neutral-500">
                      {file.modified || '--'}
                    </div>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        downloadFile(file);
                      }}
                      class="p-1.5 text-neutral-400 hover:text-white hover:bg-neutral-700 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-900"
                      aria-label={`Download ${file.name}`}
                    >
                      <TbOutlineDownload size={16} />
                    </button>
                  </div>
                )}
              </For>
            </div>
          }
        >
          {/* Grid view */}
          <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3">
            <For each={filteredFiles()}>
              {(file) => (
                <div
                  class={cn(
                    "group relative rounded-lg border transition-all cursor-pointer overflow-hidden",
                    selectedFiles().has(file.id || file.name)
                      ? 'bg-blue-900/20 border-blue-600/30'
                      : 'bg-neutral-900/30 border-neutral-700 hover:border-neutral-600 hover:bg-neutral-800/50'
                  )}
                  onClick={() => toggleSelect(file.id || file.name)}
                >
                  {/* Thumbnail or icon */}
                  <div class="aspect-square flex items-center justify-center bg-neutral-800/50">
                    <Show
                      when={file.thumbnail}
                      fallback={getFileIcon(file, 48)}
                    >
                      <img
                        src={file.thumbnail}
                        alt={file.name}
                        class="w-full h-full object-cover"
                      />
                    </Show>
                  </div>
                  
                  {/* Info */}
                  <div class="p-3">
                    <div class="text-sm text-neutral-300 truncate" title={file.name}>
                      {file.name}
                    </div>
                    <div class="text-xs text-neutral-500 mt-1">
                      {formatFileSize(file.size)}
                    </div>
                  </div>
                  
                  {/* Quick actions (hover) */}
                  <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        downloadFile(file);
                      }}
                      class="p-1.5 bg-neutral-800 hover:bg-neutral-700 rounded-lg text-neutral-400 hover:text-white transition-colors"
                    >
                      <TbOutlineDownload size={14} />
                    </button>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>

        {/* Empty state */}
        <Show when={filteredFiles().length === 0}>
          <div class="text-center py-12 text-neutral-500">
            <TbOutlineFolder size={48} class="mx-auto mb-3 opacity-50" />
            <p class="text-sm">No files found</p>
            <Show when={searchQuery()}>
              <p class="text-xs mt-1">Try adjusting your search</p>
            </Show>
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
            {' • '}Updated: {ui()!.metadata!.timestamp}
          </Show>
        </div>
      </Show>
    </Motion.div>
  );
}
