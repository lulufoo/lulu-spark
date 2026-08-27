import { escHtml } from '../../shared/utils.js';
import * as api from '../../host/api.js';

export function showPendingBadge() {
  const btn = document.getElementById('btn-panel-commit');
  if (btn) btn.style.display = '';
}

export function hidePendingBadge() {
  const btn = document.getElementById('btn-panel-commit');
  if (btn) btn.style.display = 'none';
}

export function closeCommitDialog() {
  document.getElementById('md-commit-dialog').classList.remove('open');
}

export async function openCommitDialog() {
  const dialog = document.getElementById('md-commit-dialog');
  const fileList = document.getElementById('md-commit-file-list');
  const msgInput = document.getElementById('md-commit-dialog-msg');
  const resultEl = document.getElementById('md-commit-dialog-result');
  const okBtn = document.getElementById('md-btn-commit-ok');

  msgInput.value = '';
  resultEl.textContent = '';
  resultEl.style.color = '';
  fileList.innerHTML = '<div style="font-size:12px;color:#8c959f;">Loading…</div>';
  okBtn.disabled = false;
  dialog.classList.add('open');

  try {
    const data = await api.fetchDiffStatus();
    if (!data) throw new Error('Could not get status');
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      fileList.innerHTML = '<div style="font-size:13px;color:#8c959f;padding:4px 0;">No changes to commit</div>';
      okBtn.disabled = true;
      return;
    }

    const GROUPS = [
      { key: 'new',        label: 'New' },
      { key: 'modified',   label: 'Modified' },
      { key: 'deleted',    label: 'Deleted' },
      { key: 'renamed',    label: 'Renamed' },
      { key: 'conflicted', label: 'Conflict' },
    ];
    let html = '<div style="display:flex;flex-direction:column;gap:10px;">';
    for (const { key, label } of GROUPS) {
      if (data[key]?.length) {
        html += `<div class="commit-file-group">
          <div class="commit-file-group-title">${label}（${data[key].length}）</div>
          ${data[key].map(f => `<div class="commit-file-item ${key}">
            <span>${escHtml(f)}</span>
            <button class="kb-revert-btn" data-path="${escHtml(f)}" data-type="${key}">Revert</button>
          </div>`).join('')}
        </div>`;
      }
    }
    html += '</div>';
    fileList.innerHTML = html;
  } catch (e) {
    fileList.innerHTML = `<div style="font-size:12px;color:#cf222e;">Failed to get status: ${escHtml(e.message)}</div>`;
  }
}
