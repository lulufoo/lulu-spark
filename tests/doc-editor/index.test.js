import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readFrontendJs } from '../helpers/read-frontend-js.js';
import {
  knowledgeDocKey,
  notesDocKey,
  todosDocKey,
} from '../../frontend/src/doc-editor/identity.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  if (rel.startsWith('frontend/src/')) return readFrontendJs(rel);
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('doc-editor identity and comment cut', () => {
  it('builds prefixed keys for Notes / Knowledge / Todos', () => {
    expect(notesDocKey('20260827/why-cache.md')).toBe('notes:20260827/why-cache.md');
    expect(knowledgeDocKey('acme/docs', 'foo.md')).toBe('knowledge:acme/docs/foo.md');
    expect(todosDocKey('task_1')).toBe('todos:task_1');
    expect(todosDocKey('task_1', 'att:plan.md')).toBe('todos:task_1:att:plan.md');
  });

  it('common overlay does not import comments', () => {
    const src = read('frontend/src/doc-editor/highlights.ts');
    expect(src).not.toMatch(/comments\.js|kb-comments/);
    expect(src).toMatch(/fetchDocHighlights|updateDocHighlights/);
    expect(src).not.toMatch(/updateHighlight\(|updateKbHighlight\(/);
  });

  it('Notes / Knowledge / Todos wire the common view and overlay', () => {
    expect(read('frontend/src/notes/viewer.ts')).toMatch(/renderDocMarkdown/);
    expect(read('frontend/src/notes/viewer.ts')).toMatch(/setDocEditMode/);
    expect(read('frontend/src/notes/viewer.ts')).toMatch(/applyCachedHighlights|initDocHighlightOverlay/);
    expect(read('frontend/src/notes/viewer.ts')).not.toMatch(/updateHighlight\(/);
    expect(read('frontend/src/corpus/corpus-viewer.ts')).toMatch(/renderDocMarkdown/);
    expect(read('frontend/src/corpus/corpus-viewer.ts')).toMatch(/setDocEditMode/);
    expect(read('frontend/src/corpus/corpus-viewer.ts')).toMatch(/applyCachedHighlights|initDocHighlightOverlay/);
    expect(read('frontend/src/corpus/corpus-viewer.ts')).not.toMatch(/updateKbHighlight\(/);
    expect(read('frontend/src/todo-task/page.tsx')).toMatch(/bindTodoDocHighlights/);
  });
});
