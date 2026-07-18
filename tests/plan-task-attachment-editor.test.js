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
  mountPlanTaskSplit,
  readPlanAttachment,
  savePlanAttachment,
} from '../frontend/js/plan-task/index.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

const sampleMaster = {
  master_task_id: 'task_alpha',
  title: 'Alpha Task',
  status: 'incomplete',
  created_at: '2026-07-01T10:00:00Z',
  plan_md: '# Plan',
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

  it('readPlanAttachment invokes read_plan_attachment with masterTaskId and fileName', async () => {
    invokeMock.mockResolvedValue({ file_name: 'notes.md', content: ATTACHMENT_BODY });
    const result = await readPlanAttachment({
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
    });
    expect(invokeMock).toHaveBeenCalledWith('read_plan_attachment', {
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
    });
    expect(result).toEqual({ file_name: 'notes.md', content: ATTACHMENT_BODY });
  });

  it('savePlanAttachment invokes save_plan_attachment with masterTaskId, fileName, content', async () => {
    invokeMock.mockResolvedValue({ ok: true });
    await savePlanAttachment({
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
      content: '# Saved',
    });
    expect(invokeMock).toHaveBeenCalledWith('save_plan_attachment', {
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
      content: '# Saved',
    });
  });

  it('throws with status when read returns service error payload', async () => {
    invokeMock.mockResolvedValue({ error: 'Attachment not found', _status: 404 });
    await expect(
      readPlanAttachment({ masterTaskId: 'task_alpha', fileName: 'missing.md' }),
    ).rejects.toMatchObject({ status: 404 });
  });
});

describe('mountPlanTaskSplit attachment editor modal', () => {
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
    container.remove();
    delete window.__TAURI__;
    delete global.marked;
  });

  function mockHappyPath() {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_plan_attachments') {
        return { attachments: sampleAttachments, _status: 200 };
      }
      if (cmd === 'read_plan_attachment') {
        return { file_name: 'notes.md', content: ATTACHMENT_BODY };
      }
      if (cmd === 'save_plan_attachment') {
        return { ok: true };
      }
      return {};
    });
  }

  async function mountAndWait() {
    const api = mountPlanTaskSplit(container, {
      masterId: 'task_alpha',
      subId: 'task_alpha_sub_01',
    });
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-attachment-item')).not.toBeNull();
    });
    return api;
  }

  async function openAttachmentEditor() {
    mockHappyPath();
    const api = await mountAndWait();
    const item = container.querySelector(
      '.plan-task-attachment-item[data-file-name="notes.md"]',
    );
    expect(item).not.toBeNull();
    item.click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-attachment-preview')).not.toBeNull();
      expect(container.querySelector('[data-action="edit-attachment"]')).not.toBeNull();
    });
    return api;
  }

  it('clicking an attachment opens a modal with preview by default (viewer-style)', async () => {
    const { dispose } = await openAttachmentEditor();
    expect(invokeMock).toHaveBeenCalledWith('read_plan_attachment', {
      masterTaskId: 'task_alpha',
      fileName: 'notes.md',
    });
    expect(invokeMock).not.toHaveBeenCalledWith('read_plan_md', expect.anything());

    const editor = container.querySelector('.plan-task-attachment-editor');
    expect(editor).not.toBeNull();
    expect(editor.querySelector('.plan-task-attachment-preview')).not.toBeNull();
    expect(editor.querySelector('.plan-task-attachment-edit-area')).toBeNull();
    expect(editor.querySelector('[data-action="edit-attachment"]')).not.toBeNull();
    expect(global.marked.parse).toHaveBeenCalledWith(ATTACHMENT_BODY);
    expect(editor.textContent).toContain('Attachment notes');
    dispose();
  });

  it('can switch to edit mode and save via save_plan_attachment', async () => {
    const { dispose } = await openAttachmentEditor();
    const editor = container.querySelector('.plan-task-attachment-editor');
    editor.querySelector('[data-action="edit-attachment"]').click();
    await vi.waitFor(() => {
      expect(editor.querySelector('.plan-task-attachment-edit-area')).not.toBeNull();
    });
    expect(editor.querySelector('.plan-task-attachment-preview')).toBeNull();

    const textarea = editor.querySelector('.plan-task-attachment-edit-area');
    textarea.value = '# Saved attachment\n\nUpdated.';
    editor.querySelector('[data-action="save-attachment"]').click();

    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('save_plan_attachment', {
        masterTaskId: 'task_alpha',
        fileName: 'notes.md',
        content: '# Saved attachment\n\nUpdated.',
      });
    });
    expect(invokeMock).not.toHaveBeenCalledWith('update_plan_md', expect.anything());

    await vi.waitFor(() => {
      const modal = container.querySelector('.plan-task-attachment-editor');
      const preview = modal?.querySelector('.plan-task-attachment-preview');
      expect(preview).not.toBeNull();
      expect(modal.querySelector('.plan-task-attachment-edit-area')).toBeNull();
    });
    dispose();
  });

  it('closing the modal discards unsaved draft without dirty check', async () => {
    const { dispose } = await openAttachmentEditor();
    const editor = container.querySelector('.plan-task-attachment-editor');
    editor.querySelector('[data-action="edit-attachment"]').click();
    await vi.waitFor(() => {
      expect(editor.querySelector('.plan-task-attachment-edit-area')).not.toBeNull();
    });
    editor.querySelector('.plan-task-attachment-edit-area').value = 'DRAFT SHOULD BE DROPPED';

    editor.querySelector('[data-action="close-attachment-editor"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-attachment-editor')).toBeNull();
    });
    expect(invokeMock).not.toHaveBeenCalledWith('save_plan_attachment', expect.anything());

    // Re-open: draft must not persist
    container
      .querySelector('.plan-task-attachment-item[data-file-name="notes.md"]')
      .click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-attachment-preview')).not.toBeNull();
    });
    const reopened = container.querySelector('.plan-task-attachment-editor');
    await vi.waitFor(() => {
      expect(reopened.querySelector('[data-action="edit-attachment"]')).not.toBeNull();
    });
    reopened.querySelector('[data-action="edit-attachment"]').click();
    await vi.waitFor(() => {
      expect(reopened.querySelector('.plan-task-attachment-edit-area')).not.toBeNull();
    });
    expect(reopened.querySelector('.plan-task-attachment-edit-area').value).toBe(
      ATTACHMENT_BODY,
    );
    expect(reopened.querySelector('.plan-task-attachment-edit-area').value).not.toContain(
      'DRAFT SHOULD BE DROPPED',
    );
    dispose();
  });

  it('keeps editor content and shows error when save fails', async () => {
    getJsonMock.mockResolvedValue([sampleMaster]);
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'list_plan_attachments') {
        return { attachments: sampleAttachments, _status: 200 };
      }
      if (cmd === 'read_plan_attachment') {
        return { file_name: 'notes.md', content: ATTACHMENT_BODY };
      }
      if (cmd === 'save_plan_attachment') {
        return { error: 'Disk full', _status: 500 };
      }
      return {};
    });
    const { dispose } = await mountAndWait();
    container
      .querySelector('.plan-task-attachment-item[data-file-name="notes.md"]')
      .click();
    await vi.waitFor(() => {
      expect(container.querySelector('.plan-task-attachment-preview')).not.toBeNull();
      expect(container.querySelector('[data-action="edit-attachment"]')).not.toBeNull();
    });
    const editor = container.querySelector('.plan-task-attachment-editor');
    editor.querySelector('[data-action="edit-attachment"]').click();
    await vi.waitFor(() => {
      expect(editor.querySelector('.plan-task-attachment-edit-area')).not.toBeNull();
    });
    editor.querySelector('.plan-task-attachment-edit-area').value = 'User draft content';
    editor.querySelector('[data-action="save-attachment"]').click();
    await vi.waitFor(() => {
      expect(editor.querySelector('.plan-task-attachment-error')).not.toBeNull();
    });
    expect(editor.querySelector('.plan-task-attachment-edit-area').value).toBe(
      'User draft content',
    );
    expect(editor.textContent).toMatch(/Disk full|保存失败/);
    dispose();
  });
});

