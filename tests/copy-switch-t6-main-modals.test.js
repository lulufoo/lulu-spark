import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

const mainJs = readFileSync(join(repoRoot, 'frontend/js/main.js'), 'utf8');
const commitDialogJs = readFileSync(
  join(repoRoot, 'frontend/js/components/modals/commit-dialog.js'),
  'utf8',
);
const deleteDialogJs = readFileSync(
  join(repoRoot, 'frontend/js/components/modals/delete-dialog.js'),
  'utf8',
);
const moveDialogJs = readFileSync(
  join(repoRoot, 'frontend/js/components/modals/move-dialog.js'),
  'utf8',
);
const settleDialogJs = readFileSync(
  join(repoRoot, 'frontend/js/components/settle-dialog.js'),
  'utf8',
);
const modalSources = [
  commitDialogJs,
  deleteDialogJs,
  moveDialogJs,
  settleDialogJs,
  readFileSync(join(repoRoot, 'frontend/js/components/modals/kb-diff-dialog.js'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/js/components/modals/base64-dialog.js'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/js/components/modals/move-project-dialog.js'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/js/components/modals/qr-dialog.js'), 'utf8'),
  readFileSync(join(repoRoot, 'frontend/js/components/modals/convert-dialog.js'), 'utf8'),
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
    expect(mainJs).toContain('✓ Copied');
    expect(mainJs).not.toContain('重试');
    expect(mainJs).not.toContain('沉淀知识库列表');
    expect(mainJs).not.toContain('加载中');
  });

  it('commit-dialog.js aligns with kb-viewer commit copy', () => {
    expect(commitDialogJs).toContain("↑ Commit changes");
    expect(commitDialogJs).toContain('Checking…');
    expect(commitDialogJs).toContain('Loading…');
    expect(commitDialogJs).toContain('No changes to commit or push');
    expect(commitDialogJs).toContain("label: 'New'");
    expect(commitDialogJs).toContain('Ready to push');
    expect(commitDialogJs).toContain('local commit(s) not yet pushed');
    expect(commitDialogJs).toContain('✓ Committed and pushed');
    expect(commitDialogJs).not.toContain('提交');
    expect(commitDialogJs).not.toContain('加载中');
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

  it('main.js skills chrome is English while inline lulu skills names may stay Chinese', () => {
    expect(mainJs).toContain('Click to copy command');
    expect(mainJs).not.toContain('点击复制');
    expect(mainJs).not.toContain('已复制');
  });

  it('t6 sources have no user-facing Chinese outside skills catalog literals', () => {
    const skillsBlock = mainJs.match(
      /const _SKILLS_CONTENT = \{[\s\S]*?\n\};/,
    )?.[0] ?? '';
    const mainWithoutSkills = mainJs.replace(skillsBlock, '');
    const body = withoutComments([mainWithoutSkills, modalSources].join('\n'));
    expect(body).not.toMatch(/[\u4e00-\u9fff]/);
  });
});
