// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFrontendJs, readShellHtml } from '../helpers/read-frontend-js.js';

const getJsonMock = vi.fn();

/** Todos paint also GET /api/doc-highlights; do not let that consume list mocks. */
function mockTodoListReads(lists) {
  let n = 0;
  getJsonMock.mockImplementation(async (path) => {
    if (String(path).startsWith('/api/doc-highlights')) {
      return { highlights: [] };
    }
    n += 1;
    return lists[Math.min(n - 1, lists.length - 1)];
  });
}

vi.mock('../../frontend/src/host/apiClient.ts', async (importOriginal) => {
  const actual = await importOriginal();
  return {
    ...actual,
    resolveReadDriver: vi.fn(() => ({ getJson: getJsonMock })),
    createApiClient: (driver) => ({
      getJson: driver?.getJson ?? getJsonMock,
    }),
  };
});

import { parseHash } from '../../frontend/src/router/index.ts';
import {
  copySubIdPair,
  formatMasterCopyText,
  formatTodoTaskStatus,
  loadTodoTasks,
  mountTodoTaskSplit,
  renderSubDetail,
  setPlanMasterStatus,
} from '../../frontend/src/todo-task/index.ts';

import { readRsPath } from '../helpers/read-rs-dir.js';
import { readMainSource } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const mainJs = readMainSource();
const indexHtml = readShellHtml();
const todoTaskDialogSrc = [
  readFrontendJs('frontend/src/todo-task/ui/dialog.tsx'),
  readFrontendJs('frontend/src/todo-task/commands/dialog.ts'),
  readFrontendJs('frontend/src/todo-task/state/dialog.ts'),
].join('\n');
const appCss = readFileSync(join(fixtureRoot, 'frontend/app.css'), 'utf8');

function extractFunctionBody(source, name) {
  const start = source.indexOf(`function ${name}`);
  if (start === -1) return '';
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let i = braceStart; i < source.length; i += 1) {
    if (source[i] === '{') depth += 1;
    if (source[i] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(braceStart, i + 1);
    }
  }
  return '';
}

const sampleMasters = [
  {
    master_task_id: 'task_alpha',
    title: 'Alpha Task',
    status: 'incomplete',
    created_at: '2026-07-01T10:00:00Z',
    sub_tasks: [
      {
        sub_task_id: 'task_alpha_sub_01',
        title: 'Alpha Sub A',
        status: 'incomplete',
        implicit: false,
        linked_archive_ids: ['arch_001', 'arch_002'],
      },
      {
        sub_task_id: 'task_alpha_sub_02',
        title: 'Alpha Sub B',
        status: 'complete',
        implicit: false,
        linked_archive_ids: [],
      },
      {
        sub_task_id: 'task_alpha_sub_03',
        title: 'Alpha Sub C',
        status: 'abandoned',
        implicit: false,
        linked_archive_ids: [],
      },
    ],
  },
  {
    master_task_id: 'task_empty',
    title: 'Empty Subs Task',
    status: 'incomplete',
    created_at: '2026-07-05T10:00:00Z',
    sub_tasks: [],
    todo_md: '',
    migration_error: false,
  },
  {
    master_task_id: 'task_migrated_err',
    title: 'Migration Error Task',
    status: 'incomplete',
    created_at: '2026-07-04T10:00:00Z',
    sub_tasks: [
      {
        sub_task_id: 'task_migrated_err_sub_01',
        title: 'Still visible sub',
        status: 'incomplete',
        implicit: false,
        linked_archive_ids: [],
      },
    ],
    todo_md: '## Notes',
    migration_error: true,
  },
  {
    master_task_id: 'task_complete',
    title: 'Completed Master',
    status: 'complete',
    created_at: '2026-07-03T10:00:00Z',
    sub_tasks: [],
    todo_md: '',
    migration_error: false,
  },
  {
    master_task_id: 'task_abandoned',
    title: 'Abandoned Master',
    status: 'abandoned',
    created_at: '2026-07-02T10:00:00Z',
    sub_tasks: [],
    todo_md: '',
    migration_error: false,
  },
];

