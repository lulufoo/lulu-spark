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
  abandonPlanSub,
  completePlanSub,
  formatPlanTaskStatus,
  mountPlanTaskSplit,
  readPlanMd,
  updatePlanMd,
} from '../frontend/js/plan-task/index.js';

const REFRESH_WARNING_MSG = '已保存，列表刷新失败，请重试';

const sampleMaster = {
  master_task_id: 'task_alpha',
  title: 'Alpha Task',
  status: 'incomplete',
  created_at: '2026-07-01T10:00:00Z',
  plan_md: '# Plan Title\n\nFirst paragraph.\n\nSecond paragraph.',
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

describe('formatPlanTaskStatus abandoned label', () => {
  it('includes abandoned Chinese label', () => {
    expect(formatPlanTaskStatus('abandoned')).toBe('已废弃');
  });
});

describe('plan_md invoke wrappers', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    setupTauri();
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('readPlanMd invokes read_plan_md with masterTaskId', async () => {
    invokeMock.mockResolvedValue('# Full text');
    const result = await readPlanMd({ masterTaskId: 'task_alpha' });
    expect(invokeMock).toHaveBeenCalledWith('read_plan_md', { masterTaskId: 'task_alpha' });
    expect(result).toBe('# Full text');
  });

  it('updatePlanMd invokes update_plan_md with masterTaskId and planMd', async () => {
    invokeMock.mockResolvedValue(undefined);
    await updatePlanMd({ masterTaskId: 'task_alpha', planMd: 'Updated' });
    expect(invokeMock).toHaveBeenCalledWith('update_plan_md', {
      masterTaskId: 'task_alpha',
      planMd: 'Updated',
    });
  });

  it('completePlanSub invokes complete_plan_sub', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    await completePlanSub({
      masterTaskId: 'task_alpha',
      subTaskId: 'task_alpha_sub_01',
    });
    expect(invokeMock).toHaveBeenCalledWith('complete_plan_sub', {
      masterTaskId: 'task_alpha',
      subTaskId: 'task_alpha_sub_01',
    });
  });

  it('abandonPlanSub invokes abandon_plan_sub', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    await abandonPlanSub({
      masterTaskId: 'task_alpha',
      subTaskId: 'task_alpha_sub_01',
    });
    expect(invokeMock).toHaveBeenCalledWith('abandon_plan_sub', {
      masterTaskId: 'task_alpha',
      subTaskId: 'task_alpha_sub_01',
    });
  });

  it('throws with status when invoke returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Already complete', _status: 409 });
    await expect(
      completePlanSub({ masterTaskId: 'task_alpha', subTaskId: 'task_alpha_sub_02' }),
    ).rejects.toMatchObject({ status: 409 });
  });
});

describe('mountPlanTaskSplit plan_md preview and edit', () => {
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
    const api = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-plan-md-preview')).not.toBeNull();
    });
    return api;
  }

  it('renders plan_md from HTTP as multi-paragraph markdown preview', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    const { dispose } = await mountAndWait();
    const preview = container.querySelector('.plan-task-plan-md-preview');
    expect(preview).not.toBeNull();
    expect(global.marked.parse).toHaveBeenCalledWith(sampleMaster.plan_md);
    expect(preview.innerHTML).toContain('First paragraph.');
    expect(preview.innerHTML).toContain('Second paragraph.');
    dispose();
  });

  it('loads full text via read_plan_md when entering edit mode', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockResolvedValue('# Edited from disk\n\nExtra line.');
    const { dispose } = await mountAndWait();
    container.querySelector('[data-action="edit-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-plan-md-editor')).not.toBeNull();
    });
    expect(invokeMock).toHaveBeenCalledWith('read_plan_md', { masterTaskId: 'task_alpha' });
    const editor = container.querySelector('.plan-task-plan-md-editor');
    expect(editor.value).toContain('Edited from disk');
    dispose();
  });

  it('saves via update_plan_md and refreshes list on success', async () => {
    getJsonMock.mockResolvedValueOnce([sampleMaster]);
    getJsonMock.mockResolvedValueOnce([
      { ...sampleMaster, plan_md: '# Saved\n\nNew content.' },
    ]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'read_plan_md') return sampleMaster.plan_md;
      if (cmd === 'update_plan_md') return undefined;
      return {};
    });
    const { dispose } = await mountAndWait();
    container.querySelector('[data-action="edit-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-plan-md-editor')).not.toBeNull();
    });
    const editor = container.querySelector('.plan-task-plan-md-editor');
    editor.value = '# Saved\n\nNew content.';
    container.querySelector('[data-action="save-plan-md"]').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('update_plan_md', {
        masterTaskId: 'task_alpha',
        planMd: '# Saved\n\nNew content.',
      });
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-plan-md-editor')).toBeNull();
    });
    dispose();
  });

  it('keeps editor content and shows error when save fails', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'read_plan_md') return sampleMaster.plan_md;
      if (cmd === 'update_plan_md') {
        return { error: 'Disk full', _status: 500 };
      }
      return {};
    });
    const { dispose } = await mountAndWait();
    container.querySelector('[data-action="edit-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-plan-md-editor')).not.toBeNull();
    });
    const editor = container.querySelector('.plan-task-plan-md-editor');
    editor.value = 'User draft content';
    container.querySelector('[data-action="save-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-plan-md-error')).not.toBeNull();
    });
    expect(container.querySelector('.plan-task-plan-md-editor').value).toBe('User draft content');
    expect(container.textContent).toMatch(/Disk full|保存失败/);
    dispose();
  });

  it('shows refresh warning when save succeeds but refresh fails', async () => {
    getJsonMock.mockResolvedValueOnce([sampleMaster]);
    getJsonMock.mockRejectedValueOnce(new Error('network down'));
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'read_plan_md') return sampleMaster.plan_md;
      if (cmd === 'update_plan_md') return undefined;
      return {};
    });
    const { dispose } = await mountAndWait();
    container.querySelector('[data-action="edit-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-plan-md-editor')).not.toBeNull();
    });
    container.querySelector('.plan-task-plan-md-editor').value = 'Saved text';
    container.querySelector('[data-action="save-plan-md"]').click();
    await vi.waitFor(() => {
      expect(container.textContent).toContain(REFRESH_WARNING_MSG);
    });
    dispose();
  });
});

