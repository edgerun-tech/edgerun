import { TbOutlineGripVertical } from "solid-icons/tb";
import { Show, createSignal, onCleanup, onMount } from "solid-js";
import { Portal } from "solid-js/web";
import { windows, closeWindow, minimizeWindow, maximizeWindow, restoreWindow, updateWindowPosition, updateWindowSize, bringWindowToFront, getActiveWindowId, getWindowLayerOffset } from "../../stores/windows";
const defaultPositions = {
  editor: { x: 100, y: 80 },
  files: { x: 150, y: 120 },
  integrations: { x: 200, y: 160 },
  settings: { x: 250, y: 200 },
  onvif: { x: 180, y: 120 },
  credentials: { x: 280, y: 210 },
  guide: { x: 220, y: 130 },
  prompt: { x: 300, y: 240 },
  products: { x: 120, y: 100 },
  github: { x: 180, y: 140 }
};
const getInitialPosition = (id) => defaultPositions[id] || { x: 100 + Math.random() * 200, y: 80 + Math.random() * 150 };
function Window(props) {
  const getStoredState = () => {
    const stored = windows()[props.id];
    return {
      position: stored?.position || getInitialPosition(props.id),
      size: stored?.size || { width: 600, height: 400 }
    };
  };
  const [position, setPosition] = createSignal(getStoredState().position);
  const [size, setSize] = createSignal(getStoredState().size);
  const [isMaximized, setIsMaximized] = createSignal(false);
  const [wasMaximized, setWasMaximized] = createSignal(false);
  const [dragState, setDragState] = createSignal({
    isDragging: false,
    startX: 0,
    startY: 0,
    initialLeft: 0,
    initialTop: 0
  });
  const isWindowActive = () => getActiveWindowId()() === props.id;
  const layerOffset = getWindowLayerOffset();
  const windowState = () => windows()[props.id];
  const isOpen = () => windowState()?.isOpen ?? false;
  const storeIsMaximized = () => windowState()?.isMaximized ?? false;
  const isMinimized = () => windowState()?.isMinimized ?? false;
  const handleClose = () => closeWindow(props.id);
  const handleMinimize = () => minimizeWindow(props.id);
  const handleMaximize = () => {
    if (!isMaximized()) {
      setWasMaximized(false);
      maximizeWindow(props.id);
    } else {
      restoreWindow(props.id);
    }
  };
  const handleMouseDown = (e) => {
    if (isMaximized()) return;
    if (e.ctrlKey) return;
    const target = e.target;
    if (target.closest(".window-controls")) return;
    e.preventDefault();
    bringWindowToFront(props.id);
    setDragState({
      isDragging: true,
      startX: e.clientX,
      startY: e.clientY,
      initialLeft: position().x,
      initialTop: position().y
    });
  };
  const handleMouseMove = (e) => {
    if (dragState().isDragging) {
      const deltaX = e.clientX - dragState().startX;
      const deltaY = e.clientY - dragState().startY;
      setPosition({
        x: dragState().initialLeft + deltaX,
        y: dragState().initialTop + deltaY
      });
    }
    if (isResizing()) {
      const deltaX = e.clientX - resizeState().startX;
      const deltaY = e.clientY - resizeState().startY;
      let newWidth = size().width;
      let newHeight = size().height;
      if (resizeState().axis.includes("e")) {
        newWidth = Math.max(300, resizeState().initialWidth + deltaX);
      }
      if (resizeState().axis.includes("w")) {
        newWidth = Math.max(300, resizeState().initialWidth - deltaX);
      }
      if (resizeState().axis.includes("s")) {
        newHeight = Math.max(200, resizeState().initialHeight + deltaY);
      }
      if (resizeState().axis.includes("n")) {
        newHeight = Math.max(200, resizeState().initialHeight - deltaY);
      }
      setSize({ width: newWidth, height: newHeight });
    }
  };
  const handleMouseUp = () => {
    if (dragState().isDragging) {
      updateWindowPosition(props.id, position());
    }
    setDragState((prev) => ({ ...prev, isDragging: false }));
    if (isResizing()) {
      setIsResizing(false);
      updateWindowSize(props.id, size());
    }
  };
  let moveHandler;
  let upHandler;
  onMount(() => {
    moveHandler = (e) => handleMouseMove(e);
    upHandler = () => handleMouseUp();
    document.addEventListener("mousemove", moveHandler);
    document.addEventListener("mouseup", upHandler);
  });
  onCleanup(() => {
    if (moveHandler) document.removeEventListener("mousemove", moveHandler);
    if (upHandler) document.removeEventListener("mouseup", upHandler);
  });
  const [isResizing, setIsResizing] = createSignal(false);
  const [resizeState, setResizeState] = createSignal({
    startX: 0,
    startY: 0,
    initialWidth: 0,
    initialHeight: 0,
    axis: ""
  });
  const handleResizeStart = (e, axis) => {
    if (isMaximized()) return;
    e.preventDefault();
    e.stopPropagation();
    setIsResizing(true);
    setResizeState({
      startX: e.clientX,
      startY: e.clientY,
      initialWidth: size().width,
      initialHeight: size().height,
      axis
    });
  };
  const handleWindowPointerDown = (event) => {
    const target = event.target;
    if (target.closest(".window-controls")) return;
    if (!isWindowActive()) {
      bringWindowToFront(props.id);
    }
  };
  return <Show when={isOpen() && !isMinimized()}>
      <Portal mount={document.body}>
        <section
    class="fixed rounded-xl overflow-hidden shadow-2xl border border-neutral-700/50 bg-[#1a1a1a] select-none"
    style={{
      left: isMaximized() ? "0" : `${position().x + layerOffset().x}px`,
      top: isMaximized() ? "0" : `${position().y + layerOffset().y}px`,
      width: isMaximized() ? "100vw" : `${size().width}px`,
      height: isMaximized() ? "100vh" : `${size().height}px`,
      "z-index": windowState()?.zIndex ?? (isWindowActive() ? 1100 : 1000)
    }}
    onMouseDown={handleWindowPointerDown}
    role="dialog"
    aria-modal="true"
    aria-label={`${props.title} window`}
  >
          {
    /* Title Bar */
  }
          <div
    class="h-10 flex items-center justify-between px-4"
    style={{ background: "linear-gradient(to bottom, #2d2d2d, #252525)" }}
  >
            <button
    type="button"
    class="flex items-center gap-2 text-neutral-300 flex-1 cursor-grab active:cursor-grabbing text-left"
    onMouseDown={handleMouseDown}
    aria-label="Drag window"
  >
              <TbOutlineGripVertical size={16} class="text-neutral-500" />
              <span class="text-sm font-medium">{props.title}</span>
            </button>

            <div class="window-controls flex items-center gap-1.5">
              <button
    type="button"
    onClick={handleMinimize}
    class="w-3 h-3 rounded-full bg-yellow-500 hover:bg-yellow-400 transition-colors"
    title="Minimize"
    aria-label="Minimize window"
  />
              <button
    type="button"
    onClick={handleMaximize}
    class="w-3 h-3 rounded-full bg-green-500 hover:bg-green-400 transition-colors"
    title={isMaximized() ? "Restore" : "Maximize"}
    aria-label={isMaximized() ? "Restore window" : "Maximize window"}
  />
              <button
    type="button"
    onClick={handleClose}
    class="w-3 h-3 rounded-full bg-red-500 hover:bg-red-400 transition-colors"
    title="Close"
    aria-label="Close window"
  />
            </div>
          </div>

          {
    /* Content */
  }
          <div class="flex-1 overflow-auto" style={{ height: "calc(100% - 40px)" }}>
            {props.children}
          </div>

          {
    /* Resize Handles */
  }
          <Show when={!isMaximized()}>
            {
    /* South */
  }
            <button
    type="button"
    class="absolute bottom-0 left-4 right-4 h-1 cursor-s-resize bg-transparent"
    onMouseDown={(e) => handleResizeStart(e, "s")}
    aria-label="Resize south"
  />
            {
    /* East */
  }
            <button
    type="button"
    class="absolute top-10 bottom-4 right-0 w-1 cursor-e-resize bg-transparent"
    onMouseDown={(e) => handleResizeStart(e, "e")}
    aria-label="Resize east"
  />
            {
    /* South-East */
  }
            <button
    type="button"
    class="absolute bottom-0 right-0 w-4 h-4 cursor-se-resize bg-transparent"
    onMouseDown={(e) => handleResizeStart(e, "se")}
    aria-label="Resize south-east"
  />
          </Show>
        </section>
      </Portal>
    </Show>;
}
export {
  Window as default
};
