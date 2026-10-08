export type AppRoute = {
  name?: string;
  params?: Record<string, string>;
};

export type SettingsConfig = {
  spark_root?: string;
  knowledge_root?: string;
  notes_root?: string;
  mcp_port?: number;
  assistant_engine?: string;
  theme?: string;
  has_host_key?: boolean;
  llm?: { model?: string; base_url?: string };
};

export function errMessage(err: unknown, fallback: string): string {
  if (err && typeof err === 'object' && 'message' in err) {
    const msg = (err as { message?: unknown }).message;
    if (typeof msg === 'string' && msg) return msg;
  }
  return fallback;
}
