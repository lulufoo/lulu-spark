/**
 * T5 / SK-4: Independent reclassification ops evidence gate.
 * After default tagging (t2), at least one sample todo must be moved from
 * 「待分类」(uncategorized) to a target category via Host set_master_category
 * (MCP update_todo_task category_id / UI setPlanCategory surface) — no MCP
 * category CRUD.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const EVIDENCE_PATH = join(
  repoRoot,
  'scripts/fixtures/todo-reclassify-ops-evidence.json',
);
const DEFAULT_CATEGORY_ID = 'uncategorized';
const DEFAULT_CATEGORY_NAME = '待分类';

describe('todo reclassify ops (t5)', () => {
  it('records at least one sample moved from 待分类 to a target category', () => {
    expect(
      existsSync(EVIDENCE_PATH),
      `missing ops evidence: ${EVIDENCE_PATH}`,
    ).toBe(true);

    const evidence = JSON.parse(readFileSync(EVIDENCE_PATH, 'utf8'));
    expect(evidence.task_id).toBe('t5');
    expect(evidence.default_category).toEqual({
      id: DEFAULT_CATEGORY_ID,
      name: DEFAULT_CATEGORY_NAME,
    });
    expect(Array.isArray(evidence.samples)).toBe(true);
    expect(evidence.samples.length).toBeGreaterThanOrEqual(1);

    const sample = evidence.samples[0];
    expect(typeof sample.master_task_id).toBe('string');
    expect(sample.master_task_id.length).toBeGreaterThan(0);
    expect(sample.from_category_id).toBe(DEFAULT_CATEGORY_ID);
    expect(typeof sample.to_category_id).toBe('string');
    expect(sample.to_category_id.length).toBeGreaterThan(0);
    expect(sample.to_category_id).not.toBe(DEFAULT_CATEGORY_ID);
    expect(typeof sample.to_category_name).toBe('string');
    expect(sample.to_category_name.length).toBeGreaterThan(0);
    // Contract: reassign via update surface, not MCP category CRUD
    expect(['update_todo_task', 'set_master_category', 'setPlanCategory']).toContain(
      sample.via,
    );
    expect(sample.status).toBe('moved');
  });

  it('documents reversible reassign and invalid category_id reject', () => {
    expect(existsSync(EVIDENCE_PATH)).toBe(true);
    const evidence = JSON.parse(readFileSync(EVIDENCE_PATH, 'utf8'));
    expect(evidence.reversible).toBe(true);
    expect(evidence.invalid_category_rejected).toBe(true);
    expect(typeof evidence.harness).toBe('string');
    expect(evidence.harness.length).toBeGreaterThan(0);
  });

  it('does not introduce MCP category directory CRUD tools', () => {
    const src = readFileSync(join(repoRoot, 'packages/knowledge-mcp/index.mjs'), 'utf8');
    for (const tool of [
      'create_todo_category',
      'delete_todo_category',
      'create_todo_task_category',
      'delete_todo_task_category',
    ]) {
      expect(src.includes(`'${tool}'`) || src.includes(`"${tool}"`)).toBe(false);
    }
  });
});
