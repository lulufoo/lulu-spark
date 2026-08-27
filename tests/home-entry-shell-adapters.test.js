// @vitest-environment jsdom
/**
 * T6: four business content adapters mount into the shell content slot;
 * own chrome (FAB / popover / modal host) is retired; host callbacks remain.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const getJsonMock = vi.fn();
const fetchIndexMock = vi.fn();
const renderFeedMock = vi.fn();

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

vi.mock('../frontend/js/api.js', () => ({
  fetchIndex: (...args) => fetchIndexMock(...args),
}));

vi.mock('../frontend/js/feed.js', () => ({
  renderFeed: (...args) => renderFeedMock(...args),
}));

import { createContentRegistry } from '../frontend/js/home-entry-shell/content-registry.js';
import {
  getAiAssistantEntry,
  getBaselineEntries,
} from '../frontend/js/home-entry-shell/entry-config.js';
import { mountHomeEntryShell } from '../frontend/js/home-entry-shell/shell.js';
import { createReadLaterContentAdapter } from '../frontend/js/read-later-assistant.js';
import { createTodoTaskContentAdapter } from '../frontend/js/todo-task-assistant.js';
import { createNotesContentAdapter } from '../frontend/js/note-assistant.js';
import { createBuildersContentAdapter } from '../frontend/js/builders-assistant.js';
import { createAiAssistantContentAdapter } from '../frontend/js/ai-assistant.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

const CHROME_RE =
  /\.(?:rl-assistant-(?:popover|fab|close)|todo-assistant-(?:popover|fab|close)|note-assistant-(?:popover|fab|close)|builders-(?:modal-host|modal-header|modal-close|entry-fab))\b/;

const ADAPTERS = [
  {
    key: 'read-later',
    create: createReadLaterContentAdapter,
    sourcePath: 'frontend/js/read-later-assistant.js',
    contentSelector:
      '.read-later-assistant-empty, .read-later-assistant-panel, .read-later-assistant-loading',
  },
  {
    key: 'todo-task',
    create: createTodoTaskContentAdapter,
    sourcePath: 'frontend/js/todo-task-assistant.js',
    contentSelector:
      '.todo-task-assistant-empty, .todo-task-assistant-list, .todo-task-assistant-loading',
  },
  {
    key: 'notes',
    create: createNotesContentAdapter,
    sourcePath: 'frontend/js/note-assistant.js',
    contentSelector: '.note-assistant-empty, .note-assistant-list, .note-assistant-loading',
  },
  {
    key: 'builders',
    create: createBuildersContentAdapter,
    sourcePath: 'frontend/js/builders-assistant.js',
    contentSelector: '.feed-mock',
  },
];

function readMain() {
  return readFileSync(join(repoRoot, 'frontend/js/main.js'), 'utf8');
}

function registerAll(registry) {
  for (const { key, create } of ADAPTERS) {
    registry.register(key, create());
  }
}

describe('home-entry-shell adapters (T6)', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    getJsonMock.mockReset();
    getJsonMock.mockResolvedValue([]);
    fetchIndexMock.mockReset();
    fetchIndexMock.mockResolvedValue({ entries: {} });
    renderFeedMock.mockReset();
    renderFeedMock.mockImplementation((container) => {
      container.innerHTML = '<div class="feed-mock">feed</div>';
    });
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('each baseline contentKey has a content adapter with mount → { unmount }', () => {
    for (const { key, create } of ADAPTERS) {
      const adapter = create();
      expect(typeof adapter.mount, `${key}.mount`).toBe('function');
      const slot = document.createElement('div');
      const handle = adapter.mount(slot, { host: {} });
      expect(handle && typeof handle.unmount, `${key}.unmount`).toBe('function');
      handle.unmount();
    }
  });

  it('adapters render business content into the slot without own chrome nodes', async () => {
    for (const { key, create, contentSelector } of ADAPTERS) {
      const slot = document.createElement('div');
      document.body.appendChild(slot);
      const handle = create().mount(slot, {
        host: {
          navigate: () => {},
          openReadLater: () => {},
          openCreateNote: () => {},
        },
      });

      await vi.waitFor(() => {
        expect(
          slot.querySelector(contentSelector),
          `${key} should paint content into slot`,
        ).not.toBeNull();
      });

      expect(slot.querySelector('.rl-assistant-popover'), key).toBeNull();
      expect(slot.querySelector('.todo-assistant-popover'), key).toBeNull();
      expect(slot.querySelector('.note-assistant-popover'), key).toBeNull();
      expect(slot.querySelector('.builders-modal-host'), key).toBeNull();
      expect(slot.querySelector('.builders-modal-header'), key).toBeNull();
      expect(slot.querySelector('[class*="-fab"]'), key).toBeNull();
      expect(slot.querySelector('[aria-label="Close"]'), key).toBeNull();

      handle.unmount();
      slot.remove();
    }
  });

  it('sources retire self-owned chrome (popover / modal host / fab close)', () => {
    for (const { key, sourcePath } of ADAPTERS) {
      const src = readFileSync(join(repoRoot, sourcePath), 'utf8');
      expect(src, key).not.toMatch(CHROME_RE);
      expect(src, key).not.toMatch(/mount(?:ReadLater|TodoTask|Note|Builders)AssistantWidget/);
    }
  });

  it('main.js registers all four baseline adapters on the ContentRegistry', () => {
    const source = readMain();
    expect(source).toMatch(/createReadLaterContentAdapter/);
    expect(source).toMatch(/createTodoTaskContentAdapter/);
    expect(source).toMatch(/createNotesContentAdapter/);
    expect(source).toMatch(/createBuildersContentAdapter/);
    expect(source).toMatch(/\.register\(\s*['"]read-later['"]/);
    expect(source).toMatch(/\.register\(\s*['"]todo-task['"]/);
    expect(source).toMatch(/\.register\(\s*['"]notes['"]/);
    expect(source).toMatch(/\.register\(\s*['"]builders['"]/);
  });

  it('read-later / todo-task adapters use host.navigate and host.openReadLater', async () => {
    const navigate = vi.fn();
    const openReadLater = vi.fn();

    const rlSlot = document.createElement('div');
    document.body.appendChild(rlSlot);
    const rl = createReadLaterContentAdapter().mount(rlSlot, {
      host: { navigate, openReadLater },
    });
    await vi.waitFor(() => {
      expect(rlSlot.querySelector('.read-later-assistant-manage-link')).not.toBeNull();
    });
    rlSlot.querySelector('.read-later-assistant-manage-link').click();
    expect(openReadLater).toHaveBeenCalled();
    rl.unmount();

    const ptSlot = document.createElement('div');
    document.body.appendChild(ptSlot);
    const pt = createTodoTaskContentAdapter().mount(ptSlot, {
      host: { navigate },
    });
    await vi.waitFor(() => {
      expect(ptSlot.querySelector('.todo-task-assistant-manage-link')).not.toBeNull();
    });
    ptSlot.querySelector('.todo-task-assistant-manage-link').click();
    expect(navigate).toHaveBeenCalledWith('#/todo-tasks');
    pt.unmount();
  });

  it('notes adapter uses host.openCreateNote', async () => {
    const openCreateNote = vi.fn();
    const slot = document.createElement('div');
    document.body.appendChild(slot);
    const handle = createNotesContentAdapter().mount(slot, { host: { openCreateNote } });
    await vi.waitFor(() => {
      expect(slot.querySelector('.note-assistant-create')).not.toBeNull();
    });
    slot.querySelector('.note-assistant-create').click();
    expect(openCreateNote).toHaveBeenCalled();
    handle.unmount();
  });

  it('smoke: four entries open shell overlay with usable content (Must Close SK-P3)', async () => {
    const registry = createContentRegistry();
    registerAll(registry);
    const config = getBaselineEntries();
    const anchor = document.createElement('div');
    document.body.appendChild(anchor);
    const shell = mountHomeEntryShell(anchor, {
      config,
      registry,
      host: {
        navigate: () => {},
        openReadLater: () => {},
        openCreateNote: () => {},
      },
    });

    anchor.querySelector('[data-role="hub"]').click();
    expect(shell.getState().mode).toBe('B');

    for (const entry of config) {
      await shell.openContent(entry.id);
      expect(shell.getState()).toEqual({ mode: 'C', entryId: entry.id });
      expect(anchor.querySelector('[data-role="overlay"]').hidden).toBe(false);

      const slot = anchor.querySelector('[data-role="content-slot"]');
      const adapterMeta = ADAPTERS.find((a) => a.key === entry.contentKey);
      await vi.waitFor(() => {
        expect(
          slot.querySelector(adapterMeta.contentSelector),
          `${entry.id} content in slot`,
        ).not.toBeNull();
      });
      expect(slot.querySelector('.builders-modal-header')).toBeNull();
      expect(slot.querySelector('.rl-assistant-popover-header')).toBeNull();
    }

    shell.unmount();
  });
});

/**
 * T2: AI bypass entry + ContentAdapter registration + C_AI mount.
 * Entry is pinned (not getBaselineEntries); adapter registers in main.js only.
 */
