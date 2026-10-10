import { referenceKindByName, referenceKindByScheme } from './registry.ts';

export type ReferenceSegment =
  | { type: 'text'; text: string }
  | { type: 'ref'; kind: string; id: string; title: string; raw: string };

export type ReferenceValue = { kind: string; id: string; title: string };

const LINK = /\[([^\[\]\n]+)\]\(([a-z][a-z0-9-]*):([^()\s]+)\)/g;

/** Split text into plain runs and references of a registered kind with a valid id. */
export function parseReferences(text: string): ReferenceSegment[] {
  const segments: ReferenceSegment[] = [];
  const pushText = (value: string) => {
    if (!value) return;
    const last = segments[segments.length - 1];
    if (last?.type === 'text') last.text += value;
    else segments.push({ type: 'text', text: value });
  };
  let cursor = 0;
  for (const match of text.matchAll(LINK)) {
    const [raw, title, scheme, id] = match;
    const kind = referenceKindByScheme(scheme);
    const start = match.index ?? 0;
    pushText(text.slice(cursor, start));
    cursor = start + raw.length;
    if (kind && kind.idPattern.test(id)) {
      segments.push({ type: 'ref', kind: kind.kind, id, title, raw });
    } else {
      pushText(raw);
    }
  }
  pushText(text.slice(cursor));
  return segments;
}

function cleanTitle(title: string): string {
  return title.replace(/\s*\n\s*/g, ' ').replace(/[\[\]]/g, '');
}

/** The markdown link that is stored and sent to the model. */
export function serializeReference(ref: ReferenceValue): string {
  const kind = referenceKindByName(ref.kind);
  if (!kind) throw new Error(`unknown reference kind: ${ref.kind}`);
  return `[${cleanTitle(ref.title)}](${kind.scheme}:${ref.id})`;
}
