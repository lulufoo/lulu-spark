// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { renderToHtml } from '../../frontend/src/island.ts';
import { SettingsDialog } from '../../frontend/src/app-shell/ui/settings/dialog.tsx';
import { NotesCategoriesEditHost } from '../../frontend/src/app-shell/ui/settings/notes-categories-edit.tsx';
import {
  emptyNotesCategoriesSnap,
  notesCategoriesStore,
} from '../../frontend/src/app-shell/state/settings/notes-categories.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchConfig: vi.fn(),
  setConfig: vi.fn(),
  invoke: vi.fn(),
  fetchNotesCategories: vi.fn(),
  fetchKbHidePatterns: vi.fn(),
}));

import * as api from '../../frontend/src/host/api.ts';

const rootDir = join(dirname(fileURLToPath(import.meta.url)), '../..');
const GLYPH = '✕';

function readRel(rel) {
  return readFileSync(join(rootDir, rel), 'utf8');
}

function visibleCopy(el) {
  return String(el?.textContent ?? '');
}

function seedNotesEditor({ busy = false } = {}) {
  notesCategoriesStore.set({
    ...emptyNotesCategoriesSnap(),
    rows: [{ id: 'work', title: 'Work', description: '', folder: 'work' }],
    selectedId: 'work',
    draftTitle: 'Work',
    draftDescription: '',
    editorOpen: true,
    busy,
  });
}

function baseConfig() {
  return {
    spark_root: '',
    knowledge_root: '',
    assistant_engine: 'host',
    has_host_key: false,
    mcp_port: 19876,
    llm: {
      platform: 'glm',
      base_url: 'https://open.bigmodel.cn/api/paas/v4',
      model: '',
    },
  };
}

describe('Settings header and footer dismiss', () => {
  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    api.fetchConfig.mockResolvedValue(baseConfig());
    api.setConfig.mockResolvedValue(baseConfig());
    api.fetchNotesCategories.mockResolvedValue({ categories: [] });
    api.fetchKbHidePatterns.mockResolvedValue({ patterns: [] });
    api.invoke.mockResolvedValue({
      channels: [],
      groups: [],
      enabled: {},
    });
    document.body.innerHTML = renderToHtml(createElement(SettingsDialog));
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog();
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('paints Settings header dismiss as OverlayDismissButton with ✕ and no Close copy', () => {
    const chromeSrc = readRel('frontend/src/app-shell/ui/settings/chrome.tsx');
    expect(chromeSrc).toMatch(/<OverlayDismissButton[\s\S]*?\bid=["']btn-settings-close["']/);

    const button = document.getElementById('btn-settings-close');
    expect(button).not.toBeNull();
    expect(button.classList.contains('overlay-dismiss-button')).toBe(true);
    expect(visibleCopy(button).trim()).toBe(GLYPH);
    expect(visibleCopy(button)).not.toContain('Close');
  });

  it('keeps existing Settings close wiring on the header ✕', () => {
    const dialog = document.getElementById('settings-dialog');
    expect(dialog.classList.contains('open')).toBe(true);
    document.getElementById('btn-settings-close').click();
    expect(dialog.classList.contains('open')).toBe(false);
  });

  it('removes Settings footer Close and still dismisses from the header ✕', () => {
    expect(document.getElementById('btn-settings-cancel')).toBeNull();
    const footer = document.getElementById('settings-dialog-footer');
    if (footer) {
      expect(visibleCopy(footer)).not.toMatch(/Close/);
      expect(footer.querySelector('button')).toBeNull();
    }

    const dialog = document.getElementById('settings-dialog');
    expect(dialog.classList.contains('open')).toBe(true);
    document.getElementById('btn-settings-close').click();
    expect(dialog.classList.contains('open')).toBe(false);
  });

  it('keeps Settings close on the command id lookup, not inside the shared control', () => {
    const dialogSrc = readRel('frontend/src/app-shell/commands/settings/dialog.ts');
    const sharedSrc = readRel('frontend/src/shared/overlay-dismiss-button.tsx');
    const chromeSrc = readRel('frontend/src/app-shell/ui/settings/chrome.tsx');
    expect(dialogSrc).toMatch(/btn\(['"]btn-settings-close['"]\)/);
    expect(sharedSrc).not.toMatch(/closeSettingsDialog/);
    expect(chromeSrc).not.toMatch(/closeSettingsDialog/);
  });
});

describe('Settings close hook without #btn-settings-close', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('does not close Settings when the command cannot find #btn-settings-close', async () => {
    vi.resetModules();
    vi.clearAllMocks();
    api.fetchConfig.mockResolvedValue(baseConfig());
    api.setConfig.mockResolvedValue(baseConfig());
    api.fetchNotesCategories.mockResolvedValue({ categories: [] });
    api.fetchKbHidePatterns.mockResolvedValue({ patterns: [] });
    api.invoke.mockResolvedValue({ channels: [], groups: [], enabled: {} });

    document.body.innerHTML = renderToHtml(createElement(SettingsDialog));
    const button = document.getElementById('btn-settings-close');
    button.removeAttribute('id');
    const { openSettingsDialog } = await import(
      '../../frontend/src/app-shell/commands/settings/dialog.ts'
    );
    await openSettingsDialog();
    const dialog = document.getElementById('settings-dialog');
    expect(dialog.classList.contains('open')).toBe(true);
    button.click();
    expect(dialog.classList.contains('open')).toBe(true);
    expect(document.getElementById('btn-settings-close')).toBeNull();
  });
});

describe('Notes-category editor dismiss', () => {
  let container;
  let root;

  function renderEditor() {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    act(() => {
      root.render(createElement(NotesCategoriesEditHost));
    });
  }

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    notesCategoriesStore.set(emptyNotesCategoriesSnap());
    document.body.innerHTML = '';
  });

  it('paints notes-category dismiss as OverlayDismissButton and still closes the editor', () => {
    const editSrc = readRel('frontend/src/app-shell/ui/settings/notes-categories-edit.tsx');
    expect(editSrc).toMatch(/<OverlayDismissButton[\s\S]*?\bid=["']btn-notes-cat-edit-close["']/);

    seedNotesEditor();
    renderEditor();
    const button = document.getElementById('btn-notes-cat-edit-close');
    expect(button).not.toBeNull();
    expect(button.classList.contains('overlay-dismiss-button')).toBe(true);
    expect(visibleCopy(button).trim()).toBe(GLYPH);
    expect(visibleCopy(button)).not.toContain('Close');

    act(() => {
      button.click();
    });
    expect(notesCategoriesStore.getSnapshot().editorOpen).toBe(false);
  });

  it('disables the shared notes-category dismiss while busy and does not close', () => {
    seedNotesEditor({ busy: true });
    renderEditor();
    const button = document.getElementById('btn-notes-cat-edit-close');
    expect(button.classList.contains('overlay-dismiss-button')).toBe(true);
    expect(button.disabled).toBe(true);
    act(() => {
      button.click();
    });
    expect(notesCategoriesStore.getSnapshot().editorOpen).toBe(true);
  });
});
