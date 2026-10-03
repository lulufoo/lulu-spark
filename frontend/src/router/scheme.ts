import { state } from '../host/state.ts';
import { openReadLaterDialog } from '../read-later/commands/dialog.ts';
import { navigateToNote } from './index.ts';

export type WorkbenchEnvelope = {
  business: string;
  action: string;
  params: Record<string, unknown>;
};

export type ParsedWorkbenchScheme =
  | { kind: 'notes-open'; id: string; path: string }
  | { kind: 'read-later-list' };

export type NotesLanding = { date: string; note: string };

function stringParam(params: Record<string, unknown>, key: string): string {
  const value = params[key];
  return typeof value === 'string' ? value : '';
}

export function composeWorkbenchScheme(
  envelope: WorkbenchEnvelope,
): string | null {
  const params = envelope.params;
  if (!params || typeof params !== 'object') return null;

  if (envelope.business === 'notes' && (envelope.action === 'create' || envelope.action === 'update')) {
    const id = stringParam(params, 'id');
    const path = stringParam(params, 'common_path');
    if (!id || !path) return null;
    return `workbench://notes/open?id=${encodeURIComponent(id)}&path=${encodeURIComponent(path)}`;
  }

  if (envelope.business === 'read_later' && envelope.action === 'create') {
    return 'workbench://read-later/list';
  }

  return null;
}

export function parseWorkbenchScheme(scheme: string): ParsedWorkbenchScheme | null {
  if (typeof scheme !== 'string' || !scheme.startsWith('workbench://')) return null;

  let url: URL;
  try {
    url = new URL(scheme);
  } catch {
    return null;
  }

  if (url.protocol !== 'workbench:') return null;
  const path = url.pathname.replace(/\/+$/, '');

  if (url.hostname === 'notes' && path === '/open') {
    const id = url.searchParams.get('id') ?? '';
    const notePath = url.searchParams.get('path') ?? '';
    if (!id || !notePath) return null;
    return { kind: 'notes-open', id, path: notePath };
  }

  if (url.hostname === 'read-later' && path === '/list') {
    if (url.search) return null;
    return { kind: 'read-later-list' };
  }

  return null;
}

export function resolveNotesLanding(id: string, path: string): NotesLanding | null {
  const data = state.index.data;
  if (!data) return null;

  const byId = id
    ? data[id] ?? Object.values(data).find((item) => item._id === id)
    : undefined;
  const entry = byId ?? Object.values(data).find((item) => path && item.common_path === path);

  const created = typeof entry?.created_at === 'string' ? entry.created_at : '';
  const note = typeof entry?.common_path === 'string' ? entry.common_path : '';
  if (created.length < 8 || !note) return null;
  return { date: created.slice(0, 8), note };
}

export function openWorkbenchScheme(scheme: string): boolean {
  const parsed = parseWorkbenchScheme(scheme);
  if (!parsed) return false;

  if (parsed.kind === 'read-later-list') {
    openReadLaterDialog();
    return true;
  }

  const landing = resolveNotesLanding(parsed.id, parsed.path);
  if (!landing) return false;
  return navigateToNote({ date: landing.date, note: landing.note });
}
