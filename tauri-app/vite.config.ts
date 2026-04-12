import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(async () => ({
  plugins: [react()],

  clearScreen: false,
  
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/backend/**"],
    },
  },

  build: {
    chunkSizeWarningLimit: 600,
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-react': ['react', 'react-dom'],
          'vendor-mui': ['@mui/material', '@mui/icons-material'],
          'vendor-tauri': ['@tauri-apps/api'],
          'vendor-i18n': ['i18next', 'react-i18next'],
          'vendor-utils': ['zustand'],
        },
      },
    },
  },
}));
