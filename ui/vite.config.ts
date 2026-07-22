import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"

// Tauri expects a fixed dev port (see crates/app/tauri.conf.json).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: "chrome110", outDir: "dist" },
})
