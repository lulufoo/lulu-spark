// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

vi.mock('../frontend/js/api.js', () => ({
  fetchKbList: vi.fn(),
}));

vi.mock('../frontend/js/components/viewer.js', () => ({
  openKbDoc: vi.fn(),
}));

import * as api from '../frontend/js/api.js';
import { openKbDoc } from '../frontend/js/components/viewer.js';
import {
  buildTreeNodes,
  mountCorpusDocList,
} from '../frontend/js/components/corpus-doc-list.js';

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

describe('buildTreeNodes', () => {
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
});

describe('mountCorpusDocList', () => {
  let container;
  let navigate;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    navigate = vi.fn();
    vi.clearAllMocks();
  });

  afterEach(() => {
    container.remove();
  });

  async function flushPromises() {
    await Promise.resolve();
    await Promise.resolve();
  }

  it('fetches root flat list on mount', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    expect(api.fetchKbList).toHaveBeenCalledWith('owner/repo', '', 'flat');
    expect(api.fetchKbList).toHaveBeenCalledTimes(1);
  });

  it('renders dual-column sidebar and main layout', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    expect(container.querySelector('.corpus-doc-layout')).not.toBeNull();
    expect(container.querySelector('.corpus-doc-sidebar')).not.toBeNull();
    expect(container.querySelector('.corpus-doc-main')).not.toBeNull();
  });

  it('lazy-loads children when expanding a directory', async () => {
    api.fetchKbList.mockImplementation(async (_repo, path) => {
      if (path === '') return sampleRootEntries;
      if (path === 'docs') return sampleDocsEntries;
      return [];
    });

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const expandBtn = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="docs"] .corpus-doc-tree-expand',
    );
    expect(expandBtn).not.toBeNull();
    expandBtn.click();
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

    const expandBtn = container.querySelector(
      '.corpus-doc-tree-node[data-relative-path="docs"] .corpus-doc-tree-expand',
    );
    expandBtn.click();
    await flushPromises();
    expandBtn.click();
    await flushPromises();
    expandBtn.click();
    await flushPromises();

    const docsCalls = api.fetchKbList.mock.calls.filter(([, p]) => p === 'docs');
    expect(docsCalls).toHaveLength(1);
  });

  it('shows selected directory children in main list', async () => {
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

    const mainRows = container.querySelectorAll('.corpus-doc-main-row');
    expect(mainRows.length).toBe(2);
    expect(container.textContent).toContain('guide.md');
  });

  it('opens kb modal when clicking a markdown file in main', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);
    openKbDoc.mockResolvedValue(undefined);

    mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    const fileRow = container.querySelector('.corpus-doc-main-row[data-relative-path="readme.md"]');
    expect(fileRow).not.toBeNull();
    fileRow.click();
    await flushPromises();

    expect(openKbDoc).toHaveBeenCalledWith(
      expect.objectContaining({ repo: 'owner/repo', path: 'readme.md' }),
    );
  });

  it('shows placeholder when repo root is empty', async () => {
    api.fetchKbList.mockResolvedValue([]);

    mountCorpusDocList(container, { repo: 'owner/empty', navigate });
    await flushPromises();

    expect(container.textContent).toMatch(/空|暂无/);
    expect(container.querySelector('.corpus-doc-empty')).not.toBeNull();
  });

  it('allows opening a single file at repo root', async () => {
    api.fetchKbList.mockResolvedValue([
      { name: 'only.md', relative_path: 'only.md', is_dir: false },
    ]);
    openKbDoc.mockResolvedValue(undefined);

    mountCorpusDocList(container, { repo: 'owner/single', navigate });
    await flushPromises();

    const fileRow = container.querySelector('.corpus-doc-main-row[data-relative-path="only.md"]');
    fileRow.click();
    await flushPromises();

    expect(openKbDoc).toHaveBeenCalledWith(
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

  it('returns cleanup that clears container', async () => {
    api.fetchKbList.mockResolvedValue(sampleRootEntries);

    const cleanup = mountCorpusDocList(container, { repo: 'owner/repo', navigate });
    await flushPromises();

    expect(typeof cleanup).toBe('function');
    cleanup();
    expect(container.innerHTML).toBe('');
  });
});

describe('corpus doc route shell integration', () => {
  it('main.js mounts corpus doc list on corpus-doc route instead of redirect stub', () => {
    expect(mainJs).toMatch(/mountCorpusDocList/);
    expect(mainJs).not.toMatch(/'corpus-doc': redirectToWorkbench/);
    expect(mainJs).toMatch(/corpus-doc-view|mountCorpusDocRoute/);
  });
});
