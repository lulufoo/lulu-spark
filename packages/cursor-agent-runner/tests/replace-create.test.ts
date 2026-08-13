import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, it } from "node:test";
import {
  agentCardinalityForTests,
  diagnosticsForTests,
  handleLine,
  resetRunnerStateForTests,
  setAgentFactoryForTests,
  type AgentFactory,
} from "../src/index.ts";

const temps: string[] = [];

function tempCwd(): string {
  const cwd = mkdtempSync(join(tmpdir(), "car-replace-"));
  temps.push(cwd);
  return cwd;
}

type FakeAgent = {
  agentId: string;
  businessId: string;
  disposed: boolean;
  disposeFail?: boolean;
  prompts: string[];
  send: (prompt: string) => Promise<{
    wait: () => Promise<{ status: string; result: string }>;
    cancel: () => Promise<void>;
  }>;
  [Symbol.asyncDispose]: () => Promise<void>;
};

function makeFactory(opts?: {
  disposeFailFor?: Set<string>;
  created?: FakeAgent[];
}): AgentFactory {
  const created = opts?.created ?? [];
  return async (params) => {
    const businessId = String(params.businessId ?? "");
    const agent: FakeAgent = {
      agentId: `agent-${businessId}`,
      businessId,
      disposed: false,
      disposeFail: opts?.disposeFailFor?.has(businessId) ?? false,
      prompts: [],
      async send(prompt: string) {
        agent.prompts.push(prompt);
        return {
          wait: async () => ({
            status: "finished",
            result: `${businessId}:${prompt}`,
          }),
          cancel: async () => {
            // The slot owns cancellation; this fake only needs a stable Run port.
          },
        };
      },
      async [Symbol.asyncDispose]() {
        if (agent.disposeFail) {
          throw new Error("asyncDispose failed");
        }
        agent.disposed = true;
      },
    };
    created.push(agent);
    return agent;
  };
}

function createLine(
  id: string,
  businessId: string,
  cwd: string,
  sessionId = `session-${id}`,
): string {
  return JSON.stringify({
    id,
    method: "create",
    params: {
      business_id: businessId,
      session_id: sessionId,
      model: "composer-2.5",
      cwd,
      mcpServers: {},
    },
  });
}

function closeLine(id: string, businessId: string, sessionId?: string): string {
  return JSON.stringify({
    id,
    method: "close",
    params: {
      business_id: businessId,
      ...(sessionId ? { session_id: sessionId } : {}),
    },
  });
}

beforeEach(() => {
  resetRunnerStateForTests();
  process.env.CURSOR_API_KEY = "test-key-not-for-live";
});

afterEach(() => {
  resetRunnerStateForTests();
  setAgentFactoryForTests(null);
  while (temps.length) {
    const p = temps.pop();
    if (p) rmSync(p, { recursive: true, force: true });
  }
});

