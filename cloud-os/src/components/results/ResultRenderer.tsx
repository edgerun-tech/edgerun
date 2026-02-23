/**
 * Result Renderer
 * Main dispatcher for morphable result views
 */

import { Show } from 'solid-js';
import type { ToolResponse, ViewType } from '../../lib/mcp/types';
import { PreviewCard } from './PreviewCard';
import { JSONTree } from './JSONTree';
import { DataTable } from './DataTable';
import { LogViewer } from './LogViewer';
import { FileGrid } from './FileGrid';
import { CodeDiffViewer } from './CodeDiffViewer';
import { Timeline } from './Timeline';
import { EmailReader } from './EmailReader';
import { DocViewer } from './DocViewer';
import { MediaGallery } from './MediaGallery';

interface ResultRendererProps {
  response: ToolResponse;
  onAction?: (intent: string) => void;
  class?: string;
}

// View type to component mapping
const viewComponents: Record<ViewType, any> = {
  'preview': PreviewCard,
  'json-tree': JSONTree,
  'table': DataTable,
  'log-viewer': LogViewer,
  'file-grid': FileGrid,
  'code-diff': CodeDiffViewer,
  'timeline': Timeline,
  'email-reader': EmailReader,
  'doc-viewer': DocViewer,
  'media-gallery': MediaGallery,
};

export function ResultRenderer(props: ResultRendererProps) {
  // Determine view type
  const viewType = () => {
    // Explicit view type from UI hints
    if (props.response.ui?.viewType) {
      return props.response.ui.viewType;
    }
    
    // Auto-detect based on data structure
    const data = props.response.data;
    
    // Check for email-like data
    if (Array.isArray(data) && data.length > 0) {
      const first = data[0];
      if (typeof first === 'object' && (first.from || first.to || first.subject)) {
        return 'email-reader';
      }
    }
    
    // Check for media-like data
    if (Array.isArray(data) && data.length > 0) {
      const first = data[0];
      if (typeof first === 'object' && (first.url || first.thumbnail) && 
          (first.mimeType?.startsWith('image/') || first.mimeType?.startsWith('video/') ||
           first.type === 'image' || first.type === 'video')) {
        return 'media-gallery';
      }
    }
    
    // Check for timeline-like data
    if (Array.isArray(data) && data.length > 0) {
      const first = data[0];
      if (typeof first === 'object' && (first.timestamp || first.date) && first.title) {
        return 'timeline';
      }
    }
    
    // Check for doc-like data
    if (typeof data === 'string' && (data.includes('# ') || data.includes('## ') || data.includes('```'))) {
      return 'doc-viewer';
    }
    if (typeof data === 'object' && data?.content && typeof data.content === 'string') {
      return 'doc-viewer';
    }
    
    // Check for diff format
    if (typeof data === 'string' && (data.includes('diff --git') || data.includes('@@ -'))) {
      return 'code-diff';
    }
    
    // Check for log-like data
    if (Array.isArray(data) && data.length > 0) {
      const first = data[0];
      if (typeof first === 'object' && (first.level || first.timestamp || first.message)) {
        return 'log-viewer';
      }
    }
    
    // Check for file-like data
    if (Array.isArray(data) && data.length > 0) {
      const first = data[0];
      if (typeof first === 'object' && (first.path || first.type === 'file' || first.type === 'folder')) {
        return 'file-grid';
      }
    }
    
    // Array of objects with similar keys -> table
    if (Array.isArray(data) && data.length > 0 && typeof data[0] === 'object') {
      return 'table';
    }
    
    // Object with nested structure -> JSON tree
    if (typeof data === 'object' && data !== null && !Array.isArray(data)) {
      return 'json-tree';
    }
    
    // Default to preview
    return 'preview';
  };

  const ViewComponent = viewComponents[viewType()];

  return (
    <div class={props.class}>
      <Show
        when={ViewComponent}
        fallback={
          <PreviewCard 
            response={props.response} 
            onAction={props.onAction}
          />
        }
      >
        <ViewComponent 
          response={props.response}
          onAction={props.onAction}
        />
      </Show>
    </div>
  );
}
