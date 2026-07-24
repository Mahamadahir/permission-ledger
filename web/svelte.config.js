import adapter from '@sveltejs/adapter-static';

// The dashboard is a client-only SPA (see src/routes/+layout.ts), so it builds
// to static files with an index.html fallback that handles client-side routing.
const config = {
  kit: {
    adapter: adapter({ fallback: 'index.html' })
  }
};

export default config;
