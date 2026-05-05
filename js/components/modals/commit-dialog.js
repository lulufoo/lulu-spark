import * as api from '../../api.js'

// ── openCommitChangesDialog ────────────────────────────────────────────────

async function openCommitChangesDialog() {
  const btn = document.getElementById('btn-push-index');
  btn.disabled = true;
  btn.textContent = '检查中…';

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
    btn.textContent = '↑ 提交变更';
  }
}

function closeCommitChangesDialog() {
  document.getElementById('commit-changes-dialog').classList.remove('open');
  document.getElementById('btn-commit-changes-ok').textContent = '提交';
}

async function doCommitChanges() {
  const btn = document.getElementById('btn-commit-changes-ok');
  const result = document.getElementById('commit-changes-result');
  const msg = document.getElementById('commit-changes-msg').value.trim();
  btn.disabled = true;
  result.textContent = '提交中…';
  result.style.color = '#8c959f';

  try {
    const data = await api.commitFiles(msg || 'chore: update via viewer');
    if (data.error) throw new Error(data.error || 'failed');
    if (data.info === 'nothing to commit') {
      result.textContent = '✓ 已推送';
      result.style.color = '#1a7f37';
      setTimeout(() => closeCommitChangesDialog(), 1500);
    } else {
      result.textContent = '✓ 提交并推送成功';
      result.style.color = '#1a7f37';
      setTimeout(() => closeCommitChangesDialog(), 1500);
    }
  } catch (e) {
    result.textContent = `失败：${e.message}`;
    result.style.color = '#cf222e';
    btn.disabled = false;
  }
}

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-push-index').addEventListener('click', openCommitChangesDialog);
document.getElementById('btn-commit-changes-cancel').addEventListener('click', closeCommitChangesDialog);
document.getElementById('btn-commit-changes-ok').addEventListener('click', doCommitChanges);
document.getElementById('commit-changes-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('commit-changes-dialog')) closeCommitChangesDialog();
});
