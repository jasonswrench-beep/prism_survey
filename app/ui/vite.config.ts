import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// `npm run dev` proxies /rpc to the development bridge (`prismsurvey serve`, port 4892 by default), so the screens can
// be tried in a browser against the real engine. A Tauri build ignores the proxy.
export default defineConfig({
  base: './',
  plugins: [react()],
  build: { outDir: 'dist', sourcemap: false },
  clearScreen: false,
  server: {
    proxy: {
      '/rpc': { target: process.env.PRISMSURVEY_BRIDGE ?? 'http://127.0.0.1:4892', changeOrigin: false },
    },
  },
});