describe('parseHash todo-tasks deep-link', () => {
  it('parses #/todo-tasks without query params', () => {
    expect(parseHash('#/todo-tasks')).toEqual({
      name: 'todo-tasks',
      params: {},
    });
  });

  it('parses #/todo-tasks?master=&sub= query params', () => {
    expect(parseHash('#/todo-tasks?master=task_beta&sub=task_beta_sub_01')).toEqual({
      name: 'todo-tasks',
      params: { master: 'task_beta', sub: 'task_beta_sub_01' },
    });
  });

  it('parses trailing slash on todo-tasks route', () => {
    expect(parseHash('#/todo-tasks/')).toEqual({
      name: 'todo-tasks',
      params: {},
    });
  });
});

describe('copySubIdPair', () => {
  it('formats per-sub copy string as master → sub, with todo_ prefix on both ids', () => {
    expect(copySubIdPair('task_alpha', 'task_alpha_sub_01')).toBe(
      'todo_task_alpha → todo_task_alpha_sub_01',
    );
  });

  it('prefixes stored task_* ids for clipboard paste-back', () => {
    expect(copySubIdPair('task_9b4bca4a89ae', 'task_9b4bca4a89ae_sub_01')).toBe(
      'todo_task_9b4bca4a89ae → todo_task_9b4bca4a89ae_sub_01',
    );
  });

  it('does not double-prefix ids that already have todo_', () => {
    expect(
      copySubIdPair('todo_task_9b4bca4a89ae', 'todo_task_9b4bca4a89ae_sub_01'),
    ).toBe('todo_task_9b4bca4a89ae → todo_task_9b4bca4a89ae_sub_01');
  });
});

describe('formatMasterCopyText', () => {
  it('formats title and ID on separate labeled lines, with todo_ prefix on ID', () => {
    expect(formatMasterCopyText('Alpha Task', 'task_alpha')).toBe(
      'Title: Alpha Task\nID: todo_task_alpha',
    );
  });

  it('prefixes stored task_* ids for clipboard paste-back', () => {
    expect(
      formatMasterCopyText(
        'Workbench Home 聊天进度：请求维 Channel 与并行 flights（方案）',
        'task_21cf9d096242',
      ),
    ).toBe(
      'Title: Workbench Home 聊天进度：请求维 Channel 与并行 flights（方案）\nID: todo_task_21cf9d096242',
    );
  });

  it('does not double-prefix an ID that already has todo_', () => {
    expect(formatMasterCopyText('Alpha Task', 'todo_task_alpha')).toBe(
      'Title: Alpha Task\nID: todo_task_alpha',
    );
  });

  it('falls back to (untitled) when title is blank', () => {
    expect(formatMasterCopyText('  ', 'task_alpha')).toBe(
      'Title: (untitled)\nID: todo_task_alpha',
    );
  });
});

describe('formatTodoTaskStatus', () => {
  it('maps incomplete, complete, and abandoned to English labels', () => {
    expect(formatTodoTaskStatus('incomplete')).toBe('In progress');
    expect(formatTodoTaskStatus('complete')).toBe('Completed');
    expect(formatTodoTaskStatus('abandoned')).toBe('Abandoned');
  });
});

describe('renderSubDetail', () => {
  it('renders all subs with English status labels and copy in menu', () => {
    const html = renderSubDetail(sampleMasters[0], 'task_alpha_sub_01');
    expect(html).toContain('Alpha Sub A');
    expect(html).toContain('Alpha Sub B');
    expect(html).toContain('Alpha Sub C');
    expect(html).toContain('In progress');
    expect(html).toContain('Completed');
    expect(html).toContain('Abandoned');
    expect(html).toContain('data-copy-text="todo_task_alpha → todo_task_alpha_sub_01"');
    expect(html).toContain('data-copy-text="todo_task_alpha → todo_task_alpha_sub_02"');
  });

  it('renders distinct status modifier classes for three sub states', () => {
    const html = renderSubDetail(sampleMasters[0], 'task_alpha_sub_01');
    expect(html).toContain('todo-task-sub-status-select--incomplete');
    expect(html).toContain('todo-task-sub-status-select--complete');
    expect(html).toContain('todo-task-sub-status-select--abandoned');
  });

  it('renders linked_archive_ids as comma list with prefix', () => {
    const html = renderSubDetail(sampleMasters[0], 'task_alpha_sub_01');
    expect(html).toContain('Linked archives:');
    expect(html).toContain('arch_001, arch_002');
  });

  it('marks selected sub with selected class', () => {
    const html = renderSubDetail(sampleMasters[0], 'task_alpha_sub_02');
    expect(html).toMatch(/data-sub-id="task_alpha_sub_02"[^>]*todo-task-sub--selected/);
  });
});

