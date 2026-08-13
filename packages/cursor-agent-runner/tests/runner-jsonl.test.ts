import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import { parseRequest, ProtocolError } from "../src/protocol.ts";

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

function createParams(): Record<string, unknown> {
  return {
    business_id: "todo_task",
    session_id: "synthetic-prewarm",
    model: "composer-2.5",
    cwd: "/tmp",
    mcpServers: {},
  };
}

function assertRunnerParseError(value: Record<string, unknown>, field: string): void {
  assert.throws(
    () => parseRequest(JSON.stringify(value)),
    (error: unknown) => {
      assert.ok(error instanceof ProtocolError);
      assert.equal(error.type, "runner");
      assert.match(error.message, new RegExp(field));
      return true;
    },
  );
}

describe("JSONL runner process (create/turn/cancel/close)", () => {
  it("requires every create field and rejects session-only or camelCase routing", () => {
    const sessionOnly = createParams();
    delete sessionOnly.business_id;
    assertRunnerParseError(
      { id: "c-required", method: "create", params: sessionOnly },
      "business_id",
    );

    const camelCase = createParams();
    delete camelCase.business_id;
    camelCase.businessId = "todo_task";
    assertRunnerParseError(
      { id: "c-camel", method: "create", params: camelCase },
      "business_id",
    );

    for (const field of [
      "business_id",
      "session_id",
      "model",
      "cwd",
      "mcpServers",
    ]) {
      const missing = createParams();
      delete missing[field];
      assertRunnerParseError(
        { id: `c-missing-${field}`, method: "create", params: missing },
        field,
      );

      for (const invalid of [null, 42, "  "]) {
        const invalidParams = createParams();
        invalidParams[field] = invalid;
        assertRunnerParseError(
          { id: `c-invalid-${field}`, method: "create", params: invalidParams },
          field,
        );
      }
    }
  });

  it("requires business_id and prompt for turns", () => {
    for (const field of ["business_id", "prompt"]) {
      const missing: Record<string, unknown> = {
        business_id: "todo_task",
        prompt: "hello",
      };
      delete missing[field];
      assertRunnerParseError(
        { id: `t-missing-${field}`, method: "turn", params: missing },
        field,
      );

      for (const invalid of [null, 42, "  "]) {
        assertRunnerParseError(
          {
            id: `t-invalid-${field}`,
            method: "turn",
            params: { business_id: "todo_task", prompt: "hello", [field]: invalid },
          },
          field,
        );
      }
    }
  });

  it("requires business_id for cancel and close", () => {
    for (const method of ["cancel", "close"] as const) {
      for (const params of [
        {},
        { session_id: "ui-session-only" },
        { business_id: null },
        { business_id: 42 },
        { business_id: "  " },
      ]) {
        assertRunnerParseError(
          { id: `${method}-invalid`, method, params },
          "business_id",
        );
      }
    }
  });

  it("ignores optional session_id on business-routed turn, cancel, and close", () => {
    const turn = parseRequest(
      JSON.stringify({
        id: "t-optional-session",
        method: "turn",
        params: {
          business_id: " todo_task ",
          session_id: "must-not-route",
          prompt: "hello",
        },
      }),
    );
    assert.deepEqual(turn.params, {
      business_id: "todo_task",
      prompt: "hello",
    });

    for (const method of ["cancel", "close"] as const) {
      const request = parseRequest(
        JSON.stringify({
          id: `${method}-optional-session`,
          method,
          params: {
            business_id: " todo_task ",
            session_id: "must-not-route",
          },
        }),
      );
      assert.deepEqual(request.params, { business_id: "todo_task" });
    }
  });

  it("fails missing business_id during JSONL parsing before create dispatch", async () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-run-"));
    temps.push(cwd);
    const responses = await exchange(
      [
        JSON.stringify({
          id: "c-no-business",
          method: "create",
          params: {
            session_id: "session-only",
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
    assert.equal(responses[0].error?.type, "runner");
  });

  it("create fails closed with typed credential error when API key missing from env", async () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-run-"));
    temps.push(cwd);
    const responses = await exchange(
      [
        JSON.stringify({
          id: "c1",
          method: "create",
          params: {
            business_id: "todo_task",
            session_id: "sess_c1",
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
            business_id: "todo_task",
            session_id: "sess_c2",
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

  it("create rejects trim-empty model during JSONL parsing", async () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-run-"));
    temps.push(cwd);
    const responses = await exchange(
      [
        JSON.stringify({
          id: "c3",
          method: "create",
          params: {
            business_id: "todo_task",
            session_id: "sess_c3",
            model: "",
            cwd,
            mcpServers: {},
          },
        }),
      ],
      { CURSOR_API_KEY: "test-key-not-for-live" },
    );
    assert.equal(responses[0].ok, false);
    assert.equal(responses[0].error?.type, "runner");
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
            business_id: "todo_task",
            session_id: "sess_c4",
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
