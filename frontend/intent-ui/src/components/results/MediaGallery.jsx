import { createSignal, Show, For, onMount } from "solid-js";
import { Motion } from "solid-motionone";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import {
  TbOutlinePhoto,
  TbOutlineX,
  TbOutlineZoomIn,
  TbOutlineZoomOut,
  TbOutlineChevronLeft,
  TbOutlineChevronRight,
  TbOutlineDownload
} from "solid-icons/tb";
function cn(...classes) {
  return twMerge(clsx(classes));
}
function parseMediaData(data) {
  if (Array.isArray(data)) {
    return data.map((item) => {
      if (typeof item === "string") {
        return {
          id: item,
          url: item,
          type: item.match(/\.(mp4|webm|ogg)$/i) ? "video" : "image"
        };
      }
      return item;
    });
  }
  if (data?.media && Array.isArray(data.media)) {
    return data.media;
  }
  if (data?.images && Array.isArray(data.images)) {
    return data.images.map((img) => ({ ...img, type: "image" }));
  }
  return [];
}
function formatFileSize(bytes) {
  if (!bytes) return "";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
}
function MediaGallery(props) {
  const ui = () => props.response.ui;
  const [selectedMedia, setSelectedMedia] = createSignal(null);
  const [zoom, setZoom] = createSignal(1);
  const [media, setMedia] = createSignal([]);
  onMount(() => {
    const parsed = parseMediaData(props.response.data);
    setMedia(parsed);
  });
  const currentIndex = () => {
    const selected = selectedMedia();
    if (!selected) return -1;
    return media().findIndex((m) => m.id === selected.id);
  };
  const navigate = (direction) => {
    const items = media();
    const current = currentIndex();
    if (current === -1) return;
    let newIndex = current + direction;
    if (newIndex < 0) newIndex = items.length - 1;
    if (newIndex >= items.length) newIndex = 0;
    setSelectedMedia(items[newIndex]);
    setZoom(1);
  };
  const downloadMedia = async (item) => {
    try {
      const response = await fetch(item.url);
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = item.title || `media-${item.id}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      window.open(item.url, "_blank");
    }
  };
  const closeLightbox = () => {
    setSelectedMedia(null);
    setZoom(1);
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
            <TbOutlinePhoto size={18} class="text-blue-400" />
            <Show when={ui()?.title}>
              <h3 class="text-sm font-medium text-white">{ui().title}</h3>
            </Show>
            <Show when={media().length}>
              <span class="text-xs text-neutral-500">
                {media().length} items
              </span>
            </Show>
          </div>
        </div>
      </div>

      {
    /* Gallery Grid */
  }
      <div class="p-4">
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3">
          <For each={media()}>
            {(item) => <Motion.div
    initial={{ opacity: 0, scale: 0.9 }}
    animate={{ opacity: 1, scale: 1 }}
    exit={{ opacity: 0, scale: 0.9 }}
    hover={{ scale: 1.05 }}
    class={cn(
      "relative aspect-square rounded-lg overflow-hidden cursor-pointer bg-neutral-900 group",
      item.type === "video" && "ring-2 ring-purple-500"
    )}
    onClick={() => setSelectedMedia(item)}
  >
                {
    /* Thumbnail */
  }
                <Show
    when={item.thumbnail}
    fallback={<div class="w-full h-full flex items-center justify-center text-neutral-600">
                      <TbOutlinePhoto size={32} />
                    </div>}
  >
                  <img
    src={item.thumbnail || item.url}
    alt={item.title || "Media"}
    class="w-full h-full object-cover"
  />
                </Show>
                
                {
    /* Type indicator */
  }
                <Show when={item.type === "video"}>
                  <div class="absolute top-2 right-2 px-2 py-1 bg-purple-600 rounded text-xs text-white">
                    Video
                  </div>
                </Show>
                
                {
    /* Hover overlay */
  }
                <div class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
                  <TbOutlineZoomIn size={24} class="text-white" />
                </div>
                
                {
    /* Title */
  }
                <Show when={item.title}>
                  <div class="absolute bottom-0 left-0 right-0 p-2 bg-gradient-to-t from-black/80 to-transparent">
                    <div class="text-xs text-white truncate">{item.title}</div>
                  </div>
                </Show>
              </Motion.div>}
          </For>
        </div>

        {
    /* Empty state */
  }
        <Show when={media().length === 0}>
          <div class="text-center py-12 text-neutral-500">
            <TbOutlinePhoto size={48} class="mx-auto mb-3 opacity-50" />
            <p class="text-sm">No media files</p>
          </div>
        </Show>
      </div>

      {
    /* Lightbox Modal */
  }
      <Show when={selectedMedia()}>
        <Motion.div
    initial={{ opacity: 0 }}
    animate={{ opacity: 1 }}
    exit={{ opacity: 0 }}
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/90"
    onClick={closeLightbox}
  >
            {
    /* Close button */
  }
            <button
    type="button"
    onClick={closeLightbox}
    class="absolute top-4 right-4 p-2 text-white hover:bg-white/10 rounded-full transition-colors z-10 cursor-pointer focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-black"
    aria-label="Close lightbox"
  >
              <TbOutlineX size={24} />
            </button>

            {
    /* Navigation */
  }
            <button
    type="button"
    onClick={(e) => {
      e.stopPropagation();
      navigate(-1);
    }}
    class="absolute left-4 p-2 text-white hover:bg-white/10 rounded-full transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-black"
    aria-label="Previous image"
  >
              <TbOutlineChevronLeft size={32} />
            </button>
            <button
    type="button"
    onClick={(e) => {
      e.stopPropagation();
      navigate(1);
    }}
    class="absolute right-4 p-2 text-white hover:bg-white/10 rounded-full transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-black"
    aria-label="Next image"
  >
              <TbOutlineChevronRight size={32} />
            </button>

            {
    /* Zoom controls */
  }
            <div class="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-2 bg-black/50 rounded-full px-4 py-2" role="group" aria-label="Zoom controls">
              <button
    type="button"
    onClick={(e) => {
      e.stopPropagation();
      setZoom(Math.max(0.5, zoom() - 0.25));
    }}
    class="p-1 text-white hover:bg-white/10 rounded cursor-pointer focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-black"
    aria-label="Zoom out"
  >
                <TbOutlineZoomOut size={20} />
              </button>
              <span class="text-sm text-white min-w-[48px] text-center">
                {Math.round(zoom() * 100)}%
              </span>
              <button
    type="button"
    onClick={(e) => {
      e.stopPropagation();
      setZoom(Math.min(3, zoom() + 0.25));
    }}
    class="p-1 text-white hover:bg-white/10 rounded cursor-pointer focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-black"
    aria-label="Zoom in"
  >
                <TbOutlineZoomIn size={20} />
              </button>
              <button
    type="button"
    onClick={(e) => {
      e.stopPropagation();
      downloadMedia(selectedMedia());
    }}
    class="p-1 text-white hover:bg-white/10 rounded cursor-pointer focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-black ml-2"
    aria-label="Download media"
  >
                <TbOutlineDownload size={20} />
              </button>
            </div>
            
            {
    /* Media content */
  }
            <Motion.div
    initial={{ scale: 0.9, opacity: 0 }}
    animate={{ scale: zoom(), opacity: 1 }}
    transition={{ duration: 0.2 }}
    class="max-w-[90vw] max-h-[80vh] overflow-auto"
    onClick={(e) => e.stopPropagation()}
  >
              <Show
    when={selectedMedia()?.type === "video"}
    fallback={<img
      src={selectedMedia().url}
      alt={selectedMedia()?.title || "Media"}
      class="max-w-full max-h-[80vh] object-contain"
    />}
  >
                <video
    src={selectedMedia().url}
    controls
    class="max-w-full max-h-[80vh] object-contain"
    autoplay
  />
              </Show>
            </Motion.div>
            
            {
    /* Info */
  }
            <Show when={selectedMedia()?.title || selectedMedia()?.description}>
              <div class="absolute top-4 left-1/2 -translate-x-1/2 text-center text-white max-w-md">
                <Show when={selectedMedia()?.title}>
                  <h3 class="text-sm font-medium">{selectedMedia().title}</h3>
                </Show>
                <Show when={selectedMedia()?.description}>
                  <p class="text-xs text-neutral-400 mt-1">{selectedMedia().description}</p>
                </Show>
              </div>
            </Show>
          </Motion.div>
        </Show>
    </Motion.div>;
}
export {
  MediaGallery
};
