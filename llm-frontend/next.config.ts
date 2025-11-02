import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  /* config options here */

  // Enable standalone output for Docker deployment
  // This creates a minimal production build with only necessary files
  output: 'standalone',
};

export default nextConfig;
