import { defineConfig, loadEnv } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { readFileSync } from "node:fs";

const { version } = JSON.parse(readFileSync("./package.json", "utf-8")) as { version: string };

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, ".", "TAURI_");
  const host = env.TAURI_DEV_HOST;

  return {
    plugins: [svelte()],
    clearScreen: false,
    define: {
      __APP_VERSION__: JSON.stringify(version),
    },
    server: {
      port: 1420,
      strictPort: true,
      host: host || "127.0.0.1",
      hmr: host
        ? {
            protocol: "ws",
            host,
            port: 1421,
          }
        : undefined,
      watch: {
        ignored: ["**/src-tauri/**"],
      },
    },
  };
});
