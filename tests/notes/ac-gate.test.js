/**
 * Note feature AC gate (tech-doc §VF AC-1～AC-11 / T10 / §SK-P3).
 * Static probes lock failure signals; behavioral coverage lives in
 * note-assistant / viewer-create-note / router / mount / list-return suites + archive_write.rs.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('Note AC gate (tech-doc VF / T-13)', () => {
  it('AC-1: conflict / history probes lock Primary raw+index non-mutation', () => {
    const rust = read('src-tauri/src/unit-tests/services/archive_write.rs');
    expect(rust).toMatch(/fn archive_note_document_conflict_does_not_mutate_history/);
    expect(rust).toMatch(/history Entry must not change on conflict/);
    expect(rust).toMatch(/raw must not be rewritten on conflict/);
  });

  it('AC-2: FAB Top-N ≤3 by created_at only; open via cta:open-entry', () => {
    const assistant = read('frontend/js/notes/assistant.js');
    expect(assistant).toMatch(/export function selectTopNotesByCreatedAt/);
    expect(assistant).toMatch(/\.slice\(0,\s*3\)/);
    expect(assistant).toMatch(/created_at/);
    // Sort comparator must not read updated_at
    expect(assistant).not.toMatch(/b\.updated_at|a\.updated_at/);
    expect(assistant).toMatch(/cta:open-entry/);

    const behavioral = read('tests/notes/assistant.test.js');
    expect(behavioral).toMatch(/sorts created_at desc, slices to ≤3/);
    expect(behavioral).toMatch(/ignores updated_at when ordering/);
    expect(behavioral).toMatch(/dispatches cta:open-entry/);
  });

  it('AC-3: create append-only via archiveDocument(source_type=note); not Overlay', () => {
    const viewer = read('frontend/js/notes/viewer.js');
    expect(viewer).toMatch(
      /archiveDocument\(\{\s*body:\s*trimmed,\s*source_type:\s*'note'\s*\}\)/,
    );
    // finalizeCreateSession must not call Annotation write APIs
    expect(viewer).not.toMatch(/updateComments|update_comments|saveAnnotation/);
    const writeMap = read('frontend/js/host/writeApiInvokeMap.js');
    expect(writeMap).toMatch(/\/api\/archive-document/);
    expect(writeMap).toMatch(/archive_document/);

    const rust = read('src-tauri/src/unit-tests/services/archive_write.rs');
    expect(rust).toMatch(/fn archive_note_document_writes_raw_index_with_source_type_note/);
    expect(rust).toMatch(/must not write Annotation path/);
  });

  it('AC-4: create/edit share viewer.js shell — no second editor module', () => {
    const viewer = read('frontend/js/notes/viewer.js');
    expect(viewer).toMatch(/export async function openDoc\s*\(/);
    expect(viewer).toMatch(/export async function openCreateNote\s*\(/);
    expect(existsSync(join(repoRoot, 'frontend/js/components/note-editor.js'))).toBe(false);
    expect(existsSync(join(repoRoot, 'frontend/js/note-editor.js'))).toBe(false);
  });

  it('AC-5: management surfaces have no note create; CRUD stays save_entry/delete_entry', () => {
    for (const rel of [
      'frontend/js/notes/sidebar.js',
      'frontend/js/notes/cards.js',
      'frontend/js/home-entry-shell/hub.js',
    ]) {
      const src = read(rel);
      expect(src, rel).not.toMatch(/openCreateNote/);
      expect(src, rel).not.toMatch(/新建随记/);
      expect(src, rel).not.toMatch(/archiveDocument/);
    }
    const writeMap = read('frontend/js/host/writeApiInvokeMap.js');
    expect(writeMap).toMatch(/cmd:\s*'save_entry'/);
    const syncMap = read('frontend/js/host/syncApiInvokeMap.js');
    expect(syncMap).toMatch(/cmd:\s*'delete_entry'/);
    // create for notes is archive_document, not a management create_*_note
    expect(writeMap).not.toMatch(/create_note|create_entry/);
  });

  it('AC-6: Overlay user-visible copy is 批注 (not 添加笔记)', () => {
    for (const rel of [
      'frontend/js/notes/comments.js',
      'frontend/js/corpus/corpus-comments.js',
      'frontend/js/corpus/corpus-viewer.js',
    ]) {
      const src = read(rel);
      expect(src, rel).toMatch(/Comment/);
      expect(src, rel).not.toMatch(/添加笔记|编辑笔记|删除笔记/);
    }
    const indexSrc = read('frontend/index.html');
    expect(indexSrc).toMatch(/Comment/);
    expect(indexSrc).not.toMatch(/添加笔记|编辑笔记|删除笔记/);
  });

  it('AC-7: archive failure keeps create session + draft (behavioral probe present)', () => {
    const viewer = read('frontend/js/notes/viewer.js');
    expect(viewer).toMatch(/session\.status = 'creating'/);
    expect(viewer).toMatch(/alert\(`Save failed: \$\{e\.message\}`\)/);
    const behavioral = read('tests/notes/viewer-create-note.test.js');
    expect(behavioral).toMatch(/archive failure: alert, keep create session \+ draft/);
    expect(behavioral).toMatch(/no Annotation API/);
  });

  it('AC-8: empty exit clears draft, no Primary create (behavioral probe present)', () => {
    const viewer = read('frontend/js/notes/viewer.js');
    expect(viewer).toMatch(/if \(!trimmed\)/);
    expect(viewer).toMatch(/clearNoteDraft\(session\.tempId\)/);
    const behavioral = read('tests/notes/viewer-create-note.test.js');
    expect(behavioral).toMatch(/empty exit: clear draft \+ closeModal, no Primary create/);
  });

  it('AC-9: create locks source_type/topic — no mutation controls', () => {
    const viewer = read('frontend/js/notes/viewer.js');
    expect(viewer).not.toMatch(/create-source-type|create-topic|name=["']source_type["']/);
    const behavioral = read('tests/notes/viewer-create-note.test.js');
    expect(behavioral).toMatch(/create flow has no source_type\/topic mutation controls/);
  });

  it('AC-10: create success navigates note=common_path; empty exit lands list via navigateBackToList', () => {
    const viewer = read('frontend/js/notes/viewer.js');
    // create success: navigateToNote with archiveDocument common_path (tech-doc T5 / chap-ar)
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
    expect(listReturn).toMatch(/#\/workbench\?date=/);

    const dialogAudit = read('tests/main/dialog-removal-audit.test.js');
    expect(dialogAudit).toMatch(/navigateBackToList/);
    expect(dialogAudit).toMatch(/no #md-modal display semantics/);

    const mount = read('tests/main/workbench-route.test.js');
    expect(mount).toMatch(/safe-empty/);
    expect(mount).toMatch(/unresolved note/);
    expect(mount).toMatch(/layer/);

    const create = read('tests/notes/viewer-create-note.test.js');
    expect(create).toMatch(/archive failure: alert, keep create session \+ draft/);
    expect(create).toMatch(/no Annotation API/);
    expect(create).toMatch(/navigateBackToList/);
  });

  it('AC-11: create chrome hides shell-field controls (body-only)', () => {
    const viewer = read('frontend/js/notes/viewer.js');
    expect(viewer).toMatch(/CREATE_CHROME_HIDDEN_IDS/);
    expect(viewer).toMatch(/applyCreateChrome/);
    expect(viewer).toMatch(/is-create/);
    expect(viewer).toMatch(/btn-copy-http/);
    expect(viewer).toMatch(/btn-copy-path/);
    expect(viewer).toMatch(/createChromePrevDisplay/);
    const css = read('frontend/app.css');
    expect(css).toMatch(/\.viewer-modal\.is-create\s+\.viewer-chrome-persisted/);
    const behavioral = read('tests/notes/viewer-create-note.test.js');
    expect(behavioral).toMatch(/applies create chrome: body-only, no shell-field controls/);
  });

  it('VF falsifiable: create must not target annotations/ Overlay path', () => {
    const viewer = read('frontend/js/notes/viewer.js');
    const finalize = viewer.slice(
      viewer.indexOf('async function finalizeCreateSession'),
      viewer.indexOf('export async function closeModal'),
    );
    expect(finalize).toMatch(/archiveDocument/);
    expect(finalize).not.toMatch(/annotations/);
    expect(finalize).not.toMatch(/updateComments|fetchAnnotation/);
  });
});
