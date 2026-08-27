// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

vi.mock('../frontend/js/host/api.js', () => ({
  fetchSedimentKbCategories: vi.fn(),
  fetchSedimentKbRepos: vi.fn(),
}));

import * as api from '../frontend/js/host/api.js';
import { mountCorpusPicker, filterReposByCategory } from '../frontend/js/corpus/corpus-picker.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const indexHtml = readFileSync(join(fixtureRoot, 'frontend/index.html'), 'utf8');
const mainJs = readFileSync(join(fixtureRoot, 'frontend/js/main.js'), 'utf8');

const sampleCategories = [
  { id: 'cat-a', name: 'Category A' },
  { id: 'cat-b', name: 'Category B' },
];

const sampleRepos = [
  { full_name: 'owner/repo-a', category_id: 'cat-a', category_name: 'Category A' },
  { full_name: 'owner/repo-b', category_id: 'cat-b', category_name: 'Category B' },
  { full_name: 'owner/repo-c', category_id: 'cat-a', category_name: 'Category A' },
];

describe('filterReposByCategory', () => {
  it('returns all repos when categoryId is null or empty', () => {
    expect(filterReposByCategory(sampleRepos, null)).toEqual(sampleRepos);
    expect(filterReposByCategory(sampleRepos, '')).toEqual(sampleRepos);
  });

  it('filters repos by category_id', () => {
    expect(filterReposByCategory(sampleRepos, 'cat-a')).toEqual([
      sampleRepos[0],
      sampleRepos[2],
    ]);
    expect(filterReposByCategory(sampleRepos, 'cat-b')).toEqual([sampleRepos[1]]);
  });
});

describe('mountCorpusPicker', () => {
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

  it('fetches categories and repos in parallel on mount', async () => {
    api.fetchSedimentKbCategories.mockResolvedValue({ categories: sampleCategories });
    api.fetchSedimentKbRepos.mockResolvedValue({ repos: sampleRepos });

    mountCorpusPicker(container, { navigate });
    await flushPromises();

    expect(api.fetchSedimentKbCategories).toHaveBeenCalledTimes(1);
    expect(api.fetchSedimentKbRepos).toHaveBeenCalledTimes(1);
  });

  it('renders category filter and repo list; click navigates to corpus hash', async () => {
    api.fetchSedimentKbCategories.mockResolvedValue({ categories: sampleCategories });
    api.fetchSedimentKbRepos.mockResolvedValue({ repos: sampleRepos });

    mountCorpusPicker(container, { navigate });
    await flushPromises();

    const select = container.querySelector('.corpus-picker-category-select');
    expect(select).not.toBeNull();
    expect(select.options[0].textContent).toMatch(/全部/);

    const rows = container.querySelectorAll('.corpus-picker-repo-row');
    expect(rows.length).toBe(3);

    rows[0].click();
    expect(navigate).toHaveBeenCalledWith('#/corpus/' + encodeURIComponent('owner/repo-a'));
  });

  it('filters repos when a category is selected', async () => {
    api.fetchSedimentKbCategories.mockResolvedValue({ categories: sampleCategories });
    api.fetchSedimentKbRepos.mockResolvedValue({ repos: sampleRepos });

    mountCorpusPicker(container, { navigate });
    await flushPromises();

    const select = container.querySelector('.corpus-picker-category-select');
    select.value = 'cat-b';
    select.dispatchEvent(new Event('change'));
    await flushPromises();

    const rows = container.querySelectorAll('.corpus-picker-repo-row');
    expect(rows.length).toBe(1);
    expect(rows[0].textContent).toContain('owner/repo-b');
  });

  it('shows empty placeholder when categories and repos are both empty', async () => {
    api.fetchSedimentKbCategories.mockResolvedValue({ categories: [] });
    api.fetchSedimentKbRepos.mockResolvedValue({ repos: [] });

    mountCorpusPicker(container, { navigate });
    await flushPromises();

    expect(container.textContent).toMatch(/暂无沉淀知识库/);
  });

  it('shows no-match placeholder when filter yields zero repos', async () => {
    api.fetchSedimentKbCategories.mockResolvedValue({ categories: sampleCategories });
    api.fetchSedimentKbRepos.mockResolvedValue({
      repos: [{ full_name: 'owner/only-a', category_id: 'cat-a' }],
    });

    mountCorpusPicker(container, { navigate });
    await flushPromises();

    const select = container.querySelector('.corpus-picker-category-select');
    select.value = 'cat-b';
    select.dispatchEvent(new Event('change'));

    expect(container.textContent).toMatch(/无匹配仓库/);
  });

  it('shows readable error when fetch rejects', async () => {
    api.fetchSedimentKbCategories.mockRejectedValue(new Error('network down'));
    api.fetchSedimentKbRepos.mockResolvedValue({ repos: sampleRepos });

    mountCorpusPicker(container, { navigate });
    await flushPromises();

    expect(container.querySelector('.corpus-picker-error')).not.toBeNull();
    expect(container.textContent).toMatch(/network down|加载失败/);
  });

  it('back button navigates to home', async () => {
    api.fetchSedimentKbCategories.mockResolvedValue({ categories: sampleCategories });
    api.fetchSedimentKbRepos.mockResolvedValue({ repos: sampleRepos });

    mountCorpusPicker(container, { navigate });
    await flushPromises();

    container.querySelector('.corpus-nav-back')?.click();
    expect(navigate).toHaveBeenCalledWith('#/home');
  });

  it('returns cleanup that removes listeners', async () => {
    api.fetchSedimentKbCategories.mockResolvedValue({ categories: sampleCategories });
    api.fetchSedimentKbRepos.mockResolvedValue({ repos: sampleRepos });

    const cleanup = mountCorpusPicker(container, { navigate });
    await flushPromises();

    expect(typeof cleanup).toBe('function');
    cleanup();
    expect(container.innerHTML).toBe('');
  });
});

describe('corpus pick shell integration', () => {
  it('header has temporary entry linking to #/corpus/pick', () => {
    expect(indexHtml).toMatch(/href="#\/corpus\/pick"/);
    expect(indexHtml).toContain('id="btn-corpus-pick"');
    expect(indexHtml).toContain('id="btn-nav-home"');
    expect(indexHtml).toContain('id="settings-panel-knowledge"');
  });

  it('main.js mounts corpus picker on corpus-pick route instead of redirect stub', () => {
    expect(mainJs).toMatch(/mountCorpusPicker/);
    expect(mainJs).not.toMatch(/'corpus-pick': redirectToWorkbench/);
    expect(mainJs).toMatch(/corpus-pick-view/);
  });
});
