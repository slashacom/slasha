import { reactRouter } from '@react-router/dev/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig, loadEnv } from 'vite';
import tsconfigPaths from 'vite-tsconfig-paths';

export default defineConfig(({ mode, command }) => {
  const env = loadEnv(mode, '../');
  const port = env.SLASHA_PORT || '3000';

  return {
    plugins: [tailwindcss(), reactRouter(), tsconfigPaths()],
    server: {
      proxy: {
        '/api': {
          target: `http://localhost:${port}`,
          changeOrigin: true,
        },
      },
    },
    resolve: {
      alias: {
        ...(command === 'build'
          ? { 'react-dom/server': 'react-dom/server.node' }
          : {}),
      },
    },
  };
});
