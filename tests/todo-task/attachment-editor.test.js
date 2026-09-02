// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFrontendJs } from '../helpers/read-frontend-js.js';

const invokeMock = vi.fn();
const getJsonMock = vi.fn();

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

import { closeFilePopup } from '../../frontend/src/file-popup/index.ts';
import { emptyFilePopupView, viewStore } from '../../frontend/src/file-popup/state/store.ts';
import {
  mountTodoTaskSplit,
  readPlanAttachment,
  savePlanAttachment,
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

const ATTACH_PATH = '/tmp/todo_tasks/tasks/task_alpha/attachments/notes.md';

const sampleAttachments = [
  {
    file_name: 'notes.md',
    original_file_name: 'notes.md',
    added_at: '2026-07-18T10:00:00Z',
    path: ATTACH_PATH,
  },
];

const ATTACHMENT_BODY = '# Attachment notes\n\nBody paragraph.';

function setupTauri() {
  window.__TAURI__ = { core: { invoke: invokeMock } };
}

describe('readPlanAttachment / savePlanAttachment', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    setupTauri();
  });

  afterEach(() => {
    delete window.__TAURI__;
  });

  it('readPlanAttachment invokes read_todo_attachment with masterTaskId and fileName', async () => {
    invokeMock.mockResolvedValue({ file_name: 'notes.md', content: ATTACHMENT_BODY });
    const result = await readPlanAttachment({
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
    });
    expect(invokeMock).toHaveBeenCalledWith('read_todo_attachment', {
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
    });
    expect(result).toEqual({ file_name: 'notes.md', content: ATTACHMENT_BODY });
  });

  it('savePlanAttachment invokes save_todo_attachment with masterTaskId, fileName, sourcePath', async () => {
    invokeMock.mockResolvedValue({ ok: true });
    await savePlanAttachment({
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
      sourcePath: '/tmp/notes.md',
    });
    expect(invokeMock).toHaveBeenCalledWith('save_todo_attachment', {
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
      sourcePath: '/tmp/notes.md',
    });
  });

  it('throws with status when read returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Attachment not found', _status: 404 });
    await expect(
      readPlanAttachment({ masterTaskId: 'task_alpha', fileName: 'missing.md' }),
    ).rejects.toMatchObject({ status: 404 });
  });
});

describe('mountTodoTaskSplit attachment FilePopup', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    getJsonMock.mockReset();
    invokeMock.mockReset();
    setupTauri();
    global.marked = {
      parse: vi.fn((md) => `<div class="mock-md">${String(md).replace(/\n/g, '<br>')}</div>`),
    };
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    closeFilePopup();
    viewStore.set(emptyFilePopupView());
    container.remove();
    delete window.__TAURI__;
    delete global.marked;
  });

  function mockHappyPath() {
    getJsonMock.mockImplementation(async (url) => {
      if (String(url).startsWith('/api/file')) {
        return { content: ATTACHMENT_BODY };
      }
      return [sampleMaster];
    });
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_todo_attachments') {
        return { attachments: sampleAttachments, _status: 200 };
      }
      return {};
    });
  }

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

  it('clicking an attachment opens FilePopup with path and todos identity', async () => {
    mockHappyPath();
    const { dispose } = await mountAndWait();
    const item = container.querySelector(
      '.todo-task-attachment-item[data-file-name="notes.md"]',
    );
    expect(item).not.toBeNull();
    expect(item.getAttribute('data-path')).toBe(ATTACH_PATH);
    item.click();
    await vi.waitFor(() => {
      expect(document.getElementById('file-popup')).not.toBeNull();
    });
    expect(container.querySelector('.todo-task-attachment-editor')).toBeNull();
    const view = viewStore.getSnapshot();
    expect(view.path).toBe(ATTACH_PATH);
    expect(view.title).toBe('notes.md');
    expect(view.identityKey).toBe('todos:task_alpha:att:notes.md');
    const urls = getJsonMock.mock.calls.map((call) => String(call[0]));
    expect(urls.some((url) => url.startsWith('/api/file?path='))).toBe(true);
    dispose();
  });

  it('does not call read_todo_attachment or save_todo_attachment when opening', async () => {
    mockHappyPath();
    const { dispose } = await mountAndWait();
    container.querySelector('.todo-task-attachment-item[data-file-name="notes.md"]').click();
    await vi.waitFor(() => {
      expect(document.getElementById('file-popup')).not.toBeNull();
    });
    expect(invokeMock).not.toHaveBeenCalledWith('read_todo_attachment', expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith('save_todo_attachment', expect.anything());
    dispose();
  });
});

describe('attachment FilePopup surface contracts', () => {
  it('open-attachment goes through FilePopup, not todo_md or AttachmentEditor', () => {
    const host = readFileSync(join(repoRoot, 'frontend/src/todo-task/state/host.ts'), 'utf8');
    const src = readFileSync(join(repoRoot, 'frontend/src/todo-task/commands/attachments.ts'), 'utf8')
      + readFileSync(join(repoRoot, 'frontend/src/todo-task/ui/attachments.tsx'), 'utf8');
    expect(host).toMatch(/read_todo_attachment/);
    expect(host).toMatch(/save_todo_attachment/);

    const openIdx = src.indexOf("action === 'open-attachment'");
    expect(openIdx).toBeGreaterThan(-1);
    const openBlock = src.slice(openIdx, openIdx + 800);
    expect(openBlock).toContain('openAttachment');
    expect(openBlock).not.toContain('read_todo_md');
    expect(openBlock).not.toContain('update_todo_md');
    expect(src).toContain('openFilePopup');
    expect(src).not.toContain('AttachmentEditor');
    expect(src).not.toContain('renderAttachmentEditor');
    expect(src).not.toContain('todo-task-attachment-editor');
  });

  it('dialog.js TodoTaskDialogType remains master/sub CRUD only (no attachment editor type)', () => {
    const dialogSrc = [
      readFrontendJs('frontend/src/todo-task/ui/dialog.tsx'),
      readFrontendJs('frontend/src/todo-task/commands/dialog.ts'),
      readFrontendJs('frontend/src/todo-task/state/dialog.ts'),
    ].join('\n');
    expect(dialogSrc).toMatch(
      /TodoTaskDialogType[\s\S]*?'create-master'[\s\S]*?'create-category'[\s\S]*?'add-sub'[\s\S]*?'delete-master'[\s\S]*?'delete-sub'/,
    );
    expect(dialogSrc).not.toMatch(/attachment-editor|open-attachment|edit-attachment/);
  });
});
