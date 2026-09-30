import { readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readFrontendJs } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const TABLE_A_BRANDS = [
  { zh: 'Workbench 笔记', en: 'Notes', file: 'frontend/src/home/page.tsx' },
  { zh: 'Read Later 待读', en: 'Read Later', file: 'frontend/src/home/page.tsx' },
  { zh: '沉淀知识库', en: 'Knowledge', file: 'frontend/src/home/page.tsx' },
  { zh: 'Lulu Workbench', en: 'Lulu Workbench', file: 'frontend/src/shell.tsx' },
  { zh: '笔记助手', en: 'Notes Assistant', file: 'frontend/src/notes/ui/assistant.tsx' },
  { zh: 'AI 助手', en: 'Chats', file: 'frontend/src/home/page.tsx' },
  { zh: 'Read Later 助手', en: 'Read Later', file: 'frontend/src/read-later/ui/assistant.tsx' },
];

const KEY_PATH_FILES = [
  'frontend/src/shell.tsx',
  'frontend/src/home/page.tsx',
  'frontend/src/notes/viewer.ts',
  'frontend/src/knowledge/viewer.ts',
  'frontend/src/notes/ui/sidebar.tsx',
  'frontend/src/notes/ui/cards.tsx',
  'frontend/src/notes/commands/cards.ts',
  'frontend/src/shared/utils.ts',
  'frontend/src/notes/ui/assistant.tsx',
  'frontend/src/notes/commands/assistant.ts',
];

const SKILLS_EXCLUDED = [
  'frontend/src/app-shell/state/skills-content.ts',
];

function readSource(relPath) {
  const abs = join(repoRoot, relPath);
  expect(existsSync(abs), `missing ${relPath}`).toBe(true);
  if (relPath.endsWith('.js')) return readFrontendJs(relPath);
  return readFileSync(abs, 'utf8');
}

function stripComments(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\/\/.*$/gm, '');
}

describe('P4 copy-switch post-switch verification (tech-doc T8 / AC-2)', () => {
  it('table A brand entries appear in key-path surfaces (no Chinese remnants)', () => {
    for (const { zh, en, file } of TABLE_A_BRANDS) {
      const src = readSource(file);
      expect(src, `${file} missing ${en}`).toContain(en);
      if (zh !== en) {
        expect(src, `${file} still contains ${zh}`).not.toContain(zh);
      }
    }
  });

  it('key-path sources have no user-facing CJK (Skills files excluded)', () => {
    for (const rel of KEY_PATH_FILES) {
      const stripped = stripComments(readSource(rel));
      expect(stripped, `${rel} contains user-facing CJK`).not.toMatch(/[\u4e00-\u9fff]/);
    }
    for (const rel of SKILLS_EXCLUDED) {
      expect(existsSync(join(repoRoot, rel)), `expected excluded skills file ${rel}`).toBe(true);
    }
  });

  it('utils.js date and importance badges use English copy (table B/B2)', () => {
    const utils = readSource('frontend/src/shared/utils.ts');
    expect(utils).toContain("'Sun'");
    expect(utils).toContain('High');
    expect(utils).toContain('Cycle importance');
    expect(utils).toContain('Medium');
    expect(utils).not.toContain('切换重要性');
    expect(utils).not.toContain('↑ 高');
  });

  it('npm test includes copy-switch P4 post-verify test', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });
});
