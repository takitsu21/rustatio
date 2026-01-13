import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';
import { readFileSync } from 'fs';

// Read version from package.json
const pkg = JSON.parse(readFileSync('./package.json', 'utf-8'));

// https://vitejs.dev/config/
export default defineConfig({
  base: process.env.GITHUB_PAGES ? '/rustatio/' : '/', // Only use /rustatio/ for GitHub Pages
  plugins: [svelte()],

  // Expose version at build time
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },

  resolve: {
    alias: {
      $lib: path.resolve('./src/lib'),
    },
  },

  // Prevent vite from obscuring errors
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: false,
  },

  build: {
    target: 'esnext', // Required for top-level await in WASM
    minify: 'esbuild',
    sourcemap: false,
    chunkSizeWarningLimit: 1500,

    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('echarts') || id.includes('zrender')) {
            return 'echarts';
          }
        },
      },
    },
  },

  optimizeDeps: {
    exclude: ['$lib/wasm/rustatio_wasm.js'],
  },
});
