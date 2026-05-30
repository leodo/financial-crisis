import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          react: ["react", "react-dom", "@tanstack/react-query"],
          charts: ["echarts", "echarts-for-react"],
          icons: ["lucide-react"]
        }
      }
    }
  },
  server: {
    port: 5173,
    proxy: {
      "/api": "http://127.0.0.1:18080",
      "/health": "http://127.0.0.1:18080"
    }
  }
});
