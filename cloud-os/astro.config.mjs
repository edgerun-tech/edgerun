import cloudflare from '@astrojs/cloudflare';
// @ts-check
import { defineConfig } from 'astro/config';
import { monaco } from '@bithero/monaco-editor-vite-plugin';
import solidJs from '@astrojs/solid-js';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath } from 'node:url';

// https://astro.build/config
export default defineConfig({
  integrations: [solidJs()],
  adapter: cloudflare({ imageService: 'compile', mode: 'worker' }),
  server: {
    host: true,
    port: 4321
  },

  vite: {
    plugins: [tailwindcss(), monaco({
      features: 'all',
      languages: ['json'],
      globalAPI: true
    })],
    resolve: {
      dedupe: ['solid-js', 'solid-js/web'],
      alias: {
        'solid-js': fileURLToPath(new URL('./node_modules/solid-js', import.meta.url)),
        'solid-js/web': fileURLToPath(new URL('./node_modules/solid-js/web', import.meta.url))
      }
    },
    server: {
      host: true,
      hmr: {
        protocol: 'ws',
        clientPort: 4321
      },
      allowedHosts: ['desktop.bengal-salary.ts.net'],
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
