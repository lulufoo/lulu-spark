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

import {
  buildDeepLink,
  createPlanTaskContentAdapter,
  formatSubProgressSummary,
  loadAssistantPlanTasks,
  mountPlanTaskAssistant,
  selectTop3ByCreatedAt,
} from '../frontend/js/plan-task-assistant.js';import { formatPlanTaskStatus } from '../frontend/js/plan-task/index.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const appCss = readFileSync(join(fixtureRoot, 'frontend/app.css'), 'utf8');
const mainJs = readFileSync(join(fixtureRoot, 'frontend/js/main.js'), 'utf8');
const assistantJs = readFileSync(
  join(fixtureRoot, 'frontend/js/plan-task-assistant.js'),
  'utf8',
);

const sampleMasters = [
  {
    master_task_id: 'task_oldest',
    title: 'Oldest Task',
    status: 'incomplete',
    created_at: '2026-07-01T10:00:00Z',
    sub_tasks: [
      {
        sub_task_id: 'task_oldest_sub_01',
        title: 'Sub A',
        status: 'incomplete',
        implicit: false,
        linked_archive_ids: [],
      },
    ],
  },
  {
    master_task_id: 'task_newest',
    title: 'Newest Task',
    status: 'incomplete',
    created_at: '2026-07-06T10:00:00Z',
    sub_tasks: [
      {
        sub_task_id: 'task_newest_sub_01',
        title: 'Sub 1',
        status: 'complete',
        implicit: false,
        linked_archive_ids: [],
      },
      {
        sub_task_id: 'task_newest_sub_02',
        title: 'Sub 2',
        status: 'incomplete',
        implicit: false,
        linked_archive_ids: [],
      },
    ],
  },
  {
    master_task_id: 'task_mid',
    title: 'Mid Task',
    status: 'incomplete',
    created_at: '2026-07-03T10:00:00Z',
    sub_tasks: [
      {
        sub_task_id: 'task_mid_sub_01',
        title: 'Only sub',
        status: 'incomplete',
        implicit: true,
        linked_archive_ids: [],
      },
    ],
  },
  {
    master_task_id: 'task_extra4',
    title: 'Extra Task 4',
    status: 'complete',
    created_at: '2026-07-04T10:00:00Z',
    sub_tasks: [
      {
        sub_task_id: 'task_extra4_sub_01',
        title: 'Done',
        status: 'complete',
        implicit: false,
        linked_archive_ids: [],
      },
    ],
  },
];

describe('plan-task-assistant source wiring', () => {
  it('independent Plan FAB stack offset is retired; shell cluster owns bottom anchor', () => {
    expect(appCss).not.toMatch(/\.pt-assistant-widget\s*\{[^}]*bottom:\s*76px/);
    expect(appCss).toMatch(/\.home-entry-shell__cluster\s*\{[^}]*bottom:\s*20px/);
  });

  it('keeps key-only Binding ownership out of the assistant list widget', () => {
    expect(assistantJs).not.toMatch(/\bset_binding\b|\breset_binding\b/);
    expect(assistantJs).not.toMatch(/\btools\s*:|\bprompt\s*:|\bcallbacks\s*:/);
    expect(assistantJs).not.toMatch(/\bengine(?:Type|_type)?\b/);
  });
});

describe('loadAssistantPlanTasks', () => {
  beforeEach(() => {
    getJsonMock.mockReset();
  });

  it('GET /api/todo-tasks via apiClient and returns master array', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const entries = await loadAssistantPlanTasks();
    expect(getJsonMock).toHaveBeenCalledWith('/api/todo-tasks');
    expect(entries).toEqual(sampleMasters);
  });

  it('throws with status when service returns unavailable payload', async () => {
    getJsonMock.mockResolvedValue({
      error: 'Workbench not running',
      _status: 503,
    });
    await expect(loadAssistantPlanTasks()).rejects.toMatchObject({
      status: 503,
    });
  });
});

describe('selectTop3ByCreatedAt', () => {
  it('sorts created_at desc and slices to 3', () => {
    const top3 = selectTop3ByCreatedAt(sampleMasters);
    expect(top3).toHaveLength(3);
    expect(top3.map((m) => m.master_task_id)).toEqual([
      'task_newest',
      'task_extra4',
      'task_mid',
    ]);
  });

  it('returns fewer than 3 when total count is below 3 without placeholders', () => {
    const entries = [sampleMasters[1], sampleMasters[2]];
    const top3 = selectTop3ByCreatedAt(entries);
    expect(top3).toHaveLength(2);
    expect(top3.map((m) => m.master_task_id)).toEqual(['task_newest', 'task_mid']);
  });

  it('returns empty array when no entries', () => {
    expect(selectTop3ByCreatedAt([])).toEqual([]);
  });
});