describe('loadTodoTasks', () => {
  beforeEach(() => {
    getJsonMock.mockReset();
  });

  it('GET /api/todo-tasks via apiClient and returns master array', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const entries = await loadTodoTasks();
    expect(getJsonMock).toHaveBeenCalledWith('/api/todo-tasks');
    expect(entries).toEqual(sampleMasters);
  });

  it('throws with status when service returns unavailable payload', async () => {
    getJsonMock.mockResolvedValue({
      error: 'Workbench not running',
      _status: 503,
    });
    await expect(loadTodoTasks()).rejects.toMatchObject({
      status: 503,
    });
  });
});

describe('mountTodoTaskSplit', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    container.remove();
  });

  it('renders left master list and right detail panes', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-split-master')).not.toBeNull();
      expect(container.querySelector('.todo-task-split-detail')).not.toBeNull();
    });
    dispose();
  });

  it('hides complete and abandoned masters by default (Active only on)', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('[data-master-id="task_alpha"]')).not.toBeNull();
    });
    expect(container.querySelector('[data-master-id="task_complete"]')).toBeNull();
    expect(container.querySelector('[data-master-id="task_abandoned"]')).toBeNull();
    expect(container.querySelectorAll('.todo-task-master-item')).toHaveLength(3);
    expect(container.querySelector('[data-action="toggle-active-only"]')).not.toBeNull();
    expect(container.textContent).toContain('Active only');
    expect(container.textContent).toContain('+ New todo');
    const toggle = container.querySelector('[data-action="toggle-active-only"]');
    expect(toggle?.getAttribute('aria-checked')).toBe('true');
    dispose();
  });

  it('shows complete and abandoned masters when Active only is turned off', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('[data-action="toggle-active-only"]')).not.toBeNull();
    });
    container.querySelector('[data-action="toggle-active-only"]').click();
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="toggle-active-only"]')?.getAttribute('aria-checked'),
      ).toBe('false');
      expect(container.querySelector('[data-master-id="task_complete"]')).not.toBeNull();
      expect(container.querySelector('[data-master-id="task_abandoned"]')).not.toBeNull();
      expect(container.querySelectorAll('.todo-task-master-item')).toHaveLength(5);
    });
    dispose();
  });

  it('shows No active todos when every master is complete or abandoned', async () => {
    getJsonMock.mockResolvedValue([
      { ...sampleMasters.find((m) => m.master_task_id === 'task_complete') },
      { ...sampleMasters.find((m) => m.master_task_id === 'task_abandoned') },
    ]);
    const { dispose } = mountTodoTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.textContent).toContain('No active todos');
      expect(container.textContent).toContain(
        'Turn off Active only to see completed and abandoned todos.',
      );
      expect(container.textContent).not.toContain('No todos yet');
      expect(
        container.querySelector('.todo-task-empty--sidebar [data-action="create-master"]'),
      ).toBeNull();
    });
    dispose();
  });

  it('clears selection for completed deep-link while Active only is on', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_complete',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-master-id="task_complete"]')).toBeNull();
      expect(container.querySelector('.todo-task-split-detail-empty')).not.toBeNull();
      expect(container.querySelector('.todo-task-split-dead-link')).toBeNull();
      expect(container.querySelector('[data-action="edit-master-title"]')).toBeNull();
    });
    dispose();
  });

  it('removes completed master from Active only list and clears detail', async () => {
    const incompleteAlpha = sampleMasters.find((m) => m.master_task_id === 'task_alpha');
    const completedAlpha = { ...incompleteAlpha, status: 'complete' };
    const afterComplete = sampleMasters.map((m) =>
      m.master_task_id === 'task_alpha' ? completedAlpha : m,
    );
    mockTodoListReads([sampleMasters, afterComplete]);

    const invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_todo_master_status') {
        return { task: { ...incompleteAlpha, status: args.status }, _status: 200 };
      }
      if (cmd === 'list_todo_attachments') return [];
      if (cmd === 'list_todo_comments') return [];
      return {};
    });
    window.__TAURI__ = { core: { invoke: invokeMock } };

    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-action="change-master-status"]')).not.toBeNull();
    });
    const statusSelect = container.querySelector('[data-action="change-master-status"]');
    statusSelect.value = 'complete';
    statusSelect.dispatchEvent(new Event('change', { bubbles: true }));

    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('set_todo_master_status', {
        masterTaskId: 'task_alpha',
        status: 'complete',
      });
      expect(container.querySelector('[data-master-id="task_alpha"]')).toBeNull();
      expect(container.querySelector('.todo-task-split-detail-empty')).not.toBeNull();
    });

    dispose();
    delete window.__TAURI__;
  });

  it('selects the first active master when opened without a deep-link', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container);
    await vi.waitFor(() => {
      expect(
        container.querySelector('.todo-task-master-item--selected')?.dataset.masterId,
      ).toBe('task_empty');
      expect(container.querySelector('.todo-task-split-detail-empty')).toBeNull();
    });
    dispose();
  });

  it('selects master and sub from initial masterId/subId options', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_02',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('.todo-task-master-item--selected')?.dataset.masterId,
      ).toBe('task_alpha');
      expect(container.querySelector('.todo-task-sub--selected')?.dataset.subId).toBe(
        'task_alpha_sub_02',
      );
    });
    dispose();
  });

  it('shows dead-link empty state for unknown masterId', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_missing',
      subId: 'task_missing_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-split-dead-link')).not.toBeNull();
    });
    dispose();
  });

  it('shows dead-link empty state for unknown subId on valid master', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_missing',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-split-dead-link')).not.toBeNull();
    });
    dispose();
  });

  it('shows error empty state when GET fails', async () => {
    getJsonMock.mockRejectedValue(new Error('Failed to fetch'));
    const { dispose } = mountTodoTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-split-error')).not.toBeNull();
    });
    dispose();
  });

  it('updates selection when master item is clicked', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.todo-task-master-item')).toHaveLength(3);
    });
    container.querySelector('[data-master-id="task_migrated_err"]').click();
    await vi.waitFor(() => {
      expect(
        container.querySelector('.todo-task-master-item--selected')?.dataset.masterId,
      ).toBe('task_migrated_err');
      expect(
        container.querySelector('.todo-task-sub-title-input')?.value,
      ).toBe('Still visible sub');
    });
    dispose();
  });

  it('preserves master list scroll when selecting another master', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('.todo-task-master-item--selected')?.dataset.masterId,
      ).toBe('task_alpha');
    });
    const masterPane = container.querySelector('.todo-task-split-master');
    masterPane.scrollTop = 140;
    container.querySelector('[data-master-id="task_migrated_err"]').click();
    await vi.waitFor(() => {
      expect(
        container.querySelector('.todo-task-master-item--selected')?.dataset.masterId,
      ).toBe('task_migrated_err');
    });
    expect(container.querySelector('.todo-task-split-master')?.scrollTop).toBe(140);
    dispose();
  });

  it('does not reset detail scroll when re-clicking the selected master', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-split-detail')).not.toBeNull();
    });
    const detailPane = container.querySelector('.todo-task-split-detail');
    detailPane.scrollTop = 220;
    container.querySelector('[data-master-id="task_alpha"]').click();
    await Promise.resolve();
    expect(container.querySelector('.todo-task-split-detail')?.scrollTop).toBe(220);
    expect(
      container.querySelector('.todo-task-master-item--selected')?.dataset.masterId,
    ).toBe('task_alpha');
    dispose();
  });

  it('preserves detail scroll across same-master paint', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-split-detail')).not.toBeNull();
    });
    const detailPane = container.querySelector('.todo-task-split-detail');
    detailPane.scrollTop = 180;
    document.dispatchEvent(new CustomEvent('todo-task-dialog-close'));
    expect(container.querySelector('.todo-task-split-detail')?.scrollTop).toBe(180);
    dispose();
  });

  it('applyRoute updates selection in-place and keeps master scroll', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const api = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('.todo-task-master-item--selected')?.dataset.masterId,
      ).toBe('task_alpha');
    });
    const masterPane = container.querySelector('.todo-task-split-master');
    masterPane.scrollTop = 99;
    api.applyRoute({
      masterId: 'task_migrated_err',
      subId: 'task_migrated_err_sub_01',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('.todo-task-master-item--selected')?.dataset.masterId,
      ).toBe('task_migrated_err');
    });
    expect(container.querySelector('.todo-task-split-master')?.scrollTop).toBe(99);
    api.dispose();
  });

  it('renders title-area master status select with English labels and title markers', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="change-master-status"]'),
      ).not.toBeNull();
    });
    const statusSelect = container.querySelector('[data-action="change-master-status"]');
    expect(statusSelect).toBeInstanceOf(HTMLSelectElement);
    expect(statusSelect.value).toBe('incomplete');
    expect(statusSelect.className).toMatch(/todo-task-master-status-select--incomplete/);
    const optionTexts = [...statusSelect.options].map((opt) => opt.textContent);
    expect(optionTexts).toEqual(['In progress', 'Completed', 'Abandoned']);
    const titleInput = container.querySelector('[data-action="edit-master-title"]');
    expect(titleInput.value).toBe('Alpha Task');
    expect(titleInput.className).toMatch(/todo-task-detail-title--incomplete/);
    dispose();
  });

  it('marks complete title muted and abandoned title with strikethrough class without changing title text', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose: disposeComplete } = mountTodoTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('[data-action="toggle-active-only"]')).not.toBeNull();
    });
    container.querySelector('[data-action="toggle-active-only"]').click();
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="toggle-active-only"]')?.getAttribute('aria-checked'),
      ).toBe('false');
      expect(container.querySelector('[data-master-id="task_complete"]')).not.toBeNull();
    });
    container.querySelector('[data-master-id="task_complete"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('[data-action="edit-master-title"]')?.value).toBe(
        'Completed Master',
      );
    });
    const completeTitle = container.querySelector('[data-action="edit-master-title"]');
    expect(completeTitle.value).toBe('Completed Master');
    expect(completeTitle.className).toMatch(/todo-task-detail-title--complete/);
    disposeComplete();

    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose: disposeAbandoned } = mountTodoTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('[data-action="toggle-active-only"]')).not.toBeNull();
    });
    container.querySelector('[data-action="toggle-active-only"]').click();
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="toggle-active-only"]')?.getAttribute('aria-checked'),
      ).toBe('false');
      expect(container.querySelector('[data-master-id="task_abandoned"]')).not.toBeNull();
    });
    container.querySelector('[data-master-id="task_abandoned"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('[data-action="edit-master-title"]')?.value).toBe(
        'Abandoned Master',
      );
    });
    const abandonedTitle = container.querySelector('[data-action="edit-master-title"]');
    expect(abandonedTitle.value).toBe('Abandoned Master');
    expect(abandonedTitle.className).toMatch(/todo-task-detail-title--abandoned/);
    disposeAbandoned();
  });

  it('invokes set_todo_master_status when title-area status changes', async () => {
    const invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'set_todo_master_status') {
        return {
          task: { ...sampleMasters[0], status: args.status },
          _status: 200,
        };
      }
      if (cmd === 'list_todo_attachments') return [];
      return {};
    });
    window.__TAURI__ = { core: { invoke: invokeMock } };
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="change-master-status"]'),
      ).not.toBeNull();
    });
    const statusSelect = container.querySelector('[data-action="change-master-status"]');
    statusSelect.value = 'abandoned';
    statusSelect.dispatchEvent(new Event('change', { bubbles: true }));
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('set_todo_master_status', {
        masterTaskId: 'task_alpha',
        status: 'abandoned',
      });
    });
    expect(setPlanMasterStatus).toBeTypeOf('function');
    dispose();
    delete window.__TAURI__;
  });

  it('copies formatted title and ID from plan-md header next to edit', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-action="copy-master-id"]')).not.toBeNull();
    });
    const actions = container.querySelector('.todo-task-plan-md-header-actions');
    const copyBtn = actions?.querySelector('[data-action="copy-master-id"]');
    const editBtn = actions?.querySelector('[data-action="edit-plan-md"]');
    expect(copyBtn?.textContent).toBe('Copy');
    expect(copyBtn?.dataset.copyText).toBe('Title: Alpha Task\nID: todo_task_alpha');
    expect(editBtn).not.toBeNull();
    expect(container.querySelector('.todo-task-detail-toolbar [data-action="copy-master-id"]')).toBeNull();
    copyBtn.click();
    expect(writeText).toHaveBeenCalledWith('Title: Alpha Task\nID: todo_task_alpha');
    await vi.waitFor(() => {
      expect(copyBtn.textContent).toBe('✓ Copied');
      expect(copyBtn.classList.contains('todo-task-copy-flash')).toBe(true);
    });
    dispose();
  });

  it('shows normal empty sub list state with add entry', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_empty',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-empty--detail')).not.toBeNull();
      expect(container.textContent).toContain('No sub-tasks yet');
      expect(container.querySelector('[data-action="add-sub"]')).not.toBeNull();
      expect(container.querySelector('.todo-task-split-state--error')).toBeNull();
    });
    dispose();
  });

  it('shows non-blocking migration_error banner while keeping plan operable', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_migrated_err',
      subId: 'task_migrated_err_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-migration-warning')).not.toBeNull();
      expect(container.textContent).toMatch(/migration|data/i);
      expect(container.querySelector('.todo-task-sub-list .todo-task-sub')).not.toBeNull();
      expect(container.querySelector('[data-action="add-sub"]')).not.toBeNull();
    });
    dispose();
  });

  it('calls navigate with updated hash when sub is clicked', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const navigate = vi.fn();
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
      navigate,
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-sub-id="task_alpha_sub_02"]')).not.toBeNull();
    });
    container.querySelector('[data-sub-id="task_alpha_sub_02"]').click();
    expect(navigate).toHaveBeenCalledWith(
      '#/todo-tasks?master=task_alpha&sub=task_alpha_sub_02',
    );
    dispose();
  });

  it('renders detail title as editable input and saves on blur', async () => {
    const invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'update_todo_master_title') {
        return {
          task: { ...sampleMasters[0], title: args.title },
          _status: 200,
        };
      }
      if (cmd === 'list_todo_attachments') return [];
      return {};
    });
    window.__TAURI__ = { core: { invoke: invokeMock } };
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="edit-master-title"]'),
      ).not.toBeNull();
    });
    const titleInput = container.querySelector('[data-action="edit-master-title"]');
    expect(titleInput).toBeInstanceOf(HTMLInputElement);
    expect(titleInput.value).toBe('Alpha Task');
    expect(titleInput.tagName).toBe('INPUT');
    titleInput.value = 'Alpha Renamed';
    titleInput.dispatchEvent(new Event('input', { bubbles: true }));
    titleInput.dispatchEvent(new FocusEvent('focusout', { bubbles: true }));
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('update_todo_master_title', {
        masterTaskId: 'task_alpha',
        title: 'Alpha Renamed',
      });
    });
    dispose();
    delete window.__TAURI__;
  });
});

