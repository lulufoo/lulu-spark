// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

vi.mock('../../frontend/js/host/api.js', () => ({
  fetchConfig: vi.fn(),
  setConfig: vi.fn(),
  inferGithubUserUrl: vi.fn(),
  checkWorkbenchKnowledgeRoot: vi.fn(),
  invoke: vi.fn(),
}));

import * as api from '../../frontend/js/host/api.js';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const indexHtml = readFileSync(join(root, 'frontend/index.html'), 'utf8');
const settingsDialogSrc = readFileSync(
  join(root, 'frontend/js/app-shell/settings-dialog.js'),
  'utf8',
);
const apiSrc = readFileSync(join(root, 'frontend/js/host/api.js'), 'utf8');

const MCP_PORT = 19876;
const HEALTH_MCP = `http://127.0.0.1:${MCP_PORT}/mcp/<scene_slot>`;
const CURSOR_IDE_URL = HEALTH_MCP.replace('<scene_slot>', 'cursor_ide');
const LIVE_HANDLE = 'ticket-live-reuse';
const NEW_HANDLE = 'ticket-opened-new';
const ROTATED_HANDLE = 'ticket-after-rotate';

function mountSettingsDom() {
  const markup = indexHtml.match(
    /<!-- Settings dialog -->[\s\S]*?<!-- Skills dialog -->/,
  )?.[0];
  if (!markup) throw new Error('settings dialog markup not found');
  document.body.innerHTML = markup.replace('<!-- Skills dialog -->', '');
}

