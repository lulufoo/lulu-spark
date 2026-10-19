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
  it('registers doc and read-later out of the box; note, knowledge, and doc are gone', () => {
    const doc = referenceKindByScheme('spark-doc');
    const readLater = referenceKindByScheme('read-later');
    expect(doc?.kind).toBe('doc');
    expect(doc?.label).toBe('');
    expect(referenceKindByScheme('doc')).toBeUndefined();
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
    const raw = `read [Default：Agent Mock](spark-doc:${NOTE_ID}) first`;
    expect(parseReferences(raw)).toEqual([
      { type: 'text', text: 'read ' },
      { type: 'ref', kind: 'doc', id: NOTE_ID, title: 'Default：Agent Mock', raw: `[Default：Agent Mock](spark-doc:${NOTE_ID})` },
      { type: 'text', text: ' first' },
    ]);
  });

  it('accepts a 32-hex doc id and leaves a workspace-prefixed one as text', () => {
    const plain = parseReferences(`[A](spark-doc:${NOTE_ID})`);
    expect(plain[0]).toMatchObject({ type: 'ref', kind: 'doc', id: NOTE_ID });
    const raw = `[B](spark-doc:k3x9ab-${NOTE_ID})`;
    expect(parseReferences(raw)).toEqual([{ type: 'text', text: raw }]);
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
    const segments = parseReferences(`[A](spark-doc:${NOTE_ID})[B](spark-doc:${READ_LATER_ID})`);
    expect(segments.map((s) => s.type)).toEqual(['ref', 'ref']);
  });

  it('leaves anything that is not a valid registered reference as text', () => {
    const cases = [
      '[x](spark-doc:abc)', // ids are 32 hex
      `[x](spark-doc:${NOTE_ID.toUpperCase()})`,
      `[x](spark-doc:Bad-${NOTE_ID})`,
      `[x](doc:${NOTE_ID})`, // the old scheme is no longer a reference
      `[x](note:${NOTE_ID})`,
      `[x](knowledge:${NOTE_ID})`,
      '[x](https://example.com)',
      '[x](unknown:abc123)',
      `[](spark-doc:${NOTE_ID})`,
      `[multi\nline](spark-doc:${NOTE_ID})`,
    ];
    for (const raw of cases) {
      expect(parseReferences(raw), raw).toEqual([{ type: 'text', text: raw }]);
    }
  });

  it('never loses or reorders characters', () => {
    const raw = `a [b](spark-doc:${NOTE_ID}) c [d](spark-doc:zz) e\n[f](doc:${NOTE_ID})`;
    const joined = parseReferences(raw)
      .map((s) => (s.type === 'text' ? s.text : s.raw))
      .join('');
    expect(joined).toBe(raw);
  });
});

describe('serializeReference', () => {
  it('writes the markdown link the model sees', () => {
    expect(serializeReference({ kind: 'doc', id: NOTE_ID, title: 'Default：Agent Mock' })).toBe(
      `[Default：Agent Mock](spark-doc:${NOTE_ID})`,
    );
  });

  it('flattens newlines and drops brackets in the title', () => {
    expect(serializeReference({ kind: 'doc', id: NOTE_ID, title: 'a[b]\nc' })).toBe(
      `[ab c](spark-doc:${NOTE_ID})`,
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
