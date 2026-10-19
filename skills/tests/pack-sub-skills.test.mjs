import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const subSkills = ["spark-auto", "spark-task"];
const gone = [
  "note-task",
  "dialogue-summary",
  "dialogue-archive",
  "theme-line",
  "theme-transcribe",
];
// Skill names outside this pack; spark-auto must not depend on any of them.
const foreignSkills = [
  "lulu-dialogue-skills",
  "lulu-discipline-skills",
  "lulu-workflow",
  "converge",
  "clash",
  "grill",
  "verify",
  "ground",
];

const read = (...parts) => fs.readFileSync(path.join(root, ...parts), "utf8");

describe("spark skills pack sub-skills", () => {
  it("retired directories are gone", () => {
    for (const name of gone) {
      assert.equal(fs.existsSync(path.join(root, name)), false, name);
    }
  });

  it("spark-task and spark-auto are the sub-skills, and the pack name is unchanged", () => {
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
    assert.match(text, /discovers sub-skills \(`spark-task`, `spark-auto`\)/);
    for (const name of subSkills) {
      assert.match(text, new RegExp(`^\\| \`${name}\` \\|`, "m"), `sub-skill table must list ${name}`);
    }
    for (const name of gone) {
      assert.equal(new RegExp(`^\\| \`${name}\` \\|`, "m").test(text), false);
    }
  });
});

describe("spark-auto saves on signal", () => {
  const text = read("spark-auto", "SKILL.md");

  it("description carries the signal trigger words", () => {
    const frontmatter = text.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
    for (const word of ["记一下", "原来如此", "下次注意", "学到了", "overturned", "achieved"]) {
      assert.match(frontmatter, new RegExp(word), `description must carry ${word}`);
    }
  });

  it("has the five-row signal table, templates, write contract, and variables", () => {
    assert.match(text, /^## Signals$/m);
    for (const n of [1, 2, 3, 4, 5]) {
      assert.match(text, new RegExp(`^\\| ${n} \\| `, "m"), `signal row ${n}`);
    }
    assert.match(text, /^## Shape$/m);
    for (const headings of ["选项 / 选择 / 理由", "原以为 / 实际 / 依据", "目标 / 结果", "Problem / What we learned / Takeaways", "原话"]) {
      assert.match(text, new RegExp(headings), `template headings ${headings}`);
    }
    assert.match(text, /^## Write contract$/m);
    assert.match(text, /^## Variables$/m);
    assert.match(text, /`\$SPARK_TARGET_WORKSPACE`/);
    assert.match(text, /`\$SPARK_TARGET_PROJECT`/);
  });

  it("writes through create_doc with a category from list_doc_categories, omitting the target ids", () => {
    assert.match(text, /`list_doc_categories`/);
    assert.match(text, /`create_doc`/);
    assert.match(text, /Omit `workspace_id` and `project_id`/);
    assert.equal(/`source_type`[^\n]*`jot`/.test(text), false, "source_type must not be fixed to jot");
    assert.match(text, /已存入 Spark：<title>/);
    assert.match(text, /建议存入 Spark：<title>/);
  });

  it("depends on no skill outside the pack, and no sibling depends on it", () => {
    for (const name of foreignSkills) {
      assert.equal(new RegExp(`\\b${name}\\b`).test(text), false, `spark-auto must not mention ${name}`);
    }
    const sparkTaskFiles = fs
      .readdirSync(path.join(root, "spark-task"), { recursive: true, withFileTypes: true })
      .filter((e) => e.isFile() && e.name.endsWith(".md"))
      .map((e) => path.join(e.parentPath ?? e.path, e.name));
    assert.ok(sparkTaskFiles.length > 0);
    for (const file of sparkTaskFiles) {
      assert.equal(
        /spark-auto/.test(fs.readFileSync(file, "utf8")),
        false,
        `${path.relative(root, file)} must not mention spark-auto`,
      );
    }
  });
});
