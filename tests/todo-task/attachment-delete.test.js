// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const invokeMock = vi.fn();
const getJsonMock = vi.fn();

vi.mock('../../frontend/js/host/apiClient.js', async (importOriginal) => {
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
  deletePlanAttachment,
  mountTodoTaskSplit,
} from '../../frontend/js/todo-task/index.js';

import { readRsPath } from '../helpers/read-rs-dir.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const sampleMaster = {
  master_task_id: 'task_alpha',
  title: 'Alpha Task',
  status: 'incomplete',
  created_at: '2026-07-01T10:00:00Z',
  todo_md: '# Plan',
  sub_tasks: [
    {
      sub_task_id: 'task_alpha_sub_01',
      title: 'Alpha Sub A',
      status: 'incomplete',
      implicit: false,
      linked_archive_ids: [],
    },
  ],
};

const sampleAttachments = [
  {
    file_name: 'notes.md',
    original_file_name: 'notes.md',
    added_at: '2026-07-18T10:00:00Z',
  },
  {
    file_name: 'spec.md',
    original_file_name: 'spec.md',
    added_at: '2026-07-18T11:00:00Z',
  },
];

function setupTauri() {
  window.__TAURI__ = {
    core: { invoke: invokeMock },
  };
}

describe('deletePlanAttachment', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    setupTauri();
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('invokes delete_todo_attachment with masterTaskId and fileName', async () => {
    invokeMock.mockResolvedValue({ ok: true, _status: 200 });
    const result = await deletePlanAttachment({
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
    });
    expect(invokeMock).toHaveBeenCalledWith('delete_todo_attachment', {
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
    });
    expect(result).toMatchObject({ ok: true });
  });

  it('throws with status when delete returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Attachment not found', _status: 404 });
    await expect(
      deletePlanAttachment({ masterTaskId: 'task_alpha', fileName: 'missing.md' }),
    ).rejects.toMatchObject({ status: 404 });
  });
});

describe('mountTodoTaskSplit attachment delete entry', () => {
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
      expect(container.querySelector('.todo-task-attachment-item')).not.toBeNull();
    });
    return api;
  }

  it('lists a per-attachment delete entry distinct from open-attachment', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: sampleAttachments, _status: 200 };
      }
      return {};
    });
    const { dispose } = await mountAndWait();
    const deleteBtns = container.querySelectorAll(
      '[data-action="delete-attachment"][data-file-name]',
    );
    expect(deleteBtns.length).toBe(sampleAttachments.length);
    expect(
      container.querySelector('[data-action="delete-attachment"][data-file-name="notes.md"]'),
    ).not.toBeNull();
    expect(
      container.querySelector('[data-action="delete-attachment"][data-file-name="spec.md"]'),
    ).not.toBeNull();
    dispose();
  });

  it('delete confirm is attachment-scoped and not the plan delete dialog types', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: sampleAttachments, _status: 200 };
      }
      return {};
    });
    const { dispose } = await mountAndWait();
    container
      .querySelector('[data-action="delete-attachment"][data-file-name="notes.md"]')
      .click();

    await vi.waitFor(() => {
      const confirm = container.querySelector(
        '[data-action="confirm-delete-attachment"], .todo-task-attachment-delete-confirm',
      );
      expect(confirm).not.toBeNull();
    });

    const confirmUi = container.querySelector(
      '.todo-task-attachment-delete-confirm, [data-attachment-delete-confirm]',
    );
    expect(confirmUi).not.toBeNull();
    expect(confirmUi.textContent).toMatch(/attachment|notes\.md/i);
    expect(confirmUi.textContent).not.toMatch(/删除计划|子任务/);
    expect(document.getElementById('todo-task-dialog')?.classList.contains('open')).not.toBe(
      true,
    );
    dispose();
  });

  it('confirming delete invokes delete_todo_attachment then removes item from list', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    let listed = [...sampleAttachments];
    invokeMock.mockImplementation(async (cmd, args) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: listed, _status: 200 };
      }
      if (cmd === 'delete_todo_attachment') {
        listed = listed.filter((entry) => entry.file_name !== args.fileName);
        return { ok: true, _status: 200 };
      }
      return {};
    });

    const { dispose } = await mountAndWait();
    expect(container.textContent).toContain('notes.md');
    expect(container.textContent).toContain('spec.md');

    container
      .querySelector('[data-action="delete-attachment"][data-file-name="notes.md"]')
      .click();
    await vi.waitFor(() => {
      expect(
        container.querySelector(
          '[data-action="confirm-delete-attachment"], .todo-task-attachment-delete-confirm',
        ),
      ).not.toBeNull();
    });

    const confirmBtn =
      container.querySelector('[data-action="confirm-delete-attachment"]') ||
      container.querySelector(
        '.todo-task-attachment-delete-confirm [data-action="confirm-delete-attachment"]',
      );
    expect(confirmBtn).not.toBeNull();
    confirmBtn.click();

    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('delete_todo_attachment', {
        masterTaskId: 'task_alpha',
        fileName: 'notes.md',
      });
    });
    await vi.waitFor(() => {
      expect(container.textContent).not.toContain('notes.md');
    });
    expect(container.textContent).toContain('spec.md');
    expect(
      container.querySelector('[data-action="delete-attachment"][data-file-name="notes.md"]'),
    ).toBeNull();
    dispose();
  });

  it('cancelling attachment delete does not invoke delete_todo_attachment', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: sampleAttachments, _status: 200 };
      }
      return {};
    });

    const { dispose } = await mountAndWait();
    container
      .querySelector('[data-action="delete-attachment"][data-file-name="notes.md"]')
      .click();
    await vi.waitFor(() => {
      expect(
        container.querySelector(
          '[data-action="cancel-delete-attachment"], .todo-task-attachment-delete-confirm',
        ),
      ).not.toBeNull();
    });

    const cancelBtn =
      container.querySelector('[data-action="cancel-delete-attachment"]') ||
      container.querySelector(
        '.todo-task-attachment-delete-confirm [data-action="cancel-delete-attachment"]',
      );
    expect(cancelBtn).not.toBeNull();
    cancelBtn.click();

    await vi.waitFor(() => {
      expect(
        container.querySelector('.todo-task-attachment-delete-confirm'),
      ).toBeNull();
    });
    expect(invokeMock).not.toHaveBeenCalledWith(
      'delete_todo_attachment',
      expect.anything(),
    );
    expect(container.textContent).toContain('notes.md');
    dispose();
  });

  it('failed delete shows error and keeps the item listed (no half-success UI)', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: sampleAttachments, _status: 200 };
      }
      if (cmd === 'delete_todo_attachment') {
        return { error: 'delete failed', _status: 500 };
      }
      return {};
    });

    const { dispose } = await mountAndWait();
    container
      .querySelector('[data-action="delete-attachment"][data-file-name="notes.md"]')
      .click();
    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="confirm-delete-attachment"]'),
      ).not.toBeNull();
    });
    container.querySelector('[data-action="confirm-delete-attachment"]').click();

    await vi.waitFor(() => {
      const err = container.querySelector('.todo-task-attachments-error');
      expect(err).not.toBeNull();
      expect(err.textContent.trim().length).toBeGreaterThan(0);
    });
    expect(container.textContent).toContain('notes.md');
    expect(
      container.querySelector('[data-action="delete-attachment"][data-file-name="notes.md"]'),
    ).not.toBeNull();
    dispose();
  });
});

