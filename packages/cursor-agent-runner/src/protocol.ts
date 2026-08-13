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
  business_id?: string;
  model: string;
  cwd: string;
  mcpServers?: Record<string, unknown>;
};

export type TurnParams = {
  business_id?: string;
  prompt: string;
};

export type RunnerRequest =
  | { id: string; method: "create"; params: CreateParams }
  | { id: string; method: "turn"; params: TurnParams }
  | {
      id: string;
      method: "cancel";
      params?: { business_id?: string };
    }
  | {
      id: string;
      method: "close";
      params?: { business_id?: string };
    };

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
      const sessionId = params.session_id;
      const businessId = params.business_id;
      const model = params.model;
      const cwd = params.cwd;
      if (typeof sessionId !== "string" || sessionId.trim().length === 0) {
        throw new ProtocolError(
          "runner",
          "create.params.session_id must be a non-empty string",
        );
      }
      if (
        businessId !== undefined &&
        (typeof businessId !== "string" || businessId.trim().length === 0)
      ) {
        throw new ProtocolError(
          "runner",
          "create.params.business_id must be a non-empty string",
        );
      }
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
        params: {
          business_id: typeof businessId === "string" ? businessId.trim() : undefined,
          session_id: sessionId,
          model,
          cwd,
          mcpServers,
        },
      };
    }
    case "turn": {
      const businessId = params.business_id;
      const prompt = params.prompt;
      if (
        businessId !== undefined &&
        (typeof businessId !== "string" || businessId.trim().length === 0)
      ) {
        throw new ProtocolError(
          "runner",
          "turn.params.business_id must be a non-empty string",
        );
      }
      if (typeof prompt !== "string") {
        throw new ProtocolError("runner", "turn.params.prompt must be a string");
      }
      return {
        id,
        method: "turn",
        params: {
          business_id: typeof businessId === "string" ? businessId.trim() : undefined,
          prompt,
        },
      };
    }
    case "cancel": {
      const businessId = params.business_id;
      if (
        businessId !== undefined &&
        (typeof businessId !== "string" || businessId.trim().length === 0)
      ) {
        throw new ProtocolError(
          "runner",
          "cancel.params.business_id must be a non-empty string",
        );
      }
      return {
        id,
        method: "cancel",
        params:
          typeof businessId === "string"
            ? { business_id: businessId.trim() }
            : undefined,
      };
    }
    case "close": {
      const businessId = params.business_id;
      if (
        businessId !== undefined &&
        (typeof businessId !== "string" || businessId.trim().length === 0)
      ) {
        throw new ProtocolError(
          "runner",
          "close.params.business_id must be a non-empty string",
        );
      }
      return {
        id,
        method: "close",
        params:
          typeof businessId === "string"
            ? { business_id: businessId.trim() }
            : undefined,
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
