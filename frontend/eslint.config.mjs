import js from "@eslint/js"
import astro from "eslint-plugin-astro"
import tsParser from "@typescript-eslint/parser"
import tsPlugin from "@typescript-eslint/eslint-plugin"

export default [
  {
    ignores: [
      "node_modules/",
      "dist/",
      ".astro/",
      "public/",
      "scripts/",
      "wrangler.toml",
    ],
  },
  js.configs.recommended,
  {
    files: ["**/*.astro"],
    ...astro.configs["flat/recommended"],
  },
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: "latest",
        sourceType: "module",
      },
    },
    plugins: {
      "@typescript-eslint": tsPlugin,
    },
    rules: {
      ...tsPlugin.configs.recommended.rules,
      "no-console": "warn",
    },
  },
]
