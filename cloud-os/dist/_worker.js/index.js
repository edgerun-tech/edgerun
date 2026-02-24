globalThis.process ??= {}; globalThis.process.env ??= {};
import { r as renderers } from './chunks/_@astro-renderers_B30lzduo.mjs';
import { c as createExports, s as serverEntrypointModule } from './chunks/_@astrojs-ssr-adapter_BXu_WkfK.mjs';
import { manifest } from './manifest_BQIqgb2m.mjs';

const serverIslandMap = new Map();;

const _page0 = () => import('./pages/_image.astro.mjs');
const _page1 = () => import('./pages/api/codex/execute.astro.mjs');
const _page2 = () => import('./pages/api/fs.astro.mjs');
const _page3 = () => import('./pages/api/qwen/callback.astro.mjs');
const _page4 = () => import('./pages/api/qwen/chat.astro.mjs');
const _page5 = () => import('./pages/api/qwen/poll.astro.mjs');
const _page6 = () => import('./pages/api/qwen/token.astro.mjs');
const _page7 = () => import('./pages/api/qwen.astro.mjs');
const _page8 = () => import('./pages/call/_id_.astro.mjs');
const _page9 = () => import('./pages/index.astro.mjs');
const pageMap = new Map([
    ["node_modules/@astrojs/cloudflare/dist/entrypoints/image-endpoint.js", _page0],
    ["src/pages/api/codex/execute.ts", _page1],
    ["src/pages/api/fs/index.ts", _page2],
    ["src/pages/api/qwen/callback.ts", _page3],
    ["src/pages/api/qwen/chat.ts", _page4],
    ["src/pages/api/qwen/poll.ts", _page5],
    ["src/pages/api/qwen/token.ts", _page6],
    ["src/pages/api/qwen/index.ts", _page7],
    ["src/pages/call/[id].astro", _page8],
    ["src/pages/index.astro", _page9]
]);

const _manifest = Object.assign(manifest, {
    pageMap,
    serverIslandMap,
    renderers,
    actions: () => import('./noop-entrypoint.mjs'),
    middleware: () => import('./_astro-internal_middleware.mjs')
});
const _args = undefined;
const _exports = createExports(_manifest);
const __astrojsSsrVirtualEntry = _exports.default;
const _start = 'start';
if (Object.prototype.hasOwnProperty.call(serverEntrypointModule, _start)) {
	serverEntrypointModule[_start](_manifest, _args);
}

export { __astrojsSsrVirtualEntry as default, pageMap };
