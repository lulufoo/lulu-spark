export type AppRoute = {
  name?: string;
  params?: Record<string, string>;
};

export type SettingsConfig = {
  spark_root?: string;
  knowledge_root?: string;
  github_user_url?: string;
  spark_github_repo_url?: string;
  has_github_token?: boolean;
  mcp_port?: number;
  assistant_engine?: string;
  has_host_key?: boolean;
  llm?: { model?: string };
};

export type InferGithubResp = {
  github_user_url?: string;
  spark_github_repo_url?: string;
};

export type HeaderSyncDeps = {
  pullProject: () => Promise<void>;
  loadIndex: () => Promise<void>;
  openWorkbenchCommit?: () => void | Promise<void>;
};

export function errMessage(err: unknown, fallback: string): string {
  if (err && typeof err === 'object' && 'message' in err) {
    const msg = (err as { message?: unknown }).message;
    if (typeof msg === 'string' && msg) return msg;
  }
  return fallback;
}
