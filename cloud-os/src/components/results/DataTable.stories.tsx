import type { Meta, StoryObj } from '@storybook/web-components';
import { DataTable } from '../results/DataTable';
import type { ToolResponse } from '../../lib/mcp/types';

const meta = {
  title: 'Results/DataTable',
  component: DataTable,
  parameters: {
    layout: 'padded',
  },
} satisfies Meta<typeof DataTable>;

export default meta;
type Story = StoryObj<typeof meta>;

const serverData = [
  { name: 'web-01', region: 'us-east-1', status: 'running', cpu: '45%', memory: '62%', uptime: '15d' },
  { name: 'web-02', region: 'us-east-1', status: 'running', cpu: '38%', memory: '58%', uptime: '15d' },
  { name: 'web-03', region: 'us-west-2', status: 'running', cpu: '52%', memory: '71%', uptime: '12d' },
  { name: 'db-01', region: 'us-east-1', status: 'running', cpu: '78%', memory: '85%', uptime: '30d' },
  { name: 'db-02', region: 'us-east-1', status: 'stopped', cpu: '0%', memory: '0%', uptime: '-' },
  { name: 'cache-01', region: 'us-east-1', status: 'running', cpu: '23%', memory: '45%', uptime: '20d' },
];

export const Servers: Story = {
  args: {
    response: {
      success: true,
      data: serverData,
      ui: {
        viewType: 'table',
        title: 'Server Status',
        description: 'Current status of all servers',
        metadata: {
          source: 'hetzner',
          itemCount: serverData.length,
        },
      },
    },
  },
};

const deployments = [
  { name: 'frontend', branch: 'main', status: 'ready', url: 'app.example.com', updatedAt: '2024-02-17' },
  { name: 'api', branch: 'main', status: 'building', url: 'api.example.com', updatedAt: '2024-02-17' },
  { name: 'admin', branch: 'staging', status: 'error', url: 'admin.example.com', updatedAt: '2024-02-16' },
  { name: 'docs', branch: 'main', status: 'ready', url: 'docs.example.com', updatedAt: '2024-02-15' },
];

export const Deployments: Story = {
  args: {
    response: {
      success: true,
      data: deployments,
      ui: {
        viewType: 'table',
        title: 'Deployments',
        description: 'Current deployment status',
        metadata: {
          source: 'vercel',
          itemCount: deployments.length,
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
        viewType: 'table',
        title: 'Results',
        description: 'No data available',
        metadata: {
          source: 'api',
          itemCount: 0,
        },
      },
    },
  },
};
