import { useState, useSyncExternalStore } from 'react';
import { createModuleStore } from '../shared/module-store.ts';
import { workbenchSkillsContent } from './skills-content.ts';
import { renderToHtml } from '../island.ts';

type SkillItem = { cmd: string; name: string; desc?: string };
type SkillsState = { open: boolean; title: string; items: SkillItem[] };

const skillsStore = createModuleStore<SkillsState>({ open: false, title: '', items: [] });

function normalizeSkill(item: string | SkillItem): SkillItem {
  return typeof item === 'string'
    ? { cmd: item, name: item, desc: 'Click to copy' }
    : item;
}

function SkillTable({ items }: { items: SkillItem[] }) {
  const [copied, setCopied] = useState<string | null>(null);
  return (
    <table className="skill-table">
      <tbody>
        {items.map((skill) => {
          const tip = skill.desc != null && skill.desc !== '' ? skill.desc : 'Click to copy';
          return (
            <tr className="skill-row" key={skill.cmd}>
              <td className="skill-name">{skill.name}</td>
              <td
                className="skill-cmd"
                data-copy={skill.cmd}
                title={tip}
                onClick={() => {
                  const cmd = skill.cmd;
                  navigator.clipboard.writeText(cmd).then(() => {
                    setCopied(cmd);
                    setTimeout(() => { setCopied(null); }, 1200);
                  });
                }}
              >
                <code>{copied === skill.cmd ? 'Copied' : skill.cmd}</code>
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function paintSkillCmd(el: HTMLElement, text: string) {
  el.innerHTML = renderToHtml(<code>{text}</code>);
}

export function _openSkillsDialog() {
  const data = workbenchSkillsContent;
  if (!data) return;
  const items = data.groups.flatMap((g: { items: Array<string | SkillItem> }) => g.items).map(normalizeSkill);
  document.getElementById('skills-dialog-title')!.textContent = data.title;
  document.getElementById('skills-dialog-body')!.innerHTML = renderToHtml(
    <SkillTable items={items} />,
  );

  document.getElementById('skills-dialog-body')!.querySelectorAll('.skill-cmd[data-copy]').forEach((node) => {
    const el = node as HTMLElement;
    el.addEventListener('click', () => {
      const cmd = el.dataset.copy as string;
      navigator.clipboard.writeText(cmd).then(() => {
        paintSkillCmd(el, 'Copied');
        setTimeout(() => { paintSkillCmd(el, cmd); }, 1200);
      });
    });
  });

  skillsStore.set({ open: true, title: data.title, items });
  document.getElementById('skills-dialog')!.classList.add('open');
}

export function _closeSkillsDialog() {
  skillsStore.set((s) => ({ ...s, open: false }));
  document.getElementById('skills-dialog')!.classList.remove('open');
}

export function SkillsDialog() {
  const { open, title, items } = useSyncExternalStore(skillsStore.subscribe, skillsStore.getSnapshot);
  return (
    <div
      id="skills-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) _closeSkillsDialog();
      }}
    >
      <div id="skills-dialog-box">
        <div id="skills-dialog-header">
          <h3 id="skills-dialog-title">{title}</h3>
          <button
            id="btn-skills-dialog-close"
            type="button"
            className="md-header-btn"
            onClick={() => _closeSkillsDialog()}
          >
            ✕ Close
          </button>
        </div>
        <div id="skills-dialog-body">
          <SkillTable items={items} />
        </div>
      </div>
    </div>
  );
}
