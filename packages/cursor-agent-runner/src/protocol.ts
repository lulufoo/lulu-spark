export const ERROR_TYPES = [
  "credential",
  "sdk_config",
  "mcp_unavailable",
  "cwd",
  "runner",
  "sdk_run",
  "cancelled",
  /** Superseded create that never executed (coalesce-to-latest); not Cancelled. */
  "coalesced",
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
  /** Opaque Host session id (replace-create / cwd cleanup). */
  session_id: string;
  /** Stable business slot key. */
  business_id: string;
  model: string;
  cwd: string;
  mcpServers: Record<string, unknown>;
};

export type TurnParams = {
  business_id: string;
  prompt: string;
};

export type RunnerRequest =
  | { id: string; method: "create"; params: CreateParams }
  | { id: string; method: "turn"; params: TurnParams }
  | {
      id: string;
      method: "cancel";
      params: { business_id: string };
    }
  | {
      id: string;
      method: "close";
      params: { business_id: string };
    };

function asRecord(value: unknown): Record<string, unknown> | null {
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return null;
}

function requiredString(
  params: Record<string, unknown>,
  method: string,
  field: string,
): string {
  const value = params[field];
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new ProtocolError(
      "runner",
      `${method}.params.${field} must be a non-empty string`,
    );
  }
  return value;
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
      const businessId = requiredString(params, "create", "business_id");
      const sessionId = requiredString(params, "create", "session_id");
      const model = requiredString(params, "create", "model");
      const cwd = requiredString(params, "create", "cwd");
      const mcpServers = asRecord(params.mcpServers);
      if (mcpServers === null) {
        throw new ProtocolError(
          "runner",
          "create.params.mcpServers must be a required object",
        );
      }
      return {
        id,
        method: "create",
        params: {
          business_id: businessId.trim(),
          session_id: sessionId,
          model,
          cwd,
          mcpServers,
        },
      };
    }
    case "turn": {
      const businessId = requiredString(params, "turn", "business_id");
      const prompt = requiredString(params, "turn", "prompt");
      return {
        id,
        method: "turn",
        params: {
          business_id: businessId.trim(),
          prompt,
        },
      };
    }
    case "cancel": {
      const businessId = requiredString(params, "cancel", "business_id");
      return {
        id,
        method: "cancel",
        params: { business_id: businessId.trim() },
      };
    }
    case "close": {
      const businessId = requiredString(params, "close", "business_id");
      return {
        id,
        method: "close",
        params: { business_id: businessId.trim() },
      };
    }
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
