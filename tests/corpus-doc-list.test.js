// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

vi.mock('../frontend/js/host/api.js', () => ({
  fetchKbList: vi.fn(),
  fetchKbDocCount: vi.fn(),
  fetchSedimentKbRepos: vi.fn(),
}));

vi.mock('../frontend/js/corpus/kb-viewer.js', () => ({
  mountKbReader: vi.fn(),
}));

import * as api from '../frontend/js/host/api.js';
import { mountKbReader } from '../frontend/js/corpus/kb-viewer.js';
import { getKbHidePattern } from '../frontend/js/corpus/kb-hide-pattern.js';
import {
  buildTreeNodes,
  buildRepoPickerOptions,
  formatRepoMenuLabel,
  mountCorpusDocList,
} from '../frontend/js/corpus/corpus-doc-list.js';
import { positionFloatingListMenu } from '../frontend/js/components/floating-list-menu.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const mainJs = readFileSync(join(fixtureRoot, 'frontend/js/main.js'), 'utf8');

const sampleRootEntries = [
  { name: 'docs', relative_path: 'docs', is_dir: true },
  { name: 'readme.md', relative_path: 'readme.md', is_dir: false },
];

const sampleDocsEntries = [
  { name: 'guide.md', relative_path: 'docs/guide.md', is_dir: false },
  { name: 'nested', relative_path: 'docs/nested', is_dir: true },
];

/** @param {HTMLElement} container @param {string} relativePath */
function clickDirLabel(container, relativePath) {
  const label = container.querySelector(
    `.corpus-doc-tree-node[data-relative-path="${relativePath}"] .corpus-doc-tree-label`,
  );
  expect(label).not.toBeNull();
  label.click();
}

function installLocalStorageMock() {
  const store = {};
  globalThis.localStorage = {
    getItem(key) {
      return Object.prototype.hasOwnProperty.call(store, key) ? store[key] : null;
    },
    setItem(key, value) {
      store[key] = String(value);
    },
    removeItem(key) {
      delete store[key];
    },
    clear() {
      for (const key of Object.keys(store)) delete store[key];
    },
  };
}

describe('buildTreeNodes', () => {
  beforeEach(() => {
    installLocalStorageMock();
  });

  it('maps flat entries to tree nodes with defaults', () => {
    const nodes = buildTreeNodes(sampleRootEntries, '');
    expect(nodes).toHaveLength(2);
    expect(nodes[0]).toMatchObject({
      name: 'docs',
      relative_path: 'docs',
      is_dir: true,
      expanded: false,
      loaded: false,
      children: [],
    });
    expect(nodes[1]).toMatchObject({
      name: 'readme.md',
      relative_path: 'readme.md',
      is_dir: false,
    });
  });

  it('deduplicates by relative_path', () => {
    const dupes = [
      { name: 'a.md', relative_path: 'a.md', is_dir: false },
      { name: 'a.md', relative_path: 'a.md', is_dir: false },
    ];
    expect(buildTreeNodes(dupes, '')).toHaveLength(1);
  });

  it('hides entries whose name matches kb hide pattern (I5: name only)', () => {
    localStorage.setItem('kb_hide_pattern', '\\.xxx$');
    const entries = [
      { name: 'noise.xxx', relative_path: 'noise.xxx', is_dir: false },
      { name: 'readme.md', relative_path: 'readme.md', is_dir: false },
      { name: 'hidden.xxx', relative_path: 'docs/hidden.xxx', is_dir: false },
    ];
    const nodes = buildTreeNodes(entries, '');
    expect(nodes.map((n) => n.name)).toEqual(['readme.md']);
  });

  it('keeps parent directory when all children would be hidden (I4)', () => {
    localStorage.setItem('kb_hide_pattern', '\\.xxx$');
    const entries = [
      { name: 'empty-dir', relative_path: 'empty-dir', is_dir: true },
      { name: 'only.xxx', relative_path: 'only.xxx', is_dir: false },
    ];
    const nodes = buildTreeNodes(entries, '');
    expect(nodes.map((n) => n.name)).toEqual(['empty-dir']);
  });

  it('does not filter when hide pattern is empty', () => {
    localStorage.clear();
    const entries = [
      { name: 'noise.xxx', relative_path: 'noise.xxx', is_dir: false },
      { name: 'readme.md', relative_path: 'readme.md', is_dir: false },
    ];
    expect(buildTreeNodes(entries, '')).toHaveLength(2);
    expect(getKbHidePattern()).toBe('');
  });
});

