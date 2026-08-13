import { existsSync, statSync } from "node:fs";
import { resolve as resolvePath } from "node:path";
import { createInterface } from "node:readline";
import { isDeepStrictEqual } from "node:util";
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
  businessId?: string;
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

type AgentProfile = {
  model: string;
  cwd: string;
  mcpServers: Record<string, McpServerConfig>;
};

type AgentSlot = {
  businessId: string;
  profile: AgentProfile;
  agent: AgentHandle | null;
  createPromise: Promise<Record<string, unknown>> | null;
  turnTail: Promise<void>;
  activeSend: Promise<Run> | null;
  activeRun: Run | null;
  cancelRequested: boolean;
  closing: boolean;
};

export type SdkRunStatus =
  | "finished"
  | "error"
  | "cancelled"
  | "running"
  | "unknown";

const DIAGNOSTIC_TEXT_MAX = 512;
const DIAGNOSTIC_JSON_MAX = 1024;
const OMITTED_SNAPSHOT_KEYS = new Set([
  "result",
  "prompt",
  "content",
  "text",
  "images",
  "conversation",
]);

export type RunnerDiagnostic = {
  event: string;
  business_id: string;
  session_id?: string;
  error_type?: RunnerError["type"];
  sdk_status?: SdkRunStatus;
  has_result?: boolean;
  text_len?: number;
  has_error?: boolean;
  sdk_error_name?: string;
  sdk_error_code?: string;
  sdk_error_message?: string;
  sdk_error_keys?: string;
  sdk_error_json?: string;
  sdk_cause_name?: string;
  sdk_cause_code?: string;
  sdk_cause_message?: string;
  sdk_wait_keys?: string;
  sdk_wait_json?: string;
  sdk_model?: string;
  sdk_http_status?: number;
  sdk_duration_ms?: number;
  sdk_retryable?: boolean;
};

type RunnerDiagnosticExtras = {
  sdkStatus?: SdkRunStatus;
  hasResult?: boolean;
  textLen?: number;
  hasError?: boolean;
  sdkErrorName?: string;
  sdkErrorCode?: string;
  sdkErrorMessage?: string;
  sdkErrorKeys?: string;
  sdkErrorJson?: string;
  sdkCauseName?: string;
  sdkCauseCode?: string;
  sdkCauseMessage?: string;
  sdkWaitKeys?: string;
  sdkWaitJson?: string;
  sdkModel?: string;
  sdkHttpStatus?: number;
  sdkRetryable?: boolean;
  sdkDurationMs?: number;
};

const slots = new Map<string, AgentSlot>();
const diagnostics: RunnerDiagnostic[] = [];

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

function sdkStatusName(status: unknown): SdkRunStatus {
  switch (status) {
    case "finished":
    case "error":
    case "cancelled":
    case "running":
      return status;
    default:
      return "unknown";
  }
}

function turnResultEvent(status: SdkRunStatus, textLen: number): string {
  if (status === "error") return "turn_sdk_error";
  if (textLen === 0) return "turn_empty";
  return "turn_ok";
}

