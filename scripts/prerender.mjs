#!/usr/bin/env node
// Prerender a URL using headless Chrome, output clean static HTML.
// Usage: node scripts/prerender.mjs <url> [output.html] [--viewport 1440x900] [--wait 3000]

import puppeteer from "puppeteer";
import { createRequire } from "module";
import { writeFile, mkdir } from "fs/promises";
import { dirname } from "path";

const require = createRequire(import.meta.url);

async function main() {
  const args = process.argv.slice(2);
  let url = args.find((a) => !a.startsWith("--"));
  if (!url) {
    console.error("Usage: prerender.mjs <url> [output.html] [--viewport WxH] [--wait MS]");
    process.exit(1);
  }

  const outputFile = args.find((a, i) => !a.startsWith("--") && i > 0) || "prerendered.html";
  const viewportArg = args.find((a) => a.startsWith("--viewport="));
  const waitArg = args.find((a) => a.startsWith("--wait="));

  const viewport = viewportArg
    ? viewportArg.split("=")[1].split("x").map(Number)
    : [1440, 900];
  const extraWait = waitArg ? parseInt(waitArg.split("=")[1]) : 3000;

  console.log(`Prerendering: ${url}`);
  console.log(`Viewport: ${viewport[0]}x${viewport[1]}`);
  console.log(`Extra wait: ${extraWait}ms`);

  const browser = await puppeteer.launch({
    headless: "new",
    args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-gpu"],
  });

  const page = await browser.newPage();
  await page.setViewport({ width: viewport[0], height: viewport[1] });

  // Intercept resource requests to skip images/fonts that slow things down
  // (we still need CSS)
  await page.setRequestInterception(true);
  page.on("request", (req) => {
    const type = req.resourceType();
    if (type === "image" || type === "font" || type === "media") {
      req.abort();
    } else {
      req.continue();
    }
  });

  await page.goto(url, { waitUntil: "networkidle0", timeout: 60000 });

  // Wait extra time for any lazy rendering / animations to settle
  await new Promise((r) => setTimeout(r, extraWait));

  // Extract all stylesheet URLs and their content
  const stylesheets = await page.evaluate(async () => {
    const links = Array.from(document.querySelectorAll("link[rel=stylesheet]"));
    const results = [];
    for (const link of links) {
      try {
        const resp = await fetch(link.href);
        const text = await resp.text();
        results.push({ href: link.href, css: text });
      } catch {
        results.push({ href: link.href, css: "/* failed to fetch */" });
      }
    }
    return results;
  });

  // Get the fully-rendered HTML and clean it
  const bodyHtml = await page.evaluate(() => {
    // Remove all script tags
    document.querySelectorAll("script").forEach((s) => s.remove());
    // Remove Next.js hydration markers
    document.querySelectorAll("[hidden]").forEach((el) => {
      if (el.innerHTML.includes("<!--$-->")) el.remove();
    });
    // Remove route announcer
    document.querySelectorAll("next-route-announcer").forEach((el) => el.remove());
    return document.body.innerHTML;
  });

  // Extract computed styles for all visible elements and inline them
  // This bypasses our CSS cascade by capturing what Chrome actually rendered
  const styledHtml = await page.evaluate(() => {
    // Remove all script tags
    document.querySelectorAll("script").forEach((s) => s.remove());
    document.querySelectorAll("[hidden]").forEach((el) => {
      if (el.innerHTML.includes("<!--$-->")) el.remove();
    });
    document.querySelectorAll("next-route-announcer").forEach((el) => el.remove());

    // Walk all elements and inline their computed styles
    const all = document.querySelectorAll("*");
    let count = 0;
    for (const el of all) {
      // Skip hidden elements
      const style = window.getComputedStyle(el);
      if (style.display === "none") continue;

      // Extract key layout/paint properties
      const props = [
        "display", "position", "top", "right", "bottom", "left",
        "width", "height", "margin", "padding",
        "background-color", "color", "font-size", "font-weight",
        "font-family", "line-height", "text-align",
        "border", "border-radius", "box-shadow",
        "overflow", "visibility", "opacity",
        "flex-direction", "flex-wrap", "gap",
        "justify-content", "align-items",
      ];
      let styleStr = "";
      for (const prop of props) {
        const val = style.getPropertyValue(prop);
        if (val && val !== "normal" && val !== "auto" && val !== "visible" &&
            val !== "0" && val !== "none" && val !== "0px" && val !== "initial" &&
            val !== "inherit" && val !== "row" && val !== "normal") {
          styleStr += `${prop}:${val};`;
        }
      }
      if (styleStr) {
        el.setAttribute("style", styleStr);
        count++;
      }
      // Remove Tailwind classes (we have inline styles now)
      el.removeAttribute("class");
    }
    console.log(`Inlined styles for ${count} elements`);
    return document.body.innerHTML;
  });
  const headExtras = await page.evaluate(() => {
    // Keep title, meta, and any inline styles that JS may have added
    const title = document.title;
    const metas = Array.from(document.head.querySelectorAll("meta"))
      .map((m) => m.outerHTML)
      .join("\n");
    const inlineStyles = Array.from(document.head.querySelectorAll("style"))
      .map((s) => s.outerHTML)
      .join("\n");
    return { title, metas, inlineStyles };
  });

  await browser.close();

  // Build clean static HTML — computed styles are already inlined, no CSS needed
  const html = `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${headExtras.title}</title>
${headExtras.metas}
</head>
<body>
${styledHtml}
</body>
</html>
`;

  await mkdir(dirname(outputFile), { recursive: true });
  await writeFile(outputFile, html, "utf-8");

  console.log(`\nWritten: ${outputFile}`);
  console.log(`Body HTML: ${(styledHtml.length / 1024).toFixed(1)} KB`);
  console.log(`Total file size: ${(html.length / 1024).toFixed(1)} KB`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
