import { FileGrid } from "../results/FileGrid";
const meta = {
  title: "Results/FileGrid",
  component: FileGrid,
  parameters: {
    layout: "padded"
  }
};
var stdin_default = meta;
const fileData = [
  { id: "1", name: "README.md", path: "/src/README.md", type: "file", size: 2048, mimeType: "text/markdown" },
  { id: "2", name: "src", path: "/src", type: "folder" },
  { id: "3", name: "components", path: "/src/components", type: "folder" },
  { id: "4", name: "App.jsx", path: "/src/App.jsx", type: "file", size: 4096, mimeType: "text/javascript" },
  { id: "5", name: "logo.png", path: "/src/assets/logo.png", type: "file", size: 15360, mimeType: "image/png", thumbnail: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==" },
  { id: "6", name: "package.json", path: "/package.json", type: "file", size: 1024, mimeType: "application/json" },
  { id: "7", name: "tests", path: "/tests", type: "folder" },
  { id: "8", name: "utils.js", path: "/src/utils.js", type: "file", size: 3072, mimeType: "text/javascript" }
];
const Grid = {
  args: {
    response: {
      success: true,
      data: fileData,
      ui: {
        viewType: "file-grid",
        title: "Project Files",
        description: "Files in current directory",
        metadata: {
          source: "file-system",
          itemCount: fileData.length
        }
      }
    }
  }
};
const SearchResults = {
  args: {
    response: {
      success: true,
      data: fileData.filter((f) => f.name.endsWith(".js") || f.name.endsWith(".jsx")),
      ui: {
        viewType: "file-grid",
        title: "TypeScript Files",
        description: 'Search results for "*.js"',
        metadata: {
          source: "file-system",
          itemCount: 2
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
        viewType: "file-grid",
        title: "Files",
        description: "No files found",
        metadata: {
          source: "file-system",
          itemCount: 0
        }
      }
    }
  }
};
export {
  Empty,
  Grid,
  SearchResults,
  stdin_default as default
};
