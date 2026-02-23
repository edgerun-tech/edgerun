import type { Meta, StoryObj } from '@storybook/web-components';
import { LogViewer } from '../results/LogViewer';
import type { ToolResponse } from '../../lib/mcp/types';

const meta = {
  title: 'Results/LogViewer',
  component: LogViewer,
  parameters: {
    layout: 'padded',
  },
} satisfies Meta<typeof LogViewer>;

export default meta;
type Story = StoryObj<typeof meta>;

const sampleLogs = [
  { timestamp: '13:45:01', level: 'info' as const, message: 'Server started on port 8080', source: 'main' },
  { timestamp: '13:45:02', level: 'info' as const, message: 'Connected to database', source: 'db' },
  { timestamp: '13:45:03', level: 'debug' as const, message: 'Loading configuration from /etc/app/config.json', source: 'config' },
  { timestamp: '13:45:10', level: 'warn' as const, message: 'High memory usage detected: 78%', source: 'monitor' },
  { timestamp: '13:46:00', level: 'error' as const, message: 'Connection timeout to external API', source: 'api' },
  { timestamp: '13:46:01', level: 'error' as const, message: 'Retry attempt 1/3 failed', source: 'api' },
  { timestamp: '13:46:05', level: 'info' as const, message: 'Connection restored', source: 'api' },
  { timestamp: '13:47:00', level: 'warn' as const, message: 'Slow query detected: 2.3s', source: 'db' },
  { timestamp: '13:48:00', level: 'info' as const, message: 'Cache cleared successfully', source: 'cache' },
  { timestamp: '13:49:00', level: 'debug' as const, message: 'Processing batch job #1234', source: 'worker' },
];

export const Default: Story = {
  args: {
    response: {
      success: true,
      data: sampleLogs,
      ui: {
        viewType: 'log-viewer',
        title: 'Application Logs',
        description: 'Recent logs from all sources',
        metadata: {
          source: 'application',
          itemCount: sampleLogs.length,
        },
      },
    },
  },
};

export const ErrorLogs: Story = {
  args: {
    response: {
      success: true,
      data: sampleLogs.filter(log => log.level === 'error'),
      ui: {
        viewType: 'log-viewer',
        title: 'Error Logs',
        description: 'Filtered to show errors only',
        metadata: {
          source: 'application',
          itemCount: 2,
          level: 'error',
        },
      },
    },
  },
};

export const TerminalOutput: Story = {
  args: {
    response: {
      success: true,
      data: `> npm run build

> browser-os@1.0.0 build
> astro build

13:50:00 [@astrojs/cloudflare] Enabling sessions...
13:50:01 [content] Syncing content
13:50:01 [content] Synced content
13:50:02 [types] Generated 73ms
13:50:02 [build] output: "static"
13:50:02 [build] Building server entrypoints...
13:50:10 [vite] ✓ built in 8.90s
13:50:11 [build] Complete!

✨ Build completed successfully`,
      ui: {
        viewType: 'log-viewer',
        title: 'Build Output',
        description: 'npm run build',
        metadata: {
          source: 'terminal',
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
        viewType: 'log-viewer',
        title: 'Logs',
        description: 'No logs available',
        metadata: {
          source: 'application',
          itemCount: 0,
        },
      },
    },
  },
};

export const LargeDataSet: Story = {
  args: {
    response: {
      success: true,
      data: Array.from({ length: 100 }, (_, i) => ({
        timestamp: `13:${String(Math.floor(i / 60)).padStart(2, '0')}:${String(i % 60).padStart(2, '0')}`,
        level: ['info', 'warn', 'error', 'debug'][i % 4] as any,
        message: `Log message #${i + 1} - ${['Starting process', 'Processing request', 'Completed task', 'Error occurred'][i % 4]}`,
        source: ['api', 'db', 'cache', 'worker'][i % 4],
      })),
      ui: {
        viewType: 'log-viewer',
        title: 'System Logs',
        description: 'Last 100 log entries',
        metadata: {
          source: 'system',
          itemCount: 100,
        },
      },
    },
  },
};
