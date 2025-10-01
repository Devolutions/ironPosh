import { defineConfig } from 'vite';
import { resolve } from 'path';
import { execSync } from 'child_process';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  plugins: [
    wasm(),
    {
      name: 'build-types',
      closeBundle() {
        console.log('\nGenerating TypeScript declarations...');
        execSync('tsc --emitDeclarationOnly', { stdio: 'inherit' });
      }
    }
  ],
  build: {
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      name: 'PowerShellTerminal',
      fileName: 'powershell-terminal',
      formats: ['es']
    },
    rollupOptions: {
      external: [],
      output: {
        assetFileNames: 'assets/[name][extname]',
        entryFileNames: '[name].js',
      }
    }
  },
  server: {
    fs: {
      allow: ['..']
    }
  }
});
