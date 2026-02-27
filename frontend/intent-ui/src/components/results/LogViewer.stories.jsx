import { LogViewer } from "../results/LogViewer";
const meta = {
  title: "Results/LogViewer",
  component: LogViewer,
  parameters: {
    layout: "padded"
  }
};
var stdin_default = meta;
const sampleLogs = [
  { timestamp: "13:45:01", level: "info", message: "Server started on port 8080", source: "main" },
  { timestamp: "13:45:02", level: "info", message: "Connected to database", source: "db" },
  { timestamp: "13:45:03", level: "debug", message: "Loading configuration from /etc/app/config.json", source: "config" },
  { timestamp: "13:45:10", level: "warn", message: "High memory usage detected: 78%", source: "monitor" },
  { timestamp: "13:46:00", level: "error", message: "Connection timeout to external API", source: "api" },
  { timestamp: "13:46:01", level: "error", message: "Retry attempt 1/3 failed", source: "api" },
  { timestamp: "13:46:05", level: "info", message: "Connection restored", source: "api" },
  { timestamp: "13:47:00", level: "warn", message: "Slow query detected: 2.3s", source: "db" },
  { timestamp: "13:48:00", level: "info", message: "Cache cleared successfully", source: "cache" },
  { timestamp: "13:49:00", level: "debug", message: "Processing batch job #1234", source: "worker" }
];
const Default = {
  args: {
    response: {
      success: true,
      data: sampleLogs,
      ui: {
        viewType: "log-viewer",
        title: "Application Logs",
        description: "Recent logs from all sources",
        metadata: {
          source: "application",
          itemCount: sampleLogs.length
        }
      }
    }
  }
};
const ErrorLogs = {
  args: {
    response: {
      success: true,
      data: sampleLogs.filter((log) => log.level === "error"),
      ui: {
        viewType: "log-viewer",
        title: "Error Logs",
        description: "Filtered to show errors only",
        metadata: {
          source: "application",
          itemCount: 2,
          level: "error"
        }
      }
    }
  }
};
const TerminalOutput = {
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
13:50:10 [vite] \u2713 built in 8.90s
13:50:11 [build] Complete!

\u2728 Build completed successfully`,
      ui: {
        viewType: "log-viewer",
        title: "Build Output",
        description: "npm run build",
        metadata: {
          source: "terminal"
        }
      }
    }
  }
};
const Empty = {
  args: {
    response: {
      success: true,
      data: [],
      ui: {
        viewType: "log-viewer",
        title: "Logs",
        description: "No logs available",
        metadata: {
          source: "application",
          itemCount: 0
        }
      }
    }
  }
};
const LargeDataSet = {
  args: {
    response: {
      success: true,
      data: Array.from({ length: 100 }, (_, i) => ({
        timestamp: `13:${String(Math.floor(i / 60)).padStart(2, "0")}:${String(i % 60).padStart(2, "0")}`,
        level: ["info", "warn", "error", "debug"][i % 4],
        message: `Log message #${i + 1} - ${["Starting process", "Processing request", "Completed task", "Error occurred"][i % 4]}`,
        source: ["api", "db", "cache", "worker"][i % 4]
      })),
      ui: {
        viewType: "log-viewer",
        title: "System Logs",
        description: "Last 100 log entries",
        metadata: {
          source: "system",
          itemCount: 100
        }
      }
    }
  }
};
export {
  Default,
  Empty,
  ErrorLogs,
  LargeDataSet,
  TerminalOutput,
  stdin_default as default
};
