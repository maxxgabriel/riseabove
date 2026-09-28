import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The interface talks to the simulation through one endpoint. In the desktop
// shell it is a Tauri command; in the browser it is the development server.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: { "/api": "http://127.0.0.1:8787" },
  },
  build: { target: "es2022", sourcemap: false },
  clearScreen: false,
});
