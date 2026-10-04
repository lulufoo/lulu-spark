import { sparkSkillsContent } from '../state/skills-content.ts';
import { skillsStore, type SkillItem } from '../state/skills.ts';

export { skillsStore };

function normalizeSkill(item: string | SkillItem): SkillItem {
  return typeof item === 'string'
    ? { cmd: item, name: item, desc: 'Click to copy' }
    : item;
}

export function _openSkillsDialog() {
  const data = sparkSkillsContent;
  if (!data) return;
  const items = data.groups.flatMap((g: { items: Array<string | SkillItem> }) => g.items).map(normalizeSkill);
  skillsStore.set({ open: true, title: data.title, items });
  document.getElementById('skills-dialog')?.classList.add('open');
}

export function _closeSkillsDialog() {
  skillsStore.set((s) => ({ ...s, open: false }));
  document.getElementById('skills-dialog')!.classList.remove('open');
}
