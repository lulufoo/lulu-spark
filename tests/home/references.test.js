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
  it('registers doc and read-later out of the box; note and knowledge are gone', () => {
    const doc = referenceKindByScheme('doc');
    const readLater = referenceKindByScheme('read-later');
    expect(doc?.kind).toBe('doc');
    expect(doc?.label).toBe('doc');
    expect(referenceKindByScheme('note')).toBeUndefined();
    expect(referenceKindByScheme('knowledge')).toBeUndefined();
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

  it('splits text around a document reference', () => {
    const raw = `read [Default：Agent Mock](doc:${NOTE_ID}) first`;
    expect(parseReferences(raw)).toEqual([
      { type: 'text', text: 'read ' },
      { type: 'ref', kind: 'doc', id: NOTE_ID, title: 'Default：Agent Mock', raw: `[Default：Agent Mock](doc:${NOTE_ID})` },
      { type: 'text', text: ' first' },
    ]);
  });

  it('accepts a plain 32-hex id and a `<workspace id>-<32 hex>` id', () => {
    const prefixed = parseReferences(`[A](doc:default-${NOTE_ID})`);
    const other = parseReferences(`[B](doc:k3x9ab-${NOTE_ID})`);
    expect(prefixed[0]).toMatchObject({ type: 'ref', kind: 'doc', id: `default-${NOTE_ID}` });
    expect(other[0]).toMatchObject({ type: 'ref', kind: 'doc', id: `k3x9ab-${NOTE_ID}` });
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
    const segments = parseReferences(`[A](doc:${NOTE_ID})[B](doc:default-${NOTE_ID})`);
    expect(segments.map((s) => s.type)).toEqual(['ref', 'ref']);
  });

  it('leaves anything that is not a valid registered reference as text', () => {
    const cases = [
      '[x](doc:abc)', // doc ids are 32 hex, optionally behind a workspace id
      `[x](doc:${NOTE_ID.toUpperCase()})`,
      `[x](doc:Bad-${NOTE_ID})`,
      `[x](note:${NOTE_ID})`, // the old schemes are no longer references
      `[x](knowledge:${NOTE_ID})`,
      '[x](https://example.com)',
      '[x](unknown:abc123)',
      `[](doc:${NOTE_ID})`,
      `[multi\nline](doc:${NOTE_ID})`,
    ];
    for (const raw of cases) {
      expect(parseReferences(raw), raw).toEqual([{ type: 'text', text: raw }]);
    }
  });

  it('never loses or reorders characters', () => {
    const raw = `a [b](doc:${NOTE_ID}) c [d](doc:zz) e\n[f](note:ab12)`;
    const joined = parseReferences(raw)
      .map((s) => (s.type === 'text' ? s.text : s.raw))
      .join('');
    expect(joined).toBe(raw);
  });
});

describe('serializeReference', () => {
  it('writes the markdown link the model sees', () => {
    expect(serializeReference({ kind: 'doc', id: NOTE_ID, title: 'Default：Agent Mock' })).toBe(
      `[Default：Agent Mock](doc:${NOTE_ID})`,
    );
  });

  it('flattens newlines and drops brackets in the title', () => {
    expect(serializeReference({ kind: 'doc', id: NOTE_ID, title: 'a[b]\nc' })).toBe(
      `[ab c](doc:${NOTE_ID})`,
    );
  });

  it('round-trips through the parser', () => {
    const raw = serializeReference({ kind: 'doc', id: NOTE_ID, title: 'T' });
    expect(parseReferences(raw)[0]).toMatchObject({ type: 'ref', kind: 'doc', id: NOTE_ID, title: 'T' });
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
