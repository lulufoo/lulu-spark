import { createModuleStore } from '../../shared/module-store.ts';

export type KnowledgeTreeMenuSnap = {
  open: boolean;
  x: number;
  y: number;
  path: string;
  isDir: boolean;
};

export const knowledgeTreeMenuStore = createModuleStore<KnowledgeTreeMenuSnap>({
  open: false,
  x: 0,
  y: 0,
  path: '',
  isDir: false,
});

export type KnowledgeTreeDeleteSnap = {
  open: boolean;
  path: string;
  name: string;
  isDir: boolean;
};

export const knowledgeTreeDeleteStore = createModuleStore<KnowledgeTreeDeleteSnap>({
  open: false,
  path: '',
  name: '',
  isDir: false,
});

export function openKnowledgeTreeMenu(snap: Omit<KnowledgeTreeMenuSnap, 'open'>) {
  knowledgeTreeMenuStore.set({ ...snap, open: true });
}

export function closeKnowledgeTreeMenu() {
  knowledgeTreeMenuStore.set({ ...knowledgeTreeMenuStore.getSnapshot(), open: false });
}

export function openKnowledgeTreeDelete(snap: Omit<KnowledgeTreeDeleteSnap, 'open'>) {
  closeKnowledgeTreeMenu();
  knowledgeTreeDeleteStore.set({ ...snap, open: true });
}

export function closeKnowledgeTreeDelete() {
  knowledgeTreeDeleteStore.set({ ...knowledgeTreeDeleteStore.getSnapshot(), open: false });
}
