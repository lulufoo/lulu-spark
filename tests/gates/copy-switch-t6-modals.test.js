import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readFrontendJs, readMainSource } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const mainJs = [
  readMainSource(),
  readFrontendJs('frontend/src/notes/page.tsx'),
].join('\n');
const sparkCommitJs = [
  readFrontendJs('frontend/src/app-shell/ui/spark-commit-dialog.tsx'),
  readFrontendJs('frontend/src/app-shell/commands/spark-commit-dialog.ts'),
  readFrontendJs('frontend/src/app-shell/state/spark-commit.ts'),
].join('\n');
const deleteDialogJs = readFileSync(
  join(repoRoot, 'frontend/src/notes/ui/delete-dialog.tsx'),
  'utf8',
);
const settleDialogJs = [
  readFrontendJs('frontend/src/notes/ui/settle-dialog.tsx'),
  readFrontendJs('frontend/src/notes/commands/settle-dialog.ts'),
].join('\n');
const modalSources = [
  sparkCommitJs,
  deleteDialogJs,
  settleDialogJs,
  readFrontendJs('frontend/src/knowledge/ui/knowledge-diff-dialog.tsx'),
  readFrontendJs('frontend/src/knowledge/ui/tree-delete-dialog.tsx'),
  readFrontendJs('frontend/src/notes/ui/move-project-dialog.tsx'),
  readFrontendJs('frontend/src/notes/commands/move-project-dialog.ts'),
].join('\n');


function withoutComments(src) {
  return src.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '');
}

describe('P2 copy-switch — main.js & modals (tech-doc T6)', () => {
  it('main.js uses table B/B2 English for core flow copy', () => {
    expect(mainJs).toContain('Retry');
    expect(mainJs).toContain('Updating…');
    expect(mainJs).toContain('↓ Update project');
    expect(mainJs).toContain('Could not load index.json:');
    expect(mainJs).toContain('Loading…');
    expect(mainJs).toContain('No repositories found');
    expect(mainJs).toContain('Uncategorized');
    expect(mainJs).toContain('Cloned');
    expect(mainJs).toContain('Not cloned');
    expect(mainJs).toContain('View local changes');
    expect(mainJs).toContain('Remove from curated list');
    expect(mainJs).toContain('Enter repository URL');
    expect(mainJs).toContain('Click to copy');
    expect(mainJs).toContain('Copied');
    expect(mainJs).not.toContain('重试');
    expect(mainJs).not.toContain('沉淀知识库列表');
    expect(mainJs).not.toContain('加载中');
  });

  it('spark-commit-dialog copy stays English', () => {
    expect(sparkCommitJs).toContain('↑ Spark commit');
    expect(sparkCommitJs).toContain('↑ Commit changes');
    expect(sparkCommitJs).toContain('Checking…');
    expect(sparkCommitJs).toContain('Loading…');
    expect(sparkCommitJs).toContain('No changes to commit or push');
    expect(sparkCommitJs).toContain("label: 'New'");
    expect(sparkCommitJs).toContain('Ready to push');
    expect(sparkCommitJs).toContain('local commit(s) not yet pushed');
    expect(sparkCommitJs).toContain('✓ Committed and pushed');
    expect(sparkCommitJs).not.toContain('提交');
    expect(sparkCommitJs).not.toContain('加载中');
  });

  it('knowledge tree delete requires CONFIRM in English', () => {
    const src = readFrontendJs('frontend/src/knowledge/ui/tree-delete-dialog.tsx');
    expect(src).toContain('Type <code>CONFIRM</code> to delete');
    expect(src).toContain('Confirm delete');
    expect(src).not.toContain('确认删除');
  });

  it('delete-dialog.js uses table B/B2 delete copy', () => {
    expect(deleteDialogJs).toContain('Deleting…');
    expect(deleteDialogJs).toContain('Confirm delete');
    expect(deleteDialogJs).toContain('Delete failed:');
    expect(deleteDialogJs).toContain("'Copy'");
    expect(deleteDialogJs).not.toContain('确认删除');
    expect(deleteDialogJs).not.toContain('删除中');
  });

  it('settle-dialog.js remains English (t4 baseline)', () => {
    expect(settleDialogJs).toContain('Push');
    expect(settleDialogJs).toContain('Loading folders…');
    expect(settleDialogJs).not.toMatch(/[\u4e00-\u9fff]/);
  });

  it('other modals have no user-facing Chinese (excl. comments)', () => {
    const body = withoutComments(modalSources);
    expect(body).not.toMatch(/[\u4e00-\u9fff]/);
  });

  it('main.js skills chrome is English', () => {
    expect(mainJs).toContain('Click to copy');
    expect(mainJs).not.toContain('点击复制');
    expect(mainJs).not.toContain('已复制');
    expect(mainJs).not.toContain('Lulu Learning Skills');
    expect(mainJs).not.toContain('Lulu Dev Skills');
  });

  it('t6 sources have no user-facing Chinese outside skills catalog literals', () => {
    const body = withoutComments([mainJs, modalSources].join('\n'));
    expect(body).not.toMatch(/[\u4e00-\u9fff]/);
  });
});
