import { defineConfig } from "vite";

// GitHub Pages ではリポジトリ名のサブパスに置かれるため base を合わせる
export default defineConfig({
  base: process.env.BASE_PATH ?? "/",
  build: { target: "es2021", outDir: "dist" },
});
