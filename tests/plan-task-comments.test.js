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
  addPlanComment,
  deletePlanComment,
  listPlanComments,
  mountPlanTaskSplit,
  updatePlanComment,
} from '../frontend/js/plan-task/index.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

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

const sampleComments = [
  {
    id: 'cmt_aaa111',
    body: 'Chose approach A over B',
    created_at: '2026-07-18T10:00:00Z',
  },
  {
    id: 'cmt_bbb222',
    body: 'Follow-up: verified edge case',
    created_at: '2026-07-18T11:00:00Z',
  },
];

function setupTauri() {
  window.__TAURI__ = {
    core: { invoke: invokeMock },
  };
}

describe('listPlanComments / addPlanComment / updatePlanComment / deletePlanComment', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    setupTauri();
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('listPlanComments invokes list_todo_comments with masterTaskId', async () => {
    invokeMock.mockResolvedValue({ comments: sampleComments, _status: 200 });
    const result = await listPlanComments({ masterTaskId: 'task_alpha' });
    expect(invokeMock).toHaveBeenCalledWith('list_todo_comments', {
      masterTaskId: 'task_alpha',
    });
    expect(result).toEqual(sampleComments);
  });

  it('addPlanComment invokes add_todo_comment with masterTaskId and body', async () => {
    invokeMock.mockResolvedValue({
      id: 'cmt_new',
      body: 'process note',
      created_at: '2026-07-18T12:00:00Z',
      _status: 201,
    });
    const result = await addPlanComment({
      masterTaskId: 'task_alpha',
      body: 'process note',
    });
    expect(invokeMock).toHaveBeenCalledWith('add_todo_comment', {
      masterTaskId: 'task_alpha',
      body: 'process note',
    });
    expect(result.id).toBe('cmt_new');
    expect(result.body).toBe('process note');
  });

  it('updatePlanComment invokes update_todo_comment with masterTaskId, commentId, body', async () => {
    invokeMock.mockResolvedValue({
      id: 'cmt_aaa111',
      body: 'revised note',
      created_at: '2026-07-18T10:00:00Z',
      _status: 200,
    });
    const result = await updatePlanComment({
      masterTaskId: 'task_alpha',
      commentId: 'cmt_aaa111',
      body: 'revised note',
    });
    expect(invokeMock).toHaveBeenCalledWith('update_todo_comment', {
      masterTaskId: 'task_alpha',
      commentId: 'cmt_aaa111',
      body: 'revised note',
    });
    expect(result.body).toBe('revised note');
  });

  it('deletePlanComment invokes delete_todo_comment with masterTaskId and commentId', async () => {
    invokeMock.mockResolvedValue({ ok: true, _status: 200 });
    await deletePlanComment({
      masterTaskId: 'task_alpha',
      commentId: 'cmt_aaa111',
    });
    expect(invokeMock).toHaveBeenCalledWith('delete_todo_comment', {
      masterTaskId: 'task_alpha',
      commentId: 'cmt_aaa111',
    });
  });

  it('throws with status when add returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Comment body must not be empty', _status: 400 });
    await expect(
      addPlanComment({ masterTaskId: 'task_alpha', body: '   ' }),
    ).rejects.toMatchObject({ status: 400 });
  });
});

