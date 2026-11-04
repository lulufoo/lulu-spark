import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const subSkills = ["spark-task"];
const gone = [
  "note-task",
  "dialogue-summary",
  "dialogue-archive",
  "theme-line",
  "theme-transcribe",
  "spark-auto",
];

const read = (...parts) => fs.readFileSync(path.join(root, ...parts), "utf8");

describe("spark skills pack sub-skills", () => {
  it("retired directories are gone", () => {
    for (const name of gone) {
      assert.equal(fs.existsSync(path.join(root, name)), false, name);
    }
  });

  it("spark-task is the sub-skill, and the pack name is unchanged", () => {
    const found = fs
      .readdirSync(root, { withFileTypes: true })
      .filter((e) => e.isDirectory() && fs.existsSync(path.join(root, e.name, "SKILL.md")))
      .map((e) => e.name)
      .sort();
    assert.deepEqual(found, subSkills);
    for (const name of subSkills) {
      assert.match(read(name, "SKILL.md"), new RegExp(`^name: ${name}$`, "m"));
    }
    const text = read("SKILL.md");
    assert.match(text, /^name: lulu-spark-skills$/m);
    assert.match(text, /discovers sub-skills \(`spark-task`\)/);
    for (const name of subSkills) {
      assert.match(text, new RegExp(`^\\| \`${name}\` \\|`, "m"), `sub-skill table must list ${name}`);
    }
    for (const name of gone) {
      assert.equal(new RegExp(`^\\| \`${name}\` \\|`, "m").test(text), false);
    }
  });

  it("remaining pack documents omit spark-auto", () => {
    const text = read("SKILL.md");
    assert.equal(/\bspark-auto\b/.test(text), false, "pack SKILL.md must not mention spark-auto");
    const sparkTaskFiles = fs
      .readdirSync(path.join(root, "spark-task"), { recursive: true, withFileTypes: true })
      .filter((e) => e.isFile() && e.name.endsWith(".md"))
      .map((e) => path.join(e.parentPath ?? e.path, e.name));
    assert.ok(sparkTaskFiles.length > 0);
    for (const file of sparkTaskFiles) {
      assert.equal(
        /\bspark-auto\b/.test(fs.readFileSync(file, "utf8")),
        false,
        `${path.relative(root, file)} must not mention spark-auto`,
      );
    }
  });
});
