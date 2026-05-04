import { defineConfig } from "astro/config"
import solidJs from "@astrojs/solid-js"
import tailwind from "@astrojs/tailwind"
import cloudflare from "@astrojs/cloudflare"

export default defineConfig({
  site: "https://benchmarks.edgerun.local",
  output: "server",
  integrations: [
    solidJs(),
    tailwind(),
  ],
  adapter: cloudflare(),
})

