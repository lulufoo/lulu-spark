import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  ERROR_TYPES,
  parseRequest,
  serializeErrorResponse,
  serializeOkResponse,
  type RunnerErrorType,
} from "../src/protocol.ts";

describe("JSONL protocol schema", () => {
  it("parses create/turn/cancel/close with request id", () => {
    const create = parseRequest(
      JSON.stringify({
        id: "r1",
        method: "create",
        params: {
          model: "composer-2.5",
          cwd: "/tmp/session",
          mcpServers: {},
        },
      }),
    );
    assert.equal(create.id, "r1");
    assert.equal(create.method, "create");
    if (create.method !== "create") throw new Error("expected create");
    assert.equal(create.params.model, "composer-2.5");
    assert.equal(create.params.cwd, "/tmp/session");
    assert.deepEqual(create.params.mcpServers, {});

    const turn = parseRequest(
      JSON.stringify({ id: "r2", method: "turn", params: { prompt: "hi" } }),
    );
    assert.equal(turn.method, "turn");

    const cancel = parseRequest(JSON.stringify({ id: "r3", method: "cancel" }));
    assert.equal(cancel.method, "cancel");

    const close = parseRequest(JSON.stringify({ id: "r4", method: "close" }));
    assert.equal(close.method, "close");
  });

  it("accepts mcpServers on create without deep validation (t2 owns readiness)", () => {
    const req = parseRequest(
      JSON.stringify({
        id: "m1",
        method: "create",
        params: {
          model: "composer-2.5",
          cwd: "/tmp/x",
          mcpServers: {
            workbench: { type: "http", url: "http://127.0.0.1:9876/mcp" },
          },
        },
      }),
    );
    assert.equal(req.method, "create");
    if (req.method !== "create") throw new Error("expected create");
    assert.equal(
      (req.params.mcpServers as Record<string, { url: string }>).workbench.url,
      "http://127.0.0.1:9876/mcp",
    );
  });

  it("exposes typed error schema covering required failure classes", () => {
    const required: RunnerErrorType[] = [
      "credential",
      "sdk_config",
      "mcp_unavailable",
      "cwd",
      "runner",
      "sdk_run",
      "cancelled",
    ];
    for (const t of required) {
      assert.ok(ERROR_TYPES.includes(t), `missing error type ${t}`);
    }
  });

  it("never serializes apiKey or CURSOR_API_KEY into JSONL responses", () => {
    const ok = serializeOkResponse("1", { agentId: "agent-x" });
    const err = serializeErrorResponse("2", {
      type: "credential",
      message: "missing API key",
    });
    for (const line of [ok, err]) {
      assert.doesNotMatch(line, /apiKey/i);
      assert.doesNotMatch(line, /CURSOR_API_KEY/);
      assert.doesNotMatch(line, /sk-/);
    }
  });

  it("rejects unknown methods as typed runner protocol errors", () => {
    assert.throws(
      () => parseRequest(JSON.stringify({ id: "x", method: "nope" })),
      (e: unknown) => {
        const err = e as { type?: string };
        return err?.type === "runner";
      },
    );
  });
});
