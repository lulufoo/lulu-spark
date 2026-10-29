import { store } from '../../state/settings/store.ts';

const IDE_CHANNELS = ['cursor', 'codex', 'claude'] as const;

export function ideServerUrl(channel: string) {
  const path = IDE_CHANNELS.includes(channel as (typeof IDE_CHANNELS)[number])
    ? channel
    : 'cursor';
  return `http://127.0.0.1:${store.mcpPort}/mcp/${path}`;
}

export function formatIdeServerBlock(channel: string) {
  const url = ideServerUrl(channel);
  if (channel === 'codex') {
    return ['[mcp_servers.lulu-spark]', `url = "${url}"`].join('\n');
  }
  if (channel === 'claude') {
    return JSON.stringify(
      {
        mcpServers: {
          'lulu-spark': {
            type: 'http',
            url,
          },
        },
      },
      null,
      2,
    );
  }
  return JSON.stringify({ url }, null, 2);
}

export function cursorIdeServerUrl() {
  return ideServerUrl('cursor');
}

export function formatCursorIdeServerBlock() {
  return formatIdeServerBlock('cursor');
}
