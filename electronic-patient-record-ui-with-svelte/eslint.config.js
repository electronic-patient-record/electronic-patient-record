import js from "@eslint/js";
import prettier from "eslint-config-prettier";
import svelte from "eslint-plugin-svelte";
import globals from "globals";
import ts from "typescript-eslint";

export default ts.config(
  {
    ignores: [
      "build/",
      ".svelte-kit/",
      "node_modules/",
      "test-results/",
      "playwright-report/",
    ],
  },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs["flat/recommended"],
  prettier,
  ...svelte.configs["flat/prettier"],
  {
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
  },
  {
    rules: {
      // An argument named with a leading underscore is unused on purpose.
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_" },
      ],
      // Links go through `localizePath` (locale prefix), the project's own resolver; the rule only
      // knows SvelteKit's `resolve`, so it would flag every link.
      "svelte/no-navigation-without-resolve": "off",
    },
  },
  {
    // Tests stub JSON bodies of many shapes; `any` there is a convenience, not a hole in a guard.
    files: ["tests/**"],
    rules: { "@typescript-eslint/no-explicit-any": "off" },
  },
  {
    // SvelteKit's own boilerplate declares empty interfaces to be filled in.
    files: ["src/app.d.ts"],
    rules: { "@typescript-eslint/no-empty-object-type": "off" },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts"],
    languageOptions: { parserOptions: { parser: ts.parser } },
  },
);
