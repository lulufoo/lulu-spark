/**
 * t4 — UI expand content editor + Process notes after sub-list
 * (tech-doc T4 / L14-T; AC4, AC5).
 */
// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
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
  mountTodoTaskSplit,
  renderSubDetailPane,
  renderSubRow,
  updatePlanSub,
} from '../frontend/js/todo-task/index.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

const sampleMaster = {
  master_task_id: 'task_new',
  title: 'New Plan',
  status: 'incomplete',
  created_at: '2026-07-08T10:00:00Z',
  todo_md: '# Plan',
  sub_tasks: [
    {
      sub_task_id: 'task_new_sub_01',
      title: 'Sub A',
      status: 'incomplete',
      implicit: false,
      linked_archive_ids: [],
      content: 'optional body',
    },
    {
      sub_task_id: 'task_new_sub_02',
      title: 'Sub B title only',
      status: 'incomplete',
      implicit: false,
      linked_archive_ids: [],
    },
  ],
};

function baseUi(overrides = {}) {
  return {
    disabled: false,
    subStatus: {},
    subActionErrors: {},
    subTitleErrors: {},
    subTitleDrafts: {},
    comments: [],
    commentsError: '',
    attachments: [],
    attachmentsError: '',
    ...overrides,
  };
}

describe('addPlanSub optional content', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('title-only invoke omits content field (backward compatible)', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    await addPlanSub({ masterTaskId: 'task_new', title: 'Sub B' });
    expect(invokeMock).toHaveBeenCalledWith('add_todo_sub', {
      masterTaskId: 'task_new',
      title: 'Sub B',
    });
    expect(invokeMock.mock.calls[0][1]).not.toHaveProperty('content');
  });

  it('passes optional content into add_todo_sub payload', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    await addPlanSub({
      masterTaskId: 'task_new',
      title: 'Sub C',
      content: 'create-time content',
    });
    expect(invokeMock).toHaveBeenCalledWith('add_todo_sub', {
      masterTaskId: 'task_new',
      title: 'Sub C',
      content: 'create-time content',
    });
  });

  it('keeps existing error path when add with content fails', async () => {
    invokeMock.mockResolvedValue({ error: 'Master not found', _status: 404 });
    await expect(
      addPlanSub({
        masterTaskId: 'missing',
        title: 'Sub',
        content: 'x',
      }),
    ).rejects.toMatchObject({ status: 404 });
  });
});

describe('updatePlanSub optional content', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('title-only invoke omits content field', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    await updatePlanSub({
      masterTaskId: 'task_new',
      subTaskId: 'task_new_sub_01',
      title: 'Renamed',
    });
    expect(invokeMock).toHaveBeenCalledWith('update_todo_sub', {
      masterTaskId: 'task_new',
      subTaskId: 'task_new_sub_01',
      title: 'Renamed',
    });
    expect(invokeMock.mock.calls[0][1]).not.toHaveProperty('content');
  });

  it('passes optional content into update_todo_sub payload', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    await updatePlanSub({
      masterTaskId: 'task_new',
      subTaskId: 'task_new_sub_01',
      title: 'Sub A',
      content: 'updated body',
    });
    expect(invokeMock).toHaveBeenCalledWith('update_todo_sub', {
      masterTaskId: 'task_new',
      subTaskId: 'task_new_sub_01',
      title: 'Sub A',
      content: 'updated body',
    });
  });

  it('passes content: "" to clear content', async () => {
    invokeMock.mockResolvedValue({ task: sampleMaster });
    await updatePlanSub({
      masterTaskId: 'task_new',
      subTaskId: 'task_new_sub_01',
      title: 'Sub A',
      content: '',
    });
    expect(invokeMock).toHaveBeenCalledWith('update_todo_sub', {
      masterTaskId: 'task_new',
      subTaskId: 'task_new_sub_01',
      title: 'Sub A',
      content: '',
    });
  });

  it('keeps existing error path when update with content fails (does not swallow)', async () => {
    invokeMock.mockResolvedValue({ error: 'Save failed', _status: 500 });
    await expect(
      updatePlanSub({
        masterTaskId: 'task_new',
        subTaskId: 'task_new_sub_01',
        title: 'Sub A',
        content: 'x',
      }),
    ).rejects.toMatchObject({ status: 500 });
  });
});

