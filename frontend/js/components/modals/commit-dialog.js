import * as api from '../../host/api.js'
import { showToast } from '../toast.js'

const HEADER_LABEL_IDLE = '↑ Commit changes';
const HEADER_LABEL_CHECKING = 'Checking…';
const CLOSE_DELAY_MS = 500;

let commitCloseTimer = null;

function clearCommitCloseTimer() {
  if (commitCloseTimer != null) {
    clearTimeout(commitCloseTimer);
    commitCloseTimer = null;
  }
}

// ── openCommitChangesDialog ────────────────────────────────────────────────

export async function openCommitChangesDialog() {
  const btn = document.getElementById('btn-push-index');
  btn.disabled = true;
  btn.textContent = HEADER_LABEL_CHECKING;

  const fileList = document.getElementById('commit-changes-file-list');
  const result = document.getElementById('commit-changes-result');
  fileList.innerHTML = '<div style="font-size:12px;color:#8c959f;">Loading…</div>';
  result.textContent = '';
  result.style.color = '';
  document.getElementById('commit-changes-msg').value = '';
  document.getElementById('btn-commit-changes-ok').disabled = false;
  document.getElementById('commit-changes-dialog').classList.add('open');

  try {
    const data = await api.fetchDiffStatus();
    if (!data) throw new Error('Could not get status');
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      fileList.innerHTML = '<div style="font-size:13px;color:#8c959f;padding:4px 0;">No changes to commit or push</div>';
      document.getElementById('btn-commit-changes-ok').disabled = true;
    } else {
      const GROUPS = [
        { key: 'new',        label: 'New' },
        { key: 'modified',   label: 'Modified' },
        { key: 'renamed',    label: 'Renamed' },
        { key: 'deleted',    label: 'Deleted' },
        { key: 'conflicted', label: 'Conflict' },
      ];
      let html = '<div style="display:flex;flex-direction:column;gap:10px;">';
      if (data.ahead) {
        html += `<div class="commit-file-group">
          <div class="commit-file-group-title" style="color:#0969da;">Ready to push (${data.ahead} local commits)</div>
          <div class="commit-file-item" style="background:#ddf4ff;color:#0550ae;">${data.ahead} local commit(s) not yet pushed</div>
        </div>`;
      }
      for (const { key, label } of GROUPS) {
        if (data[key]?.length) {
          html += `<div class="commit-file-group">
            <div class="commit-file-group-title">${label}（${data[key].length}）</div>
            ${data[key].map(f => `<div class="commit-file-item ${key}">${f}</div>`).join('')}
          </div>`;
        }
      }
      html += '</div>';
      document.getElementById('btn-commit-changes-ok').textContent = data.total ? 'Commit' : 'Push';
      fileList.innerHTML = html;
    }
  } catch (e) {
    fileList.innerHTML = `<div style="font-size:12px;color:#cf222e;">Failed to get status: ${e.message}</div>`;
  } finally {
    btn.disabled = false;
    btn.textContent = HEADER_LABEL_IDLE;
  }
}

function closeCommitChangesDialog() {
  clearCommitCloseTimer();
  document.getElementById('commit-changes-dialog').classList.remove('open');
  document.getElementById('btn-commit-changes-ok').textContent = 'Commit';
  const headerBtn = document.getElementById('btn-push-index');
  headerBtn.disabled = false;
  headerBtn.textContent = HEADER_LABEL_IDLE;
}

function doCommitChanges() {
  const msg = document.getElementById('commit-changes-msg').value.trim();
  clearCommitCloseTimer();
  commitCloseTimer = setTimeout(() => {
    commitCloseTimer = null;
    closeCommitChangesDialog();

    void api.commitFiles(msg || 'chore: update via viewer')
      .then(data => {
        const successMsg = data?.info === 'nothing to commit'
          ? '✓ Pushed'
          : '✓ Committed and pushed';
        showToast(successMsg, 'success');
      })
      .catch(e => {
        showToast(`Commit failed: ${e.message}`, 'error');
      });
  }, CLOSE_DELAY_MS);
}

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-commit-changes-cancel').addEventListener('click', closeCommitChangesDialog);
document.getElementById('btn-commit-changes-ok').addEventListener('click', doCommitChanges);
document.getElementById('commit-changes-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('commit-changes-dialog')) closeCommitChangesDialog();
});
