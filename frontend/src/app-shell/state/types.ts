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
  llm?: Record<string, EngineLlmConfig>;
};

export type EngineLlmConfig = {
  platform?: string;
  base_url?: string;
  model?: string;
  key_saved?: boolean;
};

export function errMessage(err: unknown, fallback: string): string {
  if (err && typeof err === 'object' && 'message' in err) {
    const msg = (err as { message?: unknown }).message;
    if (typeof msg === 'string' && msg) return msg;
  }
  return fallback;
}
