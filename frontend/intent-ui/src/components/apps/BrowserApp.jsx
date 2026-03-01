import { Show, createSignal, onCleanup, onMount } from "solid-js";
import { TbOutlineExternalLink, TbOutlineReload, TbOutlineX } from "solid-icons/tb";
import { closeWindow } from "../../stores/windows";
import { UI_EVENT_TOPICS } from "../../lib/ui-intents";
import { subscribeEvent } from "../../stores/eventbus";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuShortcut,
  ContextMenuTrigger
} from "../ui";

const DEFAULT_URL = "https://example.com";
const toProxyUrl = (target) => `/api/browser/proxy?url=${encodeURIComponent(target)}`;

function normalizeUrl(raw) {
  const trimmed = raw.trim();
  if (!trimmed) return DEFAULT_URL;
  if (/^https?:\/\//i.test(trimmed)) return trimmed;
  return `https://${trimmed}`;
}

function BrowserApp(props) {
  const initialUrl = normalizeUrl(typeof props.initialUrl === "string" && props.initialUrl.trim() ? props.initialUrl : DEFAULT_URL);
  const [inputValue, setInputValue] = createSignal(initialUrl);
  const [url, setUrl] = createSignal(initialUrl);
  const [frameKey, setFrameKey] = createSignal(0);
  let unsubscribeNavigate;

  const navigate = (next) => {
    const normalized = normalizeUrl(next);
    setInputValue(normalized);
    setUrl(normalized);
  };

  const reload = () => {
    setFrameKey((prev) => prev + 1);
  };
  const closeCurrent = () => {
    if (props.windowId) {
      closeWindow(props.windowId);
    }
  };
  const copyUrl = async () => {
    const current = url();
    try {
      await navigator.clipboard.writeText(current);
    } catch {
      const area = document.createElement("textarea");
      area.value = current;
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.appendChild(area);
      area.focus();
      area.select();
      document.execCommand("copy");
      document.body.removeChild(area);
    }
  };

  onMount(() => {
    unsubscribeNavigate = subscribeEvent(UI_EVENT_TOPICS.action.browserNavigated, (event) => {
      const text = event?.payload?.url;
      if (typeof text === "string" && text.trim()) {
        navigate(text);
      }
    });
  });
  onCleanup(() => {
    if (unsubscribeNavigate) unsubscribeNavigate();
  });

  return (
    <div class="flex h-full min-h-0 flex-col bg-[#111]">
      <form
        class="flex items-center gap-2 border-b border-neutral-800 bg-[#161616] px-3 py-2"
        onSubmit={(e) => {
          e.preventDefault();
          navigate(inputValue());
        }}
      >
        <input
          type="text"
          value={inputValue()}
          onInput={(e) => setInputValue(e.currentTarget.value)}
          placeholder="Enter URL..."
          class="h-9 flex-1 rounded-md border border-neutral-700 bg-neutral-900 px-3 text-sm text-neutral-200 outline-none focus:border-neutral-500"
        />
        <button
          type="button"
          onClick={reload}
          class="inline-flex h-9 w-9 items-center justify-center rounded-md border border-neutral-700 bg-neutral-900 text-neutral-300 hover:bg-neutral-800"
          title="Reload"
          aria-label="Reload page"
        >
          <TbOutlineReload size={16} />
        </button>
        <button
          type="submit"
          class="inline-flex h-9 items-center gap-1 rounded-md border border-blue-500/60 bg-blue-600/20 px-3 text-xs font-medium text-blue-100 hover:bg-blue-600/30"
        >
          Go
        </button>
        <button
          type="button"
          onClick={closeCurrent}
          class="inline-flex h-9 w-9 items-center justify-center rounded-md border border-rose-500/40 bg-rose-600/15 text-rose-200 hover:bg-rose-600/25"
          title="Close browser window"
          aria-label="Close browser window"
        >
          <TbOutlineX size={16} />
        </button>
      </form>

      <div class="flex min-h-0 flex-1 flex-col">
        <div class="flex items-center justify-between border-b border-neutral-800 bg-[#131313] px-3 py-1.5 text-xs text-neutral-400">
          <span class="truncate">{url()}</span>
          <span class="rounded-full border border-sky-500/40 bg-sky-500/10 px-2 py-0.5 text-[10px] uppercase tracking-wide text-sky-200">
            via backend
          </span>
          <a
            href={url()}
            target="_blank"
            rel="noreferrer"
            class="inline-flex items-center gap-1 text-neutral-300 hover:text-white"
          >
            Open in new tab
            <TbOutlineExternalLink size={12} />
          </a>
        </div>

        <ContextMenu>
          <ContextMenuTrigger class="min-h-0 flex-1 bg-black">
            <iframe
              key={frameKey()}
              src={toProxyUrl(url())}
              title="Browser"
              class="h-full w-full border-0"
              referrerPolicy="strict-origin-when-cross-origin"
              sandbox="allow-forms allow-modals allow-popups allow-same-origin allow-scripts allow-downloads"
            />
          </ContextMenuTrigger>
          <ContextMenuContent class="w-56">
            <ContextMenuItem onSelect={reload}>
              Reload
              <ContextMenuShortcut>R</ContextMenuShortcut>
            </ContextMenuItem>
            <ContextMenuItem
              onSelect={() => {
                window.open(url(), "_blank", "noopener,noreferrer");
              }}
            >
              Open in new tab
              <ContextMenuShortcut>↗</ContextMenuShortcut>
            </ContextMenuItem>
            <ContextMenuItem onSelect={copyUrl}>
              Copy URL
              <ContextMenuShortcut>⌘C</ContextMenuShortcut>
            </ContextMenuItem>
            <ContextMenuSeparator />
            <ContextMenuItem variant="destructive" onSelect={closeCurrent}>
              Close window
              <ContextMenuShortcut>Esc</ContextMenuShortcut>
            </ContextMenuItem>
          </ContextMenuContent>
        </ContextMenu>

        <Show when={!url().startsWith("https://") && !url().startsWith("http://")}> 
          <div class="border-t border-amber-500/30 bg-amber-600/10 px-3 py-2 text-xs text-amber-100">
            URL normalized to HTTPS.
          </div>
        </Show>
      </div>
    </div>
  );
}

export default BrowserApp;
