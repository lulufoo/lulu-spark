import { existsSync, statSync } from "node:fs";
import { resolve as resolvePath } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { Agent, AuthenticationError, ConfigurationError } from "@cursor/sdk";
import type { McpServerConfig, SDKAgent, Run } from "@cursor/sdk";
import {
  parseRequest,
  ProtocolError,
  serializeErrorResponse,
  serializeOkResponse,
  type RunnerError,
  type RunnerRequest,
} from "./protocol.ts";
import { ensureNodeAndSandbox } from "./sandbox.ts";

type AgentHandle = SDKAgent & {
  agentId?: string;
  [Symbol.asyncDispose]?: () => Promise<void>;
};

let agent: AgentHandle | null = null;
let activeRun: Run | null = null;

function readApiKeyFromEnv(): string | null {
  const key = process.env.CURSOR_API_KEY;
  if (typeof key !== "string") return null;
  const trimmed = key.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function workbenchHostFromMcpServers(
  mcpServers: Record<string, unknown> | undefined,
): string {
  if (!mcpServers) return "127.0.0.1";
  for (const value of Object.values(mcpServers)) {
    if (
      value &&
      typeof value === "object" &&
      "url" in value &&
      typeof (value as { url: unknown }).url === "string"
    ) {
      try {
        const u = new URL((value as { url: string }).url);
        if (u.hostname) return u.hostname;
      } catch {
        // ignore malformed placeholder URLs in t1
      }
    }
  }
  return "127.0.0.1";
}

function mapSdkError(err: unknown): RunnerError {
  if (err instanceof ProtocolError) {
    return { type: err.type, message: err.message };
  }
  if (err instanceof AuthenticationError) {
    return { type: "credential", message: "Cursor authentication failed" };
  }
  if (err instanceof ConfigurationError) {
    const msg = err instanceof Error ? err.message : "SDK configuration error";
    if (/sandbox/i.test(msg)) {
      return { type: "sdk_config", message: msg };
    }
    return { type: "sdk_config", message: "SDK configuration error" };
  }
  if (err instanceof Error) {
    if (/cancel/i.test(err.message)) {
      return { type: "cancelled", message: "run cancelled" };
    }
    return { type: "sdk_run", message: "SDK run failed" };
  }
  return { type: "sdk_run", message: "SDK run failed" };
}

function redactError(error: RunnerError): RunnerError {
  // Never echo secrets or env names in JSONL.
  const message = error.message
    .replace(/CURSOR_API_KEY/gi, "[redacted]")
    .replace(/api[_-]?key/gi, "[redacted]")
    .replace(/sk-[a-zA-Z0-9_-]+/g, "[redacted]");
  return { type: error.type, message };
}

async function handleCreate(req: Extract<RunnerRequest, { method: "create" }>) {
  if (agent) {
    throw new ProtocolError("runner", "agent already created; close first");
  }

  const apiKey = readApiKeyFromEnv();
  if (!apiKey) {
    throw new ProtocolError(
      "credential",
      "missing API key in process environment",
    );
  }

  const model = req.params.model.trim();
  if (!model) {
    throw new ProtocolError("sdk_config", "model must be non-empty");
  }

  const cwd = req.params.cwd;
  if (!existsSync(cwd) || !statSync(cwd).isDirectory()) {
    throw new ProtocolError("cwd", "local.cwd is missing or not a directory");
  }

  const gate = ensureNodeAndSandbox({
    cwd,
    workbenchMcpHost: workbenchHostFromMcpServers(req.params.mcpServers),
  });
  if (!gate.ok) {
    throw new ProtocolError(gate.error.type, gate.error.message);
  }

  const mcpServers = (req.params.mcpServers ?? {}) as Record<
    string,
    McpServerConfig
  >;

  try {
    const created = (await Agent.create({
      apiKey,
      model: { id: model },
      local: {
        cwd,
        settingSources: [],
        sandboxOptions: { enabled: true },
      },
      mcpServers,
    })) as AgentHandle;
    agent = created;
    return {
      agentId: created.agentId ?? null,
    };
  } catch (err) {
    throw Object.assign(new Error("create failed"), { cause: err });
  }
}

async function handleTurn(req: Extract<RunnerRequest, { method: "turn" }>) {
  if (!agent) {
    throw new ProtocolError("runner", "no agent; call create first");
  }
  if (activeRun) {
    throw new ProtocolError("runner", "turn already in flight");
  }

  try {
    const run = await agent.send(req.params.prompt);
    activeRun = run;
    const result = await run.wait();
    activeRun = null;
    const status = (result as { status?: string } | undefined)?.status;
    if (status === "cancelled") {
      throw new ProtocolError("cancelled", "run cancelled");
    }
    const text =
      typeof (result as { result?: string } | undefined)?.result === "string"
        ? (result as { result: string }).result
        : "";
    return { text, status: status ?? "finished" };
  } catch (err) {
    activeRun = null;
    if (err instanceof ProtocolError) throw err;
    throw Object.assign(new Error("turn failed"), { cause: err });
  }
}

async function handleCancel() {
  if (!activeRun) {
    return { cancelled: false };
  }
  try {
    await activeRun.cancel();
  } catch (err) {
    throw Object.assign(new Error("cancel failed"), { cause: err });
  } finally {
    activeRun = null;
  }
  return { cancelled: true };
}

async function handleClose() {
  if (activeRun) {
    try {
      await activeRun.cancel();
    } catch {
      // best-effort cancel before dispose
    }
    activeRun = null;
  }
  if (agent) {
    const dispose = agent[Symbol.asyncDispose];
    if (typeof dispose === "function") {
      await dispose.call(agent);
    }
    agent = null;
  }
  return { closed: true };
}

async function dispatch(req: RunnerRequest): Promise<Record<string, unknown>> {
  switch (req.method) {
    case "create":
      return handleCreate(req);
    case "turn":
      return handleTurn(req);
    case "cancel":
      return handleCancel();
    case "close":
      return handleClose();
  }
}

function errorFromCaught(err: unknown): RunnerError {
  if (err instanceof ProtocolError) {
    return redactError({ type: err.type, message: err.message });
  }
  if (err && typeof err === "object" && "cause" in err) {
    return redactError(mapSdkError((err as { cause: unknown }).cause));
  }
  return redactError(mapSdkError(err));
}

export async function handleLine(line: string): Promise<string> {
  const trimmed = line.trim();
  if (!trimmed) return "";

  let id = "unknown";
  try {
    const req = parseRequest(trimmed);
    id = req.id;
    const result = await dispatch(req);
    return serializeOkResponse(id, result);
  } catch (err) {
    const error = errorFromCaught(err);
    if (id === "unknown") {
      try {
        const maybe = JSON.parse(trimmed) as { id?: unknown };
        if (typeof maybe.id === "string") id = maybe.id;
      } catch {
        // keep unknown
      }
    }
    return serializeErrorResponse(id, error);
  }
}

async function main(): Promise<void> {
  const rl = createInterface({ input: process.stdin, crlfDelay: Infinity });
  for await (const line of rl) {
    const response = await handleLine(line);
    if (response) {
      process.stdout.write(`${response}\n`);
    }
  }
}

const isMain =
  typeof process.argv[1] === "string" &&
  resolvePath(process.argv[1]) === fileURLToPath(import.meta.url);

if (isMain) {
  main().catch((err) => {
    const error = redactError({
      type: "runner",
      message: err instanceof Error ? err.message : "runner crashed",
    });
    process.stdout.write(`${serializeErrorResponse("unknown", error)}\n`);
    process.exitCode = 1;
  });
}
