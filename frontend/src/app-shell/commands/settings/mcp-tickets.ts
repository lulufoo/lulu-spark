import * as api from '../../../host/api.ts';
import { setResult, store } from '../../state/settings/store.ts';
import { errMessage } from '../../state/types.ts';

type TicketView = {
  channel?: string;
  state?: string;
  hint?: string;
  handle?: string;
  devices?: Array<{
    device_id?: string;
    device_label?: string;
    revoked?: boolean;
    hint?: string;
  }>;
};

const IDE_CHANNELS = ['cursor', 'codex', 'claude'] as const;
const IDE_LABELS: Record<string, string> = {
  cursor: 'Cursor',
  codex: 'Codex',
  claude: 'Claude',
};

function isIdeChannel(channel: string) {
  return IDE_CHANNELS.includes(channel as (typeof IDE_CHANNELS)[number]);
}

export function ideServerUrl(channel: string) {
  const path = isIdeChannel(channel) ? channel : 'cursor';
  return `http://127.0.0.1:${store.mcpPort}/mcp/${path}`;
}

export function formatIdeServerBlock(channel: string, handle: string) {
  const url = ideServerUrl(channel);
  if (channel === 'codex') {
    return [
      '[mcp_servers.lulu-spark]',
      `url = "${url}"`,
      `http_headers = { Authorization = "Bearer ${handle}" }`,
    ].join('\n');
  }
  if (channel === 'claude') {
    return JSON.stringify(
      {
        mcpServers: {
          'lulu-spark': {
            type: 'http',
            url,
            headers: { Authorization: `Bearer ${handle}` },
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
      headers: { Authorization: `Bearer ${handle}` },
    },
    null,
    2,
  );
}

export function cursorIdeServerUrl() {
  return ideServerUrl('cursor');
}

export function formatCursorIdeServerBlock(handle: string) {
  return formatIdeServerBlock('cursor', handle);
}

export function setMcpServerBlock(text: string) {
  const el = document.getElementById('settings-mcp-server-block') as HTMLTextAreaElement | null;
  if (el) el.value = text;
}

export function clearMcpServerBlock() {
  setMcpServerBlock('');
}

export function ticketChannel() {
  return (
    (document.getElementById('settings-mcp-ticket-channel') as HTMLSelectElement | null)?.value
    || 'cursor'
  );
}

export function showTicketPane(channel: string) {
  const pane = isIdeChannel(channel) ? 'ide' : channel;
  for (const id of ['spark', 'mobile', 'ide']) {
    const el = document.getElementById(`settings-mcp-tickets-${id}`);
    if (el) el.hidden = id !== pane;
  }
  const label = document.getElementById('settings-mcp-server-block-label');
  if (label) label.textContent = `${IDE_LABELS[channel] || 'Cursor'} server block`;
}

async function copyServerBlock(text: string) {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Tauri webview often denies Clipboard API; keep the ticket in the field.
  }
  return false;
}

function paintSpark(view: TicketView) {
  const mask = document.getElementById('settings-mcp-spark-mask');
  const state = document.getElementById('settings-mcp-spark-state');
  const expire = document.getElementById(
    'btn-settings-mcp-spark-expire',
  ) as HTMLButtonElement | null;
  const live = view.state === 'live';
  if (mask) mask.textContent = live ? view.hint || '••••' : 'No live ticket';
  if (state) {
    state.textContent = live
      ? 'Host-issued ticket. Masked. Expire voids it for the next session.'
      : view.state === 'revoked'
        ? 'Expired. The next Spark session issues a new ticket.'
        : 'No Spark ticket yet.';
  }
  if (expire) expire.disabled = !live;
}

function paintMobile(view: TicketView) {
  const list = document.getElementById('settings-mcp-device-list');
  if (!list) return;
  list.replaceChildren();
  const devices = view.devices || [];
  if (!devices.length) {
    const empty = document.createElement('p');
    empty.className = 'settings-field-hint';
    empty.textContent = 'No bound devices.';
    list.appendChild(empty);
    return;
  }
  for (const device of devices) {
    const card = document.createElement('div');
    card.className = 'settings-mcp-device-card';
    if (device.revoked) card.classList.add('is-expired');
    const title = document.createElement('div');
    title.className = 'settings-mcp-ticket-kicker';
    title.textContent = device.device_label || device.device_id || 'Device';
    const id = document.createElement('div');
    id.className = 'settings-mcp-device-id';
    id.textContent = device.device_id || '';
    const mask = document.createElement('div');
    mask.className = 'settings-mcp-token-mask';
    mask.textContent = device.hint || '••••';
    card.appendChild(title);
    card.appendChild(id);
    card.appendChild(mask);
    if (device.revoked) {
      const status = document.createElement('div');
      status.className = 'settings-field-hint';
      status.textContent = 'Expired';
      card.appendChild(status);
    } else if (device.device_id) {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'btn-settings-save';
      btn.dataset.mcpDeviceExpire = device.device_id;
      btn.textContent = 'Expire';
      card.appendChild(btn);
    }
    list.appendChild(card);
  }
}

let cursorLive = false;

function paintCursorPrimary() {
  const btn = document.getElementById('btn-settings-mcp-primary') as HTMLButtonElement | null;
  const copy = document.getElementById('btn-settings-mcp-copy') as HTMLButtonElement | null;
  if (btn) {
    btn.textContent = cursorLive ? 'Refresh' : 'Generate';
    btn.dataset.mcpTicketAction = cursorLive ? 'refresh' : 'generate';
  }
  if (copy) copy.disabled = !cursorLive;
}

function paintCursor(view: TicketView) {
  cursorLive = view.state === 'live' && Boolean(view.handle);
  if (cursorLive && view.handle) {
    setMcpServerBlock(formatIdeServerBlock(ticketChannel(), view.handle));
  } else {
    clearMcpServerBlock();
  }
  paintCursorPrimary();
}

export async function loadMcpTicketView() {
  const channel = ticketChannel();
  showTicketPane(channel);
  try {
    const view = (await api.invoke('get_mcp_ticket_view', { channel })) as TicketView;
    if (channel === 'spark') paintSpark(view);
    else if (channel === 'mobile') paintMobile(view);
    else paintCursor(view);
  } catch (e) {
    setResult('settings-result-mcp', `Load tickets failed: ${errMessage(e, String(e))}`, true);
  }
}

async function issueOrRotateCursorIdeBlock(cmd: string, copiedMessage: string, failLabel: string) {
  setResult('settings-result-mcp', '');
  try {
    const resp = (await api.invoke(cmd)) as { handle?: string };
    const issued = resp?.handle;
    if (!issued) throw new Error('Ticket command failed');
    const text = formatIdeServerBlock(ticketChannel(), issued);
    setMcpServerBlock(text);
    const copied = await copyServerBlock(text);
    cursorLive = true;
    paintCursorPrimary();
    const doneLabel = failLabel === 'Refresh' ? 'Refreshed' : 'Generated';
    setResult(
      'settings-result-mcp',
      copied
        ? copiedMessage
        : `${doneLabel} ${IDE_LABELS[ticketChannel()] || 'Cursor'} server block. Clipboard copy was blocked — copy the block from the field.`,
    );
  } catch (e) {
    if (!cursorLive) clearMcpServerBlock();
    if (failLabel === 'Generate') {
      cursorLive = false;
      paintCursorPrimary();
    }
    setResult('settings-result-mcp', `${failLabel} failed: ${errMessage(e, String(e))}`, true);
  }
}

export async function runCursorIdePrimaryAction() {
  const btn = document.getElementById('btn-settings-mcp-primary') as HTMLButtonElement | null;
  if (btn) btn.disabled = true;
  try {
    if (cursorLive) {
      await issueOrRotateCursorIdeBlock(
        'rotate_cursor_ide_ticket',
        `Refreshed and copied ${IDE_LABELS[ticketChannel()] || 'Cursor'} server block.`,
        'Refresh',
      );
    } else {
      await issueOrRotateCursorIdeBlock(
        'issue_cursor_ide_ticket',
        `Generated and copied ${IDE_LABELS[ticketChannel()] || 'Cursor'} server block.`,
        'Generate',
      );
    }
  } finally {
    if (btn) btn.disabled = false;
  }
}

export async function copyCursorIdeServerBlock() {
  const text = (document.getElementById('settings-mcp-server-block') as HTMLTextAreaElement | null)
    ?.value
    .trim();
  setResult('settings-result-mcp', '');
  if (!text) {
    setResult('settings-result-mcp', `Generate a ${IDE_LABELS[ticketChannel()] || 'Cursor'} ticket first.`, true);
    return;
  }
  const copied = await copyServerBlock(text);
  setResult(
    'settings-result-mcp',
    copied
      ? `Copied ${IDE_LABELS[ticketChannel()] || 'Cursor'} server block.`
      : 'Clipboard copy was blocked — copy the block from the field.',
  );
}

export async function expireSparkTicket() {
  const btn = document.getElementById(
    'btn-settings-mcp-spark-expire',
  ) as HTMLButtonElement | null;
  if (btn) btn.disabled = true;
  setResult('settings-result-mcp', '');
  try {
    await api.invoke('revoke_mcp_slot_ticket', { slot: 'spark' });
    setResult('settings-result-mcp', 'Expired Spark ticket.');
    await loadMcpTicketView();
  } catch (e) {
    setResult('settings-result-mcp', `Expire failed: ${errMessage(e, String(e))}`, true);
  } finally {
    if (btn) btn.disabled = false;
  }
}

export async function expireMobileDevice(deviceId: string) {
  setResult('settings-result-mcp', '');
  try {
    await api.invoke('revoke_mcp_device_ticket', { deviceId });
    setResult('settings-result-mcp', 'Expired device ticket.');
    await loadMcpTicketView();
  } catch (e) {
    setResult('settings-result-mcp', `Expire failed: ${errMessage(e, String(e))}`, true);
  }
}
