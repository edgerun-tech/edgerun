import { render } from "solid-js/web";
import App from "./App";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing root element");
}

// Client-only boot to avoid SSR hydration mismatches in dynamic/portal-heavy UI.
root.textContent = "";
render(() => <App />, root);
