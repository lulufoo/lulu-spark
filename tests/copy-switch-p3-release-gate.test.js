import { readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const FEATURE_DIR = 'docs/features/ui-english-copy-switch';
const P0_GATE_PATH = join(FEATURE_DIR, 'p0-gate-confirmation.json');
const P3_CHECKLIST_PATH = join(FEATURE_DIR, 'p3-release-gate-checklist.json');

function loadJson(relPath) {
  const abs = join(repoRoot, relPath);
  expect(existsSync(abs), `missing ${relPath}`).toBe(true);
  return JSON.parse(readFileSync(abs, 'utf8'));
}

describe('P3 copy-switch release hard-gate (tech-doc T7 / AC-3)', () => {
  it('records pre-merge verification of three unlock signals', () => {
    const checklist = loadJson(P3_CHECKLIST_PATH);
    expect(checklist.version).toBe(1);
    expect(checklist.feature_id).toBe('feature-20260718130642-22a23648');
    expect(checklist.task_id).toBe('t7');
    expect(checklist.phase).toBe('P3');

    expect(checklist.p0_gate_ref).toBe(P0_GATE_PATH);

    expect(checklist.implementation_tasks.status).toBe('completed');
    expect(checklist.implementation_tasks.task_ids).toEqual([
      't1',
      't2',
      't3',
      't4',
      't5',
      't6',
    ]);

    expect(checklist.pre_merge_verification.table_a.status).toBe('confirmed_line_by_line');
    expect(checklist.pre_merge_verification.table_a.entry_count).toBe(8);

    expect(checklist.pre_merge_verification.table_b_union_b2.status).toBe('confirmed_whole');
    expect(checklist.pre_merge_verification.table_b_union_b2.tables).toEqual(['table_b', 'table_b2']);
    expect(checklist.pre_merge_verification.table_b_union_b2.table_b2_entry_count).toBe(95);

    expect(checklist.pre_merge_verification.omission_review.status).toBe('closed');
    expect(checklist.pre_merge_verification.omission_review.omission_count).toBe(95);

    expect(checklist.unlock_signals).toEqual({
      table_a_line_confirmed: true,
      table_b_union_b2_whole_confirmed: true,
      omission_review_passed: true,
    });
  });

  it('blocks user-facing merge when any unlock signal is missing', () => {
    const checklist = loadJson(P3_CHECKLIST_PATH);
    expect(checklist.merge_policy.no_runtime_feature_flag).toBe(true);
    expect(checklist.merge_policy.reject_if_any_signal_missing).toBe(true);
    expect(checklist.merge_policy.user_facing_merge_blocked_until_unlock).toBe(true);

    const signals = checklist.unlock_signals;
    const allPresent =
      signals.table_a_line_confirmed &&
      signals.table_b_union_b2_whole_confirmed &&
      signals.omission_review_passed;
    expect(checklist.release_unlock.all_signals_present).toBe(allPresent);
    expect(checklist.release_unlock.status).toBe(allPresent ? 'ready' : 'blocked');
  });

  it('cross-checks P3 unlock signals against P0 gate record', () => {
    const p0 = loadJson(P0_GATE_PATH);
    const p3 = loadJson(P3_CHECKLIST_PATH);

    expect(p3.unlock_signals).toEqual(p0.unlock_signals);
    expect(p3.pre_merge_verification.table_a.artifact_ref).toBe(p0.table_a.artifact_ref);
    expect(p3.pre_merge_verification.table_b_union_b2.table_b_artifact_ref).toBe(
      p0.table_b.artifact_ref,
    );
    expect(p3.pre_merge_verification.table_b_union_b2.table_b2_artifact_ref).toBe(
      p0.table_b2.artifact_ref,
    );
    expect(p3.pre_merge_verification.omission_review.artifact_ref).toBe(
      p0.omission_review.artifact_ref,
    );
  });

  it('npm test includes copy-switch P3 release gate test', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toContain('tests/copy-switch-p3-release-gate.test.js');
  });
});
