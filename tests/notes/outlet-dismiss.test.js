// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { notifyState, state } from '../../frontend/src/host/state.ts';
import { repoRoot } from '../helpers/read-frontend-js.js';

const { closeModal } = vi.hoisted(() => ({
  closeModal: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchFileContent: vi.fn(),
  fetchAnnotation: vi.fn(),
  fetchDiffStatus: vi.fn(),
  saveFile: vi.fn(),
  getNoteDraft: vi.fn(),
  saveNoteDraft: vi.fn(),
  clearNoteDraft: vi.fn(),
  createNote: vi.fn(),
  getDraft: vi.fn(),
  saveDraft: vi.fn(),
  updateComments: vi.fn(),
  reorderComments: vi.fn(),
  fetchLinkTitle: vi.fn(),
  addNoteLink: vi.fn(),
  removeNoteLink: vi.fn(),
  fetchDocHighlights: vi.fn(async () => ({ highlights: [] })),
}));

vi.mock('../../frontend/src/notes/viewer.ts', () => ({
  closeModal,
  enterEditMode: vi.fn(),
  exitEditMode: vi.fn(),
  saveDoc: vi.fn(),
  switchLang: vi.fn(),
}));

vi.mock('../../frontend/src/notes/ui/sidebar-resize.ts', () => ({
  initSidebarResize: vi.fn(),
}));

vi.mock('../../frontend/src/notes/ui/viewer/body.tsx', () => ({
  renderDocBody: vi.fn(),
}));

vi.mock('../../frontend/src/knowledge/ui/knowledge-search.tsx', () => ({
  KnowledgeSearchHost: () => null,
}));

vi.mock('../../frontend/src/notes/ui/comments.tsx', () => ({
  NotesCommentFloatNav: () => null,
  NotesCommentsBar: () => null,
  NotesDeleteZone: () => null,
  openCommentDialog: vi.fn(),
}));

vi.mock('../../frontend/src/notes/ui/sidebar.tsx', () => ({
  NotesSidebar: () => null,
}));

vi.mock('../../frontend/src/notes/ui/cards.tsx', () => ({
  NotesDocList: () => null,
}));

import { NotesMain } from '../../frontend/src/notes/page.tsx';

const GLYPH = '✕';

function visibleCopy(el) {
  return String(el?.textContent ?? '');
}

function readRel(rel) {
  return readFileSync(`${repoRoot}/${rel}`, 'utf8');
}

describe('Notes viewer header dismiss', () => {
  let container;
  let root;

  function renderOutlet() {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    state.viewer.outletMode = 'open';
    state.viewer.entry = {
      common_path: 'inbox/a.md',
      created_at: '20261003120000',
    };
    state.viewer.rawText = '# Hello';
    state.viewer.loading = false;
    state.viewer.loadError = '';
    state.viewer.createSession = null;
    state.viewer.editing = false;
    state.ui.activeDate = '20261003';
    act(() => {
      root.render(createElement(NotesMain, { routeParams: { note: 'inbox/a.md', date: '20261003' } }));
      notifyState();
    });
  }

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    closeModal.mockReset();
    state.viewer.entry = null;
    state.viewer.outletMode = '';
    state.viewer.rawText = '';
    state.viewer.createSession = null;
    document.body.innerHTML = '';
  });

  it('paints Notes viewer dismiss as OverlayDismissButton with glyph and no Close copy', () => {
    const src = readRel('frontend/src/notes/page.tsx');
    expect(src).toMatch(/<OverlayDismissButton[\s\S]*?\bid=["']md-close["']/);

    renderOutlet();
    const button = document.getElementById('md-close');
    expect(button).not.toBeNull();
    expect(button.classList.contains('overlay-dismiss-button')).toBe(true);
    expect(visibleCopy(button).trim()).toBe(GLYPH);
    expect(visibleCopy(button)).not.toContain('Close');
  });

  it('keeps Notes close on closeModal from the shared header dismiss', () => {
    renderOutlet();
    act(() => {
      document.getElementById('md-close').click();
    });
    expect(closeModal).toHaveBeenCalledTimes(1);
  });

  it('keeps Notes close in commands and not inside the shared control', () => {
    const pageSrc = readRel('frontend/src/notes/page.tsx');
    const commandSrc = readRel('frontend/src/notes/commands/viewer/create.ts');
    const sharedSrc = readRel('frontend/src/shared/overlay-dismiss-button.tsx');
    expect(commandSrc).toMatch(/export async function closeModal/);
    expect(pageSrc).toMatch(/closeModal/);
    expect(sharedSrc).not.toMatch(/closeModal/);
  });
});

describe('Out-of-scope dismiss chrome stays unchanged', () => {
  it('does not restyle Home entry or context usage dismiss chrome', () => {
    const homeSrc = readRel('frontend/src/home-entry-shell/shell.tsx');
    const contextSrc = readRel('frontend/src/home/ui/context-percent.tsx');
    expect(homeSrc).not.toMatch(/OverlayDismissButton/);
    expect(homeSrc).toMatch(/className="home-entry-shell__close"/);
    expect(homeSrc).toMatch(/aria-label="Close"/);
    expect(homeSrc).toMatch(/>\s*×\s*</);
    expect(contextSrc).not.toMatch(/OverlayDismissButton/);
    expect(contextSrc).toMatch(/className="home-chat-context-usage-close"/);
    expect(contextSrc).toMatch(/aria-label="Close"/);
    expect(contextSrc).toMatch(/>\s*×\s*</);
  });

  it('keeps confirm-only overlays on Cancel without a new header x', () => {
    const confirmFiles = [
      'frontend/src/notes/ui/delete-dialog.tsx',
      'frontend/src/app-shell/ui/workbench-commit-dialog.tsx',
      'frontend/src/notes/ui/comments.tsx',
      'frontend/src/shared/comment-delete.tsx',
    ];
    for (const rel of confirmFiles) {
      const src = readRel(rel);
      expect(src, rel).not.toMatch(/OverlayDismissButton/);
      expect(src, rel).toMatch(/>\s*Cancel\s*</);
    }
  });
});
