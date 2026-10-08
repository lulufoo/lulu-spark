import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const producers = [
  "dialogue-summary",
  "dialogue-archive",
  "theme-line",
  "theme-transcribe",
];
const forbidden = /note-task|sink=spark|load note-task|handoff note-task/;

function walkMd(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...walkMd(full));
    } else if (entry.name.endsWith(".md")) {
      out.push(full);
    }
  }
  return out;
}

describe("producer skills do not hand off note-task", () => {
  for (const name of producers) {
    it(`${name} markdown omits note-task / spark sink`, () => {
      const files = walkMd(path.join(root, name));
      assert.ok(files.length > 0, `${name} has markdown`);
      for (const file of files) {
        const text = fs.readFileSync(file, "utf8");
        assert.equal(
          forbidden.test(text),
          false,
          `${path.relative(root, file)} must not mention note-task or spark sink`,
        );
      }
    });
  }
});
