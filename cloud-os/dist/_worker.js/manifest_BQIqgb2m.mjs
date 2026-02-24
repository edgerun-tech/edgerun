globalThis.process ??= {}; globalThis.process.env ??= {};
import { p as decodeKey } from './chunks/astro/server_B5lkohcy.mjs';
import './chunks/astro-designed-error-pages_pJXwdU1O.mjs';
import { N as NOOP_MIDDLEWARE_FN } from './chunks/noop-middleware_CxVEMDDX.mjs';

function sanitizeParams(params) {
  return Object.fromEntries(
    Object.entries(params).map(([key, value]) => {
      if (typeof value === "string") {
        return [key, value.normalize().replace(/#/g, "%23").replace(/\?/g, "%3F")];
      }
      return [key, value];
    })
  );
}
function getParameter(part, params) {
  if (part.spread) {
    return params[part.content.slice(3)] || "";
  }
  if (part.dynamic) {
    if (!params[part.content]) {
      throw new TypeError(`Missing parameter: ${part.content}`);
    }
    return params[part.content];
  }
  return part.content.normalize().replace(/\?/g, "%3F").replace(/#/g, "%23").replace(/%5B/g, "[").replace(/%5D/g, "]");
}
function getSegment(segment, params) {
  const segmentPath = segment.map((part) => getParameter(part, params)).join("");
  return segmentPath ? "/" + segmentPath : "";
}
function getRouteGenerator(segments, addTrailingSlash) {
  return (params) => {
    const sanitizedParams = sanitizeParams(params);
    let trailing = "";
    if (addTrailingSlash === "always" && segments.length) {
      trailing = "/";
    }
    const path = segments.map((segment) => getSegment(segment, sanitizedParams)).join("") + trailing;
    return path || "/";
  };
}

function deserializeRouteData(rawRouteData) {
  return {
    route: rawRouteData.route,
    type: rawRouteData.type,
    pattern: new RegExp(rawRouteData.pattern),
    params: rawRouteData.params,
    component: rawRouteData.component,
    generate: getRouteGenerator(rawRouteData.segments, rawRouteData._meta.trailingSlash),
    pathname: rawRouteData.pathname || void 0,
    segments: rawRouteData.segments,
    prerender: rawRouteData.prerender,
    redirect: rawRouteData.redirect,
    redirectRoute: rawRouteData.redirectRoute ? deserializeRouteData(rawRouteData.redirectRoute) : void 0,
    fallbackRoutes: rawRouteData.fallbackRoutes.map((fallback) => {
      return deserializeRouteData(fallback);
    }),
    isIndex: rawRouteData.isIndex,
    origin: rawRouteData.origin
  };
}

function deserializeManifest(serializedManifest) {
  const routes = [];
  for (const serializedRoute of serializedManifest.routes) {
    routes.push({
      ...serializedRoute,
      routeData: deserializeRouteData(serializedRoute.routeData)
    });
    const route = serializedRoute;
    route.routeData = deserializeRouteData(serializedRoute.routeData);
  }
  const assets = new Set(serializedManifest.assets);
  const componentMetadata = new Map(serializedManifest.componentMetadata);
  const inlinedScripts = new Map(serializedManifest.inlinedScripts);
  const clientDirectives = new Map(serializedManifest.clientDirectives);
  const serverIslandNameMap = new Map(serializedManifest.serverIslandNameMap);
  const key = decodeKey(serializedManifest.key);
  return {
    // in case user middleware exists, this no-op middleware will be reassigned (see plugin-ssr.ts)
    middleware() {
      return { onRequest: NOOP_MIDDLEWARE_FN };
    },
    ...serializedManifest,
    assets,
    componentMetadata,
    inlinedScripts,
    clientDirectives,
    routes,
    serverIslandNameMap,
    key
  };
}

const manifest = deserializeManifest({"hrefRoot":"file:///home/ken/edgerun/cloud-os/","cacheDir":"file:///home/ken/edgerun/cloud-os/node_modules/.astro/","outDir":"file:///home/ken/edgerun/cloud-os/dist/","srcDir":"file:///home/ken/edgerun/cloud-os/src/","publicDir":"file:///home/ken/edgerun/cloud-os/public/","buildClientDir":"file:///home/ken/edgerun/cloud-os/dist/","buildServerDir":"file:///home/ken/edgerun/cloud-os/dist/_worker.js/","adapterName":"@astrojs/cloudflare","routes":[{"file":"","links":[],"scripts":[],"styles":[],"routeData":{"type":"page","component":"_server-islands.astro","params":["name"],"segments":[[{"content":"_server-islands","dynamic":false,"spread":false}],[{"content":"name","dynamic":true,"spread":false}]],"pattern":"^\\/_server-islands\\/([^/]+?)\\/?$","prerender":false,"isIndex":false,"fallbackRoutes":[],"route":"/_server-islands/[name]","origin":"internal","_meta":{"trailingSlash":"ignore"}}},{"file":"index.html","links":[],"scripts":[],"styles":[],"routeData":{"route":"/","isIndex":true,"type":"page","pattern":"^\\/$","segments":[],"params":[],"component":"src/pages/index.astro","pathname":"/","prerender":true,"fallbackRoutes":[],"distURL":[],"origin":"project","_meta":{"trailingSlash":"ignore"}}},{"file":"","links":[],"scripts":[],"styles":[],"routeData":{"type":"endpoint","isIndex":false,"route":"/_image","pattern":"^\\/_image\\/?$","segments":[[{"content":"_image","dynamic":false,"spread":false}]],"params":[],"component":"node_modules/@astrojs/cloudflare/dist/entrypoints/image-endpoint.js","pathname":"/_image","prerender":false,"fallbackRoutes":[],"origin":"internal","_meta":{"trailingSlash":"ignore"}}},{"file":"","links":[],"scripts":[],"styles":[],"routeData":{"route":"/api/codex/execute","isIndex":false,"type":"endpoint","pattern":"^\\/api\\/codex\\/execute\\/?$","segments":[[{"content":"api","dynamic":false,"spread":false}],[{"content":"codex","dynamic":false,"spread":false}],[{"content":"execute","dynamic":false,"spread":false}]],"params":[],"component":"src/pages/api/codex/execute.ts","pathname":"/api/codex/execute","prerender":false,"fallbackRoutes":[],"distURL":[],"origin":"project","_meta":{"trailingSlash":"ignore"}}},{"file":"","links":[],"scripts":[],"styles":[],"routeData":{"route":"/api/fs","isIndex":true,"type":"endpoint","pattern":"^\\/api\\/fs\\/?$","segments":[[{"content":"api","dynamic":false,"spread":false}],[{"content":"fs","dynamic":false,"spread":false}]],"params":[],"component":"src/pages/api/fs/index.ts","pathname":"/api/fs","prerender":false,"fallbackRoutes":[],"distURL":[],"origin":"project","_meta":{"trailingSlash":"ignore"}}},{"file":"","links":[],"scripts":[],"styles":[],"routeData":{"route":"/api/qwen/callback","isIndex":false,"type":"endpoint","pattern":"^\\/api\\/qwen\\/callback\\/?$","segments":[[{"content":"api","dynamic":false,"spread":false}],[{"content":"qwen","dynamic":false,"spread":false}],[{"content":"callback","dynamic":false,"spread":false}]],"params":[],"component":"src/pages/api/qwen/callback.ts","pathname":"/api/qwen/callback","prerender":false,"fallbackRoutes":[],"distURL":[],"origin":"project","_meta":{"trailingSlash":"ignore"}}},{"file":"","links":[],"scripts":[],"styles":[],"routeData":{"route":"/api/qwen/chat","isIndex":false,"type":"endpoint","pattern":"^\\/api\\/qwen\\/chat\\/?$","segments":[[{"content":"api","dynamic":false,"spread":false}],[{"content":"qwen","dynamic":false,"spread":false}],[{"content":"chat","dynamic":false,"spread":false}]],"params":[],"component":"src/pages/api/qwen/chat.ts","pathname":"/api/qwen/chat","prerender":false,"fallbackRoutes":[],"distURL":[],"origin":"project","_meta":{"trailingSlash":"ignore"}}},{"file":"","links":[],"scripts":[],"styles":[],"routeData":{"route":"/api/qwen/poll","isIndex":false,"type":"endpoint","pattern":"^\\/api\\/qwen\\/poll\\/?$","segments":[[{"content":"api","dynamic":false,"spread":false}],[{"content":"qwen","dynamic":false,"spread":false}],[{"content":"poll","dynamic":false,"spread":false}]],"params":[],"component":"src/pages/api/qwen/poll.ts","pathname":"/api/qwen/poll","prerender":false,"fallbackRoutes":[],"distURL":[],"origin":"project","_meta":{"trailingSlash":"ignore"}}},{"file":"","links":[],"scripts":[],"styles":[],"routeData":{"route":"/api/qwen/token","isIndex":false,"type":"endpoint","pattern":"^\\/api\\/qwen\\/token\\/?$","segments":[[{"content":"api","dynamic":false,"spread":false}],[{"content":"qwen","dynamic":false,"spread":false}],[{"content":"token","dynamic":false,"spread":false}]],"params":[],"component":"src/pages/api/qwen/token.ts","pathname":"/api/qwen/token","prerender":false,"fallbackRoutes":[],"distURL":[],"origin":"project","_meta":{"trailingSlash":"ignore"}}},{"file":"","links":[],"scripts":[],"styles":[],"routeData":{"route":"/api/qwen","isIndex":true,"type":"endpoint","pattern":"^\\/api\\/qwen\\/?$","segments":[[{"content":"api","dynamic":false,"spread":false}],[{"content":"qwen","dynamic":false,"spread":false}]],"params":[],"component":"src/pages/api/qwen/index.ts","pathname":"/api/qwen","prerender":false,"fallbackRoutes":[],"distURL":[],"origin":"project","_meta":{"trailingSlash":"ignore"}}},{"file":"","links":[],"scripts":[],"styles":[{"type":"external","src":"/_astro/_id_.LDbrBgaC.css"}],"routeData":{"route":"/call/[id]","isIndex":false,"type":"page","pattern":"^\\/call\\/([^/]+?)\\/?$","segments":[[{"content":"call","dynamic":false,"spread":false}],[{"content":"id","dynamic":true,"spread":false}]],"params":["id"],"component":"src/pages/call/[id].astro","prerender":false,"fallbackRoutes":[],"distURL":[],"origin":"project","_meta":{"trailingSlash":"ignore"}}}],"base":"/","trailingSlash":"ignore","compressHTML":true,"componentMetadata":[["/home/ken/edgerun/cloud-os/src/pages/call/[id].astro",{"propagation":"none","containsHead":true}],["/home/ken/edgerun/cloud-os/src/pages/index.astro",{"propagation":"none","containsHead":true}]],"renderers":[],"clientDirectives":[["idle","(()=>{var l=(n,t)=>{let i=async()=>{await(await n())()},e=typeof t.value==\"object\"?t.value:void 0,s={timeout:e==null?void 0:e.timeout};\"requestIdleCallback\"in window?window.requestIdleCallback(i,s):setTimeout(i,s.timeout||200)};(self.Astro||(self.Astro={})).idle=l;window.dispatchEvent(new Event(\"astro:idle\"));})();"],["load","(()=>{var e=async t=>{await(await t())()};(self.Astro||(self.Astro={})).load=e;window.dispatchEvent(new Event(\"astro:load\"));})();"],["media","(()=>{var n=(a,t)=>{let i=async()=>{await(await a())()};if(t.value){let e=matchMedia(t.value);e.matches?i():e.addEventListener(\"change\",i,{once:!0})}};(self.Astro||(self.Astro={})).media=n;window.dispatchEvent(new Event(\"astro:media\"));})();"],["only","(()=>{var e=async t=>{await(await t())()};(self.Astro||(self.Astro={})).only=e;window.dispatchEvent(new Event(\"astro:only\"));})();"],["visible","(()=>{var a=(s,i,o)=>{let r=async()=>{await(await s())()},t=typeof i.value==\"object\"?i.value:void 0,c={rootMargin:t==null?void 0:t.rootMargin},n=new IntersectionObserver(e=>{for(let l of e)if(l.isIntersecting){n.disconnect(),r();break}},c);for(let e of o.children)n.observe(e)};(self.Astro||(self.Astro={})).visible=a;window.dispatchEvent(new Event(\"astro:visible\"));})();"]],"entryModules":{"\u0000astro-internal:middleware":"_astro-internal_middleware.mjs","\u0000virtual:astro:actions/noop-entrypoint":"noop-entrypoint.mjs","\u0000@astro-page:src/pages/api/codex/execute@_@ts":"pages/api/codex/execute.astro.mjs","\u0000@astro-page:src/pages/api/fs/index@_@ts":"pages/api/fs.astro.mjs","\u0000@astro-page:src/pages/api/qwen/callback@_@ts":"pages/api/qwen/callback.astro.mjs","\u0000@astro-page:src/pages/api/qwen/chat@_@ts":"pages/api/qwen/chat.astro.mjs","\u0000@astro-page:src/pages/api/qwen/poll@_@ts":"pages/api/qwen/poll.astro.mjs","\u0000@astro-page:src/pages/api/qwen/token@_@ts":"pages/api/qwen/token.astro.mjs","\u0000@astro-page:src/pages/api/qwen/index@_@ts":"pages/api/qwen.astro.mjs","\u0000@astro-page:src/pages/call/[id]@_@astro":"pages/call/_id_.astro.mjs","\u0000@astrojs-ssr-virtual-entry":"index.js","\u0000@astro-page:node_modules/@astrojs/cloudflare/dist/entrypoints/image-endpoint@_@js":"pages/_image.astro.mjs","\u0000@astro-page:src/pages/index@_@astro":"pages/index.astro.mjs","\u0000@astro-renderers":"renderers.mjs","\u0000@astrojs-ssr-adapter":"_@astrojs-ssr-adapter.mjs","\u0000@astrojs-manifest":"manifest_BQIqgb2m.mjs","/home/ken/edgerun/cloud-os/node_modules/unstorage/drivers/cloudflare-kv-binding.mjs":"chunks/cloudflare-kv-binding_DMly_2Gl.mjs","/home/ken/edgerun/cloud-os/node_modules/astro/dist/assets/services/sharp.js":"chunks/sharp_Ipb4zuV0.mjs","/home/ken/edgerun/cloud-os/src/components/Editor.tsx":"_astro/Editor.C9ZNC8Ms.js","/home/ken/edgerun/cloud-os/src/components/Terminal.tsx":"_astro/Terminal.CZfb-w7B.js","/home/ken/edgerun/cloud-os/src/components/FileManager.tsx":"_astro/FileManager.NM_RJ82g.js","/home/ken/edgerun/cloud-os/src/components/GitHubBrowser.tsx":"_astro/GitHubBrowser.DhMvtoiE.js","/home/ken/edgerun/cloud-os/src/components/GmailPanel.tsx":"_astro/GmailPanel.Dw4p_7Sk.js","/home/ken/edgerun/cloud-os/src/components/CallApp.tsx":"chunks/CallApp_FSORTp_T.mjs","/home/ken/edgerun/cloud-os/src/components/CalendarPanel.tsx":"_astro/CalendarPanel.RG6YWqIT.js","/home/ken/edgerun/cloud-os/src/components/CloudflarePanel.tsx":"_astro/CloudflarePanel.CQCkLlr5.js","/home/ken/edgerun/cloud-os/src/components/IntegrationsPanel.tsx":"_astro/IntegrationsPanel.BD7vuJCr.js","/home/ken/edgerun/cloud-os/src/components/SettingsPanel.tsx":"_astro/SettingsPanel.B-BUdy22.js","/home/ken/edgerun/cloud-os/src/components/CloudPanel.tsx":"_astro/CloudPanel.BtCbuXcK.js","@/components/OfflineIndicator":"_astro/OfflineIndicator.Bhw9tjku.js","@/components/KeybindingsHelp":"_astro/KeybindingsHelp.DyEkLuUt.js","@/components/Dock":"_astro/Dock.BtwSO55p.js","@astrojs/solid-js/client.js":"_astro/client.BFm0c59U.js","/home/ken/edgerun/cloud-os/src/stores/github.ts":"_astro/github.jFT5BxHH.js","/home/ken/edgerun/cloud-os/src/components/ActivityFeed.tsx":"_astro/ActivityFeed.CVA3T1Fa.js","/home/ken/edgerun/cloud-os/src/components/Placeholder.tsx":"_astro/Placeholder.Dl_IgE4v.js","/home/ken/edgerun/cloud-os/src/lib/db.ts":"_astro/db.m3QulWUD.js","@/components/Onboarding":"_astro/Onboarding.Bt9pjePl.js","@/components/WindowManager":"_astro/WindowManager.B9qMqoBK.js","@/components/IntentBar":"_astro/IntentBar.BL3askDg.js","/home/ken/edgerun/cloud-os/src/components/CallApp":"_astro/CallApp.H7Caw_U7.js","astro:scripts/before-hydration.js":""},"inlinedScripts":[],"assets":["/_astro/_id_.LDbrBgaC.css","/_astro/index.DOrYoP_4.css","/apple-touch-icon.png","/build.json","/favicon-16x16.png","/favicon-32x32.png","/favicon.ico","/favicon.svg","/icon-192.png","/icon-512.png","/manifest.json","/og-image.png","/screenshot-wide.png","/_astro/ActivityFeed.CVA3T1Fa.js","/_astro/CalendarPanel.RG6YWqIT.js","/_astro/CallApp.DZGq7XiU.js","/_astro/CallApp.H7Caw_U7.js","/_astro/CloudPanel.BtCbuXcK.js","/_astro/CloudflarePanel.CQCkLlr5.js","/_astro/Dock.BtwSO55p.js","/_astro/Editor.C9ZNC8Ms.js","/_astro/FileManager.NM_RJ82g.js","/_astro/GitHubBrowser.DhMvtoiE.js","/_astro/GmailPanel.Dw4p_7Sk.js","/_astro/IntegrationsPanel.BD7vuJCr.js","/_astro/IntentBar.BL3askDg.js","/_astro/KeybindingsHelp.DyEkLuUt.js","/_astro/OfflineIndicator.Bhw9tjku.js","/_astro/Onboarding.Bt9pjePl.js","/_astro/Placeholder.Dl_IgE4v.js","/_astro/SettingsPanel.B-BUdy22.js","/_astro/Terminal.CZfb-w7B.js","/_astro/WindowManager.B9qMqoBK.js","/_astro/client.BFm0c59U.js","/_astro/db.m3QulWUD.js","/_astro/github.jFT5BxHH.js","/_astro/index.BwLFhBDG.js","/_astro/index.D8J74ltv.js","/_astro/index.h0zGHtDZ.js","/_astro/index.qRH22Uks.js","/_astro/integrations.CE4HFLe9.js","/_astro/preload-helper.BlTxHScW.js","/_astro/router.B2cEilhV.js","/_astro/store.CYmLMFrj.js","/_astro/web.CTOVB8SF.js","/_astro/windows.DqodbEGz.js","/_worker.js/_@astrojs-ssr-adapter.mjs","/_worker.js/_astro-internal_middleware.mjs","/_worker.js/index.js","/_worker.js/noop-entrypoint.mjs","/_worker.js/renderers.mjs","/_worker.js/_astro/_id_.LDbrBgaC.css","/_worker.js/_astro/index.DOrYoP_4.css","/_worker.js/chunks/CalendarPanel_CJNtsrEC.mjs","/_worker.js/chunks/CallApp_FSORTp_T.mjs","/_worker.js/chunks/CloudPanel_y8MZ6aeR.mjs","/_worker.js/chunks/CloudflarePanel_DRD_9tSX.mjs","/_worker.js/chunks/Editor_DbdTVEhX.mjs","/_worker.js/chunks/FileManager_aqvFe6ou.mjs","/_worker.js/chunks/GitHubBrowser_CHScdUYo.mjs","/_worker.js/chunks/GmailPanel_BHpN5uJw.mjs","/_worker.js/chunks/IntegrationsPanel_BqpkYHs8.mjs","/_worker.js/chunks/SettingsPanel_CMjvshKe.mjs","/_worker.js/chunks/Terminal_Crj8uqOi.mjs","/_worker.js/chunks/_@astro-renderers_B30lzduo.mjs","/_worker.js/chunks/_@astrojs-ssr-adapter_BXu_WkfK.mjs","/_worker.js/chunks/astro-designed-error-pages_pJXwdU1O.mjs","/_worker.js/chunks/astro_C0Lo0rDq.mjs","/_worker.js/chunks/cloudflare-kv-binding_DMly_2Gl.mjs","/_worker.js/chunks/image-endpoint_femtW2c8.mjs","/_worker.js/chunks/index_BjAcyYff.mjs","/_worker.js/chunks/index_DxG_ot85.mjs","/_worker.js/chunks/index_Gncz_ErS.mjs","/_worker.js/chunks/noop-middleware_CxVEMDDX.mjs","/_worker.js/chunks/path_CH3auf61.mjs","/_worker.js/chunks/remote_CrdlObHx.mjs","/_worker.js/chunks/sharp_Ipb4zuV0.mjs","/workers/mcp/base.js","/workers/mcp/base.js.map","/workers/mcp/browser-os.js","/workers/mcp/browser-os.js.map","/workers/mcp/cloudflare.js","/workers/mcp/cloudflare.js.map","/workers/mcp/github.js","/workers/mcp/github.js.map","/workers/mcp/google.js","/workers/mcp/google.js.map","/workers/mcp/qwen.js","/workers/mcp/qwen.js.map","/workers/mcp/terminal.js","/workers/mcp/terminal.js.map","/workers/mcp/vercel.js","/workers/mcp/vercel.js.map","/_worker.js/pages/_image.astro.mjs","/_worker.js/pages/index.astro.mjs","/_worker.js/chunks/astro/server_B5lkohcy.mjs","/_worker.js/pages/api/fs.astro.mjs","/_worker.js/pages/api/qwen.astro.mjs","/_worker.js/pages/call/_id_.astro.mjs","/_worker.js/pages/api/codex/execute.astro.mjs","/_worker.js/pages/api/qwen/callback.astro.mjs","/_worker.js/pages/api/qwen/chat.astro.mjs","/_worker.js/pages/api/qwen/poll.astro.mjs","/_worker.js/pages/api/qwen/token.astro.mjs","/index.html"],"buildFormat":"directory","checkOrigin":true,"allowedDomains":[],"serverIslandNameMap":[],"key":"MG5H+upx8F0owk79Ayi8uvMo4vJTU1g6x0Xi4zFpIY4=","sessionConfig":{"driver":"cloudflare-kv-binding","options":{"binding":"SESSION"}}});
if (manifest.sessionConfig) manifest.sessionConfig.driverModule = () => import('./chunks/cloudflare-kv-binding_DMly_2Gl.mjs');

export { manifest };
