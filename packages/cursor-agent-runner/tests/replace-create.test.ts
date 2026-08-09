import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, it } from "node:test";
import {
  agentCardinalityForTests,
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
  sessionId: string;
  cancelled: boolean;
  disposed: boolean;
  disposeFail?: boolean;
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
    const sessionId = String(params.sessionId ?? "");
    const agent: FakeAgent = {
      agentId: `agent-${sessionId}`,
      sessionId,
      cancelled: false,
      disposed: false,
      disposeFail: opts?.disposeFailFor?.has(sessionId) ?? false,
      async send(_prompt: string) {
        return {
          wait: async () => ({ status: "finished", result: "ok" }),
          cancel: async () => {
            agent.cancelled = true;
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

function createLine(id: string, sessionId: string, cwd: string): string {
  return JSON.stringify({
    id,
    method: "create",
    params: {
      session_id: sessionId,
      model: "composer-2.5",
      cwd,
      mcpServers: {},
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

describe("replace-style create (T-ReplaceCreate)", () => {
  it("first create succeeds without replaced_session_id and exposes one agent", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));
    const cwd = tempCwd();
    const raw = await handleLine(createLine("c1", "sess_a", cwd));
    const res = JSON.parse(raw);
    assert.equal(res.ok, true);
    assert.equal(res.result.agentId, "agent-sess_a");
    assert.equal(res.result.replaced_session_id, undefined);
    assert.equal(agentCardinalityForTests(), 1);
    assert.equal(created.length, 1);
  });

  it("second create cancels in-flight, awaits dispose, then creates; returns replaced_session_id", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));
    const cwdA = tempCwd();
    const cwdB = tempCwd();

    const first = JSON.parse(await handleLine(createLine("c1", "sess_a", cwdA)));
    assert.equal(first.ok, true);

    const second = JSON.parse(await handleLine(createLine("c2", "sess_b", cwdB)));
    assert.equal(second.ok, true);
    assert.equal(second.result.replaced_session_id, "sess_a");
    assert.equal(second.result.agentId, "agent-sess_b");
    assert.equal(agentCardinalityForTests(), 1);
    assert.equal(created.length, 2);
    assert.equal(created[0].disposed, true);
  });

  it("replace keeps methods create/turn/cancel/close and never needs close for switch", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));
    const a = tempCwd();
    const b = tempCwd();
    assert.equal(JSON.parse(await handleLine(createLine("1", "s1", a))).ok, true);
    assert.equal(JSON.parse(await handleLine(createLine("2", "s2", b))).ok, true);
    assert.equal(created[0].disposed, true);
    assert.equal(agentCardinalityForTests(), 1);
    const closed = JSON.parse(
      await handleLine(JSON.stringify({ id: "x", method: "close" })),
    );
    assert.equal(closed.ok, true);
    assert.equal(agentCardinalityForTests(), 0);
  });

  it("asyncDispose failure fails create, exposes no new agent, omits replaced_session_id", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));
    const a = tempCwd();
    const b = tempCwd();
    assert.equal(JSON.parse(await handleLine(createLine("1", "sess_old", a))).ok, true);
    created[0].disposeFail = true;
    const res = JSON.parse(await handleLine(createLine("2", "sess_new", b)));
    assert.equal(res.ok, false);
    assert.equal(res.error?.type, "runner");
    assert.equal(res.result?.replaced_session_id, undefined);
    assert.ok(agentCardinalityForTests() <= 1);
    assert.equal(
      created.filter((c) => c.sessionId === "sess_new" && !c.disposed).length,
      0,
    );
  });

  it("rapid switch coalesce-to-latest: skipped middle create never starts and is not Cancelled", async () => {
    const created: FakeAgent[] = [];
    let releaseFirstCreate: (() => void) | undefined;
    const firstCreateGate = new Promise<void>((resolve) => {
      releaseFirstCreate = resolve;
    });

    setAgentFactoryForTests(async (params) => {
      const sessionId = String(params.sessionId ?? "");
      if (sessionId === "sess_a") {
        await firstCreateGate;
      }
      return makeFactory({ created })(params);
    });

    const cwd = tempCwd();
    const pA = handleLine(createLine("a", "sess_a", cwd));
    await new Promise((r) => setTimeout(r, 10));
    const pB = handleLine(createLine("b", "sess_b", cwd));
    const pC = handleLine(createLine("c", "sess_c", cwd));
    releaseFirstCreate?.();

    const [a, b, c] = await Promise.all([pA, pB, pC]);
    const resA = JSON.parse(a);
    const resB = JSON.parse(b);
    const resC = JSON.parse(c);

    assert.equal(resA.ok, true);
    // B was coalesced away — never executed as an Agent; not Cancelled / not success.
    assert.equal(resB.ok, false);
    assert.equal(resB.error?.type, "coalesced");
    assert.notEqual(resB.error?.type, "cancelled");
    assert.equal(resC.ok, true);
    assert.equal(resC.result?.agentId, "agent-sess_c");
    assert.ok(
      !created.some((x) => x.sessionId === "sess_b"),
      "coalesced middle session must never Agent.create",
    );
    assert.equal(agentCardinalityForTests(), 1);
  });
});
