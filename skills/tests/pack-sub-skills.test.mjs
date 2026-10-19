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

  it("description names the host-bound $SPARK_AUTO_SIGNALS and defines no signal itself", () => {
    const frontmatter = text.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
    assert.match(frontmatter, /`\$SPARK_AUTO_SIGNALS`/);
    for (const word of ["记一下", "原来如此", "下次注意", "学到了", "overturned", "achieved"]) {
      assert.equal(new RegExp(word).test(text), false, `spark-auto must not define the signal ${word}`);
    }
  });

  it("has Variables, Signal, Save, and Done, and no note template", () => {
    for (const section of ["Variables", "Signal", "Save", "Done"]) {
      assert.match(text, new RegExp(`^## ${section}$`, "m"), `section ${section}`);
    }
    assert.equal(/^## Shape$/m.test(text), false, "spark-auto must not prescribe a note template");
    for (const headings of ["选项 / 选择 / 理由", "原以为 / 实际 / 依据", "Problem / What we learned / Takeaways"]) {
      assert.equal(new RegExp(headings).test(text), false, `template headings ${headings}`);
    }
  });

  it("binds the signal set and the target ids to the host, omitting an unbound one", () => {
    assert.match(text, /^\| `\$SPARK_AUTO_SIGNALS` \| Host-bound; required \| The signal set\. Unbound → no automatic save \|$/m);
    assert.match(text, /^\| `\$SPARK_TARGET_WORKSPACE` \| Host-bound; optional \| `workspace_id` of the save\. Unbound → not passed \|$/m);
    assert.match(text, /^\| `\$SPARK_TARGET_PROJECT` \| Host-bound; optional \| `project_id` of the save\. Unbound → not passed \|$/m);
    assert.equal(/not consumed/.test(text), false);
    assert.equal(/Omit `workspace_id` and `project_id`/.test(text), false);
    assert.match(text, /match each bound variable/);
  });

  it("delegates the write to spark-task and fixes only source_type and the target ids", () => {
    assert.match(text, /`spark-task` create flow, with `source_type` fitting the unit and the target ids from Variables/);
    assert.equal(/`source_type`[^\n]*`jot`/.test(text), false, "source_type must not be fixed to jot");
    assert.equal(/`digest`/.test(text), false, "digest belongs to spark-task");
    assert.match(text, /已存入 Spark：<title>/);
    assert.equal(/建议存入/.test(text), false, "permission handling belongs to host rules");
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
