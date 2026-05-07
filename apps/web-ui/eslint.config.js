import tseslint from "typescript-eslint";
import reactPlugin from "eslint-plugin-react";
import reactHooksPlugin from "eslint-plugin-react-hooks";

export default tseslint.config(
  // Global ignores
  {
    ignores: ["dist/", "node_modules/", ".vite/"],
  },

  // Base TypeScript config (includes recommended rules)
  ...tseslint.configs.recommended,

  // React + React Hooks rules for source files
  {
    files: ["src/**/*.{ts,tsx}"],
    plugins: {
      react: reactPlugin,
      "react-hooks": reactHooksPlugin,
    },
    languageOptions: {
      parserOptions: {
        // Enables type-aware lint rules (no-floating-promises, no-misused-promises).
        // Uses tsconfig.json auto-resolution — no explicit path needed.
        project: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    settings: {
      react: {
        version: "detect",
      },
    },
    rules: {
      // React rules
      ...reactPlugin.configs.recommended.rules,
      // React Hooks rules — this is the main gate that catches the original bug.
      // set-state-in-effect (v7+) is enabled globally. The two App.tsx violations
      // (findingsRefetchKey, findingsRunId sync) were refactored to useMemo and
      // the render-time "storing information from previous renders" pattern (GH #86).
      // The remaining intentional patterns (fetch-on-mount in PastRunsPane/SettingsPane,
      // clear-before-refetch in useFindings/useTauriEvents) each carry a per-line
      // eslint-disable comment with a rationale explaining why the pattern is safe.
      ...reactHooksPlugin.configs.recommended.rules,
      // No need to import React in scope (JSX runtime handles it)
      "react/react-in-jsx-scope": "off",
      "react/prop-types": "off",
      // TypeScript-specific
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/no-explicit-any": "error",
      // Type-aware promise safety rules. These require `parserOptions.project`
      // above and catch fire-and-forget async calls that swallow errors.
      "@typescript-eslint/no-floating-promises": "error",
      // no-misused-promises with full checks — JSX event handlers that pass
      // an async function must use `() => void asyncFn()` to make the ignored
      // promise explicit and prevent unhandled rejection surprises.
      "@typescript-eslint/no-misused-promises": "error",
    },
  },

  // Relax rules for test files
  {
    files: ["src/**/*.test.{ts,tsx}", "src/__tests__/**/*.{ts,tsx}"],
    rules: {
      // Test mocks routinely need `any` for flexibility
      "@typescript-eslint/no-explicit-any": "off",
      // Test utilities often use require-style imports in mock factories
      "@typescript-eslint/no-require-imports": "off",
    },
  },
);