describe('attachment delete does not loosen MCP / dialog contracts', () => {
  it('dialog.js TodoTaskDialogType stays master/sub CRUD only (no attachment delete type)', () => {
    const dialogSrc = readFileSync(
      join(repoRoot, 'frontend/js/todo-task/dialog.js'),
      'utf8',
    );
    expect(dialogSrc).toMatch(
      /@typedef \{'create-master' \| 'create-category' \| 'add-sub' \| 'delete-master' \| 'delete-sub'\} TodoTaskDialogType/,
    );
    expect(dialogSrc).not.toMatch(/delete-attachment|attachment-delete/);
  });

  it('index.js attachment delete confirm path does not openTodoTaskDialog', () => {
    const src = readFileSync(join(repoRoot, 'frontend/js/todo-task/attachments.js'), 'utf8')
      + readFileSync(join(repoRoot, 'frontend/js/todo-task/attachments-render.js'), 'utf8');
    expect(src).toMatch(/data-action="delete-attachment"/);
    const marker = "action === 'delete-attachment'";
    const idx = src.indexOf(marker);
    expect(idx).toBeGreaterThan(-1);
    const actionBlock = src.slice(idx, idx + 500);
    expect(actionBlock).not.toContain('openTodoTaskDialog');
  });

  it('MCP schema still has no attachment delete tool', () => {
    const mcpSrc = readRsPath(join(repoRoot, 'src-tauri/src/services/mcp_protocol_adapter'));
    expect(mcpSrc).toMatch(/add_todo_attachment/);
    expect(mcpSrc).toMatch(/list_todo_attachments/);
    expect(mcpSrc).toMatch(/get_todo_attachment/);
    expect(mcpSrc).toMatch(/update_todo_attachment/);
    expect(mcpSrc).not.toMatch(/['"]delete_plan_attachment['"]/);
    expect(mcpSrc).not.toMatch(/['"]remove_plan_attachment['"]/);
    expect(mcpSrc).not.toMatch(/['"]delete_todo_attachment['"]/);
    expect(mcpSrc).not.toMatch(/['"]remove_todo_attachment['"]/);

    const e2eSrc = readFileSync(join(repoRoot, 'scripts/todo-task-mcp-e2e.mjs'), 'utf8');
    expect(e2eSrc).toMatch(/FORBIDDEN_PLAN_TOOLS|FORBIDDEN_PLAN_/);
    expect(e2eSrc).not.toMatch(/name:\s*['"]delete_todo_attachment['"]/);
    expect(e2eSrc).not.toMatch(/name:\s*['"]delete_plan_attachment['"]/);
  });
});
