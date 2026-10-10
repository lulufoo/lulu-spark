import { describe, expect, it } from 'vitest';

import {
  parseReferences,
  referenceKindByScheme,
  registerReferenceKind,
  serializeReference,
} from '../../frontend/src/home/references/index.ts';

const NOTE_ID = 'f6692dc5d5242eec5206b6400e704bd6';
const READ_LATER_ID = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';

describe('reference kinds', () => {
  it('registers note, knowledge, and read-later out of the box', () => {
    const note = referenceKindByScheme('note');
    const knowledge = referenceKindByScheme('knowledge');
    const readLater = referenceKindByScheme('read-later');
    expect(note?.kind).toBe('note');
    expect(note?.label).toBe('note');
    expect(knowledge?.kind).toBe('knowledge');
    expect(knowledge?.label).toBe('knowledge');
    expect(readLater?.kind).toBe('read-later');
    expect(readLater?.label).toBe('read-later');
    expect(referenceKindByScheme('file')).toBeUndefined();
  });

  it('lets a new kind plug in without touching the parser', () => {
    registerReferenceKind({
      kind: 'skill',
      scheme: 'skill',
      label: 'skill',
      idPattern: /^[a-z-]+$/,
      open: () => {},
    });
    const segments = parseReferences('use [Review](skill:code-review)');
    expect(segments[1]).toMatchObject({ type: 'ref', kind: 'skill', id: 'code-review', title: 'Review' });
  });
});

describe('parseReferences', () => {
  it('keeps plain text as one text segment', () => {
    expect(parseReferences('hello\nworld')).toEqual([{ type: 'text', text: 'hello\nworld' }]);
    expect(parseReferences('')).toEqual([]);
  });

  it('splits text around a note reference', () => {
    const raw = `read [Agent Mock](note:${NOTE_ID}) first`;
    expect(parseReferences(raw)).toEqual([
      { type: 'text', text: 'read ' },
      { type: 'ref', kind: 'note', id: NOTE_ID, title: 'Agent Mock', raw: `[Agent Mock](note:${NOTE_ID})` },
      { type: 'text', text: ' first' },
    ]);
  });

  it('accepts a knowledge id of any hex length', () => {
    const long = parseReferences('[A](knowledge:b5c48a6383d20ccb1251d2cc0feb9e3a)');
    const legacy = parseReferences('[B](knowledge:b5c48a6383d2)');
    expect(long[0]).toMatchObject({ type: 'ref', kind: 'knowledge' });
    expect(legacy[0]).toMatchObject({ type: 'ref', kind: 'knowledge', id: 'b5c48a6383d2' });
  });

  it('accepts a 32-hex read-later id and leaves a short id as text', () => {
    const ok = parseReferences(`[Saved](read-later:${READ_LATER_ID})`);
    expect(ok[0]).toMatchObject({
      type: 'ref',
      kind: 'read-later',
      id: READ_LATER_ID,
      title: 'Saved',
    });
    expect(parseReferences('[Saved](read-later:abc123)')).toEqual([
      { type: 'text', text: '[Saved](read-later:abc123)' },
    ]);
  });

  it('parses adjacent references', () => {
    const segments = parseReferences(`[A](note:${NOTE_ID})[B](knowledge:abc123)`);
    expect(segments.map((s) => s.type)).toEqual(['ref', 'ref']);
  });

  it('leaves anything that is not a valid registered reference as text', () => {
    const cases = [
      '[x](note:abc)', // note ids are 32 hex
      `[x](note:${NOTE_ID.toUpperCase()})`,
      '[x](https://example.com)',
      '[x](unknown:abc123)',
      `[](note:${NOTE_ID})`,
      `[multi\nline](note:${NOTE_ID})`,
    ];
    for (const raw of cases) {
      expect(parseReferences(raw), raw).toEqual([{ type: 'text', text: raw }]);
    }
  });

  it('never loses or reorders characters', () => {
    const raw = `a [b](note:${NOTE_ID}) c [d](note:zz) e\n[f](knowledge:ab12)`;
    const joined = parseReferences(raw)
      .map((s) => (s.type === 'text' ? s.text : s.raw))
      .join('');
    expect(joined).toBe(raw);
  });
});

describe('serializeReference', () => {
  it('writes the markdown link the model sees', () => {
    expect(serializeReference({ kind: 'note', id: NOTE_ID, title: 'Agent Mock' })).toBe(
      `[Agent Mock](note:${NOTE_ID})`,
    );
  });

  it('flattens newlines and drops brackets in the title', () => {
    expect(serializeReference({ kind: 'knowledge', id: 'ab12', title: 'a[b]\nc' })).toBe(
      '[ab c](knowledge:ab12)',
    );
  });

  it('round-trips through the parser', () => {
    const raw = serializeReference({ kind: 'note', id: NOTE_ID, title: 'T' });
    expect(parseReferences(raw)[0]).toMatchObject({ type: 'ref', kind: 'note', id: NOTE_ID, title: 'T' });
  });

  it('round-trips a read-later reference', () => {
    const raw = serializeReference({ kind: 'read-later', id: READ_LATER_ID, title: 'Saved page' });
    expect(raw).toBe(`[Saved page](read-later:${READ_LATER_ID})`);
    expect(parseReferences(raw)[0]).toMatchObject({
      type: 'ref',
      kind: 'read-later',
      id: READ_LATER_ID,
      title: 'Saved page',
    });
  });

  it('refuses a kind that is not registered', () => {
    expect(() => serializeReference({ kind: 'nope', id: 'x', title: 'T' })).toThrow();
  });
});
