import { workbenchSkillsContent } from './skills-content.js';

function _escapeAttr(str) {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/"/g, '&quot;')
    .replace(/</g, '&lt;');
}

export function _openSkillsDialog() {
  const data = workbenchSkillsContent;
  if (!data) return;
  document.getElementById('skills-dialog-title').textContent = data.title;
  const rows = data.groups.flatMap((g) => g.items).map((item) => {
    const skill = typeof item === 'string'
      ? { cmd: item, name: item, desc: 'Click to copy' }
      : item;
    const tip = skill.desc != null && skill.desc !== ''
      ? _escapeAttr(skill.desc)
      : 'Click to copy';
    return `<tr class="skill-row">` +
      `<td class="skill-name">${_escapeAttr(skill.name)}</td>` +
      `<td class="skill-cmd" data-copy="${_escapeAttr(skill.cmd)}" title="${tip}">` +
        `<code>${_escapeAttr(skill.cmd)}</code>` +
      `</td>` +
      `</tr>`;
  }).join('');
  document.getElementById('skills-dialog-body').innerHTML =
    `<table class="skill-table"><tbody>${rows}</tbody></table>`;

  document.getElementById('skills-dialog-body').querySelectorAll('.skill-cmd[data-copy]').forEach((el) => {
    el.addEventListener('click', () => {
      const cmd = el.dataset.copy;
      navigator.clipboard.writeText(cmd).then(() => {
        el.innerHTML = '<code>Copied</code>';
        setTimeout(() => { el.innerHTML = `<code>${_escapeAttr(cmd)}</code>`; }, 1200);
      });
    });
  });

  document.getElementById('skills-dialog').classList.add('open');
}

export function _closeSkillsDialog() {
  document.getElementById('skills-dialog').classList.remove('open');
}

document.getElementById('btn-skill-workbench').addEventListener('click', () => {
  document.getElementById('skills-menu-dropdown')?.classList.remove('open');
  _openSkillsDialog();
});

document.getElementById('btn-skills-dialog-close').addEventListener('click', _closeSkillsDialog);
document.getElementById('skills-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('skills-dialog')) _closeSkillsDialog();
});