function redactSecrets(text: string): string {
  return text
    .replace(/CURSOR_API_KEY/gi, "[REDACTED]")
    .replace(/Bearer\s+\S+/gi, "Bearer [REDACTED]")
    .replace(/sk-[a-zA-Z0-9_-]+/g, "sk-[REDACTED]")
    .replace(/api[_-]?key[=:]\s*["']?[^"' \t]+/gi, "api_key=[REDACTED]");
}

function boundedText(value: unknown, max = DIAGNOSTIC_TEXT_MAX): string | undefined {
  if (value == null) return undefined;
  const raw =
    typeof value === "string"
      ? value
      : typeof value === "number" || typeof value === "boolean"
        ? String(value)
        : undefined;
  if (raw === undefined) return undefined;
  const collapsed = redactSecrets(raw)
    .replace(/[\r\n\t]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (!collapsed) return undefined;
  return collapsed.length > max ? collapsed.slice(0, max) : collapsed;
}

function errorSnapshot(error: Error): Record<string, unknown> {
  const obj = error as Error & Record<string, unknown>;
  return {
    name: error.name,
    message: error.message,
    ...(obj.code !== undefined ? { code: obj.code } : {}),
    ...(obj.status !== undefined ? { status: obj.status } : {}),
    ...(obj.statusCode !== undefined ? { statusCode: obj.statusCode } : {}),
    ...(typeof obj.isRetryable === "boolean"
      ? { isRetryable: obj.isRetryable }
      : {}),
    ...(obj.cause !== undefined ? { cause: obj.cause } : {}),
  };
}

function boundedJson(value: unknown, max = DIAGNOSTIC_JSON_MAX): string | undefined {
  try {
    const json = JSON.stringify(value, (key, nested) => {
      if (OMITTED_SNAPSHOT_KEYS.has(key)) return "[omitted]";
      const current = nested instanceof Error ? errorSnapshot(nested) : nested;
      if (typeof current === "string" && current.length > 300) {
        return redactSecrets(current).slice(0, 300);
      }
      if (typeof current === "string") return redactSecrets(current);
      return current;
    });
    return boundedText(json, max);
  } catch {
    return boundedText(value, max);
  }
}

function objectKeys(value: unknown): string | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
  const keys = Object.keys(value as Record<string, unknown>).sort();
  return keys.length > 0 ? keys.join(",") : undefined;
}

function readFiniteNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.round(value)
    : undefined;
}

function summarizeUnknownError(error: unknown): RunnerDiagnosticExtras {
  if (error == null) return {};
  if (typeof error !== "object") {
    return { sdkErrorMessage: boundedText(error) };
  }
  const obj = error as Record<string, unknown>;
  const ctorName =
    typeof obj.name === "string" && obj.name.trim()
      ? obj.name
      : error instanceof Error
        ? error.constructor.name
        : undefined;
  return {
    sdkErrorName: boundedText(ctorName, 64),
    sdkErrorCode: boundedText(obj.code ?? obj.type ?? obj.error_code, 64),
    sdkErrorMessage: boundedText(obj.message ?? obj.reason, DIAGNOSTIC_TEXT_MAX),
    sdkErrorKeys: objectKeys(error),
    sdkErrorJson: boundedJson(error),
    sdkHttpStatus: readFiniteNumber(obj.status ?? obj.statusCode),
    sdkRetryable:
      typeof obj.isRetryable === "boolean" ? obj.isRetryable : undefined,
  };
}

function summarizeCaught(err: unknown): RunnerDiagnosticExtras {
  const direct = summarizeUnknownError(err);
  if (!err || typeof err !== "object" || !("cause" in err)) return direct;
  const cause = summarizeUnknownError((err as { cause: unknown }).cause);
  return {
    ...direct,
    sdkCauseName: cause.sdkErrorName,
    sdkCauseCode: cause.sdkErrorCode,
    sdkCauseMessage: cause.sdkErrorMessage,
  };
}

function summarizeWaitResult(result: unknown): RunnerDiagnosticExtras {
  if (!result || typeof result !== "object") {
    return { sdkWaitJson: boundedJson({ wait_typeof: typeof result }) };
  }
  const obj = result as Record<string, unknown>;
  const errorExtras = summarizeUnknownError(obj.error);
  const model =
    obj.model && typeof obj.model === "object" && !Array.isArray(obj.model)
      ? boundedText((obj.model as { id?: unknown }).id, 64)
      : boundedText(obj.model, 64);
  return {
    sdkWaitKeys: objectKeys(result),
    sdkWaitJson: boundedJson(result),
    sdkModel: model,
    sdkDurationMs: readFiniteNumber(obj.durationMs),
    sdkErrorName: errorExtras.sdkErrorName,
    sdkErrorCode: errorExtras.sdkErrorCode,
    sdkErrorMessage: errorExtras.sdkErrorMessage,
    sdkErrorKeys: errorExtras.sdkErrorKeys,
    sdkErrorJson: errorExtras.sdkErrorJson,
    sdkHttpStatus: errorExtras.sdkHttpStatus,
    sdkRetryable: errorExtras.sdkRetryable,
  };
}

function extrasToJsonl(extras: RunnerDiagnosticExtras): Record<string, unknown> {
  return {
    ...(extras.sdkErrorName ? { sdk_error_name: extras.sdkErrorName } : {}),
    ...(extras.sdkErrorCode ? { sdk_error_code: extras.sdkErrorCode } : {}),
    ...(extras.sdkErrorMessage
      ? { sdk_error_message: extras.sdkErrorMessage }
      : {}),
    ...(extras.sdkErrorKeys ? { sdk_error_keys: extras.sdkErrorKeys } : {}),
    ...(extras.sdkErrorJson ? { sdk_error_json: extras.sdkErrorJson } : {}),
    ...(extras.sdkCauseName ? { sdk_cause_name: extras.sdkCauseName } : {}),
    ...(extras.sdkCauseCode ? { sdk_cause_code: extras.sdkCauseCode } : {}),
    ...(extras.sdkCauseMessage
      ? { sdk_cause_message: extras.sdkCauseMessage }
      : {}),
    ...(extras.sdkWaitKeys ? { sdk_wait_keys: extras.sdkWaitKeys } : {}),
    ...(extras.sdkWaitJson ? { sdk_wait_json: extras.sdkWaitJson } : {}),
    ...(extras.sdkModel ? { sdk_model: extras.sdkModel } : {}),
    ...(extras.sdkHttpStatus !== undefined
      ? { sdk_http_status: extras.sdkHttpStatus }
      : {}),
    ...(extras.sdkDurationMs !== undefined
      ? { sdk_duration_ms: extras.sdkDurationMs }
      : {}),
    ...(extras.sdkRetryable !== undefined
      ? { sdk_retryable: extras.sdkRetryable }
      : {}),
  };
}

function emitDiagnostic(
  event: string,
  businessId: string,
  sessionId?: string,
  errorType?: RunnerError["type"],
  extras?: RunnerDiagnosticExtras,
): void {
  const diagnostic: RunnerDiagnostic = {
    event,
    business_id: businessId,
    ...(sessionId ? { session_id: sessionId } : {}),
    ...(errorType ? { error_type: errorType } : {}),
    ...(extras?.sdkStatus ? { sdk_status: extras.sdkStatus } : {}),
    ...(extras?.hasResult !== undefined ? { has_result: extras.hasResult } : {}),
    ...(extras?.textLen !== undefined ? { text_len: extras.textLen } : {}),
    ...(extras?.hasError !== undefined ? { has_error: extras.hasError } : {}),
    ...(extras?.sdkErrorName ? { sdk_error_name: extras.sdkErrorName } : {}),
    ...(extras?.sdkErrorCode ? { sdk_error_code: extras.sdkErrorCode } : {}),
    ...(extras?.sdkErrorMessage
      ? { sdk_error_message: extras.sdkErrorMessage }
      : {}),
    ...(extras?.sdkErrorKeys ? { sdk_error_keys: extras.sdkErrorKeys } : {}),
    ...(extras?.sdkErrorJson ? { sdk_error_json: extras.sdkErrorJson } : {}),
    ...(extras?.sdkCauseName ? { sdk_cause_name: extras.sdkCauseName } : {}),
    ...(extras?.sdkCauseCode ? { sdk_cause_code: extras.sdkCauseCode } : {}),
    ...(extras?.sdkCauseMessage
      ? { sdk_cause_message: extras.sdkCauseMessage }
      : {}),
    ...(extras?.sdkWaitKeys ? { sdk_wait_keys: extras.sdkWaitKeys } : {}),
    ...(extras?.sdkWaitJson ? { sdk_wait_json: extras.sdkWaitJson } : {}),
    ...(extras?.sdkModel ? { sdk_model: extras.sdkModel } : {}),
    ...(extras?.sdkHttpStatus !== undefined
      ? { sdk_http_status: extras.sdkHttpStatus }
      : {}),
    ...(extras?.sdkDurationMs !== undefined
      ? { sdk_duration_ms: extras.sdkDurationMs }
      : {}),
    ...(extras?.sdkRetryable !== undefined
      ? { sdk_retryable: extras.sdkRetryable }
      : {}),
  };
  diagnostics.push(diagnostic);
  try {
    process.stderr.write(`${JSON.stringify(diagnostic)}\n`);
  } catch {
    // Diagnostics must not change runner behavior when stderr is unavailable.
  }
}

function emitProtocolFailure(
  event: string,
  businessId: string,
  error: ProtocolError,
  sessionId?: string,
): void {
  emitDiagnostic(
    event,
    businessId,
    sessionId,
    error.type,
    summarizeCaught(error),
  );
}

function profileFromCreate(
  req: Extract<RunnerRequest, { method: "create" }>,
): AgentProfile {
  return {
    model: req.params.model.trim(),
    cwd: req.params.cwd,
    mcpServers: req.params.mcpServers as Record<
      string,
      McpServerConfig
    >,
  };
}

function newSlot(
  businessId: string,
  profile: AgentProfile,
): AgentSlot {
  const slot: AgentSlot = {
    businessId,
    profile,
    agent: null,
    createPromise: null,
    turnTail: Promise.resolve(),
    activeSend: null,
    activeRun: null,
    cancelRequested: false,
    closing: false,
  };
  slots.set(businessId, slot);
  return slot;
}

function validateCreate(
  req: Extract<RunnerRequest, { method: "create" }>,
): {
  apiKey: string;
  profile: AgentProfile;
} {
  const apiKey = readApiKeyFromEnv();
  if (!apiKey) {
    throw new ProtocolError(
      "credential",
      "missing API key in process environment",
    );
  }

  const profile = profileFromCreate(req);
  if (!profile.model) {
    throw new ProtocolError("sdk_config", "model must be non-empty");
  }

  if (!existsSync(profile.cwd) || !statSync(profile.cwd).isDirectory()) {
    throw new ProtocolError("cwd", "local.cwd is missing or not a directory");
  }

  const gate = ensureNodeAndSandbox({
    cwd: profile.cwd,
    workbenchMcpHost: workbenchHostFromMcpServers(profile.mcpServers),
    sdkSandboxEnabled: false,
  });
  if (!gate.ok) {
    throw new ProtocolError(gate.error.type, gate.error.message);
  }

  return { apiKey, profile };
}

async function runSlotCreate(
  slot: AgentSlot,
  req: Extract<RunnerRequest, { method: "create" }>,
): Promise<Record<string, unknown>> {
  const businessId = slot.businessId;
  const sessionId = req.params.session_id;
  try {
    const { apiKey, profile } = validateCreate(req);
    const created = await activeFactory()({
      apiKey,
      businessId,
      model: profile.model,
      cwd: profile.cwd,
      mcpServers: profile.mcpServers,
      sessionId,
    });
    slot.profile = profile;
    slot.agent = created;
    slot.createPromise = null;
    emitDiagnostic("prewarm_create_ok", businessId, sessionId);
    return { agentId: created.agentId ?? null };
  } catch (err) {
    if (slots.get(businessId) === slot) {
      slots.delete(businessId);
    }
    const error = errorFromCaught(err);
    const extras = summarizeCaught(err);
    emitDiagnostic("prewarm_create_fail", businessId, sessionId, error.type, extras);
    emitDiagnostic("create_fail", businessId, sessionId, error.type, extras);
    if (err instanceof ProtocolError) throw err;
    throw Object.assign(new Error("create failed"), { cause: err });
  }
}

async function handleSlotCreate(
  req: Extract<RunnerRequest, { method: "create" }>,
): Promise<Record<string, unknown>> {
  const businessId = req.params.business_id;

  const requestedProfile = profileFromCreate(req);
  const existing = slots.get(businessId);
  if (existing?.closing) {
    const error = new ProtocolError(
      "runner",
      `business_id=${businessId} slot is closing`,
    );
    emitProtocolFailure("create_fail", businessId, error, req.params.session_id);
    throw error;
  }
  if (existing?.agent) {
    if (!isDeepStrictEqual(existing.profile, requestedProfile)) {
      const error = new ProtocolError(
        "runner",
        `ready slot profile mismatch for business_id=${businessId}`,
      );
      emitProtocolFailure("create_fail", businessId, error, req.params.session_id);
      throw error;
    }
    emitDiagnostic("create_hit", businessId, req.params.session_id);
    return { agentId: existing.agent.agentId ?? null };
  }

  if (existing?.createPromise) {
    emitDiagnostic("prewarm_in_progress", businessId, req.params.session_id);
    return existing.createPromise;
  }

  const slot = existing ?? newSlot(businessId, requestedProfile);
  const creation = runSlotCreate(slot, req);
  slot.createPromise = creation;
  return creation;
}

async function runSlotTurn(
  slot: AgentSlot,
  req: Extract<RunnerRequest, { method: "turn" }>,
): Promise<Record<string, unknown>> {
  if (slot.closing) {
    const error = new ProtocolError(
      "runner",
      `business_id=${slot.businessId} slot is closing`,
    );
    emitProtocolFailure("turn_fail", slot.businessId, error);
    throw error;
  }
  const agent = slot.agent;
  if (!agent) {
    const error = new ProtocolError(
      "runner",
      `business_id=${slot.businessId} has no ready agent`,
    );
    emitProtocolFailure("turn_fail", slot.businessId, error);
    throw error;
  }

  try {
    const sendPromise = agent.send(req.params.prompt);
    slot.activeSend = sendPromise;
    let run: Run;
    try {
      run = await sendPromise;
    } finally {
      slot.activeSend = null;
    }
    slot.activeRun = run;
    if (slot.closing) {
      try {
        await run.cancel();
      } catch {
        // Close will still wait for run.wait() before disposing the Agent.
      }
      const error = new ProtocolError(
        "runner",
        `business_id=${slot.businessId} slot is closing`,
      );
      emitProtocolFailure("turn_fail", slot.businessId, error);
      throw error;
    }
    if (slot.cancelRequested) {
      slot.cancelRequested = false;
      await run.cancel();
    }
    const result = await run.wait();
    slot.activeRun = null;
    const status = sdkStatusName(
      (result as { status?: string } | undefined)?.status,
    );
    if (status === "cancelled") {
      throw new ProtocolError("cancelled", "run cancelled");
    }
    const rawResult = (result as { result?: unknown } | undefined)?.result;
    const hasResult = typeof rawResult === "string";
    const text = hasResult ? (rawResult as string) : "";
    const hasError =
      (result as { error?: unknown } | undefined)?.error != null;
    const textLen = new TextEncoder().encode(text).length;
    const waitExtras = summarizeWaitResult(result);
    emitDiagnostic(
      turnResultEvent(status, textLen),
      slot.businessId,
      undefined,
      status === "error" ? "sdk_run" : undefined,
      {
        ...waitExtras,
        sdkStatus: status,
        hasResult,
        textLen,
        hasError,
      },
    );
    return { text, status, ...extrasToJsonl(waitExtras) };
  } catch (err) {
    slot.activeSend = null;
    slot.cancelRequested = false;
    slot.activeRun = null;
    if (!(err instanceof ProtocolError && err.type === "cancelled")) {
      emitDiagnostic(
        "turn_fail",
        slot.businessId,
        undefined,
        errorFromCaught(err).type,
        summarizeCaught(err),
      );
    }
    if (err instanceof ProtocolError) throw err;
    throw Object.assign(new Error("turn failed"), { cause: err });
  }
}

function enqueueSlotTurn(
  slot: AgentSlot,
  req: Extract<RunnerRequest, { method: "turn" }>,
): Promise<Record<string, unknown>> {
  const previous = slot.turnTail;
  let release!: () => void;
  slot.turnTail = new Promise<void>((resolve) => {
    release = resolve;
  });
  return previous.then(async () => {
    try {
      return await runSlotTurn(slot, req);
    } finally {
      release();
    }
  });
}

async function handleSlotTurn(
  req: Extract<RunnerRequest, { method: "turn" }>,
): Promise<Record<string, unknown>> {
  const businessId = req.params.business_id;
  const slot = slots.get(businessId);
  if (!slot) {
    const error = new ProtocolError(
      "runner",
      `business_id=${businessId} has an empty agent slot`,
    );
    emitProtocolFailure("turn_fail", businessId, error);
    throw error;
  }
  if (slot.createPromise) {
    emitDiagnostic("prewarm_in_progress", businessId);
    try {
      await slot.createPromise;
    } catch (err) {
      emitDiagnostic(
        "turn_fail",
        businessId,
        undefined,
        errorFromCaught(err).type,
        summarizeCaught(err),
      );
      throw err;
    }
  }
  return enqueueSlotTurn(slot, req);
}

async function handleSlotCancel(
  req: Extract<RunnerRequest, { method: "cancel" }>,
): Promise<Record<string, unknown>> {
  const businessId = req.params.business_id;
  const slot = slots.get(businessId);
  const run = slot?.activeRun;
  emitDiagnostic("cancel", businessId);
  if (!run) {
    if (slot?.activeSend) {
      slot.cancelRequested = true;
      return { cancelled: true };
    }
    return { cancelled: false };
  }
  try {
    await run.cancel();
  } catch (err) {
    throw Object.assign(new Error("cancel failed"), { cause: err });
  }
  return { cancelled: true };
}

async function disposeSlot(slot: AgentSlot): Promise<void> {
  slot.closing = true;
  if (slot.activeSend) {
    slot.cancelRequested = true;
    try {
      await slot.activeSend;
    } catch {
      // Best-effort wait for a send that is about to be cancelled.
    }
  }
  if (slot.activeRun) {
    try {
      await slot.activeRun.cancel();
    } catch {
      // Best-effort cancel before dispose.
    }
  }
  try {
    await slot.turnTail;
  } catch {
    // A cancelled/closing turn is expected to reject before disposal.
  }
  const agent = slot.agent;
  if (!agent) return;
  const dispose = agent[Symbol.asyncDispose];
  if (typeof dispose === "function") {
    await dispose.call(agent);
  }
  slot.agent = null;
}

async function handleSlotClose(
  req: Extract<RunnerRequest, { method: "close" }>,
): Promise<Record<string, unknown>> {
  const businessId = req.params.business_id;
  const slot = slots.get(businessId);
  if (!slot) return { closed: true };
  slot.closing = true;
  if (slot.createPromise) {
    try {
      await slot.createPromise;
    } catch {
      return { closed: true };
    }
  }
  try {
    await disposeSlot(slot);
    if (slots.get(businessId) === slot) {
      slots.delete(businessId);
    }
    return { closed: true };
  } catch (err) {
    // Keep the slot quarantined so a later close can retry the same Agent.
    slot.closing = true;
    throw err;
  }
}

async function shutdownSlots(): Promise<void> {
  const current = [...slots.values()];
  await Promise.all(
    current.map(async (slot) => {
      slot.closing = true;
      if (slot.createPromise) {
        try {
          await slot.createPromise;
        } catch {
          return;
        }
      }
      try {
        await disposeSlot(slot);
      } catch {
        // Shutdown is best-effort for every independent slot.
        return;
      }
      if (slots.get(slot.businessId) === slot) {
        slots.delete(slot.businessId);
      }
    }),
  );
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
    // Retain the old Agent so a later create can retry disposal.
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

async function handleLegacyCreate(
  req: Extract<RunnerRequest, { method: "create" }>,
) {
  return enqueueCreate(req);
}

async function handleLegacyTurn(
  req: Extract<RunnerRequest, { method: "turn" }>,
) {
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

async function handleLegacyCancel() {
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

async function handleLegacyClose() {
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
      return handleSlotCreate(req);
    case "turn":
      return handleSlotTurn(req);
    case "cancel":
      return handleSlotCancel(req);
    case "close":
      return handleSlotClose(req);
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
  slots.clear();
  diagnostics.length = 0;
}

export function agentCardinalityForTests(): number {
  const slotAgents = [...slots.values()].filter((slot) => slot.agent).length;
  return slotAgents + (agent ? 1 : 0);
}

export function diagnosticsForTests(): RunnerDiagnostic[] {
  return diagnostics.map((event) => ({ ...event }));
}

export async function shutdownRunnerForTests(): Promise<void> {
  await shutdownSlots();
  await handleLegacyClose();
}

/** Serialize stdout writes while allowing overlapping handleLine (slot waits / demux). */
let stdoutWriteChain: Promise<void> = Promise.resolve();

function writeResponseLine(response: string): Promise<void> {
  const run = stdoutWriteChain.then(() => {
    process.stdout.write(`${response}\n`);
  });
  stdoutWriteChain = run.catch(() => {
    // keep chain alive after write errors
  });
  return run;
}

async function main(): Promise<void> {
  const rl = createInterface({ input: process.stdin, crlfDelay: Infinity });
  for await (const line of rl) {
    // Do not await handleLine before reading the next stdin line — Host may
    // concurrently submit JSONL (pending-map demux); create coalesce needs overlap.
    void handleLine(line)
      .then((response) => {
        if (response) return writeResponseLine(response);
      })
      .catch((err) => {
        const error = redactError({
          type: "runner",
          message: err instanceof Error ? err.message : "runner crashed",
        });
        return writeResponseLine(serializeErrorResponse("unknown", error));
      });
  }
  await shutdownRunnerForTests();
}

const isMain =
  typeof process.argv[1] === "string" &&
  resolvePath(process.argv[1]) === fileURLToPath(import.meta.url);

if (isMain) {
  main().catch((err) => {
    const error = errorFromCaught(err);
    process.stderr.write(
      `${JSON.stringify({
        event: "runner_fatal",
        error_type: error.type,
        ...extrasToJsonl(summarizeCaught(err)),
      })}\n`,
    );
    process.stdout.write(`${serializeErrorResponse("unknown", error)}\n`);
    process.exitCode = 1;
  });
}
