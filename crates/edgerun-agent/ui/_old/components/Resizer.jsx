/** @jsxImportSource solid-js **/
/** @jsxImportSource solid-js **/
/**
 * Resizable panel divider component.
 */

export function Resizer(props) {
  let startX = 0;
  let startWidth = 0;
  function onMouseDown(e) {
    e.preventDefault();
    startX = e.clientX;
    startWidth = props.width;
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }
  function onMouseMove(e) {
    const dx = startX - e.clientX;
    const newWidth = Math.max(props.min, Math.min(props.max, startWidth + dx));
    props.onResize(Math.round(newWidth));
  }
  function onMouseUp() {
    document.removeEventListener("mousemove", onMouseMove);
    document.removeEventListener("mouseup", onMouseUp);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }
  return <div class="w-1 bg-border-default hover:bg-accent-blue cursor-col-resize shrink-0 transition-colors z-40" onMouseDown={onMouseDown} />;
}
// Benchmark comment