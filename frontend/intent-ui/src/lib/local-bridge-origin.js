function normalizePath(pathname = "") {
  const value = String(pathname || "").trim();
  if (!value) return "/";
  return value.startsWith("/") ? value : `/${value}`;
}

function resolveBridgeOrigin() {
  if (typeof window === "undefined") {
    return {
      httpOrigin: "http://127.0.0.1:7777",
      wsOrigin: "ws://127.0.0.1:7777"
    };
  }
  const protocol = window.location.protocol === "https:" ? "https:" : "http:";
  const wsProtocol = protocol === "https:" ? "wss:" : "ws:";
  const host = window.location.host;
  return {
    httpOrigin: `${protocol}//${host}`,
    wsOrigin: `${wsProtocol}//${host}`
  };
}

function localBridgeHttpUrl(pathname) {
  const { httpOrigin } = resolveBridgeOrigin();
  return `${httpOrigin}${normalizePath(pathname)}`;
}

function localBridgeWsUrl(pathname) {
  const { wsOrigin } = resolveBridgeOrigin();
  return `${wsOrigin}${normalizePath(pathname)}`;
}

export { localBridgeHttpUrl, localBridgeWsUrl };
