import { JSONTree } from "../results/JSONTree";
const meta = {
  title: "Results/JSONTree",
  component: JSONTree,
  parameters: {
    layout: "padded"
  }
};
var stdin_default = meta;
const configData = {
  name: "browser-os",
  version: "1.0.0",
  environment: "production",
  features: {
    authentication: true,
    darkMode: true,
    notifications: false
  },
  database: {
    host: "db.example.com",
    port: 5432,
    ssl: true,
    pool: {
      min: 2,
      max: 10
    }
  },
  api: {
    baseUrl: "https://api.example.com",
    timeout: 3e4,
    retries: 3
  }
};
const Configuration = {
  args: {
    response: {
      success: true,
      data: configData,
      ui: {
        viewType: "json-tree",
        title: "App Configuration",
        description: "Current application settings",
        metadata: {
          source: "config"
        }
      }
    }
  }
};
const apiResponse = {
  user: {
    id: "usr_123456",
    email: "user@example.com",
    profile: {
      firstName: "John",
      lastName: "Doe",
      avatar: "https://example.com/avatar.jpg"
    },
    roles: ["admin", "user"],
    permissions: ["read", "write", "delete"],
    metadata: {
      createdAt: "2024-01-01T00:00:00Z",
      lastLogin: "2024-02-17T13:00:00Z",
      loginCount: 42
    }
  }
};
const APIResponse = {
  args: {
    response: {
      success: true,
      data: apiResponse,
      ui: {
        viewType: "json-tree",
        title: "User API Response",
        metadata: {
          source: "api"
        }
      }
    }
  }
};
const SimpleObject = {
  args: {
    response: {
      success: true,
      data: {
        success: true,
        message: "Operation completed",
        timestamp: (/* @__PURE__ */ new Date()).toISOString()
      },
      ui: {
        viewType: "json-tree",
        title: "Simple Response"
      }
    }
  }
};
export {
  APIResponse,
  Configuration,
  SimpleObject,
  stdin_default as default
};
