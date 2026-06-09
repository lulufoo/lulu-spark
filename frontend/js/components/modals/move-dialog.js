import * as api from '../../api.js'

// ── Panel switching ──────────────────────────────────────────────────────────

function switchGhOpsPanel(panelId) {
  document.querySelectorAll('.gh-ops-nav-item').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.panel === panelId);
  });
  document.querySelectorAll('.gh-ops-panel').forEach(panel => {
    panel.classList.toggle('active', panel.id === `gh-ops-panel-${panelId}`);
  });
}

document.querySelectorAll('.gh-ops-nav-item').forEach(btn => {
  btn.addEventListener('click', () => {
    const panelId = btn.dataset.panel;
    switchGhOpsPanel(panelId);
    if (panelId === 'move') document.getElementById('move-src-url').focus();
    else document.getElementById('delete-url').focus();
  });
});

// ── openMoveDocDialog / closeMoveDocDialog ─────────────────────────────────

function resetDeleteColumn() {
  document.getElementById('delete-url').value = '';
  const deleteResult = document.getElementById('delete-doc-result');
  deleteResult.textContent = '';
  deleteResult.style.color = '';
  const armBtn = document.getElementById('btn-delete-doc-arm');
  armBtn.disabled = false;
  armBtn.textContent = '确认删除意图';
  armBtn.classList.remove('armed');
  const deleteOkBtn = document.getElementById('btn-delete-doc-ok');
  deleteOkBtn.disabled = true;
  deleteOkBtn.textContent = '确认删除';
}

function openMoveDocDialog() {
  document.getElementById('move-src-url').value = '';
  document.getElementById('move-dst-url').value = '';
  const result = document.getElementById('move-doc-result');
  result.textContent = '';
  result.style.color = '';
  document.getElementById('btn-move-doc-ok').disabled = false;
  document.getElementById('btn-move-doc-ok').textContent = '确认移动';
  resetDeleteColumn();
  switchGhOpsPanel('move');
  document.getElementById('move-doc-dialog').classList.add('open');
  document.getElementById('move-src-url').focus();
}

function closeMoveDocDialog() {
  document.getElementById('move-doc-dialog').classList.remove('open');
}

async function doMoveDoc() {
  const srcUrl = document.getElementById('move-src-url').value.trim();
  const dstUrl = document.getElementById('move-dst-url').value.trim();
  const result = document.getElementById('move-doc-result');
  const okBtn = document.getElementById('btn-move-doc-ok');

  if (!srcUrl || !dstUrl) {
    result.style.color = '#cf222e';
    result.textContent = '请填写两个 URL';
    return;
  }

  okBtn.disabled = true;
  okBtn.textContent = '移动中…';
  result.style.color = '#57606a';
  result.textContent = '正在执行 gh api…';

  try {
    const data = await api.ghMove(srcUrl, dstUrl);
    if (!data.ok || data.error) throw new Error(data.error || 'failed');
    if (data.warn) {
      result.style.color = '#e09b00';
      const movedInfo = data.moved !== undefined ? `（已移动 ${data.moved} 个文件）` : '';
      result.textContent = `⚠ ${data.warn}${movedInfo}`;
    } else {
      result.style.color = '#1a7f37';
      const movedInfo = data.moved !== undefined ? `（共 ${data.moved} 个文件）` : '';
      result.textContent = `✓ 已移动到 ${data.dst_path}${movedInfo}`;
      document.dispatchEvent(new CustomEvent('cta:reload'));
      setTimeout(closeMoveDocDialog, 2000);
    }
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = `✗ ${e.message}`;
  } finally {
    okBtn.disabled = false;
    okBtn.textContent = '确认移动';
  }
}

function armDeleteDoc() {
  const url = document.getElementById('delete-url').value.trim();
  const result = document.getElementById('delete-doc-result');
  if (!url) {
    result.style.color = '#cf222e';
    result.textContent = '请填写 URL';
    return;
  }
  const armBtn = document.getElementById('btn-delete-doc-arm');
  armBtn.textContent = '已确认，可删除';
  armBtn.classList.add('armed');
  armBtn.disabled = true;
  document.getElementById('btn-delete-doc-ok').disabled = false;
  result.textContent = '';
  result.style.color = '';
}

function resetDeleteArm() {
  const armBtn = document.getElementById('btn-delete-doc-arm');
  armBtn.disabled = false;
  armBtn.textContent = '确认删除意图';
  armBtn.classList.remove('armed');
  document.getElementById('btn-delete-doc-ok').disabled = true;
}

async function doDeleteDoc() {
  const url = document.getElementById('delete-url').value.trim();
  const result = document.getElementById('delete-doc-result');
  const okBtn = document.getElementById('btn-delete-doc-ok');

  if (!url) {
    result.style.color = '#cf222e';
    result.textContent = '请填写 URL';
    return;
  }
  if (okBtn.disabled) return;

  okBtn.disabled = true;
  okBtn.textContent = '删除中…';
  result.style.color = '#57606a';
  result.textContent = '正在执行 gh api…';

  try {
    const data = await api.ghDelete(url);
    if (!data.ok || data.error) throw new Error(data.error || 'failed');
    if (data.warn) {
      result.style.color = '#e09b00';
      const deletedInfo = data.deleted !== undefined ? `（已删除 ${data.deleted} 个文件）` : '';
      result.textContent = `⚠ ${data.warn}${deletedInfo}`;
      resetDeleteArm();
    } else {
      result.style.color = '#1a7f37';
      const deletedInfo = data.deleted !== undefined ? `（共 ${data.deleted} 个文件）` : '';
      result.textContent = `✓ 已删除${deletedInfo}`;
      document.dispatchEvent(new CustomEvent('cta:reload'));
      setTimeout(closeMoveDocDialog, 2000);
    }
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = `✗ ${e.message}`;
  } finally {
    okBtn.textContent = '确认删除';
    okBtn.disabled = !document.getElementById('btn-delete-doc-arm').disabled;
  }
}

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-move-doc-header').addEventListener('click', openMoveDocDialog);
document.getElementById('btn-move-doc-cancel').addEventListener('click', closeMoveDocDialog);
document.getElementById('btn-move-doc-close').addEventListener('click', closeMoveDocDialog);
document.getElementById('move-doc-dialog').addEventListener('click', (e) => {
  if (e.target === document.getElementById('move-doc-dialog')) closeMoveDocDialog();
});
document.getElementById('btn-move-doc-ok').addEventListener('click', doMoveDoc);
document.getElementById('btn-delete-doc-arm').addEventListener('click', armDeleteDoc);
document.getElementById('btn-delete-doc-ok').addEventListener('click', doDeleteDoc);
document.getElementById('move-src-url').addEventListener('keydown', e => {
  if (e.key === 'Enter') document.getElementById('move-dst-url').focus();
});
document.getElementById('move-dst-url').addEventListener('keydown', e => {
  if (e.key === 'Enter') doMoveDoc();
  if (e.key === 'Escape') closeMoveDocDialog();
});
document.getElementById('delete-url').addEventListener('keydown', e => {
  if (e.key === 'Enter') {
    const okBtn = document.getElementById('btn-delete-doc-ok');
    if (okBtn.disabled) armDeleteDoc();
    else doDeleteDoc();
  }
  if (e.key === 'Escape') closeMoveDocDialog();
});
