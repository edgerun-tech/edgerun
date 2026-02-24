globalThis.process ??= {}; globalThis.process.env ??= {};
import { e as createComponent, g as addAttribute, n as renderHead, o as renderSlot, r as renderTemplate, h as createAstro } from './astro/server_B5lkohcy.mjs';
/* empty css                        */
import { n as ssrElement, m as mergeProps } from './_@astro-renderers_B30lzduo.mjs';

const $$Astro = createAstro();
const $$MainLayout = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro, $$props, $$slots);
  Astro2.self = $$MainLayout;
  const { title, description } = Astro2.props;
  const siteUrl = "https://cloud-os.kensservices.workers.dev";
  const siteName = "CloudOS";
  const defaultTitle = "CloudOS - Unified Cloud Operating System";
  const defaultDescription = "A unified cloud operating system to manage servers, deployments, domains, and infrastructure across Cloudflare, Vercel, Hetzner, and more.";
  const pageTitle = title ? `${title} | ${siteName}` : defaultTitle;
  const pageDescription = description || defaultDescription;
  return renderTemplate`<html lang="en" class="dark bg-gray-950"> <head><meta charset="utf-8"><!-- Primary Meta Tags --><title>${pageTitle}</title><meta name="title"${addAttribute(pageTitle, "content")}><meta name="description"${addAttribute(pageDescription, "content")}><meta name="keywords" content="cloud OS, cloud management, server management, devops, cloudflare, vercel, hetzner, cloud dashboard, infrastructure management, deployment, DNS management, MCP, AI assistant"><meta name="author" content="Ken"><meta name="robots" content="index, follow"><link rel="canonical"${addAttribute(siteUrl, "href")}><!-- Favicon --><link rel="icon" type="image/svg+xml" href="/favicon.svg"><link rel="icon" href="/favicon.ico" sizes="any"><link rel="apple-touch-icon" href="/apple-touch-icon.png" sizes="180x180"><link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png"><link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png"><!-- Viewport --><meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no"><!-- PWA --><link rel="manifest" href="/manifest.json"><meta name="theme-color" content="#2563eb"><meta name="apple-mobile-web-app-capable" content="yes"><meta name="apple-mobile-web-app-status-bar-style" content="black-translucent"><meta name="apple-mobile-web-app-title" content="CloudOS"><!-- Open Graph / Facebook --><meta property="og:type" content="website"><meta property="og:url"${addAttribute(siteUrl, "content")}><meta property="og:title"${addAttribute(pageTitle, "content")}><meta property="og:description"${addAttribute(pageDescription, "content")}><meta property="og:site_name"${addAttribute(siteName, "content")}><meta property="og:image"${addAttribute(`${siteUrl}/og-image.png`, "content")}><meta property="og:image:width" content="1200"><meta property="og:image:height" content="630"><meta property="og:locale" content="en_US"><!-- Twitter --><meta name="twitter:card" content="summary_large_image"><meta name="twitter:url"${addAttribute(siteUrl, "content")}><meta name="twitter:title"${addAttribute(pageTitle, "content")}><meta name="twitter:description"${addAttribute(pageDescription, "content")}><meta name="twitter:image"${addAttribute(`${siteUrl}/og-image.png`, "content")}><meta name="twitter:creator" content="@ken"><!-- Performance --><link rel="preconnect" href="https://fonts.googleapis.com"><link rel="preconnect" href="https://fonts.gstatic.com" crossorigin><link rel="dns-prefetch" href="https://api.cloudflare.com"><link rel="dns-prefetch" href="https://api.vercel.com">${renderHead()}</head> <body class="min-h-screen bg-cover bg-center bg-no-repeat bg-fixed text-white relative" style="background-image: url('/wallpaper.png')"> <div class="fixed inset-0 bg-black/40"></div> ${renderSlot($$result, $$slots["default"])} </body></html>`;
}, "/home/ken/edgerun/cloud-os/src/layouts/MainLayout.astro", void 0);

function IconTemplate(iconSrc, props) {
  return ssrElement("svg", mergeProps(() => iconSrc.a, props, {
    get color() {
      return props.color || "currentColor";
    },
    get height() {
      return props.size || "1em";
    },
    get width() {
      return props.size || "1em";
    },
    xmlns: "http://www.w3.org/2000/svg",
    get style() {
      return {
        ...typeof props.style === "object" ? props.style : {},
        overflow: "visible"
      };
    },
    get innerHTML() {
      return props.title ? `${iconSrc.c}<title>${props.title}</title>` : iconSrc.c;
    },
    src: void 0
  }), void 0);
}

export { $$MainLayout as $, IconTemplate as I };
