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

// The five MCP groups (LW-404), in Settings order.
const GROUPS = {
  Global: ["search"],
  Workspace: ["list_workspaces", "create_workspace", "rename_workspace"],
  Project: ["list_projects", "create_project", "update_project"],
  Document: [
    "list_doc_categories",
    "get_doc_digest",
    "get_doc_file",
    "get_doc_content",
    "create_doc",
    "update_doc",
    "delete_doc",
  ],
  "Read Later": ["get_read_later"],
};
const ALL_TOOLS = Object.values(GROUPS).flat();

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

  it("spark-task lists every tool of the five groups, each under its group", () => {
    const text = fs.readFileSync(path.join(root, "spark-task", "SKILL.md"), "utf8");
    for (const tool of ALL_TOOLS) {
      assert.match(text, new RegExp(`\`${tool}\``), `spark-task must list ${tool}`);
    }
    for (const [group, tools] of Object.entries(GROUPS)) {
      for (const tool of tools) {
        const row = new RegExp(`^\\| \`${tool}\` \\| ${group} \\|`, "m");
        assert.match(text, row, `${tool} must be listed under ${group}`);
      }
    }
  });

  it("spark-task routes workspace, project, document, search and Read Later intents", () => {
    const text = fs.readFileSync(path.join(root, "spark-task", "SKILL.md"), "utf8");
    for (const intent of [
      "Create a workspace",
      "Rename a workspace",
      "Create a project",
      "Edit a project",
      "Create / save / archive a document",
      "Search documents",
      "Read one document body",
      "Edit an existing document",
      "Delete a document",
      "Open a Read Later link",
    ]) {
      assert.match(text, new RegExp(`^\\| ${intent}`, "m"), `missing route: ${intent}`);
    }
  });

  it("spark-task states which tools are not on every channel", () => {
    const text = fs.readFileSync(path.join(root, "spark-task", "SKILL.md"), "utf8");
    assert.match(text, /`get_doc_content`[^\n]*mobile-only/);
    assert.match(text, /`delete_doc` and `get_read_later` are offered on `\/mcp\/spark` only/);
  });

  it("spark-task confirms the workspace and project, and omits project for the inbox", () => {
    const text = fs.readFileSync(path.join(root, "spark-task", "SKILL.md"), "utf8");
    assert.match(text, /list_workspaces/);
    assert.match(text, /list_projects/);
    assert.match(text, /`Default`/);
    assert.match(text, /omit `project_id`/);
    assert.match(text, /is_default/);
    assert.equal(/project_id: "inbox"/.test(text), true);
    assert.equal(/project `inbox`/.test(text), false);
  });

  it("digest_body shape is not a skill reference file", () => {
    assert.equal(
      fs.existsSync(path.join(root, "spark-task", "references", "digest-body.md")),
      false,
    );
    const text = fs.readFileSync(path.join(root, "spark-task", "SKILL.md"), "utf8");
    assert.equal(/digest-body/.test(text), false);
    assert.match(text, /live MCP `digest_body` description/);
  });

  it("path-or-paste names `folder`, not the retired `theme` field", () => {
    const text = fs.readFileSync(path.join(root, "spark-task", "references", "path-or-paste.md"), "utf8");
    assert.match(text, /^\| `folder` \|/m);
    assert.equal(/^\| `theme` \|/m.test(text), false);
  });
});
