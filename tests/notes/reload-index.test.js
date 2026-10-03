// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { state } from '../../frontend/src/host/state.ts';
import { refreshNotesIndex } from '../../frontend/src/notes/commands/reload-index.ts';

const OLD_ID = 'old1';
const NEW_ID = 'new1';
const NEW_PATH = 'inbox/new.md';
const NEW_DATE = '20261003';

function seedOld() {
  state.index.data = {
    [OLD_ID]: {
      _id: OLD_ID,
      common_path: 'other/old.md',
      created_at: '20260101120000',
    },
  };
  state.index.groupedByDate = [];
  state.index.filteredGroups = [];
  state.ui.activeTopic = 'inbox';
  state.ui.activeTagKey = 'tag-a';
  state.ui.activeDate = '20260101';
}

beforeEach(() => {
  seedOld();
});

afterEach(() => {
  state.index.data = null;
  state.index.groupedByDate = [];
  state.index.filteredGroups = [];
  state.ui.activeTopic = null;
  state.ui.activeTagKey = null;
  state.ui.activeDate = null;
  vi.restoreAllMocks();
});

describe('refreshNotesIndex', () => {
  it('replaces the snapshot from Host index.json and keeps topic/tag/date', async () => {
    vi.spyOn(api, 'fetchIndex').mockResolvedValue({
      entries: {
        [NEW_ID]: {
          common_path: NEW_PATH,
          created_at: `${NEW_DATE}120000`,
        },
      },
    });

    await expect(refreshNotesIndex()).resolves.toBe(true);
    expect(state.index.data?.[NEW_ID]?.common_path).toBe(NEW_PATH);
    expect(state.index.data?.[NEW_ID]?._id).toBe(NEW_ID);
    expect(state.index.data?.[OLD_ID]).toBeUndefined();
    expect(state.index.groupedByDate.map((g) => g.date)).toEqual([NEW_DATE]);
    expect(state.ui.activeTopic).toBe('inbox');
    expect(state.ui.activeTagKey).toBe('tag-a');
    expect(state.ui.activeDate).toBe('20260101');
  });

  it('leaves the snapshot when Host fetch fails', async () => {
    vi.spyOn(api, 'fetchIndex').mockRejectedValue(new Error('offline'));
    const before = state.index.data;
    await expect(refreshNotesIndex()).resolves.toBe(false);
    expect(state.index.data).toBe(before);
    expect(state.ui.activeTopic).toBe('inbox');
  });
});
