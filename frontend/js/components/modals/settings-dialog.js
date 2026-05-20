import * as api from '../../api.js';

// ── Nav switching ──────────────────────────────────────────────────────────

function switchPanel(panelId) {
  document.querySelectorAll('.settings-nav-item').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.panel === panelId);
  });
  document.querySelectorAll('.settings-panel').forEach(panel => {
    panel.classList.toggle('active', panel.id === `settings-panel-${panelId}`);
  });
}

document.querySelectorAll('.settings-nav-item').forEach(btn => {
  btn.addEventListener('click', () => switchPanel(btn.dataset.panel));
});

// ── Helpers ────────────────────────────────────────────────────────────────

function setResult(resultElId, message, isError = false) {
  const el = document.getElementById(resultElId);
  if (!el) return;
  el.textContent = message || '';
  el.style.color = isError ? '#cf222e' : '#1a7f37';
}

// ── Load snapshot ──────────────────────────────────────────────────────────

async function loadSettingsSnapshot() {
  try {
    const cfg = await api.fetchConfig();

    const archiveInput = document.getElementById('settings-archive-root');
    const kbInput = document.getElementById('settings-kb-root');
    if (cfg?.archive_root) {
      archiveInput.placeholder = cfg.archive_root;
      archiveInput.value = cfg.archive_root;
    }
    if (cfg?.kb_root) {
      kbInput.placeholder = cfg.kb_root;
      kbInput.value = cfg.kb_root;
    }

    const hintEl = document.getElementById('settings-token-hint');
    hintEl.textContent = cfg?.has_github_token
      ? '当前已配置 GitHub Token。输入新 Token 可覆盖。'
      : '当前未配置 GitHub Token。';
  } catch {
    document.getElementById('settings-token-hint').textContent =
      '读取当前配置失败，可直接输入并保存。';
  }
}

// ── Open / close ───────────────────────────────────────────────────────────

export async function openSettingsDialog() {
  setResult('settings-result-directories', '');
  setResult('settings-result-github', '');
  document.getElementById('settings-github-token').value = '';
  switchPanel('directories');
  await loadSettingsSnapshot();
  document.getElementById('settings-dialog').classList.add('open');
}

function closeSettingsDialog() {
  document.getElementById('settings-dialog').classList.remove('open');
}

document.getElementById('btn-settings-close').addEventListener('click', closeSettingsDialog);
document.getElementById('btn-settings-cancel').addEventListener('click', closeSettingsDialog);
document.getElementById('settings-dialog').addEventListener('click', (e) => {
  if (e.target === document.getElementById('settings-dialog')) closeSettingsDialog();
});

// ── Save: 目录配置 ──────────────────────────────────────────────────────────

document.getElementById('btn-settings-save-directories').addEventListener('click', async () => {
  const btn = document.getElementById('btn-settings-save-directories');
  const archiveRoot = document.getElementById('settings-archive-root').value.trim();
  const kbRoot = document.getElementById('settings-kb-root').value.trim();

  if (!archiveRoot && !kbRoot) {
    setResult('settings-result-directories', '请填写至少一个目录路径。', true);
    return;
  }

  const payload = {};
  if (archiveRoot) payload.archive_root = archiveRoot;
  if (kbRoot) payload.kb_root = kbRoot;

  btn.disabled = true;
  btn.textContent = '保存中…';
  try {
    const resp = await api.setConfig(payload);
    if (resp?.error) throw new Error(resp.error);
    const parts = [];
    if (archiveRoot) parts.push('归档目录');
    if (kbRoot) parts.push('知识库目录');
    setResult('settings-result-directories', `已保存：${parts.join('、')}。`);
    await loadSettingsSnapshot();
  } catch (e) {
    setResult('settings-result-directories', `保存失败：${e.message || String(e)}`, true);
  } finally {
    btn.disabled = false;
    btn.textContent = '保存目录';
  }
});

// ── Save: GitHub Token ─────────────────────────────────────────────────────

document.getElementById('btn-settings-save-github').addEventListener('click', async () => {
  const btn = document.getElementById('btn-settings-save-github');
  const token = document.getElementById('settings-github-token').value.trim();

  if (!token) {
    setResult('settings-result-github', '请输入 GitHub Token 后再保存。', true);
    return;
  }

  btn.disabled = true;
  btn.textContent = '保存中…';
  try {
    const resp = await api.setConfig({ github_token: token });
    if (resp?.error) throw new Error(resp.error);
    setResult('settings-result-github', 'GitHub Token 已保存。');
    document.getElementById('settings-github-token').value = '';
    await loadSettingsSnapshot();
  } catch (e) {
    setResult('settings-result-github', `保存失败：${e.message || String(e)}`, true);
  } finally {
    btn.disabled = false;
    btn.textContent = '保存 Token';
  }
});
