/** @type {import('next').NextConfig} */
const nextConfig = {
  allowedDevOrigins: [
    "localhost",
    "127.0.0.1",
    "192.168.1.42",
  ],
  images: {
    unoptimized: true,
  },
  // Keep API routes enabled for Gmail OAuth and exchange proxy routes.
  // Do not hide TypeScript build errors; surface them so UI drift is caught early.
  devIndicators: false,
}

export default nextConfig
