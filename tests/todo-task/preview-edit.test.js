// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const invokeMock = vi.fn();
const getJsonMock = vi.fn();

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

import {
  abandonPlanSub,
  completePlan,
  formatTodoTaskStatus,
  mountTodoTaskSplit,
  readPlanMd,
  updatePlanMd,
} from '../../frontend/src/todo-task/index.ts';

const REFRESH_WARNING_MSG = 'Saved, but list refresh failed — retry';

/** Todos paint also GET /api/doc-highlights; do not let that consume list mocks. */
function mockTodoListReads(lists, { failAt } = {}) {
  let n = 0;
  getJsonMock.mockImplementation(async (path) => {
    if (String(path).startsWith('/api/doc-highlights')) {
      return { highlights: [] };
    }
    n += 1;
    if (failAt != null && n === failAt) {
      throw new Error('network down');
    }
    return lists[Math.min(n - 1, lists.length - 1)];
  });
}

const sampleMaster = {
  master_task_id: 'task_alpha',
  title: 'Alpha Task',
  status: 'incomplete',
  created_at: '2026-07-01T10:00:00Z',
  todo_md: '# Plan Title\n\nFirst paragraph.\n\nSecond paragraph.',
  sub_tasks: [
    {
      sub_task_id: 'task_alpha_sub_01',
      title: 'Alpha Sub A',
      status: 'incomplete',
      implicit: false,
      linked_archive_ids: [],
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
};

function setupTauri() {
  window.__TAURI__ = { core: { invoke: invokeMock } };
}

describe('formatTodoTaskStatus abandoned label', () => {
  it('includes abandoned English label', () => {
    expect(formatTodoTaskStatus('abandoned')).toBe('Abandoned');
  });
});

describe('todo_md invoke wrappers', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    setupTauri();
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('readPlanMd invokes read_todo_md with masterTaskId', async () => {
    invokeMock.mockResolvedValue('# Full text');
    const result = await readPlanMd({ masterTaskId: 'task_alpha' });
    expect(invokeMock).toHaveBeenCalledWith('read_todo_md', { masterTaskId: 'task_alpha' });
    expect(result).toBe('# Full text');
  });

  it('updatePlanMd invokes update_todo_md with masterTaskId and planMd', async () => {
    invokeMock.mockResolvedValue(undefined);
    await updatePlanMd({ masterTaskId: 'task_alpha', planMd: 'Updated' });
    expect(invokeMock).toHaveBeenCalledWith('update_todo_md', {
      masterTaskId: 'task_alpha',
      planMd: 'Updated',
    });
  });

  it('completePlan invokes complete_todo with optional subTaskId', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    await completePlan({
      masterTaskId: 'task_alpha',
      subTaskId: 'task_alpha_sub_01',
    });
    expect(invokeMock).toHaveBeenCalledWith('complete_todo', {
      masterTaskId: 'task_alpha',
      subTaskId: 'task_alpha_sub_01',
    });
  });

  it('completePlan without subTaskId invokes complete_todo for master', async () => {
    invokeMock.mockResolvedValue({
      task: { ...sampleMaster, status: 'complete' },
    });
    await completePlan({ masterTaskId: 'task_alpha' });
    expect(invokeMock).toHaveBeenCalledWith('complete_todo', {
      masterTaskId: 'task_alpha',
      subTaskId: undefined,
    });
  });

  it('abandonPlanSub invokes abandon_todo_sub', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    await abandonPlanSub({
      masterTaskId: 'task_alpha',
      subTaskId: 'task_alpha_sub_01',
    });
    expect(invokeMock).toHaveBeenCalledWith('abandon_todo_sub', {
      masterTaskId: 'task_alpha',
      subTaskId: 'task_alpha_sub_01',
    });
  });

  it('throws with status when invoke returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Already complete', _status: 409 });
    await expect(
      completePlan({ masterTaskId: 'task_alpha', subTaskId: 'task_alpha_sub_02' }),
    ).rejects.toMatchObject({ status: 409 });
  });
});

