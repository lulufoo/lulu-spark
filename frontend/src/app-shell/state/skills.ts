import { createModuleStore } from '../../shared/module-store.ts';

export type SkillItem = { cmd: string; name: string; desc?: string };
export type SkillsState = { open: boolean; title: string; items: SkillItem[] };

export const skillsStore = createModuleStore<SkillsState>({ open: false, title: '', items: [] });
