import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  build: {
    target: 'esnext',        // Tauri uses modern Chromium, no need for legacy transforms
    minify: 'esbuild',       // Faster than terser
    cssMinify: true,
    rollupOptions: {
      output: {
        manualChunks: {
          'mathjax': ['mathjax'],
          'chart': ['chart.js'],
        }
      }
    }
  },
  optimizeDeps: {
    include: ['@tauri-apps/api/core', '@tauri-apps/plugin-dialog', '@tauri-apps/plugin-fs']
  }
})