describe('attachment editor surface contracts', () => {
  it('does not reuse read_plan_md / update_plan_md for attachment editor path', () => {
    const src = readFileSync(join(repoRoot, 'frontend/js/plan-task/index.js'), 'utf8');
    expect(src).toMatch(/read_plan_attachment/);
    expect(src).toMatch(/save_plan_attachment/);

    const openIdx = src.indexOf("action === 'open-attachment'");
    expect(openIdx).toBeGreaterThan(-1);
    const openBlock = src.slice(openIdx, openIdx + 800);
    expect(openBlock).not.toContain('read_plan_md');
    expect(openBlock).not.toContain('update_plan_md');

    const saveIdx = src.indexOf("action === 'save-attachment'");
    expect(saveIdx).toBeGreaterThan(-1);
    const saveBlock = src.slice(saveIdx, saveIdx + 600);
    expect(saveBlock).not.toContain('update_plan_md');
    expect(saveBlock).not.toContain('read_plan_md');
  });

  it('attachment editor is a dedicated modal surface, not comments default-edit or plan_md inline', () => {
    const src = readFileSync(join(repoRoot, 'frontend/js/plan-task/index.js'), 'utf8');
    expect(src).toMatch(/plan-task-attachment-editor/);
    expect(src).toMatch(/plan-task-attachment-preview/);
    expect(src).toMatch(/data-action="edit-attachment"/);

    // Must not open attachment via the master/sub CRUD dialog types
    const openIdx = src.indexOf("action === 'open-attachment'");
    expect(openIdx).toBeGreaterThan(-1);
    const openBlock = src.slice(openIdx, openIdx + 500);
    expect(openBlock).not.toContain('openPlanTaskDialog');

    // Preview-default: edit area must not be the sole initial surface in render helper
    const renderIdx = src.indexOf('function renderAttachmentEditor');
    expect(renderIdx).toBeGreaterThan(-1);
    const renderBlock = src.slice(renderIdx, renderIdx + 1200);
    expect(renderBlock).toMatch(/attachment-preview|editMode|preview/);
  });

  it('dialog.js PlanTaskDialogType remains master/sub CRUD only (no attachment editor type)', () => {
    const dialogSrc = readFileSync(
      join(repoRoot, 'frontend/js/plan-task/dialog.js'),
      'utf8',
    );
    expect(dialogSrc).toMatch(
      /@typedef \{'create-master' \| 'add-sub' \| 'delete-master' \| 'delete-sub'\} PlanTaskDialogType/,
    );
    expect(dialogSrc).not.toMatch(/attachment-editor|open-attachment|edit-attachment/);
  });
});
