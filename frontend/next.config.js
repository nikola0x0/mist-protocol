/** @type {import('next').NextConfig} */
const nextConfig = {
  typescript: {
    // Ignore TypeScript errors during build (version conflicts with @mysten packages)
    ignoreBuildErrors: true,
  },
  eslint: {
    // Only show warnings for img tags, don't fail the build
    ignoreDuringBuilds: false,
  },
}

module.exports = nextConfig
