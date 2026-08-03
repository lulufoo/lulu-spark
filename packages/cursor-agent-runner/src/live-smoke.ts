/**
 * Opt-in live smoke for official @cursor/sdk Local (T5).
 *
 * Docs: https://cursor.com/docs/sdk/typescript
 * Gate: CURSOR_API_KEY + Node≥22.13 + MCP ready → run;
 * otherwise skip (never fake-green). Mock evidence never satisfies Cursor AC.
 *
 * SDK sandbox is disabled (same as production runner) so Workbench MCP tools
 * are not blocked by headless interactive-approval.
 */
import { mkdtempSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Agent } from "@cursor/sdk";
import {
  ensureNodeAndSandbox,
  isNodeVersionAtLeast,
} from "./sandbox.ts";

export type LiveSmokeGateInput = {
  apiKey: string;
  nodeVersion: string;
  /** @deprecated Ignored — production path keeps SDK sandbox off for MCP. */
  sandboxAvailable?: boolean;
  mcpReady: boolean;
};

export type LiveSmokeGateResult = {
  shouldRun: boolean;
  verdict: "run" | "skip";
  reason: string;
};

export type LiveSmokeRunInput = LiveSmokeGateInput & {
  forceSkip?: boolean;
  mockEvidenceOnly?: boolean;
  /** Override MCP URL for live create (defaults to local Workbench MCP). */
  mcpUrl?: string;
  model?: string;
};

export type LiveSmokeResult = {
  status: "pass" | "skip" | "fail";
  cursorAcSatisfied: boolean;
  externalAcceptancePending: boolean;
  notes: string;
  usedNonGitCwd?: boolean;
  mcpServerVisible?: boolean;
  agentCreated?: boolean;
};

export function evaluateLiveSmokeGate(
  input: LiveSmokeGateInput,
): LiveSmokeGateResult {
  const key = (input.apiKey ?? "").trim();
  if (!key) {
    return {
      shouldRun: false,
      verdict: "skip",
      reason: "missing CURSOR_API_KEY / credential",
    };
  }
  if (!isNodeVersionAtLeast(input.nodeVersion)) {
    return {
      shouldRun: false,
      verdict: "skip",
      reason: "Node version below 22.13",
    };
  }
  if (!input.mcpReady) {
    return {
      shouldRun: false,
      verdict: "skip",
      reason: "Workbench MCP endpoint not ready",
    };
  }
  return { shouldRun: true, verdict: "run", reason: "env ready" };
}

function pendingNotes(extra: string): string {
  return `live-smoke: skip — 外部验收待完成. ${extra}`;
}

/**
 * Env-gated live smoke. Without credentials/env → status=skip and
 * externalAcceptancePending=true (not fake-green; Cursor AC unsatisfied).
 */
export async function runLiveSmokeCursorSdk(
  input: LiveSmokeRunInput,
): Promise<LiveSmokeResult> {
  if (input.mockEvidenceOnly) {
    return {
      status: "skip",
      cursorAcSatisfied: false,
      externalAcceptancePending: true,
      notes: pendingNotes("mock/test-double evidence only; not Cursor AC"),
    };
  }

  if (input.forceSkip) {
    return {
      status: "skip",
      cursorAcSatisfied: false,
      externalAcceptancePending: true,
      notes: pendingNotes("forceSkip"),
    };
  }

  const gate = evaluateLiveSmokeGate(input);
  if (!gate.shouldRun) {
    return {
      status: "skip",
      cursorAcSatisfied: false,
      externalAcceptancePending: true,
      notes: pendingNotes(gate.reason),
    };
  }

  const cwd = mkdtempSync(join(tmpdir(), "cursor-live-smoke-"));
  let agent: Awaited<ReturnType<typeof Agent.create>> | null = null;
  try {
    if (existsSync(join(cwd, ".git"))) {
      throw new Error("live smoke cwd unexpectedly contains .git");
    }

    const gateLocal = ensureNodeAndSandbox({
      nodeVersion: input.nodeVersion,
      cwd,
      workbenchMcpHost: "127.0.0.1",
      sdkSandboxEnabled: false,
    });
    if (!gateLocal.ok) {
      return {
        status: "fail",
        cursorAcSatisfied: false,
        externalAcceptancePending: true,
        notes: `live-smoke fail-closed: ${gateLocal.error.message}`,
      };
    }

    const mcpUrl = input.mcpUrl ?? "http://127.0.0.1:9876/mcp";
    const mcpServers = {
      workbench: {
        url: mcpUrl,
        headers: {
          Accept: "application/json, text/event-stream",
        },
      },
    };

    writeFileSync(join(cwd, ".live-smoke-marker"), "ok", "utf8");

    agent = await Agent.create({
      apiKey: input.apiKey.trim(),
      model: { id: input.model ?? "composer-2.5" },
      local: {
        cwd,
        settingSources: [],
        sandboxOptions: { enabled: false },
      },
      mcpServers,
    });

    const run = await agent.send(
      "Reply with exactly: pong. Do not modify any files.",
    );
    await run.wait();

    return {
      status: "pass",
      cursorAcSatisfied: true,
      externalAcceptancePending: false,
      notes: "live-smoke pass: Agent.create + send/wait; MCP server config accepted",
      usedNonGitCwd: true,
      mcpServerVisible: true,
      agentCreated: true,
    };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      status: "fail",
      cursorAcSatisfied: false,
      externalAcceptancePending: true,
      notes: `live-smoke fail: ${msg}; 外部验收待完成`,
      usedNonGitCwd: true,
      agentCreated: Boolean(agent),
    };
  } finally {
    try {
      const dispose = (agent as { [Symbol.asyncDispose]?: () => Promise<void> } | null)?.[
        Symbol.asyncDispose
      ];
      if (typeof dispose === "function" && agent) {
        await dispose.call(agent);
      }
    } catch {
      // best-effort dispose
    }
    try {
      rmSync(cwd, { recursive: true, force: true });
    } catch {
      // ignore cleanup errors
    }
  }
}
