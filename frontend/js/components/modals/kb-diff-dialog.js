import * as api from '../../host/api.js';

const GROUPS = [
  { key: 'new', label: 'New' },
  { key: 'modified', label: 'Modified' },
  { key: 'renamed', label: 'Renamed' },
  { key: 'deleted', label: 'Deleted' },
  { key: 'conflicted', label: 'Conflict' },
];

let activeRepo = '';
let closeTimer = null;

function getElements() {
  return {
    dialog: document.getElementById('kb-diff-dialog'),
    box: document.getElementById('kb-diff-dialog-box'),
    title: document.getElementById('kb-diff-dialog-title'),
    fileList: document.getElementById('kb-diff-file-list'),
    msg: document.getElementById('kb-diff-msg'),
    result: document.getElementById('kb-diff-result'),
    okBtn: document.getElementById('btn-kb-diff-ok'),
    cancelBtn: document.getElementById('btn-kb-diff-cancel'),
    revertBtn: document.getElementById('btn-kb-diff-revert-all'),
  };
}

function repoShortName(repo) {
  return repo.split('/').pop() || repo;
}

function clearCloseTimer() {
  if (closeTimer) {
    clearTimeout(closeTimer);
    closeTimer = null;
  }
}

function emitUpdated() {
  window.dispatchEvent(new CustomEvent('kb-diff-updated', { detail: { repo: activeRepo } }));
}

function renderGroups(data) {
  const { fileList } = getElements();
  let html = '<div style="display:flex;flex-direction:column;gap:10px;">';
  if (data.ahead) {
    html += `<div class="commit-file-group">
      <div class="commit-file-group-title" style="color:#0969da;">Ready to push (${data.ahead} local commits)</div>
      <div class="commit-file-item renamed" style="display:block;">${data.ahead} local commit(s) not yet pushed</div>
    </div>`;
  }
  for (const { key, label } of GROUPS) {
    if (!data[key]?.length) continue;
    html += `<div class="commit-file-group">
      <div class="commit-file-group-title">${label}（${data[key].length}）</div>
      ${data[key].map((path) => `<div class="commit-file-item ${key}">${path}</div>`).join('')}
    </div>`;
  }
  html += '</div>';
  fileList.innerHTML = html;
}

async function refreshDialog() {
  const { fileList, okBtn, revertBtn, title } = getElements();
  title.textContent = `✎ Local changes · ${repoShortName(activeRepo)}`;
  fileList.innerHTML = '<div style="font-size:12px;color:#8c959f;">Loading…</div>';
  okBtn.disabled = true;
  revertBtn.disabled = true;

  try {
    const data = await api.fetchKbStatus(activeRepo);
    if (data?.error) throw new Error(data.error);
    if (!data?.total && !data?.ahead) {
      fileList.innerHTML = '<div style="font-size:13px;color:#8c959f;padding:4px 0;">No local changes</div>';
      return;
    }
    renderGroups(data);
    okBtn.disabled = false;
    revertBtn.disabled = false;
  } catch (error) {
    fileList.innerHTML = `<div style="font-size:12px;color:#cf222e;">Failed to get status: ${error.message}</div>`;
  }
}

export async function openKbDiffDialog(repo) {
  const { dialog, msg, result, okBtn } = getElements();
  activeRepo = repo;
  clearCloseTimer();
  msg.value = '';
  result.textContent = '';
  result.style.color = '';
  okBtn.textContent = 'Commit';
  dialog.classList.add('open');
  await refreshDialog();
}

export function closeKbDiffDialog() {
  const { dialog, result, okBtn } = getElements();
  clearCloseTimer();
  dialog.classList.remove('open');
  result.textContent = '';
  result.style.color = '';
  okBtn.disabled = false;
  okBtn.textContent = 'Commit';
}

async function handleCommit() {
  const { msg, result, okBtn, revertBtn } = getElements();
  okBtn.disabled = true;
  revertBtn.disabled = true;
  result.textContent = 'Committing…';
  result.style.color = '#8c959f';

  try {
    const data = await api.commitKbFile(activeRepo, msg.value.trim() || 'chore: update via viewer');
    if (data?.error) throw new Error(data.error);
    result.textContent = '✓ Committed and pushed';
    result.style.color = '#1a7f37';
    emitUpdated();
    closeTimer = setTimeout(() => closeKbDiffDialog(), 1500);
  } catch (error) {
    result.textContent = `Failed: ${error.message}`;
    result.style.color = '#cf222e';
    okBtn.disabled = false;
    revertBtn.disabled = false;
  }
}

async function handleRevertAll() {
  const { result, okBtn, revertBtn } = getElements();
  okBtn.disabled = true;
  revertBtn.disabled = true;
  result.textContent = 'Reverting…';
  result.style.color = '#8c959f';

  try {
    const data = await api.revertKbFile(activeRepo);
    if (data?.error) throw new Error(data.error);
    result.textContent = '✓ Local changes discarded';
    result.style.color = '#1a7f37';
    emitUpdated();
    await refreshDialog();
    closeTimer = setTimeout(() => closeKbDiffDialog(), 1500);
  } catch (error) {
    result.textContent = `Failed: ${error.message}`;
    result.style.color = '#cf222e';
    okBtn.disabled = false;
    revertBtn.disabled = false;
  }
}

getElements().cancelBtn?.addEventListener('click', closeKbDiffDialog);
getElements().okBtn?.addEventListener('click', handleCommit);
getElements().revertBtn?.addEventListener('click', handleRevertAll);
getElements().dialog?.addEventListener('click', (event) => {
  if (event.target === getElements().dialog) closeKbDiffDialog();
});
