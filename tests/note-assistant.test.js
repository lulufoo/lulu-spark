// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const fetchIndexMock = vi.fn();
const openCreateNoteMock = vi.fn();

vi.mock('../frontend/js/api.js', () => ({
  fetchIndex: (...args) => fetchIndexMock(...args),
}));

import {
  selectTopNotesByCreatedAt,
  loadAssistantNotes,
  mountNoteAssistant,
  mountNoteAssistantWidget,
} from '../frontend/js/note-assistant.js';

const sampleEntries = [
  {
    _id: 'n_oldest',
    source_type: 'note',
    common_path: 'inbox/notes/202607011000-oldest.md',
    created_at: '202607011000',
    title: 'Oldest Note',
    layers: ['raw'],
  },
  {
    _id: 'n_newest',
    source_type: 'note',
    common_path: 'inbox/notes/202607061000-newest.md',
    created_at: '202607061000',
    title: 'Newest Note',
    layers: ['raw'],
  },
  {
    _id: 'n_mid',
    source_type: 'note',
    common_path: 'inbox/notes/202607031000-mid.md',
    created_at: '202607031000',
    title: 'Mid Note',
    layers: ['raw'],
  },
  {
    _id: 'n_extra4',
    source_type: 'note',
    common_path: 'inbox/notes/202607041000-extra4.md',
    created_at: '202607041000',
    title: 'Extra Note 4',
    layers: ['raw'],
  },
  {
    _id: 'd_dialogue',
    source_type: 'dialogue',
    common_path: 'inbox/202607071000-dialogue.md',
    created_at: '202607071000',
    title: 'Dialogue Entry',
    layers: ['raw'],
  },
  {
    _id: 'n_updated_trap',
    source_type: 'note',
    common_path: 'inbox/notes/202606011000-trap.md',
    created_at: '202606011000',
    updated_at: '202607099999',
    title: 'Updated Trap',
    layers: ['raw'],
  },
];

function indexPayload(entries = sampleEntries) {
  const map = {};
  for (const entry of entries) {
    const { _id, ...rest } = entry;
    map[_id] = rest;
  }
  return { entries: map };
}

describe('selectTopNotesByCreatedAt', () => {
  it('filters source_type=note, sorts created_at desc, slices to ≤3', () => {
    const top = selectTopNotesByCreatedAt(sampleEntries);
    expect(top).toHaveLength(3);
    expect(top.map((e) => e._id)).toEqual(['n_newest', 'n_extra4', 'n_mid']);
    expect(top.every((e) => e.source_type === 'note')).toBe(true);
  });

  it('ignores updated_at when ordering (created_at only)', () => {
    const top = selectTopNotesByCreatedAt(sampleEntries);
    expect(top.map((e) => e._id)).not.toContain('n_updated_trap');
    const withTrapOnly = selectTopNotesByCreatedAt([
      sampleEntries.find((e) => e._id === 'n_updated_trap'),
      sampleEntries.find((e) => e._id === 'n_oldest'),
    ]);
    expect(withTrapOnly.map((e) => e._id)).toEqual(['n_oldest', 'n_updated_trap']);
  });

  it('returns fewer than 3 when note count is below 3 without placeholders', () => {
    const notes = sampleEntries.filter((e) => e._id === 'n_newest' || e._id === 'n_mid');
    const top = selectTopNotesByCreatedAt(notes);
    expect(top).toHaveLength(2);
    expect(top.map((e) => e._id)).toEqual(['n_newest', 'n_mid']);
  });

  it('returns empty array when no note entries', () => {
    expect(selectTopNotesByCreatedAt([])).toEqual([]);
    expect(
      selectTopNotesByCreatedAt([
        { _id: 'x', source_type: 'summary', created_at: '202607099999' },
      ]),
    ).toEqual([]);
  });
});

describe('loadAssistantNotes', () => {
  beforeEach(() => {
    fetchIndexMock.mockReset();
  });

  it('loads corpus-index via fetchIndex and returns entry list', async () => {
    fetchIndexMock.mockResolvedValue(indexPayload());
    const entries = await loadAssistantNotes();
    expect(fetchIndexMock).toHaveBeenCalled();
    expect(entries.some((e) => e.source_type === 'note')).toBe(true);
    expect(entries.find((e) => e._id === 'n_newest')?.common_path).toBe(
      'inbox/notes/202607061000-newest.md',
    );
  });

  it('throws when index payload is invalid / error', async () => {
    fetchIndexMock.mockResolvedValue({
      error: 'No such file: /tmp/workbench-x/index.json',
      _status: 404,
    });
    await expect(loadAssistantNotes()).rejects.toThrow(/No such file/);
  });
});

