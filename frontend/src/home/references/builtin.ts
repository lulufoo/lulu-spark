import { openDocById } from '../commands/open-doc-link.ts';
import { openReadLaterById } from '../commands/open-read-later-link.ts';
import { registerReferenceKind } from './registry.ts';

registerReferenceKind({
  kind: 'doc',
  scheme: 'spark-doc',
  label: '',
  idPattern: /^(?:[a-z0-9_]+-)?[0-9a-f]{32}$/,
  open: openDocById,
});

registerReferenceKind({
  kind: 'read-later',
  scheme: 'read-later',
  label: 'read-later',
  idPattern: /^[0-9a-f]{32}$/,
  open: openReadLaterById,
});
