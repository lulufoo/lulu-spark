// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({
  searchKnowledge: vi.fn(),
  reindexKnowledge: vi.fn(),
  getReindexStatus: vi.fn(),
}));

vi.mock('../../frontend/js/host/api.js', () => ({
  searchKnowledge: (...args) => apiMocks.searchKnowledge(...args),
  reindexKnowledge: (...args) => apiMocks.reindexKnowledge(...args),
  getReindexStatus: (...args) => apiMocks.getReindexStatus(...args),
}));

async function loadModule() {
  vi.resetModules();
  return import('../../frontend/js/corpus/corpus-knowledge-search.js');
}

function seedPanel() {
  document.body.innerHTML = '<div id="knowledge-panel"></div>';
  return document.getElementById('knowledge-panel');
}

describe('corpus knowledge search', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    seedPanel();
    apiMocks.searchKnowledge.mockResolvedValue({ hits: [] });
    apiMocks.reindexKnowledge.mockResolvedValue({});
    apiMocks.getReindexStatus.mockResolvedValue({ status: 'running' });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('exports mountKnowledgeSearch and triggerKnowledgeSearch', async () => {
    const mod = await loadModule();
    expect(typeof mod.mountKnowledgeSearch).toBe('function');
    expect(typeof mod.triggerKnowledgeSearch).toBe('function');
  });

  it('mount builds the Related knowledge panel', async () => {
    const { mountKnowledgeSearch } = await loadModule();
    const panel = seedPanel();
    mountKnowledgeSearch(panel);
    expect(panel.querySelector('.ks-input')).toBeTruthy();
    expect(panel.querySelector('.ks-toggle')).toBeTruthy();
    expect(panel.querySelector('.ks-refresh-btn')).toBeTruthy();
    expect(panel.textContent).toContain('Related knowledge');
  });

  it('triggerKnowledgeSearch queries title plus path slug', async () => {
    const { mountKnowledgeSearch, triggerKnowledgeSearch } = await loadModule();
    const panel = seedPanel();
    mountKnowledgeSearch(panel);
    triggerKnowledgeSearch({
      title: 'Alpha',
      common_path: 'notes/20260101120000-alpha-note.md',
    });
    await Promise.resolve();
    expect(apiMocks.searchKnowledge).toHaveBeenCalledWith('Alpha alpha-note', 10);
    expect(panel.querySelector('.ks-input').value).toBe('Alpha alpha-note');
  });

  it('debounced input calls searchKnowledge(q, 10)', async () => {
    const { mountKnowledgeSearch } = await loadModule();
    const panel = seedPanel();
    mountKnowledgeSearch(panel);
    const input = panel.querySelector('.ks-input');
    input.value = 'topic';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);
    expect(apiMocks.searchKnowledge).toHaveBeenCalledWith('topic', 10);
  });

  it('empty query shows Enter keywords to search', async () => {
    const { mountKnowledgeSearch } = await loadModule();
    const panel = seedPanel();
    mountKnowledgeSearch(panel);
    const input = panel.querySelector('.ks-input');
    input.value = '';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.advanceTimersByTimeAsync(300);
    expect(apiMocks.searchKnowledge).not.toHaveBeenCalled();
    expect(panel.querySelector('.ks-status-msg').textContent).toContain('Enter keywords to search');
  });

  it('renders hits from searchKnowledge', async () => {
    apiMocks.searchKnowledge.mockResolvedValue({
      hits: [{ title: 'Doc A', repo: 'owner/repo', url: 'https://example.com/a', body: 'hello' }],
    });
    const { mountKnowledgeSearch, triggerKnowledgeSearch } = await loadModule();
    const panel = seedPanel();
    mountKnowledgeSearch(panel);
    triggerKnowledgeSearch({ title: 'Doc A', common_path: 'doc-a.md' });
    await Promise.resolve();
    const hit = panel.querySelector('.ks-hit');
    expect(hit).toBeTruthy();
    expect(hit.getAttribute('href')).toBe('https://example.com/a');
    expect(hit.textContent).toContain('Doc A');
    expect(hit.textContent).toContain('repo');
  });

  it('unavailable response marks the panel', async () => {
    apiMocks.searchKnowledge.mockResolvedValue({ error: 'unavailable' });
    const { mountKnowledgeSearch, triggerKnowledgeSearch } = await loadModule();
    const panel = seedPanel();
    mountKnowledgeSearch(panel);
    triggerKnowledgeSearch({ title: 'X', common_path: 'x.md' });
    await Promise.resolve();
    expect(panel.classList.contains('ks-unavailable')).toBe(true);
  });

  it('refresh button starts reindexKnowledge', async () => {
    const { mountKnowledgeSearch } = await loadModule();
    const panel = seedPanel();
    mountKnowledgeSearch(panel);
    panel.querySelector('.ks-refresh-btn').click();
    await Promise.resolve();
    expect(apiMocks.reindexKnowledge).toHaveBeenCalledTimes(1);
    expect(panel.querySelector('.ks-sync-bar').textContent).toContain('Syncing knowledge');
  });

  it('toggle collapses the panel', async () => {
    const { mountKnowledgeSearch } = await loadModule();
    const panel = seedPanel();
    mountKnowledgeSearch(panel);
    const toggle = panel.querySelector('.ks-toggle');
    expect(panel.classList.contains('ks-collapsed')).toBe(false);
    toggle.click();
    expect(panel.classList.contains('ks-collapsed')).toBe(true);
    expect(toggle.textContent).toBe('‹');
  });
});
