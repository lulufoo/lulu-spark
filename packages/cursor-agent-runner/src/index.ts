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

export type AgentFactoryParams = {
  apiKey: string;
  model: string;
  cwd: string;
  mcpServers: Record<string, McpServerConfig>;
  sessionId: string;
};

export type AgentFactory = (params: AgentFactoryParams) => Promise<AgentHandle>;

let agent: AgentHandle | null = null;
let agentSessionId: string | null = null;
let activeRun: Run | null = null;
let agentFactoryOverride: AgentFactory | null = null;

type PendingCreate = {
  req: Extract<RunnerRequest, { method: "create" }>;
  resolve: (value: Record<string, unknown>) => void;
  reject: (err: unknown) => void;
};

let createRunning = false;
let pendingCreate: PendingCreate | null = null;

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
        // ignore malformed placeholder URLs
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
  const message = error.message
    .replace(/CURSOR_API_KEY/gi, "[redacted]")
    .replace(/api[_-]?key/gi, "[redacted]")
    .replace(/sk-[a-zA-Z0-9_-]+/g, "[redacted]");
  return { type: error.type, message };
}

async function defaultAgentFactory(
  params: AgentFactoryParams,
): Promise<AgentHandle> {
  return (await Agent.create({
    apiKey: params.apiKey,
    model: { id: params.model },
    local: {
      cwd: params.cwd,
      settingSources: [],
      sandboxOptions: { enabled: false },
    },
    mcpServers: params.mcpServers,
  })) as AgentHandle;
}

function activeFactory(): AgentFactory {
  return agentFactoryOverride ?? defaultAgentFactory;
}

async function disposeCurrentAgent(): Promise<string | undefined> {
  if (!agent) return undefined;
  if (activeRun) {
    try {
      await activeRun.cancel();
    } catch {
      // best-effort cancel before dispose
    }
    activeRun = null;
  }
  const replaced = agentSessionId ?? undefined;
  const dispose = agent[Symbol.asyncDispose];
  try {
    if (typeof dispose === "function") {
      await dispose.call(agent);
    }
  } catch (err) {
    // Consistent failure: no usable current instance; allow retry create.
    agent = null;
    agentSessionId = null;
    throw new ProtocolError(
      "runner",
      err instanceof Error ? err.message : "agent dispose failed",
    );
  }
  agent = null;
  agentSessionId = null;
  return replaced;
}

async function runReplaceCreate(
  req: Extract<RunnerRequest, { method: "create" }>,
): Promise<Record<string, unknown>> {
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
    sdkSandboxEnabled: false,
  });
  if (!gate.ok) {
    throw new ProtocolError(gate.error.type, gate.error.message);
  }

  const mcpServers = (req.params.mcpServers ?? {}) as Record<
    string,
    McpServerConfig
  >;
  const sessionId = req.params.session_id;

  const replaced = await disposeCurrentAgent();

  try {
    const created = await activeFactory()({
      apiKey,
      model,
      cwd,
      mcpServers,
      sessionId,
    });
    agent = created;
    agentSessionId = sessionId;
    const result: Record<string, unknown> = {
      agentId: created.agentId ?? null,
    };
    if (replaced) {
      result.replaced_session_id = replaced;
    }
    return result;
  } catch (err) {
    agent = null;
    agentSessionId = null;
    throw Object.assign(new Error("create failed"), { cause: err });
  }
}

function enqueueCreate(
  req: Extract<RunnerRequest, { method: "create" }>,
): Promise<Record<string, unknown>> {
  return new Promise((resolve, reject) => {
    if (createRunning) {
      if (pendingCreate) {
        // Coalesce-to-latest: skipped middle create never executed → not Cancelled.
        pendingCreate.reject(
          new ProtocolError(
            "coalesced",
            "create superseded by newer foreground; never executed",
          ),
        );
      }
      pendingCreate = { req, resolve, reject };
      return;
    }
    createRunning = true;
    runReplaceCreate(req)
      .then(resolve, reject)
      .finally(() => {
        createRunning = false;
        if (pendingCreate) {
          const next = pendingCreate;
          pendingCreate = null;
          enqueueCreate(next.req).then(next.resolve, next.reject);
        }
      });
  });
}

async function handleCreate(req: Extract<RunnerRequest, { method: "create" }>) {
  return enqueueCreate(req);
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
    agentSessionId = null;
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

export function setAgentFactoryForTests(factory: AgentFactory | null): void {
  agentFactoryOverride = factory;
}

export function resetRunnerStateForTests(): void {
  agent = null;
  agentSessionId = null;
  activeRun = null;
  createRunning = false;
  pendingCreate = null;
}

export function agentCardinalityForTests(): number {
  return agent ? 1 : 0;
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
