import type { NextConfig } from 'next';
const withPWA = require('next-pwa').default;

const nextConfig: NextConfig = withPWA({
  /* config options here */
  typescript: {
    ignoreBuildErrors: true,
  },
  eslint: {
    ignoreDuringBuilds: true,
  },
  images: {
    remotePatterns: [
      {
        protocol: 'https',
        hostname: 'placehold.co',
        port: '',
        pathname: '/**',
      },
    ],
  },
  pwa: {
    dest: 'public',
    register: true,
    skipWaiting: true,
    sw: 'sw.js',
    disable: process.env.NODE_ENV === 'development',
  },
});

export default nextConfig;
