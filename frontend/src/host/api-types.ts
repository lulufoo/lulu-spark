export type ServiceError = Error & { status?: number };

export type InvokeResponse = {
  ok: boolean;
  status: number;
  json: () => Promise<any>;
  text: () => Promise<string>;
};

export type JsonReader = {
  getJson: (pathAndQuery: string) => Promise<unknown>;
};

export type ApiDriver = JsonReader & {
  fetchGet: (pathAndQuery: string) => Promise<InvokeResponse>;
  postJson: (path: string, body?: unknown) => Promise<InvokeResponse>;
  buildUrl?: (pathAndQuery: string) => string;
};

export type TauriInvoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

/** Path args from leftover DOM / hash often arrive as null. */
export type PathArg = string | null | undefined;

export function asRecord(data: unknown): Record<string, unknown> | null {
  if (!data || typeof data !== 'object' || Array.isArray(data)) return null;
  return data as Record<string, unknown>;
}
