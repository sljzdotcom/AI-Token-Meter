import { fileURLToPath } from "node:url"

import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"

export default defineConfig({
  build: {
    emptyOutDir: true,
    outDir: "dist-density",
    rollupOptions: {
      input: fileURLToPath(new URL("density-browser.html", import.meta.url)),
    },
  },
  plugins: [react()],
  publicDir: "../Sources/AIMeterApp/Resources/Backgrounds",
})
