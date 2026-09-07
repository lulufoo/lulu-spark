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
const workbenchCommitJs = [
  readFrontendJs('frontend/src/app-shell/ui/workbench-commit-dialog.tsx'),
  readFrontendJs('frontend/src/app-shell/commands/workbench-commit-dialog.ts'),
  readFrontendJs('frontend/src/app-shell/state/workbench-commit.ts'),
].join('\n');
const deleteDialogJs = readFileSync(
  join(repoRoot, 'frontend/src/notes/ui/delete-dialog.tsx'),
  'utf8',
);
const moveDialogJs = [
  readFrontendJs('frontend/src/app-shell/ui/move-dialog.tsx'),
  readFrontendJs('frontend/src/app-shell/commands/move-dialog.ts'),
].join('\n');
const settleDialogJs = [
  readFrontendJs('frontend/src/notes/ui/settle-dialog.tsx'),
  readFrontendJs('frontend/src/notes/commands/settle-dialog.ts'),
].join('\n');
const modalSources = [
  workbenchCommitJs,
  deleteDialogJs,
  moveDialogJs,
  settleDialogJs,
  readFrontendJs('frontend/src/knowledge/ui/knowledge-diff-dialog.tsx'),
  readFrontendJs('frontend/src/app-shell/ui/convert-dialog.tsx'),
  readFrontendJs('frontend/src/app-shell/commands/convert-dialog.ts'),
  readFrontendJs('frontend/src/notes/ui/move-project-dialog.tsx'),
  readFrontendJs('frontend/src/notes/commands/move-project-dialog.ts'),
  readFrontendJs('frontend/src/app-shell/ui/qr-dialog.tsx'),
  readFrontendJs('frontend/src/app-shell/commands/qr-dialog.ts'),
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

  it('workbench-commit-dialog copy stays English', () => {
    expect(workbenchCommitJs).toContain('↑ Workbench commit');
    expect(workbenchCommitJs).toContain('↑ Commit changes');
    expect(workbenchCommitJs).toContain('Checking…');
    expect(workbenchCommitJs).toContain('Loading…');
    expect(workbenchCommitJs).toContain('No changes to commit or push');
    expect(workbenchCommitJs).toContain("label: 'New'");
    expect(workbenchCommitJs).toContain('Ready to push');
    expect(workbenchCommitJs).toContain('local commit(s) not yet pushed');
    expect(workbenchCommitJs).toContain('✓ Committed and pushed');
    expect(workbenchCommitJs).not.toContain('提交');
    expect(workbenchCommitJs).not.toContain('加载中');
  });

  it('delete-dialog.js uses table B/B2 delete copy', () => {
    expect(deleteDialogJs).toContain('Deleting…');
    expect(deleteDialogJs).toContain('Confirm delete');
    expect(deleteDialogJs).toContain('Delete failed:');
    expect(deleteDialogJs).toContain("'Copy'");
    expect(deleteDialogJs).not.toContain('确认删除');
    expect(deleteDialogJs).not.toContain('删除中');
  });

  it('move-dialog.js uses table B/B2 move/delete copy', () => {
    expect(moveDialogJs).toContain('Confirm move');
    expect(moveDialogJs).toContain('Confirm delete');
    expect(moveDialogJs).toContain('Enter both URLs');
    expect(moveDialogJs).toContain('Moving…');
    expect(moveDialogJs).toContain('Deleting…');
    expect(moveDialogJs).toContain('Running gh api…');
    expect(moveDialogJs).not.toContain('确认移动');
    expect(moveDialogJs).not.toContain('请填写');
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
