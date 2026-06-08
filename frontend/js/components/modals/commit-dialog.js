import * as api from '../../api.js'
import { showToast } from '../toast.js'

const HEADER_LABEL_IDLE = '↑ 提交变更';
const HEADER_LABEL_CHECKING = '检查中…';

// ── openCommitChangesDialog ────────────────────────────────────────────────

async function openCommitChangesDialog() {
  const btn = document.getElementById('btn-push-index');
  btn.disabled = true;
  btn.textContent = HEADER_LABEL_CHECKING;

  const fileList = document.getElementById('commit-changes-file-list');
  const result = document.getElementById('commit-changes-result');
  fileList.innerHTML = '<div style="font-size:12px;color:#8c959f;">加载中…</div>';
  result.textContent = '';
  result.style.color = '';
  document.getElementById('commit-changes-msg').value = '';
  document.getElementById('btn-commit-changes-ok').disabled = false;
  document.getElementById('commit-changes-dialog').classList.add('open');

  try {
    const data = await api.fetchDiffStatus();
    if (!data) throw new Error('无法获取状态');
    if (data.error) throw new Error(data.error);

    if (!data.total && !data.ahead) {
      fileList.innerHTML = '<div style="font-size:13px;color:#8c959f;padding:4px 0;">没有待提交或待推送的变更</div>';
      document.getElementById('btn-commit-changes-ok').disabled = true;
    } else {
      const GROUPS = [
        { key: 'new',        label: '新增' },
        { key: 'modified',   label: '修改' },
        { key: 'renamed',    label: '重命名' },
        { key: 'deleted',    label: '删除' },
        { key: 'conflicted', label: '冲突' },
      ];
      let html = '<div style="display:flex;flex-direction:column;gap:10px;">';
      if (data.ahead) {
        html += `<div class="commit-file-group">
          <div class="commit-file-group-title" style="color:#0969da;">待推送（${data.ahead} 个本地提交）</div>
          <div class="commit-file-item" style="background:#ddf4ff;color:#0550ae;">本地已有 ${data.ahead} 个提交尚未推送到远端</div>
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
      document.getElementById('btn-commit-changes-ok').textContent = data.total ? '提交' : '推送';
      fileList.innerHTML = html;
    }
  } catch (e) {
    fileList.innerHTML = `<div style="font-size:12px;color:#cf222e;">获取状态失败：${e.message}</div>`;
  } finally {
    btn.disabled = false;
    btn.textContent = HEADER_LABEL_IDLE;
  }
}

function closeCommitChangesDialog() {
  document.getElementById('commit-changes-dialog').classList.remove('open');
  document.getElementById('btn-commit-changes-ok').textContent = '提交';
  const headerBtn = document.getElementById('btn-push-index');
  headerBtn.disabled = false;
  headerBtn.textContent = HEADER_LABEL_IDLE;
}

function doCommitChanges() {
  const msg = document.getElementById('commit-changes-msg').value.trim();
  closeCommitChangesDialog();

  void api.commitFiles(msg || 'chore: update via viewer')
    .then(data => {
      const successMsg = data?.info === 'nothing to commit'
        ? '✓ 已推送'
        : '✓ 提交并推送成功';
      showToast(successMsg, 'success');
    })
    .catch(e => {
      showToast(`提交失败：${e.message}`, 'error');
    });
}

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-push-index').addEventListener('click', openCommitChangesDialog);
document.getElementById('btn-commit-changes-cancel').addEventListener('click', closeCommitChangesDialog);
document.getElementById('btn-commit-changes-ok').addEventListener('click', doCommitChanges);
document.getElementById('commit-changes-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('commit-changes-dialog')) closeCommitChangesDialog();
});
