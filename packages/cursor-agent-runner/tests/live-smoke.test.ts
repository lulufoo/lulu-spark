/**
 * T5 opt-in live smoke for official @cursor/sdk Local.
 *
 * Env-gated: requires CURSOR_API_KEY + Node≥22.13 + MCP ready.
 * CI without key MUST skip (not fake-green). Mock success must never mark
 * Cursor AC satisfied — that is recorded as 外部验收待完成 when skipped.
 */
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  evaluateLiveSmokeGate,
  runLiveSmokeCursorSdk,
  type LiveSmokeResult,
} from "../src/live-smoke.ts";

describe("opt-in live_smoke_cursor_sdk", () => {
  it("skips (not fake-green) when CURSOR_API_KEY is absent", async () => {
    const gate = evaluateLiveSmokeGate({
      apiKey: "",
      nodeVersion: process.versions.node,
      mcpReady: true,
    });
    assert.equal(gate.shouldRun, false);
    assert.equal(gate.verdict, "skip");
    assert.match(gate.reason, /CURSOR_API_KEY|credential/i);

    const result: LiveSmokeResult = await runLiveSmokeCursorSdk({
      apiKey: "",
      nodeVersion: process.versions.node,
      mcpReady: true,
      forceSkip: true,
    });
    assert.equal(result.status, "skip");
    assert.equal(result.cursorAcSatisfied, false);
    assert.equal(result.externalAcceptancePending, true);
    assert.match(result.notes, /外部验收待完成/);
  });

  it("does not treat mock/test-double success as Cursor AC satisfied", async () => {
    const result = await runLiveSmokeCursorSdk({
      apiKey: "",
      nodeVersion: "22.13.0",
      mcpReady: true,
      mockEvidenceOnly: true,
    });
    assert.equal(result.cursorAcSatisfied, false);
    assert.equal(result.externalAcceptancePending, true);
    assert.notEqual(result.status, "pass");
  });

  it("requires Node≥22.13 and MCP ready before opt-in run", () => {
    const noNode = evaluateLiveSmokeGate({
      apiKey: "sk-test",
      nodeVersion: "22.12.0",
      mcpReady: true,
    });
    assert.equal(noNode.shouldRun, false);
    assert.equal(noNode.verdict, "skip");

    const noMcp = evaluateLiveSmokeGate({
      apiKey: "sk-test",
      nodeVersion: "22.13.0",
      mcpReady: false,
    });
    assert.equal(noMcp.shouldRun, false);

    const ready = evaluateLiveSmokeGate({
      apiKey: "sk-live",
      nodeVersion: "22.13.0",
      mcpReady: true,
    });
    assert.equal(ready.shouldRun, true);
    assert.equal(ready.verdict, "run");
  });

  it("when env is ready, live smoke creates agent in non-git cwd and checks MCP visibility", async () => {
    const envKey = (process.env.CURSOR_API_KEY ?? "").trim();
    // Ignore short / probe stubs left in the shell (e.g. sk-test).
    const looksReal = envKey.length >= 40 && !/^sk-test/i.test(envKey);
    const result = await runLiveSmokeCursorSdk({
      apiKey: looksReal ? envKey : "",
      nodeVersion: process.versions.node,
      mcpReady: looksReal,
    });

    if (!looksReal) {
      assert.equal(result.status, "skip");
      assert.equal(result.cursorAcSatisfied, false);
      assert.equal(result.externalAcceptancePending, true);
      assert.match(result.notes, /外部验收待完成/);
      return;
    }

    assert.equal(result.status, "pass");
    assert.equal(result.cursorAcSatisfied, true);
    assert.equal(result.usedNonGitCwd, true);
    assert.equal(result.mcpServerVisible, true);
    assert.ok(result.agentCreated);
  });
});

