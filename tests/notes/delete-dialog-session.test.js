// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const refreshAuthSession = vi.hoisted(() => vi.fn());
const deleteEntry = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/auth/refresh.ts', () => ({
  refreshAuthSession: (...args) => refreshAuthSession(...args),
}));
vi.mock('../../frontend/src/host/api.ts', () => ({
  deleteEntry: (...args) => deleteEntry(...args),
}));

const { confirmDeleteDocument, deleteOpenStore, openDeleteDialog } = await import(
  '../../frontend/src/notes/commands/delete-dialog.ts'
);
const { state } = await import('../../frontend/src/notes/state/host.ts');

let reloads;
const onReload = () => {
  reloads += 1;
};

beforeEach(() => {
  refreshAuthSession.mockReset();
  deleteEntry.mockReset();
  refreshAuthSession.mockResolvedValue({ kind: 'ok' });
  state.viewer.entry = { _id: 'doc-1' };
  state.ui.activeDate = null;
  window.location.hash = '#/spark/doc-1';
  reloads = 0;
  document.addEventListener('cta:reload', onReload);
  openDeleteDialog();
});

afterEach(() => {
  document.removeEventListener('cta:reload', onReload);
  state.viewer.entry = null;
});

describe('confirmDeleteDocument · the session is made fresh first', () => {
  it('refreshes the session before it asks the host to delete, then deletes that document', async () => {
    const order = [];
    refreshAuthSession.mockImplementation(async () => {
      order.push('refresh');
      return { kind: 'ok' };
    });
    deleteEntry.mockImplementation(async () => {
      order.push('delete');
      return { ok: true };
    });

    await confirmDeleteDocument();

    expect(order).toEqual(['refresh', 'delete']);
    expect(deleteEntry).toHaveBeenCalledWith('doc-1');
  });

  const outcomes = [
    ['signed out', async () => ({ kind: 'signed_out' })],
    ['the session could not be refreshed (network)', async () => ({ kind: 'failed', reason: 'network' })],
    ['the session could not be refreshed (refused)', async () => ({ kind: 'failed', reason: 'invalid' })],
    ['the refresh itself threw', async () => { throw new Error('vault is locked'); }],
  ];

  it.each(outcomes)('still asks the host to delete when %s: the host decides', async (_name, answer) => {
    refreshAuthSession.mockImplementation(answer);
    deleteEntry.mockResolvedValue({ ok: true });

    await confirmDeleteDocument();

    expect(deleteEntry).toHaveBeenCalledTimes(1);
    expect(deleteEntry).toHaveBeenCalledWith('doc-1');
  });
});

describe('confirmDeleteDocument · a delete the host could not make', () => {
  it('throws the host message and leaves the dialog, the page and the lists as they were', async () => {
    deleteEntry.mockResolvedValue({
      error: 'Could not remove the document from the cloud, so it was not deleted: offline',
      _status: 502,
    });

    await expect(confirmDeleteDocument()).rejects.toThrow(
      'Could not remove the document from the cloud, so it was not deleted: offline',
    );

    expect(deleteOpenStore.getSnapshot()).toBe(true);
    expect(window.location.hash).toBe('#/spark/doc-1');
    expect(reloads).toBe(0);
  });

  it('throws what the host call throws, with the same result', async () => {
    deleteEntry.mockRejectedValue(new Error('Delete request failed'));

    await expect(confirmDeleteDocument()).rejects.toThrow('Delete request failed');

    expect(deleteOpenStore.getSnapshot()).toBe(true);
    expect(reloads).toBe(0);
  });
});

describe('confirmDeleteDocument · a delete that was made', () => {
  it('closes the dialog, leaves the document page and reloads the lists', async () => {
    deleteEntry.mockResolvedValue({ ok: true });

    await confirmDeleteDocument();

    expect(deleteOpenStore.getSnapshot()).toBe(false);
    expect(window.location.hash).toBe('#/spark');
    expect(reloads).toBe(1);
  });

  it('goes back to the day it was opened from', async () => {
    state.ui.activeDate = '2026-11-01';
    deleteEntry.mockResolvedValue({ ok: true });

    await confirmDeleteDocument();

    expect(window.location.hash).toBe('#/spark?date=2026-11-01');
  });
});
