import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const skillMd = path.join(root, "SKILL.md");
const readme = path.join(root, "README.md");
const fetchDir = path.join(root, "theme-fetch");

describe("theme-fetch skill removal", () => {
  it("theme-fetch directory does not exist", () => {
    assert.equal(fs.existsSync(fetchDir), false);
  });

  it("SKILL.md description, discovery list, and sub-skill table omit theme-fetch", () => {
    const text = fs.readFileSync(skillMd, "utf8");
    const frontmatter = text.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
    assert.equal(
      /\btheme-fetch\b/.test(frontmatter),
      false,
      "frontmatter description must not advertise theme-fetch",
    );
    assert.equal(
      /发现子 skill（[^）]*theme-fetch/.test(text),
      false,
      "discovery list must not include theme-fetch",
    );
    assert.equal(
      /^\| `theme-fetch` \|/m.test(text),
      false,
      "sub-skill table must not list theme-fetch",
    );
  });

  it("README and sibling skills omit theme-fetch", () => {
    const files = [
      readme,
      path.join(root, "theme-line/SKILL.md"),
      path.join(root, "theme-transcribe/SKILL.md"),
      path.join(root, "note-task/SKILL.md"),
    ];
    for (const file of files) {
      const text = fs.readFileSync(file, "utf8");
      assert.equal(
        /\btheme-fetch\b/.test(text),
        false,
        `${path.relative(root, file)} must not mention theme-fetch`,
      );
    }
  });
});
