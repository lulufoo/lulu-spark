import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const gone = [
  "dialogue-summary",
  "dialogue-archive",
  "theme-line",
  "theme-transcribe",
];

describe("spark skills pack keeps only note-task", () => {
  it("producer directories are gone", () => {
    for (const name of gone) {
      assert.equal(fs.existsSync(path.join(root, name)), false, name);
    }
  });

  it("note-task remains and pack name is unchanged", () => {
    assert.equal(fs.existsSync(path.join(root, "note-task/SKILL.md")), true);
    const text = fs.readFileSync(path.join(root, "SKILL.md"), "utf8");
    assert.match(text, /^name: lulu-spark-skills$/m);
    assert.match(text, /发现子 skill（`note-task`）/);
    for (const name of gone) {
      assert.equal(
        new RegExp(`发现子 skill（[^）]*${name}`).test(text),
        false,
        `discovery must not include ${name}`,
      );
      assert.equal(new RegExp(`^\\| \`${name}\` \\|`, "m").test(text), false);
    }
  });
});
