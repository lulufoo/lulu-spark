import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { READ_API_INVOKE_MAP } from '../../frontend/src/host/readApiInvokeMap.ts';
import {
  ATTACHMENT_COMMANDS,
  MCP_ATTACHMENT_TOOLS,
  MCP_FORBIDDEN_DELETE_TOOLS,
  RUST_SERVICE_AC_TESTS,
  UI_ATTACHMENT_TEST_PROBES,
} from '../fixtures/todo-task-ac15.js';
import {
  readTodoTaskServiceTestsSource,
  readTodoTaskUiSource,
  TODO_TASK_UI_FILES,
} from '../helpers/todo-task-ui-source.js';
import { readRsPath } from '../helpers/read-rs-dir.js';
import { readFrontendJs } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const WRITE_COMMANDS = [
  'create_todo_task',
  'delete_todo_task',
  'add_todo_sub',
  'delete_todo_sub',
];

function read(rel) {
  const abs = join(repoRoot, rel);
  if (rel.includes('src-tauri/')) return readRsPath(abs);
  if (rel.startsWith('frontend/src/') && rel.endsWith('.js')) return readFrontendJs(rel);
  return readFileSync(abs, 'utf8');
}

describe('FM-4 AC gate (tech-doc VF / T-5)', () => {
  it('I-1: four write commands registered in lib.rs', () => {
    const lib = read('src-tauri/src/lib.rs');
    for (const cmd of WRITE_COMMANDS) {
      expect(lib).toMatch(new RegExp(`commands::todo_task::${cmd}`));
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

  it('I-4: todo-task write layer uses direct Tauri invoke (not fetch/local_http)', () => {
    const src = read('frontend/src/todo-task/state/host.ts');
    expect(src).toMatch(/invokePlanWrite\('create_todo_task'/);
    expect(src).toMatch(/invokePlanWrite\('delete_todo_task'/);
    expect(src).toMatch(/invokePlanWrite\('add_todo_sub'/);
    expect(src).toMatch(/invokePlanWrite\('delete_todo_sub'/);
    expect(src).not.toMatch(/\bfetch\s*\(/);
    expect(src).not.toMatch(/127\.0\.0\.1/);
  });

  it('GO: get_todo_tasks read path maps to Tauri command via readApiInvokeMap', () => {
    expect(READ_API_INVOKE_MAP['/api/todo-tasks']).toEqual({ cmd: 'get_todo_tasks' });
  });

  it('NG: todo-task UI has no complete_sub entry', () => {
    const src = readTodoTaskUiSource();
    expect(src).not.toMatch(/complete_sub/);
    expect(src).not.toMatch(/completeSub/);
  });
});

describe('Plan-task attachment AC gate (tech-doc VF / T15)', () => {
  it('AC1/AC3/AC6/AC8: Rust service unit tests lock stem suffix, reject non-md, rollback, dual delete, cascade', () => {
    const rust = readTodoTaskServiceTestsSource();
    for (const marker of RUST_SERVICE_AC_TESTS) {
      expect(rust, marker).toContain(marker);
    }
  });

  it('AC2: attachment commands registered in lib.rs and write-api ACL', () => {
    const lib = read('src-tauri/src/lib.rs');
    const acl = read('src-tauri/permissions/write-api.toml');
    for (const cmd of ATTACHMENT_COMMANDS) {
      expect(lib).toMatch(new RegExp(`commands::todo_task::${cmd}`));
      expect(acl).toContain(`"${cmd}"`);
    }
  });

  it('AC2: MCP schema exposes add/list/get/update attachment tools', () => {
    const mcp = read('src-tauri/src/services/mcp_protocol_adapter');
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
    const mcp = read('src-tauri/src/services/mcp_protocol_adapter');
    for (const tool of MCP_FORBIDDEN_DELETE_TOOLS) {
      expect(mcp).not.toMatch(new RegExp(`['"]${tool}['"]`));
    }
    const deleteUi = read('tests/todo-task/attachment-delete.test.js');
    expect(deleteUi).toMatch(/delete_todo_attachment/);
    expect(deleteUi).toMatch(/MCP schema still has no attachment delete tool/);
  });

  it('AC2/AC4/AC5: UI host.js uses verb-first attachment Tauri commands', () => {
    const src = read('frontend/src/todo-task/state/host.ts');
    expect(src).toMatch(/invokePlanPlain\('add_todo_attachment'/);
    expect(src).toMatch(/invokePlanPlain\('list_todo_attachments'/);
    expect(src).toMatch(/invokePlanPlain\('read_todo_attachment'/);
    expect(src).toMatch(/invokePlanPlain\('save_todo_attachment'/);
    expect(src).toMatch(/invokePlanPlain\('delete_todo_attachment'/);
    expect(src).not.toMatch(/\/api\/todo-task-add-attachment/);
  });
});

const TODO_TASK_UI_SOURCES = [
  'frontend/src/todo-task/index.ts',
  'frontend/src/todo-task/page.tsx',
  'frontend/src/todo-task/ui/page-render.tsx',
  'frontend/src/todo-task/commands/page-dialogs.ts',
  'frontend/src/todo-task/commands/page-events.ts',
  'frontend/src/todo-task/state/host.ts',
  'frontend/src/todo-task/state/format.ts',
  'frontend/src/todo-task/ui/list.tsx',
  'frontend/src/todo-task/commands/list.ts',
  'frontend/src/todo-task/commands/detail.ts',
  'frontend/src/todo-task/ui/detail.tsx',
  'frontend/src/todo-task/ui/plan-md.tsx',
  'frontend/src/todo-task/commands/plan-md.ts',
  'frontend/src/todo-task/commands/attachments.ts',
  'frontend/src/todo-task/ui/attachments.tsx',
  'frontend/src/todo-task/ui/comments.tsx',
  'frontend/src/todo-task/commands/comments.ts',
  'frontend/src/todo-task/ui/dialog.tsx',
  'frontend/src/todo-task/commands/dialog.ts',
  'frontend/src/todo-task/state/dialog.ts',
];

const CJK = /[\u4e00-\u9fff]/;

describe('P2 copy-switch — Plan Tasks UI (tech-doc T3)', () => {
  it('index.js and dialog.js contain no user-visible Chinese', () => {
    for (const rel of TODO_TASK_UI_SOURCES) {
      const src = read(rel);
      expect(src, rel).not.toMatch(CJK);
    }
  });

  it('status labels use table B English', () => {
    const index = readTodoTaskUiSource();
    expect(index).toContain("incomplete: 'In progress'");
    expect(index).toContain("complete: 'Completed'");
    expect(index).toContain("abandoned: 'Abandoned'");
  });

  it('empty states and toolbar use table B ∪ B2 English', () => {
    const index = readTodoTaskUiSource();
    expect(index).toContain('No todos yet');
    expect(index).toContain('Create your first todo to manage sub-tasks');
    expect(index).toContain('+ New todo');
    expect(index).toContain('Select a todo on the left');
    expect(index).toContain('Linked archives:');
    expect(index).toContain('aria-label="Todos list"');
    expect(index).toContain('aria-label="Task details"');
    expect(index).toContain('Just now');
    expect(index).toContain('minutes ago');
    expect(index).toContain("toLocaleDateString('en-US')");
    expect(index).toContain('Active only');
    expect(index).toContain('No active todos');
    expect(index).toContain('Turn off Active only to see completed and abandoned todos.');
  });

  it('dialog.js uses table B ∪ B2 English for CRUD copy', () => {
    const dialog = [
      read('frontend/src/todo-task/ui/dialog.tsx'),
      read('frontend/src/todo-task/commands/dialog.ts'),
      read('frontend/src/todo-task/state/dialog.ts'),
    ].join('\n');
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
    const index = readTodoTaskUiSource();
    expect(index).toContain("client.getJson('/api/todo-tasks')");
    expect(index).toContain('#/todo-tasks');
    expect(index).toContain('create_todo_task');
    expect(index).toContain('delete_todo_task');
    expect(index).toContain('add_todo_sub');
    expect(index).toContain('delete_todo_sub');
  });
});

const T9_FRONTEND_TARGETS = [
  ...TODO_TASK_UI_FILES,
  'frontend/src/todo-task/ui/assistant.tsx',
  'frontend/src/todo-task/commands/assistant.ts',
];

describe('T9 — frontend/assistant Host API follow (tech-doc T9)', () => {
  it('index and assistant call GET /api/todo-tasks (not /api/plan-tasks)', () => {
    const index = readTodoTaskUiSource();
    const assistant = [
      read('frontend/src/todo-task/ui/assistant.tsx'),
      read('frontend/src/todo-task/commands/assistant.ts'),
    ].join('\n');
    expect(index).toContain("client.getJson('/api/todo-tasks')");
    expect(assistant).toContain("client.getJson('/api/todo-tasks')");
    expect(index).not.toContain("client.getJson('/api/plan-tasks')");
    expect(assistant).not.toContain("client.getJson('/api/plan-tasks')");
  });

  it('target sources retain no /api/plan-* HTTP paths (hard cut, no aliases)', () => {
    for (const rel of T9_FRONTEND_TARGETS) {
      const src = read(rel);
      expect(src, rel).not.toMatch(/\/api\/plan-/);
    }
  });

  it('index consumes todo_md wire field (not plan_md)', () => {
    const index = readTodoTaskUiSource();
    expect(index).toMatch(/master\.todo_md/);
    expect(index).not.toMatch(/master\.plan_md/);
  });

  it('readApiInvokeMap maps /api/todo-tasks only (no /api/plan-tasks dual alias)', () => {
    expect(READ_API_INVOKE_MAP['/api/todo-tasks']).toEqual({ cmd: 'get_todo_tasks' });
    expect(READ_API_INVOKE_MAP['/api/plan-tasks']).toBeUndefined();
  });

  it('todo-task invoke surface uses Host todo_* command names', () => {
    const index = readTodoTaskUiSource();
    for (const cmd of [
      'create_todo_task',
      'delete_todo_task',
      'add_todo_sub',
      'delete_todo_sub',
      'read_todo_md',
      'update_todo_md',
      'complete_todo',
      'add_todo_attachment',
      'list_todo_attachments',
    ]) {
      expect(index, cmd).toContain(cmd);
    }
    expect(index).not.toMatch(/'(create|delete|add|read|update|complete)_plan/);
  });
});

describe('T10 — end-to-end acceptance pointer (tech-doc SK-5)', () => {
  it('delegates contract/migration/equivalence gates to todo-task-ac-gate.test.js', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(readFileSync(join(repoRoot, 'tests/todo-task/ac-gate.test.js'), 'utf8')).toMatch(
      /EQUIVALENCE_TODO_TOOLS/,
    );
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });
});
