import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { READ_API_INVOKE_MAP } from '../frontend/js/readApiInvokeMap.js';
import {
  ATTACHMENT_COMMANDS,
  MCP_ATTACHMENT_TOOLS,
  MCP_FORBIDDEN_DELETE_TOOLS,
  RUST_SERVICE_AC_TESTS,
  UI_ATTACHMENT_TEST_PROBES,
} from './fixtures/plan-task-ac15.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const WRITE_COMMANDS = [
  'create_plan_task',
  'delete_plan_task',
  'add_plan_sub',
  'delete_plan_sub',
];

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('FM-4 AC gate (tech-doc VF / T-5)', () => {
  it('I-1: four write commands registered in lib.rs', () => {
    const lib = read('src-tauri/src/lib.rs');
    for (const cmd of WRITE_COMMANDS) {
      expect(lib).toMatch(new RegExp(`commands::plan_task::${cmd}`));
    }
  });

  it('I-1: write-api ACL whitelist covers four plan write commands', () => {
    const acl = read('src-tauri/permissions/write-api.toml');
    for (const cmd of WRITE_COMMANDS) {
      expect(acl).toContain(`"${cmd}"`);
    }
  });

  it('I-1: ACL contract test lists four plan write commands', () => {
    const contract = read('src-tauri/src/unit-tests/config/write_api_acl_contract.rs');
    for (const cmd of WRITE_COMMANDS) {
      expect(contract).toContain(`"${cmd}"`);
    }
  });

  it('I-4: plan-task write layer uses direct Tauri invoke (not fetch/local_http)', () => {
    const src = read('frontend/js/plan-task/index.js');
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
    const src = read('frontend/js/plan-task/index.js');
    expect(src).not.toMatch(/complete_sub/);
    expect(src).not.toMatch(/completeSub/);
  });
});

describe('Plan-task attachment AC gate (tech-doc VF / T15)', () => {
  it('AC1/AC3/AC6/AC8: Rust service unit tests lock stem suffix, reject non-md, rollback, dual delete, cascade', () => {
    const rust = read('src-tauri/src/unit-tests/services/plan_task.rs');
    for (const marker of RUST_SERVICE_AC_TESTS) {
      expect(rust, marker).toContain(marker);
    }
  });

  it('AC2: attachment commands registered in lib.rs and write-api ACL', () => {
    const lib = read('src-tauri/src/lib.rs');
    const acl = read('src-tauri/permissions/write-api.toml');
    for (const cmd of ATTACHMENT_COMMANDS) {
      expect(lib).toMatch(new RegExp(`commands::plan_task::${cmd}`));
      expect(acl).toContain(`"${cmd}"`);
    }
  });

  it('AC2: MCP schema exposes add/list/get/update attachment tools', () => {
    const mcp = read('packages/knowledge-mcp/index.mjs');
    for (const tool of MCP_ATTACHMENT_TOOLS) {
      expect(mcp).toMatch(new RegExp(`['"]${tool}['"]`));
    }
  });

  it('AC4/AC5: UI add/list and editor save paths covered by integration tests', () => {
    for (const [rel, probes] of Object.entries(UI_ATTACHMENT_TEST_PROBES)) {
      const src = read(rel);
      for (const probe of probes) {
        expect(src, `${rel} :: ${probe}`).toMatch(probe);
      }
    }
  });

  it('AC6: MCP has no attachment delete tool; UI delete path is tested', () => {
    const mcp = read('packages/knowledge-mcp/index.mjs');
    for (const tool of MCP_FORBIDDEN_DELETE_TOOLS) {
      expect(mcp).not.toMatch(new RegExp(`['"]${tool}['"]`));
    }
    const deleteUi = read('tests/plan-task-attachment-delete.test.js');
    expect(deleteUi).toMatch(/delete_plan_attachment/);
    expect(deleteUi).toMatch(/MCP schema still has no attachment delete tool/);
  });

  it('AC2/AC4/AC5: UI index.js uses verb-first attachment Tauri commands', () => {
    const src = read('frontend/js/plan-task/index.js');
    expect(src).toMatch(/invokePlanPlain\('add_plan_attachment'/);
    expect(src).toMatch(/invokePlanPlain\('list_plan_attachments'/);
    expect(src).toMatch(/invokePlanPlain\('read_plan_attachment'/);
    expect(src).toMatch(/invokePlanPlain\('save_plan_attachment'/);
    expect(src).toMatch(/invokePlanPlain\('delete_plan_attachment'/);
    expect(src).not.toMatch(/\/api\/plan-task-add-attachment/);
  });
});

const PLAN_TASK_UI_SOURCES = [
  'frontend/js/plan-task/index.js',
  'frontend/js/plan-task/dialog.js',
];

const CJK = /[\u4e00-\u9fff]/;

describe('P2 copy-switch — Plan Tasks UI (tech-doc T3)', () => {
  it('index.js and dialog.js contain no user-visible Chinese', () => {
    for (const rel of PLAN_TASK_UI_SOURCES) {
      const src = read(rel);
      expect(src, rel).not.toMatch(CJK);
    }
  });

  it('status labels use table B English', () => {
    const index = read('frontend/js/plan-task/index.js');
    expect(index).toContain("incomplete: 'In progress'");
    expect(index).toContain("complete: 'Completed'");
    expect(index).toContain("abandoned: 'Abandoned'");
  });

  it('empty states and toolbar use table B ∪ B2 English', () => {
    const index = read('frontend/js/plan-task/index.js');
    expect(index).toContain('No todos yet');
    expect(index).toContain('Create your first todo to manage sub-tasks');
    expect(index).toContain('+ New todo');
    expect(index).toContain('Select a todo on the left');
    expect(index).toContain('Linked archives:');
    expect(index).toContain('aria-label="Todos list"');
    expect(index).toContain('aria-label="Task details"');
    expect(index).toContain('Just now');
    expect(index).toContain('minutes ago');
    expect(index).toContain('toLocaleDateString(\'en-US\')');
  });

  it('dialog.js uses table B ∪ B2 English for CRUD copy', () => {
    const dialog = read('frontend/js/plan-task/dialog.js');
    expect(dialog).toContain('New todo');
    expect(dialog).toContain('Create todo');
    expect(dialog).toContain('Please enter a todo name');
    expect(dialog).toContain('If you skip sub-tasks, the todo will have none');
    expect(dialog).toContain('Delete todo?');
    expect(dialog).toContain('Delete sub-task?');
    expect(dialog).toContain('Saving…');
    expect(dialog).toContain('Parent todo:');
    expect(dialog).toContain('Initial sub-tasks (optional)');
  });

  it('API routes and invoke commands remain unchanged', () => {
    const index = read('frontend/js/plan-task/index.js');
    expect(index).toContain("client.getJson('/api/plan-tasks')");
    expect(index).toContain('#/plan-tasks');
    expect(index).toContain('create_plan_task');
    expect(index).toContain('delete_plan_task');
    expect(index).toContain('add_plan_sub');
    expect(index).toContain('delete_plan_sub');
  });
});
