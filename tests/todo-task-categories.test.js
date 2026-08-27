// @vitest-environment jsdom
/**
 * T4: Todos UI category dropdown — select/create/delete/filter/reassign.
 * Host invoke: list_todo_categories / create_todo_category / delete_todo_category;
 * reassign via set_todo_category (todo update with category_id). No management page.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const getJsonMock = vi.fn();

vi.mock('../frontend/js/host/apiClient.js', async (importOriginal) => {
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
  createPlanCategory,
  deletePlanCategory,
  listPlanCategories,
  mountTodoTaskSplit,
  setPlanCategory,
} from '../frontend/js/todo-task/index.js';
import { parseHash } from '../frontend/js/router/index.js';
import { readTodoTaskUiSource } from './helpers/todo-task-ui-source.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const todoTaskIndex = readTodoTaskUiSource();
const routerSource = readFileSync(
  join(fixtureRoot, 'frontend/js/router/index.js'),
  'utf8',
);

const DEFAULT_CATEGORY = {
  id: 'uncategorized',
  name: '待分类',
  is_default: true,
};
const WORK_CATEGORY = {
  id: 'cat_work',
  name: 'Work',
  is_default: false,
};

const sampleMasters = [
  {
    master_task_id: 'task_alpha',
    title: 'Alpha Task',
    status: 'incomplete',
    created_at: '2026-07-01T10:00:00Z',
    category_id: 'uncategorized',
    sub_tasks: [
      {
        sub_task_id: 'task_alpha_sub_01',
        title: 'Alpha Sub A',
        status: 'incomplete',
        implicit: false,
        linked_archive_ids: [],
      },
    ],
  },
  {
    master_task_id: 'task_beta',
    title: 'Beta Task',
    status: 'incomplete',
    created_at: '2026-07-02T10:00:00Z',
    category_id: 'cat_work',
    sub_tasks: [],
  },
];

function waitFor(predicate, { timeoutMs = 2000 } = {}) {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const tick = () => {
      try {
        if (predicate()) {
          resolve();
          return;
        }
      } catch (err) {
        reject(err);
        return;
      }
      if (Date.now() - started > timeoutMs) {
        reject(new Error('waitFor timeout'));
        return;
      }
      setTimeout(tick, 10);
    };
    tick();
  });
}

describe('todo-task category source wiring (t4)', () => {
  it('invokes Host list/create/delete category commands (not Sidecar HTTP)', () => {
    expect(todoTaskIndex).toMatch(/list_todo_categories/);
    expect(todoTaskIndex).toMatch(/create_todo_category/);
    expect(todoTaskIndex).toMatch(/delete_todo_category/);
    expect(todoTaskIndex).toMatch(/set_todo_category/);
    expect(todoTaskIndex).not.toMatch(/\/api\/todo-task-list-categories/);
    expect(todoTaskIndex).not.toMatch(/sediment_kb_.*category/);
  });

  it('has no separate category management route', () => {
    expect(routerSource).not.toMatch(/todo-task-categories|todo-categories|category-manage/);
    expect(parseHash('#/todo-tasks')).toEqual({ name: 'todo-tasks', params: {} });
    expect(parseHash('#/todo-task-categories').name).not.toBe('todo-task-categories');
  });
});

describe('category Host invoke helpers', () => {
  let invokeMock;

  beforeEach(() => {
    invokeMock = vi.fn();
    window.__TAURI__ = { core: { invoke: invokeMock } };
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('listPlanCategories invokes list_todo_categories', async () => {
    invokeMock.mockResolvedValue({
      categories: [DEFAULT_CATEGORY, WORK_CATEGORY],
      _status: 200,
    });
    const result = await listPlanCategories();
    expect(invokeMock).toHaveBeenCalledWith('list_todo_categories', {});
    expect(result).toEqual([DEFAULT_CATEGORY, WORK_CATEGORY]);
  });

  it('createPlanCategory invokes create_todo_category with name', async () => {
    invokeMock.mockResolvedValue({
      category_id: 'cat_new',
      category: { id: 'cat_new', name: 'New', is_default: false },
      _status: 201,
    });
    const result = await createPlanCategory({ name: 'New' });
    expect(invokeMock).toHaveBeenCalledWith('create_todo_category', { name: 'New' });
    expect(result.category_id).toBe('cat_new');
  });

  it('deletePlanCategory invokes delete_todo_category with categoryId', async () => {
    invokeMock.mockResolvedValue({ ok: true, _status: 200 });
    await deletePlanCategory({ categoryId: 'cat_work' });
    expect(invokeMock).toHaveBeenCalledWith('delete_todo_category', {
      categoryId: 'cat_work',
    });
  });

  it('deletePlanCategory surfaces Host failure (does not pretend success)', async () => {
    invokeMock.mockResolvedValue({
      error: 'Cannot delete default category',
      _status: 400,
    });
    await expect(
      deletePlanCategory({ categoryId: 'uncategorized' }),
    ).rejects.toMatchObject({ status: 400 });
  });

  it('setPlanCategory invokes set_todo_category with masterTaskId and categoryId', async () => {
    invokeMock.mockResolvedValue({
      task: { ...sampleMasters[0], category_id: 'cat_work' },
      _status: 200,
    });
    await setPlanCategory({
      masterTaskId: 'task_alpha',
      categoryId: 'cat_work',
    });
    expect(invokeMock).toHaveBeenCalledWith('set_todo_category', {
      masterTaskId: 'task_alpha',
      categoryId: 'cat_work',
    });
  });
});

describe('mountTodoTaskSplit category dropdown', () => {
  let container;
  let invokeMock;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    const dialogHost = document.createElement('div');
    dialogHost.id = 'todo-task-dialog-host';
    dialogHost.innerHTML = `
      <div id="todo-task-dialog">
        <div id="todo-task-dialog-box">
          <div id="todo-task-dialog-header">
            <h3 id="todo-task-dialog-title"></h3>
          </div>
          <div id="todo-task-dialog-body"></div>
          <p id="todo-task-dialog-error" hidden></p>
          <div id="todo-task-dialog-actions">
            <button type="button" id="todo-task-dialog-cancel" class="md-header-btn">Cancel</button>
            <button type="button" id="todo-task-dialog-primary" class="md-header-btn primary">OK</button>
          </div>
        </div>
      </div>
    `;
    document.body.appendChild(dialogHost);
    getJsonMock.mockReset();
    getJsonMock.mockResolvedValue(sampleMasters);
    invokeMock = vi.fn().mockImplementation(async (cmd, args) => {
      if (cmd === 'list_todo_categories') {
        return { categories: [DEFAULT_CATEGORY, WORK_CATEGORY], _status: 200 };
      }
      if (cmd === 'set_binding') return { ok: true, state: 'bound' };
      if (cmd === 'reset_binding') return { ok: true, state: 'unbound' };
      if (cmd === 'list_todo_attachments') return [];
      if (cmd === 'list_todo_comments') return [];
      if (cmd === 'create_todo_category') {
        return {
          category_id: 'cat_new',
          category: { id: 'cat_new', name: args?.name ?? 'New', is_default: false },
          _status: 201,
        };
      }
      if (cmd === 'delete_todo_category') {
        if (args?.categoryId === 'uncategorized') {
          return { error: 'Cannot delete default category', _status: 400 };
        }
        if (args?.categoryId === 'cat_work') {
          return { error: 'Category not empty', _status: 400 };
        }
        return { ok: true, _status: 200 };
      }
      if (cmd === 'set_todo_category') {
        return {
          task: { ...sampleMasters[0], category_id: args?.categoryId },
          _status: 200,
        };
      }
      return {};
    });
    window.__TAURI__ = {
      core: { invoke: invokeMock },
      event: { listen: vi.fn(async () => vi.fn()) },
    };
  });

  afterEach(() => {
    container.remove();
    document.getElementById('todo-task-dialog-host')?.remove();
    delete window.__TAURI__;
    vi.restoreAllMocks();
  });

  it('shows create/delete as options inside the category filter select', async () => {
    const { dispose } = mountTodoTaskSplit(container);
    await waitFor(() => container.querySelector('.todo-task-category-filter') != null);
    const filter = container.querySelector('.todo-task-category-filter');
    expect(filter).not.toBeNull();
    expect(filter.textContent).toContain('Uncategorized');
    expect(filter.textContent).not.toContain('待分类');
    expect(filter.textContent).toContain('Work');
    expect(filter.textContent).toMatch(/All categories/i);
    expect(filter.textContent).toContain('+ Add category');
    expect(filter.textContent).not.toContain('…');
    expect(filter.textContent).toContain('Delete category');
    expect(filter.style.width).toMatch(/^\d+px$/);
    expect(Number.parseFloat(filter.style.width)).toBeGreaterThan(120);
    expect(
      [...filter.querySelectorAll('option')].some((o) => o.value === '__create_category__'),
    ).toBe(true);
    expect(
      [...filter.querySelectorAll('option')].some((o) => o.value === '__delete_category__'),
    ).toBe(true);
    expect(container.querySelector('.todo-task-category-menu')).toBeNull();
    expect(invokeMock).toHaveBeenCalledWith('list_todo_categories', {});
    dispose();
  });

  it('filters the master list by selected category', async () => {
    const { dispose } = mountTodoTaskSplit(container);
    await waitFor(() => container.querySelector('.todo-task-category-filter') != null);
    const filter = container.querySelector('.todo-task-category-filter');
    filter.value = 'cat_work';
    filter.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(() => {
      const items = [...container.querySelectorAll('.todo-task-master-item')];
      return items.length === 1 && items[0].dataset.masterId === 'task_beta';
    });
    expect(container.querySelector('[data-master-id="task_alpha"]')).toBeNull();
    dispose();
  });

  it('creates a category via create_todo_category dialog and refreshes the filter', async () => {
    const { dispose } = mountTodoTaskSplit(container);
    await waitFor(() => container.querySelector('.todo-task-category-filter') != null);
    const filter = container.querySelector('.todo-task-category-filter');
    filter.value = '__create_category__';
    filter.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(() =>
      document.getElementById('todo-task-dialog')?.classList.contains('open'),
    );
    expect(document.getElementById('todo-task-dialog-title')?.textContent).toBe('New category');
    const nameInput = document.querySelector('#todo-task-dialog-body [data-field="name"]');
    expect(nameInput).not.toBeNull();
    nameInput.value = 'Ideas';
    document.getElementById('todo-task-dialog-primary').click();
    await waitFor(() =>
      invokeMock.mock.calls.some((c) => c[0] === 'create_todo_category'),
    );
    expect(invokeMock).toHaveBeenCalledWith('create_todo_category', { name: 'Ideas' });
    await waitFor(() =>
      invokeMock.mock.calls.filter((c) => c[0] === 'list_todo_categories').length >= 2,
    );
    dispose();
  });

  it('does not pretend success when deleting default/non-empty category fails', async () => {
    const { dispose } = mountTodoTaskSplit(container);
    await waitFor(() => container.querySelector('.todo-task-category-filter') != null);

    let filter = container.querySelector('.todo-task-category-filter');
    filter.value = 'uncategorized';
    filter.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(() => container.querySelector('.todo-task-category-filter')?.value === 'uncategorized');
    const deleteOpt = [...container.querySelectorAll('.todo-task-category-filter option')].find(
      (o) => o.value === '__delete_category__',
    );
    expect(deleteOpt?.disabled).toBe(true);

    // Re-query after paint — prior select node is detached.
    filter = container.querySelector('.todo-task-category-filter');
    filter.value = 'cat_work';
    filter.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(() => {
      const opt = [...container.querySelectorAll('.todo-task-category-filter option')].find(
        (o) => o.value === '__delete_category__',
      );
      return opt && !opt.disabled;
    });
    filter = container.querySelector('.todo-task-category-filter');
    filter.value = '__delete_category__';
    filter.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(() =>
      Boolean(container.querySelector('.todo-task-category-error')?.textContent?.length),
    );
    expect(container.querySelector('.todo-task-category-error').textContent).toMatch(
      /not empty|failed|cannot delete/i,
    );
    expect(
      [...container.querySelectorAll('.todo-task-category-filter option')].some(
        (o) => o.value === 'cat_work',
      ),
    ).toBe(true);
    dispose();
  });

  it('reassigns selected todo via set_todo_category', async () => {
    const { dispose } = mountTodoTaskSplit(container, { masterId: 'task_alpha' });
    await waitFor(
      () => container.querySelector('[data-action="change-master-category"]') != null,
    );
    const select = container.querySelector('[data-action="change-master-category"]');
    expect(select.value).toBe('uncategorized');
    select.value = 'cat_work';
    select.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(() =>
      invokeMock.mock.calls.some((c) => c[0] === 'set_todo_category'),
    );
    expect(invokeMock).toHaveBeenCalledWith('set_todo_category', {
      masterTaskId: 'task_alpha',
      categoryId: 'cat_work',
    });
    dispose();
  });
});
