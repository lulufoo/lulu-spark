export const ERROR_TYPES = [
  "credential",
  "sdk_config",
  "mcp_unavailable",
  "cwd",
  "runner",
  "sdk_run",
  "cancelled",
] as const;

export type RunnerErrorType = (typeof ERROR_TYPES)[number];

export type RunnerError = {
  type: RunnerErrorType;
  message: string;
};

export class ProtocolError extends Error {
  readonly type: RunnerErrorType;

  constructor(type: RunnerErrorType, message: string) {
    super(message);
    this.name = "ProtocolError";
    this.type = type;
  }
}

export type CreateParams = {
  model: string;
  cwd: string;
  mcpServers?: Record<string, unknown>;
};

export type TurnParams = {
  prompt: string;
};

export type RunnerRequest =
  | { id: string; method: "create"; params: CreateParams }
  | { id: string; method: "turn"; params: TurnParams }
  | { id: string; method: "cancel"; params?: Record<string, never> }
  | { id: string; method: "close"; params?: Record<string, never> };

function asRecord(value: unknown): Record<string, unknown> | null {
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return null;
}

export function parseRequest(line: string): RunnerRequest {
  let raw: unknown;
  try {
    raw = JSON.parse(line);
  } catch {
    throw new ProtocolError("runner", "invalid JSONL request");
  }

  const obj = asRecord(raw);
  if (!obj) {
    throw new ProtocolError("runner", "request must be a JSON object");
  }

  const id = obj.id;
  if (typeof id !== "string" || id.length === 0) {
    throw new ProtocolError("runner", "request id is required");
  }

  const method = obj.method;
  if (typeof method !== "string") {
    throw new ProtocolError("runner", "request method is required");
  }

  const params = asRecord(obj.params) ?? {};

  switch (method) {
    case "create": {
      const model = params.model;
      const cwd = params.cwd;
      if (typeof model !== "string") {
        throw new ProtocolError("sdk_config", "create.params.model must be a string");
      }
      if (typeof cwd !== "string") {
        throw new ProtocolError("cwd", "create.params.cwd must be a string");
      }
      const mcpServers =
        params.mcpServers === undefined
          ? {}
          : asRecord(params.mcpServers);
      if (mcpServers === null) {
        throw new ProtocolError(
          "runner",
          "create.params.mcpServers must be an object when provided",
        );
      }
      return {
        id,
        method: "create",
        params: { model, cwd, mcpServers },
      };
    }
    case "turn": {
      const prompt = params.prompt;
      if (typeof prompt !== "string") {
        throw new ProtocolError("runner", "turn.params.prompt must be a string");
      }
      return { id, method: "turn", params: { prompt } };
    }
    case "cancel":
      return { id, method: "cancel" };
    case "close":
      return { id, method: "close" };
    default:
      throw new ProtocolError("runner", `unknown method: ${method}`);
  }
}

export function serializeOkResponse(
  id: string,
  result: Record<string, unknown> = {},
): string {
  return JSON.stringify({ id, ok: true, result });
}

export function serializeErrorResponse(id: string, error: RunnerError): string {
  return JSON.stringify({
    id,
    ok: false,
    error: { type: error.type, message: error.message },
  });
}
