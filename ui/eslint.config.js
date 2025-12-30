import js from '@eslint/js';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';

export default [
  js.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    rules: {
      // Disable overly strict rules
      'svelte/no-useless-children-snippet': 'off', // shadcn pattern is fine
      'svelte/no-at-html-tags': 'warn', // Warn but don't error for {@html}
      'svelte/prefer-svelte-reactivity': 'off', // Set is fine for non-reactive use
      'no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
    },
  },
  {
    ignores: ['dist/', 'node_modules/', '.svelte-kit/', 'src/lib/wasm/'],
  },
];
