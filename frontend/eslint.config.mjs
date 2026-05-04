import js from "@eslint/js"
import nextPlugin from "@next/eslint-plugin-next"
import tsPlugin from "typescript-eslint"
import reactHooks from "eslint-plugin-react-hooks"
import reactRefresh from "eslint-plugin-react-refresh"

export default [
  {
    ignores: [
      ".next/**",
      "out/**",
      "coverage/**",
      "node_modules/**",
      "public/workers/**",
      "tsconfig.tsbuildinfo",
      "next-env.d.ts",
      "types/**/*.d.ts",
      "gen/**/*.ts",
    ],
  },
  js.configs.recommended,
  ...tsPlugin.configs.recommended,
  {
    plugins: {
      "@next/next": nextPlugin,
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...nextPlugin.configs.recommended.rules,
      ...nextPlugin.configs.coreWebVitals.rules,
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      "react-refresh/only-export-components": "warn",
    },
  },
]
