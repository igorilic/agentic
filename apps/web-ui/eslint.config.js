import tseslint from "typescript-eslint";
import reactPlugin from "eslint-plugin-react";
import reactHooksPlugin from "eslint-plugin-react-hooks";

export default tseslint.config(
  // Global ignores
  {
    ignores: ["dist/", "node_modules/", ".vite/"],
  },

  // Base TypeScript config
  ...tseslint.configs.recommended,

  // React + React Hooks rules for source files
  {
    files: ["src/**/*.{ts,tsx}"],
    plugins: {
      react: reactPlugin,
      "react-hooks": reactHooksPlugin,
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