function baseConfig(overrides = {}) {
  return {
    workbench_knowledge_root: '',
    knowledge_corpus_root: '',
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
    '../../frontend/js/app-shell/settings-dialog.js'
  );
  await openSettingsDialog({ panel: 'mcp' });
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
  it('adds an independent MCP nav item and panel beside Notes / Knowledge / Assistant / Sync', () => {
    const nav = indexHtml.match(/<nav id="settings-nav">([\s\S]*?)<\/nav>/)?.[1] ?? '';
    expect(nav).toMatch(/data-panel="directories"/);
    expect(nav).toMatch(/data-panel="knowledge"/);
    expect(nav).toMatch(/data-panel="llm"/);
    expect(nav).toMatch(/data-panel="github"/);
    expect(nav).toMatch(/data-panel="mcp"[^>]*>\s*MCP/);
    expect(indexHtml).toMatch(/id="settings-panels"/);
    expect(indexHtml).toMatch(/id="settings-panel-mcp"/);
    expect(indexHtml).toMatch(/id="settings-panel-directories"/);
    expect(indexHtml).toMatch(/id="settings-panel-knowledge"/);
    expect(indexHtml).toMatch(/id="settings-panel-llm"/);
    expect(indexHtml).toMatch(/id="settings-panel-github"/);
  });

  it('keeps generate/copy, revoke-any-slot, and cursor_ide rotate as separate controls', () => {
    const panel = indexHtml.match(
      /id="settings-panel-mcp"[\s\S]*?(?=<div id="settings-panel-|<div id="settings-dialog-footer")/,
    )?.[0];
    expect(panel, 'settings-panel-mcp markup').toBeTruthy();
    expect(panel).toMatch(/id="btn-settings-mcp-generate"/);
    expect(panel).toMatch(/id="settings-mcp-server-block"/);
    expect(panel).toMatch(/id="settings-mcp-revoke-slot"/);
    expect(panel).toMatch(/id="btn-settings-mcp-revoke"/);
    expect(panel).toMatch(/id="btn-settings-mcp-rotate"/);
    expect(panel).toMatch(/value="workbench"/);
    expect(panel).toMatch(/value="cursor_ide"/);
    expect(panel).not.toMatch(/id="btn-settings-mcp-revoke"[^>]*rotate/i);
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
    api.checkWorkbenchKnowledgeRoot.mockResolvedValue({ ok: true });
    api.invoke.mockResolvedValue({ handle: LIVE_HANDLE });
    await import('../../frontend/js/app-shell/settings-dialog.js');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('generate invokes issue_cursor_ide_ticket and Host decides reuse vs new', async () => {
    await openMcpPanel();
    api.invoke.mockResolvedValueOnce({ handle: LIVE_HANDLE });
    await clickId('btn-settings-mcp-generate');
    expect(invokedNames()).toEqual(['issue_cursor_ide_ticket']);
    expect(invokedNames()).not.toContain('rotate_cursor_ide_ticket');

    api.invoke.mockClear();
    api.invoke.mockResolvedValueOnce({ handle: NEW_HANDLE });
    await clickId('btn-settings-mcp-generate');
    expect(invokedNames()).toEqual(['issue_cursor_ide_ticket']);
    expect(invokedNames()).not.toContain('rotate_cursor_ide_ticket');
    expect(parseServerBlock(serverBlockText()).headers.Authorization).toBe(
      `Bearer ${NEW_HANDLE}`,
    );
  });

  it('generate/copy yields a full mcp.json server block on the health-template cursor_ide URL', async () => {
    await openMcpPanel();
    await clickId('btn-settings-mcp-generate');

    const displayed = parseServerBlock(serverBlockText());
    expect(displayed.url).toBe(CURSOR_IDE_URL);
    expect(displayed.url).toBe(`http://127.0.0.1:${MCP_PORT}/mcp/cursor_ide`);
    expect(displayed.headers.Authorization).toBe(`Bearer ${LIVE_HANDLE}`);

    expect(writeText).toHaveBeenCalled();
    const copied = parseServerBlock(writeText.mock.calls[0][0]);
    expect(copied).toEqual(displayed);
  });

  it('reuse, new issue, and rotate each still provide a full server block', async () => {
    await openMcpPanel();

    api.invoke.mockResolvedValueOnce({ handle: LIVE_HANDLE });
    await clickId('btn-settings-mcp-generate');
    expect(parseServerBlock(serverBlockText())).toMatchObject({
      url: CURSOR_IDE_URL,
      headers: { Authorization: `Bearer ${LIVE_HANDLE}` },
    });

    api.invoke.mockClear();
    api.invoke.mockResolvedValueOnce({ handle: NEW_HANDLE });
    await clickId('btn-settings-mcp-generate');
    expect(parseServerBlock(serverBlockText())).toMatchObject({
      url: CURSOR_IDE_URL,
      headers: { Authorization: `Bearer ${NEW_HANDLE}` },
    });

    api.invoke.mockClear();
    api.invoke.mockResolvedValueOnce({ handle: ROTATED_HANDLE });
    await clickId('btn-settings-mcp-rotate');
    expect(invokedNames()).toEqual(['rotate_cursor_ide_ticket']);
    expect(parseServerBlock(serverBlockText())).toMatchObject({
      url: CURSOR_IDE_URL,
      headers: { Authorization: `Bearer ${ROTATED_HANDLE}` },
    });
  });

  it.each(['workbench', 'cursor_ide'])(
    'revoke control voids slot %s via revoke_mcp_slot_ticket',
    async (slot) => {
      api.invoke.mockResolvedValue({ ok: true });
      await openMcpPanel();
      document.getElementById('settings-mcp-revoke-slot').value = slot;
      await clickId('btn-settings-mcp-revoke');
      expect(api.invoke).toHaveBeenCalledWith('revoke_mcp_slot_ticket', { slot });
      expect(invokedNames()).not.toContain('rotate_cursor_ide_ticket');
    },
  );

  it('keeps cursor_ide rotate off the revoke control', async () => {
    await openMcpPanel();
    const revoke = document.getElementById('btn-settings-mcp-revoke');
    const rotate = document.getElementById('btn-settings-mcp-rotate');
    expect(revoke).toBeTruthy();
    expect(rotate).toBeTruthy();
    expect(revoke.id).not.toBe(rotate.id);
    expect(revoke).not.toBe(rotate);

    api.invoke.mockResolvedValue({ ok: true });
    document.getElementById('settings-mcp-revoke-slot').value = 'cursor_ide';
    await clickId('btn-settings-mcp-revoke');
    expect(invokedNames()).toEqual(['revoke_mcp_slot_ticket']);
    expect(api.invoke).toHaveBeenCalledWith('revoke_mcp_slot_ticket', {
      slot: 'cursor_ide',
    });
  });

  it('never writes user mcp.json from Host or the panel', async () => {
    await openMcpPanel();
    await clickId('btn-settings-mcp-generate');
    api.invoke.mockClear();
    api.invoke.mockResolvedValueOnce({ handle: ROTATED_HANDLE });
    await clickId('btn-settings-mcp-rotate');
    api.invoke.mockClear();
    api.invoke.mockResolvedValueOnce({ ok: true });
    document.getElementById('settings-mcp-revoke-slot').value = 'workbench';
    await clickId('btn-settings-mcp-revoke');

    expect(api.setConfig).not.toHaveBeenCalled();
    expect(invokedNames().every((cmd) =>
      ['issue_cursor_ide_ticket', 'rotate_cursor_ide_ticket', 'revoke_mcp_slot_ticket'].includes(cmd),
    )).toBe(true);
    expect(settingsDialogSrc).not.toMatch(/writeFile|createWriteStream/);
    expect(resultText()).not.toMatch(/wrote|written|saved.*mcp\.json/i);
  });

  it('copied server block may include full Authorization and that is not a log', async () => {
    await openMcpPanel();
    await clickId('btn-settings-mcp-generate');
    const copied = writeText.mock.calls[0][0];
    expect(copied).toContain(`Bearer ${LIVE_HANDLE}`);
    expect(copied).toContain('Authorization');
    expect(logSurfaces()).not.toContain(LIVE_HANDLE);
    expect(logSurfaces()).not.toMatch(/Bearer\s+\S+/);
  });

  it('panel logs and debug prints omit ticket secrets and full Authorization', async () => {
    await openMcpPanel();
    await clickId('btn-settings-mcp-generate');
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
    await clickId('btn-settings-mcp-generate');
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
    document.getElementById('btn-settings-mcp-generate').click();
    await vi.waitFor(() => expect(resultText()).toMatch(/fail/i));
    expect(serverBlockText().trim()).toBe('');
    expect(writeText).not.toHaveBeenCalled();
    expect(invokedNames()).toEqual(['issue_cursor_ide_ticket']);
    expect(resultText()).not.toMatch(/mcp\.json/i);

    api.invoke.mockClear();
    api.invoke.mockRejectedValueOnce(new Error('rotate failed'));
    document.getElementById('btn-settings-mcp-rotate').click();
    await vi.waitFor(() => expect(resultText()).toMatch(/fail/i));
    expect(serverBlockText().trim()).toBe('');
    expect(invokedNames()).toEqual(['rotate_cursor_ide_ticket']);
    expect(resultText()).not.toMatch(/wrote|written|saved/i);

    api.invoke.mockClear();
    api.invoke.mockRejectedValueOnce(new Error('revoke failed'));
    document.getElementById('settings-mcp-revoke-slot').value = 'cursor_ide';
    document.getElementById('btn-settings-mcp-revoke').click();
    await vi.waitFor(() => expect(resultText()).toMatch(/fail/i));
    expect(invokedNames()).toEqual(['revoke_mcp_slot_ticket']);
    expect(invokedNames()).not.toContain('rotate_cursor_ide_ticket');
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
