/**
 * Note feature AC gate (tech-doc §VF AC-1～AC-11 / T10 / §SK-P3).
 * Static probes lock failure signals; behavioral coverage lives in
 * note-assistant / viewer-create-note / router / mount / list-return suites + notes.rs.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readFrontendJs, readNotesViewerSource, readShellHtml } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  if (rel.startsWith('frontend/src/')) return readFrontendJs(rel);
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('Note AC gate (tech-doc VF / T-13)', () => {
  it('AC-1: conflict / history probes lock Primary raw+index non-mutation', () => {
    const rust = read('src-tauri/src/unit-tests/services/notes.rs');
    expect(rust).toMatch(/fn create_jot_second_write_gets_new_filename/);
    expect(rust).toMatch(/fn create_jot_rejects_empty_body_without_writing/);
    expect(rust).toMatch(/fn create_note_second_write_gets_new_filename/);
    expect(rust).toMatch(/assert_ne!\(first\["common_path"\], second\["common_path"\]\)/);
    expect(rust).toMatch(/assert_eq!\(index_after, index_before\)/);
  });

  it('AC-2: FAB Top-N ≤3 by created_at only; open via cta:open-entry', () => {
    const assistant = read('frontend/src/notes/ui/assistant.tsx');
    const assistantCommands = read('frontend/src/notes/commands/assistant.ts');
    const selectors = read('frontend/src/notes/state/selectors.ts');
    expect(assistantCommands).toMatch(/selectTopNotesByCreatedAt/);
    expect(selectors).toMatch(/export function selectTopNotesByCreatedAt/);
    expect(selectors).toMatch(/\.slice\(0,\s*3\)/);
    expect(selectors).toMatch(/created_at/);
    // Sort comparator must not read updated_at
    expect(selectors).not.toMatch(/b\.updated_at|a\.updated_at/);
    expect(assistant).toMatch(/cta:open-entry/);

    const behavioral = read('tests/notes/assistant.test.js');
    expect(behavioral).toMatch(/sorts created_at desc, slices to ≤3/);
    expect(behavioral).toMatch(/ignores updated_at when ordering/);
    expect(behavioral).toMatch(/dispatches cta:open-entry/);
  });

  it('AC-3: create append-only via createNote(source_type=jot); not Overlay', () => {
    // Read create.js alone (no re-export follow) so comment write APIs elsewhere do not pollute.
    const create = readFileSync(join(repoRoot, 'frontend/src/notes/commands/viewer/create.ts'), 'utf8');
    expect(create).toMatch(
      /createNote\(\{\s*body:\s*trimmed,\s*source_type:\s*'jot'\s*\}\)/,
    );
    // finalizeCreateSession must not call Annotation write APIs
    expect(create).not.toMatch(/updateComments|update_comments|saveAnnotation/);
    const writeMap = read('frontend/src/host/writeApiInvokeMap.ts');
    expect(writeMap).toMatch(/\/api\/create-note/);
    expect(writeMap).toMatch(/create_note/);

    const rust = read('src-tauri/src/unit-tests/services/notes.rs');
    expect(rust).toMatch(/fn create_jot_writes_raw_index_with_source_type_jot/);
    expect(rust).toMatch(/assert_eq!\(entry\["source_type"\], json!\("jot"\)\)/);
    expect(rust).toMatch(/contains\(&json!\("raw"\)\)/);
    expect(rust).not.toMatch(/annotations/);
  });

  it('AC-4: create/edit share viewer.js shell — no second editor module', () => {
    const viewer = readNotesViewerSource();
    expect(viewer).toMatch(/export async function openDoc\s*\(/);
    expect(viewer).toMatch(/export async function openCreateNote\s*\(/);
    expect(existsSync(join(repoRoot, 'frontend/src/components/note-editor.js'))).toBe(false);
    expect(existsSync(join(repoRoot, 'frontend/src/note-editor.js'))).toBe(false);
  });

  it('AC-5: management surfaces have no note create; CRUD stays save_entry/delete_entry', () => {
    for (const rel of [
      'frontend/src/notes/ui/sidebar.tsx',
      'frontend/src/notes/commands/sidebar.ts',
      'frontend/src/notes/ui/cards.tsx',
      'frontend/src/notes/commands/cards.ts',
      'frontend/src/notes/commands/assistant.ts',
      'frontend/src/notes/commands/delete-dialog.ts',
      'frontend/src/notes/commands/move-project-dialog.ts',
      'frontend/src/notes/commands/viewer/doc.ts',
    ]) {
      const src = read(rel);
      expect(src, rel).not.toMatch(/openCreateNote/);
      expect(src, rel).not.toMatch(/新建随记/);
      expect(src, rel).not.toMatch(/createNote/);
    }
    // Home nav hosts the create entry (T-notes-entry). It may call openCreateNote,
    // but must not call createNote itself or restore the retired add-note copy.
    const home = read('frontend/src/home/page.tsx');
    expect(home).not.toMatch(/新建随记/);
    expect(home).not.toMatch(/(?<!open)createNote/);
    const writeMap = read('frontend/src/host/writeApiInvokeMap.ts');
    expect(writeMap).toMatch(/cmd:\s*'save_entry'/);
    const syncMap = read('frontend/src/host/syncApiInvokeMap.ts');
    expect(syncMap).toMatch(/cmd:\s*'delete_entry'/);
    // management CRUD stays save/delete; create_note is the write command
    expect(writeMap).not.toMatch(/create_entry/);
  });

  it('AC-6: Overlay user-visible copy is 批注 (not 添加笔记)', () => {
    for (const rel of [
      'frontend/src/notes/ui/comments.tsx',
      'frontend/src/knowledge/ui/comments.tsx',
    ]) {
      const src = read(rel);
      expect(src, rel).toMatch(/Comment/);
      expect(src, rel).not.toMatch(/添加笔记|编辑笔记|删除笔记/);
    }
    const indexSrc = readShellHtml();
    expect(indexSrc).toMatch(/Comment/);
    expect(indexSrc).not.toMatch(/添加笔记|编辑笔记|删除笔记/);
  });

  it('AC-7: archive failure keeps create session + draft (behavioral probe present)', () => {
    const viewer = readNotesViewerSource();
    expect(viewer).toMatch(/session\.status = 'creating'/);
    expect(viewer).toMatch(/alert\(`Save failed: \$\{e\.message\}`\)/);
    const behavioral = read('tests/notes/viewer-create-note.test.js');
    expect(behavioral).toMatch(/archive failure: alert, keep create session \+ draft/);
    expect(behavioral).toMatch(/no Annotation API/);
  });

  it('AC-8: empty exit clears draft, no Primary create (behavioral probe present)', () => {
    const viewer = readNotesViewerSource();
    expect(viewer).toMatch(/if \(!trimmed\)/);
    expect(viewer).toMatch(/clearNoteDraft\(session\.tempId\)/);
    const behavioral = read('tests/notes/viewer-create-note.test.js');
    expect(behavioral).toMatch(/empty exit: clear draft \+ closeModal, no Primary create/);
  });

  it('AC-9: create locks source_type/topic — no mutation controls', () => {
    const viewer = readNotesViewerSource();
    expect(viewer).not.toMatch(/create-source-type|create-topic|name=["']source_type["']/);
    const behavioral = read('tests/notes/viewer-create-note.test.js');
    expect(behavioral).toMatch(/create flow has no source_type\/topic mutation controls/);
  });

  it('AC-10: create success navigates note=common_path; empty exit lands list via navigateBackToList', () => {
    const viewer = readNotesViewerSource();
    // create success: navigateToNote with createNote common_path (tech-doc T5 / chap-ar)
    expect(viewer).toMatch(/navigateToNote/);
    expect(viewer).toMatch(/common_path/);
    expect(viewer).toMatch(/navigateBackToList/);
    // Empty create exit must leave via navigateBackToList (ban-hash / AC-10 retired — T10 / VF)
    expect(viewer).not.toMatch(/dismissViewerModal\(\{\s*navigate:\s*false/);
    expect(viewer).not.toMatch(/do not mutate hash \(legacy AC-10/);
    expect(viewer).not.toMatch(/navigate\(['"]#\/home['"]\)/);
    const behavioral = read('tests/notes/viewer-create-note.test.js');
    expect(behavioral).toMatch(/navigate note=common_path/);
    expect(behavioral).toMatch(
      /empty exit lands list via navigateBackToList \(history\.back or list location\)/,
    );
  });

  it('VF micro-routing probes: layer / scroll / delete-replace / Dialog / safe-empty / create-fail', () => {
    // Lock peer suites that prove navigate/history/list location — not modal.display alone (chap-vf)
    const router = read('tests/router/index.test.js');
    expect(router).toMatch(/history\.back/);
    expect(router).toMatch(/navigateBackToList/);
    expect(router).toMatch(/abolishes exit-must-not-change-hash/);

    const openEntry = read('tests/notes/open-entry-navigate.test.js');
    expect(openEntry).toMatch(/layer/);
    expect(openEntry).toMatch(/navigate-to-note/);
    expect(openEntry).not.toMatch(/empty exit only closeModal — does not mutate hash/);

    const listReturn = read('tests/notes/list-return-delete.test.js');
    expect(listReturn).toMatch(/cta_scroll_/);
    expect(listReturn).toMatch(/location\.replace/);
    expect(listReturn).toMatch(/#\/spark\?date=/);

    const dialogAudit = read('tests/main/dialog-removal-audit.test.js');
    expect(dialogAudit).toMatch(/navigateBackToList/);
    expect(dialogAudit).toMatch(/no #md-modal display semantics/);

    const mount = read('tests/main/spark-route.test.js');
    expect(mount).toMatch(/safe-empty/);
    expect(mount).toMatch(/unresolved note/);
    expect(mount).toMatch(/layer/);

    const create = read('tests/notes/viewer-create-note.test.js');
    expect(create).toMatch(/archive failure: alert, keep create session \+ draft/);
    expect(create).toMatch(/no Annotation API/);
    expect(create).toMatch(/navigateBackToList/);
  });

  it('AC-11: create chrome hides shell-field controls (body-only)', () => {
    const viewer = readNotesViewerSource();
    expect(viewer).toMatch(/CREATE_CHROME_HIDDEN_IDS/);
    expect(viewer).toMatch(/applyCreateChrome/);
    expect(viewer).toMatch(/is-create/);
    expect(viewer).not.toMatch(/btn-copy-http/);
    expect(viewer).toMatch(/btn-copy-path/);
    expect(viewer).toMatch(/btn-open-in-chat/);
    expect(viewer).toMatch(/createChromePrevDisplay/);
    const css = read('frontend/app.css');
    expect(css).toMatch(/\.viewer-modal\.is-create\s+\.viewer-chrome-persisted/);
    const behavioral = read('tests/notes/viewer-create-note.test.js');
    expect(behavioral).toMatch(/applies create chrome: body-only, no shell-field controls/);
  });

  it('VF falsifiable: create must not target annotations/ Overlay path', () => {
    const create = readFileSync(join(repoRoot, 'frontend/src/notes/commands/viewer/create.ts'), 'utf8');
    const finalize = create.slice(
      create.indexOf('async function finalizeCreateSession'),
      create.indexOf('export async function closeModal'),
    );
    expect(finalize).toMatch(/createNote/);
    expect(finalize).not.toMatch(/annotations/);
    expect(finalize).not.toMatch(/updateComments|fetchAnnotation/);
  });
});

describe('Notes viewport after React host', () => {
  it('app.css keeps #root / #root-shell on the body → .layout flex chain', () => {
    const css = read('frontend/app.css');
    expect(css).toMatch(/#root\s*,\s*#root-shell\s*\{[^}]*display:\s*flex/);
    expect(css).toMatch(/#root\s*,\s*#root-shell\s*\{[^}]*flex-direction:\s*column/);
    expect(css).toMatch(/#root\s*,\s*#root-shell\s*\{[^}]*flex:\s*1/);
    expect(css).toMatch(/\.layout\s*\{[^}]*flex:\s*1/);
  });
});
