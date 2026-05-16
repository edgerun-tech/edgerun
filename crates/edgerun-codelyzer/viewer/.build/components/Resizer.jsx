import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div class="w-1 bg-border-default hover:bg-accent-blue cursor-col-resize shrink-0 transition-colors z-40">`);
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
  return (() => {
    var _el$ = _tmpl$();
    _el$.$$mousedown = onMouseDown;
    return _el$;
  })();
}
_$delegateEvents(["mousedown"]);