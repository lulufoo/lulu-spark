// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const openCreateNoteMock = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/notes/commands/viewer/create.ts', () => ({
  openCreateNote: (...args) => openCreateNoteMock(...args),
  finalizeCreateSession: vi.fn(),
  closeModal: vi.fn(),
  applyCreateChrome: vi.fn(),
  clearCreateChrome: vi.fn(),
}));

function readPage() {
  return readFileSync(join(repoRoot, 'frontend/src/home/page.tsx'), 'utf8');
}

describe('home Notes create entry · source', () => {
  const page = readPage();

  it('puts a create control beside the Notes shortcut, not on the Notes title', () => {
    expect(page).toMatch(/data-home-entry="workbench"/);
    expect(page).toMatch(/data-role="create-note"/);
    expect(page).toMatch(/aria-label="New note"/);
    const notesBtn = page.indexOf('data-home-entry="workbench"');
    const createBtn = page.indexOf('data-role="create-note"');
    const notesLabel = page.indexOf('>Notes<');
    expect(notesBtn).toBeGreaterThan(-1);
    expect(createBtn).toBeGreaterThan(notesBtn);
    expect(notesLabel).toBeGreaterThan(notesBtn);
    expect(page.slice(notesBtn, notesLabel)).not.toMatch(/data-role="create-note"/);
  });

  it('prepares a non-empty temp_id and calls existing openCreateNote', () => {
    expect(page).toMatch(/from ['"]\.\.\/notes\/commands\/viewer\/create\.ts['"]/);
    expect(page).toMatch(/openCreateNote\(\{\s*temp_id\s*\}\)/);
    expect(page).toMatch(/temp_id/);
    expect(page).not.toMatch(/(?<!open)createNote/);
    expect(page).not.toMatch(/openCreateNoteFromFab/);
  });

  it('keeps the Read Later shortcut and does not restore the old add entry', () => {
    expect(page).toMatch(/data-home-entry="read-later"/);
    expect(page).toMatch(/goHomeEntry\(\s*['"]read-later['"]/);
    expect(page).not.toMatch(/note-assistant-fab|note-assistant-create/);
    expect(page).not.toMatch(/data-home-entry="notes"/);
    expect(page).not.toMatch(/新建随记/);
  });
});

describe('home Notes create entry · behavior', () => {
  let container;
  let cleanup;
  let navigate;
  /** @type {import('vitest').MockInstance} */
  let invokeSpy;
  /** @type {import('vitest').MockInstance} */
  let createNoteSpy;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    navigate = vi.fn();
    openCreateNoteMock.mockReset();
    window.__TAURI__ = {
      event: {
        listen: vi.fn(async () => vi.fn()),
      },
    };
    vi.spyOn(api, 'getMessageChannelUnread').mockResolvedValue(false);
    vi.spyOn(api, 'markMessageChannelRead').mockResolvedValue(undefined);
    createNoteSpy = vi.spyOn(api, 'createNote').mockResolvedValue({});
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'unbound' };
      if (cmd === 'list_chat_sessions') return { sessions: [], current_session_id: '' };
      return {};
    });
  });

  afterEach(() => {
    cleanup?.();
    vi.restoreAllMocks();
    container.remove();
    delete window.__TAURI__;
  });

  it('opens the existing create session from a control beside Notes', async () => {
    cleanup = mountHomeHub(container, { navigate });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-home-entry="workbench"]')).not.toBeNull();
    });

    const notes = container.querySelector('[data-home-entry="workbench"]');
    const create = container.querySelector('[data-role="create-note"]');
    const readLater = container.querySelector('[data-home-entry="read-later"]');
    expect(create).not.toBeNull();
    expect(readLater).not.toBeNull();
    expect(notes.contains(create)).toBe(false);
    expect(notes.parentElement.contains(create)).toBe(true);
    expect(container.querySelector('.note-assistant-fab')).toBeNull();
    expect(container.querySelector('[data-home-entry="notes"]')).toBeNull();

    create.click();
    expect(navigate).toHaveBeenCalledWith('#/workbench');
    expect(openCreateNoteMock).toHaveBeenCalledTimes(1);
    const arg = openCreateNoteMock.mock.calls[0][0];
    expect(typeof arg.temp_id).toBe('string');
    expect(arg.temp_id.length).toBeGreaterThan(0);
    expect(createNoteSpy).not.toHaveBeenCalled();
    expect(invokeSpy.mock.calls.map((call) => call[0])).not.toContain('create_note');
  });
});
