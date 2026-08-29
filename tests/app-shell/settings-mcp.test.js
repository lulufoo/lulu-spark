// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { createElement } from 'react';
import { readShellHtml } from '../helpers/read-frontend-js.js';
import { renderToHtml } from '../../frontend/src/island.ts';
import { SettingsDialog } from '../../frontend/src/app-shell/ui/settings/dialog.tsx';
import { listFrontendSourceFiles, readHostApiSource } from '../helpers/read-frontend-js.js';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchConfig: vi.fn(),
  setConfig: vi.fn(),
  inferGithubUserUrl: vi.fn(),
  checkWorkbenchRoot: vi.fn(),
  invoke: vi.fn(),
}));

import * as api from '../../frontend/src/host/api.ts';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const appShellSrc = listFrontendSourceFiles(join(root, 'frontend/src/app-shell'))
  .sort()
  .map((abs) => readFileSync(abs, 'utf8'))
  .join('\n');
const indexHtml = [readShellHtml(), appShellSrc].join('\n');
const settingsDialogSrc = appShellSrc;
const apiSrc = readHostApiSource();

const MCP_PORT = 19876;
const HEALTH_MCP = `http://127.0.0.1:${MCP_PORT}/mcp/<scene_slot>`;
const CURSOR_IDE_URL = HEALTH_MCP.replace('<scene_slot>', 'cursor_ide');
const LIVE_HANDLE = 'ticket-live-reuse';
const ROTATED_HANDLE = 'ticket-after-rotate';
const TOOLS_SNAPSHOT = {
  channels: ['workbench', 'cursor_ide', 'mobile'],
  groups: [
    {
      id: 'notes',
      label: 'Notes',
      tools: [
        { name: 'create_note', description: 'Create a note' },
        { name: 'get_notes_catalog', description: 'Catalog' },
      ],
    },
    {
      id: 'todo',
      label: 'Todo',
      tools: [{ name: 'list_todo_tasks', description: 'List todos' }],
    },
  ],
  enabled: {
    workbench: ['create_note', 'get_notes_catalog', 'list_todo_tasks'],
    cursor_ide: ['create_note', 'get_notes_catalog', 'list_todo_tasks'],
    mobile: ['get_notes_catalog', 'list_todo_tasks'],
  },
};

function mountSettingsDom() {
  document.body.innerHTML = renderToHtml(createElement(SettingsDialog));
}

function baseConfig(overrides = {}) {
  return {
    workbench_root: '',
    knowledge_root: '',
    github_user_url: '',
    workbench_github_repo_url: '',
    has_github_token: false,
    assistant_engine: 'host',
    has_host_key: false,
    mcp_port: MCP_PORT,
    llm: {
      platform: 'glm',
      base_url: 'https://open.bigmodel.cn/api/paas/v4',
      model: '',
    },
    ...overrides,
  };
}

function parseServerBlock(text) {
  const json = JSON.parse(text);
  if (json && typeof json === 'object' && json.url && json.headers) return json;
  if (json?.mcpServers && typeof json.mcpServers === 'object') {
    const first = Object.values(json.mcpServers)[0];
    if (first && first.url && first.headers) return first;
  }
  const nested = Object.values(json).find(
    (value) => value && typeof value === 'object' && value.url && value.headers,
  );
  if (nested) return nested;
  throw new Error(`mcp.json server block not found in: ${text}`);
}

function serverBlockText() {
  const el = document.getElementById('settings-mcp-server-block');
  if (!el) throw new Error('settings-mcp-server-block missing');
  return el.value ?? el.textContent ?? '';
}

function resultText() {
  return document.getElementById('settings-result-mcp')?.textContent ?? '';
}

function logSurfaces() {
  return [
    resultText(),
    document.getElementById('settings-mcp-log')?.textContent ?? '',
    ...consoleSpies.flatMap((spy) => spy.mock.calls.map((args) => args.map(String).join(' '))),
  ].join('\n');
}

