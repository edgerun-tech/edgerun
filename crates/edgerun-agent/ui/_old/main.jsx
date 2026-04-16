/** @jsxImportSource solid-js **/
/** @jsxImportSource solid-js **/
/**
 * Main application entry point — bootstraps Solid.js app.
 */
import { render } from "solid-js/web";
import { App } from "./app.jsx";

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const root = document.getElementById("app-root");
render(() => <App />, root);
console.log("[viewer] App mounted");
// Benchmark comment