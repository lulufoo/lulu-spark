// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, it, expect } from 'vitest';
import { state, notifyState } from '../../frontend/src/host/state.ts';
import { WorkspaceNavItems } from '../../frontend/src/app-shell/ui/settings/workspace-nav.tsx';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');

describe('Settings left nav: workspaces sit indented under "Workspaces"', () => {
  it('every workspace item (Default, Knowledge, others) is a nav child', async () => {
    state.workspaces = {
      current: 'default',
      list: [
        { workspace_id: 'default', workspace_name: 'Default', view_mode: 'timeline', is_default: true },
        { workspace_id: 'knowledge', workspace_name: 'Knowledge', view_mode: 'directory', is_default: false },
        { workspace_id: 'research', workspace_name: 'Research', view_mode: 'directory', is_default: false },
      ],
    };
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => {
      root.render(createElement(WorkspaceNavItems));
      notifyState();
    });
    const items = [...host.querySelectorAll('.settings-nav-item')];
    expect(items.map((b) => b.dataset.panel)).toEqual(['workspace:default', 'workspace:knowledge', 'workspace:research']);
    for (const item of items) expect(item.classList.contains('settings-nav-child')).toBe(true);
    await act(async () => root.unmount());
  });

  it('the child indent is deeper than the top-level items', () => {
    const child = css.match(/\.settings-nav-item\.settings-nav-child \{\s*padding-left: (\d+)px;/);
    const top = css.match(/\.settings-nav-item \{[^}]*?padding: \d+px (\d+)px;/);
    expect(child).not.toBeNull();
    expect(Number(child[1])).toBeGreaterThan(Number(top[1]));
  });
});
