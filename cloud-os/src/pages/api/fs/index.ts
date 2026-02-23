import type { APIRoute } from 'astro';
import fs from 'node:fs/promises';
import path from 'node:path';

const BASE_DIR = process.env.HOME || '/home/ken';

export const prerender = false;

// GET handler for listing and reading files
export const GET: APIRoute = async ({ url }) => {
  try {
    const action = url.searchParams.get('action') || 'list';
    const filePath = url.searchParams.get('path') || '/';
    const query = url.searchParams.get('query') || '';
    const searchType = url.searchParams.get('type') || 'name';
    const limit = parseInt(url.searchParams.get('limit') || '20', 10);

    const fullPath = path.join(BASE_DIR, filePath || '/');

    if (action === 'search') {
      // Search for files
      const results = await searchFiles(BASE_DIR, query, searchType, limit);
      return new Response(JSON.stringify({ results }), {
        headers: { 'Content-Type': 'application/json' }
      });
    }

    if (action === 'read') {
      // Read file content
      const stat = await fs.stat(fullPath);
      if (stat.isDirectory()) {
        return new Response(JSON.stringify({ error: 'Cannot read a directory' }), {
          status: 400,
          headers: { 'Content-Type': 'application/json' }
        });
      }
      const fileContent = await fs.readFile(fullPath, 'utf-8');
      return new Response(JSON.stringify({ content: fileContent, size: stat.size }), {
        headers: { 'Content-Type': 'application/json' }
      });
    }

    // Default: list directory
    const entries = await fs.readdir(fullPath, { withFileTypes: true });
    const files = await Promise.all(
      entries.map(async (entry) => {
        const entryPath = path.join(fullPath, entry.name);
        let size = 0;
        let modified = new Date();

        try {
          const stat = await fs.stat(entryPath);
          if (entry.isFile()) {
            size = stat.size;
            modified = stat.mtime;
          } else if (entry.isDirectory()) {
            modified = stat.mtime;
          }
        } catch (e) {
          // Skip files we can't stat
        }

        return {
          id: entryPath,
          name: entry.name,
          type: entry.isDirectory() ? 'folder' : 'file',
          size,
          modified: modified.toISOString(),
        };
      })
    );

    return new Response(JSON.stringify({ files }), {
      headers: { 'Content-Type': 'application/json' }
    });
  } catch (error: any) {
    console.error('FS API Error:', error);
    return new Response(JSON.stringify({ error: error.message || 'Unknown error' }), {
      status: 500,
      headers: { 'Content-Type': 'application/json' }
    });
  }
};

// Recursive file search
async function searchFiles(
  dir: string,
  query: string,
  type: 'name' | 'content',
  limit: number,
  results: any[] = []
): Promise<any[]> {
  if (results.length >= limit) return results;

  try {
    const entries = await fs.readdir(dir, { withFileTypes: true });

    for (const entry of entries) {
      if (entry.name.startsWith('.')) continue; // Skip hidden files

      const entryPath = path.join(dir, entry.name);

      if (entry.isDirectory()) {
        await searchFiles(entryPath, query, type, limit, results);
        if (results.length >= limit) break;
      } else if (entry.isFile()) {
        let match = false;

        if (type === 'name') {
          match = entry.name.toLowerCase().includes(query.toLowerCase());
        } else if (type === 'content') {
          try {
            const content = await fs.readFile(entryPath, 'utf-8');
            match = content.toLowerCase().includes(query.toLowerCase());
          } catch {
            // Skip binary files
          }
        }

        if (match) {
          const stat = await fs.stat(entryPath);
          results.push({
            name: entry.name,
            path: entryPath,
            type: 'file',
            size: stat.size,
            modified: stat.mtime.toISOString(),
          });
        }
      }
    }
  } catch (e) {
    // Skip directories we can't read
  }

  return results;
}
