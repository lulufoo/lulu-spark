// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { renderToHtml } from '../../frontend/src/island.ts';
import { SettingsDialog } from '../../frontend/src/app-shell/ui/settings/dialog.tsx';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchConfig: vi.fn(),
  setConfig: vi.fn(),
  invoke: vi.fn(),
  fetchProjects: vi.fn(),
  fetchHidePatterns: vi.fn(),
}));

import * as api from '../../frontend/src/host/api.ts';

const rootDir = join(dirname(fileURLToPath(import.meta.url)), '../..');

function readRel(rel) {
  return readFileSync(join(rootDir, rel), 'utf8');
}

function baseConfig(network) {
  return {
    spark_root: '',
    assistant_engine: 'host',
    mcp_port: 19876,
    theme: 'system',
    network,
    llm: {},
  };
}

function applied(network) {
  return {
    ...baseConfig(network),
    network: {
      ...network,
      effective: {
        generation: 2,
        plan: {
          http: null,
          https: { host_port: '127.0.0.1:7890', source: 'system' },
        },
      },
    },
  };
}

async function renderAndOpen() {
  document.body.innerHTML = renderToHtml(createElement(SettingsDialog));
  const { openSettingsDialog } = await import(
    '../../frontend/src/app-shell/commands/settings/dialog.ts'
  );
  await openSettingsDialog();
}

describe('Settings Network panel', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    api.fetchConfig.mockResolvedValue(baseConfig({ use_system_proxy: false }));
    api.setConfig.mockResolvedValue(baseConfig({ use_system_proxy: false }));
    api.fetchProjects.mockResolvedValue({ projects: [] });
    api.fetchHidePatterns.mockResolvedValue({ patterns: [] });
    api.invoke.mockResolvedValue({ channels: [], groups: [], enabled: {} });
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('adds a Network nav item after Theme with one system-proxy switch', async () => {
    await renderAndOpen();
    const nav = document.getElementById('settings-nav');
    const items = [...nav.querySelectorAll('.settings-nav-item')].map((el) => el.dataset.panel);
    expect(items.indexOf('theme')).toBeGreaterThanOrEqual(0);
    expect(items.indexOf('network')).toBe(items.indexOf('theme') + 1);

    const chromeSrc = readRel('frontend/src/app-shell/ui/settings/chrome.tsx');
    expect(chromeSrc).toMatch(/id=["']settings-panel-network["']/);
    expect(document.getElementById('settings-system-proxy')).not.toBeNull();
    expect(document.getElementById('settings-network-result')).not.toBeNull();
  });

  it('states the scope: system proxy only, env proxies keep working, no TUN control', async () => {
    await renderAndOpen();
    const hint = document.querySelector('#settings-panel-network .settings-field-hint');
    const copy = String(hint?.textContent ?? '');
    expect(copy).toMatch(/HTTP\/HTTPS proxy/);
    expect(copy).toMatch(/environment variables keep working/);
    expect(copy).toMatch(/does not control a TUN device/);
    expect(copy).toMatch(/without a restart/);
  });

  it('loads the persisted toggle and shows the applied plan snapshot', async () => {
    api.fetchConfig.mockResolvedValue(applied({ use_system_proxy: true }));
    await renderAndOpen();
    const toggle = document.getElementById('settings-system-proxy');
    expect(toggle.checked).toBe(true);
    const result = document.getElementById('settings-network-result').textContent;
    expect(result).toContain('Applied without restart');
    expect(result).toContain('https via system proxy 127.0.0.1:7890');
    expect(result).toContain('http direct');
  });

  it('saves the toggle and paints the effective snapshot, not an unconditional success', async () => {
    api.setConfig.mockResolvedValue(applied({ use_system_proxy: true }));
    await renderAndOpen();
    const { saveSystemProxySelection } = await import(
      '../../frontend/src/app-shell/commands/settings/network.ts'
    );
    await saveSystemProxySelection(true);

    expect(api.setConfig).toHaveBeenCalledWith({ network: { use_system_proxy: true } });
    expect(document.getElementById('settings-system-proxy').checked).toBe(true);
    const result = document.getElementById('settings-network-result').textContent;
    expect(result).toContain('system proxy on');
    expect(result).toContain('https via system proxy 127.0.0.1:7890');
  });

  it('reverts the toggle and shows the error when the save fails', async () => {
    api.fetchConfig.mockResolvedValue(applied({ use_system_proxy: true }));
    api.setConfig.mockRejectedValue(new Error('config write failed'));
    await renderAndOpen();
    const { saveSystemProxySelection } = await import(
      '../../frontend/src/app-shell/commands/settings/network.ts'
    );
    await saveSystemProxySelection(false);

    expect(document.getElementById('settings-system-proxy').checked).toBe(true);
    const result = document.getElementById('settings-network-result');
    expect(result.textContent).toContain('Not saved: config write failed');
  });

  it('reflects an off snapshot as direct connections', async () => {
    api.fetchConfig.mockResolvedValue(applied({ use_system_proxy: false }));
    await renderAndOpen();
    const result = document.getElementById('settings-network-result').textContent;
    expect(result).toContain('system proxy off');
  });
});
