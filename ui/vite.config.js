import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';
import { readFileSync } from 'fs';
import { THEMES } from './src/lib/themes/themeRegistry.js';

// Read version from package.json
const pkg = JSON.parse(readFileSync('./package.json', 'utf-8'));

// Inject the theme id -> tone map into index.html so the pre-paint script can
// validate saved theme ids without duplicating the registry
function themeTones() {
  return {
    name: 'rustatio-theme-tones',
    transformIndexHtml(html) {
      const tones = Object.fromEntries(
        Object.entries(THEMES).map(([id, info]) => [id, info.isDark])
      );
      return html.replaceAll('__THEME_TONES__', JSON.stringify(tones));
    },
  };
}

// https://vitejs.dev/config/
export default defineConfig({
  base: process.env.GITHUB_PAGES ? '/rustatio/' : '/', // Only use /rustatio/ for GitHub Pages
  plugins: [themeTones(), svelte()],

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
