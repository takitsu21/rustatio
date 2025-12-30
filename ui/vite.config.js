import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  base: process.env.GITHUB_PAGES ? '/rustatio/' : '/', // Only use /rustatio/ for GitHub Pages
  plugins: [svelte()],

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
