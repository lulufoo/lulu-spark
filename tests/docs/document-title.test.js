// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { setDocTitle } = vi.hoisted(() => ({ setDocTitle: vi.fn() }));
vi.mock('../../frontend/src/host/api.ts', async (importOriginal) => ({
  ...(await importOriginal()), setDocTitle,
}));

import { state } from '../../frontend/src/host/state.ts';
import { NotesDocumentTitle, documentCreatedLabel } from '../../frontend/src/notes/ui/document-title.tsx';

let root;
let host;
let entry;

function paint(note) {
  act(() => root.render(createElement(NotesDocumentTitle, { key: note.id, entry: note })));
}
function input() { return host.querySelector('[aria-label="Document title"]'); }
function type(value) {
  act(() => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(input(), value);
    input().dispatchEvent(new Event('input', { bubbles: true }));
  });
}
async function blur() {
  await act(async () => { input().focus(); input().blur(); });
}

beforeEach(() => {
  globalThis.IS_REACT_ACT_ENVIRONMENT = true;
  setDocTitle.mockReset().mockResolvedValue({ ok: true });
  entry = { id: 'doc1', title: 'Original', created_at: '202605141721', raw_path: 'original.md' };
  state.viewer.entry = entry;
  state.viewer.rawText = '# Body heading\n\nBody content';
  state.index.data = { doc1: entry };
  state.index.groupedByDate = [{ date: '20260514', entries: [{ id: 'doc1', entry }] }];
  state.index.filteredGroups = state.index.groupedByDate;
  state.index.titleCache = new Map([['20260514', new Map([['doc1', 'Body heading']])]]);
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
  paint(entry);
});

afterEach(() => {
  act(() => root.unmount());
  host.remove();
  state.viewer.entry = null;
  state.index.data = null;
  state.index.groupedByDate = [];
  state.index.filteredGroups = [];
  state.index.titleCache = new Map();
});

describe('independent document title', () => {
  it('shows the stored title and read-only creation time in both timestamp formats', () => {
    expect(input().value).toBe('Original');
    expect(host.textContent).toContain('Created · May 14, 2026 17:21');
    expect(documentCreatedLabel('20260514172100')).toBe('Created · May 14, 2026 17:21');
    expect(documentCreatedLabel('20260514')).toBe('Created · May 14, 2026');
    expect(documentCreatedLabel(undefined)).toBe('Created · Unknown');
    expect(documentCreatedLabel('202613141721')).toBe('Created · Unknown');
  });

  it('saves an edited title without changing the body, filename or entry identity', async () => {
    type('  New title  ');
    await blur();
    expect(setDocTitle).toHaveBeenCalledWith('doc1', 'New title');
    expect(state.viewer.entry).toBe(entry);
    expect(entry.title).toBe('New title');
    expect(entry.raw_path).toBe('original.md');
    expect(state.viewer.rawText).toBe('# Body heading\n\nBody content');
    expect(state.index.titleCache.get('20260514').has('doc1')).toBe(false);
    expect(input().value).toBe('New title');
  });

  it('keeps the draft and reports validation and save errors', async () => {
    type('   ');
    await blur();
    expect(setDocTitle).not.toHaveBeenCalled();
    expect(host.querySelector('[role="alert"]').textContent).toContain('Title cannot be empty');
    type('Retry title');
    setDocTitle.mockRejectedValueOnce(new Error('Write failed'));
    await blur();
    expect(entry.title).toBe('Original');
    expect(input().value).toBe('Retry title');
    expect(host.querySelector('[role="alert"]').textContent).toBe('Write failed');
    await blur();
    expect(entry.title).toBe('Retry title');
  });

  it('does not apply a delayed save to a different open document', async () => {
    let finish;
    setDocTitle.mockReturnValueOnce(new Promise((resolve) => { finish = resolve; }));
    type('Saved later');
    act(() => { input().focus(); input().blur(); });
    const next = { id: 'doc2', title: 'Other document' };
    state.viewer.entry = next;
    paint(next);
    await act(async () => finish({ ok: true }));
    expect(entry.title).toBe('Saved later');
    expect(next.title).toBe('Other document');
    expect(input().value).toBe('Other document');
  });

  it('serializes saves when navigating away and back to the same document', async () => {
    let finishFirst;
    setDocTitle.mockReturnValueOnce(new Promise((resolve) => { finishFirst = resolve; }));
    type('First title');
    act(() => { input().focus(); input().blur(); });
    const next = { id: 'doc2', title: 'Other document' };
    state.viewer.entry = next;
    paint(next);
    state.viewer.entry = entry;
    paint(entry);
    type('Second title');
    act(() => { input().focus(); input().blur(); });
    expect(setDocTitle).toHaveBeenCalledTimes(1);
    await act(async () => finishFirst({ ok: true }));
    expect(setDocTitle).toHaveBeenNthCalledWith(2, 'doc1', 'Second title');
    expect(entry.title).toBe('Second title');
    expect(input().value).toBe('Second title');
  });

  it('reports a failed save after navigating away, including the unsaved title', async () => {
    let fail;
    const alert = vi.spyOn(window, 'alert').mockImplementation(() => {});
    setDocTitle.mockReturnValueOnce(new Promise((_, reject) => { fail = reject; }));
    type('Unsaved title');
    act(() => { input().focus(); input().blur(); });
    const next = { id: 'doc2', title: 'Other document' };
    state.viewer.entry = next;
    paint(next);
    await act(async () => fail(new Error('Write failed')));
    expect(alert).toHaveBeenCalledWith(expect.stringContaining('Unsaved title'));
    expect(alert).toHaveBeenCalledWith(expect.stringContaining('Write failed'));
    expect(entry.title).toBe('Original');
    alert.mockRestore();
  });

  it('preserves a new draft if an earlier save finishes while editing', async () => {
    let finishFirst;
    setDocTitle.mockReturnValueOnce(new Promise((resolve) => { finishFirst = resolve; }));
    type('First title');
    act(() => { input().focus(); input().blur(); });
    const next = { id: 'doc2', title: 'Other document' };
    state.viewer.entry = next;
    paint(next);
    state.viewer.entry = entry;
    paint(entry);
    act(() => input().focus());
    type('New draft');
    await act(async () => finishFirst({ ok: true }));
    paint(entry);
    expect(input().value).toBe('New draft');
    await act(async () => input().blur());
    expect(entry.title).toBe('New draft');
  });

  it('saves an explicit return to the original title while an earlier save is pending', async () => {
    let finishFirst;
    setDocTitle.mockReturnValueOnce(new Promise((resolve) => { finishFirst = resolve; }));
    type('First title');
    act(() => { input().focus(); input().blur(); });
    const next = { id: 'doc2', title: 'Other document' };
    state.viewer.entry = next;
    paint(next);
    state.viewer.entry = entry;
    paint(entry);
    type('Changed again');
    type('Original');
    act(() => { input().focus(); input().blur(); });
    await act(async () => finishFirst({ ok: true }));
    expect(setDocTitle).toHaveBeenNthCalledWith(2, 'doc1', 'Original');
    expect(entry.title).toBe('Original');
  });

  it('cancels an edit with Escape', () => {
    type('Unsaved');
    act(() => {
      input().focus();
      input().dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });
    expect(input().value).toBe('Original');
    expect(setDocTitle).not.toHaveBeenCalled();
  });

  it('saves on Enter without inserting a newline', async () => {
    act(() => input().focus());
    type('New title');
    await act(async () => input().dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })));
    expect(setDocTitle).toHaveBeenCalledWith('doc1', 'New title');
    expect(entry.title).toBe('New title');
  });
});
