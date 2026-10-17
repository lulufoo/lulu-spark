// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const builtin = [
  { id: 'summary', label: 'Summary', exposed: true },
  { id: 'dialogue', label: 'Dialogue', exposed: true },
  { id: 'video', label: 'Video', exposed: true },
  { id: 'article', label: 'Article', exposed: true },
  { id: 'jot', label: 'Jot', exposed: false },
];

let stored = [];
let failNext = '';
const calls = [];

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchDocCategories: async () => stored.map((row) => ({ ...row })),
  saveDocCategories: async (next) => {
    calls.push(['save', next.map((row) => ({ ...row }))]);
    if (failNext) {
      const error = failNext;
      failNext = '';
      throw new Error(error);
    }
    stored = next.map((row) => ({ ...row }));
    return stored.map((row) => ({ ...row }));
  },
}));

const commands = await import('../../frontend/src/app-shell/commands/settings/document-categories.ts');
const { documentCategoriesStore, emptyDocumentCategoriesSnap } = await import(
  '../../frontend/src/app-shell/state/settings/document-categories.ts'
);
const { DocumentPanel } = await import('../../frontend/src/app-shell/ui/settings/document-panel.tsx');

const snap = () => documentCategoriesStore.getSnapshot();
const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('Settings Document categories', () => {
  beforeEach(() => {
    stored = builtin.map((row) => ({ ...row }));
    failNext = '';
    calls.length = 0;
    documentCategoriesStore.set(emptyDocumentCategoriesSnap());
  });

  it('loads every item including hidden jot', async () => {
    await commands.loadDocCategories();
    expect(snap().rows).toEqual(builtin);
    expect(snap().rows.find((row) => row.id === 'jot')?.exposed).toBe(false);
  });

  it('toggles jot onto MCP and writes the full list', async () => {
    await commands.loadDocCategories();
    await commands.toggleCategoryExposed('jot', true);
    expect(calls).toHaveLength(1);
    expect(calls[0][1].find((row) => row.id === 'jot')?.exposed).toBe(true);
    expect(snap().rows.find((row) => row.id === 'jot')?.exposed).toBe(true);
  });

  it('slugs the name into a lowercase-hyphen id, defaulting to exposed', async () => {
    await commands.loadDocCategories();
    commands.setNewCategoryLabel('Theme line');
    expect(snap().newId).toBe('theme-line');
    expect(snap().newExposed).toBe(true);
    expect(await commands.addDocCategory()).toBe(true);
    expect(snap().newId).toBe('');
    expect(stored.at(-1)).toEqual({ id: 'theme-line', label: 'Theme line', exposed: true });
  });

  it('rejects empty, duplicate, and names that cannot slug before writing', async () => {
    await commands.loadDocCategories();
    expect(await commands.addDocCategory()).toBe(false);
    expect(snap().error).toMatch(/empty/i);
    commands.setNewCategoryLabel('Summary');
    expect(await commands.addDocCategory()).toBe(false);
    expect(snap().error).toMatch(/duplicate/i);
    commands.setNewCategoryLabel('***');
    expect(await commands.addDocCategory()).toBe(false);
    expect(snap().error).toMatch(/Could not make an id/);
    expect(calls).toEqual([]);
  });

  it('deletes from the list only', async () => {
    await commands.loadDocCategories();
    await commands.removeDocCategory('article');
    expect(stored.map((row) => row.id)).toEqual(['summary', 'dialogue', 'video', 'jot']);
  });

  it('reloads the list when the host refuses a write', async () => {
    await commands.loadDocCategories();
    failNext = 'category id cannot contain whitespace';
    await commands.toggleCategoryExposed('jot', true);
    expect(snap().error).toBe('category id cannot contain whitespace');
    expect(snap().rows.find((row) => row.id === 'jot')?.exposed).toBe(false);
    expect(snap().busy).toBe(false);
  });

  it('paints Categories/Add tabs and a card-shaped add row', async () => {
    await commands.loadDocCategories();
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(DocumentPanel)));
    expect(host.querySelector('#settings-panel-document')).not.toBeNull();
    expect(host.querySelector('[data-tab="list"]')?.textContent).toBe('Categories');
    expect(host.querySelector('[data-tab="add"]')?.textContent).toBe('Add');
    expect(host.querySelector('#settings-tab-document-list')).not.toBeNull();
    expect(host.querySelector('#settings-tab-document-add')).not.toBeNull();
    expect(host.querySelector('[data-doc-cat-id="jot"]')?.tagName).toBe('CODE');
    expect(host.querySelector('input[data-doc-cat-id]')).toBeNull();
    expect(host.querySelector('[data-doc-cat-exposed="jot"]').checked).toBe(false);
    expect(host.querySelector('#doc-cat-new-exposed').checked).toBe(true);
    expect(host.querySelector('#btn-doc-cat-add').textContent).toBe('Add');
    expect(host.querySelector('input#doc-cat-new-id')).toBeNull();
    expect(host.querySelector('#doc-cat-new-id')?.tagName).toBe('CODE');
    expect(host.querySelector('#doc-cat-new-label')?.placeholder).toBe('Name');
    expect(host.querySelector('label[for="doc-cat-new-label"]')).toBeNull();
    expect(host.querySelector('[data-doc-cat-remove="jot"]')?.textContent).toBe('Remove');
    expect(host.textContent).toContain('cannot change');
    expect(host.textContent).toContain('id is generated from the name');
    expect(host.textContent).not.toContain('list_doc_categories');
    expect(host.textContent).not.toContain('Exposed to agents');
    act(() => root.unmount());
  });

  it('does not reuse the old notes-categories path-folder UI', () => {
    const panel = readFileSync(
      join(repoRoot, 'frontend/src/app-shell/ui/settings/document-panel.tsx'),
      'utf8',
    );
    expect(panel).not.toMatch(/notes-cat/);
    expect(panel).toContain('id="settings-panel-document"');
  });
});
