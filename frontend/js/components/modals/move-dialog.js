import * as api from '../../api.js'

// ── openMoveDocDialog / closeMoveDocDialog ─────────────────────────────────

function openMoveDocDialog() {
  document.getElementById('move-src-url').value = '';
  document.getElementById('move-dst-url').value = '';
  const result = document.getElementById('move-doc-result');
  result.textContent = '';
  result.style.color = '';
  document.getElementById('btn-move-doc-ok').disabled = false;
  document.getElementById('btn-move-doc-ok').textContent = '确认移动';
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
    let data;
    try { data = await api.ghMove(srcUrl, dstUrl); }
    catch { throw new Error('服务器未返回 JSON，请重试或检查 App 日志'); }
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

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-move-doc-header').addEventListener('click', openMoveDocDialog);
document.getElementById('btn-move-doc-cancel').addEventListener('click', closeMoveDocDialog);
document.getElementById('btn-move-doc-ok').addEventListener('click', doMoveDoc);
document.getElementById('move-src-url').addEventListener('keydown', e => {
  if (e.key === 'Enter') document.getElementById('move-dst-url').focus();
});
document.getElementById('move-dst-url').addEventListener('keydown', e => {
  if (e.key === 'Enter') doMoveDoc();
  if (e.key === 'Escape') closeMoveDocDialog();
});
