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
  shutdownRunnerForTests,
  type AgentFactory,
} from "../src/index.ts";

type Deferred<T> = {
  promise: Promise<T>;
  resolve: (value: T) => void;
};

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => {
    resolve = next;
  });
  return { promise, resolve };
}

type FakeRun = {
  wait: () => Promise<{ status: string; result: string }>;
  cancel: () => Promise<void>;
};

type FakeAgent = {
  agentId: string;
  businessId: string;
  disposed: boolean;
  cancelledRuns: number;
  prompts: string[];
  maxConcurrent: number;
  send: (prompt: string) => Promise<FakeRun>;
  [Symbol.asyncDispose]: () => Promise<void>;
};

type FactoryOptions = {
  created?: FakeAgent[];
  createGates?: Map<string, Deferred<void>>;
  failCreates?: Map<string, number>;
  waitGates?: Map<string, Deferred<void>>;
  sendStarted?: Map<string, Deferred<void>>;
  waitStarted?: Map<string, Deferred<void>>;
  failPrompts?: Set<string>;
  cancelResolvesWait?: boolean;
};

function makeFactory(options: FactoryOptions = {}): AgentFactory {
  const created = options.created ?? [];
  const activeRuns = new Map<string, number>();

  return async (params) => {
    const businessId = String(params.businessId ?? "");
    const createGate = options.createGates?.get(businessId);
    if (createGate) await createGate.promise;

    const remainingFailures = options.failCreates?.get(businessId) ?? 0;
    if (remainingFailures > 0) {
      options.failCreates?.set(businessId, remainingFailures - 1);
      throw new Error(`create failed for ${businessId}`);
    }

    const agent: FakeAgent = {
      agentId: `agent-${businessId}`,
      businessId,
      disposed: false,
      cancelledRuns: 0,
      prompts: [],
      maxConcurrent: 0,
      async send(prompt) {
        options.sendStarted?.get(prompt)?.resolve(undefined);
        if (options.failPrompts?.has(prompt)) {
          throw new Error(`turn failed for ${businessId}`);
        }

        agent.prompts.push(prompt);
        const active = (activeRuns.get(businessId) ?? 0) + 1;
        activeRuns.set(businessId, active);
        agent.maxConcurrent = Math.max(agent.maxConcurrent, active);

        let cancelled = false;
        const waitGate = options.waitGates?.get(prompt);
        const wait = async () => {
          options.waitStarted?.get(prompt)?.resolve(undefined);
          if (waitGate) await waitGate.promise;
          activeRuns.set(businessId, Math.max(0, (activeRuns.get(businessId) ?? 1) - 1));
          return {
            status: cancelled ? "cancelled" : "finished",
            result: cancelled ? "" : `${businessId}:${prompt}`,
          };
        };

        return {
          wait,
          async cancel() {
            cancelled = true;
            agent.cancelledRuns += 1;
            if (options.cancelResolvesWait !== false) {
              waitGate?.resolve(undefined);
            }
          },
        };
      },
      async [Symbol.asyncDispose]() {
        agent.disposed = true;
      },
    };
    created.push(agent);
    return agent;
  };
}

const temps: string[] = [];

function tempCwd(): string {
  const cwd = mkdtempSync(join(tmpdir(), "car-slot-map-"));
  temps.push(cwd);
  return cwd;
}

function createLine(
  id: string,
  businessId: string,
  cwd: string,
  sessionId = `session-${id}`,
  extra: Record<string, unknown> = {},
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
      ...extra,
    },
  });
}

function turnLine(id: string, businessId: string, prompt: string): string {
  return JSON.stringify({
    id,
    method: "turn",
    params: { business_id: businessId, prompt },
  });
}

function cancelLine(id: string, businessId: string): string {
  return JSON.stringify({
    id,
    method: "cancel",
    params: { business_id: businessId },
  });
}

function closeLine(id: string, businessId: string): string {
  return JSON.stringify({
    id,
    method: "close",
    params: { business_id: businessId },
  });
}

function response(raw: string): Record<string, any> {
  return JSON.parse(raw) as Record<string, any>;
}

