export type AppRoute = {
  name?: string;
  params?: Record<string, string>;
};

export type SettingsConfig = {
  spark_root?: string;
  workspaces_root?: string;
  mcp_port?: number;
  assistant_engine?: string;
  theme?: string;
  network?: NetworkConfig;
  llm?: Record<string, EngineLlmConfig>;
};

export type NetworkConfig = {
  use_system_proxy?: boolean;
  /** Host snapshot of the plan the kernel applies right now. */
  effective?: {
    generation?: number;
    plan?: {
      http?: PlanEntry | null;
      https?: PlanEntry | null;
      no_proxy?: string;
    };
  };
};

export type PlanEntry = {
  host_port?: string;
  source?: string;
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
