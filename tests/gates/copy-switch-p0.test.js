import { readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const GATE_PATH = 'docs/archive/ui/ui-english-copy-switch/p0-gate-confirmation.json';

function loadGate() {
  const abs = join(repoRoot, GATE_PATH);
  expect(existsSync(abs), `missing ${GATE_PATH}`).toBe(true);
  return JSON.parse(readFileSync(abs, 'utf8'));
}

describe('P0 copy-switch gate (tech-doc T0 / AC-1)', () => {
  it('records three-list overall confirmation with unlock signals', () => {
    const gate = loadGate();
    expect(gate.version).toBe(1);
    expect(gate.feature_id).toBe('feature-20260718130642-22a23648');

    expect(gate.omission_review.status).toBe('closed');
    expect(gate.omission_review.omission_count).toBe(95);
    expect(gate.omission_review.artifact).toBe('yi-lou-shen-cha.md');

    expect(gate.table_a.status).toBe('confirmed_line_by_line');
    expect(gate.table_a.entry_count).toBe(8);
    expect(gate.table_a.artifact).toBe('xu-tao-lun-que-ren-fan-yi-qing-dan.md');

    expect(gate.table_b.status).toBe('confirmed_whole');
    expect(gate.table_b.artifact).toBe('pu-tong-ming-ming-qing-dan.md');

    expect(gate.table_b2.status).toBe('confirmed_whole');
    expect(gate.table_b2.entry_count).toBe(95);
    expect(gate.table_b2.artifact).toBe('pu-tong-ming-ming-qing-dan-b2.md');
    expect(gate.table_b2.baseline).toBe('yi-lou-shen-cha.md');

    expect(gate.three_list_overall.status).toBe('confirmed');
    expect(gate.unlock_signals).toEqual({
      table_a_line_confirmed: true,
      table_b_union_b2_whole_confirmed: true,
      omission_review_passed: true,
    });
  });

  it('npm test includes copy-switch P0 gate test', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });
});
