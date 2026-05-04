import type { NextConfig } from "next"

const nextConfig: NextConfig = {
  outputFileTracingRoot: __dirname,
  experimental: {
    turbopackRoot: __dirname,
  },
}

export default nextConfig
