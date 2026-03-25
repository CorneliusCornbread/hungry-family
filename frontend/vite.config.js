import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: '../static',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      // Proxy API calls to your Axum server during dev
      '/api': 'http://localhost:800',
    }
  }
})