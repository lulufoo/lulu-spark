/** A kind of thing a message can point at. Add a kind here; the parser, composer and chat render follow. */
export type ReferenceKind = {
  kind: string;
  /** The link scheme in `[title](scheme:id)`. */
  scheme: string;
  /** The word shown on the chip before the title. */
  label: string;
  /** What a valid id looks like; anything else stays plain text. */
  idPattern: RegExp;
  open(id: string): void | Promise<void>;
};

const byScheme = new Map<string, ReferenceKind>();
const byKind = new Map<string, ReferenceKind>();

/** Registering the same kind again replaces it. */
export function registerReferenceKind(definition: ReferenceKind): void {
  const previous = byKind.get(definition.kind);
  if (previous) byScheme.delete(previous.scheme);
  byKind.set(definition.kind, definition);
  byScheme.set(definition.scheme, definition);
}

export function referenceKindByScheme(scheme: string): ReferenceKind | undefined {
  return byScheme.get(scheme);
}

export function referenceKindByName(kind: string): ReferenceKind | undefined {
  return byKind.get(kind);
}
