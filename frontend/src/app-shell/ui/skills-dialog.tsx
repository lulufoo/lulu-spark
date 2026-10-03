import { useState, useSyncExternalStore } from 'react';
import { OverlayDismissButton } from '../../shared/overlay-dismiss-button.tsx';
import { _closeSkillsDialog, skillsStore } from '../commands/skills-dialog.ts';
import type { SkillItem } from '../state/skills.ts';

export { _closeSkillsDialog, _openSkillsDialog } from '../commands/skills-dialog.ts';

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
          <OverlayDismissButton
            id="btn-skills-dialog-close"
            onClick={() => _closeSkillsDialog()}
          />
        </div>
        <div id="skills-dialog-body">
          <SkillTable items={items} />
        </div>
      </div>
    </div>
  );
}
