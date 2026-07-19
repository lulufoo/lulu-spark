// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

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

import { parseHash } from '../frontend/js/router/index.js';
import {
  copySubIdPair,
  formatPlanTaskStatus,
  loadPlanTasks,
  mountPlanTaskSplit,
  renderSubDetail,
} from '../frontend/js/plan-task/index.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const mainJs = readFileSync(join(fixtureRoot, 'frontend/js/main.js'), 'utf8');
const indexHtml = readFileSync(join(fixtureRoot, 'frontend/index.html'), 'utf8');
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
    plan_md: '',
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
    plan_md: '## Notes',
    migration_error: true,
  },
];

describe('parseHash plan-tasks deep-link', () => {
  it('parses #/plan-tasks without query params', () => {
    expect(parseHash('#/plan-tasks')).toEqual({
      name: 'plan-tasks',
      params: {},
    });
  });

  it('parses #/plan-tasks?master=&sub= query params', () => {
    expect(parseHash('#/plan-tasks?master=task_beta&sub=task_beta_sub_01')).toEqual({
      name: 'plan-tasks',
      params: { master: 'task_beta', sub: 'task_beta_sub_01' },
    });
  });

  it('parses trailing slash on plan-tasks route', () => {
    expect(parseHash('#/plan-tasks/')).toEqual({
      name: 'plan-tasks',
      params: {},
    });
  });
});

describe('copySubIdPair', () => {
  it('formats per-sub copy string as master → sub', () => {
    expect(copySubIdPair('task_alpha', 'task_alpha_sub_01')).toBe(
      'task_alpha → task_alpha_sub_01',
    );
  });
});

describe('formatPlanTaskStatus', () => {
  it('maps incomplete, complete, and abandoned to English labels', () => {
    expect(formatPlanTaskStatus('incomplete')).toBe('In progress');
    expect(formatPlanTaskStatus('complete')).toBe('Completed');
    expect(formatPlanTaskStatus('abandoned')).toBe('Abandoned');
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
    expect(html).toContain('data-copy-text="task_alpha → task_alpha_sub_01"');
    expect(html).toContain('data-copy-text="task_alpha → task_alpha_sub_02"');
  });

  it('renders distinct status modifier classes for three sub states', () => {
    const html = renderSubDetail(sampleMasters[0], 'task_alpha_sub_01');
    expect(html).toContain('plan-task-sub-status-select--incomplete');
    expect(html).toContain('plan-task-sub-status-select--complete');
    expect(html).toContain('plan-task-sub-status-select--abandoned');
  });

  it('renders linked_archive_ids as comma list with prefix', () => {
    const html = renderSubDetail(sampleMasters[0], 'task_alpha_sub_01');
    expect(html).toContain('Linked archives:');
    expect(html).toContain('arch_001, arch_002');
  });

  it('marks selected sub with selected class', () => {
    const html = renderSubDetail(sampleMasters[0], 'task_alpha_sub_02');
    expect(html).toMatch(/data-sub-id="task_alpha_sub_02"[^>]*plan-task-sub--selected/);
  });
});

describe('loadPlanTasks', () => {
  beforeEach(() => {
    getJsonMock.mockReset();
  });

  it('GET /api/plan-tasks via apiClient and returns master array', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const entries = await loadPlanTasks();
    expect(getJsonMock).toHaveBeenCalledWith('/api/plan-tasks');
    expect(entries).toEqual(sampleMasters);
  });

  it('throws with status when service returns unavailable payload', async () => {
    getJsonMock.mockResolvedValue({
      error: 'Workbench not running',
      _status: 503,
    });
    await expect(loadPlanTasks()).rejects.toMatchObject({
      status: 503,
    });
  });
});