describe('formatSubProgressSummary', () => {
  it('combines title with complete/total sub progress', () => {
    expect(formatSubProgressSummary(sampleMasters[1])).toBe(
      'Newest Task · 1/2 complete',
    );
  });

  it('handles single implicit sub', () => {
    expect(formatSubProgressSummary(sampleMasters[2])).toBe(
      'Mid Task · 0/1 complete',
    );
  });

  it('keeps title string unchanged for abandoned masters (no status baked into title)', () => {
    const abandoned = {
      master_task_id: 'task_abandoned',
      title: 'Abandoned Master',
      status: 'abandoned',
      created_at: '2026-07-05T10:00:00Z',
      sub_tasks: [
        {
          sub_task_id: 'task_abandoned_sub_01',
          title: 'Left behind',
          status: 'incomplete',
          implicit: false,
          linked_archive_ids: [],
        },
      ],
    };
    expect(formatSubProgressSummary(abandoned)).toBe(
      'Abandoned Master · 0/1 complete',
    );
    expect(formatSubProgressSummary(abandoned)).not.toContain('Abandoned ·');
  });
});

describe('buildDeepLink', () => {
  it('builds split panel hash with master and sub query params', () => {
    expect(buildDeepLink('task_newest', 'task_newest_sub_02')).toBe(
      '#/plan-tasks?master=task_newest&sub=task_newest_sub_02',
    );
  });
});

describe('mountPlanTaskAssistant', () => {
  let root;

  beforeEach(() => {
    root = document.createElement('div');
    root.id = 'plan-task-assistant-root';
    document.body.appendChild(root);
    getJsonMock.mockReset();
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    root.remove();
    document.querySelectorAll('.pt-assistant-widget').forEach((el) => el.remove());
  });

  it('loads tasks on mount and displays top3 in created_at desc order', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountPlanTaskAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelectorAll('.plan-task-assistant-item')).toHaveLength(3);
    });
    const items = root.querySelectorAll('.plan-task-assistant-item');
    expect(items[0].dataset.masterId).toBe('task_newest');
    expect(items[1].dataset.masterId).toBe('task_extra4');
    expect(items[2].dataset.masterId).toBe('task_mid');
    expect(items[0].querySelector('.plan-task-assistant-item-summary')?.textContent).toBe(
      'Newest Task · 1/2 complete',
    );
    dispose();
  });

  it('renders deep-link href with master and sub query on each item', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { dispose } = mountPlanTaskAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.plan-task-assistant-item-link')).not.toBeNull();
    });
    const link = root.querySelector('.plan-task-assistant-item-link');
    expect(link.getAttribute('href')).toBe(
      '#/plan-tasks?master=task_newest&sub=task_newest_sub_02',
    );
    dispose();
  });

  it('shows empty state when no tasks without placeholders', async () => {
    getJsonMock.mockResolvedValue([]);
    const { dispose } = mountPlanTaskAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.plan-task-assistant-empty')).not.toBeNull();
    });
    expect(root.querySelector('.plan-task-assistant-item')).toBeNull();
    expect(root.querySelector('.plan-task-assistant-unavailable')).toBeNull();
    dispose();
  });

  it('shows 1–2 tasks without padding to 3', async () => {
    getJsonMock.mockResolvedValue(sampleMasters.slice(0, 2));
    const { dispose } = mountPlanTaskAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelectorAll('.plan-task-assistant-item')).toHaveLength(2);
    });
    dispose();
  });

  it('shows error empty state on GET failure without silent blank', async () => {
    getJsonMock.mockRejectedValue(new Error('Failed to fetch'));
    const { dispose } = mountPlanTaskAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelector('.plan-task-assistant-state--error')).not.toBeNull();
    });
    expect(root.querySelector('.plan-task-assistant-item')).toBeNull();
    dispose();
  });

  it('navigates to #/plan-tasks when manage link is clicked', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const navigate = vi.fn();
    const { dispose } = mountPlanTaskAssistant(root, { navigate });
    await vi.waitFor(() => {
      expect(root.querySelector('.plan-task-assistant-manage-link')).not.toBeNull();
    });
    root.querySelector('.plan-task-assistant-manage-link').click();
    expect(navigate).toHaveBeenCalledWith('#/plan-tasks');
    dispose();
  });

  it('navigates via deep-link when item is clicked', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const navigate = vi.fn();
    const { dispose } = mountPlanTaskAssistant(root, { navigate });
    await vi.waitFor(() => {
      expect(root.querySelector('.plan-task-assistant-item-link')).not.toBeNull();
    });
    root.querySelector('.plan-task-assistant-item-link').click();
    expect(navigate).toHaveBeenCalledWith(
      '#/plan-tasks?master=task_newest&sub=task_newest_sub_02',
    );
    dispose();
  });

  it('shows master status marks with the same English labels as the list', async () => {
    const masters = [
      {
        master_task_id: 'task_in_progress',
        title: 'Active Master',
        status: 'incomplete',
        created_at: '2026-07-08T10:00:00Z',
        sub_tasks: [
          {
            sub_task_id: 'task_in_progress_sub_01',
            title: 'Sub',
            status: 'incomplete',
            implicit: false,
            linked_archive_ids: [],
          },
        ],
      },
      {
        master_task_id: 'task_done',
        title: 'Done Master',
        status: 'complete',
        created_at: '2026-07-07T10:00:00Z',
        sub_tasks: [
          {
            sub_task_id: 'task_done_sub_01',
            title: 'Done',
            status: 'complete',
            implicit: false,
            linked_archive_ids: [],
          },
        ],
      },
      {
        master_task_id: 'task_abandoned',
        title: 'Abandoned Master',
        status: 'abandoned',
        created_at: '2026-07-06T10:00:00Z',
        sub_tasks: [
          {
            sub_task_id: 'task_abandoned_sub_01',
            title: 'Left',
            status: 'incomplete',
            implicit: false,
            linked_archive_ids: [],
          },
        ],
      },
    ];
    getJsonMock.mockResolvedValue(masters);
    const { dispose } = mountPlanTaskAssistant(root);
    await vi.waitFor(() => {
      expect(root.querySelectorAll('.plan-task-assistant-item')).toHaveLength(3);
    });

    const byId = (id) => root.querySelector(`[data-master-id="${id}"]`);
    const labelOf = (id) =>
      byId(id)?.querySelector('.plan-task-assistant-item-status')?.textContent;

    expect(labelOf('task_in_progress')).toBe(formatPlanTaskStatus('incomplete'));
    expect(labelOf('task_done')).toBe(formatPlanTaskStatus('complete'));
    expect(labelOf('task_abandoned')).toBe(formatPlanTaskStatus('abandoned'));
    expect(labelOf('task_in_progress')).toBe('In progress');
    expect(labelOf('task_done')).toBe('Completed');
    expect(labelOf('task_abandoned')).toBe('Abandoned');

    expect(byId('task_in_progress')?.className).toMatch(
      /plan-task-assistant-item--incomplete/,
    );
    expect(byId('task_done')?.className).toMatch(
      /plan-task-assistant-item--complete/,
    );
    expect(byId('task_abandoned')?.className).toMatch(
      /plan-task-assistant-item--abandoned/,
    );

    const abandonedSummary = byId('task_abandoned')?.querySelector(
      '.plan-task-assistant-item-summary',
    )?.textContent;
    expect(abandonedSummary).toContain('Abandoned Master');
    expect(abandonedSummary).toBe('Abandoned Master · 0/1 complete');
    dispose();
  });
});

