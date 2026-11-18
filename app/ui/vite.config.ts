import { resolve } from 'node:path';
import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';
import { ICON_ROOT, writeIcons } from './scripts/build-icons';

// Writes src/assets/icons.generated.ts from assets/icons when a build starts, and again while the dev server runs
// whenever an icon file is added, changed or removed, so a newly drawn icon shows with no code change.
function icons(): Plugin {
  const root = resolve(ICON_ROOT).toLowerCase();
  const isIcon = (file: string) => resolve(file).toLowerCase().startsWith(root) && file.toLowerCase().endsWith('.svg');
  return {
    name: 'prismsurvey-icons',
    buildStart() {
      writeIcons();
    },
    configureServer(server) {
      server.watcher.add(ICON_ROOT);
      const regenerate = (file: string) => {
        if (!isIcon(file)) return;
        try {
          if (writeIcons()) server.config.logger.info(`icons: ${file} picked up`);
        } catch (e) {
          server.config.logger.error(`icons: ${(e as Error).message}`);
        }
      };
      for (const event of ['add', 'change', 'unlink'] as const) server.watcher.on(event, regenerate);
    },
  };
}

// `npm run dev` proxies /rpc to the development bridge (`prismsurvey serve`, port 4892 by default), so the screens can
// be tried in a browser against the real engine. A Tauri build ignores the proxy.
export default defineConfig({
  base: './',
  plugins: [icons(), react()],
  build: { outDir: 'dist', sourcemap: false },
  clearScreen: false,
  server: {
    proxy: {
      '/rpc': { target: process.env.PRISMSURVEY_BRIDGE ?? 'http://127.0.0.1:4892', changeOrigin: false },
    },
  },
});