describe('mountTodoTaskSplit todo_md preview and edit', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    invokeMock.mockReset();
    setupTauri();
    global.marked = {
      parse: vi.fn((md) => `<div class="mock-md">${md.replace(/\n/g, '<br>')}</div>`),
    };
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    container.remove();
    delete window.__TAURI__;
    delete global.marked;
  });

  async function mountAndWait() {
    const api = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-plan-md-preview')).not.toBeNull();
    });
    return api;
  }

  it('renders todo_md from HTTP as multi-paragraph markdown preview', async () => {
    mockTodoListReads([[sampleMaster]]);
    const { dispose } = await mountAndWait();
    const preview = container.querySelector('.todo-task-plan-md-preview');
    expect(preview).not.toBeNull();
    expect(global.marked.parse).toHaveBeenCalledWith(sampleMaster.todo_md);
    expect(preview.innerHTML).toContain('First paragraph.');
    expect(preview.innerHTML).toContain('Second paragraph.');
    dispose();
  });

  it('loads full text via read_todo_md when entering edit mode', async () => {
    mockTodoListReads([[sampleMaster]]);
    invokeMock.mockResolvedValue('# Edited from disk\n\nExtra line.');
    const { dispose } = await mountAndWait();
    container.querySelector('[data-action="edit-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-plan-md-editor')).not.toBeNull();
    });
    expect(invokeMock).toHaveBeenCalledWith('read_todo_md', { masterTaskId: 'task_alpha' });
    const editor = container.querySelector('.todo-task-plan-md-editor');
    expect(editor.value).toContain('Edited from disk');
    dispose();
  });

  it('saves via update_todo_md and refreshes list on success', async () => {
    mockTodoListReads([
      [sampleMaster],
      [{ ...sampleMaster, todo_md: '# Saved\n\nNew content.' }],
    ]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'read_todo_md') return sampleMaster.todo_md;
      if (cmd === 'update_todo_md') return undefined;
      return {};
    });
    const { dispose } = await mountAndWait();
    container.querySelector('[data-action="edit-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-plan-md-editor')).not.toBeNull();
    });
    const editor = container.querySelector('.todo-task-plan-md-editor');
    editor.value = '# Saved\n\nNew content.';
    container.querySelector('[data-action="save-plan-md"]').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('update_todo_md', {
        masterTaskId: 'task_alpha',
        planMd: '# Saved\n\nNew content.',
      });
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-plan-md-editor')).toBeNull();
    });
    dispose();
  });

  it('keeps editor content and shows error when save fails', async () => {
    mockTodoListReads([[sampleMaster]]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'read_todo_md') return sampleMaster.todo_md;
      if (cmd === 'update_todo_md') {
        return { error: 'Disk full', _status: 500 };
      }
      return {};
    });
    const { dispose } = await mountAndWait();
    container.querySelector('[data-action="edit-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-plan-md-editor')).not.toBeNull();
    });
    const editor = container.querySelector('.todo-task-plan-md-editor');
    editor.value = 'User draft content';
    container.querySelector('[data-action="save-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-plan-md-error')).not.toBeNull();
    });
    expect(container.querySelector('.todo-task-plan-md-editor').value).toBe('User draft content');
    expect(container.textContent).toMatch(/Disk full|Save failed/);
    dispose();
  });

  it('shows refresh warning when save succeeds but refresh fails', async () => {
    mockTodoListReads([[sampleMaster]], { failAt: 2 });
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'read_todo_md') return sampleMaster.todo_md;
      if (cmd === 'update_todo_md') return undefined;
      return {};
    });
    const { dispose } = await mountAndWait();
    container.querySelector('[data-action="edit-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-plan-md-editor')).not.toBeNull();
    });
    container.querySelector('.todo-task-plan-md-editor').value = 'Saved text';
    container.querySelector('[data-action="save-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.textContent).toContain(REFRESH_WARNING_MSG);
    });
    dispose();
  });
});