describe('assistant status mark source alignment', () => {
  it('reuses list formatPlanTaskStatus instead of a local label table', () => {
    expect(assistantJs).toMatch(
      /import\s*\{\s*formatPlanTaskStatus\s*\}\s*from\s*'\.\/plan-task\/index\.js'/,
    );
    expect(assistantJs).not.toMatch(/STATUS_LABELS\s*=/);
  });

  it('app.css styles assistant complete muted and abandoned strike/gray', () => {
    expect(appCss).toMatch(/\.plan-task-assistant-item--complete/);
    expect(appCss).toMatch(/\.plan-task-assistant-item--abandoned/);
    expect(appCss).toMatch(
      /\.plan-task-assistant-item--abandoned[\s\S]*?text-decoration:\s*line-through/,
    );
  });
});

describe('main window wiring', () => {
  it('main.js orchestrates via home-entry shell (legacy four-FAB mounts retired)', () => {
    expect(mainJs).toMatch(/mountHomeEntryShell\s*\(\s*document\.body\b/);
    expect(mainJs).not.toMatch(/mountPlanTaskAssistantWidget\s*\(\s*document\.body\b/);
    expect(mainJs).not.toMatch(/mountReadLaterAssistantWidget\s*\(\s*document\.body\b/);
    expect(mainJs).toMatch(/createPlanTaskContentAdapter/);
  });

  it('independent FAB bottom stack offsets are retired in app.css', () => {
    expect(appCss).not.toMatch(/\.pt-assistant-widget\s*\{[^}]*bottom:\s*76px/);
    expect(appCss).toMatch(/\.home-entry-shell__cluster\s*\{[^}]*bottom:\s*20px/);
  });
});

describe('createPlanTaskContentAdapter', () => {
  /** @type {HTMLElement} */
  let slot;

  beforeEach(() => {
    slot = document.createElement('div');
    document.body.appendChild(slot);
    getJsonMock.mockReset();
  });

  afterEach(() => {
    slot.remove();
  });

  it('mounts task list into the slot without self-owned chrome', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const handle = createPlanTaskContentAdapter().mount(slot, {
      host: { navigate: () => {} },
    });
    await vi.waitFor(() => {
      expect(slot.querySelectorAll('.plan-task-assistant-item')).toHaveLength(3);
    });
    expect(slot.querySelector('.pt-assistant-fab')).toBeNull();
    expect(slot.querySelector('.pt-assistant-popover')).toBeNull();
    handle.unmount();
    expect(slot.innerHTML).toBe('');
  });
});
