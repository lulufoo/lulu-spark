import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const skillMd = path.join(root, "SKILL.md");
const todoDir = path.join(root, "todo-task");

describe("todo-task skill removal", () => {
  it("todo-task directory does not exist", () => {
    assert.equal(fs.existsSync(todoDir), false);
  });

  it("SKILL.md description, discovery list, and sub-skill table omit todo-task", () => {
    const text = fs.readFileSync(skillMd, "utf8");
    const frontmatter = text.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
    assert.equal(
      /\btodo-task\b/.test(frontmatter),
      false,
      "frontmatter description must not advertise todo-task",
    );
    assert.equal(
      /发现子 skill（[^）]*todo-task/.test(text),
      false,
      "discovery list must not include todo-task",
    );
    assert.equal(
      /^\| `todo-task` \|/m.test(text),
      false,
      "sub-skill table must not list todo-task",
    );
  });
});
