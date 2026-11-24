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
    files: ['src/**/*.{js,mjs,cjs,ts,tsx,jsx,vue}', 'tests/unit/**/*.{js,mjs,cjs,ts,tsx,jsx,vue}', 'docs/examples/**/*.{ts,vue}', 'docs/.vitepress/theme/**/*.{ts,vue}'],
    languageOptions: {
      globals: globals.browser,
    },
    rules: {
      'no-empty': ['error', { allowEmptyCatch: true }],
      '@typescript-eslint/no-empty-object-type': ['error', { allowInterfaces: 'with-single-extends' }],
    },
  },
  {
    files: ['packages/*/src/**/*.ts'],
    languageOptions: {
      globals: globals.browser,
    },
  },
  {
    files: ['scripts/**/*.{js,mjs,cjs,ts,tsx,jsx}', 'tests/bun/**/*.{js,mjs,cjs,ts,tsx,jsx}', 'docs/scripts/**/*.ts', 'docs/.vitepress/*.ts', 'vite*.config.ts', 'vitest*.config.ts'],
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
    // Tests must resolve translated UI text from the catalogs via
    // testMessage(locale, key), never from a locale ternary with literals.
    files: ['tests/**/*.ts', 'src/**/*.test.ts'],
    rules: {
      'no-restricted-syntax': [
        'error',
        {
          selector: `ConditionalExpression[test.type="BinaryExpression"][test.left.name="locale"] > Literal[raw=/^['"]/]`,
          message: 'Use testMessage(locale, key) instead of a hardcoded translated string.',
        },
      ],
    },
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
      // One disclosure primitive everywhere: BaseDisclosure. Native summaries
      // cannot be styled consistently and cannot be placed inline in a row.
      'no-restricted-syntax': [
        'error',
        { selector: 'VElement[name="details"]', message: 'Use <BaseDisclosure> instead of <details>.' },
        { selector: 'VElement[name="summary"]', message: 'Use <BaseDisclosure> instead of <summary>.' },
      ],
    },
  },
);
