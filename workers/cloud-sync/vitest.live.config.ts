import { defineConfig } from "vitest/config";

// Tests that talk to the real Google Cloud project. Kept out of `npm test`; see test-live/.
export default defineConfig({
  test: {
    include: ["test-live/**/*.test.ts"],
    testTimeout: 60_000,
    hookTimeout: 60_000,
  },
});
