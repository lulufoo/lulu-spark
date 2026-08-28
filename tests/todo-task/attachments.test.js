// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFrontendJs } from '../helpers/read-frontend-js.js';

const invokeMock = vi.fn();
const getJsonMock = vi.fn();
const dialogOpenMock = vi.fn();
const convertFileSrcMock = vi.fn((path) => `asset://localhost/${encodeURIComponent(path)}`);

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
  addPlanAttachment,
  listPlanAttachments,
  mountTodoTaskSplit,
  pickLocalMarkdownFile,
} from '../../frontend/src/todo-task/index.ts';

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
    core: { invoke: invokeMock, convertFileSrc: convertFileSrcMock },
    dialog: { open: dialogOpenMock },
  };
}

describe('listPlanAttachments / addPlanAttachment', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    setupTauri();
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('listPlanAttachments invokes list_todo_attachments with masterTaskId', async () => {
    invokeMock.mockResolvedValue({ attachments: sampleAttachments, _status: 200 });
    const result = await listPlanAttachments({ masterTaskId: 'task_alpha' });
    expect(invokeMock).toHaveBeenCalledWith('list_todo_attachments', {
      masterTaskId: 'task_alpha',
    });
    expect(result).toEqual(sampleAttachments);
  });

  it('addPlanAttachment invokes add_todo_attachment with masterTaskId, sourcePath', async () => {
    invokeMock.mockResolvedValue({
      file_name: 'notes.md',
      original_file_name: 'notes.md',
      added_at: '2026-07-18T10:00:00Z',
      _status: 201,
    });
    const result = await addPlanAttachment({
      masterTaskId: 'task_alpha',
      sourcePath: '/tmp/notes.md',
    });
    expect(invokeMock).toHaveBeenCalledWith('add_todo_attachment', {
      masterTaskId: 'task_alpha',
      sourcePath: '/tmp/notes.md',
    });
    expect(result.file_name).toBe('notes.md');
  });

  it('throws with status when add returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Only .md attachments are supported', _status: 400 });
    await expect(
      addPlanAttachment({ masterTaskId: 'task_alpha', sourcePath: '/tmp/a.txt' }),
    ).rejects.toMatchObject({ status: 400 });
  });
});

describe('pickLocalMarkdownFile', () => {
  beforeEach(() => {
    dialogOpenMock.mockReset();
    convertFileSrcMock.mockClear();
    setupTauri();
    global.fetch = vi.fn();
  });

  afterEach(() => {
    delete window.__TAURI__;
    delete global.fetch;
  });

  it('opens Host dialog filtered to md and returns fileName + content', async () => {
    dialogOpenMock.mockResolvedValue('/tmp/docs/notes.md');
    global.fetch.mockResolvedValue({
      ok: true,
      text: async () => '# From disk',
    });
    const picked = await pickLocalMarkdownFile();
    expect(dialogOpenMock).toHaveBeenCalledWith(
      expect.objectContaining({
        multiple: false,
        filters: [expect.objectContaining({ extensions: ['md'] })],
      }),
    );
    expect(convertFileSrcMock).toHaveBeenCalledWith('/tmp/docs/notes.md');
    expect(picked).toEqual({ fileName: 'notes.md', content: '# From disk' });
  });

  it('returns null when user cancels Host dialog', async () => {
    dialogOpenMock.mockResolvedValue(null);
    const picked = await pickLocalMarkdownFile();
    expect(picked).toBeNull();
    expect(global.fetch).not.toHaveBeenCalled();
  });
});