function invokedNames() {
  return api.invoke.mock.calls.map(([cmd]) => cmd);
}

async function openMcpPanel() {
  const { openSettingsDialog } = await import(
    '../../frontend/src/app-shell/commands/settings/dialog.ts'
  );
  await openSettingsDialog({ panel: 'mcp' });
  api.invoke.mockClear();
}

async function clickId(id) {
  const el = document.getElementById(id);
  if (!el) throw new Error(`${id} missing`);
  el.click();
  await vi.waitFor(() => expect(api.invoke).toHaveBeenCalled());
  await vi.waitFor(() => expect(resultText().length).toBeGreaterThan(0));
}

const consoleSpies = [];
let writeText;

describe('Settings MCP panel markup', () => {
  it('adds an independent MCP nav item and panel beside Workbench / Knowledge / Assistant / Sync', () => {
    const nav = indexHtml.match(/<nav id="settings-nav">([\s\S]*?)<\/nav>/)?.[1] ?? '';
    expect(nav).toMatch(/data-panel="workbench"/);
    expect(nav).toMatch(/data-panel="knowledge"/);
    expect(nav).toMatch(/data-panel="llm"/);
    expect(nav).toMatch(/data-panel="github"/);
    expect(nav).toMatch(/data-panel="mcp"[^>]*>\s*MCP/);
    expect(indexHtml).toMatch(/id="settings-panels"/);
    expect(indexHtml).toMatch(/id="settings-panel-mcp"/);
    expect(indexHtml).toMatch(/id="settings-panel-workbench"/);
    expect(indexHtml).toMatch(/id="settings-panel-knowledge"/);
    expect(indexHtml).toMatch(/id="settings-panel-llm"/);
    expect(indexHtml).toMatch(/id="settings-panel-github"/);
  });

  it('keeps one state-dependent cursor ticket button plus Copy', () => {
    const panel = indexHtml.match(
      /id="settings-panel-mcp"[\s\S]*?(?=<div id="settings-panel-|<div id="settings-dialog-footer")/,
    )?.[0];
    expect(panel, 'settings-panel-mcp markup').toBeTruthy();
    expect(panel).toMatch(/data-tab="tickets"/);
    expect(panel).toMatch(/data-tab="tools"/);
    expect(panel).toMatch(/id="settings-tab-mcp-tickets"/);
    expect(panel).toMatch(/id="settings-tab-mcp-tools"/);
    expect(indexHtml).toMatch(/id="settings-mcp-ticket-channel"/);
    expect(indexHtml).toMatch(/id="btn-settings-mcp-primary"/);
    expect(indexHtml).toMatch(/id="btn-settings-mcp-copy"/);
    expect(indexHtml).toMatch(/id="settings-mcp-server-block"/);
    expect(indexHtml).not.toMatch(/id="btn-settings-mcp-generate"/);
    expect(indexHtml).not.toMatch(/id="btn-settings-mcp-rotate"/);
    expect(indexHtml).toMatch(/id="btn-settings-mcp-workbench-expire"/);
    expect(indexHtml).toMatch(/id="settings-mcp-device-list"/);
    expect(indexHtml).not.toMatch(/id="settings-mcp-revoke-slot"/);
    expect(indexHtml).not.toMatch(/id="btn-settings-mcp-revoke"/);
    expect(indexHtml).toMatch(/id="settings-mcp-channel"/);
    expect(indexHtml).toMatch(/id="settings-mcp-tool-groups"/);
    expect(indexHtml).toMatch(/id="btn-settings-mcp-tools-select-all"/);
    expect(indexHtml).toMatch(/id="btn-settings-mcp-tools-deselect-all"/);
    expect(indexHtml).not.toMatch(/id="btn-settings-mcp-tools-save"/);
    expect(indexHtml).toMatch(/value="mobile"/);
    expect(panel).not.toMatch(/PKCE|authorization code|Cursor Connect|system notification/i);
  });

  it('exposes invoke on the frontend api module used by Settings', () => {
    expect(apiSrc).toMatch(/export async function invoke\b/);
  });

  it('includes this file in npm test', () => {
    const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });
});

describe('Settings MCP panel actions', () => {
  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    consoleSpies.length = 0;
    for (const level of ['log', 'debug', 'info', 'warn', 'error']) {
      consoleSpies.push(vi.spyOn(console, level).mockImplementation(() => {}));
    }
    writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input) => {
        const url = String(input);
        if (url.includes('/health')) {
          return {
            ok: true,
            json: async () => ({ ok: true, mcp: HEALTH_MCP }),
          };
        }
        throw new Error(`unexpected fetch: ${url}`);
      }),
    );
    mountSettingsDom();
    api.fetchConfig.mockResolvedValue(baseConfig());
    api.setConfig.mockResolvedValue(baseConfig());
    api.inferGithubUserUrl.mockResolvedValue({});
    api.checkWorkbenchRoot.mockResolvedValue({ ok: true });
    api.invoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'get_mcp_channel_tools') return structuredClone(TOOLS_SNAPSHOT);
      if (cmd === 'set_mcp_channel_tools') return { ok: true };
      if (cmd === 'get_mcp_ticket_view') {
        const channel = args?.channel || 'cursor_ide';
        if (channel === 'workbench') {
          return { channel: 'workbench', state: 'live', hint: '••••ab12' };
        }
        if (channel === 'mobile') {
          return {
            channel: 'mobile',
            devices: [
              { device_id: 'phone-1', device_label: 'Pixel', revoked: false, hint: '••••c4e2' },
              { device_id: 'phone-2', device_label: 'Old', revoked: true, hint: '••••99aa' },
            ],
          };
        }
        return { channel: 'cursor_ide', state: 'none' };
      }
      if (cmd === 'revoke_mcp_device_ticket' || cmd === 'revoke_mcp_slot_ticket') {
        return { ok: true };
      }
      return { handle: LIVE_HANDLE };
    });
    await import('../../frontend/src/app-shell/commands/settings/dialog.ts');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('empty ticket Generate issues; after live, same button Refresh rotates', async () => {
    await openMcpPanel();
    const primary = document.getElementById('btn-settings-mcp-primary');
    expect(primary.textContent).toBe('Generate');
    expect(primary.dataset.mcpTicketAction).toBe('generate');
    expect(document.getElementById('btn-settings-mcp-copy').disabled).toBe(true);

    api.invoke.mockResolvedValueOnce({ handle: LIVE_HANDLE });
    await clickId('btn-settings-mcp-primary');
    expect(invokedNames()).toEqual(['issue_cursor_ide_ticket']);
    expect(invokedNames()).not.toContain('rotate_cursor_ide_ticket');
    expect(primary.textContent).toBe('Refresh');
    expect(primary.dataset.mcpTicketAction).toBe('refresh');
    expect(document.getElementById('btn-settings-mcp-copy').disabled).toBe(false);

    api.invoke.mockClear();
    api.invoke.mockResolvedValueOnce({ handle: ROTATED_HANDLE });
    await clickId('btn-settings-mcp-primary');
    expect(invokedNames()).toEqual(['rotate_cursor_ide_ticket']);
    expect(parseServerBlock(serverBlockText()).headers.Authorization).toBe(
      `Bearer ${ROTATED_HANDLE}`,
    );
  });

  it('live cursor view paints Refresh and primary then rotates', async () => {
    const fallback = api.invoke.getMockImplementation();
    api.invoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'get_mcp_ticket_view' && (args?.channel || 'cursor_ide') === 'cursor_ide') {
        return { channel: 'cursor_ide', state: 'live', handle: LIVE_HANDLE };
      }
      return fallback(cmd, args);
    });
    await openMcpPanel();
    const primary = document.getElementById('btn-settings-mcp-primary');
    expect(primary.textContent).toBe('Refresh');
    expect(document.getElementById('btn-settings-mcp-copy').disabled).toBe(false);
    expect(parseServerBlock(serverBlockText()).headers.Authorization).toBe(
      `Bearer ${LIVE_HANDLE}`,
    );

    api.invoke.mockClear();
    api.invoke.mockResolvedValueOnce({ handle: ROTATED_HANDLE });
    await clickId('btn-settings-mcp-primary');
    expect(invokedNames()).toEqual(['rotate_cursor_ide_ticket']);
  });

  it('generate/copy yields a full mcp.json server block on the health-template cursor_ide URL', async () => {
    await openMcpPanel();
    await clickId('btn-settings-mcp-primary');

    const displayed = parseServerBlock(serverBlockText());
    expect(displayed.url).toBe(CURSOR_IDE_URL);
    expect(displayed.url).toBe(`http://127.0.0.1:${MCP_PORT}/mcp/cursor_ide`);
    expect(displayed.headers.Authorization).toBe(`Bearer ${LIVE_HANDLE}`);

    expect(writeText).toHaveBeenCalled();
    const copied = parseServerBlock(writeText.mock.calls[0][0]);
    expect(copied).toEqual(displayed);
  });

  it('Copy pastes the live block without issuing or rotating', async () => {
    await openMcpPanel();
    await clickId('btn-settings-mcp-primary');
    writeText.mockClear();
    api.invoke.mockClear();
    document.getElementById('btn-settings-mcp-copy').click();
    await vi.waitFor(() => expect(writeText).toHaveBeenCalled());
    expect(invokedNames()).toEqual([]);
    expect(parseServerBlock(writeText.mock.calls[0][0]).headers.Authorization).toBe(
      `Bearer ${LIVE_HANDLE}`,
    );
  });

  it('workbench ticket is masked and expire voids the workbench slot', async () => {
    await openMcpPanel();
    document.getElementById('settings-mcp-ticket-channel').value = 'workbench';
    document.getElementById('settings-mcp-ticket-channel').dispatchEvent(new Event('change'));
    await vi.waitFor(() =>
      expect(document.getElementById('settings-mcp-workbench-mask')?.textContent).toBe('••••ab12'),
    );
    expect(document.getElementById('settings-mcp-tickets-workbench').hidden).toBe(false);
    expect(document.getElementById('settings-mcp-tickets-cursor_ide').hidden).toBe(true);
    expect(document.getElementById('settings-mcp-workbench-mask')?.textContent).not.toContain(
      LIVE_HANDLE,
    );
    api.invoke.mockClear();
    await clickId('btn-settings-mcp-workbench-expire');
    expect(api.invoke).toHaveBeenCalledWith('revoke_mcp_slot_ticket', { slot: 'workbench' });
    expect(invokedNames()).not.toContain('rotate_cursor_ide_ticket');
  });

  it('mobile lists bound devices and expire voids that device ticket', async () => {
    await openMcpPanel();
    document.getElementById('settings-mcp-ticket-channel').value = 'mobile';
    document.getElementById('settings-mcp-ticket-channel').dispatchEvent(new Event('change'));
    await vi.waitFor(() =>
      expect(document.querySelector('[data-mcp-device-expire="phone-1"]')).toBeTruthy(),
    );
    expect(document.getElementById('settings-mcp-device-list')?.textContent).toMatch(/Pixel/);
    expect(document.getElementById('settings-mcp-device-list')?.textContent).toMatch(/••••c4e2/);
    expect(document.querySelector('[data-mcp-device-expire="phone-2"]')).toBeNull();
    api.invoke.mockClear();
    document.querySelector('[data-mcp-device-expire="phone-1"]').click();
    await vi.waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith('revoke_mcp_device_ticket', { deviceId: 'phone-1' }),
    );
    expect(invokedNames()).not.toContain('rotate_cursor_ide_ticket');
  });

  it('keeps cursor_ide refresh off the workbench expire control', async () => {
    await openMcpPanel();
    const expire = document.getElementById('btn-settings-mcp-workbench-expire');
    const primary = document.getElementById('btn-settings-mcp-primary');
    expect(expire).toBeTruthy();
    expect(primary).toBeTruthy();
    expect(expire.id).not.toBe(primary.id);
    expect(document.getElementById('btn-settings-mcp-rotate')).toBeNull();
    expect(document.getElementById('btn-settings-mcp-revoke')).toBeNull();
  });

  it('never writes user mcp.json from Host or the panel', async () => {
    await openMcpPanel();
    await clickId('btn-settings-mcp-primary');
    api.invoke.mockClear();
    api.invoke.mockResolvedValueOnce({ handle: ROTATED_HANDLE });
    await clickId('btn-settings-mcp-primary');
    api.invoke.mockClear();
    document.getElementById('settings-mcp-ticket-channel').value = 'workbench';
    document.getElementById('settings-mcp-ticket-channel').dispatchEvent(new Event('change'));
    await vi.waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith('get_mcp_ticket_view', { channel: 'workbench' }),
    );
    api.invoke.mockClear();
    await clickId('btn-settings-mcp-workbench-expire');

    expect(api.setConfig).not.toHaveBeenCalled();
    expect(invokedNames().every((cmd) =>
      [
        'issue_cursor_ide_ticket',
        'rotate_cursor_ide_ticket',
        'revoke_mcp_slot_ticket',
        'get_mcp_ticket_view',
      ].includes(cmd),
    )).toBe(true);
    expect(settingsDialogSrc).not.toMatch(/writeFile|createWriteStream/);
    expect(resultText()).not.toMatch(/wrote|written|saved.*mcp\.json/i);
  });

  it('copied server block may include full Authorization and that is not a log', async () => {
    await openMcpPanel();
    await clickId('btn-settings-mcp-primary');
    const copied = writeText.mock.calls[0][0];
    expect(copied).toContain(`Bearer ${LIVE_HANDLE}`);
    expect(copied).toContain('Authorization');
    expect(logSurfaces()).not.toContain(LIVE_HANDLE);
    expect(logSurfaces()).not.toMatch(/Bearer\s+\S+/);
  });

  it('panel logs and debug prints omit ticket secrets and full Authorization', async () => {
    await openMcpPanel();
    await clickId('btn-settings-mcp-primary');
    expect(logSurfaces()).not.toContain(LIVE_HANDLE);
    expect(logSurfaces()).not.toMatch(/Authorization:\s*Bearer/i);
    expect(settingsDialogSrc).not.toMatch(/console\.\w+\([^)]*Authorization/);
    expect(settingsDialogSrc).not.toMatch(/console\.\w+\([^)]*handle/);
  });

  it('keeps the generated server block when clipboard write is denied', async () => {
    writeText.mockRejectedValueOnce(
      new Error(
        'The request is not allowed by the user agent or the platform in the current context, possibly because the user denied permission.',
      ),
    );
    await openMcpPanel();
    await clickId('btn-settings-mcp-primary');
    expect(parseServerBlock(serverBlockText()).headers.Authorization).toBe(
      `Bearer ${LIVE_HANDLE}`,
    );
    expect(resultText()).toMatch(/Clipboard copy was blocked/i);
    expect(resultText()).not.toMatch(/^Generate failed/i);
    expect(logSurfaces()).not.toContain(LIVE_HANDLE);
  });

  it('does not pretend mcp.json was written or silently rotate when commands fail', async () => {
    await openMcpPanel();
    api.invoke.mockRejectedValueOnce(new Error('issue failed'));
    document.getElementById('btn-settings-mcp-primary').click();
    await vi.waitFor(() => expect(resultText()).toMatch(/fail/i));
    expect(serverBlockText().trim()).toBe('');
    expect(writeText).not.toHaveBeenCalled();
    expect(invokedNames()).toEqual(['issue_cursor_ide_ticket']);
    expect(document.getElementById('btn-settings-mcp-primary').textContent).toBe('Generate');
    expect(resultText()).not.toMatch(/mcp\.json/i);

    api.invoke.mockClear();
    api.invoke.mockResolvedValueOnce({ handle: LIVE_HANDLE });
    await clickId('btn-settings-mcp-primary');
    expect(invokedNames()).toEqual(['issue_cursor_ide_ticket']);

    api.invoke.mockClear();
    api.invoke.mockRejectedValueOnce(new Error('rotate failed'));
    document.getElementById('btn-settings-mcp-primary').click();
    await vi.waitFor(() => expect(resultText()).toMatch(/Refresh failed/i));
    expect(invokedNames()).toEqual(['rotate_cursor_ide_ticket']);
    expect(parseServerBlock(serverBlockText()).headers.Authorization).toBe(
      `Bearer ${LIVE_HANDLE}`,
    );
    expect(document.getElementById('btn-settings-mcp-primary').textContent).toBe('Refresh');
    expect(resultText()).not.toMatch(/wrote|written|saved/i);

    api.invoke.mockClear();
    document.getElementById('settings-mcp-ticket-channel').value = 'workbench';
    document.getElementById('settings-mcp-ticket-channel').dispatchEvent(new Event('change'));
    await vi.waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith('get_mcp_ticket_view', { channel: 'workbench' }),
    );
    api.invoke.mockClear();
    api.invoke.mockRejectedValueOnce(new Error('expire failed'));
    document.getElementById('btn-settings-mcp-workbench-expire').click();
    await vi.waitFor(() => expect(resultText()).toMatch(/fail/i));
    expect(invokedNames()).toEqual(['revoke_mcp_slot_ticket']);
    expect(invokedNames()).not.toContain('rotate_cursor_ide_ticket');
  });

  it('keeps ticket controls and tool checkboxes on separate MCP tabs', async () => {
    await openMcpPanel();
    const tickets = document.getElementById('settings-tab-mcp-tickets');
    const tools = document.getElementById('settings-tab-mcp-tools');
    expect(tickets.classList.contains('active')).toBe(true);
    expect(tools.classList.contains('active')).toBe(false);
    expect(tickets.contains(document.getElementById('btn-settings-mcp-primary'))).toBe(true);
    expect(tickets.contains(document.getElementById('settings-mcp-channel'))).toBe(false);
    expect(tools.contains(document.getElementById('settings-mcp-channel'))).toBe(true);
    document.querySelector('#settings-panel-mcp .settings-tab[data-tab="tools"]').click();
    expect(tools.classList.contains('active')).toBe(true);
    expect(tickets.classList.contains('active')).toBe(false);
  });

  it('loads grouped tool checkboxes per /mcp channel', async () => {
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog({ panel: 'mcp', tab: 'tools' });
    expect(api.invoke).toHaveBeenCalledWith('get_mcp_channel_tools');
    expect(document.getElementById('settings-tab-mcp-tools').classList.contains('active')).toBe(true);
    expect(document.querySelector('[data-mcp-tool="create_note"]')).toBeTruthy();
    expect(document.querySelector('[data-mcp-tool="create_note"]').checked).toBe(true);
    document.getElementById('settings-mcp-channel').value = 'mobile';
    document.getElementById('settings-mcp-channel').dispatchEvent(new Event('change'));
    expect(document.querySelector('[data-mcp-tool="create_note"]').checked).toBe(false);
    expect(document.querySelector('[data-mcp-tool="get_notes_catalog"]').checked).toBe(true);
  });

  it('writes the selected channel when a tool checkbox changes', async () => {
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog({ panel: 'mcp', tab: 'tools' });
    api.invoke.mockClear();
    document.getElementById('settings-mcp-channel').value = 'mobile';
    document.getElementById('settings-mcp-channel').dispatchEvent(new Event('change'));
    const createNote = document.querySelector('[data-mcp-tool="create_note"]');
    createNote.checked = true;
    createNote.dispatchEvent(new Event('change', { bubbles: true }));
    await vi.waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith('set_mcp_channel_tools', {
        channel: 'mobile',
        enabled: ['create_note', 'get_notes_catalog', 'list_todo_tasks'],
      }),
    );
    await vi.waitFor(() =>
      expect(document.getElementById('settings-result-mcp-tools')?.textContent).toMatch(
        /Updated \/mcp\/mobile tools/,
      ),
    );
    expect(resultText()).toBe('');
    expect(api.setConfig).not.toHaveBeenCalled();
  });

  it('All and None write the current channel', async () => {
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog({ panel: 'mcp', tab: 'tools' });
    document.getElementById('settings-mcp-channel').value = 'mobile';
    document.getElementById('settings-mcp-channel').dispatchEvent(new Event('change'));
    api.invoke.mockClear();
    document.getElementById('btn-settings-mcp-tools-select-all').click();
    await vi.waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith('set_mcp_channel_tools', {
        channel: 'mobile',
        enabled: ['create_note', 'get_notes_catalog', 'list_todo_tasks'],
      }),
    );
    api.invoke.mockClear();
    document.getElementById('btn-settings-mcp-tools-deselect-all').click();
    await vi.waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith('set_mcp_channel_tools', {
        channel: 'mobile',
        enabled: [],
      }),
    );
  });

  it('paints every catalog tool checked when a channel has no saved list', async () => {
    const fallback = api.invoke.getMockImplementation();
    api.invoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'get_mcp_channel_tools') {
        return { channels: TOOLS_SNAPSHOT.channels, groups: TOOLS_SNAPSHOT.groups };
      }
      return fallback(cmd, args);
    });
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog({ panel: 'mcp', tab: 'tools' });
    const boxes = [...document.querySelectorAll('#settings-mcp-tool-groups input[data-mcp-tool]')];
    expect(boxes.map((el) => el.dataset.mcpTool)).toEqual([
      'create_note',
      'get_notes_catalog',
      'list_todo_tasks',
    ]);
    expect(boxes.every((el) => el.checked)).toBe(true);
  });

  it('keeps an explicit empty allowlist as all unchecked', async () => {
    const fallback = api.invoke.getMockImplementation();
    api.invoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'get_mcp_channel_tools') {
        return {
          ...TOOLS_SNAPSHOT,
          enabled: { workbench: [], cursor_ide: [], mobile: [] },
        };
      }
      return fallback(cmd, args);
    });
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog({ panel: 'mcp', tab: 'tools' });
    const boxes = [...document.querySelectorAll('#settings-mcp-tool-groups input[data-mcp-tool]')];
    expect(boxes.length).toBe(3);
    expect(boxes.every((el) => !el.checked)).toBe(true);
  });

  it('does not persist when the tool list is empty', async () => {
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog({ panel: 'mcp', tab: 'tools' });
    await new Promise((resolve) => requestAnimationFrame(resolve));
    document.getElementById('settings-mcp-tool-groups').replaceChildren();
    expect(document.querySelector('#settings-mcp-tool-groups input[data-mcp-tool]')).toBeNull();
    api.invoke.mockClear();
    document.getElementById('settings-mcp-tool-groups').dispatchEvent(new Event('change', { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(invokedNames()).not.toContain('set_mcp_channel_tools');
  });

  it('does not walk Cursor Connect, system notifications, auth codes, discovery, or PKCE', () => {
    expect(settingsDialogSrc).not.toMatch(
      /PKCE|authorization_code|authorization code|Cursor Connect|cursor_connect|system notification/i,
    );
    expect(indexHtml).not.toMatch(
      /PKCE|authorization code|Cursor Connect|system notification/i,
    );
  });
});
