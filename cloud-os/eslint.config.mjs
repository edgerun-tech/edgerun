// SPDX-License-Identifier: Apache-2.0
import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import solid from 'eslint-plugin-solid'

export default [
  {
    ignores: ['node_modules/**', 'dist/**', '.astro/**', '../out/**']
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['**/*.js', '**/*.mjs', '**/*.cjs'],
    languageOptions: {
      globals: {
        ...globals.node
      }
    }
  },
  {
    files: ['**/*.ts', '**/*.tsx'],
    languageOptions: {
      parserOptions: {
        ecmaFeatures: { jsx: true }
      },
      globals: {
        ...globals.browser,
        ...globals.node
      }
    },
    plugins: {
      solid
    },
    rules: {
      ...solid.configs.typescript.rules,
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-empty-object-type': 'warn',
      '@typescript-eslint/no-unused-vars': 'warn',
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      'no-empty': 'warn',
      'no-unassigned-vars': 'warn',
      'no-unused-vars': 'off',
      'solid/no-innerhtml': 'warn',
      'solid/prefer-for': 'warn'
    }
  }
]
