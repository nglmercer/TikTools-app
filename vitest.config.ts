import { defineConfig, mergeConfig } from 'vitest/config';
import viteConfig from './vite.config.ts';

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      root: process.cwd(),
      environment: 'happy-dom',
      // Reuse the DOM runtime per worker while retaining isolated module/global
      // contexts for every file. Do not use isolate:false for stateful stores.
      pool: 'vmThreads',
      isolate: true,
      include: ['src/web/**/*.test.ts'],
      restoreMocks: true,
    },
  }),
);
