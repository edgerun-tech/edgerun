import type { Meta, StoryObj } from '@storybook/web-components';
import { FileGrid } from '../results/FileGrid';
import type { ToolResponse } from '../../lib/mcp/types';

const meta = {
  title: 'Results/FileGrid',
  component: FileGrid,
  parameters: {
    layout: 'padded',
  },
} satisfies Meta<typeof FileGrid>;

export default meta;
type Story = StoryObj<typeof meta>;

const fileData = [
  { id: '1', name: 'README.md', path: '/src/README.md', type: 'file' as const, size: 2048, mimeType: 'text/markdown' },
  { id: '2', name: 'src', path: '/src', type: 'folder' as const },
  { id: '3', name: 'components', path: '/src/components', type: 'folder' as const },
  { id: '4', name: 'App.tsx', path: '/src/App.tsx', type: 'file' as const, size: 4096, mimeType: 'text/typescript' },
  { id: '5', name: 'logo.png', path: '/src/assets/logo.png', type: 'file' as const, size: 15360, mimeType: 'image/png', thumbnail: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==' },
  { id: '6', name: 'package.json', path: '/package.json', type: 'file' as const, size: 1024, mimeType: 'application/json' },
  { id: '7', name: 'tests', path: '/tests', type: 'folder' as const },
  { id: '8', name: 'utils.ts', path: '/src/utils.ts', type: 'file' as const, size: 3072, mimeType: 'text/typescript' },
];

export const Grid: Story = {
  args: {
    response: {
      success: true,
      data: fileData,
      ui: {
        viewType: 'file-grid',
        title: 'Project Files',
        description: 'Files in current directory',
        metadata: {
          source: 'file-system',
          itemCount: fileData.length,
        },
      },
    },
  },
};

export const SearchResults: Story = {
  args: {
    response: {
      success: true,
      data: fileData.filter(f => f.name.endsWith('.ts') || f.name.endsWith('.tsx')),
      ui: {
        viewType: 'file-grid',
        title: 'TypeScript Files',
        description: 'Search results for "*.ts"',
        metadata: {
          source: 'file-system',
          itemCount: 2,
        },
      },
    },
  },
};

export const Empty: Story = {
  args: {
    response: {
      success: true,
      data: [],
      ui: {
        viewType: 'file-grid',
        title: 'Files',
        description: 'No files found',
        metadata: {
          source: 'file-system',
          itemCount: 0,
        },
      },
    },
  },
};
