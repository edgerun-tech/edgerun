globalThis.process ??= {}; globalThis.process.env ??= {};
import { e as createComponent, k as renderComponent, r as renderTemplate, m as maybeRenderHead } from '../../chunks/astro/server_B5lkohcy.mjs';
import { $ as $$MainLayout } from '../../chunks/index_BjAcyYff.mjs';
import CallApp from '../../chunks/CallApp_FSORTp_T.mjs';
export { r as renderers } from '../../chunks/_@astro-renderers_B30lzduo.mjs';

const prerender = false;
const $$id = createComponent(($$result, $$props, $$slots) => {
  return renderTemplate`${renderComponent($$result, "MainLayout", $$MainLayout, { "title": "Call - Browser OS", "description": "Video call" }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<div class="fixed inset-0 bg-black"> <div class="h-full"> ${renderComponent($$result2, "CallApp", CallApp, { "client:load": true, "client:component-hydration": "load", "client:component-path": "/home/ken/edgerun/cloud-os/src/components/CallApp", "client:component-export": "default" })} </div> </div> ` })}`;
}, "/home/ken/edgerun/cloud-os/src/pages/call/[id].astro", void 0);

const $$file = "/home/ken/edgerun/cloud-os/src/pages/call/[id].astro";
const $$url = "/call/[id]";

const _page = /*#__PURE__*/Object.freeze(/*#__PURE__*/Object.defineProperty({
  __proto__: null,
  default: $$id,
  file: $$file,
  prerender,
  url: $$url
}, Symbol.toStringTag, { value: 'Module' }));

const page = () => _page;

export { page };