describe('mountPlanTaskSplit complete/abandon actions', () => {
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
    const api = mountPlanTaskSplit(container, {
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

  it('shows complete and abandon buttons only for incomplete subs', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    const { dispose } = await mountAndWait();
    expect(subRow('task_alpha_sub_01').querySelector('[data-action="complete-sub"]')).not.toBeNull();
    expect(subRow('task_alpha_sub_01').querySelector('[data-action="abandon-sub"]')).not.toBeNull();
    expect(subRow('task_alpha_sub_02').querySelector('[data-action="complete-sub"]')).toBeNull();
    expect(subRow('task_alpha_sub_02').querySelector('[data-action="abandon-sub"]')).toBeNull();
    expect(subRow('task_alpha_sub_03').querySelector('[data-action="complete-sub"]')).toBeNull();
    expect(subRow('task_alpha_sub_03').querySelector('[data-action="abandon-sub"]')).toBeNull();
    expect(subRow('task_alpha_sub_02').querySelector('[data-action="delete-sub"]')).not.toBeNull();
    dispose();
  });

  it('completes sub via invoke and refreshes to terminal state', async () => {
    getJsonMock.mockResolvedValueOnce([sampleMaster]);
    getJsonMock.mockResolvedValueOnce([
      {
        ...sampleMaster,
        sub_tasks: sampleMaster.sub_tasks.map((sub) =>
          sub.sub_task_id === 'task_alpha_sub_01'
            ? { ...sub, status: 'complete' }
            : sub,
        ),
      },
    ]);
    invokeMock.mockResolvedValue({ task: sampleMaster });
    const { dispose } = await mountAndWait();
    subRow('task_alpha_sub_01').querySelector('[data-action="complete-sub"]').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('complete_plan_sub', {
        masterTaskId: 'task_alpha',
        subTaskId: 'task_alpha_sub_01',
      });
    });
    await vi.waitFor(() => {
      expect(subRow('task_alpha_sub_01').querySelector('[data-action="complete-sub"]')).toBeNull();
    });
    dispose();
  });

  it('abandons sub via invoke and refreshes to terminal state', async () => {
    getJsonMock.mockResolvedValueOnce([sampleMaster]);
    getJsonMock.mockResolvedValueOnce([
      {
        ...sampleMaster,
        sub_tasks: sampleMaster.sub_tasks.map((sub) =>
          sub.sub_task_id === 'task_alpha_sub_01'
            ? { ...sub, status: 'abandoned' }
            : sub,
        ),
      },
    ]);
    invokeMock.mockResolvedValue({ task: sampleMaster });
    const { dispose } = await mountAndWait();
    subRow('task_alpha_sub_01').querySelector('[data-action="abandon-sub"]').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('abandon_plan_sub', {
        masterTaskId: 'task_alpha',
        subTaskId: 'task_alpha_sub_01',
      });
    });
    await vi.waitFor(() => {
      expect(subRow('task_alpha_sub_01').textContent).toContain('已废弃');
    });
    dispose();
  });

  it('rolls back optimistic status update when complete invoke fails', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockResolvedValue({ error: 'Already complete', _status: 409 });
    const { dispose } = await mountAndWait();
    subRow('task_alpha_sub_01').querySelector('[data-action="complete-sub"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-sub-action-error')).not.toBeNull();
    });
    expect(subRow('task_alpha_sub_01').textContent).toContain('进行中');
    expect(subRow('task_alpha_sub_01').querySelector('[data-action="complete-sub"]')).not.toBeNull();
    dispose();
  });

  it('shows user-readable error on 404 without blanking the page', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockResolvedValue({ error: 'Sub not found', _status: 404 });
    const { dispose } = await mountAndWait();
    subRow('task_alpha_sub_01').querySelector('[data-action="abandon-sub"]').click();
    await vi.waitFor(() => {
      expect(container.textContent).toContain('Sub not found');
    });
    expect(container.querySelector('.plan-task-split-master')).not.toBeNull();
    dispose();
  });
});
