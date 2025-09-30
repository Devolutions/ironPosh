import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  build: {
    target: 'esnext',
    rollupOptions: {
      external: [],
    },
  },
  optimizeDeps: {
    exclude: ['ironposh-web'],
  },
  assetsInclude: ['**/*.wasm'],
  server: {
    fs: {
      // Allow serving files from the ironwinrm root directory
      allow: [
        path.resolve(__dirname, '../..'),
      ],
    },
  },
});