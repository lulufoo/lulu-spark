// @vitest-environment jsdom
/**
 * T6: three business content adapters mount into the shell content slot;
 * own chrome (FAB / popover / modal host) is retired; host callbacks remain.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const getJsonMock = vi.fn();
const fetchIndexMock = vi.fn();
const renderFeedMock = vi.fn();

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

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchIndex: (...args) => fetchIndexMock(...args),
}));

vi.mock('../../frontend/src/builders/ui/feed.tsx', () => ({
  renderFeed: (...args) => renderFeedMock(...args),
}));

import { createContentRegistry } from '../../frontend/src/home-entry-shell/content-registry.ts';
import { getBaselineEntries } from '../../frontend/src/home-entry-shell/entry-config.ts';
import { mountHomeEntryShell } from '../../frontend/src/home-entry-shell/shell.tsx';
import { createNotesContentAdapter } from '../../frontend/src/notes/ui/assistant.tsx';
import { createBuildersContentAdapter } from '../../frontend/src/builders/ui/assistant.tsx';
import { readFrontendJs, readMainSource } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const CHROME_RE =
  /\.(?:rl-assistant-(?:popover|fab|close)|todo-assistant-(?:popover|fab|close)|note-assistant-(?:popover|fab|close)|builders-(?:modal-host|modal-header|modal-close|entry-fab))\b/;

const ADAPTERS = [
  {
    key: 'notes',
    create: createNotesContentAdapter,
    sourcePath: 'frontend/src/notes/ui/assistant.tsx',
    contentSelector: '.note-assistant-empty, .note-assistant-list, .note-assistant-loading',
  },
  {
    key: 'builders',
    create: createBuildersContentAdapter,
    sourcePath: 'frontend/src/builders/ui/assistant.tsx',
    contentSelector: '.feed-mock',
  },
];

function readMain() {
  return readMainSource();
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
      const src = sourcePath.startsWith('frontend/src/')
        ? readFrontendJs(sourcePath)
        : readFileSync(join(repoRoot, sourcePath), 'utf8');
      expect(src, key).not.toMatch(CHROME_RE);
      expect(src, key).not.toMatch(/mount(?:ReadLater|TodoTask|Note|Builders)AssistantWidget/);
    }
  });

  it('main.js does not register hub content adapters', () => {
    const source = readMain();
    expect(source).not.toMatch(/createReadLaterContentAdapter/);
    expect(source).not.toMatch(/createTodoTaskContentAdapter/);
    expect(source).not.toMatch(/createNotesContentAdapter/);
    expect(source).not.toMatch(/createBuildersContentAdapter/);
    expect(source).not.toMatch(/\.register\(\s*['"]read-later['"]/);
    expect(source).not.toMatch(/\.register\(\s*['"]todo-task['"]/);
    expect(source).not.toMatch(/\.register\(\s*['"]notes['"]/);
    expect(source).not.toMatch(/\.register\(\s*['"]builders['"]/);
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

  it('smoke: remaining adapters still mount into a local shell slot', async () => {
    const registry = createContentRegistry();
    registerAll(registry);
    const config = getBaselineEntries().filter((entry) =>
      ADAPTERS.some((adapter) => adapter.key === entry.contentKey),
    );
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

describe('home-entry-shell adapters · Assistant overlay retired', () => {
  it('does not ship an Assistant pin, adapter, or overlay module', () => {
    const mainSrc = readMain();
    expect(mainSrc).not.toMatch(/createAiAssistantContentAdapter/);
    expect(mainSrc).not.toMatch(/getAiAssistantEntry/);
    expect(mainSrc).not.toMatch(/\baiEntry\s*:/);

    const configSrc = readFileSync(
      join(repoRoot, 'frontend/src/home-entry-shell/entry-config.ts'),
      'utf8',
    );
    expect(configSrc).not.toMatch(/getAiAssistantEntry/);
    expect(configSrc).not.toMatch(/ai-assistant/);

    const registrySrc = readFileSync(
      join(repoRoot, 'frontend/src/home-entry-shell/content-registry.ts'),
      'utf8',
    );
    expect(registrySrc).not.toMatch(/ai-assistant/);
    expect(registrySrc).not.toMatch(/createAiAssistantContentAdapter/);

    expect(existsSync(join(repoRoot, 'frontend/src/ai-assistant.js'))).toBe(false);
    const baseline = getBaselineEntries();
    expect(baseline.some((e) => e.id === 'ai-assistant')).toBe(false);
  });
});
