import { PreviewCard } from "../results/PreviewCard";
const meta = {
  title: "Results/PreviewCard",
  component: PreviewCard,
  parameters: {
    layout: "centered"
  },
  argTypes: {
    onAction: { action: "clicked" }
  }
};
var stdin_default = meta;
const defaultResponse = {
  success: true,
  data: { message: "Operation completed successfully" },
  ui: {
    viewType: "preview",
    title: "Window Opened",
    description: "The terminal window is now open",
    metadata: {
      source: "browser-os",
      timestamp: (/* @__PURE__ */ new Date()).toISOString(),
      itemCount: 1
    },
    actions: [
      { label: "Close Window", intent: "close terminal", variant: "secondary" },
      { label: "Open Another", intent: "open files", variant: "primary" }
    ]
  }
};
const Default = {
  args: {
    response: defaultResponse
  }
};
const WithError = {
  args: {
    response: {
      success: false,
      error: "Failed to connect to server",
      ui: {
        viewType: "preview",
        title: "Error",
        description: "Connection timeout after 30s",
        metadata: {
          source: "api",
          timestamp: (/* @__PURE__ */ new Date()).toISOString()
        }
      }
    }
  }
};
const WithComplexData = {
  args: {
    response: {
      success: true,
      data: {
        userId: "usr_123",
        name: "John Doe",
        email: "john@example.com",
        role: "admin",
        permissions: ["read", "write", "delete"],
        settings: {
          theme: "dark",
          notifications: true
        }
      },
      ui: {
        viewType: "preview",
        title: "User Profile",
        description: "User details from database",
        metadata: {
          source: "database",
          itemCount: 6
        }
      }
    }
  }
};
const WithoutUIHints = {
  args: {
    response: {
      success: true,
      data: "Simple text response without UI hints"
    }
  }
};
const WithLongContent = {
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
        viewType: "preview",
        title: "Command Output",
        metadata: {
          source: "terminal"
        }
      }
    }
  }
};
export {
  Default,
  WithComplexData,
  WithError,
  WithLongContent,
  WithoutUIHints,
  stdin_default as default
};
