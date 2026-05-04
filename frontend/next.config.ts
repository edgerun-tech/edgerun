import type { NextConfig } from "next"

const nextConfig: NextConfig = {
  outputFileTracingRoot: __dirname,
  experimental: {
    turbopackRoot: __dirname,
  },
  allowedDevOrigins: ['192.168.1.42']
}

export default nextConfig
