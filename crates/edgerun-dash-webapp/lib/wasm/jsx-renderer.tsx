import { useEffect, useRef, useCallback } from "react"

interface JsxRendererProps {
  jsx: string
  onAction: (action: string) => void
  className?: string
}

const FORBIDDEN_ATTRS = [
  "onload", "onerror", "onmouseover", "onclick", "onfocus", "onblur",
  "onchange", "onsubmit", "onkeydown", "onkeyup", "onkeypress",
  "onmousedown", "onmouseup", "onmousemove", "onmouseout", "onmouseenter",
  "onmouseleave", "ondblclick", "oncontextmenu", "onwheel", "ondrag",
  "ondragend", "ondragenter", "ondragleave", "ondragover", "ondragstart",
  "ondrop", "onscroll", "oninput", "oninvalid", "onreset", "onsearch",
  "onselect", "ontoggle", "onpointerdown", "onpointerup", "onpointermove",
  "onpointerenter", "onpointerleave", "ontouchstart", "ontouchend",
  "ontouchmove", "onabort", "oncanplay", "oncanplaythrough", "ondurationchange",
  "onemptied", "onended", "onloadeddata", "onloadedmetadata", "onloadstart",
  "onpause", "onplay", "onplaying", "onprogress", "onratechange", "onseeked",
  "onseeking", "onstalled", "onsuspend", "ontimeupdate", "onvolumechange",
  "onwaiting", "onanimationstart", "onanimationend", "onanimationiteration",
  "ontransitionend", "oncut", "oncopy", "onpaste", "onresize", "onstorage",
  "onunload", "onbeforeunload", "onhashchange", "onpopstate",
]

const FORBIDDEN_TAGS = ["script", "iframe", "object", "embed", "form", "link", "style", "meta", "base"]

function sanitizeHtml(html: string): string {
  let sanitized = html

  FORBIDDEN_TAGS.forEach((tag) => {
    const regex = new RegExp(`<${tag}[\\s>][\\s\\S]*?</${tag}>`, "gi")
    sanitized = sanitized.replace(regex, "")
    const selfCloseRegex = new RegExp(`<${tag}[\\s\\S]*?/?>`, "gi")
    sanitized = sanitized.replace(selfCloseRegex, "")
  })

  FORBIDDEN_ATTRS.forEach((attr) => {
    const regex = new RegExp(`\\s${attr}\\s*=\\s*["'][^"']*["']`, "gi")
    sanitized = sanitized.replace(regex, "")
    const regex2 = new RegExp(`\\s${attr}\\s*=\\s*[^\\s>]+`, "gi")
    sanitized = sanitized.replace(regex2, "")
  })

  sanitized = sanitized.replace(/javascript\s*:/gi, "blocked:")
  sanitized = sanitized.replace(/vbscript\s*:/gi, "blocked:")
  sanitized = sanitized.replace(/data\s*:/gi, "blocked:")

  return sanitized
}

export function JsxRenderer({ jsx, onAction, className }: JsxRendererProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null)

  const handleClick = useCallback((action: string) => {
    onAction(action)
  }, [onAction])

  useEffect(() => {
    const iframe = iframeRef.current
    if (!iframe) return

    const sanitized = sanitizeHtml(jsx)

    const fullHtml = `
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <script src="https://cdn.tailwindcss.com"></script>
  <script>
    tailwind.config = {
      corePlugins: {
        preflight: false,
      },
      theme: {
        extend: {},
      },
    }
  </script>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    html, body { height: 100%; background: transparent; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
    [data-action] { cursor: pointer; }
  </style>
</head>
<body>
${sanitized}
<script>
  (function() {
    function findAction(el) {
      while (el && el !== document.body) {
        if (el.hasAttribute && el.hasAttribute('data-action')) {
          return el.getAttribute('data-action');
        }
        el = el.parentElement;
      }
      return null;
    }
    document.addEventListener('click', function(e) {
      var action = findAction(e.target);
      if (action) {
        window.parent.postMessage({ type: 'wasm-action', action: action }, '*');
      }
    });
    document.addEventListener('input', function(e) {
      var el = e.target;
      if (el && el.hasAttribute && el.hasAttribute('data-input-action')) {
        window.parent.postMessage({ type: 'wasm-input', action: el.getAttribute('data-input-action'), value: el.value }, '*');
      }
    });
  })();
</script>
</body>
</html>
    `

    iframe.srcdoc = fullHtml
  }, [jsx])

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      if (event.data?.type === "wasm-action" && event.data.action) {
        handleClick(event.data.action)
      } else if (event.data?.type === "wasm-input" && event.data.action) {
        handleClick(`input:${event.data.value || ""}:${event.data.action}`)
      }
    }
    window.addEventListener("message", handler)
    return () => window.removeEventListener("message", handler)
  }, [handleClick])

  return (
    <iframe
      ref={iframeRef}
      className={className ?? "h-full w-full border-0 bg-transparent"}
      sandbox="allow-same-origin"
      title="wasm-app"
    />
  )
}