describe('positionFloatingListMenu', () => {
  const triggerRect = {
    top: 100,
    bottom: 136,
    left: 10,
    width: 200,
    right: 210,
    height: 36,
    x: 10,
    y: 100,
    toJSON: () => ({}),
  };

  /**
   * @param {number} optionCount
   */
  function makeMenu(optionCount) {
    const menu = document.createElement('div');
    menu.className = 'list-select-menu';
    menu.innerHTML = Array.from({ length: optionCount }, (_, i) =>
      `<button type="button" class="list-select-option">item ${i}</button>`,
    ).join('');
    document.body.appendChild(menu);
    return menu;
  }

  afterEach(() => {
    document.querySelectorAll('.list-select-menu').forEach((el) => el.remove());
  });

  it('expands to natural height when viewport space is sufficient', () => {
    const menu = makeMenu(3);
    Object.defineProperty(menu, 'scrollHeight', { value: 120, configurable: true });

    const clipped = positionFloatingListMenu(menu, triggerRect, 800);

    expect(clipped).toBe(false);
    expect(menu.style.maxHeight).toBe('');
    expect(menu.style.top).toBe('140px');
  });

  it('clips to available viewport space when list is taller', () => {
    const menu = makeMenu(20);
    Object.defineProperty(menu, 'scrollHeight', { value: 600, configurable: true });

    const clipped = positionFloatingListMenu(menu, triggerRect, 400);

    expect(clipped).toBe(true);
    expect(menu.style.maxHeight).toBe('204px');
  });
});

describe('formatRepoMenuLabel', () => {
  it('appends middle dot count when available', () => {
    expect(formatRepoMenuLabel('repo', 5)).toBe('repo · 5');
    expect(formatRepoMenuLabel('repo', null)).toBe('repo');
  });

  it('buildRepoPickerOptions maps counts onto labels', () => {
    const opts = buildRepoPickerOptions(
      [{ full_name: 'owner/repo' }],
      new Map([['owner/repo', 7]]),
    );
    expect(opts[0].label).toBe('repo · 7');
  });
});

