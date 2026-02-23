import type { Meta, StoryObj } from '@storybook/web-components';
import { Timeline } from '../results/Timeline';
import type { ToolResponse } from '../../lib/mcp/types';

const meta = {
  title: 'Results/Timeline',
  component: Timeline,
  parameters: {
    layout: 'padded',
  },
  argTypes: {
    onAction: { action: 'action clicked' },
  },
} satisfies Meta<typeof Timeline>;

export default meta;
type Story = StoryObj<typeof meta>;

const deploymentData = [
  {
    id: '1',
    timestamp: new Date(Date.now() - 1000 * 60 * 30).toISOString(),
    title: 'Production Deploy - v2.3.1',
    description: 'Commit: "Fix authentication bug in OAuth flow"',
    type: 'deployment' as const,
    metadata: {
      commit: 'abc123',
      author: 'John Doe',
      duration: '2m 34s',
    },
    actions: [
      { label: 'View Logs', intent: 'show deployment logs for v2.3.1' },
      { label: 'Rollback', intent: 'rollback to v2.3.0', variant: 'danger' as const },
    ],
  },
  {
    id: '2',
    timestamp: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(),
    title: 'Production Deploy - v2.3.0',
    description: 'Commit: "Add new dashboard analytics"',
    type: 'deployment' as const,
    metadata: {
      commit: 'def456',
      author: 'Jane Smith',
      duration: '3m 12s',
    },
  },
  {
    id: '3',
    timestamp: new Date(Date.now() - 1000 * 60 * 60 * 24).toISOString(),
    title: 'Build Failed',
    description: 'Commit: "WIP: broken change" - TypeScript compilation error',
    type: 'error' as const,
    metadata: {
      commit: 'ghi789',
      error: 'TS2304: Cannot find name',
    },
    actions: [
      { label: 'View Error', intent: 'show build error details' },
    ],
  },
  {
    id: '4',
    timestamp: new Date(Date.now() - 1000 * 60 * 60 * 48).toISOString(),
    title: 'Staging Deploy - v2.2.5',
    description: 'Commit: "Update dependencies"',
    type: 'deployment' as const,
    metadata: {
      commit: 'jkl012',
      environment: 'staging',
    },
  },
];

export const DeploymentHistory: Story = {
  args: {
    response: {
      success: true,
      data: deploymentData,
      ui: {
        viewType: 'timeline',
        title: 'Deployment History',
        description: 'Recent deployments to production',
        metadata: {
          source: 'vercel',
          itemCount: deploymentData.length,
        },
      },
    },
  },
};

const commitHistory = [
  {
    id: '1',
    timestamp: new Date(Date.now() - 1000 * 60 * 15).toISOString(),
    title: 'Fix null pointer in auth handler',
    description: 'Added null check before accessing user.id property',
    type: 'commit' as const,
    metadata: {
      hash: 'a1b2c3d',
      author: 'Alice',
      branch: 'main',
    },
  },
  {
    id: '2',
    timestamp: new Date(Date.now() - 1000 * 60 * 60).toISOString(),
    title: 'Add unit tests for API endpoints',
    description: 'Increased coverage to 85%',
    type: 'commit' as const,
    metadata: {
      hash: 'e4f5g6h',
      author: 'Bob',
      branch: 'main',
    },
  },
  {
    id: '3',
    timestamp: new Date(Date.now() - 1000 * 60 * 60 * 5).toISOString(),
    title: 'Refactor database queries',
    description: 'Optimized slow queries, reduced latency by 40%',
    type: 'commit' as const,
    metadata: {
      hash: 'i7j8k9l',
      author: 'Charlie',
      branch: 'feature/perf',
    },
  },
];

export const CommitHistory: Story = {
  args: {
    response: {
      success: true,
      data: commitHistory,
      ui: {
        viewType: 'timeline',
        title: 'Recent Commits',
        description: 'Last 3 commits to main branch',
        metadata: {
          source: 'github',
          itemCount: commitHistory.length,
          branch: 'main',
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
        viewType: 'timeline',
        title: 'Event History',
        description: 'No events found',
        metadata: {
          source: 'api',
          itemCount: 0,
        },
      },
    },
  },
};

export const WithFilters: Story = {
  args: {
    response: {
      success: true,
      data: [
        ...deploymentData,
        ...commitHistory,
        {
          id: '5',
          timestamp: new Date().toISOString(),
          title: 'Warning: High memory usage',
          description: 'Memory usage exceeded 80% threshold',
          type: 'warning' as const,
          metadata: {
            metric: 'memory',
            value: '87%',
          },
        },
      ],
      ui: {
        viewType: 'timeline',
        title: 'All Events',
        description: 'Deployments, commits, and system events',
        metadata: {
          source: 'multiple',
          itemCount: 8,
        },
      },
    },
  },
};
