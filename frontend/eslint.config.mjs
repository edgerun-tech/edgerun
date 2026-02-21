import js from '@eslint/js'
import globals from 'globals'
import tseslint from 'typescript-eslint'
import jsxA11y from 'eslint-plugin-jsx-a11y'
import solid from 'eslint-plugin-solid'

export default [
  {
    ignores: ['node_modules/**', '.next/**', '../out/**', 'control-panel/**', 'public/wasm/**']
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
      'jsx-a11y': jsxA11y,
      solid
    },
    rules: {
      ...jsxA11y.configs.recommended.rules,
      ...solid.configs.typescript.rules,
      '@typescript-eslint/no-explicit-any': 'off',
      'no-console': ['error', { allow: ['warn', 'error'] }]
    }
  }
]
