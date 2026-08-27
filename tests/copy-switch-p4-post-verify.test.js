import { readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const FEATURE_DIR = 'docs/archive/ui/ui-english-copy-switch';
const P3_CHECKLIST_PATH = join(FEATURE_DIR, 'p3-release-gate-checklist.json');
const P4_VERIFY_PATH = join(FEATURE_DIR, 'p4-post-switch-verification.json');
const P4_SMOKE_PATH = join(FEATURE_DIR, 'p4-smoke-checklist.md');

const TABLE_A_BRANDS = [
  { zh: 'Workbench 笔记', en: 'Notes', file: 'frontend/js/components/home-hub.js' },
  { zh: 'Read Later 待读', en: 'Read Later', file: 'frontend/js/components/home-hub.js' },
  { zh: '沉淀知识库', en: 'Knowledge', file: 'frontend/js/components/home-hub.js' },
  { zh: 'Todos', en: 'Todos', file: 'frontend/js/components/home-hub.js' },
  { zh: 'LuLu Workbench', en: 'LuLu Workbench', file: 'frontend/index.html' },
  { zh: '笔记助手', en: 'Notes Assistant', file: 'frontend/js/note-assistant.js' },
  { zh: 'AI 助手', en: 'Chats', file: 'frontend/js/components/home-hub.js' },
  { zh: 'Read Later 助手', en: 'Read Later', file: 'frontend/js/read-later-assistant.js' },
];

const KEY_PATH_FILES = [
  'frontend/index.html',
  'frontend/js/components/home-hub.js',
  'frontend/js/todo-task/index.js',
  'frontend/js/todo-task/dialog.js',
  'frontend/js/components/viewer.js',
  'frontend/js/components/kb-viewer.js',
  'frontend/js/components/sidebar.js',
  'frontend/js/components/cards.js',
  'frontend/js/utils.js',
  'frontend/js/note-assistant.js',
];

const SKILLS_EXCLUDED = [
  'frontend/js/skills-workbench-content.js',
];

function loadJson(relPath) {
  const abs = join(repoRoot, relPath);
  expect(existsSync(abs), `missing ${relPath}`).toBe(true);
  return JSON.parse(readFileSync(abs, 'utf8'));
}

function readSource(relPath) {
  const abs = join(repoRoot, relPath);
  expect(existsSync(abs), `missing ${relPath}`).toBe(true);
  return readFileSync(abs, 'utf8');
}

function stripComments(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\/\/.*$/gm, '');
}

describe('P4 copy-switch post-switch verification (tech-doc T8 / AC-2)', () => {
  it('records P4 verification artifact with P3 gate cross-reference', () => {
    const verify = loadJson(P4_VERIFY_PATH);
    expect(verify.version).toBe(1);
    expect(verify.feature_id).toBe('feature-20260718130642-22a23648');
    expect(verify.task_id).toBe('t8');
    expect(verify.phase).toBe('P4');
    expect(verify.p3_gate_ref).toBe(P3_CHECKLIST_PATH);

    const p3 = loadJson(P3_CHECKLIST_PATH);
    expect(verify.pre_switch_gate.release_unlock.status).toBe('ready');
    expect(verify.pre_switch_gate.unlock_signals).toEqual(p3.unlock_signals);
  });

  it('documents spot-check alignment against tables A, B, and B2', () => {
    const verify = loadJson(P4_VERIFY_PATH);
    expect(verify.spot_check.table_a.status).toBe('aligned');
    expect(verify.spot_check.table_a.entry_count).toBe(8);
    expect(verify.spot_check.table_b.status).toBe('sampled');
    expect(verify.spot_check.table_b2.status).toBe('sampled');
    expect(verify.spot_check.table_b2.entry_count).toBe(95);
    expect(verify.spot_check.omission_review.status).toBe('closed');
  });

  it('records key-path smoke result for home hub → workbench → plan-tasks → corpus', () => {
    const verify = loadJson(P4_VERIFY_PATH);
    expect(verify.key_path_smoke.status).toBe('passed');
    expect(verify.key_path_smoke.route).toEqual([
      'home-hub',
      'workbench',
      'plan-tasks',
      'corpus',
    ]);
    expect(verify.key_path_smoke.skills_excluded).toBe(true);
    expect(verify.key_path_smoke.user_content_excluded).toBe(true);
  });

  it('p4 smoke checklist documents manual key-path steps', () => {
    const checklist = readSource(P4_SMOKE_PATH);
    expect(checklist).toContain('home hub');
    expect(checklist).toContain('workbench');
    expect(checklist).toContain('plan-tasks');
    expect(checklist).toContain('corpus');
    expect(checklist).toMatch(/Skills/i);
    expect(checklist).toMatch(/user.*content|user-created/i);
  });

  it('table A eight brand entries appear in key-path surfaces (no Chinese remnants)', () => {
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
    const utils = readSource('frontend/js/utils.js');
    expect(utils).toContain("'Sun'");
    expect(utils).toContain('High');
    expect(utils).toContain('Cycle importance');
    expect(utils).toContain('Medium');
    expect(utils).not.toContain('切换重要性');
    expect(utils).not.toContain('↑ 高');
  });

  it('npm test includes copy-switch P4 post-verify test', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toContain('tests/copy-switch-p4-post-verify.test.js');
  });
});
