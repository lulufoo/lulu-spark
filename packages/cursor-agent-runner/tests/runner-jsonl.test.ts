import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = join(__dirname, "..");
const ENTRY = join(PKG_ROOT, "dist", "index.js");

const temps: string[] = [];

afterEach(() => {
  while (temps.length) {
    const p = temps.pop();
    if (p) rmSync(p, { recursive: true, force: true });
  }
});

type JsonlResponse = {
  id: string;
  ok: boolean;
  result?: Record<string, unknown>;
  error?: { type: string; message: string };
};

async function exchange(
  lines: string[],
  env: NodeJS.ProcessEnv = {},
): Promise<JsonlResponse[]> {
  const child = spawn(process.execPath, [ENTRY], {
    cwd: PKG_ROOT,
    env: { ...process.env, ...env },
    stdio: ["pipe", "pipe", "pipe"],
  });

  let stdout = "";
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk: string) => {
    stdout += chunk;
  });

  for (const line of lines) {
    child.stdin.write(line.endsWith("\n") ? line : `${line}\n`);
  }
  child.stdin.end();

  const code: number | null = await new Promise((resolve) => {
    child.on("close", (c) => resolve(c));
  });
  assert.equal(code, 0, `runner exited ${code}; stdout=${stdout}`);

  return stdout
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean)
    .map((l) => JSON.parse(l) as JsonlResponse);
}

describe("JSONL runner process (create/turn/cancel/close)", () => {
  it("create fails closed with typed credential error when API key missing from env", async () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-run-"));
    temps.push(cwd);
    const responses = await exchange(
      [
        JSON.stringify({
          id: "c1",
          method: "create",
          params: {
            model: "composer-2.5",
            cwd,
            mcpServers: {},
          },
        }),
      ],
      { CURSOR_API_KEY: "" },
    );
    assert.equal(responses.length, 1);
    assert.equal(responses[0].ok, false);
    assert.equal(responses[0].error?.type, "credential");
    const blob = JSON.stringify(responses[0]);
    assert.doesNotMatch(blob, /CURSOR_API_KEY\s*[:=]/);
    assert.doesNotMatch(blob, /sk-[a-zA-Z0-9]/);
  });

  it("create fails with cwd error when cwd is missing/unusable", async () => {
    const responses = await exchange(
      [
        JSON.stringify({
          id: "c2",
          method: "create",
          params: {
            model: "composer-2.5",
            cwd: join(tmpdir(), `car-missing-cwd-${Date.now()}`),
            mcpServers: {},
          },
        }),
      ],
      { CURSOR_API_KEY: "test-key-not-for-live" },
    );
    assert.equal(responses[0].ok, false);
    assert.equal(responses[0].error?.type, "cwd");
  });

  it("create rejects empty model as sdk_config", async () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-run-"));
    temps.push(cwd);
    const responses = await exchange(
      [
        JSON.stringify({
          id: "c3",
          method: "create",
          params: { model: "", cwd, mcpServers: {} },
        }),
      ],
      { CURSOR_API_KEY: "test-key-not-for-live" },
    );
    assert.equal(responses[0].ok, false);
    assert.equal(responses[0].error?.type, "sdk_config");
  });

  it("create accepts mcpServers field (placeholder ok for t1)", async () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-run-"));
    temps.push(cwd);
    const responses = await exchange(
      [
        JSON.stringify({
          id: "c4",
          method: "create",
          params: {
            model: "composer-2.5",
            cwd,
            mcpServers: {
              workbench: { type: "http", url: "http://127.0.0.1:9876/mcp" },
            },
          },
        }),
      ],
      { CURSOR_API_KEY: "test-key-not-for-live" },
    );
    assert.ok(responses[0]);
    assert.notEqual(responses[0].error?.type, "runner");
    if (responses[0].ok) {
      assert.ok(responses[0].result?.agentId);
    } else {
      assert.ok(
        ["credential", "sdk_config", "sdk_run", "mcp_unavailable"].includes(
          responses[0].error?.type ?? "",
        ),
      );
    }
  });
});
