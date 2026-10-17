import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readFrontendJs, readNotesViewerSource } from '../helpers/read-frontend-js.js';
import * as identity from '../../frontend/src/doc-editor/identity.ts';
import * as docEditor from '../../frontend/src/doc-editor/index.ts';
import { docKey } from '../../frontend/src/doc-editor/identity.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  if (rel.startsWith('frontend/src/')) return readFrontendJs(rel);
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('doc-editor identity and comment cut', () => {
  it('builds the prefixed doc key only', () => {
    expect(docKey('20260827/why-cache.md')).toBe('doc:20260827/why-cache.md');
    expect(identity).not.toHaveProperty('notesDocKey');
    expect(identity).not.toHaveProperty('knowledgeDocKey');
    expect(identity).not.toHaveProperty('todosDocKey');
    expect(docEditor).not.toHaveProperty('todosDocKey');
    expect(docEditor).not.toHaveProperty('bindTodoDocHighlights');
  });

  it('does not ship todos identity or todo bind', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/doc-editor/todo-bind.ts'))).toBe(false);
    const identitySrc = read('frontend/src/doc-editor/identity.ts');
    expect(identitySrc).not.toMatch(/todos:|notes:|knowledge:/);
    expect(identitySrc).not.toMatch(/todosDocKey/);
    const indexSrc = read('frontend/src/doc-editor/index.ts');
    expect(indexSrc).not.toMatch(/todosDocKey|todo-bind|bindTodoDocHighlights/);
  });

  it('common overlay does not import comments', () => {
    const src = read('frontend/src/doc-editor/highlights.ts');
    expect(src).not.toMatch(/comments\.js|kb-comments/);
    expect(src).toMatch(/fetchDocHighlights|updateDocHighlights/);
    expect(src).not.toMatch(/updateHighlight\(|updateKbHighlight\(/);
  });

  it('Notes wire the common view and overlay', () => {
    expect(readNotesViewerSource()).toMatch(/renderDocMarkdown/);
    expect(readNotesViewerSource()).toMatch(/setDocEditMode/);
    expect(readNotesViewerSource()).toMatch(/applyCachedHighlights|initDocHighlightOverlay/);
    expect(readNotesViewerSource()).not.toMatch(/updateHighlight\(/);
    expect(existsSync(join(repoRoot, 'frontend/src/todo-task/page.tsx'))).toBe(false);
  });
});
