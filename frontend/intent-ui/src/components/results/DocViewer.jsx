import { createSignal, Show } from "solid-js";
import { Motion } from "solid-motionone";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import { TbOutlineFileText, TbOutlineCopy, TbOutlineDownload } from "solid-icons/tb";
function cn(...classes) {
  return twMerge(clsx(classes));
}
function parseMarkdown(text) {
  if (!text) return "";
  let html = text;
  html = html.replace(/^### (.*$)/gim, '<h3 class="text-lg font-semibold text-white mt-4 mb-2">$1</h3>');
  html = html.replace(/^## (.*$)/gim, '<h2 class="text-xl font-semibold text-white mt-6 mb-3">$1</h2>');
  html = html.replace(/^# (.*$)/gim, '<h1 class="text-2xl font-bold text-white mt-6 mb-4">$1</h1>');
  html = html.replace(/\*\*(.*?)\*\*/g, '<strong class="font-semibold text-white">$1</strong>');
  html = html.replace(/\*(.*?)\*/g, '<em class="italic">$1</em>');
  html = html.replace(/`([^`]+)`/g, '<code class="px-1.5 py-0.5 bg-neutral-800 rounded text-sm font-mono text-pink-400">$1</code>');
  html = html.replace(/```(\w*)\n([\s\S]*?)```/g, '<pre class="bg-neutral-900 rounded-lg p-4 my-3 overflow-x-auto"><code class="text-sm font-mono text-neutral-300">$2</code></pre>');
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" class="text-blue-400 hover:text-blue-300 underline" target="_blank" rel="noopener">$1</a>');
  html = html.replace(/^\s*[-*+]\s+(.*$)/gim, '<li class="ml-4 text-neutral-300">$1</li>');
  html = html.replace(/^\s*\d+\.\s+(.*$)/gim, '<li class="ml-4 text-neutral-300 list-decimal">$1</li>');
  html = html.replace(/^>\s+(.*$)/gim, '<blockquote class="border-l-4 border-neutral-600 pl-4 my-3 text-neutral-400 italic">$1</blockquote>');
  html = html.replace(/^---$/gim, '<hr class="border-neutral-700 my-6" />');
  html = html.replace(/\n\n/g, '</p><p class="my-3">');
  html = html.replace(/\n/g, "<br />");
  return '<p class="my-3">' + html + "</p>";
}
function DocViewer(props) {
  const ui = () => props.response.ui;
  const [copied, setCopied] = createSignal(false);
  const content = () => {
    if (typeof props.response.data === "string") {
      return props.response.data;
    }
    if (props.response.data?.content) {
      return props.response.data.content;
    }
    return JSON.stringify(props.response.data, null, 2);
  };
  const copyToClipboard = async () => {
    try {
      await navigator.clipboard.writeText(content());
      setCopied(true);
      setTimeout(() => setCopied(false), 2e3);
    } catch (e) {
      console.error("Failed to copy:", e);
    }
  };
  const downloadFile = () => {
    const blob = new Blob([content()], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = ui()?.title?.replace(/\s+/g, "-").toLowerCase() || "document.md";
    a.click();
    URL.revokeObjectURL(url);
  };
  return <Motion.div
    initial={{ opacity: 0, y: 8 }}
    animate={{ opacity: 1, y: 0 }}
    exit={{ opacity: 0, y: -8 }}
    transition={{ duration: 0.2 }}
    class={cn(
      "bg-neutral-800/50 rounded-xl border border-neutral-700 overflow-hidden",
      props.class
    )}
  >
      {
    /* Header */
  }
      <div class="px-4 py-3 border-b border-neutral-700 bg-neutral-800/50">
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-3">
            <TbOutlineFileText size={18} class="text-blue-400" />
            <Show when={ui()?.title}>
              <h3 class="text-sm font-medium text-white">{ui().title}</h3>
            </Show>
            <Show when={ui()?.metadata?.source}>
              <span class="text-xs text-neutral-500">{ui().metadata.source}</span>
            </Show>
          </div>
          
          <div class="flex items-center gap-2" role="group" aria-label="Document actions">
            <button
    type="button"
    onClick={copyToClipboard}
    class="p-1.5 text-neutral-400 hover:text-white hover:bg-neutral-700 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800"
    title="Copy content"
    aria-label="Copy content"
  >
              <TbOutlineCopy size={16} />
              <Show when={copied()}>
                <span class="sr-only">Copied!</span>
              </Show>
            </button>
            <button
    type="button"
    onClick={downloadFile}
    class="p-1.5 text-neutral-400 hover:text-white hover:bg-neutral-700 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800"
    title="Download"
    aria-label="Download document"
  >
              <TbOutlineDownload size={16} />
            </button>
          </div>
        </div>
      </div>

      {
    /* Content */
  }
      <div class="p-6 overflow-auto max-h-[600px]">
        <div
    class="prose prose-invert prose-sm max-w-none"
    innerHTML={parseMarkdown(content())}
  />
      </div>

      {
    /* Footer */
  }
      <Show when={ui()?.metadata}>
        <div class="px-4 py-3 bg-neutral-800/30 border-t border-neutral-700 text-xs text-neutral-500">
          <Show when={ui().metadata.itemCount}>
            {ui().metadata.itemCount} characters
          </Show>
          <Show when={ui().metadata.timestamp}>
            {" \u2022 "}Last updated: {ui().metadata.timestamp}
          </Show>
        </div>
      </Show>
    </Motion.div>;
}
export {
  DocViewer
};
