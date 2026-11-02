import eslint from '@eslint/js';
import globals from 'globals';
import vue from 'eslint-plugin-vue';
import tseslint from 'typescript-eslint';
import vueParser from 'vue-eslint-parser';

export default tseslint.config(
  {
    ignores: [
      'dist/**',
      'node_modules/**',
      'out/**',
      'release/**',
      'target/**',
      'vendor/**',
      '.dev-plugins/**',
      '.plugin-staging/**',
    ],
  },
  eslint.configs.recommended,
  ...vue.configs['flat/essential'],
  ...tseslint.configs.recommended,
  {
    files: ['**/*.{js,mjs,cjs,ts,tsx,jsx,vue}'],
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
      parserOptions: {
        ecmaFeatures: { jsx: true },
      },
    },
  },
  {
    files: ['src/**/*.{js,mjs,cjs,ts,tsx,jsx,vue}'],
    languageOptions: {
      globals: globals.browser,
    },
    rules: {
      'no-empty': ['error', { allowEmptyCatch: true }],
      '@typescript-eslint/no-empty-object-type': ['error', { allowInterfaces: 'with-single-extends' }],
    },
  },
  {
    files: ['plugins/sonicboom/ui/src/**/*.{ts,tsx,vue}', 'plugins/sonicboom/ui/tests/**/*.ts'],
    languageOptions: {
      globals: globals.browser,
    },
  },
  {
    files: ['packages/*/src/**/*.ts'],
    languageOptions: {
      globals: globals.browser,
    },
  },
  {
    files: ['plugins/sonicboom/ui/playwright.config.ts', 'plugins/sonicboom/ui/vite.config.ts'],
    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
  },
  {
    files: ['scripts/**/*.{js,mjs,cjs,ts,tsx,jsx}', 'vite.config.ts', 'vitest.config.ts'],
    languageOptions: {
      globals: {
        ...globals.node,
        Bun: 'readonly',
      },
    },
  },
  {
    files: ['src/web/**/*.vue'],
    rules: Object.assign({}, ...vue.configs['flat/recommended'].map((config) => config.rules ?? {})),
  },
  {
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: tseslint.parser,
      },
    },
    rules: {
      // A few render-function modules intentionally export several primitives.
      'vue/one-component-per-file': 'off',
    },
  },
);
