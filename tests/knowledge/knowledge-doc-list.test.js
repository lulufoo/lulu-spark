// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchKbList: vi.fn(),
  fetchKbDocCount: vi.fn(),
  fetchSedimentKbRepos: vi.fn(),
}));

vi.mock('../../frontend/src/knowledge/viewer.ts', () => ({
  mountKbReader: vi.fn(),
  ReaderShell: () => null,
}));

import * as api from '../../frontend/src/host/api.ts';
import { mountKbReader } from '../../frontend/src/knowledge/viewer.ts';
import { getKbHidePattern } from '../../frontend/src/knowledge/state/hide-pattern.ts';
import {
  buildTreeNodes,
  buildRepoPickerOptions,
  formatRepoMenuLabel,
  mountKnowledgeDocList,
} from '../../frontend/src/knowledge/page.tsx';
import { positionFloatingListMenu } from '../../frontend/src/shared/floating-list-menu.ts';
import { readMainSource } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const mainJs = readMainSource();

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
    `.knowledge-doc-tree-node[data-relative-path="${relativePath}"] .knowledge-doc-tree-label`,
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

describe('mountKnowledgeDocList', () => {
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

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    expect(api.fetchKbList).toHaveBeenCalledWith('owner/repo', '', 'flat');
    expect(api.fetchKbList).toHaveBeenCalledTimes(1);
  });

  it('renders sidebar and reader pane layout', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    expect(container.querySelector('.knowledge-doc-layout')).not.toBeNull();
    expect(container.querySelector('.knowledge-doc-sidebar')).not.toBeNull();
    expect(container.querySelector('.knowledge-doc-reader-pane')).not.toBeNull();
    expect(container.querySelector('.knowledge-doc-main')).toBeNull();
    expect(container.querySelector('.knowledge-doc-main-list')).toBeNull();
  });

  it('applies depth * 16px padding to tree nodes', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const rootNode = container.querySelector(
      '.knowledge-doc-tree-node[data-relative-path="docs"]',
    );
    expect(rootNode.style.paddingLeft).toBe('0px');

    rootNode.querySelector('.knowledge-doc-tree-label').click();
    await flushPromises();

    const childNode = container.querySelector(
      '.knowledge-doc-tree-node[data-relative-path="docs/guide.md"]',
    );
    expect(childNode.style.paddingLeft).toBe('16px');
  });

  it('lazy-loads children when expanding a directory', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const dirLabel = container.querySelector(
      '.knowledge-doc-tree-node[data-relative-path="docs"] .knowledge-doc-tree-label',
    );
    expect(dirLabel).not.toBeNull();
    dirLabel.click();
    await flushPromises();

    expect(api.fetchKbList).toHaveBeenCalledWith('owner/repo', 'docs', 'flat');
    expect(
      container.querySelector('.knowledge-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).not.toBeNull();
  });

  it('does not refetch when collapsing and re-expanding a loaded directory', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
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

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    clickDirLabel(container, 'docs');
    await flushPromises();
    expect(
      container.querySelector('.knowledge-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).not.toBeNull();

    clickDirLabel(container, 'docs');
    await flushPromises();
    expect(
      container.querySelector('.knowledge-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).toBeNull();
  });

  it('expands directory in tree when clicking a folder label', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const dirRow = container.querySelector(
      '.knowledge-doc-tree-node[data-relative-path="docs"] .knowledge-doc-tree-label',
    );
    dirRow.click();
    await flushPromises();

    expect(
      container.querySelector('.knowledge-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).not.toBeNull();
    expect(container.querySelectorAll('.knowledge-doc-main-row')).toHaveLength(0);
  });

  it('opens a second file in the reader after the first', async () => {
    api.fetchKbList.mockResolvedValue([
      { name: 'alpha.md', relative_path: 'alpha.md', is_dir: false },
      { name: 'beta.md', relative_path: 'beta.md', is_dir: false },
    ]);

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const clickFile = (relativePath) => {
      const label = container.querySelector(
        `.knowledge-doc-tree-node[data-relative-path="${relativePath}"] .knowledge-doc-tree-label`,
      );
      expect(label).not.toBeNull();
      label.click();
    };

    clickFile('alpha.md');
    await flushPromises();
    clickFile('beta.md');
    await flushPromises();

    expect(mountKbReader).toHaveBeenCalledTimes(2);
    expect(mountKbReader).toHaveBeenNthCalledWith(
      1,
      container.querySelector('.knowledge-doc-reader-pane'),
      expect.objectContaining({ repo: 'owner/repo', path: 'alpha.md' }),
    );
    expect(mountKbReader).toHaveBeenNthCalledWith(
      2,
      container.querySelector('.knowledge-doc-reader-pane'),
      expect.objectContaining({ repo: 'owner/repo', path: 'beta.md' }),
    );
    expect(
      container
        .querySelector('.knowledge-doc-tree-node[data-relative-path="beta.md"] .knowledge-doc-tree-label')
        ?.classList.contains('selected'),
    ).toBe(true);
  });

  it('opens file in reader and syncs hash without router navigate', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);
    const replaceState = vi.spyOn(history, 'replaceState');

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const fileRow = container.querySelector(
      '.knowledge-doc-tree-node[data-relative-path="readme.md"] .knowledge-doc-tree-label',
    );
    expect(fileRow).not.toBeNull();
    fileRow.click();
    await flushPromises();

    expect(navigate).not.toHaveBeenCalled();
    expect(mountKbReader).toHaveBeenCalledWith(
      container.querySelector('.knowledge-doc-reader-pane'),
      expect.objectContaining({ repo: 'owner/repo', path: 'readme.md' }),
    );
    expect(replaceState).toHaveBeenCalled();
    replaceState.mockRestore();
  });

  it('passes fallback GitHub blob URL to reader', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const fileRow = container.querySelector(
      '.knowledge-doc-tree-node[data-relative-path="readme.md"] .knowledge-doc-tree-label',
    );
    expect(fileRow).not.toBeNull();
    fileRow.click();
    await flushPromises();

    expect(mountKbReader).toHaveBeenCalledWith(
      container.querySelector('.knowledge-doc-reader-pane'),
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

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    clickDirLabel(container, 'docs');
    await flushPromises();

    const fileRow = container.querySelector(
      '.knowledge-doc-tree-node[data-relative-path="docs/guide.md"] .knowledge-doc-tree-label',
    );
    fileRow.click();
    await flushPromises();

    expect(
      container.querySelector('.knowledge-doc-tree-node[data-relative-path="docs/nested"]'),
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

    const handle = mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    clickDirLabel(container, 'docs');
    await flushPromises();
    expect(
      container.querySelector('.knowledge-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).not.toBeNull();

    await handle.navigateToPath('docs/guide.md');
    await flushPromises();

    expect(
      container.querySelector('.knowledge-doc-tree-node[data-relative-path="docs/guide.md"]'),
    ).not.toBeNull();
    expect(
      container.querySelector('.knowledge-doc-tree-node[data-relative-path="docs"] .knowledge-doc-tree-label.selected'),
    ).toBeNull();
    const guideLabel = container.querySelector(
      '.knowledge-doc-tree-node[data-relative-path="docs/guide.md"] .knowledge-doc-tree-label',
    );
    expect(guideLabel?.classList.contains('selected')).toBe(true);
    expect(mountKbReader).toHaveBeenCalled();
  });

  it('shows placeholder when repo root is empty', async () => {
    api.fetchKbList.mockResolvedValue([]);

    mountKnowledgeDocList(container, { repo: 'owner/empty', navigate });
    await flushPromises();

    expect(container.textContent).toContain('Repository is empty');
    expect(container.querySelector('.knowledge-doc-empty')).not.toBeNull();
  });

  it('allows navigating to a single file at repo root', async () => {
    api.fetchKbList.mockResolvedValue([
      { name: 'only.md', relative_path: 'only.md', is_dir: false },
    ]);

    mountKnowledgeDocList(container, { repo: 'owner/single', navigate });
    await flushPromises();

    const fileRow = container.querySelector(
      '.knowledge-doc-tree-node[data-relative-path="only.md"] .knowledge-doc-tree-label',
    );
    fileRow.click();
    await flushPromises();

    expect(navigate).not.toHaveBeenCalled();
    expect(mountKbReader).toHaveBeenCalledWith(
      container.querySelector('.knowledge-doc-reader-pane'),
      expect.objectContaining({ repo: 'owner/single', path: 'only.md' }),
    );
  });

  it('shows error state when fetchKbList rejects', async () => {
    api.fetchKbList.mockRejectedValue(new Error('repo not cloned'));

    mountKnowledgeDocList(container, { repo: 'owner/missing', navigate });
    await flushPromises();

    expect(container.querySelector('.knowledge-doc-error')).not.toBeNull();
    expect(container.textContent).toMatch(/repo not cloned|加载失败/);
  });

  it('does not render duplicate sidebar back button', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    expect(container.querySelector('.corpus-nav-back')).toBeNull();
    expect(container.querySelector('.list-select-trigger')).not.toBeNull();
    expect(container.querySelector('.knowledge-doc-sync-panel')).toBeNull();
  });

  it('mount attaches knowledge sidebar resizer', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const resizer = container.querySelector('.knowledge-sidebar-resizer.sidebar-resizer');
    expect(resizer).not.toBeNull();
    expect(resizer?.getAttribute('aria-label')).toBe('Resize knowledge tree');
  });

  it('repo select navigates to chosen repository', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
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
    expect(navigate).toHaveBeenCalledWith('#/knowledge/' + encodeURIComponent('owner/other'));
    expect(document.querySelector('.list-select-menu')).toBeNull();
  });

  it('auto-selects first repo when route has no repo', async () => {
    mountKnowledgeDocList(container, { repo: '', navigate });
    await flushPromises();

    expect(navigate).toHaveBeenCalledWith('#/knowledge/' + encodeURIComponent('owner/repo'));
  });

  it('returns cleanup that clears container and unmounts reader', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);
    const unmountReader = vi.fn();
    mountKbReader.mockResolvedValue({ unmount: unmountReader });

    const cleanup = mountKnowledgeDocList(container, {
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

    mountKnowledgeDocList(container, {
      repo: 'owner/repo',
      navigate,
      initialPath: 'docs/guide.md',
    });
    await flushPromises();

    expect(mountKbReader).toHaveBeenCalledWith(
      container.querySelector('.knowledge-doc-reader-pane'),
      expect.objectContaining({ repo: 'owner/repo', path: 'docs/guide.md' }),
    );
  });

  it('reloads tree when kb:hide-pattern-changed fires', async () => {
    localStorage.setItem('kb_hide_pattern', '\\.xxx$');
    api.fetchKbList.mockResolvedValue([
      { name: 'noise.xxx', relative_path: 'noise.xxx', is_dir: false },
      { name: 'readme.md', relative_path: 'readme.md', is_dir: false },
    ]);

    mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
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

    const cleanup = mountKnowledgeDocList(container, { repo: 'owner/repo', navigate });
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

describe('knowledge doc route shell integration', () => {
  it('main.js paints KnowledgeDocPage on knowledge-doc instead of createRoot-mounting the slot', () => {
    expect(mainJs).toMatch(/KnowledgeDocPage/);
    expect(mainJs).not.toMatch(/'knowledge-doc': redirectToWorkbench/);
    expect(mainJs).toMatch(/knowledge-doc-view|mountKnowledgeDocRoute/);
    const body = (() => {
      const start = mainJs.indexOf('function mountKnowledgeDocRoute');
      if (start === -1) return '';
      const braceStart = mainJs.indexOf('{', start);
      let depth = 0;
      for (let i = braceStart; i < mainJs.length; i += 1) {
        if (mainJs[i] === '{') depth += 1;
        if (mainJs[i] === '}') {
          depth -= 1;
          if (depth === 0) return mainJs.slice(braceStart, i + 1);
        }
      }
      return '';
    })();
    expect(body).not.toMatch(/mountKnowledgeDocList/);
  });

  it('KnowledgeDocPage applies path in-place and remounts when repo changes', () => {
    const pageSrc = readFileSync(
      join(fixtureRoot, 'frontend/src/knowledge/page.tsx'),
      'utf8',
    );
    expect(pageSrc).toMatch(/function KnowledgeDocPage/);
    expect(pageSrc).toMatch(/navigateToPath/);
    expect(pageSrc).toMatch(/sessionRef\.current\?\.navigateToPath/);
  });
});
