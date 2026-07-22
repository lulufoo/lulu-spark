// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const invokeMock = vi.fn();
const getJsonMock = vi.fn();

vi.mock('../frontend/js/apiClient.js', async (importOriginal) => {
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
  addPlanSub,
  createPlanTask,
  deletePlanSub,
  deletePlanTask,
  loadPlanTasks,
  mountPlanTaskSplit,
  updatePlanMasterTitle,
} from '../frontend/js/plan-task/index.js';

const sampleMaster = {
  master_task_id: 'task_new',
  title: 'New Plan',
  status: 'incomplete',
  created_at: '2026-07-08T10:00:00Z',
  sub_tasks: [
    {
      sub_task_id: 'task_new_sub_01',
      title: 'Sub A',
      status: 'incomplete',
      implicit: false,
      linked_archive_ids: [],
    },
  ],
};

const createInvokeResult = {
  master_task_id: 'task_new',
  task: sampleMaster,
};

describe('createPlanTask', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('invokes create_todo_task with title and subTitles', async () => {
    invokeMock.mockResolvedValue(createInvokeResult);
    const result = await createPlanTask({ title: 'New Plan', subTitles: ['Sub A'] });
    expect(invokeMock).toHaveBeenCalledWith('create_todo_task', {
      title: 'New Plan',
      subTitles: ['Sub A'],
    });
    expect(result.task).toEqual(sampleMaster);
    expect(result.task.sub_tasks).toHaveLength(1);
  });

  it('omits subTitles when subTitles is undefined', async () => {
    invokeMock.mockResolvedValue(createInvokeResult);
    await createPlanTask({ title: 'Implicit Plan' });
    expect(invokeMock).toHaveBeenCalledWith('create_todo_task', { title: 'Implicit Plan' });
    expect(invokeMock.mock.calls[0][1]).not.toHaveProperty('subTitles');
  });

  it('omits subTitles when subTitles is null', async () => {
    invokeMock.mockResolvedValue(createInvokeResult);
    await createPlanTask({ title: 'Implicit Plan', subTitles: null });
    expect(invokeMock).toHaveBeenCalledWith('create_todo_task', { title: 'Implicit Plan' });
    expect(invokeMock.mock.calls[0][1]).not.toHaveProperty('subTitles');
  });

  it('throws with status when invoke returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Missing title', _status: 400 });
    await expect(createPlanTask({ title: '' })).rejects.toMatchObject({ status: 400 });
  });

  it('throws when invoke rejects', async () => {
    invokeMock.mockRejectedValue(new Error('IPC failed'));
    await expect(createPlanTask({ title: 'X' })).rejects.toThrow('IPC failed');
  });

  it('does not use createApiClient or local_http', async () => {
    invokeMock.mockResolvedValue(createInvokeResult);
    await createPlanTask({ title: 'Direct' });
    expect(getJsonMock).not.toHaveBeenCalled();
  });
});

describe('deletePlanTask', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('invokes delete_todo_task with masterTaskId', async () => {
    invokeMock.mockResolvedValue({ ok: true });
    await deletePlanTask({ masterTaskId: 'task_new' });
    expect(invokeMock).toHaveBeenCalledWith('delete_todo_task', {
      masterTaskId: 'task_new',
    });
  });

  it('throws with status when invoke returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Master not found', _status: 404 });
    await expect(deletePlanTask({ masterTaskId: 'missing' })).rejects.toMatchObject({
      status: 404,
    });
  });
});

describe('addPlanSub', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('invokes add_todo_sub with masterTaskId and title', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    const result = await addPlanSub({ masterTaskId: 'task_new', title: 'Sub B' });
    expect(invokeMock).toHaveBeenCalledWith('add_todo_sub', {
      masterTaskId: 'task_new',
      title: 'Sub B',
    });
    expect(result.task.sub_tasks).toBeDefined();
  });

  it('throws with status when invoke returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Master not found', _status: 404 });
    await expect(
      addPlanSub({ masterTaskId: 'missing', title: 'Sub' }),
    ).rejects.toMatchObject({ status: 404 });
  });
});

describe('deletePlanSub', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('invokes delete_todo_sub with masterTaskId and subTaskId', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    const result = await deletePlanSub({
      masterTaskId: 'task_new',
      subTaskId: 'task_new_sub_01',
    });
    expect(invokeMock).toHaveBeenCalledWith('delete_todo_sub', {
      masterTaskId: 'task_new',
      subTaskId: 'task_new_sub_01',
    });
    expect(result.task.sub_tasks).toBeDefined();
  });

  it('throws with status when invoke returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Cannot delete last sub', _status: 400 });
    await expect(
      deletePlanSub({ masterTaskId: 'task_new', subTaskId: 'task_new_sub_01' }),
    ).rejects.toMatchObject({ status: 400 });
  });
});

describe('updatePlanMasterTitle', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('invokes update_todo_master_title with masterTaskId and title', async () => {
    invokeMock.mockResolvedValue({ task: { ...sampleMaster, title: 'Renamed' } });
    const result = await updatePlanMasterTitle({
      masterTaskId: 'task_new',
      title: 'Renamed',
    });
    expect(invokeMock).toHaveBeenCalledWith('update_todo_master_title', {
      masterTaskId: 'task_new',
      title: 'Renamed',
    });
    expect(result.task.title).toBe('Renamed');
  });

  it('throws with status when invoke returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Missing title', _status: 400 });
    await expect(
      updatePlanMasterTitle({ masterTaskId: 'task_new', title: '' }),
    ).rejects.toMatchObject({ status: 400 });
  });
});

describe('plan-task read path regression', () => {
  let container;

  beforeEach(() => {
    getJsonMock.mockReset();
    container = document.createElement('div');
    document.body.appendChild(container);
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    container.remove();
  });

  it('loadPlanTasks still GETs /api/plan-tasks via apiClient', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    const entries = await loadPlanTasks();
    expect(getJsonMock).toHaveBeenCalledWith('/api/plan-tasks');
    expect(entries).toEqual([sampleMaster]);
  });

  it('mountPlanTaskSplit still loads read-only list', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    const { dispose } = mountPlanTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-split-master')).not.toBeNull();
    });
    dispose();
  });
});
