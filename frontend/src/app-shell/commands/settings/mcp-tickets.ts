import * as api from '../../../host/api.ts';
import { setResult } from '../../state/settings/store.ts';
import { errMessage } from '../../state/types.ts';
import { formatIdeServerBlock, ideServerUrl } from './mcp-server-block.ts';

export { formatCursorIdeServerBlock, formatIdeServerBlock, ideServerUrl } from './mcp-server-block.ts';

type TicketView = {
  channel?: string;
  state?: string;
  hint?: string;
  handle?: string;
  env_var?: string;
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

export function cursorIdeServerUrl() {
  return ideServerUrl('cursor');
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
  const live = view.state === 'live';
  if (mask) mask.textContent = live ? view.hint || '••••' : 'No live ticket';
  if (state) {
    state.textContent = live
      ? 'Host-issued ticket. Masked. Built-in OAuth value for the Spark agent.'
      : view.state === 'revoked'
        ? 'Expired. The next Spark session issues a new ticket.'
        : 'No Spark ticket yet.';
  }
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
    }
    list.appendChild(card);
  }
}

let cursorLive = false;

function paintIdeActions() {
  const discard = document.getElementById('btn-settings-mcp-discard') as HTMLButtonElement | null;
  if (discard) discard.hidden = !cursorLive;
}

function paintCursor(view: TicketView) {
  cursorLive = view.state === 'live' && Boolean(view.handle);
  setMcpServerBlock(formatIdeServerBlock(ticketChannel()));
  paintIdeActions();
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

export async function copyCursorIdeServerBlock() {
  const text = (document.getElementById('settings-mcp-server-block') as HTMLTextAreaElement | null)
    ?.value
    .trim();
  setResult('settings-result-mcp', '');
  if (!text) {
    setResult('settings-result-mcp', `Copy a ${IDE_LABELS[ticketChannel()] || 'Cursor'} server block first.`, true);
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

export async function discardIdeTicket() {
  const btn = document.getElementById('btn-settings-mcp-discard') as HTMLButtonElement | null;
  if (btn) btn.disabled = true;
  setResult('settings-result-mcp', '');
  try {
    await api.invoke('revoke_mcp_slot_ticket', { slot: ticketChannel() });
    cursorLive = false;
    setMcpServerBlock(formatIdeServerBlock(ticketChannel()));
    paintIdeActions();
    setResult(
      'settings-result-mcp',
      `Discarded ${IDE_LABELS[ticketChannel()] || 'Cursor'} authorization.`,
    );
  } catch (e) {
    setResult('settings-result-mcp', `Discard failed: ${errMessage(e, String(e))}`, true);
  } finally {
    if (btn) btn.disabled = false;
  }
}
