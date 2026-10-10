import { referenceKindByName } from './registry.ts';
import { serializeReference, type ReferenceValue } from './parse.ts';

export const CHIP_CLASS = 'home-ref-chip';
export const CHIP_LABEL_CLASS = 'home-ref-chip-label';

/** The words on a chip: `note: Title`. */
export function chipLabelText(kind: string, title: string): string {
  return `${referenceKindByName(kind)?.label ?? kind}: ${title}`;
}

/** A read-only inline chip for one reference. The markdown link stays recoverable from its data. */
export function createChipElement(doc: Document, ref: ReferenceValue): HTMLElement {
  const chip = doc.createElement('span');
  chip.className = CHIP_CLASS;
  chip.setAttribute('contenteditable', 'false');
  chip.setAttribute('data-ref-kind', ref.kind);
  chip.setAttribute('data-ref-id', ref.id);
  chip.setAttribute('data-ref-title', ref.title);
  chip.setAttribute('title', ref.title);
  const label = doc.createElement('span');
  label.className = CHIP_LABEL_CLASS;
  label.textContent = chipLabelText(ref.kind, ref.title);
  chip.appendChild(label);
  return chip;
}

export function chipFromNode(node: Node | null): HTMLElement | null {
  const element = node instanceof Element ? node : node?.parentElement;
  return element?.closest<HTMLElement>(`.${CHIP_CLASS}`) ?? null;
}

export function chipValue(chip: HTMLElement): ReferenceValue | null {
  const kind = chip.getAttribute('data-ref-kind');
  const id = chip.getAttribute('data-ref-id');
  if (!kind || !id) return null;
  return { kind, id, title: chip.getAttribute('data-ref-title') ?? '' };
}

export function serializeChip(chip: HTMLElement): string {
  const value = chipValue(chip);
  return value ? serializeReference(value) : (chip.textContent ?? '');
}