describe('mountCorpusDocList', () => {
  let container;
  let navigate;

  beforeEach(() => {
    installLocalStorageMock();
    container = document.createElement('div');
    document.body.appendChild(container);
    navigate = vi.fn();
    vi.clearAllMocks();
    mountKbReader.mockResolvedValue({ unmount: vi.fn() });
    api.fetchSedimentKbRepos.mockResolvedValue({
      repos: [
        { full_name: 'owner/repo' },
        { full_name: 'owner/other' },
      ],
    });
    api.fetchKbDocCount.mockImplementation((repo) => {
      if (repo === 'owner/repo') return Promise.resolve(5);
      if (repo === 'owner/other') return Promise.resolve(12);
      return Promise.resolve(0);
    });
  });

  afterEach(() => {
    container.remove();
  });

  async function flushPromises() {
    await Promise.resolve();
    await Promise.resolve();
    await new Promise((resolve) => setTimeout(resolve, 0));
  }

  it('fetches root flat list on mount', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    expect(api.fetchKbList).toHaveBeenCalledWith('owner/repo', '', 'flat');
    expect(api.fetchKbList).toHaveBeenCalledTimes(1);
  });

  it('renders sidebar and reader pane layout', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    expect(container.querySelector('.corpus-doc-layout')).not.toBeNull();
    expect(container.querySelector('.corpus-doc-sidebar')).not.toBeNull();
    expect(container.querySelector('.corpus-doc-reader-pane')).not.toBeNull();
    expect(container.querySelector('.corpus-doc-main')).toBeNull();
    expect(container.querySelector('.corpus-doc-main-list')).toBeNull();
  });

  it('applies depth * 16px padding to tree nodes', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const rootNode = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="docs"]',
    );
    expect(rootNode.style.paddingLeft).toBe('0px');

    rootNode.querySelector('.corpus-doc-tree-label').click();
    await flushPromises();

    const childNode = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="docs/guide.md"]',
    );
    expect(childNode.style.paddingLeft).toBe('16px');
  });

  it('lazy-loads children when expanding a directory', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const dirLabel = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="docs"] .corpus-doc-tree-label',
    );
    expect(dirLabel).not.toBeNull();
    dirLabel.click();
    await flushPromises();

    expect(api.fetchKbList).toHaveBeenCalledWith('owner/repo', 'docs', 'flat');
    expect(
      container.querySelector('.corpus-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).not.toBeNull();
  });

  it('does not refetch when collapsing and re-expanding a loaded directory', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    clickDirLabel(container, 'docs');
    await flushPromises();
    clickDirLabel(container, 'docs');
    await flushPromises();
    clickDirLabel(container, 'docs');
    await flushPromises();

    const docsCalls = api.fetchKbList.mock.calls.filter(([, p]) => p === 'docs');
    expect(docsCalls).toHaveLength(1);
  });

  it('collapses directory when clicking folder label again', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    clickDirLabel(container, 'docs');
    await flushPromises();
    expect(
      container.querySelector('.corpus-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).not.toBeNull();

    clickDirLabel(container, 'docs');
    await flushPromises();
    expect(
      container.querySelector('.corpus-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).toBeNull();
  });

  it('expands directory in tree when clicking a folder label', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const dirRow = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="docs"] .corpus-doc-tree-label',
    );
    dirRow.click();
    await flushPromises();

    expect(
      container.querySelector('.corpus-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).not.toBeNull();
    expect(container.querySelectorAll('.corpus-doc-main-row')).toHaveLength(0);
  });

  it('opens file in reader and syncs hash without router navigate', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);
    const replaceState = vi.spyOn(history, 'replaceState');

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const fileRow = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="readme.md"] .corpus-doc-tree-label',
    );
    expect(fileRow).not.toBeNull();
    fileRow.click();
    await flushPromises();

    expect(navigate).not.toHaveBeenCalled();
    expect(mountKbReader).toHaveBeenCalledWith(
      container.querySelector('.corpus-doc-reader-pane'),
      expect.objectContaining({ repo: 'owner/repo', path: 'readme.md' }),
    );
    expect(replaceState).toHaveBeenCalled();
    replaceState.mockRestore();
  });

  it('passes fallback GitHub blob URL to reader', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const fileRow = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="readme.md"] .corpus-doc-tree-label',
    );
    expect(fileRow).not.toBeNull();
    fileRow.click();
    await flushPromises();

    expect(mountKbReader).toHaveBeenCalledWith(
      container.querySelector('.corpus-doc-reader-pane'),
      expect.objectContaining({
        repo: 'owner/repo',
        path: 'readme.md',
        url: 'https://github.com/owner/repo/blob/main/readme.md',
      }),
    );
  });

  it('clicking a file keeps expanded directories visible', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    clickDirLabel(container, 'docs');
    await flushPromises();

    const fileRow = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="docs/guide.md"] .corpus-doc-tree-label',
    );
    fileRow.click();
    await flushPromises();

    expect(
      container.querySelector('.corpus-doc-tree-node[data-relative-path="docs/nested"]'),
    ).not.toBeNull();
    expect(mountKbReader).toHaveBeenCalled();
    expect(navigate).not.toHaveBeenCalled();
  });

  it('navigateToPath keeps expanded directories when switching files', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    const handle = mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    clickDirLabel(container, 'docs');
    await flushPromises();
    expect(
      container.querySelector('.corpus-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).not.toBeNull();

    await handle.navigateToPath('docs/guide.md');
    await flushPromises();

    expect(
      container.querySelector('.corpus-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).not.toBeNull();
    expect(
      container.querySelector('.corpus-doc-tree-node[data-relative-path="docs"] .corpus-doc-tree-label.selected'),
    ).toBeNull();
    const guideLabel = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="docs/guide.md"] .corpus-doc-tree-label',
    );
    expect(guideLabel?.classList.contains('selected')).toBe(true);
    expect(mountKbReader).toHaveBeenCalled();
  });

  it('shows placeholder when repo root is empty', async () => {
    api.fetchKbList.mockResolvedValue([]);

    mountCorpusDocList(container, { repo: 'owner/empty', navigate });
    await flushPromises();

    expect(container.textContent).toContain('Repository is empty');
    expect(container.querySelector('.corpus-doc-empty')).not.toBeNull();
  });

  it('allows navigating to a single file at repo root', async () => {
    api.fetchKbList.mockResolvedValue([
      { name: 'only.md', relative_path: 'only.md', is_dir: false },
    ]);

    mountCorpusDocList(container, { repo: 'owner/single', navigate });
    await flushPromises();

    const fileRow = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="only.md"] .corpus-doc-tree-label',
    );
    fileRow.click();
    await flushPromises();

    expect(navigate).not.toHaveBeenCalled();
    expect(mountKbReader).toHaveBeenCalledWith(
      container.querySelector('.corpus-doc-reader-pane'),
      expect.objectContaining({ repo: 'owner/single', path: 'only.md' }),
    );
  });

  it('shows error state when fetchKbList rejects', async () => {
    api.fetchKbList.mockRejectedValue(new Error('repo not cloned'));

    mountCorpusDocList(container, { repo: 'owner/missing', navigate });
    await flushPromises();

    expect(container.querySelector('.corpus-doc-error')).not.toBeNull();
    expect(container.textContent).toMatch(/repo not cloned|加载失败/);
  });

  it('does not render duplicate sidebar back button', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    expect(container.querySelector('.corpus-nav-back')).toBeNull();
    expect(container.querySelector('.list-select-trigger')).not.toBeNull();
    expect(container.querySelector('.corpus-doc-sync-panel')).toBeNull();
  });

  it('mount attaches corpus sidebar resizer', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const resizer = container.querySelector('.corpus-sidebar-resizer.sidebar-resizer');
    expect(resizer).not.toBeNull();
    expect(resizer?.getAttribute('aria-label')).toBe('Resize knowledge tree');
  });

  it('repo select navigates to chosen repository', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();
    await flushPromises();

    const trigger = container.querySelector('.list-select-trigger');
    expect(trigger).not.toBeNull();
    trigger.getBoundingClientRect = () => ({
      top: 100,
      bottom: 136,
      left: 10,
      width: 200,
      right: 210,
      height: 36,
      x: 10,
      y: 100,
      toJSON: () => ({}),
    });
    trigger.click();

    const menu = document.querySelector('.list-select-menu');
    expect(menu).not.toBeNull();
    expect(menu.style.maxHeight).toBe('');
    const labels = Array.from(menu.querySelectorAll('.list-select-option')).map((o) => o.textContent);
    expect(labels).toEqual(['repo · 5', 'other · 12']);
    expect(trigger.textContent).toMatch(/repo · 5/);

    menu.querySelector('[data-value="owner/other"]').click();
    expect(navigate).toHaveBeenCalledWith('#/corpus/' + encodeURIComponent('owner/other'));
    expect(document.querySelector('.list-select-menu')).toBeNull();
  });

  it('auto-selects first repo when route has no repo', async () => {
    mountCorpusDocList(container, { repo: '', navigate });
    await flushPromises();

    expect(navigate).toHaveBeenCalledWith('#/corpus/' + encodeURIComponent('owner/repo'));
  });

  it('returns cleanup that clears container and unmounts reader', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);
    const unmountReader = vi.fn();
    mountKbReader.mockResolvedValue({ unmount: unmountReader });

    const cleanup = mountCorpusDocList(container, {
      repo: 'owner/repo',
      navigate,
      initialPath: 'readme.md',
    });
    await flushPromises();

    expect(typeof cleanup).toBe('function');
    cleanup();
    expect(unmountReader).toHaveBeenCalled();
    expect(container.innerHTML).toBe('');
  });

  it('mounts reader when initialPath is provided', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountCorpusDocList(container, {
      repo: 'owner/repo',
      navigate,
      initialPath: 'docs/guide.md',
    });
    await flushPromises();

    expect(mountKbReader).toHaveBeenCalledWith(
      container.querySelector('.corpus-doc-reader-pane'),
      expect.objectContaining({ repo: 'owner/repo', path: 'docs/guide.md' }),
    );
  });

  it('reloads tree when kb:hide-pattern-changed fires', async () => {
    localStorage.setItem('kb_hide_pattern', '\\.xxx$');
    api.fetchKbList.mockResolvedValue([
      { name: 'noise.xxx', relative_path: 'noise.xxx', is_dir: false },
      { name: 'readme.md', relative_path: 'readme.md', is_dir: false },
    ]);

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    expect(container.textContent).not.toMatch(/noise\.xxx/);
    expect(container.textContent).toMatch(/readme\.md/);

    api.fetchKbList.mockClear();
    api.fetchKbList.mockResolvedValue([
      { name: 'noise.xxx', relative_path: 'noise.xxx', is_dir: false },
      { name: 'readme.md', relative_path: 'readme.md', is_dir: false },
    ]);
    localStorage.removeItem('kb_hide_pattern');
    window.dispatchEvent(new CustomEvent('kb:hide-pattern-changed'));
    await flushPromises();

    expect(api.fetchKbList).toHaveBeenCalledWith('owner/repo', '', 'flat');
    expect(container.textContent).toMatch(/noise\.xxx/);
  });

  it('removes kb:hide-pattern-changed listener on unmount', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);
    const addSpy = vi.spyOn(window, 'addEventListener');
    const removeSpy = vi.spyOn(window, 'removeEventListener');

    const cleanup = mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const addCalls = addSpy.mock.calls.filter(([evt]) => evt === 'kb:hide-pattern-changed');
    expect(addCalls.length).toBeGreaterThan(0);

    cleanup();

    const removeCalls = removeSpy.mock.calls.filter(([evt]) => evt === 'kb:hide-pattern-changed');
    expect(removeCalls.length).toBeGreaterThan(0);

    addSpy.mockRestore();
    removeSpy.mockRestore();
  });
});

describe('corpus doc route shell integration', () => {
  it('main.js mounts corpus doc list on corpus-doc route instead of redirect stub', () => {
    expect(mainJs).toMatch(/mountCorpusDocList/);
    expect(mainJs).not.toMatch(/'corpus-doc': redirectToWorkbench/);
    expect(mainJs).toMatch(/corpus-doc-view|mountCorpusDocRoute/);
  });
});
