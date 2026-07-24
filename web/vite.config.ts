/// <reference types="vitest/config" />
import { svelteTesting } from '@testing-library/svelte/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit(), svelteTesting()],
  test: {
    include: ['src/**/*.test.ts'],
    setupFiles: ['./src/test-setup.ts'],
    environmentMatchGlobs: [['src/**/*.svelte.test.ts', 'jsdom']],
    environment: 'node'
  }
});
