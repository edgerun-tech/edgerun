globalThis.process ??= {}; globalThis.process.env ??= {};
import fs from 'node:fs/promises';
import path from 'node:path';
export { r as renderers } from '../../chunks/_@astro-renderers_B30lzduo.mjs';

const BASE_DIR = process.env.HOME || "/home/ken";
const prerender = false;
const GET = async ({ url }) => {
  try {
    const action = url.searchParams.get("action") || "list";
    const filePath = url.searchParams.get("path") || "/";
    const query = url.searchParams.get("query") || "";
    const searchType = url.searchParams.get("type") || "name";
    const limit = parseInt(url.searchParams.get("limit") || "20", 10);
    const fullPath = path.join(BASE_DIR, filePath || "/");
    if (action === "search") {
      const results = await searchFiles(BASE_DIR, query, searchType, limit);
      return new Response(JSON.stringify({ results }), {
        headers: { "Content-Type": "application/json" }
      });
    }
    if (action === "read") {
      const stat = await fs.stat(fullPath);
      if (stat.isDirectory()) {
        return new Response(JSON.stringify({ error: "Cannot read a directory" }), {
          status: 400,
          headers: { "Content-Type": "application/json" }
        });
      }
      const fileContent = await fs.readFile(fullPath, "utf-8");
      return new Response(JSON.stringify({ content: fileContent, size: stat.size }), {
        headers: { "Content-Type": "application/json" }
      });
    }
    const entries = await fs.readdir(fullPath, { withFileTypes: true });
    const files = await Promise.all(
      entries.map(async (entry) => {
        const entryPath = path.join(fullPath, entry.name);
        let size = 0;
        let modified = /* @__PURE__ */ new Date();
        try {
          const stat = await fs.stat(entryPath);
          if (entry.isFile()) {
            size = stat.size;
            modified = stat.mtime;
          } else if (entry.isDirectory()) {
            modified = stat.mtime;
          }
        } catch (e) {
        }
        return {
          id: entryPath,
          name: entry.name,
          type: entry.isDirectory() ? "folder" : "file",
          size,
          modified: modified.toISOString()
        };
      })
    );
    return new Response(JSON.stringify({ files }), {
      headers: { "Content-Type": "application/json" }
    });
  } catch (error) {
    console.error("FS API Error:", error);
    return new Response(JSON.stringify({ error: error.message || "Unknown error" }), {
      status: 500,
      headers: { "Content-Type": "application/json" }
    });
  }
};
async function searchFiles(dir, query, type, limit, results = []) {
  if (results.length >= limit) return results;
  try {
    const entries = await fs.readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.name.startsWith(".")) continue;
      const entryPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        await searchFiles(entryPath, query, type, limit, results);
        if (results.length >= limit) break;
      } else if (entry.isFile()) {
        let match = false;
        if (type === "name") {
          match = entry.name.toLowerCase().includes(query.toLowerCase());
        } else if (type === "content") {
          try {
            const content = await fs.readFile(entryPath, "utf-8");
            match = content.toLowerCase().includes(query.toLowerCase());
          } catch {
          }
        }
        if (match) {
          const stat = await fs.stat(entryPath);
          results.push({
            name: entry.name,
            path: entryPath,
            type: "file",
            size: stat.size,
            modified: stat.mtime.toISOString()
          });
        }
      }
    }
  } catch (e) {
  }
  return results;
}

const _page = /*#__PURE__*/Object.freeze(/*#__PURE__*/Object.defineProperty({
  __proto__: null,
  GET,
  prerender
}, Symbol.toStringTag, { value: 'Module' }));

const page = () => _page;

export { page };
