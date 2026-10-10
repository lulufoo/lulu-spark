import { openKnowledgeById } from '../commands/open-knowledge-link.ts';
import { openNoteById } from '../commands/open-note-link.ts';
import { registerReferenceKind } from './registry.ts';

registerReferenceKind({
  kind: 'note',
  scheme: 'note',
  label: 'note',
  idPattern: /^[0-9a-f]{32}$/,
  open: openNoteById,
});

// The id length is not checked here: Rust decides whether the id exists (legacy ids are shorter).
registerReferenceKind({
  kind: 'knowledge',
  scheme: 'knowledge',
  label: 'knowledge',
  idPattern: /^[0-9a-f]+$/,
  open: openKnowledgeById,
});
