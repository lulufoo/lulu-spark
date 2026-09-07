export const TREE_COLLAPSE_STORAGE_KEY = 'knowledge_tree_collapsed';

function storage(): Storage | null {
  try {
    const store = globalThis.localStorage;
    if (!store || typeof store.getItem !== 'function' || typeof store.setItem !== 'function') {
      return null;
    }
    return store;
  } catch {
    return null;
  }
}

export function readKnowledgeTreeCollapsed(): boolean {
  return storage()?.getItem(TREE_COLLAPSE_STORAGE_KEY) === '1';
}

export function persistKnowledgeTreeCollapsed(collapsed: boolean) {
  storage()?.setItem(TREE_COLLAPSE_STORAGE_KEY, collapsed ? '1' : '0');
}
