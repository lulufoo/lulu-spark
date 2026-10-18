// @vitest-environment jsdom
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { renderToHtml } from '../../frontend/src/island.ts';
import { SettingsDialog } from '../../frontend/src/app-shell/ui/settings/dialog.tsx';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchConfig: vi.fn(async () => ({ workspaces_root: '/data' })),
  setConfig: vi.fn(),
  invoke: vi.fn(async () => ({ channels: [], groups: [], enabled: {} })),
  fetchProjects: vi.fn(async () => ({ projects: [] })),
  fetchDisabledWorkspaces: vi.fn(async () => ({ workspaces: [] })),
  fetchHidePatterns: vi.fn(async () => ({ patterns: [] })),
}));

import * as api from '../../frontend/src/host/api.ts';
import { state } from '../../frontend/src/host/state.ts';

const activeTab = (panel) =>
  document.querySelector(`#settings-panel-${panel} .settings-tab.active`)?.dataset.tab;
const click = (selector) => document.querySelector(selector).click();
const flush = () => new Promise((r) => setTimeout(r, 0));

describe('Settings tabs open on Directory', () => {
  beforeEach(async () => {
    vi.resetModules();
    state.workspaces.list = [
      { workspace_id: 'default', workspace_name: 'Default', view_mode: 'directory', is_default: true },
      { workspace_id: 'ws2', workspace_name: 'Wiki', view_mode: 'timeline', is_default: false },
    ];
    document.body.innerHTML = renderToHtml(createElement(SettingsDialog));
    const { openSettingsDialog } = await import('../../frontend/src/app-shell/commands/settings/dialog.ts');
    await openSettingsDialog({ panel: 'workspace:default' });
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('paints a nav item per workspace with the shared nav class only', () => {
    const items = [...document.querySelectorAll('[data-panel^="workspace:"]')];
    expect(items.map((b) => b.dataset.panel)).toEqual(['workspace:default', 'workspace:ws2']);
    for (const item of items) expect(item.classList.contains('settings-nav-item')).toBe(true);
    for (const item of items) expect(item.classList.contains('settings-nav-workspace')).toBe(false);
  });

  it('resets the workspace page to Directory and loads that workspace\'s rules on a nav switch', async () => {
    expect(activeTab('workspace')).toBe('directory');
    click('#settings-panel-workspace [data-tab="hidden"]');
    expect(activeTab('workspace')).toBe('hidden');
    api.fetchHidePatterns.mockClear();
    click('[data-panel="workspace:ws2"]');
    await flush();
    expect(activeTab('workspace')).toBe('directory');
    expect(api.fetchHidePatterns).toHaveBeenCalledWith('ws2');
    expect(api.fetchProjects).toHaveBeenLastCalledWith('ws2');
  });

  it('resets the Workspaces page to Directory when it is chosen again', async () => {
    click('[data-panel="notes"]');
    click('#settings-panel-notes [data-tab="edit"]');
    expect(activeTab('notes')).toBe('edit');
    click('[data-panel="workspace:default"]');
    click('[data-panel="notes"]');
    await flush();
    expect(activeTab('notes')).toBe('directory');
  });
});
