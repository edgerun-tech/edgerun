import { Show } from "solid-js";
import { PreviewCard } from "./PreviewCard";
import { JSONTree } from "./JSONTree";
import { DataTable } from "./DataTable";
import { LogViewer } from "./LogViewer";
import { FileGrid } from "./FileGrid";
import { CodeDiffViewer } from "./CodeDiffViewer";
import { Timeline } from "./Timeline";
import { EmailReader } from "./EmailReader";
import { DocViewer } from "./DocViewer";
import { MediaGallery } from "./MediaGallery";
const viewComponents = {
  "preview": PreviewCard,
  "json-tree": JSONTree,
  "table": DataTable,
  "log-viewer": LogViewer,
  "file-grid": FileGrid,
  "code-diff": CodeDiffViewer,
  "timeline": Timeline,
  "email-reader": EmailReader,
  "doc-viewer": DocViewer,
  "media-gallery": MediaGallery
};
function ResultRenderer(props) {
  const viewType = () => {
    if (props.response.ui?.viewType) {
      return props.response.ui.viewType;
    }
    const data = props.response.data;
    if (Array.isArray(data) && data.length > 0) {
      const first = data[0];
      if (typeof first === "object" && (first.from || first.to || first.subject)) {
        return "email-reader";
      }
    }
    if (Array.isArray(data) && data.length > 0) {
      const first = data[0];
      if (typeof first === "object" && (first.url || first.thumbnail) && (first.mimeType?.startsWith("image/") || first.mimeType?.startsWith("video/") || first.type === "image" || first.type === "video")) {
        return "media-gallery";
      }
    }
    if (Array.isArray(data) && data.length > 0) {
      const first = data[0];
      if (typeof first === "object" && (first.timestamp || first.date) && first.title) {
        return "timeline";
      }
    }
    if (typeof data === "string" && (data.includes("# ") || data.includes("## ") || data.includes("```"))) {
      return "doc-viewer";
    }
    if (typeof data === "object" && data?.content && typeof data.content === "string") {
      return "doc-viewer";
    }
    if (typeof data === "string" && (data.includes("diff --git") || data.includes("@@ -"))) {
      return "code-diff";
    }
    if (Array.isArray(data) && data.length > 0) {
      const first = data[0];
      if (typeof first === "object" && (first.level || first.timestamp || first.message)) {
        return "log-viewer";
      }
    }
    if (Array.isArray(data) && data.length > 0) {
      const first = data[0];
      if (typeof first === "object" && (first.path || first.type === "file" || first.type === "folder")) {
        return "file-grid";
      }
    }
    if (Array.isArray(data) && data.length > 0 && typeof data[0] === "object") {
      return "table";
    }
    if (typeof data === "object" && data !== null && !Array.isArray(data)) {
      return "json-tree";
    }
    return "preview";
  };
  const ViewComponent = viewComponents[viewType()];
  return <div class={props.class}>
      <Show
    when={ViewComponent}
    fallback={<PreviewCard
      response={props.response}
      onAction={props.onAction}
    />}
  >
        <ViewComponent
    response={props.response}
    onAction={props.onAction}
  />
      </Show>
    </div>;
}
export {
  ResultRenderer
};
