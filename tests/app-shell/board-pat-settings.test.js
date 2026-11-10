// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const TOKEN = 'lbp_do_not_show_0123456789abcdef';
const calls = [];
let status = false;
let failSave = false;
let failRemove = false;

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchBoardPatStatus: async () => status,
  setBoardPat: async (pat) => {
    calls.push(['set', pat]);
    if (failSave) throw new Error(`store blew up ${pat}`);
  },
  deleteBoardPat: async () => {
    calls.push(['delete']);
    if (failRemove) throw new Error(`remove blew up ${TOKEN}`);
  },
}));

const { settingsOpenStore } = await import('../../frontend/src/app-shell/state/dialog-open.ts');
const { boardPatStore, emptyBoardPatSnap } = await import(
  '../../frontend/src/app-shell/state/settings/board-pat.ts'
);
const { SettingsDialogChrome } = await import('../../frontend/src/app-shell/ui/settings/chrome.tsx');

function render() {
  const host = document.createElement('div');
  document.body.appendChild(host);
  const root = createRoot(host);
  act(() => root.render(createElement(SettingsDialogChrome)));
  return { host, root };
}

beforeEach(() => {
  calls.length = 0;
  status = false;
  failSave = false;
  failRemove = false;
  boardPatStore.set(emptyBoardPatSnap());
  settingsOpenStore.set(false);
  document.body.innerHTML = '';
});

describe('Board token settings', () => {
  it('adds a Board nav item and shows saved state without the token', async () => {
    status = true;
    const { host, root } = render();
    const board = host.querySelector('[data-panel="board"]');
    expect(board.textContent).toBe('Board');
    expect(board.previousElementSibling.dataset.panel).toBe('devices');
    expect(board.nextElementSibling.dataset.panel).toBe('theme');
    await act(async () => {
      settingsOpenStore.set(true);
    });
    const panel = host.querySelector('#settings-panel-board');
    expect(panel.querySelector('h3').textContent).toBe('Board');
    expect(panel.textContent).toContain('Connect your Board account to preview Sketch links.');
    expect(panel.querySelector('.board-pat-status').textContent).toBe('Board PAT configured');
    expect(panel.querySelector('.board-pat-status').classList.contains('is-set')).toBe(true);
    expect(panel.querySelector('label').textContent).toBe('Board PAT');
    expect(host.querySelector('#settings-board-pat').placeholder).toBe('Paste a new personal access token');
    expect(host.querySelector('#settings-board-pat').value).toBe('');
    expect(host.querySelector('#settings-board-pat-hint').textContent).toBe(
      'Your saved token stays hidden and is stored securely.',
    );
    const actions = [...panel.querySelectorAll('.settings-panel-actions button')].map((button) => button.textContent);
    expect(actions).toEqual(['Remove', 'Save']);
    expect(host.textContent).not.toContain(TOKEN);
    act(() => root.unmount());
  });

  it('saves a replacement, then clears the field to the mask', async () => {
    const { host, root } = render();
    await act(async () => {
      settingsOpenStore.set(true);
    });
    const input = host.querySelector('#settings-board-pat');
    const form = input.closest('form');
    input.value = TOKEN;
    await act(async () => {
      form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    });
    expect(calls).toEqual([['set', TOKEN]]);
    expect(host.querySelector('#settings-result-board').textContent).toBe('Board token saved.');
    expect(host.querySelector('.board-pat-status').textContent).toBe('Board PAT configured');
    expect(input.value).toBe('');
    expect(JSON.stringify(boardPatStore.getSnapshot())).not.toContain(TOKEN);
    expect(host.textContent).not.toContain(TOKEN);
    act(() => root.unmount());
  });

  it('shows save and remove failures without the token and re-enables the buttons', async () => {
    failSave = true;
    const { host, root } = render();
    await act(async () => {
      settingsOpenStore.set(true);
    });
    const input = host.querySelector('#settings-board-pat');
    const save = host.querySelector('#settings-panel-board .btn-settings-save');
    input.value = TOKEN;
    await act(async () => {
      input.closest('form').dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    });
    expect(host.querySelector('#settings-result-board').textContent).toBe("Couldn't save the Board token.");
    expect(save.disabled).toBe(false);
    expect(host.textContent).not.toContain(TOKEN);
    failRemove = true;
    await act(async () => {
      host.querySelector('#settings-panel-board .btn-settings-ghost').click();
    });
    expect(host.querySelector('#settings-result-board').textContent).toBe("Couldn't remove the Board token.");
    expect(host.textContent).not.toContain(TOKEN);
    act(() => root.unmount());
  });
});
