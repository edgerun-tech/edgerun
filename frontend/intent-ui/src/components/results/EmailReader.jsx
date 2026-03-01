import { createSignal, createMemo, Show, For, onMount } from "solid-js";
import { Motion } from "solid-motionone";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import {
  TbOutlineMail,
  TbOutlineStar,
  TbOutlineMessage,
  TbOutlineShare,
  TbOutlineTrash,
  TbOutlineArchive
} from "solid-icons/tb";
function cn(...classes) {
  return twMerge(clsx(classes));
}
function parseEmailData(data) {
  if (Array.isArray(data)) {
    return data;
  }
  if (data?.messages && Array.isArray(data.messages)) {
    return data.messages;
  }
  return [];
}
function formatEmailDate(date) {
  const d = new Date(date);
  const now = /* @__PURE__ */ new Date();
  const diff = now.getTime() - d.getTime();
  const days = Math.floor(diff / (1e3 * 60 * 60 * 24));
  if (days === 0) return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  if (days === 1) return "Yesterday";
  if (days < 7) return d.toLocaleDateString([], { weekday: "short" });
  return d.toLocaleDateString([], { month: "short", day: "numeric", year: "numeric" });
}
function stripHtml(html) {
  const tmp = document.createElement("div");
  tmp.innerHTML = html;
  return tmp.textContent || tmp.innerText || "";
}
function EmailReader(props) {
  const ui = () => props.response.ui;
  const [selectedEmail, setSelectedEmail] = createSignal(null);
  const [starredOnly, setStarredOnly] = createSignal(false);
  const [searchQuery, setSearchQuery] = createSignal("");
  const [emails, setEmails] = createSignal([]);
  onMount(() => {
    const parsed = parseEmailData(props.response.data);
    setEmails(parsed);
    if (parsed.length > 0) {
      setSelectedEmail(parsed[0]);
    }
  });
  const filteredEmails = createMemo(() => {
    let result = emails();
    if (starredOnly()) {
      result = result.filter((e) => e.isStarred);
    }
    const query = searchQuery().toLowerCase();
    if (query) {
      result = result.filter(
        (e) => e.subject.toLowerCase().includes(query) || e.from.name.toLowerCase().includes(query) || e.from.email.toLowerCase().includes(query) || e.snippet?.toLowerCase().includes(query)
      );
    }
    return result;
  });
  const toggleStar = (id) => {
    setEmails((prev) => prev.map(
      (e) => e.id === id ? { ...e, isStarred: !e.isStarred } : e
    ));
  };
  const unreadCount = createMemo(() => emails().filter((e) => !e.isRead).length);
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
            <TbOutlineMail size={18} class="text-blue-400" />
            <Show when={ui()?.title}>
              <h3 class="text-sm font-medium text-white">{ui().title}</h3>
            </Show>
            <Show when={unreadCount() > 0}>
              <span class="text-xs px-2 py-0.5 bg-blue-600 text-white rounded-full">
                {unreadCount()} new
              </span>
            </Show>
          </div>
          
          <button
    type="button"
    onClick={() => setStarredOnly(!starredOnly())}
    class={cn(
      "p-1.5 rounded transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-yellow-500 focus:ring-offset-2 focus:ring-offset-neutral-800",
      starredOnly() ? "text-yellow-400 bg-yellow-900/20" : "text-neutral-400 hover:text-yellow-400 hover:bg-neutral-700"
    )}
    title="Show starred only"
    aria-label="Show starred only"
    aria-pressed={starredOnly()}
  >
            {starredOnly() ? <TbOutlineStar size={18} class="text-yellow-400 fill-yellow-400" /> : <TbOutlineStar size={18} />}
          </button>
        </div>
        
        {
    /* Search */
  }
        <div class="mt-3">
          <input
    type="text"
    value={searchQuery()}
    onInput={(e) => setSearchQuery(e.currentTarget.value)}
    placeholder="Search emails..."
    class="w-full px-4 py-1.5 bg-neutral-900 border border-neutral-700 rounded-lg text-sm text-neutral-300 placeholder-neutral-500 focus:outline-none focus:border-neutral-600"
  />
        </div>
      </div>

      {
    /* Content */
  }
      <div class="flex h-[500px]">
        {
    /* Email list */
  }
        <div class="w-80 border-r border-neutral-700 overflow-y-auto">
          <For each={filteredEmails()}>
            {(email) => <div
    class={cn(
      "p-3 border-b border-neutral-800 cursor-pointer transition-colors hover:bg-neutral-800/50",
      selectedEmail()?.id === email.id ? "bg-neutral-800" : "",
      !email.isRead && "bg-blue-900/10"
    )}
    onClick={() => setSelectedEmail(email)}
  >
                <div class="flex items-start justify-between gap-2 mb-1">
                  <span class={cn(
    "text-sm truncate",
    !email.isRead ? "text-white font-medium" : "text-neutral-300"
  )}>
                    {email.from.name || email.from.email}
                  </span>
                  <button
    type="button"
    onClick={(e) => {
      e.stopPropagation();
      toggleStar(email.id);
    }}
    class={cn(
      "flex-shrink-0 transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-yellow-500 focus:ring-offset-2 focus:ring-offset-neutral-800"
    )}
    aria-label={email.isStarred ? "Remove star" : "Add star"}
    aria-pressed={email.isStarred}
  >
                    {email.isStarred ? <TbOutlineStar size={14} class="text-yellow-400 fill-yellow-400" /> : <TbOutlineStar size={14} />}
                  </button>
                </div>
                
                <div class="text-sm text-neutral-300 truncate mb-1">{email.subject}</div>
                
                <div class="flex items-center justify-between gap-2">
                  <div class="text-xs text-neutral-500 truncate flex-1">
                    {email.snippet || stripHtml(email.body).slice(0, 50)}...
                  </div>
                  <span class="text-xs text-neutral-500 flex-shrink-0">
                    {formatEmailDate(email.date)}
                  </span>
                </div>
              </div>}
          </For>
          
          <Show when={filteredEmails().length === 0}>
            <div class="p-8 text-center text-neutral-500">
              <TbOutlineMail size={32} class="mx-auto mb-2 opacity-50" />
              <p class="text-sm">No emails found</p>
            </div>
          </Show>
        </div>

        {
    /* Email detail */
  }
        <div class="flex-1 overflow-y-auto bg-neutral-900/30">
          <Show
    when={selectedEmail()}
    fallback={<div class="h-full flex items-center justify-center text-neutral-500">
                <div class="text-center">
                  <TbOutlineMail size={48} class="mx-auto mb-3 opacity-50" />
                  <p class="text-sm">Select an email to read</p>
                </div>
              </div>}
  >
            <div class="p-6">
              {
    /* Subject */
  }
              <h2 class="text-lg font-medium text-white mb-4">
                {selectedEmail().subject}
              </h2>
              
              {
    /* From/To */
  }
              <div class="flex items-start justify-between mb-4 pb-4 border-b border-neutral-700">
                <div class="flex items-center gap-3">
                  <div class="w-10 h-10 rounded-full bg-blue-600 flex items-center justify-center text-white font-medium">
                    {selectedEmail().from.name.charAt(0).toUpperCase()}
                  </div>
                  <div>
                    <div class="text-sm text-white">{selectedEmail().from.name}</div>
                    <div class="text-xs text-neutral-500">{selectedEmail().from.email}</div>
                  </div>
                </div>
                <div class="text-sm text-neutral-500">
                  {formatEmailDate(selectedEmail().date)}
                </div>
              </div>
              
              {
    /* Body */
  }
              <div
    class="prose prose-invert prose-sm max-w-none text-neutral-300 mb-6"
    innerHTML={selectedEmail().body}
  />
              
              {
    /* Attachments */
  }
              <Show when={selectedEmail().attachments?.length}>
                <div class="border-t border-neutral-700 pt-4 mb-4">
                  <h4 class="text-sm font-medium text-neutral-400 mb-2">Attachments</h4>
                  <div class="flex flex-wrap gap-2">
                    <For each={selectedEmail().attachments}>
                      {(att) => <div class="flex items-center gap-2 px-3 py-2 bg-neutral-800 rounded-lg text-sm">
                          <TbOutlineMail size={16} class="text-neutral-400" />
                          <span class="text-neutral-300">{att.name}</span>
                          <span class="text-xs text-neutral-500">
                            ({Math.round(att.size / 1024)} KB)
                          </span>
                        </div>}
                    </For>
                  </div>
                </div>
              </Show>
              
              {
    /* Actions */
  }
              <div class="flex items-center gap-2 pt-4 border-t border-neutral-700" role="group" aria-label="Email actions">
                <button
    type="button"
    onClick={() => props.onAction?.(`reply to ${selectedEmail().from.email}`)}
    class="flex items-center gap-1.5 px-3 py-1.5 bg-neutral-700 hover:bg-neutral-600 rounded-lg text-sm text-neutral-300 transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800"
  >
                  <TbOutlineMessage size={16} />
                  Reply
                </button>
                <button
    type="button"
    onClick={() => props.onAction?.(`forward email: ${selectedEmail().subject}`)}
    class="flex items-center gap-1.5 px-3 py-1.5 bg-neutral-700 hover:bg-neutral-600 rounded-lg text-sm text-neutral-300 transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800"
  >
                  <TbOutlineShare size={16} />
                  Forward
                </button>
                <button
    type="button"
    class="flex items-center gap-1.5 px-3 py-1.5 bg-neutral-700 hover:bg-red-600 rounded-lg text-sm text-neutral-300 hover:text-white transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 focus:ring-offset-neutral-800"
    aria-label="Delete email"
  >
                  <TbOutlineTrash size={16} />
                  Delete
                </button>
                <button
    type="button"
    class="flex items-center gap-1.5 px-3 py-1.5 bg-neutral-700 hover:bg-neutral-600 rounded-lg text-sm text-neutral-300 transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-neutral-800 ml-auto"
    aria-label="Archive email"
  >
                  <TbOutlineArchive size={16} />
                  Archive
                </button>
              </div>
            </div>
          </Show>
        </div>
      </div>
    </Motion.div>;
}
export {
  EmailReader
};