describe('mountTodoTaskSplit attachment list + add', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    invokeMock.mockReset();
    dialogOpenMock.mockReset();
    convertFileSrcMock.mockClear();
    setupTauri();
    global.fetch = vi.fn();
    global.marked = { parse: vi.fn((md) => `<p>${md}</p>`) };
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    container.remove();
    delete window.__TAURI__;
    delete global.fetch;
    delete global.marked;
  });

  async function mountAndWait() {
    const api = mountTodoTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.todo-task-attachments-section')).not.toBeNull();
    });
    return api;
  }

  it('shows attachment section listing associated files', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: sampleAttachments, _status: 200 };
      }
      return {};
    });
    const { dispose } = await mountAndWait();
    expect(invokeMock).toHaveBeenCalledWith('list_todo_attachments', {
      masterTaskId: 'task_alpha',
    });
    expect(container.textContent).toContain('notes.md');
    expect(container.textContent).toContain('spec.md');
    expect(container.querySelector('[data-action="pick-attachment-md"]')).not.toBeNull();
    expect(container.textContent).toContain('Choose local .md');
    dispose();
  });

  it('shows understandable empty state when there are no attachments', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: [], _status: 200 };
      }
      return {};
    });
    const { dispose } = await mountAndWait();
    const empty = container.querySelector('.todo-task-attachments-empty');
    expect(empty).not.toBeNull();
    expect(empty.textContent.trim().length).toBeGreaterThan(0);
    expect(container.querySelectorAll('.todo-task-attachment-item')).toHaveLength(0);
    dispose();
  });

  it('pick flow stages then invokes add_todo_attachment with sourcePath', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    let listed = [];
    invokeMock.mockImplementation(async (cmd, args) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: listed, _status: 200 };
      }
      if (cmd === 'stage_todo_attachment_source') {
        return {
          source_path: `/tmp/staged/${args.preferredName}`,
          _status: 201,
        };
      }
      if (cmd === 'add_todo_attachment') {
        const base = String(args.sourcePath).split('/').pop();
        listed = [
          {
            file_name: base,
            original_file_name: base,
            added_at: '2026-07-18T12:00:00Z',
          },
        ];
        return { ...listed[0], _status: 201 };
      }
      return {};
    });
    dialogOpenMock.mockResolvedValue('/Users/me/new-notes.md');
    global.fetch.mockResolvedValue({
      ok: true,
      text: async () => '# New notes',
    });

    const { dispose } = await mountAndWait();
    expect(container.querySelector('.todo-task-attachments-empty')).not.toBeNull();

    container.querySelector('[data-action="pick-attachment-md"]').click();
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('stage_todo_attachment_source', {
        preferredName: 'new-notes.md',
        content: '# New notes',
      });
      expect(invokeMock).toHaveBeenCalledWith('add_todo_attachment', {
        masterTaskId: 'task_alpha',
        sourcePath: '/tmp/staged/new-notes.md',
      });
    });
    await vi.waitFor(() => {
      expect(container.textContent).toContain('new-notes.md');
    });
    expect(container.querySelector('.todo-task-attachments-empty')).toBeNull();
    dispose();
  });

  it('shows understandable error when user cancels pick (not silent)', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: [], _status: 200 };
      }
      return {};
    });
    dialogOpenMock.mockResolvedValue(null);

    const { dispose } = await mountAndWait();
    container.querySelector('[data-action="pick-attachment-md"]').click();
    await vi.waitFor(() => {
      const err = container.querySelector('.todo-task-attachments-error');
      expect(err).not.toBeNull();
      expect(err.textContent.trim().length).toBeGreaterThan(0);
    });
    expect(invokeMock).not.toHaveBeenCalledWith(
      'add_todo_attachment',
      expect.anything(),
    );
    dispose();
  });

  it('shows understandable error when Host dialog fails', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: [], _status: 200 };
      }
      return {};
    });
    dialogOpenMock.mockRejectedValue(new Error('dialog unavailable'));

    const { dispose } = await mountAndWait();
    container.querySelector('[data-action="pick-attachment-md"]').click();
    await vi.waitFor(() => {
      const err = container.querySelector('.todo-task-attachments-error');
      expect(err).not.toBeNull();
      expect(err.textContent).toMatch(/dialog unavailable|Failed to pick file|pick file/i);
    });
    dispose();
  });

  it('does not introduce new 计划任务 copy in attachment UI strings', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: [], _status: 200 };
      }
      return {};
    });
    const { dispose } = await mountAndWait();
    const section = container.querySelector('.todo-task-attachments-section');
    expect(section.textContent).not.toContain('计划任务');
    dispose();
  });
});

describe('attachment add surface does not reuse dialog.js CRUD types', () => {
  it('dialog.js TodoTaskDialogType remains the four master/sub CRUD types only', () => {
    const dialogSrc = readFrontendJs('frontend/src/todo-task/dialog.tsx');
    expect(dialogSrc).toMatch(
      /TodoTaskDialogType[\s\S]*?'create-master'[\s\S]*?'create-category'[\s\S]*?'add-sub'[\s\S]*?'delete-master'[\s\S]*?'delete-sub'/,
    );
    expect(dialogSrc).not.toMatch(/add-attachment|pick-attachment|attachment/);
  });

  it('index.js attachment pick action does not call openTodoTaskDialog', () => {
    const src = readFileSync(join(repoRoot, 'frontend/src/todo-task/attachments.tsx'), 'utf8')
      + readFileSync(join(repoRoot, 'frontend/src/todo-task/attachments-render.tsx'), 'utf8');
    expect(src).toMatch(/data-action="pick-attachment-md"/);
    const marker = "action === 'pick-attachment-md'";
    const idx = src.indexOf(marker);
    expect(idx).toBeGreaterThan(-1);
    const actionBlock = src.slice(idx, idx + 400);
    expect(actionBlock).not.toContain('openTodoTaskDialog');
  });
});
