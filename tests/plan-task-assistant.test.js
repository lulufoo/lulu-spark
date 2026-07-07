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
  formatSubProgressSummary,
  loadAssistantPlanTasks,
  mountPlanTaskAssistant,
  mountPlanTaskAssistantWidget,
  selectTop3ByCreatedAt,
} from '../frontend/js/plan-task-assistant.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const assistantHtml = readFileSync(
  join(fixtureRoot, 'frontend/plan-task-assistant.html'),
  'utf8',
);
const assistantCapability = JSON.parse(
  readFileSync(
    join(fixtureRoot, 'src-tauri/capabilities/plan-task-assistant.json'),
    'utf8',
  ),
);
const appCss = readFileSync(join(fixtureRoot, 'frontend/app.css'), 'utf8');

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
  it('plan-task-assistant.html includes assistant root shell', () => {
    expect(assistantHtml).toMatch(/id="plan-task-assistant-root"/);
  });

  it('plan-task-assistant.html loads plan-task-assistant.js module', () => {
    expect(assistantHtml).toMatch(/plan-task-assistant\.js/);
  });

  it('capability grants read-api to plan-task-assistant window', () => {
    expect(assistantCapability.windows).toContain('plan-task-assistant');
    expect(assistantCapability.permissions).toContain('read-api');
  });

  it('Plan FAB widget is offset 56px above Read Later FAB', () => {
    expect(appCss).toMatch(/\.pt-assistant-widget[\s\S]*bottom:\s*76px/);
  });
});

describe('loadAssistantPlanTasks', () => {
  beforeEach(() => {
    getJsonMock.mockReset();
  });

  it('GET /api/plan-tasks via apiClient and returns master array', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const entries = await loadAssistantPlanTasks();
    expect(getJsonMock).toHaveBeenCalledWith('/api/plan-tasks');
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
      'Newest Task · 1/2 完成',
    );
  });

  it('handles single implicit sub', () => {
    expect(formatSubProgressSummary(sampleMasters[2])).toBe(
      'Mid Task · 0/1 完成',
    );
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
      'Newest Task · 1/2 完成',
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
});

describe('mountPlanTaskAssistantWidget', () => {
  let anchor;

  beforeEach(() => {
    anchor = document.createElement('div');
    document.body.appendChild(anchor);
    getJsonMock.mockReset();
  });

  afterEach(() => {
    anchor.remove();
    document.querySelectorAll('.pt-assistant-widget').forEach((el) => el.remove());
  });

  it('renders fixed launcher with popover hidden by default', () => {
    mountPlanTaskAssistantWidget(anchor);
    expect(document.querySelector('.pt-assistant-fab')).not.toBeNull();
    expect(document.querySelector('.pt-assistant-popover')?.hidden).toBe(true);
  });

  it('opens popover and loads top3 on first open', async () => {
    getJsonMock.mockResolvedValue(sampleMasters);
    const { setOpen } = mountPlanTaskAssistantWidget(anchor);
    setOpen(true);
    await vi.waitFor(() => {
      expect(document.querySelectorAll('.plan-task-assistant-item')).toHaveLength(3);
    });
  });
});
