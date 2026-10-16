import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function markdownFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return entry.name === "tests" ? [] : markdownFiles(full);
    return entry.name.endsWith(".md") ? [full] : [];
  });
}

const RETIRED_TOOLS = [
  "create_note",
  "update_note",
  "get_note_digest",
  "get_note_file",
  "list_notes_categories",
  "get_knowledge_digest",
  "get_knowledge_file",
  "list_knowledge_categories",
];

const GENERIC_TOOLS = [
  "list_workspaces",
  "list_projects",
  "create_doc",
  "get_doc_digest",
  "get_doc_file",
  "update_doc",
  "search",
];

describe("skills use the generic docs MCP tools", () => {
  it("no skill document mentions a retired notes or knowledge tool", () => {
    for (const file of markdownFiles(root)) {
      const text = fs.readFileSync(file, "utf8");
      for (const tool of RETIRED_TOOLS) {
        assert.equal(
          new RegExp(`\\b${tool}\\b`).test(text),
          false,
          `${path.relative(root, file)} must not mention ${tool}`,
        );
      }
    }
  });

  it("note-task lists every generic tool", () => {
    const text = fs.readFileSync(path.join(root, "note-task", "SKILL.md"), "utf8");
    for (const tool of GENERIC_TOOLS) {
      assert.match(text, new RegExp(`\`${tool}\``), `note-task must list ${tool}`);
    }
  });

  it("note-task confirms the workspace and project, and defaults to Default / inbox", () => {
    const text = fs.readFileSync(path.join(root, "note-task", "SKILL.md"), "utf8");
    assert.match(text, /list_workspaces/);
    assert.match(text, /list_projects/);
    assert.match(text, /`Default`/);
    assert.match(text, /`inbox`/);
  });

  it("path-or-paste names `folder`, not the retired `theme` field", () => {
    const text = fs.readFileSync(path.join(root, "note-task", "references", "path-or-paste.md"), "utf8");
    assert.match(text, /^\| `folder` \|/m);
    assert.equal(/^\| `theme` \|/m.test(text), false);
  });
});
