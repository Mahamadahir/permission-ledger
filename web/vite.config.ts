/// <reference types="vitest/config" />
import { svelteTesting } from '@testing-library/svelte/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit(), svelteTesting()],
  server: {
    // Mirror the production topology: the API is reachable at /api on the same
    // origin, so cookies are first-party in development too.
    proxy: {
      '/api': {
        target: process.env.API_PROXY_TARGET ?? 'http://localhost:3000',
        changeOrigin: false
      }
    }
  },
  test: {
    include: ['src/**/*.test.ts'],
    setupFiles: ['./src/test-setup.ts'],
    environmentMatchGlobs: [['src/**/*.svelte.test.ts', 'jsdom']],
    environment: 'node'
  }
});
