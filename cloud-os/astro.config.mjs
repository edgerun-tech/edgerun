import cloudflare from '@astrojs/cloudflare';
// @ts-check
import { defineConfig } from 'astro/config';
import { monaco } from '@bithero/monaco-editor-vite-plugin';
import solidJs from '@astrojs/solid-js';
import tailwindcss from '@tailwindcss/vite';

// https://astro.build/config
export default defineConfig({
  integrations: [solidJs()],
  adapter: cloudflare({ imageService: 'compile', mode: 'worker' }),

  vite: {
    plugins: [tailwindcss(),monaco({
            features: "all",
            languages: ["json"],
            globalAPI: true,
        })],
    server: {
      allowedHosts: ['desktop.bengal-salary.ts.net'],
      headers: {
        "Cross-Origin-Opener-Policy": "same-origin",
        "Cross-Origin-Embedder-Policy": "require-corp",
        "Cross-Origin-Resource-Policy": "same-origin",
      },
    },
    preview: {
      headers: {
        "Cross-Origin-Opener-Policy": "same-origin",
        "Cross-Origin-Embedder-Policy": "require-corp",
        "Cross-Origin-Resource-Policy": "same-origin",
      },
    },
  }
});