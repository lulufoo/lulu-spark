import { workbenchSkillsContent } from './skills-content.ts';
import { renderToHtml } from '../island.ts';

type SkillItem = { cmd: string; name: string; desc?: string };

function normalizeSkill(item: string | SkillItem): SkillItem {
  return typeof item === 'string'
    ? { cmd: item, name: item, desc: 'Click to copy' }
    : item;
}

function SkillTable({ items }: { items: SkillItem[] }) {
  return (
    <table className="skill-table">
      <tbody>
        {items.map((skill) => {
          const tip = skill.desc != null && skill.desc !== '' ? skill.desc : 'Click to copy';
          return (
            <tr className="skill-row" key={skill.cmd}>
              <td className="skill-name">{skill.name}</td>
              <td className="skill-cmd" data-copy={skill.cmd} title={tip}>
                <code>{skill.cmd}</code>
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
  document.getElementById('skills-dialog-title')!.textContent = data.title;
  const items = data.groups.flatMap((g: { items: Array<string | SkillItem> }) => g.items).map(normalizeSkill);
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

  document.getElementById('skills-dialog')!.classList.add('open');
}

export function _closeSkillsDialog() {
  document.getElementById('skills-dialog')!.classList.remove('open');
}

document.getElementById('btn-skill-workbench')!.addEventListener('click', () => {
  document.getElementById('skills-menu-dropdown')?.classList.remove('open');
  _openSkillsDialog();
});

document.getElementById('btn-skills-dialog-close')!.addEventListener('click', _closeSkillsDialog);
document.getElementById('skills-dialog')!.addEventListener('click', (e) => {
  if (e.target === document.getElementById('skills-dialog')) _closeSkillsDialog();
});