describe("business-scoped create lifecycle (T-ReplaceCreate)", () => {
  it("creates one business slot and treats a matching create as an idempotent hit", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));
    const cwd = tempCwd();

    const first = JSON.parse(
      await handleLine(createLine("c1", "todos", cwd, "sess_a")),
    );
    const second = JSON.parse(
      await handleLine(createLine("c2", "todos", cwd, "sess_b")),
    );

    assert.equal(first.ok, true);
    assert.equal(second.ok, true);
    assert.equal(first.result.agentId, "agent-todos");
    assert.equal(second.result.agentId, "agent-todos");
    assert.equal(first.result.replaced_session_id, undefined);
    assert.equal(second.result.replaced_session_id, undefined);
    assert.equal(agentCardinalityForTests(), 1);
    assert.equal(created.length, 1);
    assert.equal(created[0].disposed, false);
  });

  it("keeps business slots independent and closes only the requested business", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));
    const todosCwd = tempCwd();
    const notesCwd = tempCwd();

    assert.equal(
      JSON.parse(await handleLine(createLine("a", "todos", todosCwd))).ok,
      true,
    );
    assert.equal(
      JSON.parse(await handleLine(createLine("b", "notes", notesCwd))).ok,
      true,
    );
    assert.equal(agentCardinalityForTests(), 2);

    const todosTurn = JSON.parse(
      await handleLine(
        JSON.stringify({
          id: "turn-todos",
          method: "turn",
          params: {
            business_id: "todos",
            session_id: "ui-todos",
            prompt: "hello",
          },
        }),
      ),
    );
    assert.equal(todosTurn.ok, true);
    assert.equal(todosTurn.result.text, "todos:hello");

    const closed = JSON.parse(
      await handleLine(closeLine("close-todos", "todos", "ui-todos")),
    );
    assert.equal(closed.ok, true);
    assert.equal(created.find((agent) => agent.businessId === "todos")?.disposed, true);
    assert.equal(created.find((agent) => agent.businessId === "notes")?.disposed, false);
    assert.equal(agentCardinalityForTests(), 1);

    const notesTurn = JSON.parse(
      await handleLine(
        JSON.stringify({
          id: "turn-notes",
          method: "turn",
          params: { business_id: "notes", prompt: "still-live" },
        }),
      ),
    );
    assert.equal(notesTurn.ok, true);
    assert.equal(notesTurn.result.text, "notes:still-live");
  });

  it("reports dispose failure through handleLine and permits a new slot create", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(
      makeFactory({ created, disposeFailFor: new Set(["todos"]) }),
    );
    const cwd = tempCwd();

    assert.equal(
      JSON.parse(await handleLine(createLine("c1", "todos", cwd))).ok,
      true,
    );
    const closeResult = JSON.parse(
      await handleLine(closeLine("close", "todos")),
    );
    assert.equal(closeResult.ok, false);
    assert.equal(closeResult.error?.type, "sdk_run");
    assert.equal(created[0].disposed, false);
    assert.equal(agentCardinalityForTests(), 0);

    setAgentFactoryForTests(makeFactory({ created }));
    const retried = JSON.parse(
      await handleLine(createLine("c2", "todos", cwd, "retry-session")),
    );
    assert.equal(retried.ok, true);
    assert.equal(retried.result.agentId, "agent-todos");
    assert.equal(created.length, 2);
    assert.equal(agentCardinalityForTests(), 1);
  });

  it("emits a typed create failure diagnostic with SDK error text", async () => {
    setAgentFactoryForTests(async () => {
      throw new Error("create rejected: model not available");
    });
    const cwd = tempCwd();

    const result = JSON.parse(
      await handleLine(createLine("failed-create", "todos", cwd)),
    );
    assert.equal(result.ok, false);

    const failure = diagnosticsForTests().find(
      (event) => event.event === "prewarm_create_fail",
    );
    assert.equal(failure?.error_type, "sdk_run");
    assert.equal(failure?.sdk_error_message, "create rejected: model not available");
    assert.match(String(failure?.sdk_error_json), /model not available/);
  });

  it("emits turn_empty when wait() finishes without result text", async () => {
    setAgentFactoryForTests(async () => ({
      agentId: "empty-agent",
      async send() {
        return {
          wait: async () => ({ status: "finished" }),
          cancel: async () => {},
        };
      },
      async [Symbol.asyncDispose]() {},
    }));
    const cwd = tempCwd();
    assert.equal(
      JSON.parse(await handleLine(createLine("c-empty", "todos", cwd))).ok,
      true,
    );

    const turned = JSON.parse(
      await handleLine(
        JSON.stringify({
          id: "t-empty",
          method: "turn",
          params: { business_id: "todos", prompt: "SECRET-PROMPT-TEXT" },
        }),
      ),
    );
    assert.equal(turned.ok, true);
    assert.equal(turned.result.text, "");

    const empty = diagnosticsForTests().find(
      (event) => event.event === "turn_empty",
    );
    assert.equal(empty?.sdk_status, "finished");
    assert.equal(empty?.has_result, false);
    assert.equal(empty?.text_len, 0);
    assert.equal(empty?.has_error, false);
    assert.match(String(empty?.sdk_wait_keys), /status/);
    assert.doesNotMatch(
      JSON.stringify(diagnosticsForTests()),
      /SECRET-PROMPT-TEXT/,
    );
  });

  it("emits turn_sdk_error with SDK code and message when wait() status is error", async () => {
    setAgentFactoryForTests(async () => ({
      agentId: "error-agent",
      async send() {
        return {
          wait: async () => ({
            status: "error",
            durationMs: 14600,
            model: { id: "composer-2" },
            error: {
              message: "quota exceeded for composer-2",
              code: "rate_limit",
            },
            result: "SECRET-REPLY-BODY",
          }),
          cancel: async () => {},
        };
      },
      async [Symbol.asyncDispose]() {},
    }));
    const cwd = tempCwd();
    assert.equal(
      JSON.parse(await handleLine(createLine("c-err", "todos", cwd))).ok,
      true,
    );

    const turned = JSON.parse(
      await handleLine(
        JSON.stringify({
          id: "t-err",
          method: "turn",
          params: { business_id: "todos", prompt: "hello" },
        }),
      ),
    );
    assert.equal(turned.ok, true);
    assert.equal(turned.result.status, "error");
    assert.equal(turned.result.sdk_error_code, "rate_limit");
    assert.equal(
      turned.result.sdk_error_message,
      "quota exceeded for composer-2",
    );

    const failure = diagnosticsForTests().find(
      (event) => event.event === "turn_sdk_error",
    );
    assert.equal(failure?.error_type, "sdk_run");
    assert.equal(failure?.sdk_status, "error");
    assert.equal(failure?.has_result, true);
    assert.equal(failure?.has_error, true);
    assert.equal(failure?.sdk_error_code, "rate_limit");
    assert.equal(failure?.sdk_error_message, "quota exceeded for composer-2");
    assert.equal(failure?.sdk_model, "composer-2");
    assert.equal(failure?.sdk_duration_ms, 14600);
    assert.match(String(failure?.sdk_wait_keys), /error/);
    assert.match(String(failure?.sdk_error_keys), /code/);
    assert.match(String(failure?.sdk_wait_json), /rate_limit/);
    assert.doesNotMatch(
      JSON.stringify(diagnosticsForTests()),
      /SECRET-REPLY-BODY/,
    );
  });

  it("redacts API key material in SDK error diagnostics", async () => {
    setAgentFactoryForTests(async () => ({
      agentId: "secret-agent",
      async send() {
        return {
          wait: async () => ({
            status: "error",
            error: {
              message: "auth failed CURSOR_API_KEY=sk-secretvalue Bearer abc.def",
              code: "unauthenticated",
            },
          }),
          cancel: async () => {},
        };
      },
      async [Symbol.asyncDispose]() {},
    }));
    const cwd = tempCwd();
    assert.equal(
      JSON.parse(await handleLine(createLine("c-secret", "todos", cwd))).ok,
      true,
    );
    await handleLine(
      JSON.stringify({
        id: "t-secret",
        method: "turn",
        params: { business_id: "todos", prompt: "hello" },
      }),
    );
    const blob = JSON.stringify(diagnosticsForTests());
    assert.match(blob, /unauthenticated/);
    assert.doesNotMatch(blob, /sk-secretvalue/);
    assert.doesNotMatch(blob, /CURSOR_API_KEY=sk-/);
    assert.doesNotMatch(blob, /Bearer abc\.def/);
  });

  it("emits turn_ok with text_len and without reply body", async () => {
    setAgentFactoryForTests(makeFactory());
    const cwd = tempCwd();
    assert.equal(
      JSON.parse(await handleLine(createLine("c-ok", "todos", cwd))).ok,
      true,
    );

    const turned = JSON.parse(
      await handleLine(
        JSON.stringify({
          id: "t-ok",
          method: "turn",
          params: { business_id: "todos", prompt: "hello" },
        }),
      ),
    );
    assert.equal(turned.ok, true);
    assert.equal(turned.result.text, "todos:hello");

    const ok = diagnosticsForTests().find((event) => event.event === "turn_ok");
    assert.equal(ok?.sdk_status, "finished");
    assert.equal(ok?.has_result, true);
    assert.equal(ok?.text_len, "todos:hello".length);
    assert.doesNotMatch(JSON.stringify(diagnosticsForTests()), /todos:hello/);
  });

  it("shares concurrent same-business creates instead of creating multiple agents", async () => {
    const created: FakeAgent[] = [];
    let releaseCreate!: () => void;
    let markCreateStarted!: () => void;
    const createStarted = new Promise<void>((resolve) => {
      markCreateStarted = resolve;
    });
    const createGate = new Promise<void>((resolve) => {
      releaseCreate = resolve;
    });

    setAgentFactoryForTests(async (params) => {
      markCreateStarted();
      await createGate;
      return makeFactory({ created })(params);
    });

    const cwd = tempCwd();
    const first = handleLine(createLine("a", "todos", cwd, "sess_a"));
    await createStarted;
    const second = handleLine(createLine("b", "todos", cwd, "sess_b"));
    const third = handleLine(createLine("c", "todos", cwd, "sess_c"));
    releaseCreate();

    const [firstRaw, secondRaw, thirdRaw] = await Promise.all([
      first,
      second,
      third,
    ]);
    const firstResult = JSON.parse(firstRaw);
    const secondResult = JSON.parse(secondRaw);
    const thirdResult = JSON.parse(thirdRaw);

    assert.equal(firstResult.ok, true);
    assert.equal(secondResult.ok, true);
    assert.equal(thirdResult.ok, true);
    assert.equal(firstResult.result.agentId, "agent-todos");
    assert.equal(secondResult.result.agentId, "agent-todos");
    assert.equal(thirdResult.result.agentId, "agent-todos");
    assert.equal(created.length, 1);
    assert.equal(agentCardinalityForTests(), 1);
  });
});
