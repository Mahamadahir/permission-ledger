/// <reference types="vitest/config" />
import { resolve } from 'node:path';
import { defineConfig } from 'vite';

export default defineConfig({
  test: {
    environment: 'node',
    environmentMatchGlobs: [['src/popup/**/*.test.ts', 'jsdom']],
    include: ['src/**/*.test.ts']
  },
  build: {
    emptyOutDir: true,
    rollupOptions: {
      input: {
        popup: resolve(__dirname, 'popup.html'),
        background: resolve(__dirname, 'src/background/service-worker.ts')
      },
      output: {
        entryFileNames: (chunk) => chunk.name === 'background' ? 'background.js' : 'assets/[name].js'
      }
    }
  }
});