function diagnostics(): Array<{
  event: string;
  business_id: string;
  error_type?: string;
}> {
  return diagnosticsForTests() as Array<{
    event: string;
    business_id: string;
    error_type?: string;
  }>;
}

beforeEach(() => {
  process.env.CURSOR_API_KEY = "test-key-not-for-live";
  resetRunnerStateForTests();
});

afterEach(async () => {
  await shutdownRunnerForTests();
  resetRunnerStateForTests();
  setAgentFactoryForTests(null);
  while (temps.length) {
    const path = temps.pop();
    if (path) rmSync(path, { recursive: true, force: true });
  }
});

describe("businessId Agent slot map", () => {
  it("creates one ready slot and treats a matching create as an idempotent hit", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));
    const cwd = tempCwd();

    const first = response(await handleLine(createLine("c1", "todo_task", cwd, "ui-a")));
    const second = response(
      await handleLine(createLine("c2", "todo_task", cwd, "ui-b")),
    );

    assert.equal(first.ok, true);
    assert.equal(second.ok, true);
    assert.equal(first.result.agentId, "agent-todo_task");
    assert.equal(second.result.agentId, "agent-todo_task");
    assert.equal(created.length, 1);
    assert.equal(created[0].disposed, false);
    assert.equal(agentCardinalityForTests(), 1);
    assert.ok(
      diagnostics().some(
        (event) => event.event === "create_hit" && event.business_id === "todo_task",
      ),
    );
  });

  it("keeps business slots independent for routing and close", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));
    const cwd = tempCwd();

    assert.equal(response(await handleLine(createLine("a", "todos", cwd))).ok, true);
    assert.equal(response(await handleLine(createLine("b", "notes", cwd))).ok, true);
    assert.equal(agentCardinalityForTests(), 2);

    const todosTurn = response(
      await handleLine(turnLine("ta", "todos", "same-session-as-notes")),
    );
    const notesTurn = response(await handleLine(turnLine("tb", "notes", "hello")));
    assert.equal(todosTurn.ok, true);
    assert.equal(todosTurn.result.text, "todos:same-session-as-notes");
    assert.equal(notesTurn.ok, true);
    assert.equal(notesTurn.result.text, "notes:hello");

    const closed = response(await handleLine(closeLine("close-a", "todos")));
    assert.equal(closed.ok, true);
    assert.equal(created.find((agent) => agent.businessId === "todos")?.disposed, true);
    assert.equal(created.find((agent) => agent.businessId === "notes")?.disposed, false);
    assert.equal(agentCardinalityForTests(), 1);
    assert.equal(response(await handleLine(turnLine("tb2", "notes", "still-live"))).ok, true);
  });

  it("rejects a ready-slot profile mismatch without disposing or replacing it", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));
    const cwd = tempCwd();
    const otherCwd = tempCwd();

    assert.equal(response(await handleLine(createLine("c1", "todos", cwd))).ok, true);
    const mismatch = response(
      await handleLine(
        createLine("c2", "todos", otherCwd, "different-session", {
          model: "different-model",
        }),
      ),
    );

    assert.equal(mismatch.ok, false);
    assert.equal(mismatch.error.type, "runner");
    assert.match(mismatch.error.message, /profile|match|ready/i);
    assert.equal(created.length, 1);
    assert.equal(created[0].disposed, false);
    assert.equal(agentCardinalityForTests(), 1);
    assert.ok(
      diagnostics().some(
        (event) =>
          event.event === "create_fail" &&
          event.business_id === "todos" &&
          event.error_type === "runner",
      ),
    );
  });

  it("shares one creating promise with concurrent create and turn waiters", async () => {
    const created: FakeAgent[] = [];
    const createGate = deferred<void>();
    const createStarted = deferred<void>();
    const createGates = new Map([["todos", createGate]]);
    setAgentFactoryForTests(async (params) => {
      createStarted.resolve(undefined);
      return makeFactory({ created, createGates })(params);
    });
    const cwd = tempCwd();

    const firstCreate = handleLine(createLine("c1", "todos", cwd, "session-a"));
    await createStarted.promise;
    const secondCreate = handleLine(createLine("c2", "todos", cwd, "session-b"));
    const waitingTurn = handleLine(turnLine("t1", "todos", "after-create"));
    await new Promise((resolve) => setTimeout(resolve, 5));
    createGate.resolve(undefined);

    const [first, second, turn] = await Promise.all([
      firstCreate,
      secondCreate,
      waitingTurn,
    ]);
    assert.equal(response(first).ok, true);
    assert.equal(response(second).ok, true);
    assert.equal(response(turn).ok, true);
    assert.equal(created.length, 1);
    assert.deepEqual(created[0].prompts, ["after-create"]);
    assert.equal(agentCardinalityForTests(), 1);
    assert.ok(
      diagnostics().some(
        (event) =>
          event.event === "prewarm_in_progress" && event.business_id === "todos",
      ),
    );
  });

  it("serializes turns within one business slot", async () => {
    const created: FakeAgent[] = [];
    const firstWait = deferred<void>();
    const waitGates = new Map([["first", firstWait]]);
    const firstStarted = deferred<void>();
    const sendStarted = new Map([["first", firstStarted]]);
    setAgentFactoryForTests(
      makeFactory({ created, waitGates, sendStarted } as FactoryOptions),
    );
    const cwd = tempCwd();
    assert.equal(response(await handleLine(createLine("c1", "todos", cwd))).ok, true);

    const firstTurn = handleLine(turnLine("t1", "todos", "first"));
    await firstStarted.promise;
    let secondSettled = false;
    const secondTurn = handleLine(turnLine("t2", "todos", "second")).then((raw) => {
      secondSettled = true;
      return raw;
    });
    await new Promise((resolve) => setTimeout(resolve, 5));
    assert.equal(secondSettled, false);

    firstWait.resolve(undefined);
    const [first, second] = await Promise.all([firstTurn, secondTurn]);
    assert.equal(response(first).ok, true);
    assert.equal(response(second).ok, true);
    assert.deepEqual(created[0].prompts, ["first", "second"]);
    assert.equal(created[0].maxConcurrent, 1);
  });

  it("fails an empty-slot turn without implicitly creating an agent", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));

    const result = response(await handleLine(turnLine("t1", "missing", "no-create")));
    assert.equal(result.ok, false);
    assert.equal(result.error.type, "runner");
    assert.match(result.error.message, /empty|create|agent/i);
    assert.equal(created.length, 0);
    assert.equal(agentCardinalityForTests(), 0);
    assert.ok(
      diagnostics().some(
        (event) =>
          event.event === "turn_fail" &&
          event.business_id === "missing" &&
          event.error_type === "runner",
      ),
    );
  });

  it("returns a failed create to its waiters, resets the slot, and permits retry", async () => {
    const created: FakeAgent[] = [];
    const failCreates = new Map([["todos", 1]]);
    setAgentFactoryForTests(makeFactory({ created, failCreates }));
    const cwd = tempCwd();

    const failed = response(await handleLine(createLine("c1", "todos", cwd)));
    const retried = response(await handleLine(createLine("c2", "todos", cwd)));

    assert.equal(failed.ok, false);
    assert.equal(retried.ok, true);
    assert.equal(created.length, 1);
    assert.equal(agentCardinalityForTests(), 1);
    assert.ok(
      diagnostics().some(
        (event) => event.event === "create_fail" && event.business_id === "todos",
      ),
    );
  });

  it("cancels only the selected business turn and keeps its agent alive", async () => {
    const created: FakeAgent[] = [];
    const slowWait = deferred<void>();
    const sendStarted = deferred<void>();
    setAgentFactoryForTests(
      makeFactory({
        created,
        waitGates: new Map([["slow", slowWait]]),
        sendStarted: new Map([["slow", sendStarted]]),
      } as FactoryOptions),
    );
    const cwd = tempCwd();
    assert.equal(response(await handleLine(createLine("a", "todos", cwd))).ok, true);
    assert.equal(response(await handleLine(createLine("b", "notes", cwd))).ok, true);

    const turn = handleLine(turnLine("t1", "todos", "slow"));
    await sendStarted.promise;
    const cancelled = response(await handleLine(cancelLine("x", "todos")));
    const turnResult = response(await turn);

    assert.equal(cancelled.ok, true);
    assert.equal(cancelled.result.cancelled, true);
    assert.equal(turnResult.ok, false);
    assert.equal(created.find((agent) => agent.businessId === "todos")?.disposed, false);
    assert.equal(created.find((agent) => agent.businessId === "notes")?.disposed, false);
    assert.equal(created.find((agent) => agent.businessId === "todos")?.cancelledRuns, 1);
    assert.ok(
      diagnostics().some(
        (event) => event.event === "cancel" && event.business_id === "todos",
      ),
    );
    slowWait.resolve(undefined);
  });

  it("attributes turn failures to the business slot", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(
      makeFactory({ created, failPrompts: new Set(["boom"]) }),
    );
    const cwd = tempCwd();
    assert.equal(response(await handleLine(createLine("c1", "todos", cwd))).ok, true);

    const failed = response(await handleLine(turnLine("t1", "todos", "boom")));
    assert.equal(failed.ok, false);
    assert.ok(
      diagnostics().some(
        (event) =>
          event.event === "turn_fail" &&
          event.business_id === "todos" &&
          event.sdk_error_message === "turn failed for todos",
      ),
    );
  });

  it("disposes every business slot during runner shutdown", async () => {
    const created: FakeAgent[] = [];
    setAgentFactoryForTests(makeFactory({ created }));
    const cwd = tempCwd();
    assert.equal(response(await handleLine(createLine("a", "todos", cwd))).ok, true);
    assert.equal(response(await handleLine(createLine("b", "notes", cwd))).ok, true);
    assert.equal(agentCardinalityForTests(), 2);

    await shutdownRunnerForTests();

    assert.equal(created.every((agent) => agent.disposed), true);
    assert.equal(agentCardinalityForTests(), 0);
  });

  it("waits for an active turn to finish before closing its Agent slot", async () => {
    const created: FakeAgent[] = [];
    const slowWait = deferred<void>();
    const waitStarted = deferred<void>();
    setAgentFactoryForTests(
      makeFactory({
        created,
        waitGates: new Map([["slow", slowWait]]),
        waitStarted: new Map([["slow", waitStarted]]),
        cancelResolvesWait: false,
      }),
    );
    const cwd = tempCwd();
    assert.equal(response(await handleLine(createLine("c", "todos", cwd))).ok, true);

    const turn = handleLine(turnLine("t", "todos", "slow"));
    await waitStarted.promise;
    const close = handleLine(closeLine("close", "todos"));
    await new Promise((resolve) => setTimeout(resolve, 0));

    assert.equal(created[0].disposed, false);
    slowWait.resolve(undefined);
    const [turnResult, closeResult] = await Promise.all([turn, close]);
    assert.equal(response(turnResult).ok, false);
    assert.equal(response(closeResult).ok, true);
    assert.equal(created[0].disposed, true);
  });

  it("waits for an active turn to finish before runner shutdown disposes slots", async () => {
    const created: FakeAgent[] = [];
    const slowWait = deferred<void>();
    const waitStarted = deferred<void>();
    setAgentFactoryForTests(
      makeFactory({
        created,
        waitGates: new Map([["slow", slowWait]]),
        waitStarted: new Map([["slow", waitStarted]]),
        cancelResolvesWait: false,
      }),
    );
    const cwd = tempCwd();
    assert.equal(response(await handleLine(createLine("c", "todos", cwd))).ok, true);

    const turn = handleLine(turnLine("t", "todos", "slow"));
    await waitStarted.promise;
    const shutdown = shutdownRunnerForTests();
    await new Promise((resolve) => setTimeout(resolve, 0));

    assert.equal(created[0].disposed, false);
    slowWait.resolve(undefined);
    const [turnResult] = await Promise.all([turn, shutdown]);
    assert.equal(response(turnResult).ok, false);
    assert.equal(created[0].disposed, true);
    assert.equal(agentCardinalityForTests(), 0);
  });
});
