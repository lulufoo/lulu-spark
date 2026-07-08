import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { READ_API_INVOKE_MAP } from '../frontend/js/readApiInvokeMap.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const WRITE_COMMANDS = [
  'create_plan_task',
  'delete_plan_task',
  'add_plan_sub',
  'delete_plan_sub',
];

describe('FM-4 AC gate (tech-doc VF / T-5)', () => {
  it('I-1: four write commands registered in lib.rs', () => {
    const lib = readFileSync(join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
    for (const cmd of WRITE_COMMANDS) {
      expect(lib).toMatch(new RegExp(`commands::plan_task::${cmd}`));
    }
  });

  it('I-1: write-api ACL whitelist covers four plan write commands', () => {
    const acl = readFileSync(join(repoRoot, 'src-tauri/permissions/write-api.toml'), 'utf8');
    for (const cmd of WRITE_COMMANDS) {
      expect(acl).toContain(`"${cmd}"`);
    }
  });

  it('I-1: ACL contract test lists four plan write commands', () => {
    const contract = readFileSync(
      join(repoRoot, 'src-tauri/src/unit-tests/config/write_api_acl_contract.rs'),
      'utf8',
    );
    for (const cmd of WRITE_COMMANDS) {
      expect(contract).toContain(`"${cmd}"`);
    }
  });

  it('I-4: plan-task write layer uses direct Tauri invoke (not fetch/local_http)', () => {
    const src = readFileSync(join(repoRoot, 'frontend/js/plan-task/index.js'), 'utf8');
    expect(src).toMatch(/invokePlanWrite\('create_plan_task'/);
    expect(src).toMatch(/invokePlanWrite\('delete_plan_task'/);
    expect(src).toMatch(/invokePlanWrite\('add_plan_sub'/);
    expect(src).toMatch(/invokePlanWrite\('delete_plan_sub'/);
    expect(src).not.toMatch(/\bfetch\s*\(/);
    expect(src).not.toMatch(/127\.0\.0\.1/);
  });

  it('GO: get_plan_tasks read path maps to Tauri command via readApiInvokeMap', () => {
    expect(READ_API_INVOKE_MAP['/api/plan-tasks']).toEqual({ cmd: 'get_plan_tasks' });
  });

  it('NG: plan-task UI has no complete_sub entry', () => {
    const src = readFileSync(join(repoRoot, 'frontend/js/plan-task/index.js'), 'utf8');
    expect(src).not.toMatch(/complete_sub/);
    expect(src).not.toMatch(/completeSub/);
  });
});
