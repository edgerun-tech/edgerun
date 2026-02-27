import { Show, For, createSignal } from "solid-js";
import { Motion } from "solid-motionone";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import { TbOutlineChevronRight, TbOutlineChevronDown } from "solid-icons/tb";
function cn(...classes) {
  return twMerge(clsx(classes));
}
function TreeNode(props) {
  const [expanded, setExpanded] = createSignal(true);
  const isObject = () => typeof props.value === "object" && props.value !== null;
  const isArray = () => Array.isArray(props.value);
  const isEmpty = () => isObject() && Object.keys(props.value).length === 0;
  const getValueType = () => {
    if (props.value === null) return "null";
    if (Array.isArray(props.value)) return "array";
    return typeof props.value;
  };
  const getTypeColor = () => {
    const type = getValueType();
    switch (type) {
      case "string":
        return "text-green-400";
      case "number":
        return "text-blue-400";
      case "boolean":
        return "text-purple-400";
      case "null":
        return "text-neutral-500";
      default:
        return "text-neutral-300";
    }
  };
  const formatValue = (value) => {
    if (typeof value === "string") return `"${value}"`;
    if (value === null) return "null";
    return String(value);
  };
  return <div class="font-mono text-sm">
      <div
    class={cn(
      "flex items-center gap-1 py-0.5 hover:bg-neutral-800/50 rounded px-2 cursor-pointer",
      props.depth > 0 && "ml-4"
    )}
    onClick={() => isObject() && setExpanded(!expanded())}
  >
        {
    /* Expand icon for objects/arrays */
  }
        <Show when={isObject() && !isEmpty()}>
          <span class="text-neutral-500 w-4 h-4 flex items-center justify-center">
            {expanded() ? <TbOutlineChevronDown size={14} /> : <TbOutlineChevronRight size={14} />}
          </span>
        </Show>
        <Show when={!isObject() || isEmpty()}>
          <span class="w-4" />
        </Show>

        {
    /* Key name */
  }
        <Show when={props.name}>
          <span class="text-neutral-400">{props.name}:</span>
        </Show>

        {
    /* Value or type indicator */
  }
        <Show
    when={isObject()}
    fallback={<span class={getTypeColor()}>{formatValue(props.value)}</span>}
  >
          <span class="text-neutral-500">
            {isArray() ? `Array(${props.value.length})` : `{${Object.keys(props.value).length}}`}
          </span>
        </Show>

        {
    /* Comma */
  }
        <Show when={!props.isLast}>
          <span class="text-neutral-600">,</span>
        </Show>
      </div>

      {
    /* Children */
  }
      <Show when={isObject() && !isEmpty() && expanded()}>
        <For each={Object.entries(props.value)}>
          {([key, value], index) => <TreeNode
    name={key}
    value={value}
    depth={props.depth + 1}
    isLast={index() === Object.entries(props.value).length - 1}
  />}
        </For>
      </Show>
    </div>;
}
function JSONTree(props) {
  const ui = () => props.response.ui;
  const [expanded, setExpanded] = createSignal(true);
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
      <div class="px-4 py-3 border-b border-neutral-700 bg-neutral-800/50 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <Show when={ui()?.title}>
            <h3 class="text-sm font-medium text-white">{ui().title}</h3>
          </Show>
          <Show when={ui()?.metadata?.itemCount}>
            <span class="text-xs text-neutral-500">
              {ui().metadata.itemCount} keys
            </span>
          </Show>
        </div>
        <button
    type="button"
    onClick={() => setExpanded(!expanded())}
    class="text-xs text-neutral-400 hover:text-white transition-colors px-2 py-1 rounded hover:bg-neutral-700 cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800"
    aria-pressed={expanded()}
    aria-label={expanded() ? "Collapse all" : "Expand all"}
  >
          {expanded() ? "Collapse All" : "Expand All"}
        </button>
      </div>

      {
    /* Content */
  }
      <div class="p-4 overflow-auto max-h-[600px]">
        <div class="bg-neutral-900/50 rounded-lg p-4 border border-neutral-800">
          <TreeNode
    name=""
    value={props.response.data}
    depth={0}
    isLast={true}
  />
        </div>
      </div>
    </Motion.div>;
}
export {
  JSONTree
};
