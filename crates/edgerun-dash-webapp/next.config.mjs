/** @type {import('next').NextConfig} */
const nextConfig = {
  typescript: {
    ignoreBuildErrors: true,
  },
  images: {
    unoptimized: true,
  },
  // Removed "output: export" to support API routes for Gmail OAuth
  devIndicators: false,
}

export default nextConfig