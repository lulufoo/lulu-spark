import { defineConfig } from "vitest/config";

// Own config: without it Vitest falls back to the app's root config, which only runs *.test.js.
export default defineConfig({
  test: {
    include: ["test/**/*.test.ts"],
  },
});
