// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { TREE_COLLAPSE_STORAGE_KEY } from '../../frontend/src/knowledge/state/tree-collapse.ts';
import {
  applyKnowledgeTreeCollapsed,
  attachKnowledgeTreeToggle,
  detachKnowledgeTreeToggle,
  restoreKnowledgeTreeCollapsed,
  toggleKnowledgeTreeCollapsed,
} from '../../frontend/src/knowledge/commands/tree-collapse.ts';

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

function paintLayout() {
  const layout = document.createElement('div');
  layout.className = 'knowledge-doc-layout';
  layout.innerHTML = `
    <aside class="knowledge-doc-sidebar"></aside>
    <section class="knowledge-doc-reader-pane">
      <button type="button" class="kb-btn-tree-toggle" title="Collapse directory" aria-expanded="true"><svg></svg></button>
    </section>
  `;
  document.body.appendChild(layout);
  return layout;
}

describe('knowledge tree collapse', () => {
  /** @type {HTMLElement} */
  let layout;

  beforeEach(() => {
    installLocalStorageMock();
    layout = paintLayout();
  });

  afterEach(() => {
    detachKnowledgeTreeToggle();
    layout.remove();
  });

  it('applies collapsed class and persists', () => {
    applyKnowledgeTreeCollapsed(layout, true);
    const btn = layout.querySelector('.kb-btn-tree-toggle');
    expect(layout.classList.contains('is-tree-collapsed')).toBe(true);
    expect(localStorage.getItem(TREE_COLLAPSE_STORAGE_KEY)).toBe('1');
    expect(btn?.classList.contains('is-collapsed')).toBe(true);
    expect(btn?.querySelector('svg')).not.toBeNull();
    expect(btn?.getAttribute('aria-expanded')).toBe('false');
    expect(btn?.title).toBe('Expand directory');
  });

  it('toggles directory from the header button', () => {
    const btn = layout.querySelector('.kb-btn-tree-toggle');
    toggleKnowledgeTreeCollapsed(btn);
    expect(layout.classList.contains('is-tree-collapsed')).toBe(true);
    toggleKnowledgeTreeCollapsed(btn);
    expect(layout.classList.contains('is-tree-collapsed')).toBe(false);
    expect(localStorage.getItem(TREE_COLLAPSE_STORAGE_KEY)).toBe('0');
    expect(btn?.classList.contains('is-collapsed')).toBe(false);
    expect(btn?.querySelector('svg')).not.toBeNull();
  });

  it('restores collapsed directory from localStorage', () => {
    localStorage.setItem(TREE_COLLAPSE_STORAGE_KEY, '1');
    restoreKnowledgeTreeCollapsed(layout);
    expect(layout.classList.contains('is-tree-collapsed')).toBe(true);
    expect(layout.querySelector('.kb-btn-tree-toggle')?.classList.contains('is-collapsed')).toBe(true);
  });

  it('does not replace the panel icon when toggling', () => {
    const btn = layout.querySelector('.kb-btn-tree-toggle');
    applyKnowledgeTreeCollapsed(layout, true);
    applyKnowledgeTreeCollapsed(layout, false);
    expect(btn?.querySelector('svg')).not.toBeNull();
  });

  it('attach binds click and restore', () => {
    localStorage.setItem(TREE_COLLAPSE_STORAGE_KEY, '1');
    attachKnowledgeTreeToggle(layout);
    expect(layout.classList.contains('is-tree-collapsed')).toBe(true);
    layout.querySelector('.kb-btn-tree-toggle')?.click();
    expect(layout.classList.contains('is-tree-collapsed')).toBe(false);
  });
});
