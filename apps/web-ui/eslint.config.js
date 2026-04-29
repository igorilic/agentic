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
      // set-state-in-effect (new in v7) is disabled: our "clear on dep change"
      // pattern (setEvents([]) at the top of effects) is an intentional React
      // idiom that avoids stale state. Refactoring to derived state would require
      // significant architectural changes across multiple components.
      // Tech-debt tracked in GH #86.
      ...reactHooksPlugin.configs.recommended.rules,
      "react-hooks/set-state-in-effect": "off",
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
      // checksVoidReturn.attributes=false: allows `onClick={async () => ...}`
      // patterns without forcing every React event handler to be non-async.
      "@typescript-eslint/no-misused-promises": [
        "error",
        { checksVoidReturn: { attributes: false } },
      ],
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
