import type { Meta, StoryObj } from '@storybook/web-components';
import { PreviewCard } from '../results/PreviewCard';
import type { ToolResponse } from '../../lib/mcp/types';

const meta = {
  title: 'Results/PreviewCard',
  component: PreviewCard,
  parameters: {
    layout: 'centered',
  },
  argTypes: {
    onAction: { action: 'clicked' },
  },
} satisfies Meta<typeof PreviewCard>;

export default meta;
type Story = StoryObj<typeof meta>;

const defaultResponse: ToolResponse = {
  success: true,
  data: { message: 'Operation completed successfully' },
  ui: {
    viewType: 'preview',
    title: 'Window Opened',
    description: 'The terminal window is now open',
    metadata: {
      source: 'browser-os',
      timestamp: new Date().toISOString(),
      itemCount: 1,
    },
    actions: [
      { label: 'Close Window', intent: 'close terminal', variant: 'secondary' },
      { label: 'Open Another', intent: 'open files', variant: 'primary' },
    ],
  },
};

export const Default: Story = {
  args: {
    response: defaultResponse,
  },
};

export const WithError: Story = {
  args: {
    response: {
      success: false,
      error: 'Failed to connect to server',
      ui: {
        viewType: 'preview',
        title: 'Error',
        description: 'Connection timeout after 30s',
        metadata: {
          source: 'api',
          timestamp: new Date().toISOString(),
        },
      },
    },
  },
};

export const WithComplexData: Story = {
  args: {
    response: {
      success: true,
      data: {
        userId: 'usr_123',
        name: 'John Doe',
        email: 'john@example.com',
        role: 'admin',
        permissions: ['read', 'write', 'delete'],
        settings: {
          theme: 'dark',
          notifications: true,
        },
      },
      ui: {
        viewType: 'preview',
        title: 'User Profile',
        description: 'User details from database',
        metadata: {
          source: 'database',
          itemCount: 6,
        },
      },
    },
  },
};

export const WithoutUIHints: Story = {
  args: {
    response: {
      success: true,
      data: 'Simple text response without UI hints',
    },
  },
};

export const WithLongContent: Story = {
  args: {
    response: {
      success: true,
      data: `This is a longer text response that demonstrates how the PreviewCard handles multi-line content.

It preserves whitespace and line breaks, making it suitable for displaying:
- Log output
- Command results
- Status messages
- Any other text content

The card will expand to fit the content while maintaining readability.`,
      ui: {
        viewType: 'preview',
        title: 'Command Output',
        metadata: {
          source: 'terminal',
        },
      },
    },
  },
};
