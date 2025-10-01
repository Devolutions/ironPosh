import { defineConfig } from "vite";
import { resolve } from "path";
import { execSync } from "child_process";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

export default defineConfig({
  assetsInclude: ["**/*.wasm", "**/*.wasm?inline"],
  plugins: [
    wasm(),
    topLevelAwait(),
    {
      name: "build-types",
      closeBundle() {
        console.log("\nGenerating TypeScript declarations...");
        execSync("tsc --emitDeclarationOnly", { stdio: "inherit" });
      },
    },
  ],
  build: {
    lib: {
      entry: resolve(__dirname, "src/index.ts"),
      name: "PowerShellTerminal",
      fileName: "powershell-terminal",
      formats: ["es"],
    },
    rollupOptions: {
      external: [],
      output: {
        assetFileNames: "assets/[name][extname]",
        entryFileNames: "[name].js",
      },
    },
  },
  server: {
    fs: {
      allow: [".."],
    },
  },
});