describe('mountPlanTaskSplit', () => {
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
    const { dispose } = mountPlanTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-split-master')).not.toBeNull();
      expect(container.querySelector('.plan-task-split-detail')).not.toBeNull();
    });
    dispose();
  });

  it('shows right empty state when no master is selected', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountPlanTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-split-detail-empty')).not.toBeNull();
    });
    dispose();
  });

  it('selects master and sub from initial masterId/subId options', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_02',
    });
    await vi.waitFor(() => {
      expect(
        container.querySelector('.plan-task-master-item--selected')?.dataset.masterId,
      ).toBe('task_alpha');
      expect(container.querySelector('.plan-task-sub--selected')?.dataset.subId).toBe(
        'task_alpha_sub_02',
      );
    });
    dispose();
  });

  it('shows dead-link empty state for unknown masterId', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_missing',
      subId: 'task_missing_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-split-dead-link')).not.toBeNull();
    });
    dispose();
  });

  it('shows dead-link empty state for unknown subId on valid master', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_missing',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-split-dead-link')).not.toBeNull();
    });
    dispose();
  });

  it('shows error empty state when GET fails', async () => {
    getJsonMock.mockRejectedValue(new Error('Failed to fetch'));
    const { dispose } = mountPlanTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-split-error')).not.toBeNull();
    });
    dispose();
  });

  it('updates selection when master item is clicked', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountPlanTaskSplit(container);
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.plan-task-master-item')).toHaveLength(3);
    });
    container.querySelector('[data-master-id="task_migrated_err"]').click();
    await vi.waitFor(() => {
      expect(
        container.querySelector('.plan-task-master-item--selected')?.dataset.masterId,
      ).toBe('task_migrated_err');
      expect(
        container.querySelector('.plan-task-sub-title-input')?.value,
      ).toBe('Still visible sub');
    });
    dispose();
  });

  it('copies master task id from plan-md header next to edit', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-action="copy-master-id"]')).not.toBeNull();
    });
    const actions = container.querySelector('.plan-task-plan-md-header-actions');
    const copyBtn = actions?.querySelector('[data-action="copy-master-id"]');
    const editBtn = actions?.querySelector('[data-action="edit-plan-md"]');
    expect(copyBtn?.dataset.copyText).toBe('task_alpha');
    expect(editBtn).not.toBeNull();
    expect(container.querySelector('.plan-task-detail-toolbar [data-action="copy-master-id"]')).toBeNull();
    copyBtn.click();
    expect(writeText).toHaveBeenCalledWith('task_alpha');
    await vi.waitFor(() => {
      expect(copyBtn.textContent).toBe('✓ Copied');
      expect(copyBtn.classList.contains('plan-task-copy-flash')).toBe(true);
    });
    dispose();
  });

  it('shows normal empty sub list state with add entry', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_empty',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-empty--detail')).not.toBeNull();
      expect(container.textContent).toContain('No sub-tasks yet');
      expect(container.querySelector('[data-action="add-sub"]')).not.toBeNull();
      expect(container.querySelector('.plan-task-split-state--error')).toBeNull();
    });
    dispose();
  });

  it('shows non-blocking migration_error banner while keeping plan operable', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_migrated_err',
      subId: 'task_migrated_err_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-migration-warning')).not.toBeNull();
      expect(container.textContent).toMatch(/migration|data/i);
      expect(container.querySelector('.plan-task-sub-list .plan-task-sub')).not.toBeNull();
      expect(container.querySelector('[data-action="add-sub"]')).not.toBeNull();
    });
    dispose();
  });

  it('calls navigate with updated hash when sub is clicked', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const navigate = vi.fn();
    const { dispose } = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
      navigate,
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-sub-id="task_alpha_sub_02"]')).not.toBeNull();
    });
    container.querySelector('[data-sub-id="task_alpha_sub_02"]').click();
    expect(navigate).toHaveBeenCalledWith(
      '#/plan-tasks?master=task_alpha&sub=task_alpha_sub_02',
    );
    dispose();
  });

  it('renders detail title as editable input and saves on blur', async () => {
    const invokeMock = vi.fn(async (cmd, args) => {
      if (cmd === 'update_plan_master_title') {
        return {
          task: { ...sampleMasters[0], title: args.title },
          _status: 200,
        };
      }
      if (cmd === 'list_plan_attachments') return [];
      return {};
    });
    window.__TAURI__ = { core: { invoke: invokeMock } };
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountPlanTaskSplit(container, {
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
      expect(invokeMock).toHaveBeenCalledWith('update_plan_master_title', {
        masterTaskId: 'task_alpha',
        title: 'Alpha Renamed',
      });
    });
    dispose();
    delete window.__TAURI__;
  });
});

describe('plan-tasks route source wiring', () => {
  it('index.html includes #plan-tasks-view shell', () => {
    expect(indexHtml).toMatch(/id="plan-tasks-view"/);
  });

  it('index.html includes plan-task dialog shell', () => {
    expect(indexHtml).toMatch(/id="plan-task-dialog"/);
    expect(indexHtml).toMatch(/id="plan-task-dialog-primary"/);
  });

  it('main.js defines mountPlanTasksRoute', () => {
    expect(mainJs).toMatch(/function mountPlanTasksRoute/);
  });

  it('main.js registers plan-tasks via wrapRouteMount in initRouter', () => {
    expect(mainJs).toMatch(
      /['"]plan-tasks['"]:\s*wrapRouteMount\s*\(\s*['"]plan-tasks['"]\s*,\s*mountPlanTasksRoute/,
    );
  });

  it('mountPlanTasksRoute passes master/sub params to split mount', () => {
    const body = extractFunctionBody(mainJs, 'mountPlanTasksRoute');
    expect(body).toMatch(/mountPlanTaskSplit/);
    expect(body).toMatch(/master/);
    expect(body).toMatch(/sub/);
  });

  it('app.css defines full-screen split layout classes', () => {
    expect(appCss).toMatch(/\.plan-task-split/);
    expect(appCss).toMatch(/\.plan-task-split-master/);
    expect(appCss).toMatch(/\.plan-task-split-detail/);
  });

  it('app.css styles plan-md preview and three sub status variants', () => {
    expect(appCss).toMatch(/\.plan-task-plan-md-preview/);
    expect(appCss).toMatch(/\.plan-task-sub-status-select--abandoned/);
    expect(appCss).not.toMatch(/\.plan-task-plan-md-preview[\s\S]*background:\s*#000/);
  });

  it('app.css keeps plan-md preview inside bordered box under flex layout', () => {
    expect(appCss).toMatch(/#plan-tasks-view\s*\{[^}]*min-width:\s*0/s);
    expect(appCss).toMatch(/\.plan-tasks-page\s*\{[^}]*min-width:\s*0/s);
    expect(appCss).toMatch(/\.plan-task-split\s*\{[^}]*min-width:\s*0/s);
    expect(appCss).toMatch(
      /\.plan-task-split-detail\s*\{[^}]*min-width:\s*0/s,
    );
    expect(appCss).toMatch(
      /\.plan-task-plan-md-preview\s*\{[^}]*width:\s*100%/s,
    );
    expect(appCss).toMatch(
      /\.plan-task-plan-md-preview\s*\{[^}]*overflow-wrap:\s*anywhere/s,
    );
    expect(appCss).toMatch(
      /\.plan-task-plan-md-preview ul,\s*\n\s*\.plan-task-plan-md-preview ol/,
    );
  });
});