describe('mountPlanTaskSplit process notes section', () => {
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
      expect(container.querySelector('.plan-task-comments-section')).not.toBeNull();
    });
    return api;
  }

  function mockList(comments = [], attachments = []) {
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_comments') {
        return { comments, _status: 200 };
      }
      if (cmd === 'list_todo_attachments') {
        return { attachments, _status: 200 };
      }
      return {};
    });
  }

  it('shows Process notes section alongside attachments when master detail opens', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    mockList(sampleComments);
    const { dispose } = await mountAndWait();
    expect(invokeMock).toHaveBeenCalledWith('list_todo_comments', {
      masterTaskId: 'task_alpha',
    });
    const commentsSection = container.querySelector('.plan-task-comments-section');
    const attachmentsSection = container.querySelector('.plan-task-attachments-section');
    expect(commentsSection).not.toBeNull();
    expect(attachmentsSection).not.toBeNull();
    expect(commentsSection.textContent).toMatch(/Process notes/i);
    expect(commentsSection.hidden).toBe(false);
    expect(commentsSection.getAttribute('hidden')).toBeNull();
    dispose();
  });

  it('lists comments in created_at order when populated', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    mockList(sampleComments);
    const { dispose } = await mountAndWait();
    const items = [...container.querySelectorAll('.plan-task-comment-item')];
    expect(items).toHaveLength(2);
    expect(items[0].textContent).toContain('Chose approach A over B');
    expect(items[1].textContent).toContain('Follow-up: verified edge case');
    dispose();
  });

  it('shows understandable empty state when there are no comments (section stays visible)', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    mockList([]);
    const { dispose } = await mountAndWait();
    const section = container.querySelector('.plan-task-comments-section');
    expect(section).not.toBeNull();
    const empty = container.querySelector('.plan-task-comments-empty');
    expect(empty).not.toBeNull();
    expect(empty.textContent.trim().length).toBeGreaterThan(0);
    expect(container.querySelectorAll('.plan-task-comment-item')).toHaveLength(0);
    dispose();
  });

  it('shows LoadError in section when list_todo_comments fails (not fake empty success)', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_comments') {
        return { error: 'comments.json is invalid', _status: 500 };
      }
      if (cmd === 'list_todo_attachments') {
        return { attachments: [], _status: 200 };
      }
      return {};
    });
    const { dispose } = await mountAndWait();
    const section = container.querySelector('.plan-task-comments-section');
    expect(section).not.toBeNull();
    const err = container.querySelector('.plan-task-comments-error');
    expect(err).not.toBeNull();
    expect(err.textContent.trim().length).toBeGreaterThan(0);
    expect(container.querySelector('.plan-task-comments-empty')).toBeNull();
    dispose();
  });

  it('add flow invokes add_todo_comment then refreshes list with new item', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    let listed = [];
    invokeMock.mockImplementation(async (cmd, args) => {
      if (cmd === 'list_todo_comments') {
        return { comments: listed, _status: 200 };
      }
      if (cmd === 'list_todo_attachments') {
        return { attachments: [], _status: 200 };
      }
      if (cmd === 'add_todo_comment') {
        listed = [
          {
            id: 'cmt_new001',
            body: args.body,
            created_at: '2026-07-18T12:00:00Z',
          },
        ];
        return { ...listed[0], _status: 201 };
      }
      return {};
    });

    const { dispose } = await mountAndWait();
    expect(container.querySelector('.plan-task-comments-empty')).not.toBeNull();

    const input = container.querySelector('[data-comment-input]');
    expect(input).not.toBeNull();
    input.value = 'Why we picked A';
    container.querySelector('[data-action="add-comment"]').click();

    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('add_todo_comment', {
        masterTaskId: 'task_alpha',
        body: 'Why we picked A',
      });
    });
    await vi.waitFor(() => {
      expect(container.textContent).toContain('Why we picked A');
    });
    expect(container.querySelector('.plan-task-comments-empty')).toBeNull();
    dispose();
  });

  it('update flow invokes update_todo_comment then refreshes with new body', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    let listed = [...sampleComments];
    invokeMock.mockImplementation(async (cmd, args) => {
      if (cmd === 'list_todo_comments') {
        return { comments: listed, _status: 200 };
      }
      if (cmd === 'list_todo_attachments') {
        return { attachments: [], _status: 200 };
      }
      if (cmd === 'update_todo_comment') {
        listed = listed.map((c) =>
          c.id === args.commentId ? { ...c, body: args.body } : c,
        );
        return { ...listed.find((c) => c.id === args.commentId), _status: 200 };
      }
      return {};
    });

    const { dispose } = await mountAndWait();
    const editBtn = container.querySelector(
      '[data-action="edit-comment"][data-comment-id="cmt_aaa111"]',
    );
    expect(editBtn).not.toBeNull();
    editBtn.click();

    await vi.waitFor(() => {
      expect(container.querySelector('[data-comment-edit-input]')).not.toBeNull();
    });
    const editInput = container.querySelector('[data-comment-edit-input]');
    editInput.value = 'revised approach note';
    container.querySelector('[data-action="save-comment"]').click();

    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('update_todo_comment', {
        masterTaskId: 'task_alpha',
        commentId: 'cmt_aaa111',
        body: 'revised approach note',
      });
    });
    await vi.waitFor(() => {
      expect(container.textContent).toContain('revised approach note');
    });
    expect(container.textContent).not.toContain('Chose approach A over B');
    dispose();
  });

  it('delete flow invokes delete_todo_comment then refreshes without the item', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    let listed = [...sampleComments];
    invokeMock.mockImplementation(async (cmd, args) => {
      if (cmd === 'list_todo_comments') {
        return { comments: listed, _status: 200 };
      }
      if (cmd === 'list_todo_attachments') {
        return { attachments: [], _status: 200 };
      }
      if (cmd === 'delete_todo_comment') {
        listed = listed.filter((c) => c.id !== args.commentId);
        return { ok: true, _status: 200 };
      }
      return {};
    });

    const { dispose } = await mountAndWait();
    container
      .querySelector('[data-action="delete-comment"][data-comment-id="cmt_aaa111"]')
      .click();

    await vi.waitFor(() => {
      expect(
        container.querySelector('[data-action="confirm-delete-comment"]'),
      ).not.toBeNull();
    });
    container.querySelector('[data-action="confirm-delete-comment"]').click();

    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('delete_todo_comment', {
        masterTaskId: 'task_alpha',
        commentId: 'cmt_aaa111',
      });
    });
    await vi.waitFor(() => {
      expect(container.textContent).not.toContain('Chose approach A over B');
    });
    expect(container.textContent).toContain('Follow-up: verified edge case');
    dispose();
  });

  it('shows WriteErrorVisible when add fails (does not pretend success)', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_comments') {
        return { comments: [], _status: 200 };
      }
      if (cmd === 'list_todo_attachments') {
        return { attachments: [], _status: 200 };
      }
      if (cmd === 'add_todo_comment') {
        return { error: 'Comment body must not be empty', _status: 400 };
      }
      return {};
    });

    const { dispose } = await mountAndWait();
    const input = container.querySelector('[data-comment-input]');
    input.value = '   ';
    container.querySelector('[data-action="add-comment"]').click();

    await vi.waitFor(() => {
      const err = container.querySelector('.plan-task-comments-error');
      expect(err).not.toBeNull();
      expect(err.textContent.trim().length).toBeGreaterThan(0);
    });
    expect(container.querySelectorAll('.plan-task-comment-item')).toHaveLength(0);
    dispose();
  });

  it('does not introduce 计划任务 or 过程备注 Chinese copy in comments UI strings', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    mockList([]);
    const { dispose } = await mountAndWait();
    const section = container.querySelector('.plan-task-comments-section');
    expect(section.textContent).not.toContain('计划任务');
    expect(section.textContent).not.toContain('过程备注');
    dispose();
  });
});

describe('comments UI uses Tauri commands only (no MCP write path)', () => {
  it('index.js wires list/add/update/delete_todo_comment and has no MCP comment write', () => {
    const src = readFileSync(join(repoRoot, 'frontend/js/plan-task/index.js'), 'utf8');
    expect(src).toMatch(/list_todo_comments/);
    expect(src).toMatch(/add_todo_comment/);
    expect(src).toMatch(/update_todo_comment/);
    expect(src).toMatch(/delete_todo_comment/);
    expect(src).not.toMatch(/create_todo_comment|mcp.*comment|comment.*mcp/i);
  });
});