describe('mountNoteAssistant', () => {
  let root;

  beforeEach(() => {
    root = document.createElement('div');
    document.body.appendChild(root);
    fetchIndexMock.mockReset();
    openCreateNoteMock.mockReset();
  });

  afterEach(() => {
    root.remove();
    document.querySelectorAll('.note-assistant-widget').forEach((el) => el.remove());
  });

  it('loads notes on mount and displays top3 in created_at desc order', async () => {
    fetchIndexMock.mockResolvedValue(indexPayload());
    const { dispose } = mountNoteAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelectorAll('.note-assistant-item')).toHaveLength(3);
    });
    const items = root.querySelectorAll('.note-assistant-item');
    expect(items[0].dataset.commonPath).toBe('inbox/notes/202607061000-newest.md');
    expect(items[1].dataset.commonPath).toBe('inbox/notes/202607041000-extra4.md');
    expect(items[2].dataset.commonPath).toBe('inbox/notes/202607031000-mid.md');
    dispose();
  });

  it('shows empty list state when no notes without placeholders', async () => {
    fetchIndexMock.mockResolvedValue(
      indexPayload(sampleEntries.filter((e) => e.source_type !== 'note')),
    );
    const { dispose } = mountNoteAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.note-assistant-empty')).not.toBeNull();
    });
    expect(root.querySelector('.note-assistant-item')).toBeNull();
    dispose();
  });

  it('shows error state when index load fails', async () => {
    fetchIndexMock.mockRejectedValue(new Error('Failed to fetch'));
    const { dispose } = mountNoteAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.note-assistant-state--error')).not.toBeNull();
    });
    expect(root.querySelector('.note-assistant-item')).toBeNull();
    dispose();
  });

  it('dispatches cta:open-entry with common_path when item is clicked', async () => {
    fetchIndexMock.mockResolvedValue(indexPayload());
    const handler = vi.fn();
    document.addEventListener('cta:open-entry', handler);
    const { dispose } = mountNoteAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.note-assistant-item-link')).not.toBeNull();
    });
    root.querySelector('.note-assistant-item-link').click();
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0].detail).toEqual({
      common_path: 'inbox/notes/202607061000-newest.md',
    });
    document.removeEventListener('cta:open-entry', handler);
    dispose();
  });

  it('create button triggers openCreateNote', async () => {
    fetchIndexMock.mockResolvedValue(indexPayload());
    const { dispose } = mountNoteAssistant(root, {
      openCreateNote: openCreateNoteMock,
    });
    await vi.waitFor(() => {
      expect(root.querySelector('.note-assistant-create')).not.toBeNull();
    });
    root.querySelector('.note-assistant-create').click();
    expect(openCreateNoteMock).toHaveBeenCalled();
    dispose();
  });
});

describe('mountNoteAssistantWidget', () => {
  let anchor;

  beforeEach(() => {
    anchor = document.createElement('div');
    document.body.appendChild(anchor);
    fetchIndexMock.mockReset();
    openCreateNoteMock.mockReset();
  });

  afterEach(() => {
    anchor.remove();
    document.querySelectorAll('.note-assistant-widget').forEach((el) => el.remove());
  });

  it('renders fixed launcher with popover hidden by default', () => {
    mountNoteAssistantWidget(anchor);
    expect(document.querySelector('.note-assistant-fab')).not.toBeNull();
    expect(document.querySelector('.note-assistant-popover')?.hidden).toBe(true);
  });

  it('opens popover and loads top3 on first open; exposes setOpen', async () => {
    fetchIndexMock.mockResolvedValue(indexPayload());
    const { setOpen, dispose } = mountNoteAssistantWidget(anchor, {
      openCreateNote: openCreateNoteMock,
    });
    expect(typeof setOpen).toBe('function');
    setOpen(true);
    await vi.waitFor(() => {
      expect(document.querySelectorAll('.note-assistant-item')).toHaveLength(3);
    });
    dispose();
  });
});