describe('todo-tasks route source wiring', () => {
  it('index.html includes #todo-tasks-view shell', () => {
    expect(indexHtml).toMatch(/id="todo-tasks-view"/);
  });

  it('TodoTaskDialog includes todo-task dialog shell', () => {
    expect(todoTaskDialogSrc).toMatch(/id="todo-task-dialog"/);
    expect(todoTaskDialogSrc).toMatch(/id="todo-task-dialog-primary"/);
    expect(todoTaskDialogSrc).toMatch(/export function TodoTaskDialog/);
    expect(todoTaskDialogSrc).toMatch(/export function openTodoTaskDialog/);
  });

  it('main.js defines mountTodoTasksRoute', () => {
    expect(mainJs).toMatch(/function mountTodoTasksRoute/);
  });

  it('main.js registers todo-tasks via wrapRouteMount in setRouteHandlers', () => {
    expect(mainJs).toMatch(/setRouteHandlers\s*\(/);
    expect(mainJs).toMatch(
      /['"]todo-tasks['"]:\s*wrapRouteMount\s*\(\s*['"]todo-tasks['"]\s*,\s*mountTodoTasksRoute/,
    );
  });

  it('mountTodoTasksRoute does not createRoot-mount Todos', () => {
    const body = extractFunctionBody(mainJs, 'mountTodoTasksRoute');
    expect(body).not.toMatch(/mountTodoTaskSplit/);
    expect(body).not.toMatch(/createRoot/);
  });

  it('ShellPages paints TodoTasksPage with master/sub from the hash', () => {
    expect(mainJs).toMatch(/TodoTasksPage/);
    expect(mainJs).toMatch(/routeParams/);
    expect(mainJs).toMatch(/masterId=\{routeParams\.master/);
    expect(mainJs).toMatch(/subId=\{routeParams\.sub/);
  });

  it('TodoTasksPage applies deep-link in-place via applyRoute', () => {
    const pageSrc = readFrontendJs('frontend/src/todo-task/page.tsx');
    expect(pageSrc).toMatch(/function TodoTasksPage/);
    expect(pageSrc).toMatch(/applyRoute/);
    expect(pageSrc).toMatch(/sessionRef\.current\?\.applyRoute/);
  });

  it('app.css defines full-screen split layout classes', () => {
    expect(appCss).toMatch(/\.todo-task-split/);
    expect(appCss).toMatch(/\.todo-task-split-master/);
    expect(appCss).toMatch(/\.todo-task-split-detail/);
  });

  it('app.css blocks scroll chaining from split panes to the document', () => {
    expect(appCss).toMatch(
      /\.todo-task-split-master\s*\{[^}]*overscroll-behavior:\s*none/s,
    );
    expect(appCss).toMatch(
      /\.todo-task-split-detail\s*\{[^}]*overscroll-behavior:\s*none/s,
    );
    expect(appCss).toMatch(/body\s*\{[^}]*overflow:\s*hidden/s);
  });

  it('app.css styles plan-md preview and three sub status variants', () => {
    expect(appCss).toMatch(/\.todo-task-plan-md-preview/);
    expect(appCss).toMatch(/\.todo-task-sub-status-select--abandoned/);
    expect(appCss).not.toMatch(/\.todo-task-plan-md-preview[\s\S]*background:\s*#000/);
  });

  it('app.css styles master title/status markers for complete muted and abandoned strike/gray', () => {
    expect(appCss).toMatch(/\.todo-task-detail-title--complete/);
    expect(appCss).toMatch(/\.todo-task-detail-title--abandoned/);
    expect(appCss).toMatch(/\.todo-task-master-item--abandoned/);
    expect(appCss).toMatch(/\.todo-task-master-status-select--abandoned/);
  });

  it('does not map set-status as an MCP tool', () => {
    const mcpIndex = readRsPath(
      join(fixtureRoot, 'src-tauri/src/services/mcp_host'),
    );
    expect(mcpIndex).not.toMatch(/set_todo_master_status|todo-task-set-status|set_master_status/);
  });

  it('app.css keeps plan-md preview inside bordered box under flex layout', () => {
    expect(appCss).toMatch(/#todo-tasks-view\s*\{[^}]*min-width:\s*0/s);
    expect(appCss).toMatch(/\.todo-tasks-page\s*\{[^}]*min-width:\s*0/s);
    expect(appCss).toMatch(/\.todo-task-split\s*\{[^}]*min-width:\s*0/s);
    expect(appCss).toMatch(
      /\.todo-task-split-detail\s*\{[^}]*min-width:\s*0/s,
    );
    expect(appCss).toMatch(
      /\.todo-task-plan-md-preview\s*\{[^}]*width:\s*100%/s,
    );
    expect(appCss).toMatch(
      /\.todo-task-plan-md-preview\s*\{[^}]*overflow-wrap:\s*anywhere/s,
    );
    expect(appCss).toMatch(
      /\.todo-task-plan-md-preview ul,\s*\n\s*\.todo-task-plan-md-preview ol/,
    );
  });
});
