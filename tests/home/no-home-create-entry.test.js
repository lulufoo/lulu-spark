// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import { useLoadedWorkspaces } from '../helpers/workspaces.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const openCreateNoteMock = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/notes/commands/viewer/create.ts', () => ({
  openCreateNote: (...args) => openCreateNoteMock(...args),
  finalizeCreateSession: vi.fn(),
  closeModal: vi.fn(),
  applyCreateChrome: vi.fn(),
  clearCreateChrome: vi.fn(),
}));

function read(file) {
  return readFileSync(join(repoRoot, file), 'utf8');
}

// A manual document needs an explicit workspace and Project, which Home does not have: the "+"
// lives on the document page (LW-399) and Home no longer offers one (LW-400).
describe('Home has no create-document entry · source', () => {
  const page = read('frontend/src/home/page.tsx');
  const nav = read('frontend/src/home/ui/workspace-nav.tsx');

  it('the workspace nav and page do not create documents', () => {
    expect(nav).not.toMatch(/data-role="create-note"/);
    expect(nav).not.toMatch(/onCreateDocument|PlusIcon/);
    expect(page).not.toMatch(/onCreateDocument/);
    expect(page).not.toMatch(/openCreateNote/);
    expect(page).not.toMatch(/(?<!open)createNote/);
  });

  it('keeps the Read Later shortcut and does not restore the old add entry', () => {
    const topActions = readFileSync(join(repoRoot, 'frontend/src/home/ui/top-actions.tsx'), 'utf8');
    expect(topActions).toMatch(/data-home-entry="read-later"/);
    expect(page).toMatch(/<TopActions/);
    expect(page).toMatch(/goReadLater\(/);
    expect(page).not.toMatch(/data-home-entry="read-later"/);
    expect(page).not.toMatch(/note-assistant-fab|note-assistant-create/);
    expect(page).not.toMatch(/data-home-entry="notes"/);
    expect(page).not.toMatch(/新建随记/);
  });
});

describe('Home has no create-document entry · behavior', () => {
  let container;
  let cleanup;

  let restoreWorkspaces;

  beforeEach(() => {
    restoreWorkspaces = useLoadedWorkspaces();
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    openCreateNoteMock.mockReset();
    window.__TAURI__ = { event: { listen: vi.fn(async () => vi.fn()) } };
    vi.spyOn(api, 'getMessageChannelUnread').mockResolvedValue(false);
    vi.spyOn(api, 'markMessageChannelRead').mockResolvedValue(undefined);
    vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'unbound' };
      if (cmd === 'list_chat_sessions') return { sessions: [], current_session_id: '' };
      return {};
    });
  });

  afterEach(() => {
    restoreWorkspaces();
    cleanup?.();
    vi.restoreAllMocks();
    container.remove();
    delete window.__TAURI__;
  });

  it('renders the workspace rows and Read Later with no "+" beside any workspace', async () => {
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-home-entry="spark"]')).not.toBeNull();
    });
    expect(container.querySelector('[data-home-entry="read-later"]')).not.toBeNull();
    expect(container.querySelector('.home-nav-workspaces [data-role="create-note"]')).toBeNull();
    expect(container.querySelector('.home-nav-workspaces .home-chat-new')).toBeNull();
    expect(container.querySelector('.note-assistant-fab')).toBeNull();
    expect(openCreateNoteMock).not.toHaveBeenCalled();
  });
});