describe('mountTodoTaskSplit status select actions', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    invokeMock.mockReset();
    setupTauri();
    global.marked = { parse: vi.fn((md) => `<p>${md}</p>`) };
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    container.remove();
    delete window.__TAURI__;
    delete global.marked;
  });

  async function mountAndWait() {
    const api = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-sub-id="task_alpha_sub_01"]')).not.toBeNull();
    });
    return api;
  }

  function subRow(subId) {
    return container.querySelector(`[data-sub-id="${subId}"]`);
  }

  function statusSelect(subId) {
    return subRow(subId).querySelector('[data-action="change-sub-status"]');
  }

  function changeSubStatus(subId, nextStatus) {
    const select = statusSelect(subId);
    select.value = nextStatus;
    select.dispatchEvent(new Event('change', { bubbles: true }));
  }

  it('enables status select only for incomplete subs', async () => {
    mockTodoListReads([[sampleMaster]]);
    const { dispose } = await mountAndWait();
    expect(statusSelect('task_alpha_sub_01')).not.toBeNull();
    expect(statusSelect('task_alpha_sub_01').disabled).toBe(false);
    expect(statusSelect('task_alpha_sub_02').disabled).toBe(true);
    expect(statusSelect('task_alpha_sub_03').disabled).toBe(true);
    expect(subRow('task_alpha_sub_02').querySelector('[data-action="delete-sub"]')).not.toBeNull();
    dispose();
  });

  it('completes sub via status select and refreshes to terminal state', async () => {
    mockTodoListReads([
      [sampleMaster],
      [
        {
          ...sampleMaster,
          sub_tasks: sampleMaster.sub_tasks.map((sub) =>
            sub.sub_task_id === 'task_alpha_sub_01'
              ? { ...sub, status: 'complete' }
              : sub,
          ),
        },
      ],
    ]);
    invokeMock.mockResolvedValue({ task: sampleMaster });
    const { dispose } = await mountAndWait();
    changeSubStatus('task_alpha_sub_01', 'complete');
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('complete_todo', {
        masterTaskId: 'task_alpha',
        subTaskId: 'task_alpha_sub_01',
      });
    });
    await vi.waitFor(() => {
      expect(statusSelect('task_alpha_sub_01').disabled).toBe(true);
      expect(statusSelect('task_alpha_sub_01').value).toBe('complete');
    });
    dispose();
  });

  it('abandons sub via status select and refreshes to terminal state', async () => {
    mockTodoListReads([
      [sampleMaster],
      [
        {
          ...sampleMaster,
          sub_tasks: sampleMaster.sub_tasks.map((sub) =>
            sub.sub_task_id === 'task_alpha_sub_01'
              ? { ...sub, status: 'abandoned' }
              : sub,
          ),
        },
      ],
    ]);
    invokeMock.mockResolvedValue({ task: sampleMaster });
    const { dispose } = await mountAndWait();
    changeSubStatus('task_alpha_sub_01', 'abandoned');
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('abandon_todo_sub', {
        masterTaskId: 'task_alpha',
        subTaskId: 'task_alpha_sub_01',
      });
    });
    await vi.waitFor(() => {
      expect(statusSelect('task_alpha_sub_01').value).toBe('abandoned');
      expect(statusSelect('task_alpha_sub_01').textContent).toContain('Abandoned');
    });
    dispose();
  });

  it('rolls back optimistic status update when complete invoke fails', async () => {
    mockTodoListReads([[sampleMaster]]);
    invokeMock.mockResolvedValue({ error: 'Already complete', _status: 409 });
    const { dispose } = await mountAndWait();
    changeSubStatus('task_alpha_sub_01', 'complete');
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-sub-action-error')).not.toBeNull();
    });
    expect(statusSelect('task_alpha_sub_01').value).toBe('incomplete');
    expect(statusSelect('task_alpha_sub_01').disabled).toBe(false);
    dispose();
  });

  it('shows user-readable error on 404 without blanking the page', async () => {
    mockTodoListReads([[sampleMaster]]);
    invokeMock.mockResolvedValue({ error: 'Sub not found', _status: 404 });
    const { dispose } = await mountAndWait();
    changeSubStatus('task_alpha_sub_01', 'abandoned');
    await vi.waitFor(() => {
      expect(container.textContent).toContain('Sub not found');
    });
    expect(container.querySelector('.todo-task-split-master')).not.toBeNull();
    dispose();
  });
});
