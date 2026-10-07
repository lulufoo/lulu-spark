import { store } from '../../state/settings/store.ts';

const IDE_CHANNELS = ['cursor', 'codex', 'claude'] as const;

export const IDE_ENV_BASES: Record<string, string> = {
  cursor: 'LULU_SPARK_CURSOR_MCP',
  codex: 'LULU_SPARK_CODEX_MCP',
  claude: 'LULU_SPARK_CLAUDE_MCP',
};

export function ideEnvBase(channel: string) {
  return IDE_ENV_BASES[channel] || IDE_ENV_BASES.cursor;
}

export function ideServerUrl(channel: string) {
  const path = IDE_CHANNELS.includes(channel as (typeof IDE_CHANNELS)[number])
    ? channel
    : 'cursor';
  return `http://127.0.0.1:${store.mcpPort}/mcp/${path}`;
}

export function formatIdeServerBlock(channel: string, envVar: string) {
  const url = ideServerUrl(channel);
  if (channel === 'codex') {
    return [
      '[mcp_servers.lulu-spark]',
      `url = "${url}"`,
      `bearer_token_env_var = "${envVar}"`,
    ].join('\n');
  }
  const bearer = channel === 'claude' ? '${' + envVar + '}' : '${env:' + envVar + '}';
  if (channel === 'claude') {
    return JSON.stringify(
      {
        mcpServers: {
          'lulu-spark': {
            type: 'http',
            url,
            headers: { Authorization: `Bearer ${bearer}` },
          },
        },
      },
      null,
      2,
    );
  }
  return JSON.stringify(
    {
      url,
      headers: { Authorization: `Bearer ${bearer}` },
    },
    null,
    2,
  );
}

export function cursorIdeServerUrl() {
  return ideServerUrl('cursor');
}

export function formatCursorIdeServerBlock(envVar: string) {
  return formatIdeServerBlock('cursor', envVar);
}
