import { state, getEntryId } from '../../state.js'
import * as api from '../../api.js'
import { closeModal } from '../viewer.js'

// ── openDeleteDialog / closeDeleteDialog ───────────────────────────────────

export function openDeleteDialog() {
  const input = document.getElementById('delete-confirm-input');
  const okBtn = document.getElementById('btn-delete-confirm-ok');
  input.value = '';
  okBtn.disabled = true;
  document.getElementById('delete-dialog').classList.add('open');
  input.focus();
}

function closeDeleteDialog() {
  document.getElementById('delete-dialog').classList.remove('open');
}

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-copy-confirm-word').addEventListener('click', () => {
  navigator.clipboard.writeText('CONFIRM');
  const btn = document.getElementById('btn-copy-confirm-word');
  btn.textContent = '✓';
  setTimeout(() => { btn.textContent = 'Copy'; }, 1200);
});

document.getElementById('delete-confirm-input').addEventListener('input', e => {
  document.getElementById('btn-delete-confirm-ok').disabled = (e.target.value !== 'CONFIRM');
});

document.getElementById('btn-delete-cancel').addEventListener('click', closeDeleteDialog);

document.getElementById('btn-delete-confirm-ok').addEventListener('click', async () => {
  const okBtn = document.getElementById('btn-delete-confirm-ok');
  okBtn.disabled = true;
  okBtn.textContent = 'Deleting…';
  try {
    const entryId = getEntryId(state.viewer.entry);
    const data = await api.deleteEntry(entryId);
    if (data.error) throw new Error(data.error);
    closeDeleteDialog();
    closeModal();
    document.dispatchEvent(new CustomEvent('cta:reload'));
  } catch (e) {
    alert(`Delete failed: ${e.message}`);
    okBtn.disabled = false;
    okBtn.textContent = 'Confirm delete';
  }
});