describe('home-entry-shell adapters · AI bypass ContentAdapter (T2)', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('getAiAssistantEntry exposes id/contentKey=ai-assistant outside getBaselineEntries', () => {
    const ai = getAiAssistantEntry();
    expect(ai).toBeTruthy();
    expect(ai.id).toBe('ai-assistant');
    expect(ai.contentKey).toBe('ai-assistant');
    expect(typeof ai.title).toBe('string');
    expect(ai.title.length).toBeGreaterThan(0);
    expect(typeof ai.overlayTitle).toBe('string');
    expect(ai.overlayTitle.length).toBeGreaterThan(0);
    expect(typeof ai.fabClass).toBe('string');
    expect(ai.fabClass.length).toBeGreaterThan(0);

    const baseline = getBaselineEntries();
    expect(baseline.some((e) => e.id === 'ai-assistant')).toBe(false);
    expect(baseline.some((e) => e.contentKey === 'ai-assistant')).toBe(false);
  });

  it('createAiAssistantContentAdapter mounts interactive chat UI into the slot', () => {
    const adapter = createAiAssistantContentAdapter();
    expect(typeof adapter.mount).toBe('function');

    const slot = document.createElement('div');
    document.body.appendChild(slot);
    const handle = adapter.mount(slot, { host: {} });
    expect(handle && typeof handle.unmount).toBe('function');

    expect(slot.querySelector('.ai-assistant-messages')).not.toBeNull();
    expect(slot.querySelector('.ai-assistant-composer')).not.toBeNull();
    expect(slot.querySelector('[data-role="input"]')).not.toBeNull();
    expect(slot.querySelector('[data-role="send"]')).not.toBeNull();
    // Shell owns chrome; adapter must not invent its own FAB / independent-window chrome.
    expect(slot.querySelector('[class*="-fab"]')).toBeNull();

    handle.unmount();
    expect(slot.querySelector('.ai-assistant-composer')).toBeNull();
  });

  it('main.js registers ai-assistant adapter beside baseline; content-registry.js stays factory-only', () => {
    const mainSrc = readMain();
    expect(mainSrc).toMatch(/createAiAssistantContentAdapter/);
    expect(mainSrc).toMatch(
      /\.register\(\s*['"]ai-assistant['"]\s*,\s*createAiAssistantContentAdapter\s*\(\s*\)\s*\)/,
    );
    expect(mainSrc).toMatch(/getAiAssistantEntry/);
    expect(mainSrc).toMatch(/\baiEntry\s*:/);

    const registrySrc = readFileSync(
      join(repoRoot, 'frontend/js/home-entry-shell/content-registry.js'),
      'utf8',
    );
    expect(registrySrc).not.toMatch(/ai-assistant/);
    expect(registrySrc).not.toMatch(/createAiAssistantContentAdapter/);
    expect(registrySrc).not.toMatch(/\.register\s*\(/);
  });

  it('ai-assistant adapter reuses mountAiAssistant and does not Reset or switch live', () => {
    const src = readFileSync(join(repoRoot, 'frontend/js/ai-assistant.js'), 'utf8');
    expect(src).toMatch(/export\s+function\s+createAiAssistantContentAdapter\s*\(/);
    const factoryIdx = src.indexOf('function createAiAssistantContentAdapter');
    expect(factoryIdx).toBeGreaterThanOrEqual(0);
    const factoryBody = src.slice(factoryIdx);
    expect(factoryBody).toMatch(/mountAiAssistant\s*\(/);
    expect(src).not.toMatch(/reset_binding|resetBinding|reset_ai_assistant/);
    expect(src).not.toMatch(/switch[_]?live|set_live_session/);
  });

  it('smoke: A shows AI bypass; C_AI mounts interactive chat via registered adapter', async () => {
    const registry = createContentRegistry();
    registerAll(registry);
    registry.register('ai-assistant', createAiAssistantContentAdapter());

    const config = getBaselineEntries();
    const aiEntry = getAiAssistantEntry();
    const anchor = document.createElement('div');
    document.body.appendChild(anchor);
    const shell = mountHomeEntryShell(anchor, {
      config,
      aiEntry,
      registry,
      host: {},
    });

    expect(shell.getState().mode).toBe('A');
    const aiBtn = anchor.querySelector(
      '[data-role="ai-entry"], [data-entry-id="ai-assistant"]',
    );
    expect(aiBtn).not.toBeNull();
    expect(aiBtn.hidden).toBe(false);

    aiBtn.click();
    await vi.waitFor(() => {
      expect(shell.getState()).toEqual({ mode: 'C', entryId: 'ai-assistant' });
    });

    const overlay = anchor.querySelector('[data-role="overlay"]');
    expect(overlay.hidden).toBe(false);
    const slot = anchor.querySelector('[data-role="content-slot"]');
    await vi.waitFor(() => {
      expect(slot.querySelector('.ai-assistant-composer')).not.toBeNull();
      expect(slot.querySelector('[data-role="input"]')).not.toBeNull();
    });
    // Composer controls are present and wired (disabled until Binding Contract bound is OK).
    const input = slot.querySelector('[data-role="input"]');
    const send = slot.querySelector('[data-role="send"]');
    const form = slot.querySelector('[data-role="form"]');
    expect(input).toBeInstanceOf(HTMLTextAreaElement);
    expect(send).toBeInstanceOf(HTMLButtonElement);
    expect(form).toBeInstanceOf(HTMLFormElement);
    expect(() => form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))).not.toThrow();

    shell.unmount();
  });

  // T7 / L22-VF: B/C hide AI entry (adapter registry still mounted; visibility is shell).
  it('B/C hide AI entry while baseline adapters remain registered', async () => {
    const registry = createContentRegistry();
    registerAll(registry);
    registry.register('ai-assistant', createAiAssistantContentAdapter());

    const config = getBaselineEntries();
    const aiEntry = getAiAssistantEntry();
    const anchor = document.createElement('div');
    document.body.appendChild(anchor);
    const shell = mountHomeEntryShell(anchor, {
      config,
      aiEntry,
      registry,
      host: {},
    });

    const aiBtn = anchor.querySelector(
      '[data-role="ai-entry"], [data-entry-id="ai-assistant"]',
    );
    expect(aiBtn).not.toBeNull();
    expect(aiBtn.hidden).toBe(false);

    anchor.querySelector('[data-role="hub"]').click();
    expect(shell.getState().mode).toBe('B');
    expect(aiBtn.hidden).toBe(true);

    await shell.openContent('notes');
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'notes' });
    expect(aiBtn.hidden).toBe(true);
    // VF 否证: B/C 仍显 AI → fail above. Adapter registration remains for remount.
    expect(registry.get('ai-assistant')).toBeTruthy();

    shell.unmount();
  });
});
