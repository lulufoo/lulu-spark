// @ts-nocheck — settings DOM wiring stays unchecked like checkJs:false.
import * as api from '../../../host/api.ts';
import { setResult, store } from '../../state/settings/store.ts';

export function cursorIdeServerUrl() {
  return `http://127.0.0.1:${store.mcpPort}/mcp/cursor_ide`;
}

export function formatCursorIdeServerBlock(handle) {
  return JSON.stringify(
    {
      url: cursorIdeServerUrl(),
      headers: { Authorization: `Bearer ${handle}` },
    },
    null,
    2,
  );
}

export function setMcpServerBlock(text) {
  const el = document.getElementById('settings-mcp-server-block');
  if (el) el.value = text;
}

export function clearMcpServerBlock() {
  setMcpServerBlock('');
}

async function copyServerBlock(text) {
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

async function issueOrRotateCursorIdeBlock(cmd, copiedMessage, failLabel) {
  setResult('settings-result-mcp', '');
  try {
    const resp = await api.invoke(cmd);
    const issued = resp?.handle;
    if (!issued) throw new Error('Ticket command failed');
    const text = formatCursorIdeServerBlock(issued);
    setMcpServerBlock(text);
    const copied = await copyServerBlock(text);
    const doneLabel = failLabel === 'Rotate' ? 'Rotated' : 'Generated';
    setResult(
      'settings-result-mcp',
      copied
        ? copiedMessage
        : `${doneLabel} cursor_ide server block. Clipboard copy was blocked — copy the block from the field.`,
    );
  } catch (e) {
    clearMcpServerBlock();
    setResult('settings-result-mcp', `${failLabel} failed: ${e.message || String(e)}`, true);
  }
}

export async function generateCursorIdeServerBlock() {
  const btn = document.getElementById('btn-settings-mcp-generate');
  if (btn) btn.disabled = true;
  try {
    await issueOrRotateCursorIdeBlock(
      'issue_cursor_ide_ticket',
      'Generated and copied cursor_ide server block.',
      'Generate',
    );
  } finally {
    if (btn) btn.disabled = false;
  }
}

export async function rotateCursorIdeTicket() {
  const btn = document.getElementById('btn-settings-mcp-rotate');
  if (btn) btn.disabled = true;
  try {
    await issueOrRotateCursorIdeBlock(
      'rotate_cursor_ide_ticket',
      'Rotated and copied cursor_ide server block.',
      'Rotate',
    );
  } finally {
    if (btn) btn.disabled = false;
  }
}

export async function revokeMcpSlotTicket() {
  const btn = document.getElementById('btn-settings-mcp-revoke');
  const slot = document.getElementById('settings-mcp-revoke-slot')?.value;
  if (btn) btn.disabled = true;
  setResult('settings-result-mcp', '');
  try {
    await api.invoke('revoke_mcp_slot_ticket', { slot });
    setResult('settings-result-mcp', `Revoked ${slot} ticket.`);
  } catch (e) {
    setResult('settings-result-mcp', `Revoke failed: ${e.message || String(e)}`, true);
  } finally {
    if (btn) btn.disabled = false;
  }
}