describe('renderSubRow title-first + default-collapsed content editor', () => {
  it('keeps title input as primary row and collapses content editor by default', () => {
    const html = renderSubRow(
      sampleMaster,
      sampleMaster.sub_tasks[0],
      'task_new_sub_01',
      baseUi(),
    );
    expect(html).toContain('todo-task-sub-title-input');
    expect(html).toContain('value="Sub A"');
    expect(html).toContain('data-action="toggle-sub-content"');
    expect(html).toMatch(/aria-expanded="false"/);
    expect(html).not.toContain('todo-task-sub-content-editor');
    expect(html).not.toContain('data-action="edit-sub-content"');
  });

  it('shows editable content editor only when session expand flag is set', () => {
    const html = renderSubRow(
      sampleMaster,
      sampleMaster.sub_tasks[0],
      'task_new_sub_01',
      baseUi({
        expandedSubContent: { task_new_sub_01: true },
      }),
    );
    expect(html).toContain('todo-task-sub-title-input');
    expect(html).toMatch(/aria-expanded="true"/);
    expect(html).toContain('todo-task-sub-content-editor');
    expect(html).toContain('data-action="edit-sub-content"');
    expect(html).toContain('optional body');
  });

  it('keeps title-only sub compact when collapsed (no empty editor placeholder)', () => {
    const html = renderSubRow(
      sampleMaster,
      sampleMaster.sub_tasks[1],
      null,
      baseUi(),
    );
    expect(html).toContain('Sub B title only');
    expect(html).toContain('todo-task-sub-title-input');
    expect(html).not.toContain('todo-task-sub-content-editor');
    // collapsed density: no empty textarea body for absent content
    expect(html).not.toMatch(/<textarea[\s\S]*data-action="edit-sub-content"/);
  });
});

describe('renderSubDetailPane Process notes after sub-list', () => {
  it('places Process notes (comments section) after the sub-list as bottom block', () => {
    const html = renderSubDetailPane(sampleMaster, 'task_new_sub_01', baseUi());
    const subListIdx = html.indexOf('todo-task-sub-list');
    const commentsIdx = html.indexOf('todo-task-comments-section');
    expect(subListIdx).toBeGreaterThan(-1);
    expect(commentsIdx).toBeGreaterThan(-1);
    expect(commentsIdx).toBeGreaterThan(subListIdx);
    // comments remain after attachments / toolbar / sub-list — last major content section
    const afterSubList = html.slice(subListIdx);
    expect(afterSubList).toContain('todo-task-comments-section');
    expect(afterSubList).toMatch(/Process notes/i);
  });
});

describe('mountTodoTaskSplit session-only expand + title error path', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    invokeMock.mockReset();
    window.__TAURI__ = { core: { invoke: invokeMock } };
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

  function mockReadApis(masters = [sampleMaster]) {
    getJsonMock.mockResolvedValue(masters);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_comments') return { comments: [], _status: 200 };
      if (cmd === 'list_todo_attachments') return { attachments: [], _status: 200 };
      return {};
    });
  }

  it('expands content editor in-session via toggle; remount defaults back to collapsed', async () => {
    mockReadApis();
    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_new',
      subId: 'task_new_sub_01',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-sub-id="task_new_sub_01"] [data-action="toggle-sub-content"]'),
      ).not.toBeNull();
    });
    expect(
      container.querySelector(
        '[data-sub-id="task_new_sub_01"] .todo-task-sub-content-editor',
      ),
    ).toBeNull();

    container
      .querySelector(
        '[data-sub-id="task_new_sub_01"] [data-action="toggle-sub-content"]',
      )
      .click();

    await vi.waitFor(() => {
      expect(
        container.querySelector(
          '[data-sub-id="task_new_sub_01"] .todo-task-sub-content-editor',
        ),
      ).not.toBeNull();
    });

    dispose();

    // remount = fresh session ui — expand must not persist
    const { dispose: dispose2 } = mountTodoTaskSplit(container, {
      masterId: 'task_new',
      subId: 'task_new_sub_01',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-sub-id="task_new_sub_01"] [data-action="toggle-sub-content"]'),
      ).not.toBeNull();
    });
    expect(
      container.querySelector(
        '[data-sub-id="task_new_sub_01"] .todo-task-sub-content-editor',
      ),
    ).toBeNull();
    dispose2();
  });

  it('title update failure still surfaces existing error UI when content optional path is present', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_comments') return { comments: [], _status: 200 };
      if (cmd === 'list_todo_attachments') return { attachments: [], _status: 200 };
      if (cmd === 'update_todo_sub') {
        return { error: 'Save failed', _status: 500 };
      }
      return {};
    });

    const { dispose } = mountTodoTaskSplit(container, {
      masterId: 'task_new',
      subId: 'task_new_sub_01',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector(
          '[data-sub-id="task_new_sub_01"] [data-action="edit-sub-title"]',
        ),
      ).not.toBeNull();
    });

    const titleInput = container.querySelector(
      '[data-sub-id="task_new_sub_01"] [data-action="edit-sub-title"]',
    );
    titleInput.value = 'Sub A renamed';
    titleInput.dispatchEvent(new Event('input', { bubbles: true }));
    titleInput.dispatchEvent(new FocusEvent('focusout', { bubbles: true }));

    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'update_todo_sub',
        expect.objectContaining({
          masterTaskId: 'task_new',
          subTaskId: 'task_new_sub_01',
          title: 'Sub A renamed',
        }),
      );
    });
    await vi.waitFor(() => {
      const err = container.querySelector(
        '[data-sub-id="task_new_sub_01"] .todo-task-sub-title-error',
      );
      expect(err).not.toBeNull();
      expect(err.textContent.trim().length).toBeGreaterThan(0);
    });
    dispose();
  });
});

describe('t4 test registration', () => {
  it('npm test includes todo-task-sub-content-ui.test.js', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toContain('tests/todo-task-sub-content-ui.test.js');
  });
});
